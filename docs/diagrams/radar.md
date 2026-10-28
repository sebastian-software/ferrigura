# Radar Diagram

The radar diagram draws curves over a set of axes that radiate from a center, with a graticule of rings or polygons and an optional legend. It starts with `radar-beta`. ferrigura parses it with a hand-written lexer and recursive-descent grammar in `ferrigura-syntax/src/radar/` and draws it with the type's own renderer in `ferrigura-render/src/radar/`. Nothing is measured and no layout engine runs: the rings, axes and curves are placed from the radius and the number of axes.

## Model

`ferrigura_model::radar::RadarDiagram` is the DB state after the parse ([canonical-model.md](../canonical-model.md#radar) has the conventions):

- `title`, `accTitle`, `accDescr`: the DB's values after its setters, sanitized, `""` when absent.
- `axes`: `{name, label}` in text order. The label is the name when the text gives none; a label of `""` (`axis B[""]`) is kept as empty. A duplicate name is kept.
- `curves`: `{name, label, entries}` in text order. The entries are in axis order for the named form (`Agility 2, Speed 3`) and in text order for the number form (`{1, 2, 3}`); their count is not checked against the axes.
- `options`: `showLegend` (true), `ticks` (5), `max`, `min` (0) and `graticule` (`circle` or `polygon`). `max` is the one nullable field: `null` when the text gives none, and `0` when it gives zero. `ticks` above 32 is 32; a fraction and zero are kept. The last option of a name wins.
- A number that is not finite is the sentinel string `"Infinity"` or `"NaN"`.

The labels of axes and curves are not sanitized (`<b>x</b>` stays as text). An entity such as `#35;` is the placeholder `ﬂ°°35¶ß` in a label and in the title. Left out: the configuration, the theme and everything the renderer derives.

## Parsing

`radar/mod.rs` runs the preprocessing steps of `Diagram.fromText`, sets the front matter title, and parses with `grammar.rs` over the tokens of `scanner.rs`. The DB calls (`db.rs`) follow Mermaid's `populate`: the header texts (the title only when it is non-empty, so a blank last title lets the front matter's through), `setAxes`, `setCurves`, `setOptions`. The first failing call is the error.

The grammar is `Radar: NEWLINE* "radar-beta" ":"? NEWLINE* (TitleAndAccessibilities | "axis" Axis ("," Axis)* | "curve" Curve ("," Curve)* | Option ("," Option)* | NEWLINE)*`, with `Curve: ID Label? "{" Entries "}"` and entries that are either all numbers or all `ID ":"? NUMBER`. The scanner ports Langium's lexer as Chevrotain runs it: the first terminal that matches wins, not the longest. The order is the keywords (`radar-beta:` first, then `radar-beta` with a word-end suffix, `showLegend`, `graticule`, `curve`, `ticks`, `axis`, `max`, `min`), the punctuation, the terminals (`circle`/`polygon`, `true`/`false`, the accessibility and title terminals, `NUMBER`, `STRING`, `ID`, `NEWLINE`), and then the hidden ones (blanks, a YAML-like block, directives, comments). Each word keyword has the ID as its longer alternative, so `maximum` and `axisX` are names.

Quirks that ferrigura keeps:

- The header must end at a blank, `%%` or the end of the text: `radar-beta{` and `radar-betaX` are rejected, though the detector takes them. `radar-beta` may be a curve name (`curve radar-beta{1}`) but not an axis name.
- Terminals without a longer alternative take the prefix of any word they match: `circlex` is `circle` and then the name `x`, `trueX` is `true` and `X`, and `title1` is the title line and then the number `1`. So `axis title1`, `axis circle` and `axis 1A` are rejected. A keyword alone is not a name (`axis max`), but a longer word that begins with one is (`axis showLegend2`).
- The blanks before `title` and the accessibility keywords belong to their token. `axis A title` is the axis A and an empty title; `title<TAB>axis A` is the title `axis A` with no axis.
- Statements need no separator. A line break is allowed only where the grammar has a `NEWLINE` slot: around the entry commas inside `{ … }` and at the end of a statement. `axis A,` and `ticks 5,` followed by a line break are rejected.
- A number has no sign and no exponent, and `01` is two numbers. A word that begins with a number is the number first. An ID cannot end in `-`; a dot, a minus sign or a non-ASCII letter is a lexer error. `{}` is rejected. A number above the largest double (310 digits) is `Infinity`; a curve of 125,000 entries parses.
- An unclosed YAML-like block in the body is a lexer error.
- The DB: `setCurves` needs the axes first when a curve names its entries (`Axes must be populated before curves for reference entries`), each axis needs a first entry that names it (`Missing entry for axis <label>`), a named entry for an undeclared axis is dropped, and the first entry of an axis wins. The axes may come after the curve. A number list is kept whatever its length.
- The title and accessibility texts are sanitized: the `&` of `title A & <b>x</b>` is held as `&amp;`, and `onerror` is removed.

Error messages are ferrigura's own ([0007](../decisions/0007-hand-written-parsers.md)); the differential run compares accept and reject and the model.

## Layout and rendering

`mod.rs` ports `renderer.ts` (`draw`), `geometry.rs` its polar arithmetic, and `style.rs` `styles.ts`. The arithmetic keeps the reference's order, left to right: `radius * axisScaleFactor * cosA` is `(radius * axisScaleFactor) * cosA`, and `(2 * j * PI) / n` is `((2 * j) * PI) / n`. Sines and cosines are `ferrigura_jsmath`'s ([0012](../decisions/0012-layout-engines-ported-exactly.md)), because V8's values differ from the platform's in the last bit. The axis angle `(2 * i * PI) / n - PI / 2` and the curve point's angle `(2 * PI * i) / n - PI / 2` are written in different orders in the reference, and both are kept.

The root is `width + marginLeft + marginRight` by `height + marginTop + marginBottom`, with `overflow="visible"` and the diagram translated to the center. The radius is `min(width, height) / 2`. A translated `g` holds, in this order: the graticule (all circles with `r = radius * (i + 1) / ticks`, or all polygons with vertices at the axis angles), then for each axis its `line` (to `radius * axisScaleFactor` along the angle) and its `text` (anchored `start`, `end` or `middle` by `cosA` beyond ±0.01, with baseline `hanging`, `auto` or `central` by `sinA`, pushed out by `radius * axisLabelFactor` plus 4), then the curves, then one legend `g` per curve when `showLegend` is set, then the title `text`, which is always written, even when empty. A curve point is at `relativeRadius(entry, min, max, radius)`, the entry clipped to `[min, max]` and scaled; the curve is `closedRoundCurve(points, curveTension)`, a closed Catmull-Rom path as cubic Béziers (`M p0`, then ` C cp1 cp2 p2` for each point with the cyclic neighbors, then ` Z`), or a `polygon` for the polygon graticule. The legend sits at `((width / 2 + marginRight) * 3 / 4, -(height / 2 + marginTop) * 3 / 4)` with a line height of 20, a 12 by 12 box and the text 16 px to its right. The text is drawn with the bundled sans face embedded as a subset ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)).

The style options are `themeVariables.radar` merged over the default theme's radar values, because the redux themes have no `radar` section: axis `#333333`, stroke width 2, label size 12, curve opacity 0.5 and stroke width 2, graticule `#DEDEDE` with opacity 0.3 and stroke width 1, legend size 12. The curve colors are the theme's `cScale0` to `cScale11`.

## Differences from Mermaid

- **Colors** are custom properties on the diagram's id, one set per color scheme, with the light values also as presentation attributes ([0011](../decisions/0011-themes-as-css-variables.md)). A color the theme does not give is not written, which draws the same as the reference's invalid declaration.
- **`maxTextSize`.** The reference's `Math.max(...entries)` throws a `RangeError` at about 125,000 entries; the pipeline's text cap makes that unreachable at the default configuration. ferrigura has no such limit.

## Known gaps

None known for this type.

## Checking it

- `pnpm -C tools/port port:diff radar` compares the canonical models on every fixture, spec string and case of `cases.ts`, which holds every text of the Rust tests and the probed edge cases (a maximum below the minimum, numbers of 309, 310 and 400 digits, a curve of 125,000 entries); `export-radar.ts` runs the reference's parser and DB. `port:diff config` compares the `radar` section.
- `pnpm -C tools/port port:geometry radar` compares the geometry of the fixtures and of the probes in `tools/port/geometry/probes/radar/`: clamped, fractional and zero ticks, a hidden legend, a header only, all-zero values, a maximum equal to or below the minimum, clipped values, a skipped curve, thirteen curves, markup and entities, one and two axes, zero width, absolute size, curve tension, theme overrides, an infinite entry.
- The Rust specs: `ferrigura-syntax/tests/radar_spec.rs` ports the parser and DB tests of the reference, `radar_grammar.rs` and `radar_db_spec.rs` pin the grammar's and the DB's probes; `ferrigura-render/tests/radar_render_spec.rs` ports the renderer's helper and `draw` tests, and `radar_quirks.rs` has one test per quirk of the renderer that changes the output.

## Learnings

- **A maximum equal to the minimum gives `NaN`.** All-zero values with no `max` write `MNaN,NaN CNaN,NaN …`, and `min 2 max 2` does the same; a maximum below the minimum divides by the negative range. The renderer writes the same strings.
- **A curve with the wrong number of entries is skipped, but its legend item is drawn**; the other curves keep their own index in the list.
- **`ticks`:** 40 draws 32 circles, 2.5 draws three circles (at 120, 240 and 360 of a 300 frame, beyond the frame), and 0 draws none. A tension above 1 is not clamped and the curve overshoots; a tension of 0 gives straight segments.
- **Values clip to `[min, max]`**: below the minimum they draw the center, above the maximum the frame.
- **The title is double-encoded when it has markup.** DOMPurify returns text without `<` unchanged, so `title Hello & more` shows as typed. With markup the sanitized text holds `&amp;`, which `.text()` does not decode, so the title shows `&amp;` where the author wrote `&`, and the serialized SVG holds `&amp;amp;`. An entity placeholder in a label is decoded at serialization (`#35;` shows `#`).
- **`legendBoxSize` is never read**: the legend box is 12 by 12 whatever the theme says.
- **A numeric `fontSize` theme variable is written unitless** (`font-size:80;`), which the browser drops, so the title keeps the inherited size. `color` does not paint SVG text: the labels and the title are filled by the root's `fill`.
- **A thirteenth curve has no style rule**: `genIndexStyles` writes the curve and legend rules for 12 colors only, so it is filled with the root's fill.
- **A width of 0 gives a radius of 0**, and every circle, curve and label sits at the center plus the label offset. One axis draws a degenerate closed curve through the one point, two axes a closed curve through two.
- **Numbers are written as JavaScript writes them**: `-0` is `0`, and values below 1e-6 keep their exponent (`6.123233995736766e-15`). The shared writer `svg::number` does it.
- **`Math.max` and `Math.min` propagate `NaN`**, where Rust's `f64::max` and `f64::min` ignore it (`js_max` and `js_min` in `geometry.rs`).
- **The axis label's anchor and baseline are chosen with a 0.01 dead zone**, which makes `cos(PI / 2)` (6e-17) a vertical axis.
