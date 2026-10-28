# Learnings

What porting Mermaid taught that concerns more than one diagram type, and that the next contributor would otherwise have to find again. The entries are findings that the code depends on; each says what was found, why it matters, and where in the code it lives. The decisions that these findings led to are in [decisions/](./decisions/README.md), and what is specific to one diagram type is on its page under [diagrams/](./diagrams/README.md). A path under "Where" that names no crate is in `crates/ferrigura-render/src`, or in `tools/port` when it starts with `geometry/`, `text/` or `fonts/`; a test file is in the `tests` folder of the crate named before it.

- [Parsing](#parsing)
- [Text measurement and fonts](#text-measurement-and-fonts)
- [JavaScript and browser arithmetic](#javascript-and-browser-arithmetic)
- [SVG and Chromium geometry](#svg-and-chromium-geometry)
- [The layout engines](#the-layout-engines)
- [The reference and the harness](#the-reference-and-the-harness)
- [The porting method](#the-porting-method)
- [Open questions](#open-questions)

## Parsing

### Lexers take the first rule that matches, not the longest

Jison, Chevrotain and Langium lexers try their rules in the written order and take the first that matches. Langium puts the keywords first, then the terminals in declaration order, with `LONGER_ALT` only on keywords; Jison gives a keyword literal that ends in a word character an implicit `\b`, and a zero-length match is a token. Most grammar quirks of the types follow from this: keywords that also match as prefixes of names, titles that are read only after another token on the same line, and Jison rules with a leading `.*`, such as `.*direction\s+WORD[^\n]*`, which take the rest of the line (a relation or a label that contains `direction LR` is swallowed). At the end of the input, a Jison lexer in a value state matches the empty string once and then answers its raw end symbol, not `EOF`, which the grammar rejects: a trailing `accTitle:` is a parse error. The scanners keep each rule table in its written order for this reason ([0007](./decisions/0007-hand-written-parsers.md)). **Where:** `crates/ferrigura-syntax/src/scan.rs` and each type's `scanner.rs`.

### Comments and directives end where JavaScript's line terminators end

`cleanupComments` removes `%%` lines with a regular expression in multiline mode, and JavaScript's `^` also matches after U+2028 and U+2029, so a comment can start after a line separator. The directive pattern stops at U+2028, U+2029 and a bare CR. The preprocessor looks after every line terminator JavaScript knows. **Where:** `crates/ferrigura-syntax/src/preprocess.rs` (`loose_directive_len`).

### Detectors are stricter than lexers

The detectors are regular expressions on the start of the text, case-sensitive (`^\s*erDiagram`), and skip only directives and `%%` lines, while several lexers are case-insensitive: `erdiagram` never reaches the ER parser, and `/* c */ railroad-beta` is not detected. A type with several keywords (railroad and its EBNF, ABNF and PEG forms) shares one model and renderer, and the SVG's `aria-roledescription` is the detector's id, not the keyword. **Where:** `crates/ferrigura-syntax/src/lib.rs` (`skip_hidden_prefix`, the detectors), `crates/ferrigura/src/lib.rs`.

### Chevrotain parsers clear the DB after the front matter

The use case and railroad parsers clear their DB when they start, after `Diagram.fromText` has set the front matter title, so the front matter title never reaches those models. **Where:** `crates/ferrigura-syntax/src/use_case.rs`, `railroad/`; [canonical-model.md](./canonical-model.md#title-precedence).

## Text measurement and fonts

ferrigura measures text in Rust, with the bundled fonts, and compares the result with Chromium ([0010](./decisions/0010-text-measurement-and-fonts.md)). The measurements are checked against Chromium in `crates/ferrigura-text/tests`, on `tests/fixtures/chromium.json`, which `pnpm -C tools/port port:text-fixture` regenerates from the requests in it.

### Widths agree within Chromium's layout unit

Shaping with `harfrust` and summing the advances of a run, kerning and ligatures included, gives the width Chromium reports to within 0.0155 px, which is its layout unit of 1/64 px: Chromium's own canvas and SVG measurements differ from each other by the same amount. Blink keeps positions in units of 1/64 px and converts the advance of each run with a round up, so the next run starts on the grid, and a line's width, the `getComputedTextLength` that Mermaid's wrapping compares, is the sum of the snapped runs. Shaping is one run per word, as Mermaid's per-word `tspan`s are, so no kerning is applied across a space.

This matters because every layout is built on label sizes: a width that is off by a unit moves a wrap or a node. **Where:** `ferrigura-text`, `measure.rs` (`layout_width`, `measure`), `tests/widths.rs`.

### The text box rounds ascent and descent on their own

Chromium rounds a font's ascent and descent to whole pixels, each separately, with a tie going down (28.5 becomes 28), after flooring the font size to 1/100 px, and builds the text box from the rounded values. With that rounding the box height and position are exact; with unrounded metrics they are off by up to 0.36 px. Round half up misses every tie (10, 30 and 50 px among them), round half to even misses the ties at 10 and 50 px, and rounding the sum of ascent and descent misses others, so the rule is the only one that fits all the probed sizes. **Where:** `metrics.rs` (`text_box`, `round_half_down`, `laid_out_size`), `tests/metrics.rs`.

### `getBBox` of text is wider than its advance

Across, the box of a `<text>` or `<tspan>` is the union of each run's snapped advance and the outline of every glyph in it, so a glyph whose ink reaches past its advance widens the box: the `j` of Lexica Ultralegible starts 0.812 px left of its origin at 14 px, and its italic `f` ends past its advance (the width of "jun" is 20, not 19, at 14 px). Down, the box is the metrics box and the ink never changes it. Wherever the reference reads `getBBox` or `getBoundingClientRect` of SVG text, ferrigura uses `measure_box`; wrapping, layout widths and anchor shifts keep the advance. **Where:** `ferrigura-text`, `bbox.rs`, `tests/extents.rs`; the rule for the custom renderers is `ferrigura-render`, `common/text.rs`.

### Mermaid's wrapping is not CSS wrapping

Mermaid wraps SVG labels with `splitLineToFitWidth` and `splitWordToFitWidth`: it splits a line into words (the segments between Unicode word boundaries, with the spaces as segments of their own), packs words greedily, measures each candidate line as one `tspan` per word with a leading space, and splits a word that does not fit on an empty line into graphemes, at least one per line. The port of exactly that gives the same lines as the JavaScript in 270 of 270 test cases; greedy wrapping at UAX #14 break opportunities gives 254, because Chromium's CSS wrapping also breaks after a hyphen. Breaking at UAX #14 reads better for dates and hyphenated words and would break parity, so it is not offered. **Where:** `ferrigura-text`, `wrap.rs`; the labels built on it are `ferrigura-render`, `unified/label.rs`, and the measurement of the custom renderers is `common/text.rs`.

### The reference memoizes its text measurements with colliding keys

The reference caches `calculateTextDimensions`, `wrapLabel` and `breakString` under a string built from the arguments (`${text}${fontSize}${fontWeight}${fontFamily}` and so on), and the keys of two different requests can be equal: the second request then gets the first one's result. `TextMemo` keeps the same keys for the length of one render, so the same collisions happen within a render; the reference's cache lives as long as its page, so a collision across renders is not reproduced. **Where:** `common/text.rs` (`TextMemo`).

### Embedded subsets need a family name of their own

A browser draws every label at the Rust width when the glyphs it needs are embedded as subsets, also through `<img>`, where an SVG without the fonts is not pixel-identical. Five details decide whether it works:

- `@font-face` rules are global to an HTML page, even inside an inline SVG's `<style>`, and of two rules with the same family the later one wins. Two diagrams with different subsets under one family name broke 33 of 549 labels of the first diagram, by up to 1.8 px. Each subset therefore gets a family name that carries a 64-bit FNV-1a hash of the subset bytes and their weight and style (`fg-<16 hex digits>-sans`), so that diagrams with equal subsets share it and others cannot disturb each other.
- One family has the four styles as `@font-face` descriptors (`font-weight`, `font-style`), and a label switches style with `font-weight="700"`.
- `white-space: pre` has to target `text`, not the root: Chromium computes `white-space: nowrap` for `text` even when the root says `pre`, and drops a label's leading and trailing spaces.
- The subsetter keeps the legacy `kern` table. `skera`, like `hb-subset`, drops it by default; Lexica keeps its kerning in `GPOS`, but a registered font may not.
- A subset with no covered character has no `cmap`, so the embedder handles an empty family. `skera` keeps the color tables (COLR, CPAL) and every bitmap strike of a CBDT font, which makes such faces large, and it drops the `SVG ` table, so OpenType-SVG fonts cannot be embedded with it.

About 95 characters of one face come to 9 to 12.5 KB, from 52 to 72 KB for the whole face. **Where:** `embed.rs` (`subset_face`, `embed_runs`, `fnv1a`), `tests/subsetting.rs`; the `white-space` rule is in the renderers' style sheets, for example `pie.rs`.

### WebKit measures SVG text unkerned and paints it kerned

Safari 27, WebKit 26.5 and Firefox 155 draw kerned, ligated SVG text at the Rust width, as Chromium does. But WebKit lays SVG text out from advances it measures one character at a time, so `getComputedTextLength` and `getBBox` return the sum of unkerned advances (up to 32 px more for a kerning line at 48 px), while it paints each fragment as one shaped run. Everything that derives a position from the layout width inherits the error: `text-anchor: middle` is misplaced by 17 px at 48 px, `end` by 33 px, and a `tspan` without its own `x` starts 13 px too far right. Letter-spacing, word-spacing, per-character position lists and `textLength` split the text into single characters, which makes the ink unkerned as well. None of the CSS switches changes how WebKit draws kerning.

So the labels of the unified renderer carry `text-anchor` start and an `x` computed from ferrigura's own measurement for every line, and an explicit `x` for every `tspan` that follows another; a centered label gets `x = center - width / 2`. They use neither per-character positions nor `textLength`. The browser then never derives a position from its own text width, and WebKit draws where Chromium and Firefox do. Mermaid centers labels with `text-anchor: middle`, so in Safari its SVG labels with noticeable kerning sit off center. The renderers of their own keep Mermaid's `text-anchor` for titles and many labels (`common/title.rs`, `common/draw.rs`, the sequence, gantt and treemap renderers, among others), so that kerned text there is aligned by the viewer, as in Mermaid. A check that compares widths in WebKit has to use character positions or pixels, never `getComputedTextLength` or `getBBox`. **Where:** `unified/label.rs`; the harness measures in Chromium for this reason (`tools/port/geometry/pie-geometry.ts` and `tools/port/text/chromium-fixture.ts` say so), and `crates/ferrigura-text/tests/support/mod.rs` notes it for the fixtures.

### The fallback faces are COLRv0

The bundled Lexica lacks the arrows, the symbols and the emoji that diagrams use. A character the text's face lacks is set in the first fallback face that has it: the arrows (U+2190 to 21FF, Noto Sans Math), the text-default symbols of U+2600 to 27BF (Noto Sans Symbols 2, 252 code points) and the emoji with emoji presentation (Twemoji Mozilla, 1,185 of them). The three cover disjoint code points, so the order decides nothing among them, and a text symbol such as U+2665 is never set in the emoji face.

The emoji face is Twemoji in the COLRv0 format because that is the color format both Chromium and WebKit draw, inline and through `<img>`. COLRv1 draws only in Chromium (WebKit draws flat blocks or black silhouettes), a CBDT face is not loaded or draws blank in WebKit, and Apple's own sbix face, which is what the reference draws on macOS, cannot be bundled. Safari 27 draws both bundled fallback faces, inline and as an image (checked in a `WKWebView` with `tools/port/fonts/webkit-snapshot.swift`); Firefox is not checked.

The visible cost: an emoji is 16 px wide at 16 px in Twemoji, where Apple Color Emoji is 20 px. The reference's page maps the families it names onto the same bundled files, so the widths of both sides match by construction. 34 emoji with emoji presentation that Twemoji lacks, CJK, kana and Arabic are in no bundled face; the harness leaves out a fixture only when a character it draws is in no bundled face, and names the character as the reason (`tools/port/fonts/bundled.ts` reads the coverage from the files' `cmap`; the box-drawing characters of a tree view are its syntax, not drawn text). The faces are cut from their sources by `pnpm -C tools/port port:cut-fonts`, which checks each source's SHA-256 first, and a second cut is byte-identical. **Where:** `measure.rs` (`fallback_runs`, `fallback_faces`), `embed.rs`, `crates/ferrigura-text/fonts` and `NOTICE.md`, `tests/emoji_and_symbols.rs`, `tests/fallback.rs`.

### Where the reference names a font, both sides measure the bundled one

Where a renderer or a library measures text in a font of its own instead of the configured one (eventmodeling's `trebuchet ms`, cytoscape's `Helvetica Neue`, C4's `Open Sans`), ferrigura measures with the bundled face, and the harness page maps the family names `trebuchet ms`, `verdana`, `arial`, `Helvetica Neue`, `Helvetica` and `Open Sans`, the font of the wardley map and the forced font family onto the bundled files. The reference's code element in eventmodeling's boxes takes the browser's monospace, which is the machine's own, so the page pins it to the bundled mono face. The unknown icon's question mark keeps the reference's `ArialMT, Arial`, which ferrigura does not ship: the harness draws it with the machine's Arial on both sides. The reference also measures in `sans-serif` and keeps those values only when they are larger in width, height and line height at once; on macOS it kept the configured font for all 492 probed requests. **Where:** `tools/port/geometry/common.ts` (`MAPPED_FAMILIES`), `ferrigura-render`, `common/text.rs` and `tests/common_text_chromium.rs` (`the_reference_keeps_the_configured_font_for_every_probed_text`).

### Text is measured with entity placeholders and drawn decoded

The parsers see `#name;` encoded as `ﬂ°name¶ß` (`encodeEntities`), and `mermaidAPI` decodes the whole serialized SVG at the very end. Layout and wrapping therefore measure the placeholder, while the SVG shows the decoded character; the embedded font subset is cut from the decoded characters. A title that is sanitized as markup turns `&` into `&amp;`, which `.text()` writes as `&amp;amp;`. **Where:** `common/text.rs` (`shown_text`), the renderers' `text.rs`.

### SVG collapses white space, so subsets are cut from the collapsed text

A label `name\nvalue` in one SVG text is drawn as `name value`. A subset cut from the raw text has no space glyph, and the viewer then takes the space from another font, which moves the text (0.42 px in sankey). The text is collapsed before the subset is cut. **Where:** `common/text.rs`, `sankey/mod.rs` (`collapse_white_space`).

### Chromium lays text out at a floored size and on a baseline grid

Chromium lays SVG text out at its font size floored to 1/100 px, and places baselines on the 1/64 px grid: `dominant-baseline: middle` is half the x-height and `hanging` is 0.8 of the rounded ascent. **Where:** `common/text.rs` (`laid_out_size`), `treemap/text.rs`.

## JavaScript and browser arithmetic

A layout engine amplifies the last bit of a number into positions that differ by pixels, so the numbers must match, not only the algorithm ([0012](./decisions/0012-layout-engines-ported-exactly.md)).

### V8's `Math` differs from the platform's

For the same input bits, the platform's `sin`, `cos` and `log` give another last bit than V8's for a few percent of inputs. A port that calls them has that share of its radial start coordinates one ulp away from the first placement on, and a tolerance would hide that where a zero tolerance fails. `ferrigura-jsmath` computes the functions as Chromium does:

- `sin`, `cos`, `tan`, `asin`, `acos`, `log` and `log10` are correctly rounded, by Ziv's method (256 fraction bits, rounding when both bounds agree). Chromium's results are correctly rounded on every input probed, so the two agree.
- `atan` and `atan2` are ported from llvm-libc, which Chromium's V8 takes them from; they are not correctly rounded.
- `pow` is the platform's, with V8's special cases, and `hypot` is V8's two-argument formula. Both runtimes agree on these two, and only macOS was checked for `pow`.
- The `node` module has Node 24's functions (V8's fdlibm port), because the parser differentials run in Node and Node and Chromium disagree on `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log` and `log10` for 0.5 to 11 percent of inputs.

The renderer takes every one of these functions from `ferrigura_jsmath` and never from the standard library: `ferrigura-render/tests/browser_math.rs` fails on a call of any of 26 platform functions (including `powi`, whose precision Rust leaves open). A million inputs per function against Chromium are compared by `pnpm -C tools/port port:jsmath`. **Where:** `crates/ferrigura-jsmath`.

### Fused multiply-adds make some outputs depend on the architecture

V8's C++ and llvm-libc are compiled with the compiler's contraction, which fuses `a * b + c` on targets that have a fused multiply-add instruction: every aarch64 target, and x86-64 with the `fma` feature. The x86-64 baseline has none, so V8 does not contract there. The functions that depend on it are `atan`, `atan2` and Node's fdlibm functions; the correctly rounded ones and `hypot` do not. `CONTRACTS` says which way the build goes, and the samples for the dependent functions were taken on aarch64. Hence the snapshots are taken on aarch64 ([0005](./decisions/0005-quality-gates.md)), and CI compares the venn layout exactly against its captures on arm64 and within a tolerance on Linux. **Where:** `ferrigura-jsmath`, `fused.rs`; `.github/workflows/ci.yml` (the venn step).

### Numbers become text the way JavaScript writes them

`String(number)` is the shortest digits that read back as the number, in plain notation from `1e-6` up to `1e21` and in exponent notation outside that (`1.8369701987210297e-14`, `1e+21`), with both zeros as `0`. When two shortest candidates are equally near, JavaScript takes the even digit and Rust may take the other (`1708300000000000.25` is `...0.2` in JavaScript and `...0.3` in Rust), so `number_text` rounds to the same length with ties to even and takes that when it reads back as the number. Every number the SVG shows goes through it. **Where:** `ferrigura-model`, `value.rs` (`number_text`), checked against Node on a table written by `tools/port/number-text-table.ts`.

### Numbers are read exactly, and the browser works in float32

`serde_json`'s default float parser is not correctly rounded. The workspace enables `float_roundtrip`, so JSON numbers are read as JavaScript reads them; without it, differences of about 1e-14 showed on otherwise identical ELK cases, and a width of `14.013293266296387` read one ulp away moved a Brandes and Köpf size. `JSON.stringify` writes `-0` as `0`, so captured values are compared with `==`.

The browser side has float32 in places: `getBBox` returns float32 values, and Blink parses the numbers of a path into float32. `browser_f32` and `path_coordinate_f32` do the same, and several tests in the unified renderer fail by one float32 ulp if they do not ([SVG and Chromium geometry](#svg-and-chromium-geometry)). **Where:** the root `Cargo.toml` (`float_roundtrip`), `ferrigura-render`, `unified/geom.rs`.

### JavaScript's sort order, key order and string order are part of the result

- `Array.prototype.sort` in V8 is TimSort, and a comparator result that is NaN counts as 0. The sankey layout sorts with `a.y0 - b.y0`, which is NaN for a position that is NaN or infinite, so two items can be "equal" to each other and "less" than a third, and the order that comes out depends on the comparisons the algorithm makes. The port is V8's TimSort, with its arguments in its order. **Where:** `common/d3/sort.rs`, `tests/d3_sort_v8.rs`.
- GWT's `Collections.sort`, which elkjs uses, is not TimSort but the legacy merge sort: insertion sort below seven elements, a shortcut when the halves are ordered, and a merge that prefers the left run. **Where:** `ferrigura-elk`, `java/sort.rs`.
- `Math.min` and `Math.max` give NaN when any operand is NaN, where `f64::min` and `f64::max` ignore it, and they distinguish signed zero. **Where:** `ferrigura-venn`, `js.rs`; `unified/tidy_tree.rs`; `unified/path.rs`.
- `Object.keys` lists integer-like keys first, in ascending order, and a plain object with the key `__proto__` sets its prototype instead of adding a key. The venn library's circles come out in `Object.keys` order, and fcose's `relocateComponent` misses a node with the id `__proto__`. **Where:** `ferrigura-venn`, `js.rs` (`JsObject`, `js_key_order`); `ferrigura-cose`, `fcose/mod.rs`.
- `localeCompare` with the default locale follows ICU's root order, in which the two cases of a letter share a primary weight; the swimlane layout sorts with it. The port gives that order exactly for printable ASCII, read from Node's own sort of the 95 characters, and reports an error for other strings until a collator is added. **Where:** `ferrigura-swimlane`, `js.rs` (`locale_compare`).
- JavaScript strings are UTF-16, and lengths, slices and regular-expression positions count code units. Where a result depends on them, the code counts them too: marked's lexer masks and emphasis positions shift for an escaped character outside the Basic Multilingual Plane. **Where:** `unified/marked.rs`.

## SVG and Chromium geometry

### The bounding box of a path is Blink's and Skia's, in float

`getBBox()` of an SVG path is `SkPath::computeTightBounds()` of the path that Blink's parser and `SVGPathBuilder` build: numbers are read in `float` by Blink's digit-by-digit parser, relative points become absolute, smooth curves are reflected, and an arc becomes conics from float matrices; the extremes of each curve are found in float. `ferrigura-path` ports that route. It matters for the mindmap's cloud and bang nodes, whose paths size the node, and where the force layout amplifies a one-unit difference into tens of pixels. Of 3,011 generated paths, 81 differ from Chromium, all within 0.05 px but one (0.094 px on a path 293,000 px across), because Chromium takes the macOS libm's float `sin`, `cos`, `tan` and `atan2`, which are not correctly rounded for a few inputs; the port takes the correctly rounded values, the same on every host. Mermaid's cloud and bang paths are not among the 81. **Where:** `crates/ferrigura-path`, `tests/corpus` (written by `pnpm -C tools/port port:path-bounds`).

### Float32 values meet exact comparisons

The reference's post-processing of an ELK result tests borders exactly, so one float32 ulp in a box moves an edge end by pixels, and the shapes' sizes come from `getBBox`, which is float32. Two consequences are in the code. A sloped rectangle's height follows a rule measured in Chromium, not derived from its source: when the span of its outline is not a float32, the browser's height is one float32 above the nearest (433 outlines measured with no exception, 163 exact spans unchanged); the same rule misses on 11 of 178 and 87 of 484 inexact spans for unrelated coordinates, so other paths keep the nearest float32. And the values the renderer hands the post-processing are the float32 ones. **Where:** `unified/geom.rs` (`browser_f32`, `path_coordinate_f32`, `sloped_outline_height_f32`), `unified/elk_geometry.rs`.

### A width off the 1/64 px grid changes the text

A width that is not on the layout unit makes Chromium scale the SVG by a fraction and lay the text out at the scaled size, which moves its width by a few hundredths of a pixel. The reference's widths are on the grid because the browser measured them, so the pie's width is snapped (`layout_unit`). The comparison must also wait for the embedded faces to load before it measures. **Where:** `pie.rs`; `tools/port/geometry/common.ts` (the page waits on `document.fonts`).

### rough.js at roughness 0 is deterministic

The shapes that the reference draws with `rc.*` set `roughness: 0` and a solid fill, so their drawing is deterministic: every offset is multiplied by the roughness, the points and the control points lie on the line, and an ellipse is its sampled core. The one random part left, the divergence point of a double stroke (`0.2 + random * 0.2`), does not change the geometry, so a fixed fraction stands for it, and the geometry run compares a path sample on a jump between two sub-paths by both ends of the jump. The port is rough.js's generator and the renderer ops that these shapes reach. The `handDrawn` look is out of scope, and its fixtures are excluded. **Where:** `unified/shapes/rough.rs`.

### Chromium reads `NaN` as 0, and a detached element measures 0

An element whose geometry is `NaN` adds nothing to the root's bounding box, so it does not move the `viewBox`, while a text with `y="NaN"` still counts. Several types reproduce `NaN` geometry where the reference computes it (sankey, treemap, block). A text inside a detached element measures 0 with `getComputedTextLength()`, so a wrap that runs before the element is attached never wraps (venn labels are one line). **Where:** `sankey/mod.rs` (`finite_box`, `svg_length`), `venn/text.rs`.

### Colors set through d3's `style` come back serialized by Blink

After `.style('fill', …)`, Chromium serializes a hex color as `rgb()` and converts `hsl()` in single precision (`hsl(247.5, 0%, 9.2156862745%)` becomes `rgb(24, 24, 24)`, where double precision gives 23); an `hsl()` with a `NaN` component is dropped. **Where:** `quadrant_chart/mod.rs` (`css_color`).

## The layout engines

Each engine is ported from its own sources and checked against the original on a captured corpus, with a tolerance of zero ([0012](./decisions/0012-layout-engines-ported-exactly.md)). The README of each crate has its source pin, its map to the sources and how its corpus is made: [ferrigura-elk](../crates/ferrigura-elk/README.md) and [ferrigura-cose](../crates/ferrigura-cose/README.md); the tidy tree is documented in `crates/ferrigura-render/src/unified/tidy_tree.rs`.

### ELK Layered follows elkjs, not the Java alone

Mermaid runs elkjs 0.9.3, which GWT compiled from ELK's Java, so the port follows what the compiled code does. The ELK commit is not recorded by elkjs and is inferred from the bundle; the options, defaults and enums are generated from the `*.melk` files, because the metacompiler's Java classes are not in the repository. Behavior the Java does not show cost the most time: two tries of the layer sweep share one flag because of a copied id, unset options read the defaults of Layered's `supports` table, GWT's `Random` and merge sort are not the JVM's, and elkjs mutates its input and adds `$H` identity hashes to its output. Only Layered is ported, and only the options Mermaid sets; the other algorithms fall back to Layered with a warning.

The comparison is layered on purpose. The processor trace with its logs is compared before the coordinates, so an error in the order of processors shows as a name, not as a shifted node; the corpus is backed by 1,200 random flat and 900 random compound graphs that the generators write, which reach what no fixture does; and every fix starts as a synthetic corpus case. **Where:** `crates/ferrigura-elk` and its README.

### Mermaid's ELK integration is ported as well as ELK

What Mermaid does around elkjs is part of the layout: it builds the ELK graph from the layout data (`buildElkGraphFromLayoutData`), and after the layout it applies the positions, evens group frames, clips edges to the shapes' outlines, straightens the port-to-channel jogs at the ends of routes and places terminal labels. These steps decide where an edge ends, so they are ported function for function, with their thresholds. The reference's patch 07 gives the harness each ELK input and layout (`globalThis.__mermaidElkCapture`), so the harness compares the graph ferrigura hands to ELK with the reference's before it compares any geometry: the two are equal for all 298 comparable flowchart fixtures. The Mermaid option `layout: dagre` is laid out with ELK as well; the geometry runs leave such fixtures out. **Where:** `ferrigura-render`, `unified/elk_build.rs`, `elk_apply.rs`, `elk_geometry.rs`, `elk_straighten.rs`, `terminal.rs`; [flowchart](./diagrams/flowchart.md).

### cose-bilkent is the tree path, and its edge ends are cytoscape's

A mindmap is one tree, so cose-base takes its radial path: it finds the center of the tree by peeling leaves, places the nodes on rings around it and starts the spring embedder from there. That path draws no random numbers, so the layout does not depend on the page; the random path is unreachable from Mermaid and is not ported. The source has quirks that the result depends on and that the port keeps: `findCenterOfTree` splices its list while it iterates it, which makes the center wrong for many trees (a path of five nodes gets its second node), and `branchRadialLayout` divides `(end - start + 1) / 2`.

Two things sit outside the obvious engine. cytoscape-cose-bilkent loads cose-base 1.0.3 and layout-base 1.0.2, not the 2.x versions of fcose, so the two engines are separate ports. And the edge endpoints that Mermaid reads back (`rscratch`) are cytoscape's, for its default 30 by 30 node style, not Mermaid's box sizes: every edge end lies on a circle of radius 15 around its node's center, and the mindmap renderer reproduces that. **Where:** `crates/ferrigura-cose`, `ferrigura-render`, `mindmap/layout.rs`; [mindmap](./diagrams/mindmap.md).

### fcose is seeded per run and runs twice

`layoutArchitecture` calls fcose twice per render, the second run from the first run's output. fcose draws from the mulberry32 generator of Mermaid's `architectureSeed.ts`, which replaces `Math.random` for one run from the seed in the configuration; a seed of 0 means "keep the real `Math.random`", which cannot be reproduced, so the port takes seeds from 1 on and the harness's generated diagrams use 1 to 9. Mermaid always passes an alignment object, so fcose turns constraints on, tiling and tree reduction off, and runs the spectral start for `randomize: true`. Only the positions of the leaves are compared, since cytoscape places a compound from its children. **Where:** `crates/ferrigura-cose`, `fcose/`; [architecture](./diagrams/architecture.md).

### The tidy tree is a package that the bundle does not register

`layout: tidy-tree` needs the package `@mermaid-js/layout-tidy-tree` to be registered by the host page; Mermaid's own bundle does not contain it, and without it the layout quietly becomes cose-bilkent. ferrigura draws the tidy tree, so the reference for it is the registered package, built by the harness from sources vendored under `tools/port/vendor`. The layout is van der Ploeg's non-layered tidy tree on a transposed tree (the breadth of a tree is the node's height) with four quirks that the port keeps because the contract is the reference's output: the first level is centered on the tops of its nodes, the reported midpoint is not an endpoint, `mindmapCircle` is not among the round shapes, and an edge between two nodes of one side doubles back. **Where:** `ferrigura-render`, `unified/tidy_tree.rs`; [mindmap](./diagrams/mindmap.md).

### The other engines

The Venn layout (`@upsetjs/venn.js`: greedy placement, and from eight areas on the constrained multidimensional scaling when it has the smaller loss, then the downhill simplex) draws `2n` numbers per restart, ten restarts, from `Math.random`; the crate takes the random source from the caller, and the renderer uses the harness's generator, seeded per render, so that the draws match ([venn](./diagrams/venn.md)). The swimlane layout is Mermaid's own Sugiyama pipeline with an orthogonal router and the geometry passes of its post-processing, ported on the default path and checked exactly on 131 captured calls of its seam; its lanes are drawn with rough.js ([swimlanes](./diagrams/swimlanes.md)). **Where:** `crates/ferrigura-venn`, `crates/ferrigura-swimlane`.

## The reference and the harness

### The reference is loaded through Vite's SSR loader

The exporters run with `tsx`, but they load the reference's modules through Vite's SSR loader (`createServer` with `ssrLoadModule`), not through Node's module loader. The reference's configuration imports `config.schema.yaml?only-defaults=true`, a query that the schema plugin of the reference's `.vite/` handles and that Node and tsx cannot load. Vite runs the same plugins as the reference's tests, so the reference is loaded the way it is tested. Two details follow: the detectors must be registered before `preprocessDiagram` runs (the reference's directive handling calls `detectType`, which throws for any directive when none is registered), and DOMPurify, which the DB setters call, needs a window, so most exporters install a jsdom window before loading the reference. **Where:** `tools/port/reference.ts` (`registerDetectors`, `provideDomForSanitizing`), `reference-root.ts`; [tools/port/README.md](../tools/port/README.md).

### The page fixes the clock, the random numbers and the environment

Two renders of one text would differ in the reference: the gantt's today marker reads the clock, gitGraph's generated commit ids and rough.js strokes read `Math.random`. The harness page replaces `Date` and `Math.random` for each render with a fixed time and a mulberry32 stream from a seed, passes the same to ferrigura (`RenderOptions::now`, and a hidden `git_graph_seed`), and runs in UTC, because the reference reads dates in the browser's time zone and ferrigura in UTC. `pnpm -C tools/port port:geometry <type> --self` renders the reference twice and fails on any difference, which finds the nondeterminism the page does not yet cover. In the parse differential, ids that the reference draws at random (gitGraph's, the regions of a state diagram) are generated deterministically in the reference's format and replaced by placeholders in order on both sides.

The page's conditions are part of the comparison: Chromium runs with `--font-render-hinting=none`; the viewport and the screen are 1200 by 900 px and the body has no margin, so a renderer that reads the screen or its container (C4's `screen.availWidth`, the gantt's `offsetWidth`) gets 1200 px, which ferrigura takes unless the configuration sets a width; `htmlLabels: false`, the bundled fonts and the strict security level are forced, on both sides and also as the site configuration, because an invalid `init` directive in a fixture makes the reference drop every directive; a theme that ferrigura does not ship is replaced on both sides by the fallback, since only colors differ; fixture text is decoded as the e2e viewer decodes it (`entityDecode`); and each SVG is measured at scale 1. **Where:** `tools/port/geometry/common.ts` (`renderReference`, the page's `<style>` and `<script>`), `diagram-geometry.ts`.

### Linux and macOS Chromium lay out text differently

The geometry comparison runs on macOS: Chromium there keeps fractional glyph advances, as ferrigura's measurement does. On Linux, with hinting off, the redux themes match, but other fixtures differ (see [Open questions](#open-questions)). The parse and layout-data runs and the engines' corpora do not depend on the browser and run anywhere. **Where:** `.github/workflows/ci.yml` (the `port-geometry` job runs on `macos-latest`).

### The reference itself fails on some inputs

A fixture on which the reference throws, or draws garbage, is not a difference of ferrigura. The geometry run classes it as "reference failed" (the sankey probes that fail in the reference, two requirement fixtures) and a differential run lists it as a named exception: `layout: elk` on a mindmap crashes the reference for every mindmap, so ferrigura lays such a mindmap out with ELK and the harness does not compare it. Such a rule is written down with its reason, never skipped silently ([0014](./decisions/0014-upstream-reference-at-a-pin.md)). **Where:** `tools/port/exceptions.ts`, `geometry/diagram-geometry.ts`.

### The reference's output goes through DOMPurify, and starts with an empty group

At every security level but `loose` and `sandbox`, the reference runs DOMPurify over the finished SVG, which drops `use` elements (sequence actor icons are not drawn) and `text-height` and trims attribute values; ferrigura writes what is left. `mermaidAPI` appends an empty `<g>` before every diagram, so the renderers write `<g></g>` first, and the geometry key counts it. **Where:** `sequence/conf.rs`, `gantt/draw.rs`; the renderers' root functions.

### Configuration is read as the reference reads it, not as the schema says

`defaultConfig.ts` builds some sections from scratch, so schema defaults such as `class.padding` do not exist in the reference; the generated configuration leaves them without a default (`CLASS_KEYS_WITHOUT_DEFAULT`, `RAILROAD_KEYS_WITHOUT_DEFAULT` in `crates/ferrigura-model/build.rs`). Some renderers read another type's section (class and requirement read `state.titleTopMargin`, `useMaxWidth` and spacing), some read their style options from `themeVariables.<type>` rather than their section (packet, radar), and the theme's variables are merged over the default theme's, so an unset variable takes the default theme's value. `port:diff config` checks the resolved result ([0008](./decisions/0008-canonical-model-and-typed-views.md)).

### The reference keeps state between renders, ferrigura does not

The reference keeps module-level state across the renders of one page (`actorCnt`, the gantt's `taskDb`, the timeline's `currentTaskId`, the measurement memo). ferrigura keeps none, and the exporters load a fresh DB module per input where the reference's state would leak (`tools/port/export-gantt.ts`). The default SVG id is a hash of the text and the configuration (`crates/ferrigura/src/lib.rs`, `derived_id`), so the output is deterministic, and any change to a fixture, even a `%%` comment, changes its snapshot ([0018](./decisions/0018-deterministic-output.md)).

## The porting method

What worked, in the order it was needed.

### Port the spec files first, test for test

Every spec file of the reference gets a Rust counterpart with the same test names ([0015](./decisions/0015-differential-testing.md)). The specs measure progress instead of estimating it, they hold the behavior that nobody wrote down elsewhere, and a spec that cannot pass is ignored with its reason instead of deleted, so the ignored count stays visible ([0005](./decisions/0005-quality-gates.md)). The counts are in [diagrams/](./diagrams/README.md#coverage).

### Compare at every stage

A wrong number in the parse shows as a model difference, not as a shifted node three stages later. So the stages are compared separately: the canonical model, then the layout data (`getData()`), then the layout engine's input and output, then the geometry. The layout data run for a unified-renderer type finds a `getData` difference before the geometry does, and the ELK input comparison finds a difference in the graph before the layout is blamed. The order of a trace comes before coordinates. Each comparison names the first difference, and counts differences by field.

### Run the reference's own code, and label what is known

A rule is only learned by running the reference: the built bundle in Chromium, the source with `tsx` for parser and DB code, or the libraries in `node_modules`. The reference is never reimplemented to learn what it does. In the notes of a port, each fact says how it is known: probed (the reference ran and the result was seen), read (in the source, not run) or unverified. A probe that settled a quirk becomes a test: an explicit case in `tools/port/cases.ts`, a probe under `tools/port/geometry/probes/<type>`, or a known answer in a crate's tests.

### Reproduce quirks, and name the exceptions

A quirk that changes the output is reproduced, with its probe as a test: the compatibility promise is that the same text renders the same diagram. A difference is accepted only where the reference crashes or its result depends on the page (a random value, the clock), with its reason, and a bug is fixed only together with a patch on the reference ([0017](./decisions/0017-reproduce-the-reference.md)). The easy mistake is to fix a quirk because it looks like a bug.

### Port the dependency, not a substitute

Where a result depends on a library, the library is ported, and checked against the original:

- **js-yaml.** A YAML 1.2 parser disagreed with js-yaml on 8 to 13 percent of generated inputs (indentation, tabs, empty keys, flow lines), because js-yaml is not a YAML 1.2 reader in every detail. `yaml.rs` is a port of js-yaml's own loader with `JSON_SCHEMA`, state for state; `pnpm -C tools/port port:diff yaml` compares them on over 4,000 inputs.
- **marked.** Mermaid reads label Markdown with marked's lexer. The port keeps its rules, builds the patterns with the same substitutions, translates JavaScript regular expressions to Oniguruma and counts UTF-16 code units where marked does. `port:diff markdown` compares lines and words.
- **DOMPurify.** The subset that `sanitizeText` reaches is ported, and runs twice on the strict path, as the reference does: once with the defaults, then with `FORBID_TAGS: ['style']`.
- **rough.js, d3 and V8.** rough.js at roughness 0, the d3 modules the renderers use (scales, arcs, time and number formats, the treemap, the sankey layout), V8's `Array.prototype.sort` and its `Math`.

**Where:** `ferrigura-syntax`, `yaml.rs` and `sanitize.rs`; `ferrigura-render`, `unified/marked.rs`, `unified/shapes/rough.rs` and `common/d3/`.

### Transcribe arithmetic, do not simplify it

Where an engine's numbers must match, the port keeps the source's expression trees and its iteration order, uses arena indices for linked structures (the tidy tree's threads and sibling lists), and returns an error where the original throws instead of panicking. Floors on corpus sizes only go up, a corpus has a manifest that a test keeps in step with its files, random generators are seeded and write outside the repository's tracked files, and each fix of an engine starts as a corpus case that fails.

## Open questions

- **Pie on Linux Chromium.** With `--font-render-hinting=none` the redux themes match the reference, but fixtures that name a classic theme (default, dark, forest, neutral), the hovered slice and the bottom legend differ by up to 0.7 px. The cause is not found; the next step is to run the comparison in Playwright's Docker image. CI compares on macOS, where all of them match.
- **Scripts the bundled faces lack.** CJK, kana, Arabic, the 34 emoji with emoji presentation that Twemoji lacks and the symbols that Noto Sans Symbols 2 lacks have to be registered by the user, and fixtures with them are not compared. Firefox is not checked for the bundled fallback faces.
- **Text measurement of the custom renderers on Linux and Windows.** The reference keeps the configured font over `sans-serif` on macOS for every probed text; the system `sans-serif` on other systems may be larger, and ferrigura measures the configured font only.
- **Arrows in eventmodeling's HTML box labels.** The reference draws them in `foreignObject`; the arrow probe is 0.5 to 1.5 px off in the line positions, with the horizontal geometry equal. The probable cause is the reference's line box taking the metrics of the fallback face (Noto Sans Math has an ascent of 1069 and a descent of 423 per 1000 em, Lexica 950 and 290); not confirmed, and not in the fixtures.
- **Other targets than macOS and aarch64.** The samples of the functions that depend on fused multiply-adds were taken on aarch64, `pow` is the platform's and was checked on macOS only, and x86-64 and Linux are checked where CI runs them.
- **The swimlane layout's sort.** Its port of `Array.prototype.sort` is exact for arrays of 17 to 129 elements as Chromium 151 runs it, with two gaps (2 to 16 elements, and 130 and more), where the sort of `common/d3/sort.rs` follows V8's TimSort completely (`ferrigura-swimlane`, `js.rs`, `js_sort_by`).
- **Grapheme and code-unit counts.** Whether `Intl.Segmenter` and `unicode-segmentation` agree on the newest emoji sequences is not checked, and the treemap's label truncation counts characters where the reference counts UTF-16 units, so an astral character may be cut differently (not probed).
- **WOFF2.** The subsets are embedded as TTF; WOFF2 would roughly halve them, but needs an encoder that has not been evaluated.
