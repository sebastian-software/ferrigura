# xychart: research notes

Research only. Reference: this worktree at `aa5c4338f` (`sebastian/port-p2`). No repository file was changed; this file is the only new one, and nothing is committed.

Labels used below:

- **read**: taken from the source. Cited as `packages/mermaid/src/diagrams/xychart/<path>:<line>` unless another package path is named.
- **probed**: run. Section 0.4 says how. Every probed statement names its input.
- **unverified**: inferred from the source or from one probe, not confirmed beyond it.

---

## 0. Summary

### 0.1 What the type is

An xychart is one SVG chart: an optional title, an optional legend, an x-axis that is either a band of categories or a linear range, a linear y-axis, and any number of line and bar plots. `xychart horizontal` (keyword) or `chartOrientation: horizontal` (config) swaps the axes. The grammar is Jison (`parser/xychart.jison`, 176 lines), the DB is 255 lines, and the layout is the type's own arithmetic in `chartBuilder/` (about 1,380 lines, plus 180 of interfaces). It does not use a layout engine, the unified renderer, Markdown, HTML labels, randomness or the clock.

### 0.2 Size of the port

- Reference: about 2,290 lines of source (grammar, DB, renderer, chart builder, the config and theme values it reads) and 1,095 lines of specs (78 `it`).
- Port estimate: 2,300 to 2,600 lines of Rust with tests.

| Part                 | Reference                        | Port (Rust lines, with tests) | Notes                                                                  |
| -------------------- | -------------------------------- | ----------------------------- | ---------------------------------------------------------------------- |
| Scanner and parser   | `parser/xychart.jison`, 176      | 500 to 600                    | Jison lexer semantics, section 2. The journey scanner is the template. |
| DB and config        | `xychartDb.ts`, 255, plus config | 300                           | Section 3. Sanitising via the existing `sanitize_text`.                |
| Chart builder        | `chartBuilder/`, about 1,380     | 900 to 1,000                  | Orchestrator, title, legend, axes, plots. Section 6.2.                 |
| d3 subset            | band, linear, ticks, line        | 250                           | New code. Section 6.3. Not present in `ferrigura-render` yet.          |
| Glyph-box text width | none (DOM)                       | 150                           | New in `ferrigura-text`. Section 6.4.                                  |
| Renderer DOM         | `xychartRenderer.ts`, 262        | 300                           | Section 6.1.                                                           |
| Theme and style      | shared                           | 50                            | `base_style_sheet`, `set_xy_chart`.                                    |

### 0.3 Riskiest parts, ranked

1. **Text width.** The reference measures a text as the bounding box of its glyphs, not as its advance. `getBoundingClientRect()` (xychart) and `getBBox()` (the shared `calculateTextDimensions`) agree and both include glyph overhangs. `ferrigura-text` measures the advance (`measure.rs:30`, `layout_width`). The difference is 0.81 px at 14 px for "jan", "jun", "jul" and "january", and it changes whole-pixel rounding in the shared text code (section 6.4 and item 6). In the 47 fixtures the only overhanging texts are "jan", "jun", "jul", "j" and "q" (234 texts measured at 14, 16 and 20 px), and none of them is the widest label, so the fixtures do not depend on the rule.
2. **The Jison lexer.** Keyword boundaries, the state stack, a garbled Markdown rule, empty strings that produce no token, and an orientation word without a boundary. Section 2.2 lists P1 to P13, each with its probe.
3. **Non-finite and out-of-range output.** A band axis with fewer values than categories produces `<rect>` elements with no `y` and `height="NaN"`. Bars below a horizontal axis minimum get negative widths. Points outside the y range are drawn outside the plot. Section 8, decision 1.
4. **Reference bugs that the port must reproduce.** Bar data labels take their numbers from the first plot. Data labels take the colour `#131300` in both shipping themes, a value that comes from the classic default theme. Plots declared before a band x-axis collapse onto its first category. Duplicate categories overlap. The left axis line has the class `axisl-line` (typo). Section 6.8.
5. **Float exactness.** d3's number interpolation (`a*(1-t)+b*t`), the band step formula, path coordinates rounded to three decimals with `Math.round(v*1000)/1000`, and JavaScript number-to-string for tick labels (`1e-7`, `1.2e-7`) and for the `x` strings the DB stores. Section 6.3.
6. **Cross-cutting.** The shared `calculateTextDimensions` (`utils.ts:706-708`) rounds `getBBox().width`. For "jun" at 14 px the box is 19.53 and the reference writes 20. `ferrigura-render/src/common/text.rs:253` (`text_dimensions`) rounds the advance (18.72), which gives 19. The same happens for "jul" (15 against 14) and "jq" (13 against 12). This affects the shared M5 and M6 text code, not only xychart.

### 0.4 How the probes were run

Scratch directory (outside the repository, not committed): `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/xychart/`. The probe scripts and their input and output files are there.

- **DB and parser.** `probe-db.mts` loads the reference's modules through `tools/port/reference.ts` (`openReference`, `provideDomForSanitising`): `xychartDb.ts`, `parser/xychart.jison`, `preprocess.ts`, `utils.ts`. For each input it runs `preprocessDiagram`, `db.clear`, the front-matter title, `encodeEntities(cleaned) + '\n'` and `parser.parse`, the steps of `Diagram.fromText` (`Diagram.ts:47-77`). 85 inputs in `cases-db*.json`; the output is `out-db*.txt`.
- **Lexer tables.** `lexer-dump.mts` runs the reference's `.build/jisonTransformer.ts` on `xychart.jison` and reads the generated `rules` array (`xychart-generated.mjs`).
- **Rendering.** `render.mts` uses the harness page of `tools/port/geometry/common.ts` (`withPage`, `renderReference`, `patchText`). The page loads the built bundle `packages/mermaid/dist/mermaid.esm.mjs` in Playwright's Chromium and applies the harness's forced settings (`htmlLabels: false`, the Lexica font, `securityLevel: strict`, and the shipping theme). All 47 fixtures and 32 extra inputs (`cases-render*.json`) were rendered. The SVGs are in `fixtures-svg/` and `svg/`.
- **d3.** `d3probe.mjs` loads `d3@7.9.0` from `node_modules/.pnpm/d3@7.9.0` and runs band, linear, ticks and line directly.
- **Text.** `measure*.mts` measure in the harness page with the Lexica font: 91 (string, size) pairs for the box and the computed length, about 20 strings for per-glyph checks, the 234 texts of the fixtures (`measure6.mts`), and plain `<text>` elements for the `calculateTextDimensions` path (`measure5.mts`). Glyph bounds were read from `crates/ferrigura-text/fonts/LexicaUltralegible-Regular.ttf` (`glyf`, `loca`, `hmtx`, `cmap`) by a small Python reader.
- **Side effects.** The harness's `withPage` created the empty, gitignored `target/geometry` directory. An untracked `docs-port/sankey/` appeared in the worktree during the run; it is not mine, and I did not touch it.

---

## 1. Sources

### 1.1 Reference files

| Role            | File                                                                                                                                    | Lines                               | Notes                                                              |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------ |
| Grammar         | `parser/xychart.jison`                                                                                                                  | 1-76 lexer, 78-176 grammar          | `%options case-insensitive` (2); start conditions 4-13             |
| Parser spec     | `parser/xychart.jison.spec.ts`                                                                                                          | 1-572                               | 58 `it` in 3 `describe` blocks (root, point labels, accessibility) |
| DB              | `xychartDb.ts`                                                                                                                          | 1-255                               | Section 3                                                          |
| DB spec         | `xychartDb.spec.ts`                                                                                                                     | 1-126                               | 6 `it` in 2 `describe` blocks                                      |
| Renderer        | `xychartRenderer.ts`                                                                                                                    | 1-262                               | Section 6.1                                                        |
| Builder entry   | `chartBuilder/index.ts`                                                                                                                 | 1-15                                | `XYChartBuilder.build`                                             |
| Orchestrator    | `chartBuilder/orchestrator.ts`                                                                                                          | 1-218                               | Section 6.2                                                        |
| Interfaces      | `chartBuilder/interfaces.ts`                                                                                                            | 1-180                               | `XYChartConfig` 102-117, `DrawableElem` 165-180                    |
| Title           | `chartBuilder/components/chartTitle.ts`                                                                                                 | 1-105                               | Spec: `chartTitle.spec.ts`, 151 lines, 8 `it`                      |
| Legend          | `chartBuilder/components/legend.ts`                                                                                                     | 1-167                               | Spec: `legend.spec.ts`, 203 lines, 3 `it`                          |
| Axis base       | `chartBuilder/components/axis/baseAxis.ts`                                                                                              | 1-446                               | Section 6.2                                                        |
| Band axis       | `chartBuilder/components/axis/bandAxis.ts`                                                                                              | 1-45                                | d3 `scaleBand`                                                     |
| Linear axis     | `chartBuilder/components/axis/linearAxis.ts`                                                                                            | 1-38                                | d3 `scaleLinear`                                                   |
| Axis factory    | `chartBuilder/components/axis/index.ts`                                                                                                 | 1-47                                | `getAxis` 24-47                                                    |
| Plots           | `chartBuilder/components/plot/index.ts` 1-97, `barPlot.ts` 1-60, `linePlot.ts` 1-95                                                     |                                     | d3 `line` in `linePlot.ts:1`                                       |
| Text measure    | `chartBuilder/textDimensionCalculator.ts`                                                                                               | 1-46                                | Spec: 43 lines, 3 `it`                                             |
| Detector        | `xychartDetector.ts`                                                                                                                    | 1-24                                | Regex `/^\s*xychart(?![\w-])/` at line 10                          |
| Diagram         | `xychartDiagram.ts`                                                                                                                     | 1-11                                | No `styles`, no `init`                                             |
| Config schema   | `packages/mermaid/src/schemas/config.schema.yaml`                                                                                       | 1408-1483 (axis), 1485-1563 (chart) | Section 6.6                                                        |
| Config defaults | `packages/mermaid/src/defaultConfig.ts`                                                                                                 | 271-274                             | `useWidth: undefined`                                              |
| Themes          | `packages/mermaid/src/themes/theme-redux-color.js` 364-379; `theme-redux-dark-color.js` 381-396; `theme-default.js` 370-386 and 503-507 |                                     | Section 6.5                                                        |

The reference has no `styles.ts` for xychart and no renderer unit spec. Its rendering checks are the Playwright spec `e2e/rendering/xychart/nightly-check/xyChart.spec.js` (444 lines, 15 tests: image snapshots plus DOM assertions, section 5) and the fixtures (section 7).

### 1.2 Shared reference code the chart calls

| Step           | File and lines                                                                                                                                                          | What it does for xychart                                                                                        |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Pipeline       | `Diagram.ts:47-77`                                                                                                                                                      | `parser.parser.yy = db` (47), `db.clear()` (49), front-matter title (53), `encodeEntities(cleaned) + '\n'` (77) |
| Preprocess     | `preprocess.ts:58-89`                                                                                                                                                   | Front matter, directives, comments. `cleanupComments` removes `%%` lines.                                       |
| Entities       | `utils.ts:874-896` (`encodeEntities`), `:904-906` (`decodeEntities`)                                                                                                    | `#name;` becomes a placeholder before parsing and is decoded in the final SVG                                   |
| Sanitising     | `diagrams/common/common.ts:56-68` (`sanitizeMore`), `:71-82` (`sanitizeText`)                                                                                           | DOMPurify; `FORBID_TAGS: ['style']`                                                                             |
| Header setters | `diagrams/common/commonDb.ts:10-14` (`clear`), `:16-30` (setters)                                                                                                       | Accessibility and title text, sanitised                                                                         |
| Measurement    | `rendering-util/createText.ts:127-140` (`computeDimensionOfText`), `:213-223` (`decodeHTMLEntities`), `:237-256` (`updateTextContentAndStyles`), `:96-` (`createTspan`) | Section 6.4                                                                                                     |
| Size           | `setupGraphViewbox.js:23-33` (`calculateSvgSizeAttrs`), `:43-47` (`configureSvgSize`)                                                                                   | Section 6.5                                                                                                     |
| Selection      | `rendering-util/selectSvgElement.ts:11-22`                                                                                                                              | `select('body')` then `#id`                                                                                     |
| Root group     | `mermaidAPI.ts:397-420` (`appendDivSvgG`), `:419` (`svgNode.append('g')`)                                                                                               | The empty `<g>` before `main`                                                                                   |
| Accessibility  | `mermaidAPI.ts:621-623`                                                                                                                                                 | `<title>`, `<desc>`, `aria-describedby`, `aria-labelledby` (probed)                                             |
| Config merge   | `utils.ts:841-843` (`cleanAndMerge`, lodash `merge`)                                                                                                                    | Partial config objects merge with defaults                                                                      |
| Config reads   | `config.ts:324-326` (`getEffectiveHtmlLabels`)                                                                                                                          | Decides whether `sanitizeMore` runs                                                                             |

### 1.3 Libraries

| Library (installed)    | Module                                       | Used by                                                                          | Port needs                                                                                                        |
| ---------------------- | -------------------------------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `d3` 7.9.0 (umbrella)  | `scaleBand`, `scaleLinear`, `line`, `select` | `bandAxis.ts:1-2`, `linearAxis.ts:1-2`, `linePlot.ts:1`, `selectSvgElement.ts:1` | See 6.3                                                                                                           |
| `d3-scale` 4.0.2       | `band.js`, `linear.js`, `continuous.js`      | the two axes                                                                     | Band `rescale` (`band.js:20-32`), continuous `scale` (`continuous.js:85-87`), `normalize` (`continuous.js:12-16`) |
| `d3-array` 3.2.4       | `ticks.js`                                   | `scale.ticks()` (`linear.js:9-12`)                                               | `tickSpec` and `ticks` (`ticks.js:5-44`)                                                                          |
| `d3-interpolate` 3.0.1 | `number.js`                                  | the default interpolator of `continuous.js`                                      | `a*(1-t)+b*t` (`number.js:3`)                                                                                     |
| `d3-shape` 3.2.0       | `line.js`, `curve/linear.js`, `path.js`      | `line()`                                                                         | Linear curve (`curve/linear.js:7-37`), `digits` 3 (`path.js:4-6`)                                                 |
| `d3-path` 3.1.0        | `path.js`                                    | the path string (`moveTo`/`lineTo`, `path.js:33-44`)                             | `appendRound` (`path.js:13-23`)                                                                                   |
| `d3-selection` 3.0.0   | via `d3`                                     | DOM building in the renderer and the measure                                     | Not ported; its `attr` and `text` semantics are in 6.1                                                            |

Not used by xychart: `d3-format`, `d3-time`, `d3-axis`, `tickFormat`, `nice`, `invert`, `clamp`, `step`, `bandwidth`, `curve`. The only `scale` calls are `scale(value)` and `scale.ticks()`. Grep of the folder for `getBBox`, `getComputedTextLength`, `format(` and `tickFormat` finds nothing. The DOM measurement calls are `getBoundingClientRect` (reached through `createText.ts:135`) and `getScreenCTM` (`textDimensionCalculator.ts:31`).

### 1.4 Ferrigura code to reuse

| Need                | Existing code                                                                                         | Gap                                                                                                                                                                                     |
| ------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Base style sheet    | `crates/ferrigura-render/src/common/style.rs:30` (`base_style_sheet`)                                 | None. The probed `<style>` is `#id{font-family:"Lexica Ultralegible";font-size:14px;fill:#28253D;}` and then the generic edge-animation rules.                                          |
| Root size           | `common/viewbox.rs:44` (`configure_svg_size`), `:28` (`calculate_svg_size_attrs`)                     | None                                                                                                                                                                                    |
| Elements            | `common/element.rs`, `common/draw.rs:21-225` (`draw_rect`, `draw_text`)                               | The attribute order of 6.1 must be written by the port's calls; check it against the probes                                                                                             |
| Text advance        | `ferrigura-text/src/measure.rs:30` (`layout_width`), `:40` (`measure`)                                | Advances only. See 6.4 for the glyph box.                                                                                                                                               |
| Text box height     | `ferrigura-text/src/metrics.rs:69` (`text_box`)                                                       | Whole pixels; the reference's box is float32 (6.4)                                                                                                                                      |
| Measured dimensions | `common/text.rs:253` (`text_dimensions`)                                                              | Rounds the advance; the reference rounds the glyph box (0.3, item 6)                                                                                                                    |
| Sanitising          | `ferrigura-syntax/src/sanitize.rs:89` (`sanitize_text`), `:688` (`decode_references`)                 | None known                                                                                                                                                                              |
| Numbers             | `ferrigura-render/src/unified/path.rs:52` (`js_number`)                                               | JavaScript `Number#toString`. `svg.rs:27-36` (`svg::number`) writes `0` for any value whose absolute value is below 1e-10, which JavaScript does not do; do not use it for tick labels. |
| Scanner template    | `ferrigura-syntax/src/journey/scanner.rs`                                                             | Reproduces Jison's `next()` with states and the `done` flag                                                                                                                             |
| Config typing       | `ferrigura-model/build.rs` (schema to structs), `config/lenient.rs` (numeric strings read as numbers) | See 6.6 and section 8, decision 3                                                                                                                                                       |
| Theme variables     | `ferrigura-model/src/theme/helpers.rs:283-321` (`set_xy_chart`)                                       | Writes the eleven colours and the palette for the redux themes, without `dataLabelColor` (the reference's redux themes lack it too)                                                     |

---

## 2. Parser

### 2.1 Pipeline before the lexer

1. `preprocessDiagram` (`preprocess.ts:58-89`): CRLF handling, front matter (its `title` and `config`), directives (`%%{init}%%`), and `cleanupComments`. The DB gets the cleaned text.
2. `Diagram.fromText` (`Diagram.ts:47-77`): `db.clear()`, the front-matter title, then `encodeEntities(cleaned) + '\n'`, then `parse`. xychart has no `init` and no `preserveCommentsWhenParsing` (read: `xychartDb.ts` sets neither), so the comment-stripped text is parsed. Probed: a `%%` comment line and a trailing `%%` comment parse with no effect (P13).
3. Jison runs with `yy` set to the DB (`Diagram.ts:47`).

### 2.2 Lexer

Start conditions (read, `xychart.jison:4-13`):

- Exclusive (`%x`): `string`, `md_string`, `title`, `acc_title`, `acc_descr`, `acc_descr_multiline`. The `title` state is declared but no rule uses it; the `"title"` rule (line 22) has no state.
- Inclusive (`%s`): `axis_data`, `axis_band_data`, `data`, `data_inner`. Rules without a state prefix apply in them too.

Table of the generated rules, in order (probed: `lexer-dump.mts` reads the `rules` array that the reference's transform generates from the grammar). The first rule that matches wins, as in Jison. "Effective" is the regex as generated.

| #   | Effective regex (generated)          | Source | Action                                      | State                          |
| --- | ------------------------------------ | ------ | ------------------------------------------- | ------------------------------ |
| 1   | `%%(?!\{)[^\n]*`                     | 15     | skip                                        | all                            |
| 2   | `[^\}]%%[^\n]*`                      | 16     | skip                                        | all                            |
| 3   | `(\r?\n)`                            | 17     | NEWLINE, pop                                | `axis_data`                    |
| 4   | `(\r?\n)`                            | 18     | NEWLINE, pop                                | `data`                         |
| 5   | `[\n\r]+`                            | 19     | NEWLINE                                     | all                            |
| 6   | `%%[^\n]*`                           | 20     | skip                                        | all                            |
| 7   | `title\b`                            | 22     | TITLE                                       | all                            |
| 8   | `accTitle\s*:\s*`                    | 24     | push `acc_title`, ACC_TITLE                 | all                            |
| 9   | `(?!\n\|\|)*[^\n]*`                  | 25     | pop, ACC_TITLE_VALUE (the rest of the line) | `acc_title`                    |
| 10  | `accDescr\s*:\s*`                    | 26     | push `acc_descr`, ACC_DESCR                 | all                            |
| 11  | `(?!\n\|\|)*[^\n]*`                  | 27     | pop, ACC_DESCR_VALUE                        | `acc_descr`                    |
| 12  | `accDescr\s*\{\s*`                   | 28     | push `acc_descr_multiline`                  | all                            |
| 13  | `\}`                                 | 29     | pop                                         | `acc_descr_multiline`          |
| 14  | `[^\}]*`                             | 30     | ACC_DESCR_MULTILINE_VALUE                   | `acc_descr_multiline`          |
| 15  | `xychart\b`                          | 32     | XYCHART                                     | all                            |
| 16  | `(?:vertical\|horizontal)` (no `\b`) | 33     | CHART_ORIENTATION                           | all                            |
| 17  | `x-axis\b`                           | 35     | push `axis_data`, X_AXIS                    | all                            |
| 18  | `y-axis\b`                           | 36     | push `axis_data`, Y_AXIS                    | all                            |
| 19  | `\[`                                 | 37     | push `axis_band_data`, SQUARE_BRACES_START  | `axis_data`                    |
| 20  | `-->`                                | 38     | ARROW_DELIMITER                             | `axis_data`                    |
| 21  | `line\b`                             | 41     | push `data`, LINE                           | all                            |
| 22  | `bar\b`                              | 42     | push `data`, BAR                            | all                            |
| 23  | `\[`                                 | 43     | push `data_inner`, SQUARE_BRACES_START      | `data`                         |
| 24  | `[+-]?(?:\d+(?:\.\d+)?\|\.\d+)`      | 44     | NUMBER_WITH_DECIMAL                         | `axis_data`, `data_inner`      |
| 25  | `\]`                                 | 45     | pop, SQUARE_BRACES_END                      | `data_inner`, `axis_band_data` |
| 26  | garbled Markdown rule (see P4)       | 50-52  | none that matches                           | none                           |
| 27  | `"`                                  | 53     | push `string`                               | all                            |
| 28  | `"`                                  | 54     | pop                                         | `string`                       |
| 29  | `[^"]*`                              | 55     | STR                                         | `string`                       |
| 30  | `\[`                                 | 58     | SQUARE_BRACES_START                         | all                            |
| 31  | `\]`                                 | 59     | SQUARE_BRACES_END                           | all                            |
| 32  | `[A-Za-z]+`                          | 60     | ALPHA                                       | all                            |
| 33  | `:`                                  | 61     | COLON                                       | all                            |
| 34  | `\+`                                 | 62     | PLUS                                        | all                            |
| 35  | `,`                                  | 63     | COMMA                                       | all                            |
| 36  | `=`                                  | 64     | EQUALS                                      | all                            |
| 37  | `\*`                                 | 65     | MULT                                        | all                            |
| 38  | `#`                                  | 66     | BRKT                                        | all                            |
| 39  | `[_]`                                | 67     | UNDERSCORE                                  | all                            |
| 40  | `\.`                                 | 68     | DOT                                         | all                            |
| 41  | `&`                                  | 69     | AMP                                         | all                            |
| 42  | `-`                                  | 70     | MINUS                                       | all                            |
| 43  | `[0-9]+`                             | 71     | NUM                                         | all                            |
| 44  | `\s+`                                | 72     | skip                                        | all                            |
| 45  | `;`                                  | 73     | SEMI                                        | all                            |
| 46  | `$`                                  | 74     | EOF                                         | all                            |

Every rule is case-insensitive (`/i`, from `%options case-insensitive`). The `\b` after `title`, `xychart`, `x-axis`, `y-axis`, `line` and `bar` is added by Jison to keyword literals that end in a word character; `accTitle` and `accDescr` end in `:` or `{` and get none.

#### Quirks of the lexer

| Id  | Quirk                                                                                                                                                                                                                                                             | Test input                                                                      | Reference output                              | Evidence |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------- | -------- |
| P1  | A keyword is recognised only at a word boundary (rules 7, 15, 17, 18, 21, 22).                                                                                                                                                                                    | `line titleX [1]`                                                               | the plot title is `titleX`                    | probed   |
| P2  | The orientation word has no boundary (rule 16).                                                                                                                                                                                                                   | `title Horizontal Sales`; `title verticalX`                                     | parse error on line 2                         | probed   |
| P3  | Blanks inside unquoted text are skipped (rule 44), and the parser joins the words with no space.                                                                                                                                                                  | `title My Chart 2`                                                              | `MyChart2`                                    | probed   |
| P4  | Markdown strings never match. Jison reads the quoting of `(?:"`)` on line 50 wrongly, so rule 26 is one regex that starts with a backtick and swallows the next two lines. A backtick outside a quoted string is a lexical error. Inside quotes it is plain text. | `title \`x\``and`line \`x\` [1, 2]`(error);`line "\`x\`" [1, 2]`                | error; the plot title is `` `x` ``            | probed   |
| P5  | An empty quoted string gives no token. After the opening `"` the closing `"` matches rule 28 before rule 29 can return an empty STR.                                                                                                                              | `title ""`                                                                      | parse error ("expecting STR ... got NEWLINE") | probed   |
|     |                                                                                                                                                                                                                                                                   | `line "" [1, 2]`                                                                | same as `line [1, 2]`, title ``               | probed   |
|     |                                                                                                                                                                                                                                                                   | `x-axis "" [a, b]`; `line [1 "", 2]`                                            | no title; no label                            | probed   |
|     |                                                                                                                                                                                                                                                                   | `x-axis ["", b]`                                                                | parse error                                   | probed   |
| P6  | A quoted string may span lines (rule 29 is `[^"]*`).                                                                                                                                                                                                              | `title "two` (newline) `lines"`                                                 | title `two\nlines`                            | probed   |
| P7  | The number rule (24) applies only in `axis_data` and `data_inner`. Elsewhere digits are NUM and join text. A leading zero is read as a number.                                                                                                                    | `line [+1.3, .6, 2.4, -.34]`; `line [012, 0.10]`                                | values 1.3, 0.6, 2.4, -0.34; 12, 0.1          | probed   |
|     |                                                                                                                                                                                                                                                                   | `line [5., 1]`; `line [1e3]`                                                    | parse error on line 2                         | probed   |
|     |                                                                                                                                                                                                                                                                   | `title 2024`                                                                    | title `2024`                                  | probed   |
| P8  | Statements need no line break between them. The DB receives both.                                                                                                                                                                                                 | `line [1, 2] bar [3, 4]`                                                        | two plots, a line then a bar                  | probed   |
| P9  | `;` separates statements, as a line break does (rule 73, `eol: SEMI`).                                                                                                                                                                                            | `xychart; line [1, 2]; bar [3, 4]`                                              | two plots                                     | probed   |
| P10 | Keywords are case-insensitive.                                                                                                                                                                                                                                    | `XYCHART` / `TITLE "Up"` / `X-AXIS Q [a, b]` / `Y-AXIS 0 --> 5` / `LINE [1, 2]` | parses; title `Up`; axis title `Q`            | probed   |
| P11 | The header must come first, after blank lines (`start`, `xychart.jison:82-87`).                                                                                                                                                                                   | `xychart diagonal`                                                              | parse error on line 1                         | probed   |
| P12 | An `accTitle:` or `accDescr:` value is the rest of the line. The lookahead in rules 9 and 11 lost its `;` and `#` alternatives in the generated regex, but the effect is the same. A block `accDescr { ... }` keeps its lines and collapses blank runs.           | `accDescr {` / `    first line` / `    second line` / `  }`                     | `first line\nsecond line`                     | probed   |
| P13 | `%%` comments are removed, from the line start (rules 1, 6) or after any character other than `}` (rule 2).                                                                                                                                                       | `line [1, 2] %% trailing`                                                       | the trailing text is gone                     | probed   |

#### Errors

A parse error is Jison's message: `Parse error on line N:`, the source line, a caret, and `Expecting 'STR', 'MD_STR', ..., got 'NEWLINE'`. The expected-token list comes from the LALR tables. The DB reports no error of its own at parse time. `getDrawableElem` throws `No Plot to render, please provide a plot with some data` (`xychartDb.ts:204-206`), but the renderer fails earlier (6.8, R1).

### 2.3 Grammar

| Production                                                                                           | Lines   | Action                                                    |
| ---------------------------------------------------------------------------------------------------- | ------- | --------------------------------------------------------- |
| `start: eol start \| XYCHART chartConfig start \| XYCHART start \| document`                         | 82-87   | none                                                      |
| `chartConfig: CHART_ORIENTATION`                                                                     | 89-91   | `setOrientation`                                          |
| `document: /* empty */ \| document statement`                                                        | 93-96   | none                                                      |
| `statement: statement eol`                                                                           | 98-99   | none                                                      |
| `statement: title text`                                                                              | 100     | `setDiagramTitle(text.trim())`                            |
| `statement: X_AXIS parseXAxis \| Y_AXIS parseYAxis`                                                  | 101-102 | the axis setters                                          |
| `statement: LINE plotData \| LINE text plotData`                                                     | 103-104 | `setLineData(text or {text:''}, points)`                  |
| `statement: BAR plotData \| BAR text plotData`                                                       | 105-106 | `setBarData(text or {text:''}, points)`                   |
| `statement: acc_title acc_title_value`                                                               | 107     | `setAccTitle(value.trim())`                               |
| `statement: acc_descr acc_descr_value \| acc_descr_multiline_value`                                  | 108-109 | `setAccDescription(value.trim())`                         |
| `plotData: [ dataPoints ]`                                                                           | 112-114 | none                                                      |
| `dataPoints: dataPoint COMMA dataPoints \| dataPoint`                                                | 116-119 | none                                                      |
| `dataPoint: NUMBER_WITH_DECIMAL STR \| NUMBER_WITH_DECIMAL`                                          | 121-124 | `{ value: Number($1), label: $2 or '' }`                  |
| `parseXAxis: text \| text xAxisData \| xAxisData`                                                    | 126-130 | `setXAxisTitle(text)`, or `{text:''}` when absent         |
| `xAxisData: bandData \| NUMBER --> NUMBER`                                                           | 132-135 | `setXAxisBand(categories)`, `setXAxisRangeData(min, max)` |
| `bandData: [ commaSeparatedTexts ]`                                                                  | 137-139 | none                                                      |
| `commaSeparatedTexts: text COMMA commaSeparatedTexts \| text`                                        | 141-144 | none                                                      |
| `parseYAxis: text \| text yAxisData \| yAxisData`                                                    | 146-150 | `setYAxisTitle(text)`, or `{text:''}`                     |
| `yAxisData: NUMBER --> NUMBER`                                                                       | 152-154 | `setYAxisRangeData(min, max)`                             |
| `eol: NEWLINE \| SEMI \| EOF`                                                                        | 156-160 | none                                                      |
| `text: alphaNum \| STR \| MD_STR`                                                                    | 163-166 | `type: 'text'` or `'markdown'`                            |
| `alphaNum: alphaNumToken \| alphaNum alphaNumToken`                                                  | 168-171 | concatenation with no space                               |
| `alphaNumToken: AMP \| NUM \| ALPHA \| PLUS \| EQUALS \| MULT \| DOT \| BRKT \| MINUS \| UNDERSCORE` | 174     | none                                                      |

The `text` type is `markdown` only for MD_STR, which never occurs (P4). The DB ignores the type in every case.

Note that `x-axis` and `y-axis` take their range in a fixed place: the `-->` range belongs to the axis, and a title may come before the band or the range. Axis titles are kept when the axis is replaced (`xychartDb.ts:92, 98`).

---

## 3. DB

### 3.1 Calls in order

Each grammar action makes one DB call, as in section 2.3. The DB keeps the state in module variables (`xychartDb.ts:26-35`). The common header values (title, accTitle, accDescription) live in `commonDb.ts`.

### 3.2 Each DB function

| Function (`xychartDb.ts`)            | Lines   | Stores                                                                                                                | Notes                                                                                                                 |
| ------------------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `setOrientation(o)`                  | 81-87   | `chartOrientation` = `horizontal` for that word, otherwise `vertical`                                                 | The keyword beats the config (probed: keyword `horizontal` with config `vertical` gives `horizontal`)                 |
| `setXAxisTitle(t)`                   | 88-90   | `xAxis.title = textSanitizer(t.text)`                                                                                 | Kept by later axis setters                                                                                            |
| `setXAxisRangeData(min, max)`        | 91-94   | `xAxis = {linear, title, min, max}`; `hasSetXAxis = true`                                                             |                                                                                                                       |
| `setXAxisBand(categories)`           | 95-102  | `xAxis = {band, title, categories: sanitised}`; `hasSetXAxis = true`                                                  |                                                                                                                       |
| `setYAxisTitle(t)`                   | 103-105 | `yAxis.title`                                                                                                         |                                                                                                                       |
| `setYAxisRangeData(min, max)`        | 106-109 | `yAxis = {linear, title, min, max}`; `hasSetYAxis = true`                                                             | Only numbers reach here (the grammar has no band y-axis)                                                              |
| `setYAxisRangeFromPlotData(data)`    | 112-123 | `yAxis.min` and `max` widened by the plot's values                                                                    | Does not set `hasSetYAxis`. Starts from the current linear range, which is `+Infinity` / `-Infinity` before any plot. |
| `transformDataWithoutCategory(data)` | 125-164 | returns the `[x, y]` pairs of one plot                                                                                | See 3.3                                                                                                               |
| `setLineData(title, data)`           | 175-189 | pushes `{type:'line', title, strokeFill, strokeWidth: 2, data, pointLabels?}`                                         | `pointLabels` only if at least one label is non-empty (`hasAnyLabel`)                                                 |
| `setBarData(title, data)`            | 191-201 | pushes `{type:'bar', title, fill, data}`                                                                              | Labels are dropped (D10)                                                                                              |
| `getDrawableElem()`                  | 203-209 | nothing                                                                                                               | Throws without plots (`204-206`); sets `xyChartData.title` from `getDiagramTitle()`; calls `XYChartBuilder.build`     |
| `clear()`                            | 223-232 | `commonClear()`, `plotIndex = 0`, the config, data and theme from the current config, the palette, both flags `false` | `tmpSVGGroup` is not reset                                                                                            |

### 3.3 The plot data transform

`transformDataWithoutCategory` (`xychartDb.ts:125-164`) runs when a plot is declared, in this order:

1. An empty list gives an empty list (`127-129`).
2. If no x-axis range or band has been set, `setXAxisRangeData(min(prev, 1), max(prev, data.length))` runs (`130-134`). That call sets `hasSetXAxis`, so the range of the first plot stays. A later, longer plot does not widen it (probed `second-plot-longer-linear`: a 2-point and a 5-point line; the axis is 1 to 2).
3. On a band axis, data longer than the category count is cut to the category count (`139-141`). The cut values are also left out of the y range.
4. If no y range has been set, the y range widens to cover the data (`143-145`).
5. Band axis: `categories.map((c, i) => [c, data[i]])` (`147-149`). A category without a value gets `undefined`.
6. Linear axis: one value gives `[[String(min), data[0]]]`. Otherwise `step = (max - min) / (n - 1)` and `x = String(min + i*step)` (`151-161`).

The x values are JavaScript strings: `String(min + i*step)` for the linear axis and the category text for the band axis. The strings round-trip, so a port may keep `f64` internally, but the model and the output must write what `String` writes (for example `1.6666666666666667`, `1e-7`).

### 3.4 Sanitising

`textSanitizer(text)` is `sanitizeText(text.trim(), config)` (`xychartDb.ts:73-76`). `sanitizeText` (`common.ts:71-82`) returns an empty text unchanged. Otherwise it runs DOMPurify, with `FORBID_TAGS: ['style']`, on `sanitizeMore(text)`. `sanitizeMore` (`common.ts:56-68`) acts only when `htmlLabels` is on (the default): under `strict`, `antiscript` and `sandbox` it runs DOMPurify again; under any level except `loose` it rewrites `<br>` and escapes `<`, `>` and `=`; under `loose` it does nothing. The harness sets `htmlLabels: false`, so only DOMPurify runs there.

Probed, title texts (`setDiagramTitle` via the grammar, `htmlLabels` as stated):

| Input title                       | `htmlLabels: false`                                                       | `htmlLabels: true`                   |
| --------------------------------- | ------------------------------------------------------------------------- | ------------------------------------ |
| `A & B <i>c</i> 1 < 2 = 3 > 0`    | `A &amp; B <i>c</i> 1 &lt; 2 = 3 &gt; 0`                                  | not probed for this input            |
| `<script>x</script>T`             | `T`                                                                       | `T`                                  |
| `a=b`                             | `a=b`                                                                     | `a=b`                                |
| `a<br>b`                          | `a<br>b`                                                                  | `a<br>b`                             |
| `a&amp;b c`                       | `a&amp;b c`                                                               | not probed                           |
| `#quot;hi#quot; #9829;`           | placeholders `ﬂ°quot¶ßhiﬂ°quot¶ß ﬂ°°9829¶ß` (decoded at the end, see 6.8) | not probed                           |
| a non-breaking space in the title | kept as U+00A0 in the DB value (jsdom's serialisation)                    | not probed. Unverified in a browser. |

Categories and axis and plot titles go through the same `textSanitizer`; a category `<b>q</b>` stays as markup text.

### 3.5 Defaults and clear

- `xAxis`: `{type: 'band', title: '', categories: []}`; `yAxis`: `{type: 'linear', title: '', min: Infinity, max: -Infinity}`; `title: ''`; `plots: []` (`xychartDb.ts:55-71`).
- Config: `cleanAndMerge(defaultConfig.xyChart, config.xyChart)` (`xychartDb.ts:47-53`), read again in `clear()`.
- Theme: `cleanAndMerge(getThemeVariables().xyChart, config.themeVariables.xyChart)` (`xychartDb.ts:42-46`). `getThemeVariables()` is the classic default theme, built with no overrides (`theme-default.js:503-507`), whatever the active theme is. That is the source of the data-label colour (6.8, R3).
- `clear()` resets everything in the module and the common header values (`commonDb.ts:10-14`).

### 3.6 DB quirks

| Id  | Quirk                                                                                                                                                                                                                                                           | Test input                                                | Evidence                                                                                                  |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| D1  | The first plot's x values fix the linear x range; later plots never widen it.                                                                                                                                                                                   | `line [1, 2]` then `line [1, 2, 3, 4, 5]`                 | probed: x-axis 1 to 2, five points                                                                        |
| D2  | A plot declared before a band x-axis keeps its linear x strings.                                                                                                                                                                                                | `line [1, 2, 3]` then `x-axis [a, b, c]`                  | probed: data `["1",1],["2",2],["3",3]`, categories `a, b, c`                                              |
| D3  | A band x-axis declared before the plot maps the values to categories in order.                                                                                                                                                                                  | `x-axis [a, b, c]` then `line [1, 2, 3]`                  | probed: data `["a",1],["b",2],["c",3]`                                                                    |
| D4  | Data longer than the categories is cut, and the cut values do not widen the y range.                                                                                                                                                                            | `x-axis [a, b, c]`, `bar [1, 2, 3, 4, 5]`                 | spec `xychartDb.spec.ts:43-61`; probed                                                                    |
| D5  | Shorter data gives `undefined` for the missing categories (`null` in JSON).                                                                                                                                                                                     | `x-axis [a, b, c, d]`, `bar [1, 2]`                       | probed: `["c", undefined], ["d", undefined]`; spec `:96-107`                                              |
| D6  | Duplicate categories are kept in the model.                                                                                                                                                                                                                     | `x-axis [a, a, b]`, `bar [1, 2, 3]`                       | probed                                                                                                    |
| D7  | `x-axis 0 --> 0` gives every x the string `"0"`.                                                                                                                                                                                                                | `x-axis 0 --> 0`, `line "dirac" [0, 0.5, 1]`              | probed                                                                                                    |
| D8  | With no y range, the range is the min and max of the data, with no zero included.                                                                                                                                                                               | `line [5, 5]`; `bar [5000, 6000, 7500]`                   | probed: `[5,5]` for equal data; `[5000, 7500]`                                                            |
| D9  | `y-axis a --> b` sets the range and it stays; data outside it is kept.                                                                                                                                                                                          | `y-axis 0 --> 100`, `line [200]`                          | probed: y range 0 to 100, point 200                                                                       |
| D10 | Bar point labels are dropped; line point labels are kept, with `""` for the unlabeled points.                                                                                                                                                                   | `bar [1 "x", 2 "y"]`; `line [1 "A", 2, 3 "C"]`            | probed: no label field; `pointLabels: ["A", "", "C"]`                                                     |
| D11 | `y-axis 10 --> 0` keeps min 10 and max 0.                                                                                                                                                                                                                       | `y-axis 10 --> 0`, `line [1, 2]`                          | probed                                                                                                    |
| D12 | The palette index counts every plot, line or bar. Colour `palette[index % length]`, index 0 gives `palette[0]`.                                                                                                                                                 | `line`, `bar`, `line`, ...                                | probed: `bar-plot-2` is `#FFA07A` (third colour)                                                          |
| D13 | The grammar's title overrides the front-matter title even when it is blank, because the xychart grammar calls `setDiagramTitle` directly. This differs from the rule in `canonical-model.md` (`populateCommonDb` sets the grammar's title only when non-empty). | front matter `title: From front matter`, body `title " "` | probed: title ``                                                                                          |
| D14 | `title ""` is a parse error (P5), but `title " "` is accepted, and it blanks the title as D13 says.                                                                                                                                                             | `title ""`; `title " "`                                   | probed                                                                                                    |
| D15 | An x-axis band or range replaces the axis but keeps the title, and does not touch the plots already built.                                                                                                                                                      | `x-axis [a, b]`, `x-axis 0 --> 5`, `bar [1, 2]`           | probed: linear 0 to 5 with data `["0",1],["5",2]`                                                         |
| D16 | A plot with no data (`line []`) is a parse error. A chart with a title and no plot parses; it fails at render (6.8, R1).                                                                                                                                        | `line []`; `title "x"`                                    | probed                                                                                                    |
| D17 | A one-point plot gives its point the x string `"1"`, and when no x range was set before, the x range becomes 1 to 1 (step 2 of 3.3 with `data.length` 1).                                                                                                       | `line [1]`                                                | probed: data `[["1", 1]]`; the range `[1, 1]` is read from `:151-157` and the step 2 arithmetic, unprobed |

---

## 4. Canonical model proposal

The model follows `docs-port/canonical-model.md`: `version: 1`, camelCase keys, strings always present (`""`, not `null`), numbers as JSON numbers, non-finite numbers as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`.

```json
{
  "version": 1,
  "orientation": "vertical",
  "title": "Sales Revenue",
  "accTitle": "",
  "accDescr": "",
  "xAxis": { "type": "band", "title": "Months", "categories": ["jan", "feb"] },
  "yAxis": { "type": "linear", "title": "Revenue (in $)", "min": 4000, "max": 11000 },
  "plots": [
    {
      "type": "line",
      "title": "avg",
      "strokeFill": "#FFF4DD",
      "strokeWidth": 2,
      "data": [
        ["jan", 5000],
        ["feb", null]
      ],
      "pointLabels": ["Start", ""]
    },
    {
      "type": "bar",
      "title": "",
      "fill": "#8493A6",
      "data": [
        ["jan", 6000],
        ["feb", 7500]
      ]
    }
  ]
}
```

Rules of the model:

- `title` is `getDiagramTitle()`, the sanitised header title. Do not use `getXYChartData().title`: `getDrawableElem` sets it, so it is `""` after a parse alone (probed).
- The x value of a plot point is the string the DB stores: the category for a band axis, and `String(min + i*step)` for a linear one. Keep the string, so `1.6666666666666667` and `1e-7` match.
- An absent value (`undefined`, D5) is `null`. `canonical-model.md` has no rule for absent values; this is a proposal (section 8, decision 2).
- Non-finite numbers follow the rule of `canonical-model.md`: `"Infinity"` and `"-Infinity"` in the y range of a chart without plots (D16). A `NaN` from the DB does not occur, because the grammar gives finite numbers.
- Palette colours are included: the DB chooses them from the palette by index. `dataLabelColor` is a theme value, not a DB value, so it is not in the model; the geometry harness checks it through the `fill` attribute (6.8, R3).
- Left out: `plotIndex`, the `has*` flags, the tmp SVG group, and the config. The config belongs to `pnpm port:diff config`.
- Lines and bars keep their order, so the list needs no key sort.

---

## 5. Specs

| File                                           | `it` | `describe` | What it tests                                                                                                                                                                                     | Port layer                                                           |
| ---------------------------------------------- | ---- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `parser/xychart.jison.spec.ts`                 | 58   | 3          | The grammar's DB calls, with a mock `yy`: the header, axes (titles, bands, ranges), plots and their labels, accTitle and accDescr, and the errors (unbalanced brackets, missing data, bad commas) | Parser. Its mock-call checks become the port's DB-call differential. |
| `xychartDb.spec.ts`                            | 6    | 2          | Sanitised legend titles; truncation of bars and lines to the category count; equal, shorter and longer data; the y range from the cut data                                                        | DB                                                                   |
| `chartBuilder/components/chartTitle.spec.ts`   | 8    | 2          | Title placement in vertical and horizontal charts, the legend exclusion, a title wider than the plot, one wider than the chart, and dropping a title that does not fit (`chartTitle.spec.ts:131`) | Layout, with a stub text measure                                     |
| `chartBuilder/components/legend.spec.ts`       | 3    | 1          | Marker and label drawables, the no-legend gate, and the integration through `XYChartBuilder.build`                                                                                                | Layout, with a stub text measure (`legend.spec.ts:12`)               |
| `chartBuilder/textDimensionCalculator.spec.ts` | 3    | 1          | Conversion of a measured size by the screen scale (`getScreenCTM`), with `computeDimensionOfText` mocked to 6 by 3                                                                                | Text                                                                 |

Total: 78 `it`. None of them tests the renderer, the axes or the plots in isolation, or the orchestrator's arithmetic.

The e2e spec `e2e/rendering/xychart/nightly-check/xyChart.spec.js` (444 lines, 15 `test`) is the only rendering spec for the type. It runs `imgSnapshotTest` (image snapshots, from `e2e/helpers/util.ts:64`) and adds DOM assertions:

| Test (abridged)                                                                      | Assertion                                                                                                                            | Port relevance                                                                                                                                                  |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| legends for named plots                                                              | 3 legend `text`, 2 `path`, 1 `rect`; the three names in `g.legend` (lines 102-107)                                                   | legend markers by type                                                                                                                                          |
| correct distances between points                                                     | the `svg` is visible (line 122)                                                                                                      | none beyond the snapshot                                                                                                                                        |
| data labels, vertical and horizontal, several variants (the many-bars ones included) | label text boxes checked against the bars with `width > 0 && height > 0` (helper `assertBarLabelsWithinBars` at the top of the file) | the label fit and the pairing with bars; the helper's comment says a value below the axis floor is a "degenerate" rect with no label, consistent with R6 and R7 |
| rotated label; rotation clamped above and below the limit                            | `transform` contains `rotate(N)` (line 81)                                                                                           | labelRotation                                                                                                                                                   |
| title centred over the plot                                                          | the absolute difference of the title's and the plot's centres is below 1 (line 371), both from `getBoundingClientRect`               | title alignment (6.2)                                                                                                                                           |
| x-axis range with the same values                                                    | snapshot, and the line path count (line 389)                                                                                         | R10                                                                                                                                                             |
| legend text inside a wide chart (#8283)                                              | 2 legend `text`; a right margin above 5 px (line 411, 422)                                                                           | legend placement when scaled                                                                                                                                    |
| title dropped in a short chart (#8283)                                               | no `chart-title` text; the line path present (lines 441-442)                                                                         | title drop (6.2)                                                                                                                                                |

The helper reads `getBBox()` of the label `text` elements. That is the glyph box of section 6.4, and it agrees with `getBoundingClientRect` in the probes.

---

## 6. Renderer

### 6.1 Pipeline and the DOM it writes

`draw` (`xychartRenderer.ts:13-258`) does the following, in order:

1. Reads the theme and the config from the DB (14-16). It then reads `labelData = plots[0].data.map(d => d[1])` (17). This line is the cause of R1.
2. Selects the root `svg` (32), appends `g.main` (33) and `rect.background` with the width and height (34-38), then `configureSvgSize(svg, height, width, true)` (41), the `viewBox` (43) and the background `fill` (45).
3. Appends the measuring group `g.mermaid-tmp-group` (47). Its children are removed after each measurement, so it stays as an empty group.
4. `getDrawableElem()` (49), which runs the layout (6.2) and returns the list in the order of `Object.values(componentStore)` (`orchestrator.ts:26-59, 213`): title, plot, legend, x-axis, y-axis.
5. For each shape with data (80-83): `getGroup(groupTexts)` (63-78) creates one `g` per level with the class of that level. The cache key is the concatenation of the classes, with no separator. Then it appends the elements by type: `rect` (88-228, with the data labels), `text` (229-244), `path` (245-255).

The DOM, as the reference writes it (probed for the structure, and the element and attribute order is the order of the calls):

```
<svg id width="100%" xmlns style="max-width: Wpx;" viewBox="0 0 W H"
     role="graphics-document document" aria-roledescription="xychart">
  [<title id="chart-title-ID">, <desc id="chart-desc-ID"> when accTitle/accDescr, and aria-describedby / aria-labelledby on the svg]
  <style>...</style>                       (base style sheet, 6.5)
  <g></g>                                  (appendDivSvgG, mermaidAPI.ts:419)
  <g class="main">
    <rect width height class="background" fill=backgroundColor>
    <g class="chart-title"><text ...>title</text></g>            (only when shown)
    <g class="plot">
      <g class="bar-plot-i"> <rect>... <text> (data labels) ... </g>
      <g class="line-plot-i"> <path/> [<g class="labels"><text>...</g>] </g>
    </g>
    <g class="legend">
      <g class="markers"> <rect/> ... <path/> ... </g>
      <g class="label"> <text/> ... </g>
    </g>                                                         (only when a legend shows)
    <g class="bottom-axis">                                      (vertical: x-axis)
      <g class="axis-line"><path/></g>
      <g class="label"><text/>...</g>
      <g class="ticks"><path/>...</g>
      <g class="title"><text/></g>
    </g>
    <g class="left-axis">                                        (vertical: y-axis; horizontal: x-axis)
      <g class="axisl-line"><path/></g>                          (the typo, R14)
      <g class="label">...</g> <g class="ticks">...</g> <g class="title">...</g>
    </g>
    <g class="top-axis">                                         (horizontal: y-axis)
      <g class="axis-line"> <g class="label"> <g class="ticks"> <g class="title">
    </g>
  </g>
  <g class="mermaid-tmp-group"></g>
</svg>
```

Only the groups that have elements are written: `shape.data.length === 0` is skipped (`xychartRenderer.ts:81-83`), and a group is created only by `getGroup` for a shape that exists. An axis with no tick labels writes no `label` group. The order inside a group is the order of its shapes.

Attribute order (the order d3 applies them, which the probes show):

- `svg`: `id`, `width`, `xmlns`, `style`, `viewBox`, `role`, `aria-roledescription` (and `aria-describedby`, `aria-labelledby` when there are accessibility texts).
- `rect` (bar): `x`, `y`, `width`, `height`, `fill`, `stroke`, `stroke-width`.
- `text` (axis, title, legend, line label): `x="0"`, `y="0"`, `fill`, `font-size`, `dominant-baseline`, `text-anchor`, `transform="translate(X, Y) rotate(R)"`. The text content is the string, set as the node's text.
- `text` (data label): `x`, `y`, `text-anchor`, `dominant-baseline`, `fill`, `font-size` (with the `px` suffix, R2).
- `path`: `d`, `fill` (`none` for lines and axes), `stroke`, `stroke-width`.

d3-selection semantics that the port must copy:

- `attr(name, undefined)` or `null` removes the attribute. The `y` of a bar with a missing value is one case (R6).
- `attr(name, number)` writes the JavaScript string of the number.
- `text(value)` sets the text content to the string; the value is not HTML-parsed.
- `selectAll().data(list).enter().append()` makes one element per item, in order.

### 6.2 Layout

The chart is laid out by the type's own arithmetic. The orchestrator (`orchestrator.ts`) builds the components in the order title, plot, legend, x-axis, y-axis (26-59). Then `calculateSpace` (201-207) runs one of two functions. The plot first reserves `p` percent of the width and of the height, where `p` is `plotReservedSpacePercent` (default 50).

**Vertical** (`calculateVerticalSpace`, 61-128):

1. `chartWidth = floor(W*p/100)`, `chartHeight = floor(H*p/100)`. The plot takes those (`plot.calculateSpace`, 67-74). `availableWidth = W - plot.width`, `availableHeight = H - plot.height`.
2. Title: `title.calculateSpace({W, availableHeight})`, `plotY = title.height`, `availableHeight -= title.height` (78-83).
3. x-axis at the bottom: `calculateSpace({availableWidth, availableHeight})`, then `availableHeight -= height` (84-89).
4. y-axis at the left: `calculateSpace({availableWidth, availableHeight})`, `plotX = width`, `availableWidth -= width` (90-96).
5. Legend: `calculateSpace({availableWidth, chartHeight})`, then `availableWidth -= legend.width` (97-101).
6. Leftover width goes to `chartWidth` and leftover height to `chartHeight`, if positive (102-109).
7. `plot.calculateSpace({chartWidth, chartHeight})` again (110-113).
8. Positions (115-124): plot at `(plotX, plotY)`; `title.alignTo(plotX, plotX + chartWidth)`; legend at `(plotX + chartWidth, plotY + max((chartHeight - legend.height)/2, 0))`; x-axis `setRange([plotX, plotX+chartWidth])` and box at `(plotX, plotY+chartHeight)`; y-axis `setRange([plotY, plotY+chartHeight])` and box at `(0, plotY)`.
9. If any plot is a bar, `xAxis.recalculateOuterPaddingToDrawBar()` (125-127).

**Horizontal** (`calculateHorizontalSpace`, 130-199): the same steps, with the x-axis on the left (`setAxisPosition('left')`, 154) and the y-axis on the top (`'top'`, 161). The title's bottom edge is `titleYEnd`, and `plotY = titleYEnd + y-axis height` (167). The axis ranges swap: `yAxis.setRange([plotX, plotX+chartWidth])` (192), `xAxis.setRange([plotY, plotY+chartHeight])` (194).

**Title** (`chartTitle.ts`):

- `calculateSpace`: the measured title height `h` plus `2*titlePadding` must fit the available height. The title shows only when `showTitle` is on and the title is non-empty (44-48). Its width is the larger of the title's width and the available width, but only the height matters for layout (42-43).
- `alignTo(start, end)` (64-72): `halfWidth = min(titleWidth, W)/2`; `centre = min(max((start+end)/2, halfWidth), W - halfWidth)`; `x = centre - halfWidth`; width `2*halfWidth`.
- Drawn (73-94): text at `(x + width/2, y + height/2)`, `dominant-baseline: middle`, `text-anchor: middle`, `font-size: titleFontSize`, `fill: titleColor`.

**Legend** (`legend.ts`):

- Visible plots are the plots with a non-empty title, if `showLegend` is on (55-57).
- With `fs = legendFontSize`: marker `ms = 0.75 fs`, marker spacing `mg = 0.35 fs`, item spacing `is = 0.5 fs` (18-36).
- Size: width `2*legendPadding + ms + mg + maxTextWidth`, height `2*legendPadding + n*fs + (n-1)*is`. It shows only if both fit the available space (74-87). The text width is measured at `fs` (68-71).
- Drawn (95-156): `startX = box.x + legendPadding`, `startY = box.y + legendPadding`, `rowH = fs + is`. A bar marker is a `rect` at `(startX, startY + i*rowH)`, size `ms`. A line marker is `M startX,my L startX+ms,my` with `my = startY + i*rowH + ms/2`. The label is at `(startX + ms + mg, my)`, `dominant-baseline: middle`, `text-anchor: start`, `fill: legendTextColor`.

**Plots** (`plot/index.ts`): the plot component stores its box (`calculateSpace` sets the size, `setBoundingBoxXY` the position). Plots draw in their order, with the index `i` of the plot in the list (`bar-plot-i`, `line-plot-i`) (53-88).

- **Bar** (`barPlot.ts:14-59`): each point becomes `[xAxis.getScaleValue(x), yAxis.getScaleValue(y)]` (15-18). `barWidth = min(xAxis.outerPadding*2, xAxis.tickDistance) * (1 - 0.05)` and `half = barWidth/2` (20-25). Vertical: `x = px - half`, `y = py`, `width = barWidth`, `height = (plot.y + plot.height) - py` (44-58). Horizontal (the first number of each pair is the category's pixel row): `x = plot.x`, `y = categoryRow - half`, `height = barWidth`, `width = py - plot.x` (27-43). Fill and stroke are the bar's colour, `stroke-width` 0.
- **Line** (`linePlot.ts:14-94`): the same points (15-18). The path is `line().x(d[0]).y(d[1])` for vertical and `line().y(d[0]).x(d[1])` for horizontal (20-29), with d3's default linear curve. An empty path gives no element (30-32). Element: `path`, `fill="none"`, `stroke` = line colour, `stroke-width` 2 (34-45). Labels (48-91): for each point with a non-empty label, offset 10 and font size 12. Vertical: `(px, py - 10)`, `text-anchor: middle`. Horizontal: `(py + 10, px)`, `text-anchor: start`. `dominant-baseline: middle`, `fill` = line colour. Drawn in `g.labels` after the path.

**Axes** (`baseAxis.ts`, `bandAxis.ts`, `linearAxis.ts`):

- Scale: a band axis uses `scaleBand` with `paddingInner(1)`, `paddingOuter(0)`, `align(0.5)` over `getRange()` (`bandAxis.ts:28-36`). A linear axis uses `scaleLinear` over `[min, max]`, reversed when the axis is on the left (`linearAxis.ts:27-33`): the largest value is at the top of the y-axis.
- `getRange()` is `[r0 + outerPadding, r1 - outerPadding]` (`baseAxis.ts:53-55`). Both axes are inset this way. The y-axis padding is half its label height, capped at 20 percent of the height (`baseAxis.ts:146-147`). Probed: in `x-axis-range-same-value`, the plot's bottom is y 436 (the x-axis line is at 437) and the value 0 is at y 427.5, which is 436 minus 8.5. `getTickDistance()` is `|range0 - range1| / tickCount` (68-71). `recalculateOuterPaddingToDrawBar` sets `outerPadding = floor(0.7*tickDistance/2)` when `0.7*tickDistance > 2*outerPadding` (84-89).
- Ticks: band axes use the category list, linear axes use `scale.ticks()` with the default count 10 (`linearAxis.ts:23-25`).
- Space, horizontal axis (`calculateSpaceIfDrawnHorizontally`, 91-136). Use `availableHeight` and `availableWidth`:
  1. Axis line: if `showAxisLine` and `availableHeight > axisLineWidth`, the line takes its width (93-96).
  2. Labels: `labelDim` is the largest tick text measure (`getLabelDimension`, 77-82). `outerPadding = min(labelDim.width/2, 0.2*availableWidth)` (99-100). This is set even when the labels turn out not to fit (R15). The height is `labelDim.height`, larger when a bottom label is rotated: `max(h, |sin θ|*w + |cos θ|*h)` (103-109), plus `2*labelPadding`. If it fits, `showLabel` and the height is taken (110-116).
  3. Tick: if `showTick` and `availableHeight >= tickLength`, the tick takes `tickLength` (118-121).
  4. Title: `titleDim` is the title measure at `titleFontSize`. Its height plus `2*titlePadding` must fit (122-133).
  5. `boundingRect.width = availableWidth`, `height = availableSpace.height - availableHeight` (134-135).
- Space, vertical axis (`calculateSpaceIfDrawnVertical`, 138-172): the same steps across the width. The label outer padding is `min(labelDim.height/2, 0.2*availableHeight)` (146-147). The label width is `labelDim.width + 2*labelPadding` (148). The title uses its line height: `titleDim.height + 2*titlePadding` (163).
- Drawing, left axis (201-276): axis line `M x,y0 L x,y1` with `x = bx + bw - lw/2` (204-212). Labels: `(bx + bw - labelPadding - tick - lw, scale(tick))`, `dominant-baseline: middle`, `text-anchor: end` (219-239). Ticks: `M x,s L x-tickLength,s` (240-256). Title: `(bx + titlePadding, by + bh/2)`, `rotate(270)`, `dominant-baseline: text-before-edge`, `text-anchor: middle` (257-274).
- Drawing, bottom axis (277-353): axis line at `y = by + lw/2` from `bx` to `bx + bw` (279-294). Labels at `(scale(tick) + offset, by + labelPadding + tick + lw + |offset_w|)`, `rotate(angle)`, `dominant-baseline: text-before-edge`, `text-anchor: middle` (295-315). Ticks `M s,y L s,y+tickLength` (316-329). Title at `(range0 + (range1 - range0)/2, by + bh - titlePadding - titleTextHeight)` (330-351).
- Drawing, top axis (354-429): axis line at `y = by + bh - lw/2` (357-370). Labels at `(scale(tick), by + (showTitle ? titleTextHeight + 2*titlePadding : 0) + labelPadding)` (372-390). Ticks (391-409). Title at `(bx + bw/2, by + titlePadding)` (410-427).

### 6.3 d3 modules: what the port needs

| Module                                                                                            | Used for                      | What a port must reproduce                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Size           |
| ------------------------------------------------------------------------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `scaleBand` (d3-scale `band.js`)                                                                  | band x-axis                   | Domain with the first index of a duplicate (the `ordinal` map). `n` is the number of unique categories. `step = (stop - start) / max(1, n - paddingInner + 2*paddingOuter)` = range/(n - 1) here. `start += (stop - start - step*(n - paddingInner)) * align`; `bandwidth = step*(1 - paddingInner)` (unused). `scale(value)` is the value's index; an unknown value gives `undefined`, and the axis falls back to `range[0]` (`bandAxis.ts:42-44`). With `n = 1` the one band is at the middle (probed: range 10 to 110 gives 60). Duplicate categories (probed `[a, a, b]`): `a` at 0, `b` at 100. | about 40 lines |
| `scaleLinear` (d3-scale `linear.js`, `continuous.js`)                                             | y-axis, linear x, every value | Coercion `+x`. `null`, `undefined`, `NaN` and the string `"abc"` give `undefined` (probed); the empty string coerces to 0 and `true` to 1 (probed by their outputs). `normalize(a, b)(x) = (x - a)/(b - a)`, or the constant 0.5 when `b - a = 0` (`continuous.js:12-16`, probed: domain `[5, 5]` maps everything to the middle). Interpolation `a*(1 - t) + b*t` (`number.js:3`; the order matters for the last digit). A reversed domain interpolates the range backwards (`bimap`, `continuous.js:26-31`).                                                                                        | about 50 lines |
| `ticks` (d3-array `ticks.js`)                                                                     | linear axis ticks             | `ticks(start, stop, 10)`: `start === stop` gives `[start]`; `count <= 0` gives `[]`. `tickSpec` (`ticks.js:5-27`): `step = (stop-start)/count`, `power = floor(log10(step))`, `error = step/10^power`, factor 10, 5, 2 or 1, then `i1 = round(start/inc)`, `i2 = round(stop/inc)` with the two correction steps, and the two branches for `power < 0` (`inc = 10^-power/factor`, `inc` negated). A reversed domain gives the same values in descending order (probed: `[10, 0]` gives 10 down to 0).                                                                                                 | about 40 lines |
| `line` (d3-shape `line.js`, `curve/linear.js`) and `path` (d3-path `path.js`)                     | line plot path                | `M` for the first point and `L` for the rest, written as `M${x},${y}` and `L${x},${y}` with no space. No `defined` test, so `NaN` is written as `NaN` (probed: `LNaN,1`). A single point ends with `Z` (`curve/linear.js:15-17`, probed: `M361.32,255.5Z`). `digits` is 3 (`path.js:4-6`): each coordinate is `Math.round(v*1000)/1000` (`path.js:13-23`). A value is written with JavaScript's `String` (probed: `1e+21`).                                                                                                                                                                          | about 40 lines |
| `select`, `append`, `attr`, `text`, `remove`, `node`, `selectAll().data().enter()` (d3-selection) | DOM                           | Not ported. See 6.1 for the `attr` and `text` semantics.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | none           |
| `interpolateNumber` (d3-interpolate)                                                              | inside `scaleLinear`          | The formula above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | in the scale   |

A port can keep the arithmetic of the reference in `f64` in the same order of operations. Two places are not plain arithmetic: the ticks use `Math.round` with ties towards plus infinity (`ticks.js:13-22`), and `Math.floor` is applied to the widths and heights of the layout (`orchestrator.ts:67-68, 137-140`).

### 6.4 Text measurement

Four things are measured. Each measurement goes through `TextDimensionCalculatorWithFont.getMaxDimension(texts, fontSize)` (`textDimensionCalculator.ts:11-45`), which returns the largest width and the largest height of the texts, not their sum.

| Measured                                             | Call                                                      | Font size             | What it feeds                                                                          |
| ---------------------------------------------------- | --------------------------------------------------------- | --------------------- | -------------------------------------------------------------------------------------- |
| Chart title                                          | `chartTitle.ts:38-41`                                     | `titleFontSize` (20)  | Title height (the layout) and title width (`alignTo`)                                  |
| Tick labels (categories, or the linear tick strings) | `baseAxis.ts:77-82` (`getLabelDimension`)                 | `labelFontSize` (14)  | Outer padding, label space, and the rotated offset (`baseAxis.ts:198`, measured again) |
| Axis titles                                          | `baseAxis.ts:123-126` (horizontal), `:159-162` (vertical) | `titleFontSize` (16)  | Title space; the title's `titleTextHeight`                                             |
| Legend names                                         | `legend.ts:68-71`                                         | `legendFontSize` (14) | Legend width (height is fixed by the font size)                                        |

Not measured: data labels (the fit uses `0.7 * fontSize * length`, `xychartRenderer.ts:120, 172, 193`), line point labels (a fixed 12 px, no measure), and the drawn text itself.

**How it is measured** (`computeDimensionOfText`, `createText.ts:127-140`):

1. A `g` with `visibility="hidden"` and `font-size` set to the size, appended to the tmp group (`textDimensionCalculator.ts:24-27`).
2. A `text` with a `tspan.text-outer-tspan` (`x="0"`, `y="0.9em"`, `dy="1em"`, `createTspan` at `createText.ts:96-108`), with a `tspan.text-inner-tspan` (`font-style: normal`, `font-weight: normal`) that holds the text after `decodeHTMLEntities` (`createText.ts:213-223`, which decodes `&amp;`, `&lt;` and `&gt;` only, and `updateTextContentAndStyles`, `:237-256`).
3. `getBoundingClientRect()` of the outer tspan gives `width` and `height`. `textDimensionCalculator.ts:31-42` divides them by `getScreenCTM()`'s `a` and `d`, which turns screen pixels into user units. At scale 1 the division does nothing.
4. The `text` is removed (`createText.ts:136-137`, `textDimensionCalculator.ts:43`).

**What the box is** (probed):

- `height`: a whole number of pixels at every probed size (10 px gives 12; 12 px gives 14; 14 px gives 17; 20 px gives 25; 30 px gives 37; 39.5 px gives 49). The exception is 16 px, where it is `20.000001907348633`, a float32 value. `ferrigura-text`'s `text_box` gives whole pixels (20), which differs by 2e-6 px. The geometry tolerance is 0.05 px. For the 91 probed pairs the box height equals the `getBBox()` height.
- `width`: for most strings it equals `getComputedTextLength()`, the advance with kerning, snapped to 1/64 px (probed: `Sales Revenue` at 14 px is 89.734375; `1` is 5.640625). For strings with glyphs that overhang their advance it is wider. The box is the union of each glyph's outline box, not the advance. Probed by single glyphs at 14 px: `j` has `rect.x = -0.812` and width 4.3745. That is the `glyf` `xMin` of `j` (-58/1000 em = -0.812 px) and the advance 3.5625. `q` has width 8.792, which is the `xMax` of `q` (628/1000 em = 8.792 px), while its advance is 8.078. Whole strings: `jan` 19.390 against an advance of 18.578; `jun` 19.531 against 18.719; `jul` 15.218 against 14.406; `january` 45.375 against 44.563; `Jj gq` 31.248 against 30.531.
- `getBBox()` of a plain `text` element (the path of `calculateTextDimensions`, `utils.ts:706`) gives the same width as the box above (probed: `jun` 19.531, `jq` 13.160, `quit` 24.000).

**The rule a port needs** (inferred from the probes, unverified for strings with several overhanging glyphs): width = `max(advance_total, max_i(pen_i + xMax_i)) - min(0, min_i(pen_i + xMin_i))`, with `pen_i` the kerned pen position of glyph `i`, the glyph bounds from `glyf`, and the advance snapped to 1/64 px. The first and last glyphs are confirmed by the single-glyph probes. The rest is unverified.

**Consequences.** For xychart the width feeds the layout (outer padding, title centre, legend width), so a 0.8 px error moves the bars and the title. In the 47 fixtures, 234 texts were measured (titles, axis titles, categories, legend names, numbers) at 14, 16 and 20 px. The overhanging ones are "j", "q", "jan", "jun" and "jul" only (`measure6.mts`). The widest month label is "may" (25.16 px at 14 px), which has no overhang, so the fixtures do not depend on the rule. User text can.

### 6.5 Root size, style and theme

- **Size.** `configureSvgSize(svg, height, width, true)` writes `width="100%"` and `style="max-width: Wpx;"` (`setupGraphViewbox.js:23-47`), and the renderer sets `viewBox="0 0 W H"` (`xychartRenderer.ts:41-43`). The `true` is a literal: `config.useMaxWidth` is never read (R19).
- **Style.** No `styles` in `xychartDiagram.ts`, so the sheet is the generic one (`styles.ts`). The probed `<style>` starts with `#id{font-family:"Lexica Ultralegible";font-size:14px;fill:#28253D;}`, then the generic keyframes and the `edge-animation` rules. The port uses `base_style_sheet` (1.4).
- **Theme values read by the chart** (`xychartDb.ts`, `chartBuilder/`): `backgroundColor`, `titleColor`, `legendTextColor`, `xAxisTitleColor`, `xAxisLabelColor`, `xAxisTickColor`, `xAxisLineColor`, `yAxisTitleColor`, `yAxisLabelColor`, `yAxisTickColor`, `yAxisLineColor`, `plotColorPalette`, and `dataLabelColor` (6.8, R3).
- **The two shipping themes** (`theme-redux-color.js:364-379`, `theme-redux-dark-color.js:381-396`) define all of the above except `dataLabelColor`. Their values are `primaryTextColor` (`#28253D` light; the dark one is probed as `#e0dfdf` for the title), `background` (`#ffffff` light; `#333` dark) and the palette `#FFF4DD,#FFD8B1,#FFA07A,#ECEFF1,#D6DBDF,#C3E0A8,#FFB6A4,#FFD74D,#738FA7,#FFFFF0` (`theme-redux-color.js:376-378`).
- **Other themes** fall back to one of the two (M6-PLAN, Themes). The harness forces the fallback on both sides (`common.ts:77-86`).

### 6.6 Configuration section

Defaults from the schema (`config.schema.yaml:1408-1563`; `defaultConfig.ts:271-274` adds `useWidth: undefined`).

| Key (`xyChart.`)               | Default         | Read by                                                      |
| ------------------------------ | --------------- | ------------------------------------------------------------ |
| `width`                        | 700             | `chartConfig.width` (renderer 36, orchestrator 62, title 65) |
| `height`                       | 500             | renderer 37, orchestrator 63                                 |
| `titleFontSize`                | 20              | `chartTitle.ts:40, 81`                                       |
| `titlePadding`                 | 10              | `chartTitle.ts:43`                                           |
| `showTitle`                    | true            | `chartTitle.ts:46`                                           |
| `showLegend`                   | true            | `legend.ts:55`                                               |
| `legendFontSize`               | 14              | `legend.ts:66, 100`                                          |
| `legendPadding`                | 10              | `legend.ts:74-76, 104-105`                                   |
| `showDataLabel`                | false           | renderer 102                                                 |
| `showDataLabelOutsideBar`      | false           | renderer 103                                                 |
| `chartOrientation`             | `vertical`      | orchestrator 202, plot 66, renderer 105                      |
| `plotReservedSpacePercent`     | 50 (minimum 30) | orchestrator 67-69, 137-139                                  |
| `xAxis` / `yAxis` (each below) |                 |                                                              |

| Key (`xyChart.xAxis.` or `.yAxis.`) | Default        | Read by                                                         |
| ----------------------------------- | -------------- | --------------------------------------------------------------- |
| `showLabel`                         | true           | `baseAxis.ts:97, 144`                                           |
| `labelFontSize`                     | 14 (minimum 1) | `baseAxis.ts:80` (measure), `:233, 309, 384` (drawing)          |
| `labelPadding`                      | 5              | `baseAxis.ts:110, 148` (space), `:228, 304, 382` (drawing)      |
| `showTitle`                         | true           | `baseAxis.ts:122, 158`                                          |
| `titleFontSize`                     | 16             | `baseAxis.ts:125, 161` (space), `:267, 344, 420` (drawing)      |
| `titlePadding`                      | 5              | `baseAxis.ts:127, 163` (space), `:264, 341, 381, 418` (drawing) |
| `showTick`                          | true           | `baseAxis.ts:118, 154`                                          |
| `tickLength`                        | 5              | `baseAxis.ts:120, 156`                                          |
| `tickWidth`                         | 2              | `baseAxis.ts:253, 326, 406`                                     |
| `showAxisLine`                      | true           | `baseAxis.ts:93, 140`                                           |
| `axisLineWidth`                     | 2              | `baseAxis.ts:93-94, 140-141`                                    |
| `labelRotation`                     | 0 (-90 to 90)  | `baseAxis.ts:37-40`, out of range gives 0                       |

Notes:

- The config is merged with `lodash merge` (`utils.ts:841-843`), so a partial `xAxis` keeps the other defaults. An undefined value does not replace a default.
- Numeric strings are read as JavaScript reads them. The fixture `should-use-all-the-config-from-directive` sets `labelFontSize: "20"`, and the reference writes `font-size="20"`. A string coerces in `*` and `-`, but `+` concatenates. `titlePadding` is added with `+` in two places (`baseAxis.ts:264, 418`). A string `titlePadding: "5"` therefore gives `translate(05, …)` (probed, R20). The port's typed config reads `"5"` as 5 (`config/lenient.rs`), so it would not reproduce this. Section 8, decision 3.
- Unknown keys are ignored (`plotBorderWidth` in the directive fixture).

### 6.7 Randomness and the clock

None. `grep` of the folder for `random`, `Date` and `now` finds nothing outside the specs. The only identifiers are the svg id and the accessibility ids (`chart-title-<id>`, `chart-desc-<id>`), which come from the generic code.

### 6.8 Renderer quirks and the DOM the port must reproduce

| Id  | Quirk                                                                                                                                                                                                                                                                                                                     | Evidence                                                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | The data labels read their numbers from `plots[0]` (`xychartRenderer.ts:17, 113, 165`), for every bar. A bar after a line shows the line's values. A first plot shorter than a bar makes `labelData[i]` undefined, and `.toString()` throws, so the whole render fails. With no plot at all, the render fails at line 17. | probed: `line [1, 2]` then `bar [100, 200]` with labels shows `1`, `2`. `line [1]` then `bar [5, 6]` with labels gives `Cannot read properties of undefined (reading 'toString')`. A chart with only a title gives `Cannot read properties of undefined (reading 'data')`. |
| R2  | The data-label `font-size` is written as `${size}px` (`:158, :224`). The size is the smallest candidate across the bars, floored, and it has no upper bound.                                                                                                                                                              | probed: a bar of 1234567890123 labels at `18px`; the two bars 4 and 8 on a 0 to 10 range label at `159px` (the dark theme probe), because the candidate is limited by the bar height only                                                                                  |
| R3  | `dataLabelColor` is missing from both shipping themes, so the DB merges it from the classic default theme (`xychartDb.ts:42-46`, `theme-default.js:373`, `:503-507`). The data label `fill` is then `#131300` in every theme.                                                                                             | probed: `fill="#131300"` in `redux-color` (the default) and `redux-dark-color`, where the background is `#333`                                                                                                                                                             |
| R4  | The label size starts at `width/(len*0.7)` (vertical) or `height*0.7` (horizontal) and drops by 1 until the label fits, so the candidates are not whole numbers. The result is `floor(min)`, which can be negative (`start` 0.5 gives -0.5, then -1). Negative `font-size` is invalid SVG.                                | read (`:127-138, 191-203`); the negative case is unprobed                                                                                                                                                                                                                  |
| R5  | Data labels are only drawn for bars (`rect` shapes). A line plot does not get them.                                                                                                                                                                                                                                       | read (`:88-102`)                                                                                                                                                                                                                                                           |
| R6  | A bar with a missing value (D5) is written with no `y` and `height="NaN"`. The label code drops it because its width or height is not positive.                                                                                                                                                                           | probed: `<rect x="404.88" width="108.3" height="NaN" …>`                                                                                                                                                                                                                   |
| R7  | A horizontal bar below the axis minimum gets a negative `width`.                                                                                                                                                                                                                                                          | probed: `horizontal-neg-out-of-range`: `width="-322.4140625"`                                                                                                                                                                                                              |
| R8  | Duplicate categories overlap: both bars sit at the first position.                                                                                                                                                                                                                                                        | probed: two `rect`s at `x="39.97"`                                                                                                                                                                                                                                         |
| R9  | Plots declared before a band x-axis (D2) all sit at the first category.                                                                                                                                                                                                                                                   | probed: a line path `M40.141,457.5L40.141,233L40.141,8.5`, all at the first label `a`                                                                                                                                                                                      |
| R10 | A zero-width domain maps every value to the middle.                                                                                                                                                                                                                                                                       | probed: `x-axis 0 --> 0` gives `M335.73,427.5L335.73,240.5L335.73,53.5`; `line [5, 5]` gives y 233, the middle                                                                                                                                                             |
| R11 | A single-point line writes `M x,yZ`.                                                                                                                                                                                                                                                                                      | probed: `M361.32,255.5Z`                                                                                                                                                                                                                                                   |
| R12 | Line paths are rounded to three decimals (`digits` 3). Axis, tick, title and bar coordinates are not.                                                                                                                                                                                                                     | probed: `M43.695,457.5L367.055,8.5L690.414,233`; axis `M 34.109375,467 L 700,467`                                                                                                                                                                                          |
| R13 | Bars run from the bottom of the plot, not from zero. The y range is inset by the y-axis outer padding at both ends (6.2), so the smallest value's bar is not empty.                                                                                                                                                       | probed: `auto-range-no-zero` (bars `5000, 6000, 7500`, auto range) has bottoms at y 466, the 7500 bar at height 457.5 and the 6000 bar at height 188.1; the 5000 bar is 8.5 px high, the padding, not 0. Negative values behave the same way (`neg-bars-labels-vertical`). |
| R14 | The left axis line has the class `axisl-line` (typo). The bottom and top axis lines have `axis-line`.                                                                                                                                                                                                                     | read (`baseAxis.ts:207`); probed                                                                                                                                                                                                                                           |
| R15 | The outer padding of an axis is set before the fit check. A label row that does not fit is hidden, but its padding still moves the bars and the plot.                                                                                                                                                                     | read (`baseAxis.ts:97-100` before `:113`); unprobed                                                                                                                                                                                                                        |
| R16 | On a linear x-axis a bar is sized by the tick spacing, not by the point spacing (`barPlot.ts:22-25`, `tickDistance` from the tick count).                                                                                                                                                                                 | probed: `correct-distances-between-data-points`, bars 38 px wide for 4 points on `0 --> 2`                                                                                                                                                                                 |
| R17 | Text is written as the sanitised string. `&amp;` shows as `&amp;`. The final SVG is decoded only for the placeholders of `encodeEntities` (`utils.ts:904-906`).                                                                                                                                                           | probed: title `A &amp; B` appears in the SVG as `A &amp;amp; B` (the browser shows `&amp;`), and `#quot;hi#quot;` shows as `"hi"`                                                                                                                                          |
| R18 | The DOM keeps the empty `<g>` before `main` (`mermaidAPI.ts:419`) and the empty `g.mermaid-tmp-group` last. Both are written in all 47 fixtures.                                                                                                                                                                          | probed                                                                                                                                                                                                                                                                     |
| R19 | `configureSvgSize` is called with `useMaxWidth` set to `true`, regardless of the config (`xychartRenderer.ts:41`).                                                                                                                                                                                                        | read; probed (every root is `width="100%"`)                                                                                                                                                                                                                                |
| R20 | A numeric string in `titlePadding` concatenates: `translate(05, …)`.                                                                                                                                                                                                                                                      | probed: `titlePadding: "5"` gives `translate(05, 217.99…) rotate(270)`; `titlePadding: 5` gives `translate(5, …)`                                                                                                                                                          |
| R21 | The plots' colours follow the position of the plot, including bars (D12).                                                                                                                                                                                                                                                 | probed                                                                                                                                                                                                                                                                     |
| R22 | The y axis with an inverted range (`y-axis 10 --> 0`) puts 0 at the top and 10 at the bottom, the reverse of the normal axis. The labels are written in ascending order, so the first label in the DOM is at the top.                                                                                                     | probed: `inverted-y-range`: the label `0` is at y 8.5 (the top); the labels are written 0 to 10                                                                                                                                                                            |

---

## 7. Fixtures

### 7.1 Count and state

- `e2e/diagrams/xychart/`: 47 `.mmd` files. `nightly-check/` has 37, `pr-check/` has 10, and `visual-check/` has only `.gitkeep`.
- Probed: all 47 render without error in the reference, through the harness page (`patchText` settings). Their SVGs are in `fixtures-svg/`.
- Non-ASCII characters appear only in the front-matter descriptions (7 files: em dashes and `°`). No chart text has a non-ASCII character, and none has `&` or `#`. No fallback font is needed.
- `e2e/platform/dev-diagrams/diagrams/xychart/` holds two more copies (identical, checked with `diff`) of `should-drop-the-title-when-the-chart-is-too-short.mmd` and `should-keep-legend-inside-a-wide-chart-scaled-to-fit.mmd`. They are not part of the 47.

### 7.2 Features the fixtures exercise

Counts are by reading each file, except where marked "by pattern".

| Feature                                                      | Files          | Notes                                                                                                                                                             |
| ------------------------------------------------------------ | -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Horizontal orientation (keyword or config)                   | 12             | 10 nightly, 2 pr                                                                                                                                                  |
| `showDataLabel: true`                                        | 18             | 13 nightly, 5 pr                                                                                                                                                  |
| `showDataLabelOutsideBar`                                    | 2              | vertical and horizontal                                                                                                                                           |
| Line plots                                                   | by pattern: 25 |                                                                                                                                                                   |
| Bar plots                                                    | by pattern: 37 |                                                                                                                                                                   |
| Point labels on lines                                        | 4              | `should-render-a-horizontal-line-chart-with-point-labels`, `…mixed-labels…`, `…line-chart-with-point-labels`, `…multiple-lines-where-only-one-has-labels`         |
| Named plots (legend)                                         | 4              | `legends-for-named-plots`, `should-drop-the-title…` (CPU), `should-keep-legend…` (CPU, RAM), `x-axis-range-same-value` (dirac)                                    |
| Linear x-axis (`-->`)                                        | 4              | `correct-distances…`, `x-axis-range-same-value`, `should-drop-the-title…`, `should-keep-legend…`                                                                  |
| Band x-axis (`[…]`)                                          | by pattern: 38 |                                                                                                                                                                   |
| Y range declared (`-->`)                                     | by pattern: 36 |                                                                                                                                                                   |
| No y-axis declaration                                        | 5              |                                                                                                                                                                   |
| Chart title (inline or front matter)                         | by pattern: 37 | `x-axis-range-same-value` takes its title from the front matter                                                                                                   |
| No title                                                     | 10             | by pattern (47 minus 37 with a title), for example `should-render-a-chart-without-title`, `correct-distances…`, the two spark charts                              |
| `theme:` key                                                 | 3              | `dark` (two spark charts) and `forest` (yaml). The harness forces the fallback.                                                                                   |
| `themeVariables.xyChart` override                            | 1              | `render-all-the-theme-color` sets `dataLabelColor: "#eeeeee"` (so R3 is not exercised there)                                                                      |
| Directive form `%%{init}%%`                                  | 1              | `should-use-all-the-config-from-directive`                                                                                                                        |
| Config in front matter                                       | 30             | by pattern                                                                                                                                                        |
| `showLabel`, `showTitle`, `showTick`, `showAxisLine` toggles | 8              | by pattern                                                                                                                                                        |
| `labelRotation`                                              | 3              | `label-rotation-clamped-above-limit` (120), `…below-limit` (-100), `rotated-label-45-degrees`                                                                     |
| `plotReservedSpacePercent`                                   | 3              | the two spark charts, and the directive and yaml fixtures                                                                                                         |
| Negative or decimal values                                   | 3              | `should-render-negative-and-decimal-values-with-correct-labels-for-vertical-xy-chart`, `…-for-horizontal-xy-chart`, `decimals-and-negative-numbers-are-supported` |

### 7.3 Fixtures that need care

- `data-labels-within-bars-vertical.mmd` and `data-labels-within-bars-horizontal.mmd` describe values "below the y-axis floor" being shown. They are not: the x-axis has 3 categories and the data has 20 values, so the data is cut to 3 (D4). The reference draws 3 bars (probed: the SVG has 4 `rect`s, the background and 3 bars). The description is wrong, and the fixture is still a valid comparison.
- `should-use-all-the-config-from-directive.mmd` sets `labelFontSize: "20"` (a string) and `plotBorderWidth` (not a key). Both are read as the reference reads them (6.6).
- `render-spark-bar-without-displaying-other-property.mmd` and `render-spark-line-with-plotreservedspacepercent.mmd` use a 200 by 20 chart with `theme: dark`. The harness forces `redux-dark-color` on both sides.
- `should-use-all-the-config-from-yaml.mmd` sets `theme: forest`. The harness forces `redux-color` on both sides.
- `should-drop-the-title-when-the-chart-is-too-short.mmd` (height 60): the title is dropped (probed: no `chart-title` group).
- `x-axis-range-same-value.mmd`: a zero-width x domain (R10) and a front-matter title ("Dirac delta function", probed).
- The fixtures need no exclusion for `handDrawn`, scripts, randomness or a non-layered layout. The harness excludes nothing for xychart unless the port finds a new reason.

---

## 8. Risks, effort and open questions

### 8.1 Risks, ranked

1. **Glyph-box width (6.4).** This is the largest risk for exact geometry in general. It needs a new function in `ferrigura-text`, and it must match Chromium for every glyph that overhangs its advance. The 47 fixtures do not depend on it (section 6.4), but any user text can. A test set of single glyphs and pairs is available in the probe output, and the coordinator may want the reference's own table (`measure*.json`) as a fixture.
2. **Jison lexer (section 2.2).** Thirteen quirks (P1 to P13). All are probed except the lookahead detail of P12, which is read. The scanner must follow the generated order and the state stack, not a general lexer. The journey scanner shows how.
3. **Out-of-range output (R6, R7, R9, R13).** The reference writes NaN heights, negative widths and points outside the plot. The port must write the same text, which is easy, or name these as exceptions, which is a decision (decision 1).
4. **Reference bugs (R1, R3, R8, R9, R14).** They are deterministic and easy to reproduce, but each needs a test, because a "fix" would change the output.
5. **Float exactness (6.3).** Interpolation order, the band step, `Math.round` ties, and JavaScript number formatting. These are small and easy, but a single wrong operation order breaks the 0.05 px tolerance on large charts.
6. **Cross-cutting text rounding (0.3, item 6).** The shared `calculateTextDimensions` path (`utils.ts:706-708`) rounds `getBBox().width`. `ferrigura-render/src/common/text.rs:253` rounds the advance. The two differ for "jun" (20 and 19), "jul" (15 and 14) and "jq" (13 and 12) at 14 px. Whether the M5 probes (`tests/fixtures/chromium.json`) covered such words is not checked here. It is a fix for the M5 and M6 text code, not for xychart alone.
7. **Labels hidden but padded (R15).** Read only. A test with a small chart would confirm it.

### 8.2 Effort

About 2,300 to 2,600 lines of Rust with tests (section 0.2). The order that keeps each step testable:

1. The DB model and the parser differential (no rendering), using the 85 probe inputs and the fixtures' texts.
2. The d3 subset with its own unit tests against the d3 output (`d3probe.mjs`).
3. The glyph-box width, against `measure*.json`.
4. The chart builder: the orchestrator, title and legend, then the axes, then the plots.
5. The renderer DOM and the fixture geometry.

### 8.3 Open questions for the coordinator

1. **Non-finite and out-of-range output (R6, R7, R13).** Reproduce them as the reference writes them (no `y` attribute, `height="NaN"`, a negative `width`), or name them as exceptions in `PLAN.md`? The M6 rule says an exception is named only where the reference "produces garbage". These are garbage in the browser sense (the browser does not draw them), but they are deterministic and the geometry harness can compare them. Recommendation: reproduce them and list them.
2. **`undefined` in the canonical model (D5).** `canonical-model.md` has no rule for an absent value. Proposed: `null`. Confirm, or name another encoding.
3. **Numeric strings in the config (R20).** The typed config reads `"5"` as 5 (`config/lenient.rs`). The reference concatenates `titlePadding` strings in two places. Reproduce (keep the raw value for those two fields), or accept the lenient reading as a named deviation? Recommendation: accept it and name it, since the fix would change the config layer for every diagram.
4. **Glyph-box width (6.4, 0.3 item 6).** Where should the box-width function live? `ferrigura-text` is the natural place. It also changes the M5 and M6 numbers for "jun", "jul" and "jq" (section 0.3, item 6), so it needs a decision on whether to re-run the M5 and M6 text probes. The reference's `getBBox` and `getBoundingClientRect` agree, so the box rule is the same for both call sites.
5. **Differential comparison of parse errors.** The reference's messages list Jison's expected tokens. Recommendation: compare only the outcome (ok or error) and the error line, as the P1 differential does for other Jison types. Confirm.
6. **Data-label colour (R3).** The port must hold the classic default theme's `primaryTextColor` (`#131300` in this build) as the data-label colour for both shipping themes. Confirm that this constant is acceptable, or that a named exception is.
7. **Title precedence (D13).** xychart's blank body title overrides the front matter, which differs from the `populateCommonDb` rule in `canonical-model.md`. The port follows xychart's grammar. Confirm that the canonical rule does not need a change for xychart.
8. **Shared d3 code.** The d3 subset (band, linear, ticks, line path) is not in `ferrigura-render` yet. Propose `common/` (a `d3` module with `band`, `linear`, `ticks` and `line_path`). Another P2 type may use them (for example the quadrant chart). Confirm the location.
9. **Fixture descriptions (7.3).** The description of the two `data-labels-within-bars-*` fixtures is wrong about the data shown. Fix the description or the fixture, or leave it. The fixture still compares correctly.
