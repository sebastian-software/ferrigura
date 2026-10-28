# Cynefin diagram

A Cynefin diagram draws the Cynefin sensemaking framework: four quadrants (complex, complicated, chaotic, clear) around a central confusion ellipse, separated by wavy boundaries, with items in each domain and optional transitions between domains. The keyword is `cynefin-beta`. ferrigura reads it with a hand-written scanner and parser (`ferrigura-syntax`, `src/cynefin`) of the Langium grammar and draws it with a renderer of its own (`ferrigura-render`, `src/cynefin`); there is no layout engine, the layout is arithmetic on the configured size, and the boundary waves come from a seeded random generator.

## Model

`ferrigura_model::cynefin::CynefinDiagram` is the DB state after the parse (`cynefinDb.ts`), version 1:

| Field         | Holds                                                                                |
| ------------- | ------------------------------------------------------------------------------------ |
| `title`       | The title, sanitized; `""` when unset                                                |
| `accTitle`    | The accessible title, sanitized, without leading blanks; `""` when unset             |
| `accDescr`    | The accessible description, sanitized, with the blanks after each line break removed |
| `domains`     | The domains that have a block, each `{ name, items }`, in order of first appearance  |
| `transitions` | The transitions `{ from, to, label }` in declaration order, without self-loops       |

The model keeps the DB's quirks:

- **A domain is in `domains` when it has a block.** A repeated block replaces the items of the first and keeps its place, as `Map.set` does. A domain that only a transition names is not in `domains`, but the renderer draws the transition.
- **Items and labels are raw.** They are not sanitized and keep the entity placeholders (`ﬂ°amp¶ß`); the renderer decodes them.
- **A transition label that is absent or `""` is `null`**, since the DB stores `undefined` for both.
- **Self-loops are dropped** by the DB, which warns; the model has no trace of them.
- **Title precedence.** The front matter title is set first. The grammar's `title`, `accTitle` and `accDescr` each replace it only when not empty, so an empty last `title` lets the front matter title stand.

The configuration is not in the model, and neither are the renderer's data (positions, boundary waves, seed). The conventions of the whole family are in [canonical-model.md](../canonical-model.md#cynefin).

## Parsing

`cynefin::parse` runs the steps of `Diagram.fromText`: it prepares the text (front matter and its title), removes directives and comment lines, encodes entities and adds a line break. The DB calls come from `populate`, in the reference's order: the accessible description, the accessible title, the title (each only when not empty), then the domains, then the transitions ([0007](../decisions/0007-hand-written-parsers.md)).

The scanner runs the terminals of the Chevrotain lexer in the order Langium builds them, and the first that matches wins, not the longest:

1. `cynefin-beta:` with no word end, then `cynefin-beta` with the word end `(?=%%)` or not followed by a non-blank;
2. `-->`, then `:`;
3. the domain names `complex`, `complicated`, `clear`, `chaotic` and `confusion`, as prefixes;
4. `accDescr`, `accTitle` and `title`, each with its leading blanks;
5. `STRING` in `"` or `'`, which may span lines;
6. a line break, then the hidden white space, YAML, directive and comment terminals.

A position that no terminal takes is a lexical error and the whole text is rejected. The grammar is `NEWLINE* HEADER NEWLINE*` followed by title lines, domain blocks and transitions. A domain name followed by `-->` starts a transition, any other starts a block; a block takes its items (strings) after any number of line breaks; a transition ends with a line break or the end of the text.

The quirks that ferrigura keeps because the reference has them, each pinned by a test in `cynefin_grammar.rs`:

- **Domain names are prefixes and case-sensitive.** `complexity` is the domain `complex` followed by an error; `Complex` and unknown words are errors.
- **Two spellings of the header.** `cynefin-beta:x` and `cynefin-betaX` are errors, `cynefin-beta%%note` parses. The detector is another expression, `^\s*cynefin-beta(?:[\s:]|$)` after comments are stripped; `cynefin_header` in `crates/ferrigura-syntax/src/lib.rs` ports it, so `cynefin-beta%%note` followed by a line is detected and the same comment at the end of the text is not.
- **Items follow their domain.** A string after a title or a transition is an error; blank lines and comment lines may sit between a block and its items.
- **Blocks and transitions share lines.** `complex "A" complicated --> clear` is a block and a transition, and only the block is a domain. A title can follow a domain on its line (`complex title x`).
- **Strings keep escapes oddly.** `\b \f \n \r \t \v \0` give control characters and any other `\c` gives `c`, so `A` is `u0041`. A raw line break in a string is kept.
- **A label can be empty**, and transitions need their line end: two arrows, an unquoted label or a label without its colon are errors.
- **Titles and accessible texts are sanitized, items and labels are not.** `a < b` becomes `a &lt; b`, `A<script>…</script>B` becomes `AB`; an item `<b>x</b> & y` is kept as written.

## Layout and rendering

The renderer ports `cynefinRenderer.ts` (`draw`), `cynefinBoundaries.ts` and `styles.ts`. `mod.rs` writes the root and the accessible texts, `layout.rs` holds the domains and their places, `boundary.rs` the seeded generator, the id hash and the four wave paths, `style.rs` the `cynefin` block of the theme with its defaults and the style rules, `text.rs` the markup of a text and the width of a badge, and `draw.rs` the groups in the reference's order.

- **Size.** The viewBox is `0 0 (width + 2 padding) (height + 2 padding)` and depends on no measurement; the defaults are 800, 600 and 40. The root takes `width="100%"` and a `max-width` with `cynefin.useMaxWidth`, which only the cynefin section sets.
- **Domains.** The quadrants are the four boxes of half the width and height; the confusion ellipse has radii of 0.15 of the width and height at the center. Items are drawn per domain in the order complex, complicated, chaotic, clear, confusion, and in declaration order inside a domain. A badge is 26 px high with 10 px of padding on each side of its text; items start 30 px apart.
- **Confusion.** The ellipse shows three items and a `+N more` badge after them.
- **Boundaries.** The fold between the left and right domains and the horizontal line between the upper and lower ones are cubic paths over seven segments, with points and control points displaced by a mulberry32 generator. The cliff between clear and chaotic uses 0.03 of the width as its amplitude and has no seed. The seed is the configured `cynefin.seed` when it is a finite non-zero number, otherwise the hash of the SVG id (`hashString`, over UTF-16 code units, with int32 arithmetic). A fractional seed is truncated by `| 0`, so 1 and 1.5 draw the same waves. An amplitude of 0 draws straight boundaries.
- **Transitions.** A transition is a quadratic curve from the center of one domain to the center of the other, bent to the left of its direction by 0.15 of its length, with the label at the control point minus 6 px. A transition to a name that is not a domain is not drawn.
- **Text.** The only measured text is an item's label, at the item font size in the regular face; an empty label measures 0 and falls back to its UTF-16 length times 7, so its badge is 20 px wide. The badge is measured on the label as the DB holds it, with the entity placeholders; the text is written decoded.
- **Theme.** The `cynefin` block of the theme variables is merged key by key onto the default theme's block, as `cleanAndMerge` does; the redux themes have no block, so they draw the default one (pastel domain backgrounds, `#131300` labels) in both schemes. Each color is a custom property on the id, and each colored element carries its color as an attribute ([0011](../decisions/0011-themes-as-css-variables.md)). The domain fills carry a class `cynefin-fill-<domain>` so that the dark scheme can choose the fill.
- **Arrows in the subtitles.** The subtitles contain `→`, which the text faces lack; the text crate sets it in the symbol face ([0010](../decisions/0010-text-measurement-and-fonts.md)).

## Differences from Mermaid

- Labels are SVG text and the fonts are embedded subsets ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)). The reference's cynefin code draws SVG text too, so only the fonts differ.
- A `cynefin` value in `themeVariables` that is not an object is not merged key by key: the reference draws `undefined` in the text, ferrigura takes the defaults, as it does for `xyChart` and `packet`.

## Known gaps

- A `boundaryAmplitude` of `null` is not representable in the typed configuration.
- The sanitizing of the header texts uses the default configuration, as in the other types.

## Checking it

- `pnpm -C tools/port port:diff cynefin` compares the canonical models of the fixtures, the string literals of Mermaid's specs, the documentation and the cases of `tools/port/cases.ts`; `pnpm -C tools/port port:diff config` compares the `cynefin` section of the configuration, including a non-numeric width, a front matter width and a directive with a boolean.
- `pnpm -C tools/port port:geometry cynefin` compares the geometry of the fixtures and of the probes under `tools/port/geometry/probes/cynefin/` (markup and entities in labels, empty and spaced labels, zero padding, negative, fractional and zero seeds, labeled transitions in every direction, four items in confusion, no descriptions, string font sizes, an item size that is not a length). The harness sets `cynefin.seed` to the fixture's own or 1, as Mermaid's browser tests do, so the waves do not depend on the id; `--self` then draws the same waves under both of its ids.
- Rust specs: `cynefin_db_spec` (the reference's DB spec), `cynefin_grammar` (its integration spec and one test per quirk above) in `ferrigura-syntax`; `cynefin_boundary_spec` (the boundary waves and `resolveSeed`, with vectors recorded in Chromium) and `cynefin_render` (one test per renderer quirk) in `ferrigura-render`.

## Learnings

- **Seed 0 hashes the SVG id.** The default seed makes the waves depend on the id, which is `ref-<slug>` for the reference and the same for ferrigura in the geometry run. By the reference's code, a seed-0 diagram rendered through Mermaid's `run` gets a time-based id and differs from render to render.
- **The seeded generator is int32 arithmetic.** `|0`, `Math.imul`, `>>>` and the sum converted to int32 by `^` must be reproduced exactly; `-padding / 2` with padding 0 is `-0`, which JavaScript writes as `0`, and Rust's `Display` would write `-0`.
- **The shortest round-trip form of a double is needed** for the path digits; Rust's `Display` gives the same digits as JavaScript in the range the paths use.
- **Values keep the type the reference reads.** `"width":"wide"` is not rejected: the viewBox becomes `0 0 wide80 680` and every rect and path takes `NaN`. A string `domainFontSize` of `"20"` concatenates in `domainFontSize + 2` and gives a title of `202` px. An item font size that is not a length (`"14px"` becomes `14pxpx` in the style sheet) is dropped by the browser, so the badges are measured at the root's font size.
- **Entities are placeholders until the whole SVG is decoded.** `#amp;` is `ﬂ°amp¶ß` in the DB and shows as `&`; `#35;` shows as `#`. The decoding cannot tell where a placeholder came from, so an item holding `ﬂ°amp¶ß` literally shows `&` too.
- **The redux themes have no `cynefin` block,** so the reference draws the default theme's colors in both schemes: pastel backgrounds with `#131300` labels even in the dark one.
- **Chevrotain's first-match lexer decides the grammar's edges:** the names without a word end, the colon form of the header without its guard, and the title terminals that take leading blanks all follow from the order of the terminals, not from the grammar text.
