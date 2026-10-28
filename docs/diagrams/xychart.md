# XY Chart

The xychart draws lines and bars over a band or a linear x-axis and a linear y-axis. It starts with `xychart` or `xychart-beta`, optionally followed by `horizontal` or `vertical`. ferrigura parses it with a hand-written scanner and recursive-descent grammar in `ferrigura-syntax/src/xychart/`, and draws it with the type's own renderer in `ferrigura-render/src/xychart/`: a port of Mermaid's chart builder, which lays the chart out with its own arithmetic. The unified renderer and the layout engines are not involved.

## Model

`ferrigura_model::xychart::XyChartDiagram` (`ferrigura-model/src/xychart.rs`) is the DB's state after the parse, before the chart builder runs ([canonical-model.md](../canonical-model.md#xychart) has the conventions that hold for every type):

- `orientation`: `horizontal` or `vertical`. The header keyword wins when it is given, else `xyChart.chartOrientation` of the resolved configuration.
- `title`, `accTitle`, `accDescr`: sanitized, `""` when absent. The title is the DB's `getDiagramTitle()`.
- `xAxis`: `{type: "band", title, categories}` or `{type: "linear", title, min, max}`.
- `yAxis`: `{type: "linear", title, min, max}`. Without a declared range it covers the data; with no plot it is `Infinity` to `-Infinity`, written as the sentinel strings.
- `plots`: in declaration order, either `{type: "line", title, strokeFill, strokeWidth: 2, data, pointLabels?}` or `{type: "bar", title, fill, data}`. `data` is a list of `[x, y]` pairs: `x` is a string (the category, or JavaScript's `String` of the linear position, such as `1.6666666666666667` or `1e-7`), `y` a number or `null` for a category the data does not reach. `pointLabels` is present only when some label is not empty, and holds `""` for the unlabeled points.
- The colors (`strokeFill`, `fill`) are entries of the `plotColorPalette` of the theme the configuration resolves to, one per plot, counting lines and bars alike.

Left out: the plot counter, the two flags that record a declared axis, the measuring group and the configuration itself (`port:diff config` checks it).

## Parsing

`xychart/mod.rs` runs the steps `Diagram.fromText` runs before the Jison lexer (front matter, directives, comment lines, entity encoding, a final line break), reads the configuration the text carries, builds the DB with the resolved orientation and palette, sets the front matter title, and then runs the grammar. The grammar makes the DB calls in the order the Jison reductions make them.

`scanner.rs` ports `xychart.jison` rule by rule, in the order Jison generates them, with its start conditions (axis data, band data, data, data inner, the accessibility states, the quoted string). The first rule that matches wins, not the longest, and keywords are case-insensitive. The scanner reproduces Jison's end-of-input handling: after the input is empty it reads the rules once more at the end, then returns a raw end symbol that no production accepts. A text that ends inside an accessibility value or a quoted string is therefore rejected or gets an empty value, as in the reference.

Quirks of the lexer and grammar that ferrigura keeps:

- A keyword (`title`, `x-axis`, `y-axis`, `line`, `bar`, `xychart`) needs a word boundary after it, so `line titleX [1]` has the plot title `titleX`. The words `vertical` and `horizontal` have none, so `title verticalX` is an error. The orientation word is compared as typed: `HORIZONTAL` is vertical.
- White space inside unquoted text is skipped and the words are joined with no space: `title My Chart 2` is `MyChart2`. Digits in text are NUM tokens and join the text; the number rule applies only inside axis data and plot data, where `+1.3`, `.6`, `-.34` and `012` (12) are numbers and `5.` and `1e3` are errors.
- A quoted string may span lines. An empty string gives no token (the closing quote pops the state before the string rule runs), so `title ""` is an error while `title " "` is accepted. A backtick outside quotes is a lexical error; Mermaid's Markdown-string rule is garbled by Jison and never matches a text a reader writes.
- Statements need no line break between them (`line [1, 2] bar [3, 4]`), and `;` is a line break. The header may be left out. `accTitle:` and `accDescr:` take the rest of the line; `accDescr { … }` keeps its lines and collapses blank runs.
- An axis or band given without a title sets the axis title to `""`; a title given with the data is set after the axis setter.
- A plot with no data (`line []`) is a parse error; a diagram with a title and no plot parses and fails when drawn.

The DB keeps these behaviors of `xychartDb.ts`:

- The first plot fixes a linear x-range (`min(prev, 1)` to `max(prev, n)`) and later plots never widen it. A plot declared before a band axis keeps its linear x strings; a band declared first maps the values to the categories in order, cuts longer data (the cut values do not widen the y-range) and gives `null` for missing values.
- Without a declared y-range the range is the minimum and maximum of the data, with no zero included. A declared range stays, data outside it is kept, and `y-axis 10 --> 0` keeps min 10 and max 0.
- Linear x positions are `String(min + i * step)` with `step = (max - min) / (n - 1)`; a one-point plot is `"1"` (or `String(min)`), and `x-axis 0 --> 0` gives `"0"` everywhere. Duplicate categories are kept.
- Bar labels are dropped, line labels are kept.
- The grammar's `title` replaces the front matter's even when it is blank.
- The text setters sanitize with the DB's `sanitizeText`; `sanitize.rs` ports the first DOMPurify pass, which is all the default configuration leaves of the second.

## Layout and rendering

`render` takes a `Context<XyChartDiagram, XYChartConfig>`, reads the `xyChart` section and the theme's `xyChart` tokens (`config.rs`), and builds the shapes with the chart builder (`builder/`), which ports `chartBuilder/`:

- `mod.rs` is the orchestrator: the components are created in the order title, plot, legend, x-axis, y-axis, space is reserved in the order the reference does it (vertical or horizontal), and the drawable elements come out title, plots, legend, x-axis, y-axis. The plot first reserves `plotReservedSpacePercent` of the width and the height, floored, and takes what the other components leave.
- `axis.rs` has the band and linear axes: `scaleBand` and `scaleLinear` with their ticks and outer padding, label rotation, and the drawing of the left, bottom and top axes. `title.rs`, `legend.rs` and `plot.rs` are the title, the legend and the bar and line plots; `text.rs` measures; `interfaces.rs` holds the shared types.
- `mod.rs` of the renderer writes the SVG: the root and the style sheet, the shapes as `rect`, `path` and `text`, and the data labels of bars. The d3 subset (number text, band and linear scales, ticks, the linear line, the ordinal scale) is in `ferrigura-render/src/common/d3/`, shared with the other renderers.

The texts of the chart are written as SVG `<text>` with the bundled sans face embedded as a subset ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)).

## Differences from Mermaid

- **Measurement.** The reference measures each text in the browser and divides the box by the scale of the SVG on the screen. ferrigura has no screen, and takes the harness's container: a chart wider than 1200 px is scaled to 1200 px, and each text is laid out at `fontSize * scale` and its box divided by the scale again (`screen_scale` in `config.rs`).
- **Themes.** Only the shipped themes exist ([0011](../decisions/0011-themes-as-css-variables.md)): a configured theme that is not shipped takes the palette of `redux-color`, or of `redux-dark-color` for a name that ends in `dark`. The exporter applies the same rule to the reference, so both sides draw the same palette.
- **Numeric strings.** A `titlePadding` written as the string `"5"` is read as 5. The reference concatenates it in the translate of an axis title (`translate(05, …)`).
- **Text without glyphs.** A character the bundled faces lack is measured with the face's `.notdef`, where the reference measures a system fallback font. The geometry run leaves such fixtures out.
- **Detector.** `xychartfoo` is not an xychart. Upstream's detector takes any word that starts with the keyword, which is a bug of Mermaid; the reference carries a patch that ends the detector at a word boundary, as ferrigura's does ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).

## Known gaps

- The chart builder's unit specs in Mermaid (`chartBuilder/*.spec.ts`) are not ported: they measure without a DOM group, a fallback ferrigura never takes. The fixtures check the layout.
- A directive in a text no detector takes, and front matter that js-yaml rejects but `yaml.rs` accepts, are the shared readers' differences, not this type's.

## Checking it

- `pnpm -C tools/port port:diff xychart` compares the canonical models on every fixture, spec string, documentation example and case of `cases.ts`; `export-xychart.ts` runs the reference's parser and DB. `port:diff config` compares the `xyChart` section.
- `pnpm -C tools/port port:geometry xychart` compares the rendered geometry of every fixture within 0.05 px.
- The Rust specs: `ferrigura-syntax/tests/xychart_parser_spec.rs` and `xychart_db_spec.rs` port the reference's two specs; `ferrigura-render/tests/xychart_render_spec.rs` pins the renderer's quirks (below); `builder/text.rs` tests the measurement against probed boxes.

## Learnings

- **The box of a text is the union of its glyph outlines, not its advance.** `getBoundingClientRect` of the tspan gives a width that exceeds the advance for glyphs that overhang it (`j`, `q`, `jan`, `jun`), and the layout reads it: outer padding, title center and legend width move by up to a pixel. The height is a whole number of pixels, except at 16 px where Chromium gives a float32 (`20.000001907348633`), 2e-6 off ferrigura's 20. `ferrigura_text::measure_box` implements the glyph box.
- **Chromium lays SVG text out at a size floored to 1/100 px**, so the measurement does the same (`laid_out_size`).
- **The data labels read `plots[0]`** for every bar. A bar after a line shows the line's numbers, and a first plot shorter than a bar makes the render fail (`Cannot read properties of undefined`), as does a chart with no plot.
- **The data label color is `#131300` in every theme.** The shipped themes do not set `dataLabelColor`, so the DB merges it from the classic default theme; the label size starts at the bar's size and drops by 1 until the label fits, with no upper bound and a negative result possible.
- **A bar with no value** has no `y` and `height="NaN"`; a horizontal bar below the axis minimum has a negative width; duplicate categories and plots before a band axis sit at the first position.
- **Bars start at the bottom of the plot, not at zero.** The axes are inset by an outer padding at both ends, so the smallest value's bar is as high as the padding. On a linear x-axis the bar width follows the tick distance, not the point distance. A band axis with one category puts it in the middle, and a zero-width domain maps everything to the middle.
- **The outer padding is set before the fit check** of the labels: a row of labels that does not fit is hidden, but its padding still moves the plot.
- **Line paths are rounded to three decimals** (d3-path `digits` 3), axes and bars are not; a single-point line is closed with `Z`; a `NaN` coordinate is written as `NaN`.
- **The left axis line has the class `axisl-line`**, a typo of the reference that the geometry key sees.
- **The root is always `width="100%"`**: the reference passes `useMaxWidth` as a literal `true`.
- **The DOM keeps an empty `<g>` before `main` and an empty `g.mermaid-tmp-group` last**, because the geometry key counts siblings.
- **Text is written escaped, and the entity placeholders are decoded on the markup**: a title `A &amp; B` appears as `A &amp;amp; B`, and `#quot;hi#quot;` reads as `"hi"`.
- **An inverted y-range** (`y-axis 10 --> 0`) puts 0 at the top, and the labels are still written in ascending order.
