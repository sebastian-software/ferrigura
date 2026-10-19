# RADAR parser and DB: plan and decisions

The parser task of `radar-beta` (P3). The research, with its sources and probes, is in [NOTES.md](./NOTES.md); this page records what the port does, the decisions it took, and every quirk of the reference that it reproduces. The parser is `crates/ferrigura-syntax/src/radar/` (`scanner.rs`, `grammar.rs`, `db.rs`, and `mod.rs` for the steps of `Diagram.fromText`), the model is `crates/ferrigura-model/src/radar.rs`, and the exporter is `tools/port/export-radar.ts`.

The reference is the grammar `packages/parser/src/language/radar/radar.langium` with the shared terminals of `language/common/common.langium` (Langium 4.4.0), and the DB and parser `packages/mermaid/src/diagrams/radar/db.ts` and `parser.ts`. The decision to write the parser by hand is [0002](../decisions/0002-hand-written-parsers.md).

## 1. Scope

The parser task covers the grammar, the preprocessing it shares with the other types, the DB calls with their checks, and the model. The polar placement, the curves, the labels and the style sheet are the renderer stage (NOTES section 6), and the configuration is compared by `pnpm port:diff config`, as the P2 types are.

## 2. The lexer

Chevrotain takes the first terminal that matches, not the longest, in the order the lexer builds (NOTES section 2.2, read from the reference's own lexer):

1. The keywords, longest literal first: `radar-beta:` (no suffix), `radar-beta` (with the word-end suffix `(?=%%)|(?!\S)`), `showLegend`, `graticule`, `curve`, `ticks`, `axis`, `max`, `min`. Each word keyword has the ID as its `LONGER_ALT`, so `maximum` and `axisX` are IDs.
2. The punctuation `:`, `,`, `[`, `]`, `{`, `}`.
3. The terminals `GRATICULE` (`circle`, `polygon`), `BOOLEAN` (`true`, `false`), `ACC_DESCR`, `ACC_TITLE`, `TITLE`, `NUMBER`, `STRING`, `ID`, `NEWLINE`, and then the hidden `WHITESPACE`, `YAML`, `DIRECTIVE` and `SINGLE_LINE_COMMENT`.

The terminals without a `LONGER_ALT` take the prefix of any word they match. So `circlex` is `circle` and then the name `x`, `trueX` is `true` and then `X`, and `title1` is the title line and then the number `1`. The blanks before `title` and the accessibility keywords belong to their token, because those terminals come before the whitespace.

The scanner writes each terminal's regex as a function: `number_len` (the two forms of NUMBER with their lookaheads), `id_len`, `string_len`, `title_at`, `acc_descr_at`, `acc_title_at`, `comment_len`, and the shared `colon_form`, `directive_len` and `yaml_len` of `lexer.rs`.

## 3. The model

`RadarDiagram` holds the DB state after the parse, as NOTES section 4 proposes, in camelCase with `"version": 1`:

- `title`, `accTitle`, `accDescr`: the DB's values after the setters, `""` when absent.
- `axes`: `{ name, label }` in text order. The label is the name when the text gives none. A duplicate name is kept.
- `curves`: `{ name, label, entries }` in text order. The entries are in axis order for the named form and in text order for the number form, and their count is not checked against the axes.
- `options`: `showLegend`, `ticks`, `max`, `min`, `graticule`. `max` is the one nullable field: JSON `null` when the text gives none, and `0` when it gives zero.
- A number that is not finite is the sentinel string `"Infinity"` or `"NaN"`, as the canonical model says. The model's serialiser writes it, so `serde_json` never writes `null` for it.

The configuration, the theme and everything the renderer derives are left out (NOTES section 4).

## 4. The DB calls and their checks

The steps of `Diagram.fromText` run in the order of NOTES section 3.1: the front matter title is set first, then `populate` (`parser.ts`): the header texts (`accDescr`, `accTitle`, and the title only when it is non-empty), `setAxes`, `setCurves`, `setOptions`.

- `setCurves` needs the axes first. A curve whose first entry names an axis needs at least one axis (`Axes must be populated before curves for reference entries`), and each axis needs a first entry that names it (`Missing entry for axis <label>`). The message uses the label, as the reference's does. An entry that names no axis is dropped.
- `setOptions` keeps the last option of each name, and each option not given keeps its default (`showLegend` true, `ticks` 5, `max` absent, `min` 0, `graticule` `circle`). `ticks` above 32 is set to 32. The reference also logs a warning, which its default log level hides; the port prints nothing, as it has no logger.

Errors are rejections; their texts are ferrigura's own, with the position of the curve or the token (P2-PARSERS).

## 5. Sanitising and the value rules

- The header texts go through the shared setters: the title and the accessibility texts through DOMPurify's subset (`sanitize.rs`), `accTitle` with its leading whitespace removed, and `accDescr` with the whitespace after each line break removed. The labels of axes and curves are not sanitised.
- `TITLE`, `ACC_TITLE` and the single-line `ACC_DESCR` are trimmed and their blank runs collapsed, and the block form of `accDescr` loses the indentation of each line (`common.rs`, shared with architecture).
- `STRING` (a label) resolves its escapes as Langium's converter does, and `NUMBER` is `Number(text)`, so a value above the largest double (310 digits of ones, about 1.8e308 from 1.1e309) is `Infinity`.

## 6. Configuration

The radar section is compared by `pnpm port:diff config`: `radar` is no longer in `NOT_PORTED_SECTIONS` (`tools/port/diff-config.ts`), and its 203 inputs agree on the section. The config run's explicit inputs follow `undetectedCases` (the empty text has no header).

## 7. Reproduced quirks

Each one is pinned by a test in `crates/ferrigura-syntax/tests/radar_grammar.rs`, `radar_db_spec.rs` or `radar_spec.rs`, and by the explicit cases of `tools/port/cases.ts`.

Lexer and grammar (NOTES section 2.5 and 2.6):

- The header is `radar-beta`, `radar-beta:` or `radar-beta` with a colon. It must end at a blank, `%%` or the end of the text, so `radar-beta{` and `radar-betaX` are rejected, though the detector takes them.
- Statements need no separator. Items of one list are separated by commas, and a line break is allowed only where the grammar has a `NEWLINE` slot: inside `{ … }` around the entry commas, and at the end of a statement. `axis A,` and `ticks 5,` followed by a line break are rejected.
- A keyword is not a name when it stands alone (`axis max`, `curve axis{1}`), and it is a name when a longer word begins with it (`axis showLegend2`, `curve curvex`).
- `title`, `circle`, `true`, `false` and a number split a word: `axis title1`, `axis circle`, `axis trueX`, `axis 1A` are rejected.
- A word that begins with a number is the number first. An ID cannot end in `-`, and a dot, a minus sign or a non-ASCII letter is a lexer error.
- Entries are either all numbers or all named. `{}` is rejected. A number has no sign and no exponent, and `01` is two numbers.
- A YAML-like block in the body is hidden, and an unclosed one is a lexer error.
- `axis A title` gives the axis A and an empty title, as the title token takes the blank before it. `title<TAB>axis A` is the title `axis A` with no axis, as a tab after `title` takes the rest of the line.
- `radar-beta` may be a curve name (`curve radar-beta{1}`), since no blank follows it there, but not an axis name (`axis radar-beta` is rejected): the header keyword is taken when a blank, `%%` or the end of the text follows it.

DB:

- A title and the accessibility texts are sanitised, so the `&` of a title is held as `&amp;`, and `onerror` is removed (D18, D19, D20). The renderer shows the title's `&amp;` as written, as the reference does; the double encoding is kept (NOTES O5).
- The axis and curve labels are not sanitised, so `<b>x</b>` is kept as text (D21).
- An entity such as `#35;` is the placeholder `ﬂ°°35¶ß` in a label and in the title (D22, D23). The placeholder is turned back into the entity text by the renderer.
- The front matter title is kept unless the grammar gives a non-empty title. A blank last title line is a title of `''`, so the front matter's title shows through, and no title is held when there is none (D24 to D26).
- A curve that names its entries takes them in axis order, whatever the text order (D4), and the axes may come after the curve (D5).
- A named entry for an axis that is not declared is dropped (D8), and the first entry of an axis wins (D9).
- A number list is kept as it is, whatever its length (D10).
- Duplicate axes and duplicate curves are kept (D11).
- The last option of each name wins (D15), `ticks` keeps its fraction and zero (D16, D17), `max 0` and `min 0` are kept (D30), and `ticks` above 32 is 32 (D13, D14).
- A label of `""` is an empty label, not the name (`axis B[""]`): the default applies only when no label is written.

## 8. The specs and the differential

- `radar_spec.rs` ports the parser and DB tests of `radar.spec.ts` (9 pass). The renderer tests (`relativeRadius`, `closedRoundCurve`, and the two `draw` calls) are in `crates/ferrigura-render/tests/radar_render_spec.rs`, and the quirks of section 6.11 are pinned in `radar_quirks.rs`.
- `radar_grammar.rs` (27 tests) and `radar_db_spec.rs` (35 tests) pin the probes of NOTES sections 2.5 and 3.
- `pnpm port:diff radar` collects 203 inputs (the fixtures, the spec literals and the explicit cases, which include every text of the Rust tests and the probes): 135 accepted by the reference, 0 differences, and no named exception.
- `pnpm port:diff config` has 0 differences, with the radar section compared on its 203 inputs.

## 9. The edge cases of NOTES O9

These were the three cases that the notes derived but had not probed. The differential runs them through both sides (the exporter and the canonical example), and the results agree.

| Input                                       | Reference (parser and DB)                                                                                                 |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `max 1` then `min 5`                        | Accepted; `max` 1 and `min` 5 kept as written. The renderer divides by the negative range (its own, NOTES section 6.11)   |
| `min 2 max 2`                               | Accepted, both 2. The renderer's `NaN` is its own (NOTES section 6.11, item 1)                                            |
| A number of 400 digits (`min`, or an entry) | Accepted; the value is `Infinity`, so the model holds `"Infinity"`                                                        |
| 309 digits (about 1.1e308)                  | Accepted, finite (1.1111111111111112e+308)                                                                                |
| 310 digits                                  | Accepted; `Infinity`                                                                                                      |
| A curve of 125,000 entries                  | Accepted, all 125,000 entries reach the DB. The `RangeError` of `Math.max(...entries)` is in the renderer, not the parser |

The `RangeError` is not reached at the default configuration: the pipeline caps the text at `maxTextSize`, 50,000 characters by default, and only a larger cap reaches it (NOTES section 6.11, item 21). The cap is the pipeline's, not the parser's, and the exporter calls the parser directly, so the differential compares both sides without it. The notes name this case O9, while the brief names it O10; O10 in the notes is the `base` fallback of the harness.

## 10. Open questions of the notes, decided

- **O5, the double-encoded title:** reproduced. The harness compares the text exactly, and the reference shows `&amp;`.
- **O6, the `ticks` warning:** the port prints nothing, as the reference's warning is silent at its default log level and the port has no logger.
- **O7, the rejection policy:** only acceptance and the model are compared; the rejection texts are ferrigura's own.
- **O8, the detector:** `tools/port/reference.ts` registers the radar detector (the scaffold), so the exporter and the configuration run both detect the type.
- **O11, `max` as `null`:** accepted; `max` is `null` when the text gives none, as the canonical model's one nullable number.
- **O1 to O4, and the renderer's O9 items:** the theme fallback, `legendBoxSize`, `fontSize` and the colours are the renderer stage.
