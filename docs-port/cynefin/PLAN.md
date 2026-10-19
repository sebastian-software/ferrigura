# CYNEFIN parser and DB: plan and decisions

The parser task of `cynefin-beta` (P3). The research, with its sources and probes, is in [NOTES.md](./NOTES.md); this page records what the port does, the decisions it took, the quirks of the reference that it reproduces, and the places where the differential run showed that the notes were wrong. The parser is `crates/ferrigura-syntax/src/cynefin/` (scanner, grammar, DB and the entry point), the model is `crates/ferrigura-model/src/cynefin.rs`, and the exporter is [`tools/port/export-cynefin.ts`](../../tools/port/export-cynefin.ts).

The reference is the Langium grammar `packages/parser/src/language/cynefin/cynefin.langium` with `common.langium`, parsed by Chevrotain, and the DB `packages/mermaid/src/diagrams/cynefin/cynefinDb.ts` with `cynefinParser.ts`. The decision to write the parser by hand is [0002](../decisions/0002-hand-written-parsers.md).

## 1. Scope

The parser task covers the grammar, the lexer's order, the value conversions, the DB calls with their sanitising and defaults, the model, the configuration section of the differential, and the specs. The renderer (the layout arithmetic, the item badges, the boundary waves and the seed) is the renderer stage: its specs stay in [`cynefin_boundary_spec.rs`](../../crates/ferrigura-syntax/tests/cynefin_boundary_spec.rs) as ignored tests.

## 2. The lexer

The terminals are tried in the order of the Chevrotain lexer, and the first one that matches wins (not the longest). The scanner in `cynefin/scanner.rs` runs that order:

1. `cynefin-beta:`, with no word end.
2. `cynefin-beta`, with the word end `(?:(?=%%)|(?!\S))`: the word ends at `%%`, a blank, or the end of the text.
3. `-->`, then `:`.
4. `DOMAIN_NAME`: `complex`, `complicated`, `clear`, `chaotic`, `confusion`, as prefixes.
5. `ACC_DESCR`, `ACC_TITLE` and `TITLE`. Each takes its leading blanks, so a title can follow a domain on the same line.
6. `STRING`, with `"` or `'`. A raw line break is allowed in it, an escape of a line break is not (`.` does not match one).
7. `NEWLINE`, then the hidden `WHITESPACE`, `YAML`, `DIRECTIVE` and `SINGLE_LINE_COMMENT`.

A position that no terminal takes is a lexical error. The reference rejects the whole text for it, so the port rejects it too; the error's wording is ferrigura's own.

## 3. The grammar

The rules are functions over the tokens (`cynefin/grammar.rs`), with the decisions the reference makes:

- The header is `NEWLINE* HEADER NEWLINE*`. Either form is the same token.
- A `DOMAIN_NAME` followed by `-->` starts a transition, any other starts a block. A transition ends with `NEWLINE+` or the end of the text.
- A block takes its items after any number of line breaks, and each item is a `STRING`.
- A header line (`accDescr`, `accTitle`, `title`) takes its line break or the end of the text, and a run of them is one statement. The last value of each header text is the one that stays.

## 4. The DB and the model

The DB (`cynefin/db.rs`) is the state the reference's `cynefinDb.ts` holds: the header texts, the domains and the transitions. `populate` (`cynefin/mod.rs`) makes the calls in the reference's order, and `Diagram.fromText` makes the front matter title call first:

1. `setDiagramTitle` with the front matter title, when there is one.
2. `populateCommonDb`: `setAccDescription`, then `setAccTitle`, then `setDiagramTitle`, each only when the grammar's value is not empty.
3. `setDomains`: one entry per block. A block whose name is already there replaces the items and keeps the entry's place (`IndexMap`, as `Map.set`).
4. `setTransitions`: the self-loops are dropped, and the label is `null` when it is absent or empty (`t.label || undefined`).

The header texts go through the sanitiser of `sanitize.rs` (the strict path of `sanitizeText`, which a diagram takes by default), with `setAccTitle`'s and `setAccDescription`'s whitespace rules. Items and labels are kept as the grammar gives them, with the entity placeholders (`ﬂ°name¶ß`).

The model is [`CynefinDiagram`](../../crates/ferrigura-model/src/cynefin.rs): `version`, `title`, `accTitle`, `accDescr`, `domains` (`name`, `items`) and `transitions` (`from`, `to`, `label`). The rules are in [canonical-model.md](../canonical-model.md), Cynefin.

## 5. Quirks reproduced

Each one has its probe as a test in `crates/ferrigura-syntax/tests/cynefin_grammar.rs`, and the differential run covers the same inputs (`cases.ts`).

1. **Domain names are prefixes.** `complexity`, `clearly` and `complex-` are errors after `complex`, `clear` and `complex`. Names are case-sensitive, and an unknown word is an error.
2. **Two spellings of the header.** `cynefin-beta:x` and `cynefin-betaX` are errors; `cynefin-beta%%note` is parsed. The detector is a different matter (quirk 15).
3. **Items follow their domain.** A string after a title or a transition is an error. A blank line and a comment line may come between a block and its items.
4. **Blocks and transitions share lines.** `complex "A" complicated --> clear` is a block and a transition. Only the block is a domain.
5. **A title can follow a domain on its line.** `complex title x` is the block and the title `x`.
6. **Strings keep escapes in an odd way.** `\b \f \n \r \t \v \0` give control characters and any other `\c` gives `c`, so `A` is `u0041`. A raw line break in a string is kept.
7. **A transition label can be empty.** `""` is no label (`null`).
8. **Transitions need their line end.** Two arrows, an unquoted label or a label without its colon are errors.
9. **The last title wins, and an empty last title is empty.** The front matter title stands when the body's last title is empty, and the body title wins otherwise.
10. **A transition to a domain with no block is drawn.** The DB keeps the transition; the domain is not in `domains`.
11. **A repeated block replaces the items and keeps its place.**
12. **Self-loops are dropped at the DB.** The reference warns, and the model has no trace of them.
13. **Titles and accessibility texts are sanitised; items and labels are not.** The title, the accessible title and the accessible description go through DOMPurify (`a < b` is `a &lt; b`, `A<script>…</script>B` is `AB`, `<img src=x onerror=…> T` is `<img src="x"> T`). An item `<b>x</b> & y` is kept as written.
14. **Entities are placeholders in the DB.** `#amp;` in an item stays `ﬂ°amp¶ß`; the renderer decodes it.
15. **Detection (a shared file).** `detectType` strips a `%%` comment that ends in a line break before it matches `^\s*cynefin-beta(?:[\s:]|$)`. So `cynefin-beta%%note` followed by a line is the header, and the same comment at the end of the text is not. `NOTES.md` section 2.4 said the detector rejects it; the port follows the reference (`cynefin_header` in `crates/ferrigura-syntax/src/lib.rs`, tests in `tests/detect.rs`).

## 6. Corrections to the notes

The differential run and the reference's own output showed two places where [NOTES.md](./NOTES.md) is wrong. The port follows the reference.

- Section 2.6 item 4 says the DB then holds all the domains of a line such as `complex "A" complicated --> clear`. It holds the block only: `setDomains` takes the blocks, and `complicated` is the transition's `from`.
- Section 2.4 says `cynefin-beta%%note` is not detected. It is detected when a line break ends the comment (quirk 15).

## 7. Configuration

The cynefin section of the effective configuration is compared by `pnpm port:diff config` (the group `cynefin` in `diff-config.ts`, its section out of `UNPORTED_SECTIONS` and `NOT_PORTED_SECTIONS`). The configuration is not in the model, and the values are kept as the reference keeps them: `"width":"wide"` stays a string, and the renderer draws the `NaN` it gives (Q5, the renderer stage). The three config cases of `cases.ts` test a non-numeric width, a front matter width and a directive with a boolean. The redux themes have no cynefin block (`NOTES.md` section 6.8), so the theme variables of the shipped themes have none either, on both sides.

## 8. Specs and differential

- `cynefin_db_spec.rs` (11): the reference's DB spec, through `parse`, since the Rust DB is not public. The `getConfig` test reads the cynefin defaults of the schema.
- `cynefin_grammar.rs` (55): the reference's integration spec (15) and one test per quirk above.
- `cynefin_boundary_spec.rs` (16, ignored): the boundary waves and `resolveSeed`, which the renderer stage draws. Each names the reason.
- `pnpm port:diff cynefin`: every collected input (the e2e fixtures, the spec literals, the docs and `cases.ts`), with no difference. `pnpm port:diff config`: the cynefin group and the section, with no difference.

## 9. Shared files changed

- `crates/ferrigura-syntax/src/lib.rs`: the cynefin header test is `cynefin_header` (quirk 15).
- `crates/ferrigura-syntax/examples/canonical.rs`: the cynefin arm detects the type first, as the architecture arm does.
- `tools/port/diff-config.ts`: `cynefin` leaves `UNPORTED_SECTIONS` and `NOT_PORTED_SECTIONS`.
- `tools/port/status.ts`: the suites of cynefin.
- `tools/port/cases.ts`: the probes, the undetected inputs and the config cases of cynefin.

## 10. Renderer (P3 stage 5)

The renderer is `crates/ferrigura-render/src/cynefin/`: `boundary.rs` (the seeded generator, the hash of the id, `resolveSeed` and the four wave paths), `layout.rs` (the domains and their places), `style.rs` (the `cynefin` block of the theme with its defaults, the JavaScript conversions of the values, and the rules), `text.rs` (the markup of a text, the characters it shows, the width of a badge), `draw.rs` (the groups in the reference's order) and `mod.rs` (the root and the accessible texts). The research is section 6 of [NOTES.md](./NOTES.md).

**Checked.** `pnpm port:geometry cynefin`: all 12 fixtures and the 11 probes of `tools/port/geometry/probes/cynefin/` are within tolerance, and every box, path sample and text is equal to the reference's (the harness reports no numeric difference). `--self` gives the same result. The forced seed is the fixture's own `cynefin.seed`, or 1 when it has none, as the e2e viewer sets it (`e2e/helpers/util.ts`). The unit and integration tests are `tests/cynefin_boundary_spec.rs` (the boundary specs of `cynefin.spec.ts`, with the vectors the probes recorded) and `tests/cynefin_render.rs` (one test per quirk below).

**Quirks reproduced** (each with its test):

1. The seed: a non-zero finite seed is used as it is, a negative one included; zero, `NaN` and the infinities take the hash of the id (`resolveSeed`). `|0` truncates a fractional seed, so 1 and 1.5 draw the same waves.
2. Amplitude 0 draws straight boundaries (`??` keeps the 0).
3. The title sits at `-padding / 2`; with padding 0 it is written `0`, not `-0`.
4. An empty label measures 0 and falls back to its length times 7, so its badge is 20 wide.
5. The badge is measured on the label as the DB holds it, with its entity placeholders; the text is written decoded (the whole-SVG pass of `mermaidAPI.ts` 359).
6. The confusion ellipse shows three items and a `+N more` badge after them; the cap is three.
7. The items group is written even when no domain has an item.
8. Without descriptions the labels and the items sit at the centres (`showDomainDescriptions`).
9. The transitions: a quadratic curve bent to the left of its direction, the label at the control point minus 6; a transition to a name that is not a domain is not drawn, and the defs stay.
10. A self-loop that reaches the renderer is skipped (the DB drops them first).
11. A null colour override removes the presentation attribute (`d3`'s `attr`); a string font size concatenates in `domainFontSize + 2` (`"20"` gives a 202 px title).
12. The root's size follows `useMaxWidth` of the cynefin section only.
13. An item font size that is not a length (`"14px"`, which the style sheet writes as `14pxpx`) is dropped by the browser, so the badges are measured at the root's font size (`style::item_measure_size`; probe `p11`).

**Decisions of the renderer.** The colours are custom properties on the id, and each coloured element carries its colour as a presentation attribute (decision 0007). The domain fills take a class `cynefin-fill-<domain>`, which the geometry harness leaves out of the structural key, as it does the pie's slot classes, so that the dark scheme can choose the fill. The arrows of the subtitles are set in the symbol face by the text crate (`fallback_runs`).

**Not reproduced.** A `cynefin` value in `themeVariables` that is not an object: the reference merges it key by key and draws `undefined` in the text, and ferrigura, as for `xyChart` and `packet`, takes the defaults. A `boundaryAmplitude` of `null`: the typed configuration has no such value.
