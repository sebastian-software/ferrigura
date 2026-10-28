# Quadrant Chart

The quadrant chart places labeled points on a square divided into four quadrants, with two axes and a text for each quadrant. It starts with `quadrantChart`. ferrigura parses it with a hand-written scanner and recursive-descent grammar in `ferrigura-syntax/src/quadrant_chart/`, and draws it with the type's own renderer in `ferrigura-render/src/quadrant_chart/`, a port of Mermaid's quadrant builder. Nothing is measured: every box is arithmetic on the configuration, so no layout engine is involved.

## Model

`ferrigura_model::quadrant_chart::QuadrantChartDiagram` is the DB state after the parse, before the builder lays anything out ([canonical-model.md](../canonical-model.md#quadrantchart) has the conventions):

- `title`, `accTitle`, `accDescr`: the common DB's values.
- `quadrant1Text` to `quadrant4Text`, `xAxisLeftText`, `xAxisRightText`, `yAxisBottomText`, `yAxisTopText`: the texts the DB holds, `""` when unset.
- `points`: each with `text`, `className` (`""` when none), `x`, `y` and the styles that were set: `radius` (a number), `color`, `strokeColor` and `strokeWidth` (as written). A style that was not set has no key.
- `classes`: a list of `[name, styles]` pairs in the order of the first definition.

The DB's quirks that the model keeps:

- **Order.** The builder prepends points, so `points` is in the reverse of the source order (`A`, `B`, `C` come out `C`, `B`, `A`). A second `classDef` of a name replaces the styles and keeps the place.
- **Coordinates** are `Number(text)` of the token the grammar read, the value the scale reads: `0x5` is 5, `0b1` is 1, and `0,5`, `0 5` and `0-5` are `"NaN"`.
- **Entities** keep the placeholders the DB holds (`#35;` is `ﬂ°°35¶ß`); the renderer decodes them.
- **Class lookup is at build time.** A class defined after its point still applies, and a class name with no `classDef` is accepted; the model keeps the name only. A point's own style wins over its class when the builder merges them.

Left out: the layout (points, rectangles, labels), the configuration and the theme variables (`port:diff config` and `port:diff theme` check them).

## Parsing

`quadrant_chart/mod.rs` runs the preprocessing steps of `Diagram.fromText`, sets the front matter title, and runs the grammar over the tokens of `scanner.rs`. The grammar is LALR(1) in Jison without conflicts, so one token of lookahead accepts the same texts. The DB calls come in the order of the reductions, and a call the DB rejects (a bad style) aborts the parse.

`scanner.rs` ports `quadrant.jison` rule by rule in the generated order. The first rule that matches wins, not the longest. Every state but the initial one is exclusive, so a rule without a start condition applies in the initial state only. The states are the quoted and Markdown strings, the class name (`:::name`), the point (`: [x, y]` with an x and a y state), and the accessibility states. After the input ends, one more read happens at the end of the input, then a raw end symbol that no production accepts.

Quirks of the lexer and grammar that ferrigura keeps:

- `title` and `classDef` have a word boundary after them; `quadrant-N`, `x-axis`, `accTitle` and `quadrantChart` have none: `quadrant-1A` is `quadrant-1` then the text `A`, and `quadrantChartX` is an error. A `quadrant-N` or `x-axis` inside a text starts its token, so `Point quadrant-1` is an error.
- Keywords are case-insensitive over ASCII only, as the `i` flag without `u` is. The grammar takes `QUADRANTCHART`, but the detector does not, so such an input is not a quadrant chart.
- Rule 1 takes one character before `%%`, a line break included: `P1%% c` loses the `1`, and a line break before a comment goes with the comment.
- A white space beyond ASCII (no-break space, U+2028, U+3000, the byte order mark) is text in the initial state and white space only around a point's colon, brackets and comma.
- A coordinate is read with a `.` that takes any character but a line terminator, one UTF-16 unit at a time: `0.5.5`, `1.5` and `1.0` are lexical errors, `0😀5` takes `0` alone, and a line break ends a coordinate.
- `title: Hello` keeps the colon, since the value is the rest of the line. The last title wins, and a blank last title replaces the front matter's.
- An axis takes its first text as the left or bottom text, and `-->` (any run of dashes and `>`) is the delimiter. A delimiter with nothing after it appends `" ⟶ "`, which the DB trims, so `x-axis Low -->` gives `Low ⟶`. `x-axis --> High` is an error.
- A quoted or Markdown string may only start a text (`"a"b` is `ab`, `a"b"` is an error). A `;` ends the line, so `&amp;` in a label is three tokens.
- The `section` statement of the grammar names a symbol that is no token, so it never matches.

Styles (`radius`, `color`, `stroke-color`, `stroke-width`) are checked by the DB's `parseStyles`: each text is trimmed and split at `\s*:\s*`, further pieces are ignored, and a failing check rejects the text. `radius` takes `^\d+$` (`007` is 7, `0` is kept; `10px`, a fraction or a negative number are errors); `color` and `stroke-color` take `#?` and three or six hex digits, written as given (`red` is an error); `stroke-width` takes `^\d+px$`; any other key is `style named K is not supported.`. Styles separated by a space and no comma, and a trailing comma, are errors.

Texts go through the DB's `sanitizeText`; `sanitize.rs` ports the DOMPurify pass the default configuration reaches (`strict`, `htmlLabels` on).

## Layout and rendering

`builder.rs` is `quadrantBuilder.ts`: `calculateSpace`, the axis labels, the quadrants, the points, the borders, the title, and `build`. The arithmetic keeps the reference's operand order because the last bit of a coordinate is part of the output. A point's position is the d3 linear scale over `[0, 1]` (`common::d3::LinearScale`), which computes `a * (1 - t) + b * t`, not `a + t * (b - a)`; the two forms differ in the last bits for hundreds of values.

`mod.rs` is `quadrantRenderer.ts`. It writes the root, then five groups in `g.main` in this order: `quadrants`, `border`, `data-points`, `labels` and `title`, each created even when empty. The four quadrant rectangles come in the order 1 (top right), 2 (top left), 3 (bottom left), 4 (bottom right); there are six border lines (four external, then two internal); each axis text is a `g.label`. The text is drawn with the bundled sans face embedded as a subset ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)).

## Differences from Mermaid

- **Sanitizing** reads no configuration: a directive that changes the sanitizing of a quoted text with markup (`securityLevel`, `htmlLabels`) is not applied. Probes with such directives agree with the reference; the reference's `getConfig` spy test is `#[ignore]`d for this reason.
- **Themes.** Only the shipped themes exist ([0011](../decisions/0011-themes-as-css-variables.md)).

## Known gaps

- Front matter that js-yaml rejects but `yaml.rs` accepts (a control character, a malformed mapping) is accepted. The gap is in the shared reader and concerns every type with front matter.
- The browser's style form of the color keywords and of `rgba()` in a border is not reproduced; no fixture uses them.

## Checking it

- `pnpm -C tools/port port:diff quadrantChart` compares the canonical models on every fixture, spec string, documentation example and case of `cases.ts`. `export-quadrantChart.ts` runs the reference's parser and DB and reads the builder's state through a device of the loader, without changing the reference's source. `port:diff config` compares the `quadrantChart` section.
- `pnpm -C tools/port port:geometry quadrantChart` compares the geometry of the fixtures and of the probes in `tools/port/geometry/probes/quadrantChart/`: points on the edges, coordinates outside the chart or unreadable, long texts, classes and point styles, many points, delimiters, entities, theme colors in CSS forms, the dark theme, sizes, accessible texts, empty texts, a chart smaller than its axes, and borders in `hsl()`.
- The Rust specs: `ferrigura-syntax/tests/quadrant_chart_parser_spec.rs` and `quadrant_chart_db_spec.rs` port the reference's two specs, and `ferrigura-render/tests/quadrant_chart_render.rs` pins the quirks of the output (below).

## Learnings

- **A coordinate the scale cannot read** (`NaN`) drops `cx` or `cy` from the circle, and the label's transform reads `translate(undefined, …)` for an x and `translate(…, NaN)` for a y.
- **The redux themes give the point fill `hsl(240, 90%, NaN%)`.** khroma returns it for `darken('#ECECFE')` and `lighten('#ECECFE')`, and the reference writes it as `fill` and as the default `stroke` of every point with no color of its own. ferrigura copies the value; the theme port produces it. The theme's condition `this.quadrantPointFill || isDark(q1) ? lighten : darken` binds `||` before `?:`, so a user value gives `lighten(q1)`, which the user's value replaces.
- **The border style is serialized by the browser.** The reference sets the stroke with d3's `style`, so Chromium writes `stroke: rgb(24, 24, 24); stroke-width: 2;`: a hex color becomes `rgb()`, and an `hsl()` color is converted in single precision (the redux border `hsl(247.5, 0%, 9.2156862745%)` reads as 24, where double precision gives 23). An `hsl()` with a `NaN` component is dropped. Other forms are written as given.
- **The x-axis is at the bottom whenever there are points**, whatever `xAxisPosition` says.
- **The empty groups and texts are always written**: the five groups, the four quadrant texts even when empty, and all six border lines.
- **A quadrant text is centered in its box when there are no points** (`dominant-baseline="middle"`), and hangs from the top padding otherwise.
- **A text is `decode_entities(escape(text))`**, the whole-SVG pass of `mermaidAPI.ts`. The font subset takes the characters the reader sees, so the apostrophe and the quote are in it, not their `&#39;` and `&quot;` markup.
- **The root writes `width` first, then the namespace, then `height` or `style`**, as the reference does; `title` and `desc` are the first children when the texts are set.
- **The builder's own defaults swap `chartWidth` and `chartHeight`**, and its `||` fallbacks turn a 0 into the default; both only apply before `getQuadrantData` replaces them, so they have no effect.
