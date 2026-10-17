# xychart parser and DB: plan

Status: parser, DB and renderer ported. Reference: this worktree's `packages/mermaid/src/diagrams/xychart/` (`xychartDb.ts`, `parser/xychart.jison`, `xychartRenderer.ts`, `chartBuilder/`). Research and probes are in [NOTES.md](./NOTES.md); this page says what the port does, which quirks it keeps and which it does not. Quirk ids are the notes' ids: `P1`–`P13` for the lexer and grammar (NOTES 2.2), `D1`–`D17` for the DB (NOTES 3.6), and `D18` for a quirk the port's tests found. The renderer quirks `R1`–`R22` (NOTES 6.8) are section 12.

Scope: the parser, the DB, the model and the configuration that the DB reads (sections 1 to 11), and the renderer with its chart builder (section 12).

## 1. Sources

| File                                                                         | Used for                                                                                      |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `diagrams/xychart/parser/xychart.jison` (lines 1-76, 78-176)                 | The rule table (generated order and states), and the grammar (82-176)                         |
| `diagrams/xychart/xychartDb.ts` (26-255)                                     | The DB state, the setters, the plot transform, the palette, `clear`                           |
| `diagrams/common/commonDb.ts` (10-30)                                        | `setDiagramTitle`, `setAccTitle`, `setAccDescription` (sanitising and the whitespace rules)   |
| `diagrams/common/common.ts` (56-82)                                          | `sanitizeText`: DOMPurify with `FORBID_TAGS: ['style']`                                       |
| `Diagram.ts` (47-77), `preprocess.ts` (58-89)                                | Order of `clear`, front matter title, `encodeEntities`, `+ "\n"`; front matter and directives |
| `utils.ts` (`detectInit`, 99-132), `config.ts` (`addDirective`)              | The configuration the DB reads in `clear`                                                     |
| `themes/theme-default.js` (370-386), `themes/theme-redux-color.js` (364-379) | The palette the DB reads (section 6)                                                          |

The Rust code: `crates/ferrigura-syntax/src/xychart/` (`mod.rs` the entry point and the grammar, `scanner.rs` the lexer, `db.rs` the DB and the number formatting), the model in `crates/ferrigura-model/src/xychart.rs`, the configuration resolution in `ferrigura_model::config::effective` and `ferrigura_syntax::config::read`.

## 2. Text before the lexer

The steps of `Diagram.fromText` after `preprocessDiagram`, in the reference's order:

1. CRLF and CR to LF; `="x"` to `='x'` inside tags; front matter removed, and its `title` read (`prepare`, shared).
2. `%%{ … }%%` directives removed (`remove_directives`, shared).
3. Comment lines removed and the text trimmed at the start (`cleanup_comments`, shared).
4. `encodeEntities` (`encode_entities`, shared), then one `\n`.
5. The configuration of the text is read (section 6). It is read before the DB is cleared, as `Diagram.fromText` leaves it.
6. The DB is cleared, then the front matter title is set through `setDiagramTitle` (sanitised, not encoded).
7. The scanner reads the result.

## 3. Scanner

Jison's `next()` takes the first rule that matches at the position, not the longest. The rules run in the generated order, and the states use the lists the generated `conditions` object gives (NOTES 2.2, generated rules 0-45). Keywords are case-insensitive. The scanner (`scanner.rs`) keeps the condition stack: `popState` never empties the initial state.

| Generated rules        | Token or action                                                                                                                                                 | States                                      |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| 0, 1, 5                | skipped comments: `%%` not followed by `{`; one character other than `}` then `%%`; `%%`                                                                        | all                                         |
| 2, 3                   | one line break, leaving the state (axis data, data)                                                                                                             | `axis_data`, `data`                         |
| 4                      | NEWLINE, one or more `\n` or `\r`                                                                                                                               | all but the quoted and accessibility states |
| 6                      | `title\b`                                                                                                                                                       | initial and the axis and data states        |
| 7, 9, 11               | `accTitle:`, `accDescr:` (enter the value state); `accDescr {` (enter the block, no token)                                                                      | initial and the axis and data states        |
| 8, 10, 12, 13          | the value of an accessibility line; the block's `}` and its text                                                                                                | the value states                            |
| 14, 15, 16, 17, 20, 21 | `xychart\b`; `vertical` or `horizontal` (no boundary, P2); `x-axis\b`, `y-axis\b` (enter axis data); `line\b`, `bar\b` (enter data)                             | all inclusive states                        |
| 18, 19, 22, 23, 24     | `[` in axis data (enter band data); `-->`; `[` in data (enter data inner); the number rule (axis data and data inner only); `]` (leave band data or data inner) | per state                                   |
| 25                     | the garbled Markdown rule (P4), see section 8                                                                                                                   | not reproduced                              |
| 26, 27, 28             | `"` enters the string state; `"` leaves it (no token); the text up to `"` as STR                                                                                | the string state                            |
| 29-45                  | `[`, `]`, ALPHA, `:`, `+`, `,`, `=`, `*`, `#`, `_`, `.`, `&`, `-`, NUM, white space (skipped), `;`, end of input                                                | the states the generated list gives         |

Jison's `done` flag is reproduced: once the input is empty, the next call reads the rules of the current state at the end of the input, and the call after that returns Jison's raw end symbol, which the grammar does not accept. A text that ends inside an accessibility value or a quoted string is therefore rejected or gets an empty value, as the reference does.

A character that no rule takes (`}` outside the block, `{`, `<`, a backtick, a lone `%`, a non-ASCII letter outside text) is a lexical error. The scanner stops there and the grammar rejects the text.

## 4. Grammar and DB calls

The grammar is read by recursive descent over the tokens. It accepts the same texts as the LALR tables and makes the DB calls in the order the reductions make them:

```
start      : eol* (XYCHART CHART_ORIENTATION?)* document          (XYCHART keeps its orientation word)
document   : (statement eol*)*
statement  : TITLE text                       setDiagramTitle(trim(text))
           | X_AXIS parseXAxis
           | Y_AXIS parseYAxis
           | LINE text? plotData              setLineData(text or '', points)
           | BAR text? plotData               setBarData(text or '', points)
           | ACC_TITLE value                  setAccTitle(trim(value))
           | ACC_DESCR value                  setAccDescription(trim(value))
           | ACC_DESCR_BLOCK value            setAccDescription(trim(value))
parseXAxis : text                             setXAxisTitle(text)
           | text bandData                    setXAxisBand, then setXAxisTitle(text)
           | text range                       setXAxisRangeData, then setXAxisTitle(text)
           | bandData | range                 setXAxisBand or setXAxisRangeData, then setXAxisTitle('')
parseYAxis : (as parseXAxis, with a range only: setYAxisRangeData, then setYAxisTitle)
plotData   : '[' (NUMBER STR? (',' …)*) ']'
text       : STR | a run of alphaNum tokens joined with no space
```

The order matters for the axis forms: a title that follows the data is set after the axis setter, and a form without a title sets the title to `""` (D18). `eol` is NEWLINE, `;` or the end of input; a statement may be followed by another statement with no `eol`, as `line [1, 2] bar [3, 4]` is (P8).

## 5. DB

| Function                                                          | Behaviour                                                                                                         |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `setOrientation(word)`                                            | `horizontal` for the exact word `horizontal`, otherwise `vertical`; so `HORIZONTAL` is vertical (P2)              |
| `setDiagramTitle(text)`                                           | `sanitizeText(text)`; the last grammar title wins even when blank (D13, D14)                                      |
| `setAccTitle`, `setAccDescription`                                | `sanitizeText`, then the common DB's whitespace rules (leading blanks; the blanks after each line break)          |
| `setXAxisTitle`, `setYAxisTitle`, labels, categories, plot titles | `sanitizeText(trim(text))`                                                                                        |
| `setXAxisRangeData(min, max)`                                     | a linear x-axis, keeping the title                                                                                |
| `setXAxisBand(categories)`                                        | a band axis, keeping the title; the categories sanitised; duplicates kept (D6)                                    |
| `setYAxisRangeData(min, max)`                                     | the declared y-range, keeping the title; sets `hasSetYAxis`                                                       |
| `transform(data)`                                                 | D1 to D5 and D17, below                                                                                           |
| `setLineData`, `setBarData`                                       | the palette colour of the plot index (D12); a line keeps its labels when one is not empty (D10); a bar drops them |

`transform` (`xychartDb.ts` 125-164): an empty list gives nothing; without a declared x-axis, the x-range becomes `[min(prev, 1), max(prev, n)]` (D1, which keeps the first plot's range); a band axis keeps as many values as it has categories (D4); without a declared y-range, the y-range widens to the values (D8, no zero included); a band pairs each category with its value, `undefined` (`null` in the model) past the data (D5); a linear axis gives one point `[String(min), v]` for one value, or `String(min + i*step)` with `step = (max - min)/(n - 1)`.

The x strings are JavaScript's `String` of the number, written by `js_string` in `db.rs`: plain notation between `1e-7` and `1e21`, otherwise exponent notation (`1.6666666666666667`, `1e-7`, `1e+21`).

## 6. Configuration

The DB reads the effective configuration after the front matter and the directives (`config::read`, then `ferrigura_model::config::effective(None, Some("xyChart"), directive)`). Two values reach the model:

- **Orientation.** The keyword of the header, when it is given, wins; otherwise `xyChart.chartOrientation` (default `vertical`).
- **Palette.** `themeVariables.xyChart.plotColorPalette` of the resolved configuration, split at commas and trimmed. The DB takes the active theme's values over the classic default theme's (`cleanAndMerge(getThemeVariables().xyChart, config.themeVariables.xyChart)`), so the palette is the active theme's. The port ships two themes, and a configured theme that is not shipped resolves to one of them (`Theme::from_name`: a name ending in `dark` takes `redux-dark-color`, any other takes `redux-color`). The exporter applies that rule to the reference's theme too (`shippedTheme` in `export-xychart.ts`), so both sides draw the palette of the same theme. The classic default palette is not reproduced: no input resolves to it in ferrigura.

The sanitising reads `htmlLabels` and `securityLevel`. The default (`strict`, `htmlLabels` on) runs DOMPurify once more, which removes nothing that the first pass left; `loose` runs DOMPurify alone. The port's `sanitize_text` is the first pass, and no fixture or probe differs.

The schema's `$ref` on `xyChart.xAxis` and `xyChart.yAxis` is a key of the reference's defaults (`defaultConfig.ts` spreads the schema's default JSON, which keeps it). `ts_defaults.json` holds it, so the effective configuration matches the reference's `getConfig()` (`pnpm port:diff config`).

## 7. Model

`XyChartDiagram` with `version` 1 and camelCase keys (`canonical-model.md`, section XyChart):

- `orientation`, `title` (the DB's `getDiagramTitle()`, not `getXYChartData().title`, which only `getDrawableElem` sets), `accTitle`, `accDescr`;
- `xAxis`: `{type: "band", title, categories}` or `{type: "linear", title, min, max}`;
- `yAxis`: `{type: "linear", title, min, max}`, with `"Infinity"` and `"-Infinity"` for a chart with no plot;
- `plots`: `{type: "line", title, strokeFill, strokeWidth: 2, data, pointLabels?}` or `{type: "bar", title, fill, data}`, where `data` is a list of `[x, y]` pairs, `x` a string and `y` a number, or `null` for an absent value. `pointLabels` is present only when a label is not empty.

Left out: `plotIndex`, `hasSetXAxis` and `hasSetYAxis`, the measuring group, and the configuration itself (checked by `pnpm port:diff config`).

## 8. Quirks

| Id  | Status                              | Where                                                                                                                                                                                                                                                                               |
| --- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1  | reproduced                          | keyword rules 6, 14, 16, 17, 20, 21 need a word boundary; `linear` is ALPHA; `Bar` in `y-axis Bar` is the keyword (both sides reject)                                                                                                                                               |
| P2  | reproduced                          | rule 15 has no boundary; `HORIZONTAL` is vertical (the word is compared as typed)                                                                                                                                                                                                   |
| P3  | reproduced                          | white space is skipped in text, so `My Chart 2` is `MyChart2`; numbers in text are NUM                                                                                                                                                                                              |
| P4  | reproduced in effect, one deviation | no Markdown string: a backtick is a lexical error (test: `title \`x\``). The garbled rule 25 can match only the literal text of the grammar's own source lines; the port does not reproduce that literal match. No input a reader writes contains it, and the differential has none |
| P5  | reproduced                          | `""` gives no token: the closing quote pops before `[^"]*` runs; `title ""` is a parse error                                                                                                                                                                                        |
| P6  | reproduced                          | a quoted string spans lines                                                                                                                                                                                                                                                         |
| P7  | reproduced                          | number rule only in axis data and data inner; `5.` and `1e3` are errors; `012` is 12; `title 2024` is text                                                                                                                                                                          |
| P8  | reproduced                          | statements need no line break between them                                                                                                                                                                                                                                          |
| P9  | reproduced                          | `;` is an `eol`                                                                                                                                                                                                                                                                     |
| P10 | reproduced                          | keywords are case-insensitive                                                                                                                                                                                                                                                       |
| P11 | reproduced                          | `xychart diagonal` is an error on line 1; the header is optional, a text that starts with a statement is accepted (the grammar's `document` alternative)                                                                                                                            |
| P12 | reproduced                          | `accTitle:` and `accDescr:` take the rest of the line; the block keeps its lines, and its blanks follow the DB's rules                                                                                                                                                              |
| P13 | reproduced                          | `%%` comments are removed from the line start (rules 0, 5) or after any character other than `}` (rule 1)                                                                                                                                                                           |
| D1  | reproduced                          | the first plot fixes the linear x-range; later plots never widen it                                                                                                                                                                                                                 |
| D2  | reproduced                          | a plot before a band x-axis keeps its linear x strings                                                                                                                                                                                                                              |
| D3  | reproduced                          | a band before a plot maps the values to the categories in order                                                                                                                                                                                                                     |
| D4  | reproduced                          | data longer than the categories is cut, and the cut values do not widen the y-range                                                                                                                                                                                                 |
| D5  | reproduced                          | shorter data gives `null` for the missing categories                                                                                                                                                                                                                                |
| D6  | reproduced                          | duplicate categories are kept                                                                                                                                                                                                                                                       |
| D7  | reproduced                          | `x-axis 0 --> 0` gives every x the string `"0"`                                                                                                                                                                                                                                     |
| D8  | reproduced                          | the y-range without a declaration is the min and max of the data, with no zero                                                                                                                                                                                                      |
| D9  | reproduced                          | a declared y-range stays; data outside it is kept                                                                                                                                                                                                                                   |
| D10 | reproduced                          | bar labels are dropped; line labels are kept with `""` for the unlabelled points                                                                                                                                                                                                    |
| D11 | reproduced                          | `y-axis 10 --> 0` keeps min 10 and max 0                                                                                                                                                                                                                                            |
| D12 | reproduced                          | the palette index counts every plot, line or bar                                                                                                                                                                                                                                    |
| D13 | reproduced                          | a blank grammar title overrides the front matter title                                                                                                                                                                                                                              |
| D14 | reproduced                          | `title ""` is an error; `title " "` is accepted and blanks the title                                                                                                                                                                                                                |
| D15 | reproduced                          | an axis setter keeps the title it has when a title is given with the data; see D18 for the other case                                                                                                                                                                               |
| D16 | reproduced                          | a plot with no data is an error; a chart with a title and no plot is accepted (the model's y-range is `Infinity` to `-Infinity`)                                                                                                                                                    |
| D17 | reproduced                          | a one-point plot gives `"1"` as its x string (or `String(min)` on a linear axis)                                                                                                                                                                                                    |
| D18 | reproduced (new)                    | an x-axis or y-axis range, or a band, given without a title sets the title to `""`: the `parseXAxis: xAxisData` and `parseYAxis: yAxisData` reductions call `setXAxisTitle('')` and `setYAxisTitle('')`. Found by the port's tests and the fuzz run; the notes did not list it      |

The renderer quirks (R1-R22 of NOTES 6.8) are not in this port: they belong to the chart builder and the renderer, and their tests come with those.

## 9. Specs

| Reference spec                                  | Rust file                                              | Tests | Notes                                                                                                                         |
| ----------------------------------------------- | ------------------------------------------------------ | ----: | ----------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/xychart/parser/xychart.jison.spec.ts` | `crates/ferrigura-syntax/tests/xychart_parser_spec.rs` |    58 | 48 at the root, 6 under point labels, 4 under accessibility. The mock DB calls become model assertions. Passing; none ignored |
| `diagrams/xychart/xychartDb.spec.ts`            | `crates/ferrigura-syntax/tests/xychart_db_spec.rs`     |     6 | The DB has no public surface; each test drives `parse`. Passing; none ignored                                                 |

The chart builder specs (`chartBuilder/*.spec.ts`, 14 tests) and the e2e rendering spec belong to the renderer. The parser spec asserts no rendering, so no test is ignored for that reason. Where the spec's argument is not the model's value, the test asserts the model: the multi-line `accDescr` loses its indentation (`commonDb.ts` line 23), and an axis title is trimmed (`textSanitizer`). Bar labels are asserted as absent.

## 10. Differential

`tools/port/export-xychart.ts` runs the reference's parser and DB on each input, with the configuration and the theme rule of section 6, and writes the model. `cases.ts` adds the probed quirks (NOTES section 2.2 and 3.6), the configuration cases (orientation from the front matter, the directive and the keyword; the palette override; `dark` and `forest` fallbacks), and a few shapes the fixtures lack.

`pnpm port:diff xychart`: 207 inputs, 173 accepted by the reference, 0 differences, 0 named exceptions. `pnpm port:diff config`: the xyChart group compares its section, with 204 of 204 accepted and 0 differences.

Fuzzing (not committed; the generator and its inputs were in the scratchpad): 11,948 random inputs of keyword, punctuation and quote tokens, and mutated fixtures. The first run included front matter and directives: 911 of 4,457 accepted, 3 differences. The second run excluded them: 1,587 of 7,491 accepted, 3 differences. The six differences are the two cases of section 11 (items 1 and 2), and none is in the parser or the DB.

## 11. Not reproduced, and open questions

1. **A directive in a text with no detected type.** The reference's `preprocess.ts` calls `detectInit`, which calls `detectType` when an `init` directive is present, and that throws `No diagram type detected` for text that no detector takes (`utils.ts` 99-132). The shared config reader (`ferrigura_syntax::config::read`) does not raise it, so ferrigura parses such text as xychart. Three of the fuzz inputs show it (`%%{init: {}}%%  `, and two texts that start with `accDescr {`). No collected input has it. The fix belongs to the configuration reader (the config port), and the xychart parser then inherits it. Recommendation: the config reader returns the UnknownDiagram error in this case, and this port's differential picks it up.
2. **Front matter YAML that js-yaml rejects.** Two fuzz inputs (`bad indentation of a mapping entry`, and a `:` on a line of its own inside a mapping) are accepted by the shared YAML reader. This is a gap in `yaml.rs` that affects every type with front matter, not xychart.
3. **The garbled rule 25 (P4).** Reproduced as a deviation in section 8. Recommendation: accept it; it is not reachable from any text a reader writes.
4. **The theme of the reference side (section 6).** The exporter sets the reference's theme to the shipped theme that ferrigura resolves it to. This is the rule the geometry harness uses (`forcedDirective`), applied to the parser's model. Recommendation: accept it, and keep it named here.
5. **The `$ref` defaults (section 6).** Kept because the reference's `getConfig()` carries them; they are unused by any reader. Recommendation: accept.

## 12. Renderer

The Rust code: `crates/ferrigura-render/src/xychart/` (`mod.rs` the SVG and the data labels, `config.rs` the reading of the configuration, the theme and the model, `builder/` the chart builder: `axis.rs`, `title.rs`, `legend.rs`, `plot.rs`, `mod.rs` the orchestrator, `text.rs` the measurement). The d3 subset is in `crates/ferrigura-render/src/common/d3/` (number text, band and linear scales, ticks, the linear line), shared with the other renderers.

**Status.** `pnpm port:geometry xychart`: 47 of 47 within tolerance. The render spec is `crates/ferrigura-render/tests/xychart_render_spec.rs`.

**Measurement on the screen.** The reference divides each text box by the scale of the SVG on the screen (`textDimensionCalculator.ts` 31-42). In the harness the SVG is laid out in a 1200 px container, so the scale is `min(1, 1200 / width)`, and each text is laid out at `fontSize * scale` and its box divided by the scale again (`builder/text.rs`, `config.rs` `screen_scale`). The title of the fixture `should-keep-legend-inside-a-wide-chart-scaled-to-fit` (width 4000) is the case that needs it: its height is 8 screen pixels over 0.3, not 25.

**Quirks.**

| Id  | Status                                                                                                            |
| --- | ----------------------------------------------------------------------------------------------------------------- |
| R1  | reproduced: labels from `plots[0]`; a missing value or a short first plot fails the render, as the reference does |
| R2  | reproduced: the font size is `${n}px`, the smallest candidate floored, with no upper bound                        |
| R3  | reproduced: `#131300` unless the configuration sets `dataLabelColor`                                              |
| R4  | reproduced: candidates start at the bar size and drop by 1; `Math.floor` of the minimum, negative allowed         |
| R5  | reproduced: data labels only for rectangles; a legend marker is a rectangle, so it takes labels too (probed)      |
| R6  | reproduced: no `y`, `height="NaN"` (probed)                                                                       |
| R7  | reproduced: negative width below the axis minimum (probed)                                                        |
| R8  | reproduced: duplicate categories share their first place (probed)                                                 |
| R9  | reproduced: a plot before a band axis at the range start (probed)                                                 |
| R10 | reproduced: a zero-width domain maps to the middle                                                                |
| R11 | reproduced: a single-point line is closed with `Z`                                                                |
| R12 | reproduced: lines to three digits; the axes and the bars are not rounded                                          |
| R13 | reproduced: bars from the plot's bottom, with the outer padding at both ends                                      |
| R14 | reproduced: `axisl-line` on the left axis                                                                         |
| R15 | reproduced: the outer padding is set before the fit check (not probed; read)                                      |
| R16 | reproduced: bar width from the tick distance (probed)                                                             |
| R17 | reproduced: the text is escaped, then the placeholders are decoded on the markup (probed)                         |
| R18 | reproduced: the empty group before `main` and the empty measurement group last                                    |
| R19 | reproduced: `width="100%"` always                                                                                 |
| R20 | not reproduced: see deviation 2 below                                                                             |
| R21 | reproduced: the colour follows the plot's position, bars included                                                 |
| R22 | reproduced: an inverted y range puts 0 at the top                                                                 |

Probes of the quirks the fixtures do not cover were run through the harness's own reference render and measurement, in a scratch file outside the repository (not committed): R1, R6, R7, R8, R9, R16, R17 and R22 matched the reference, and so did the legend markers with labels and the negative values.

**Deviations.**

1. A text with a character that the bundled faces do not cover (`♥`, a symbol) is measured with the glyph that the face has (`.notdef`), while the reference measures it in a system fallback font. The harness excludes such texts for this reason; no fixture has one.
2. R20 (a `titlePadding` given as a string concatenates in the reference): the typed configuration reads `"5"` as 5, as the other renderers do.
3. The chart builder's unit specs (`chartBuilder/*.spec.ts`, 14 tests) use the reference's measurement without a DOM group, a fallback ferrigura never takes; they are not ported. The layout is checked by the 47 fixtures.
