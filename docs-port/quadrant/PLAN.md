# quadrantChart: plan

Status: ported (the parser, the DB and the model in sections 1 to 11; the builder and the renderer in section 12). Reference: this worktree's `packages/mermaid/src/diagrams/quadrant-chart/` (`parser/quadrant.jison`, `quadrantDb.ts`). Research and probes are in [NOTES.md](./NOTES.md); this page says what the port does, which quirks it keeps and which it does not. Quirk ids are the notes' ids: `Q1`–`Q38` for the grammar and the DB (NOTES 2.4), `C1`–`C5` for the coordinates (NOTES 2.3), and `L1`–`L6` for the lexer (NOTES 2.1, the rows the notes do not number).

Scope: the parser, the DB and the model (sections 1 to 11), and the chart builder and the renderer (section 12). The theme is the theme port's (`ferrigura-model`), and the configuration is compared by `pnpm port:diff config`.

## 1. Sources

| File                                                   | Used for                                                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `diagrams/quadrant-chart/parser/quadrant.jison` (1-77) | The rule table: the generated rules and the `conditions` of each state (section 3)               |
| same (81-189)                                          | The grammar (section 4)                                                                          |
| `diagrams/quadrant-chart/quadrantDb.ts` (1-174)        | The DB: the setters, `parseStyles`, `addPoint`, `addClass`, `clear`                              |
| `diagrams/quadrant-chart/quadrantBuilder.ts` (201-229) | `addPoints` (prepends), `addClass` (a `Map`), `clear`: the state the model reads                 |
| `diagrams/quadrant-chart/utils.ts`                     | The validators of the style values (`validateNumber`, `validateHexCode`, `validateSizeInPixels`) |
| `diagrams/common/commonDb.ts` (10-30)                  | `setDiagramTitle`, `setAccTitle`, `setAccDescription` (sanitising and the whitespace rules)      |
| `diagrams/common/common.ts` (56-82)                    | `sanitizeText`, `sanitizeMore`                                                                   |
| `Diagram.ts`, `preprocess.ts`                          | Order of the front matter title, `encodeEntities` and the line break                             |

The Rust code: `crates/ferrigura-syntax/src/quadrant_chart/` (`mod.rs` the entry point and the grammar, `scanner.rs` the lexer, `db.rs` the DB and `parseStyles`), the model in `crates/ferrigura-model/src/quadrant_chart.rs`.

## 2. Text before the lexer

The steps of `Diagram.fromText` after `preprocessDiagram`, in the reference's order: CRLF and CR to LF, `="x"` to `='x'` in tags, front matter removed and its `title` read, `%%{ … }%%` directives removed, comment lines removed, `encodeEntities`, one `\n` (shared, `prepare`, `remove_directives`, `cleanup_comments`, `encode_entities`). The front matter title is set through `setDiagramTitle` before the parse, and the grammar's title replaces it, even when blank (section 5).

The configuration of the text is read as the render reads it. The exporter applies it (`export-quadrantChart.ts`, as the xychart exporter does), since the sanitiser reads `securityLevel` and `htmlLabels` from it. Section 6 says what the port takes from it.

## 3. Scanner

Jison's `next()` takes the first rule of the current state that matches at the position, not the longest (`options.flex` is off). The state stack starts with `INITIAL`, and `popState` never empties it. Every state but `INITIAL` is exclusive (`%x`), so a rule without a start condition applies in `INITIAL` only: the generated `conditions` list 39 rules for `INITIAL` and only the rules of their own state for the others.

| Generated rules | Token or action                                                                                              | State                             |
| --------------- | ------------------------------------------------------------------------------------------------------------ | --------------------------------- |
| 0, 1, 3         | comments: `%%` not followed by `{`; one character other than `}` then `%%`; `%%`                             | `INITIAL`                         |
| 2               | NEWLINE, one or more `\n` or `\r`                                                                            | `INITIAL`                         |
| 4, 20           | `title\b`, `classDef\b` (the word boundary is Jison's, for a literal ending in a word)                       | `INITIAL`                         |
| 6, 8, 10        | `accTitle\s*:\s*`, `accDescr\s*:\s*` enter their value state; `accDescr\s*\{\s*` enters the block (no token) | `INITIAL`                         |
| 5, 7, 9         | the rest of the line: the value of `title`, `accTitle` or `accDescr`                                         | `title`, `acc_title`, `acc_descr` |
| 11, 12          | `}` leaves the block (no token); `[^\}]*` is its text, which may be empty                                    | `acc_descr_multiline`             |
| 13-19, 34       | ` *x-axis *`, ` *y-axis *`, ` *--+> *` (the delimiter), ` *quadrant-N *`, ` *quadrantChart *`                | `INITIAL`                         |
| 21, 23          | `"` followed by a backtick enters a Markdown string; a backtick then `"` leaves it (no token)                | `INITIAL`, `md_string`            |
| 22              | `[^`"]+`, the Markdown text                                                                                  | `md_string`                       |
| 24, 25, 26      | `"` enters the quoted string; `"` leaves it (no token); `[^"]*` is its text, which may be empty              | `INITIAL`, `string`               |
| 27, 28          | `:::` enters the class name (no token); `^\w+` is the name                                                   | `INITIAL`, `class_name`           |
| 29, 30, 31      | `\s*:\s*\[\s*` starts a point; the x coordinate; `\s*\] *` ends the point (no token)                         | `INITIAL`, `point_start`          |
| 32, 33          | `\s*,\s*` (no token) starts the y state; the y coordinate                                                    | `point_x`, `point_y`              |
| 35-48           | `[A-Za-z]+`, `[^\x00-\x7F]+`, the one-character tokens, `[0-9]+`                                             | `INITIAL`                         |
| 49              | one white space character (`\s`)                                                                             | `INITIAL`                         |
| 50, 51          | `;`, and the characters of `[!"#$%&'*+,-.`?\\\_/]`                                                           | `INITIAL`                         |
| 52              | `$`, the end of the input, as the grammar's `EOF`                                                            | `INITIAL`                         |

The reproduced lexer quirks:

- **L1** The keywords `title` and `classDef` have a word boundary, so `titles` is a word. `quadrant-1`, `x-axis`, `accTitle` and `quadrantChart` have none: `quadrant-1A` is `quadrant-1` then `A` (a quadrant text of `A`), and `quadrantChartX` is the header then the word `X`, which is an error on its own line.
- **L2** Keywords are case-insensitive over ASCII only, as the `i` flag without `u` is.
- **L3** A `quadrant-N` or `x-axis` inside a text starts its token: `Point quadrant-1` and `P-x-axis` are errors (NOTES 2.1).
- **L4** Rule 1 takes one character before `%%`, a line break included, so `P1%% c` loses the `1` and a line break before a comment is removed with the comment.
- **L5** Rule 36 precedes rule 49, so a white space beyond ASCII (NBSP, U+2028, U+3000, the BOM) is `UNICODE_TEXT` in `INITIAL`. It is white space only in the `\s*` of rules 6, 8, 10, 29, 31 and 32.
- **L6** In a coordinate, `.` is any character but a line terminator, and it is one UTF-16 unit: `0😀5` takes `0` alone, and `0 5` and `0\t5` are one coordinate that reads as NaN (C3).

The done flag: once the input is empty, the next call reads the rules of the current state at the end of the input (rule 52 in `INITIAL`, which gives `EOF`), and the call after that returns Jison's raw end symbol, which the grammar does not accept. A character no rule takes (`<`, `(`, `{`, `]` in `INITIAL`, a lone `@`) is a lexical error. The scan stops there (Q-probes in the differential).

## 4. Grammar and DB calls

Jison reports no conflicts for the grammar (LALR(1)), so a recursive descent with one token of lookahead accepts the same texts. Its shape, from the productions (`quadrant.jison` 110-187):

```
start      : (eol | SPACE)* QUADRANT line*
line       : SPACE* statement? eol
statement  : classDef | axis | quadrantDetail | points
           | TITLE TITLE_VALUE | ACC_TITLE ACC_TITLE_VALUE | ACC_DESCR ACC_DESCR_VALUE
           | ACC_DESCR_ML_VALUE
text       : (alphaNum | STR | MD_STR) (alphaNum | SPACE | MINUS)*
points     : text [CLASS_NAME] POINT_START POINT_X POINT_Y [style (COMMA style)*]
classDef   : CLASSDEF SPACE idString SPACE style (COMMA style)*
axis       : X_AXIS text [DELIM text?] | Y_AXIS text [DELIM text?]
```

- `section` (statement, 135) names a symbol that is no token, so it never matches (Q1). The terminals `DOWN`, `DEFAULT`, `NODE_STRING`, `UNIT`, `STYLE` and `PCT` are never produced.
- A `style` is one or more style components (`ALPHA`, `NUM`, `:`, SPACE, `#`, `-`), trimmed, and the parts of a `stylesOpt` are trimmed too (the DB trims again). A trailing comma, or a style that is blank, is an error at the DB (Q27, and the blank style of a tab in Q-probes).
- `axis` with a delimiter and no text after it appends `" ⟶ "` to the left or bottom text (line 147, 150). The DB trims it, so `x-axis Low -->` gives `Low ⟶` (Q8).
- A text is a run of alphanumeric tokens with spaces and hyphens. A quoted or Markdown string may only start a text (`"a"b` is `ab`, `a"b"` is an error).

The DB calls, in the order of the reductions: `setXAxisLeftText` then `setXAxisRightText`, `setYAxisBottomText` then `setYAxisTopText`, `setQuadrantNText`, `setDiagramTitle`, `setAccTitle`, `setAccDescription`, `addClass` (on the reduction of the line) and `addPoint`. A reduction that the DB rejects aborts the parse, so the model of a rejected text is never read.

## 5. DB

| Function                              | Behaviour                                                                                                                                          |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `setQuadrantNText`, the axis setters  | `sanitizeText(text.trim())`; the last one wins (Q5, Q11)                                                                                           |
| `setDiagramTitle`                     | `sanitizeText(trimmed)`; the last grammar title wins even when blank, so `title` alone replaces the front matter title (Q5)                        |
| `setAccTitle`                         | `sanitizeText(trimmed)`, then the blanks at its start removed                                                                                      |
| `setAccDescription`                   | `sanitizeText(trimmed)`, then each line break and the blanks after it reduced to one line break                                                    |
| `addPoint(text, class, x, y, styles)` | `parseStyles(styles)` first; the point's text sanitised and trimmed; `x` and `y` are `Number(text)` (C1-C5); the point goes in front of the others |
| `addClass(name, styles)`              | `parseStyles(styles)`; a `Map` keyed by the name, so a second definition replaces the styles and keeps the place (Q17)                             |

**Point order.** `addPoints` prepends (`[...points, ...this.data.points]`, builder 214), so the DB lists the points in the reverse of the source order (`A`, `B`, `C` come out as `C`, `B`, `A`). The model keeps that order.

**Styles** (`parseStyles`, DB 64-93). Each text is trimmed and split at `\s*:\s*`, and the first two pieces are the key and the value; any further piece is ignored (Q22 through Q26, and `color: #fff:x` is valid). The keys and their checks:

- `radius`: `^\d+$`, then `parseInt`, so `007` is 7 and `0` is kept (Q20);
- `color`, `stroke-color`: `#?` and three or six hex digits, written as given (Q22, Q23);
- `stroke-width`: `^\d+px$`, written as given (Q24);
- any other key: `style named K is not supported.` (Q25);
- a key without a value fails its check, as `undefined` does in the reference (Q19).

A value that fails its check, or an unknown key, rejects the text. The DB spreads only the keys that were set, so an absent style is absent in the model.

**Clear.** The reference's `clear()` resets the builder (texts, points, classes) and the common DB. The exporter clears before each input and the port starts each parse from an empty DB.

## 6. Sanitising and configuration

`textSanitizer(text)` is `sanitizeText(text.trim(), getConfig())` (DB 21-23). `sanitizeText` runs `sanitizeMore` and then DOMPurify with `FORBID_TAGS: ['style']`. `sanitizeMore` depends on `htmlLabels` and `securityLevel`: under `antiscript`, `strict` and `sandbox` it removes script; under `loose` it does nothing; under any other value it escapes `<`, `>` and `=`.

The port's `sanitize_text` is the DOMPurify pass that the default configuration (`strict`, `htmlLabels` on) reaches. The render's configuration is applied to the reference in the exporter, and 22 probes with directives (`securityLevel` set to `loose`, `strict`, `sandbox` and an unknown value, `htmlLabels: false`, `<style>`, `<br>`, `=` in quoted texts) agree with the port. Section 11, item 2, records what is left.

The text of a header is sanitised before its own rule: the title and the accessibility texts go through the common DB's `sanitizeText` (section 5).

## 7. Model

`QuadrantChartDiagram`, `version` 1, camelCase keys (`canonical-model.md`, section QuadrantChart):

- `title`, `accTitle`, `accDescr`: the common DB's values;
- `quadrant1Text` … `quadrant4Text`, `xAxisLeftText`, `xAxisRightText`, `yAxisBottomText`, `yAxisTopText`: the texts the DB holds, `""` when unset;
- `points`: in the DB's order; each has `text`, `className` (`""` when none), `x` and `y`, and the styles that were set (`radius` a number, `color` and `strokeColor` as written, `strokeWidth` as written);
- `classes`: a list of `[name, styles]` pairs in the order of the first definition.

**Coordinates (C1-C5).** `x` and `y` are `Number(text)`, the value d3 reads, since the DB keeps the grammar's text and the builder's `+` converts it:

- **C1** `0.5`, `1`, `0.00`, `0.12345` are the numbers they read as.
- **C2** `0x5` is 5 (`Number("0x5")`), and `0e5` is 0. The unescaped `.` of the rule takes any character, so `0b1` is 1 and `0o7` is 7.
- **C3** `0,5`, `0 5`, `0a5` and `0-5` are NaN. The model writes `"NaN"` (the sentinel of `canonical-model.md`).
- **C4** `0.5.5`, `1.5` and `1.0` are lexical errors, as the reference rejects them.
- **C5** A line break inside a coordinate ends it: `[0\n5, 0.5]` is an error, since `.` excludes the line break.

**Entities.** The DB holds the placeholders of `encodeEntities` (`#35;` is `ﬂ°°35¶ß`), and the model keeps them, as the DB does. The decoded text is the whole-SVG pass of the renderer (`mermaidAPI.ts` 359).

Left out: the builder's layout (points, rectangles, labels), the configuration (`pnpm port:diff config`), the theme variables (`pnpm port:diff theme`) and `setWidth`/`setHeight`, which the renderer calls.

## 8. Quirks

| Id    | Status               | Where                                                                                                     |
| ----- | -------------------- | --------------------------------------------------------------------------------------------------------- |
| Q1    | reproduced           | `section` is no token; the statement is an error                                                          |
| Q2    | reproduced           | the grammar takes `QUADRANTCHART`; the detector does not, so the input is `undetectedCases` in `cases.ts` |
| Q3    | reproduced           | a second header is a statement, and the grammar rejects it                                                |
| Q4    | reproduced           | `title: Hello` keeps the colon: the value is the rest of the line                                         |
| Q5    | reproduced           | the last title wins, trimmed; a blank last title overrides the front matter                               |
| Q6    | reproduced           | one axis text is the left or bottom text                                                                  |
| Q7    | reproduced           | any `-->`-like run is the delimiter                                                                       |
| Q8    | reproduced           | a delimiter with no text after it appends `" ⟶ "` and the DB trims it                                     |
| Q9    | reproduced           | an axis needs its first text: `x-axis --> High` is an error                                               |
| Q10   | reproduced           | the first text of `y-axis` is the bottom one                                                              |
| Q11   | reproduced           | the last quadrant text wins                                                                               |
| Q12   | reproduced           | a point with no text is an error                                                                          |
| Q13   | reproduced           | the blanks around the colon, and inside the brackets, are free                                            |
| Q14   | reproduced           | a class name without a `classDef` is accepted with no styles                                              |
| Q15   | reproduced           | a class defined after its point applies: the lookup is at build time, so the model keeps the name only    |
| Q16   | reproduced (builder) | a point's own style wins over its class at build time; the model keeps both                               |
| Q17   | reproduced           | a second `classDef` of a name replaces the styles, in the first place                                     |
| Q18   | reproduced           | `radius: 10px` is an error                                                                                |
| Q19   | reproduced           | a negative or fractional radius, and a missing value, are errors                                          |
| Q20   | reproduced           | `radius: 0` is kept                                                                                       |
| Q21   | reproduced           | `color: red` and `stroke-color: blue` are errors (hex only)                                               |
| Q22   | reproduced           | `color: abcdef` (no `#`) is written as given                                                              |
| Q23   | reproduced           | `color: #abc` is a valid hex code                                                                         |
| Q24   | reproduced           | `stroke-width` needs `px`; `2px` is kept as written                                                       |
| Q25   | reproduced           | `opacity` is `style named opacity is not supported.`                                                      |
| Q26   | reproduced           | styles separated by a space and no comma: `#ff0000 radius` is not a hex code                              |
| Q27   | reproduced           | a trailing comma after the styles is an error                                                             |
| Q28   | reproduced           | `%% trailing` after a point is dropped                                                                    |
| Q29   | reproduced           | a Markdown text keeps its text and drops the markdown type (the DB ignores it)                            |
| Q30   | reproduced           | `Point & Q` is kept (the sanitiser's fast path)                                                           |
| Q31   | reproduced           | `&amp;` is three tokens with a `;`, and the `;` is an end of line                                         |
| Q32   | reproduced           | a `;` in a label ends the line                                                                            |
| Q33   | reproduced           | `#35;` is the placeholder in the model (section 7)                                                        |
| Q34   | reproduced           | CJK, emoji and other characters beyond ASCII are text                                                     |
| Q35   | reproduced           | CRLF is normalised before the lexer                                                                       |
| Q36   | reproduced           | a bare header, or one with blank lines, is accepted with no texts                                         |
| Q37   | reproduced           | a `classDef` with no name or no styles is an error                                                        |
| Q38   | reproduced           | a name that starts with a digit is text                                                                   |
| C1-C5 | reproduced           | section 7                                                                                                 |
| L1-L6 | reproduced           | section 3                                                                                                 |

## 9. Specs

| Reference spec                  | Rust file                                                     | Tests | Notes                                                                                                                                                                                                                                       |
| ------------------------------- | ------------------------------------------------------------- | ----: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `parser/quadrant.jison.spec.ts` | `crates/ferrigura-syntax/tests/quadrant_chart_parser_spec.rs` |    24 | 14 at the root, 10 under the Unicode block. The mock DB calls become model assertions. The constructor test rewrites its `fill` style (the reference's DB rejects it) and keeps the class-name protocol with `color`. Passing; none ignored |
| `quadrantDb.spec.ts`            | `crates/ferrigura-syntax/tests/quadrant_chart_db_spec.rs`     |     5 | `parseStyles` is driven through a `classDef` line or a point. The empty-styles test is a point with no styles. The `getConfig` spy test is ignored (section 11, item 2). 4 passing, 1 ignored                                               |

The chart builder and the renderer have no spec in this folder (NOTES 5), so no test is ignored for a layout reason.

## 10. Differential

`tools/port/export-quadrantChart.ts` runs the reference's parser and DB on each input and writes the model. A line the loader appends to `quadrantDb.ts` (`exposeBuilder`, the same device as `export-kanban.ts`) returns the builder, whose `data` and `classes` are the state the model reads. The reference's source is not changed.

`cases.ts` adds the probes of NOTES 2.1 to 2.4 (Q1 to Q38, the coordinate table, the lexer rows), the Unicode and Markdown forms, and the front matter title cases, as one input each. The header-less probes and the uppercase header are in `undetectedCases` (the reference's detector takes none of them).

`pnpm port:diff quadrantChart`: 340 inputs, 224 accepted by the reference, 0 differences, 0 named exceptions. `pnpm port:diff config`: the quadrantChart section is compared, with 0 differences.

Fuzzing (not committed; the generators and the inputs were in the scratchpad): 110,000 inputs. Three runs of 30,000 each, from random tokens and mutated fixtures, accepted about 15% of their inputs; and a structured generator of charts, 20,000 inputs with 2,523 accepted. Every difference outside the front matter agrees. The 150 differences are all front matter that the shared YAML reader accepts and js-yaml rejects (section 11, item 1).

## 11. Not reproduced, and open questions

1. **Front matter that js-yaml rejects.** Two kinds: js-yaml's "the stream contains non-printable characters" (a control character such as U+000B or U+000C in the front matter), and a malformed mapping that js-yaml reports as "a line break is expected" (for example `--->escription: >-`). The shared `yaml.rs` accepts both, and the `yaml` differential (4,281 inputs, no difference) does not cover them. This is a gap of the shared front matter, which affects every type with front matter. It is not fixed here. Recommendation: the YAML port adds these inputs and fixes the reader.
2. **The sanitiser's configuration.** The port applies the first pass of `sanitizeText` with the default configuration. `sanitizeMore` reads `securityLevel` and `htmlLabels` (section 6); the probes agree, but a directive that changes the sanitising of a quoted text with markup is not reproduced. The `getConfig` spy test of the reference's DB spec is ignored for this reason. Recommendation: accept, and revisit with the sanitiser port.
3. **The NaN coordinate in the drawing.** The model holds `"NaN"` (C3). Reproduced, not named as an exception: the renderer writes no `cx` (or `cy`) and `translate(undefined, …)` (or `NaN`), as section 12 lists (R1).
4. **The NaN fill of the redux themes** (NOTES 6.6, `hsl(240, 90%, NaN%)`). The theme port gives it, and the renderer copies it (section 12, R2).
5. **The layout data differential** (NOTES 8.3, question 3). Not added. The renderer's exact text is compared instead, with the same numbers (section 12).

## 12. Builder and renderer

Ported in `crates/ferrigura-render/src/quadrant_chart/`: `builder.rs` is `quadrantBuilder.ts` (`calculateSpace`, `getAxisLabels`, `getQuadrants`, `getQuadrantPoints`, `getBorders`, `getTitle`, `build`), and `mod.rs` is `quadrantRenderer.ts` (`draw`). Nothing is measured: every box is the configuration's arithmetic (NOTES 6.3, 6.7).

- **Scale.** The point positions use `common::d3::LinearScale` on [0, 1], which computes `a * (1 - t) + b * t` (NOTES 6.4). The complete chart's `cx="295.47999999999996"` is the reference's bit pattern.
- **R1, NaN coordinates.** A coordinate that the scale cannot read drops `cx` or `cy`; the label's transform writes `translate(undefined, …)` for an x and `translate(…, NaN)` for a y (NOTES 2.3). Reproduced.
- **R2, the NaN fill.** The redux themes give `quadrantPointFill` as `hsl(240, 90%, NaN%)`; the theme port produces it (`redux_color.rs`), and the point's fill and default stroke copy it. The renderer does not correct it.
- **R3, the x axis.** It is at the bottom whenever there are points (`builder` 611). A stroke-less point takes the theme's point fill as its stroke, not its own colour.
- **R4, empty texts and groups.** The five groups of `g.main` are always written, the four quadrant texts are written when empty, and a border line is written for each of the six.
- **Border styles.** The reference sets the stroke with d3's `style`, so the browser writes it as a declaration: `stroke: rgb(24, 24, 24); stroke-width: 2;`. A hex colour becomes `rgb()`. An `hsl()` colour is converted as Blink does it, in single precision: the redux border `hsl(247.5, 0%, 9.2156862745%)` reads as 24, where double precision reads 23. An `hsl()` with a NaN component is dropped, as the browser drops it. Other forms are written as given (probe p10: `red`, `rgb(10, 20, 30)`). This answers NOTES 8.3, question 5.
- **Entities.** A text is `decode_entities(escape(text))`, the whole-SVG pass of `mermaidAPI.ts` 359. The font subset takes the characters the reader sees, so the apostrophe and the quote are in it, and not their `&#39;` and `&quot;` markup.
- **Root.** The width comes first, then the namespace, then `height` or `style`, as the reference writes it; `title` and `desc` are the first children when the texts are set.

**Verification.** `pnpm port:geometry quadrantChart` covers the 14 fixtures and 16 probes (`tools/port/geometry/probes/quadrantChart/`, p01 to p16: edges, coordinates read as numbers, NaN coordinates, long texts, classes and point styles, 121 points, delimiters, entities, theme colours in CSS forms, the dark theme, sizes, accessible texts, empty texts, a chart smaller than its axes, hsl borders). Every comparable case is within tolerance. Beyond the harness, the SVG text of each case was compared with the reference's, with the `<style>` and the empty-element serialisation set aside: 29 of 30 are byte for byte the same, and the other writes `&#35;` where the browser's DOM writes `#` (p09), which is the same text.

**The harness.** `diagram-geometry.ts` now matches the type name without regard to case: `quadrantChart` was rejected because the argument was lower-cased before the lookup.

**Not covered.** The browser's style form of the colour keywords and of `rgba()`, which no reference fixture uses; the `<style>` sheet, which the harness does not compare; and the configuration differential of quadrantChart (open question 7).
