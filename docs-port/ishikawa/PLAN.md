# ishikawa: parser, DB and model (P2)

The parser, the DB and (section 10) the renderer of the ishikawa (fishbone) diagram: `packages/mermaid/src/diagrams/ishikawa/parser/ishikawa.jison` and `ishikawaDb.ts`, ported by hand in `crates/ferrigura-syntax/src/ishikawa/` (decision 0002). The model is `crates/ferrigura-model/src/ishikawa.rs`, the exporter `tools/port/export-ishikawa.ts`. The research and every rule with its reference line are in [NOTES.md](./NOTES.md); this file records what the port does, the quirks it reproduces, and the corrections the parser trace made to the notes. The renderer is section 10.

Status: the differential (`pnpm port:diff ishikawa`) shows no difference on 81 inputs (74 accepted by the reference), the three spec files pass (3, 26 and 11 tests), and `pnpm port:diff config` compares the ishikawa section with no difference.

## 1. Pipeline

`ishikawa::parse` runs the steps of `Diagram.fromText`, as the journey port does: `prepare` (CRLF and CR to LF, the quoting of HTML attributes, the front matter and its title), `remove_directives`, `cleanup_comments`, `encode_entities`, and a line break. The scanner reads that text. The grammar then calls the DB as each statement is read, in document order, and the DB's state becomes the model.

The exporter does the same through the reference's own `Diagram.fromText` (`preprocessDiagram`, then `Diagram.fromText(code, { title })`), so each input gets the reference's own fresh `IshikawaDB`, the common DB is cleared, and a non-ishikawa detection is a rejection.

## 2. Scanner and grammar

The scanner tries the rules of the Jison lexer in the file's order, and the first match wins:

| Rule (`ishikawa.jison`)                                   | Token     | Port                                                                                           |
| --------------------------------------------------------- | --------- | ---------------------------------------------------------------------------------------------- |
| `\s*%%.*` (6)                                             | SPACELINE | `comment_len`: JavaScript white space, `%%`, then the rest of the line up to `.`'s terminators |
| `ishikawa-beta\b` (7), `ishikawa\b` (8), case-insensitive | ISHIKAWA  | `header_len`: the next character must not be a word character (`ASCII`, `_`, digits)           |
| `[\s]+[\n]` (9)                                           | SPACELINE | `space_line_len`: up to the last line break that is not the first character                    |
| `[\n]+` (10)                                              | NL        | `next_rule`: a run of line breaks                                                              |
| `[\s]+` (11)                                              | SPACELIST | the count of characters, which is the level's length (all white space is in the BMP)           |
| `[^\n]+` (12)                                             | TEXT      | the rest of the line, blanks included                                                          |
| `<<EOF>>` (13)                                            | EOF       | the end of the text, once                                                                      |

The grammar is `start: spaceLines? ISHIKAWA NL? document`, where `document` is one or more `statement stop`: `statement` is `SPACELIST TEXT` (a cause with its indentation), `TEXT` (the head, or a cause at level 0), `SPACELINE` or `SPACELIST` (no DB call), and `stop` is one of NL, EOF or SPACELINE followed by any NL or EOF. The grammar has no start condition, so the scanner has one state. The parser does not check the header: the text reaches it after the cleanup, which removes the leading white space.

## 3. The DB

`IshikawaDb` (`db.rs`) keeps the title, an arena of nodes, the stack of open nodes with their levels, and the base level. It follows `ishikawaDb.ts` (lines 42-71):

- The first node is the head. It creates the root and the stack `[(0, root)]`, and sets the title to its own text, sanitised again (the reference's `setDiagramTitle` sanitises it a second time).
- A later node takes `baseLevel ??= rawLevel` on the first one, so the first cause fixes the base; `level = rawLevel − baseLevel + 1`, and a level of 0 or less is 1; the stack pops while its top is at least `level` (never the head); the parent is the top, and the node is appended to its children and pushed.
- The text is sanitised once (`sanitize_text`, the flowchart's DOMPurify subset). Entity placeholders are kept as the DB keeps them.

## 4. Quirks reproduced

Each row is a probe through the reference (`out-parse.json` in the scratchpad, and the differential), and a Rust test in `crates/ferrigura-syntax/tests/ishikawa_grammar.rs` with the same number. The quirk numbers are those of `NOTES.md` section 2.3.

| #   | Behaviour                                                                                           | Test                                             |
| --- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| I1  | A bare header is an error (`ISHIKAWA NL EOF`: the grammar needs a statement)                        | `i1_a_bare_header_is_an_error`                   |
| I2  | A header followed by a line break is accepted, with no root and an empty title (section 4.1)        | `i2_a_header_and_a_line_break_have_no_root`      |
| I3  | `ishikawaEffect` is text, not the header: an error in both the reference's detector and the grammar | `i3_a_header_glued_to_a_word_is_not_the_header`  |
| I4  | The header line's own text is the root                                                              | `i4_the_header_line_text_is_the_root`            |
| I5  | `ishikawa-betafoo` is the header, followed by the text `-betafoo`                                   | `i5_a_glued_beta_keyword_takes_the_plain_header` |
| I6  | Blanks after the header are an error                                                                | `i6_blanks_after_the_header_are_an_error`        |
| I7  | The header is case-insensitive (ASCII only, as the lexer's `i` flag folds)                          | `i7_the_header_is_case_insensitive`              |
| I8  | A second header is an error                                                                         | `i8_a_second_header_is_an_error`                 |
| I9  | Blank lines and whitespace before the header are ignored (`cleanup` trims the start)                | `i9_blank_lines_before_the_header_are_ignored`   |
| I10 | `%%` comment lines, indented or not, are ignored                                                    | `i10_comment_lines_are_ignored`                  |
| I11 | `%%` inside a text is text                                                                          | `i11_a_percent_pair_inside_a_text_is_text`       |
| I12 | Each tab is one level unit                                                                          | `i12_each_tab_is_one_level_unit`                 |
| I13 | The first cause sets the base level; a dedent to 0 or less is level 1                               | `i13_the_first_cause_sets_the_base_level`        |
| I14 | Deeper lines are children; a return to the base level is a sibling                                  | `i14_nested_causes_follow_the_indentation`       |
| I15 | The first statement after the header is the root, whatever its indentation                          | `i15_an_indented_first_statement_is_the_root`    |
| I16 | `accTitle:`, `accDescr:` and `title` lines are causes; the header texts stay empty                  | `i16_accessibility_and_title_lines_are_causes`   |
| I17 | `<br>` and `<br/>` are kept as `<br>`                                                               | `i17_line_break_markup_is_kept_as_written`       |
| I18 | A text with `<` is sanitised (`&` becomes `&amp;`); a text without `<` is unchanged                 | `i18_a_text_with_markup_is_sanitised`            |
| I19 | `;` is text                                                                                         | `i19_a_semicolon_is_text`                        |
| I20 | CRLF line endings are accepted                                                                      | `i20_crlf_line_endings_are_accepted`             |
| I21 | A line of only a no-break space is blank                                                            | `i21_a_no_break_space_line_is_blank`             |
| I22 | Causes at the same indentation as the one before are its children                                   | `i22_the_same_indentation_gives_a_child`         |
| I23 | A blank line between causes is ignored                                                              | `i23_a_blank_line_between_causes_is_ignored`     |
| I24 | No line break at the end is accepted                                                                | `i24_no_final_line_break_is_accepted`            |
| I25 | Front matter before the header is removed                                                           | `i25_front_matter_is_removed_before_the_header`  |
| I26 | A directive before the header is removed                                                            | `i26_a_directive_before_the_header_is_removed`   |

The DB quirks, each in `crates/ferrigura-syntax/tests/ishikawa_db_spec.rs`:

- **D1 Title.** The title is the head's text, sanitised twice. A head sets the title even when its text sanitises to nothing, so an empty head overrides the front matter title (`should_set_an_empty_title_for_a_head_that_sanitises_to_nothing`).
- **D2 Front matter title without a head.** `Diagram.fromText` sets the title from the front matter before the parse, and nothing replaces it when no statement was read (`should_keep_the_front_matter_title_when_there_is_no_head`).
- **D3 Empty head.** `if (!this.root)` tests the node, not its text, so a head whose text sanitises to `""` is a root with an empty text.
- **D4 Fresh DB.** `Diagram.fromText` reads the `db` getter, which returns a new `IshikawaDB` for each diagram, and `db.clear()` resets the common DB. Five sequential `Diagram.fromText` calls gave five instances (`parser.parser.yy` is the same instance each time), with no title carried from one to the next (`should_start_each_parse_from_an_empty_db`).
- **D5 Levels.** `should_place_causes_by_their_level_with_pops`, `should_take_the_base_level_from_the_first_cause`, `should_keep_the_order_of_the_causes`.
- **D6 Sanitising.** Head and causes are sanitised (`should_sanitise_the_head_and_the_causes`: `<script>` with its content is removed).
- **D7 Model.** `root` is `null` when no statement was read (`should_serialise_a_missing_root_as_null`); `accTitle` and `accDescr` are always empty, since no rule sets them.

Two ways the reference rejects a text were checked. `%%{ bad\nishikawa\nP` is rejected by the reference's detector (its loose directive pattern takes the header with the directive), and by the port's grammar for the same reason, since `remove_directives` is the same pattern. `%%{init}\nishikawa` is rejected the same way. The differential holds both as agreements, and the configuration run lists them in `undetectedCases`.

## 5. Corrections to the notes

- **I2 (`NOTES.md` section 2.3).** The notes say the parser sees `ishikawa NL EOF`. It sees `ISHIKAWA SPACELINE EOF`: the input's own line break and the one the reference adds make `\n\n`, which the rule `[\s]+[\n]` (9) takes as one SPACELINE. The grammar accepts a SPACELINE as a statement, which calls no DB method, and then the stop. The probe (`tokens.mts`, scratchpad) printed these tokens for `ishikawa\n` and for `ishikawa`, which is `ISHIKAWA NL EOF` (I1). The scanner unit test `a_header_followed_by_two_line_breaks_is_a_space_line` pins the mechanism.
- **Diagram loading (`NOTES.md` section 3.4).** The trace is closed: `Diagram.fromText` destructures `db` from the definition, so each diagram gets a new `IshikawaDB` (`Diagram.ts`, `getDiagram(type)`), and `parser.parser.yy` is set to it. The probe found no leak of the common DB's title between diagrams.

## 6. Configuration

- **`ishikawa.useMaxWidth`.** The schema's own default is `false` (`IshikawaDiagramConfig`, `config.schema.yaml` 1234-1236), against the base's `true` (`BaseDiagramConfig`, line 470). The reference's effective value is `true` for every ishikawa input without an explicit `useMaxWidth`. The defaults generator (`.build/jsonSchema.ts`) applies the `allOf` base first, and the repository's ajv gives `true` for this section (checked). The port takes the effective value in `crates/ferrigura-model/src/config/ts_defaults.json`, the file of the reference's defaults that its schema does not hold (as `treemap.useMaxWidth` is). The configuration differential compares the ishikawa section with no difference.
- **Sankey, not traced.** The sankey section has the same shape (`allOf` the base, own `useMaxWidth: false`) and the reference's effective value is `false`, though the same ajv merge gives `true`. The source of that value was not found. The sankey port must check its own section; it is not compared now (`UNPORTED_SECTIONS`, `NOT_PORTED_SECTIONS`).

## 7. Not reproduced

- **Configuration-dependent sanitising.** The DB sanitises with the default configuration (strict). A directive that sets `securityLevel` or `htmlLabels` changes the reference's first pass (`sanitizeMore`). The exporter, like the journey's, runs without directives, and so does the port's DB; the flowchart and journey DBs have the same limit.
- **Rendering.** The layout, the text measurement and `look: handDrawn` are the renderer, which is section 10 (`NOTES.md` sections 6 and 8).

## 8. Files

Own: `crates/ferrigura-model/src/ishikawa.rs`, `crates/ferrigura-syntax/src/ishikawa/` (`mod.rs`, `scanner.rs` with its unit tests, `db.rs`), `crates/ferrigura-syntax/tests/ishikawa_spec.rs`, `ishikawa_grammar.rs`, `ishikawa_db_spec.rs`, `tools/port/export-ishikawa.ts`, this file.

Shared, and small: `tools/port/cases.ts` (the ishikawa entries of `explicitCases` and `undetectedCases`), `tools/port/status.ts` (the suites), `tools/port/diff-config.ts` (the ishikawa section is compared: it stays in `UNPORTED_SECTIONS`, which removes it for other types, and leaves `NOT_PORTED_SECTIONS`), `crates/ferrigura-model/src/config/ts_defaults.json` (`ishikawa.useMaxWidth`), `docs-port/STATUS.md` (the ishikawa row), `docs-port/canonical-model.md` (the ishikawa section).

## 9. Open points

1. The sankey `useMaxWidth` value (section 6), for the sankey task.
2. The renderer is done (section 10). The handDrawn look is drawn classic, and roughjs is not ported (`NOTES.md` section 8.3, question 1).

## 10. Renderer

The renderer is `ishikawaRenderer.ts` (`draw`) and `ishikawaStyles.ts`. Its files: `crates/ferrigura-render/src/ishikawa/` (`mod.rs` for the document, the spine and the head; `bones.rs` for a cause, its label and its descendants; `text.rs` for the label boxes and the wrapping; `style.rs` for the style sheet). The research and every rule with its line are in [NOTES.md](./NOTES.md) section 6.

Checks, at the time of writing:

- `pnpm port:geometry ishikawa`: 28 of 28 within tolerance, 0.1% of the 0.05 px tolerance at worst. The 12 comparable fixtures and 16 probes under `tools/port/geometry/probes/ishikawa/` (many causes, deep nesting, a long head, long and wrapped texts, `<br>`, a wide cause, unbalanced sides, dark theme, padding and width, a header with no statement, empty lines and empty causes). `pr-check/6` (handDrawn) is excluded.
- `crates/ferrigura-render/tests/ishikawa_render.rs`: the reference's numbers for fixture one (the viewBox, the head's shape, the spine) and the structure of the markup; the unit tests of `text.rs` and `mod.rs` cover the wrapping, the baselines, the empty lines and the side lengths.

Quirks reproduced, numbered here (the code comments name the reference's lines):

1. `I-R1` The head is wrapped at `max(6, floor(110 / (fontSize * 0.6)))` characters, which is 11 at the configured 16 px, and it is set in the bold face: the weight 600 maps to 700 (`ishikawaRenderer.ts` 173, `ishikawaStyles.ts` 39).
2. `I-R2` The layout spaces the lines and the sub-causes with the configured font size (16), and draws the labels at the theme's size (14 in the redux themes). The two sizes are kept apart (`Context::font_size`, `Context::label_size`).
3. `I-R3` A cause's label is not wrapped; a sub-cause is wrapped at 15 characters. A `<br>` splits a label in both (`splitLines`).
4. `I-R4` The root's box takes the tight bounds of the head's curve: the extreme of the quadratic is 1.2 times the width, not the control point's 2.4 times (`NOTES.md` 6.3).
5. `I-R5` The text's box comes from the stylesheet's baseline: the alphabetic ascent (the rounded one), the middle baseline at the x-height's half, and the hanging baseline at 0.8 of the rounded ascent, each on the 1/64 px grid. `dominant-baseline: baseline`, which the `up` labels declare, is not a keyword, so Chromium uses the alphabetic baseline; the port writes the declaration as the reference does.
6. `I-R6` The spine starts at the leftmost text of the last pair, not of all the pairs: each pair's text sets `spineX` for the next (`ishikawaRenderer.ts` 136-139).
7. `I-R7` The upper side reads its children in reverse at every depth, so the upper side mirrors the lower one (`flattenTree`, the `direction` argument).
8. `I-R8` A header with no statement draws no `g.ishikawa` and no viewBox: the reference returns before the layout (`if (!root) return`). The probe `p14` checks it.
9. `I-R9` `ishikawa.useMaxWidth` is true by default (section 6.7), so the root takes `width="100%"` and a `max-width`.
10. `I-R10` `look: handDrawn` draws the classic geometry, as the other ported types do: roughjs is not ported (section 8.3, question 1).
11. `I-R11` A line without characters makes no `tspan` fragment: it takes no line step (its `dy` is never applied, so the next line follows the last one with characters) and adds no box. A label of empty lines only has the zero box at the origin, which is why an empty head is 40 px tall (`probes/ishikawa/p15`, `p16`).

Not covered:

- `look: handDrawn` is drawn classic, as above; its fixture is excluded by the harness.
- The label size of the default theme (16 px). Only the redux themes ship, and the harness forces one of them, so the 16 px labels are not compared. The formulas take the size from the theme's `fontSize` and are the same at any size; the 14 px values are the measured ones.
- `accTitle` and `accDescr`: the reference's renderer does not write them, and neither does the port.
