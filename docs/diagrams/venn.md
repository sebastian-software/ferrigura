# Venn diagram

A venn diagram draws area-proportional sets: each `set` is a circle, each `union` an intersection with its own size, and labels, text nodes inside the areas, `style` colors and a title decorate them. It starts with `venn-beta`. ferrigura parses it with a hand-written lexer and parser in `ferrigura-syntax` (`src/venn`), draws it with a renderer of its own in `ferrigura-render` (`src/venn`, not the unified renderer), and places the circles with `ferrigura-venn`, the port of the layout of `@upsetjs/venn.js` 2.0.0 that Mermaid's `vennRenderer.ts` calls ([0012](../decisions/0012-layout-engines-ported-exactly.md)).

## Model

The canonical model is the diagram DB after the parse (`vennDB.ts`), as `VennDiagram` in `ferrigura-model` (`src/venn.rs`). The conventions of [canonical-model.md](../canonical-model.md) apply; these are the fields.

| Field       | Holds                                                                                                   |
| ----------- | ------------------------------------------------------------------------------------------------------- |
| `title`     | The last `title` statement, sanitized; the front matter title when the grammar has none; `""` otherwise |
| `subsets`   | `{ sets, size, label }` for every `set` and `union`, in statement order, duplicates kept                |
| `textNodes` | `{ sets, id, label }` for every `text` statement and every indented text line                           |
| `styles`    | `{ targets, styles }` for every `style` statement; `styles` is a list of `[key, value]` pairs           |

What the DB keeps and the model keeps with it:

- `sets` and `targets` are normalized (trimmed, one pair of double quotes removed) and sorted as JavaScript's default `sort` sorts strings, by UTF-16 code units: `"10"` comes before `"2"`. The `Object.keys` order that the layout uses is not in the model.
- `size` is the statement's number, or `10 / n²` for the `n` identifiers as written (10 for a set, 2.5 for a pair). A negative size and zero are kept. A size that is not finite is its sentinel string, as the exporter writes it, since `JSON.stringify` would write `null`.
- `label` is `""` when absent. Labels and text ids are not sanitized: markup stays raw, and an entity placeholder (`ﬂ°name¶ß`) stays as the DB holds it. Only the title is sanitized.
- `styles` follows the DB's plain object: a key given twice keeps its first place and takes its last value, and `__proto__` is dropped.

The model leaves out the layout and the geometry, the parser's indent mode, the known-set bookkeeping, and the configuration.

## Parsing

`prepare`, the comment cleanup, the directive removal and `encodeEntities` run as `Diagram.fromText` runs them, and the text gets a final line feed. The front matter title is set first; a grammar `title` replaces it.

The scanner (`scanner.rs`) is `venn.jison`'s lexer as a hand-written rule table, tried in the written order, first match wins, with Jison's implicit `\b` after the keyword literals ([0007](../decisions/0007-hand-written-parsers.md)). The parser (`mod.rs`) is recursive descent with one token of lookahead. The DB calls run where Jison's reductions run, which matters for the indent mode: a `set` or `union` turns it on when its last token has been read, before the line break that ends it is lexed, so the next line is lexed in the mode. The DB (`db.rs`) holds the model plus the known sets and the current sets.

The quirks of the grammar that ferrigura keeps on purpose, each pinned by a test in `venn_quirks_spec.rs`:

- **Indent mode.** After a `set` or `union`, an indented `text` line is an indented text and joins that statement's sets. A line that starts with a non-blank character ends the mode, so a `style` or `title` line at any indent ends it, while a comment line or a `x%%` line does not. A `%%{` at the start of a line ends it too. An inline `text A "x"` works only at column 0; indented, it is a parse error.
- **A comment glued to a token takes the character before `%%`.** `set A%% c` is an error, `set A %% c` is not. `[^\}]` matches one UTF-16 unit, so a character outside the BMP before `%%` is an error too.
- **Statements need no separator.** `set A set B` and `venn-beta set A` are two statements each.
- **Keywords need a word boundary.** `setA`, `settings`, `unionA` and `textual` are identifiers. The keywords are case-insensitive in the parser (`VENN-BETA` parses), though the detector, `/^\s*venn-beta/`, does not take that header.
- **A title** ends at `#` or `;` (a lexical error follows), keeps its trailing blanks and may contain `%%`.
- **A union** needs two identifiers, each a `set` given before it; the error names every unknown id. A union may repeat an id.
- **A number in a `text` statement takes no label**; the grammar has no such rule.
- **A bare `venn-beta` is accepted.** The renderer rejects it.
- **A non-breaking space is not a blank** for the lexer.

Errors carry positions against the original input, including the front matter; the messages are ferrigura's own.

## Layout and rendering

`render` in `ferrigura-render/src/venn/mod.rs` ports `vennRenderer.ts` (`draw`) and `styles.ts`. Its configuration section is `VennSection`: the `venn` configuration (`width` 800, `height` 450, `padding` 8, `useMaxWidth`, `useDebugLayout`) and the seed of the layout's random numbers.

**The data.** `ensurePairwiseSubsets` (`data.rs`) adds, for every union of three or more sets, the pairs of its sets that are not subsets yet, in order. A pair's size is a quarter of the smaller of its two single sets' sizes, or 2.5 when one of them has no single-set subset. The result is the list both layout calls receive.

**Two layout calls, as the reference makes them.** The chart (`ferrigura_venn::chart`, the library's `VennDiagram`) draws the circles and the labels with the box `width` by `height` minus the title band and the library's padding of 15, which the renderer does not pass. `ferrigura_venn::layout` gives the areas' text centers and circles for the text nodes with the configuration's padding of 8. Both draw from one random source, the chart first. The chart leaves out the zero-size single sets and every area that names one; `layout` keeps them.

**`ferrigura-venn`** ports the parts of the library the renderer reaches, from `build/venn.esm.js` (which inlines `fmin`): `venn` (greedy placement; from eight areas on, the constrained multidimensional scaling with ten restarts, kept when it beats the greedy loss by more than 1e-8; then the downhill simplex over the squared overlap error), `addMissingAreas`, `normalizeSolution` (disjoint clusters oriented at π/2 and arranged in a grid), `scaleSolution`, the text centers (`computeTextCentres` with its own simplex), and the intersection paths. `js.rs` holds the JavaScript behaviors that decide the order of results: `Object.keys` order (`JsObject`), V8's stable sort, `Math.min` and `Math.max` on NaN and signed zero, UTF-16 string order, and the names a plain object inherits. `sin`, `cos`, `atan2` and `acos` come from `ferrigura-jsmath`. `NOTICE.md` lists which build lines each function ports.

**The drawing.** The SVG follows the reference's element order: the base style sheet, the empty group `mermaidAPI` adds, then the title, then `g` with `translate(0, titleHeight)` holding one `g.venn-area` per drawn area and, last, `g.venn-text-nodes`.

- `scale` is `width / 1600`; the title is `32 * scale` high at `y = 32 * scale`, and its band is `48 * scale`. The `viewBox` has the full height.
- A circle path is `circlePath`, an intersection path the arcs of `intersectionAreaPath`, both unrounded, written with JavaScript's number conversion. The `venn-set-N` class and the palette color follow the order of the circles drawn. The fill opacity is 0.1, the stroke width `5 * scale`, the set label `48 * scale` high, and the label's x and y are the floored center. An intersection is transparent unless a style gives it a fill.
- A circle's label color is the style's `color`, else the base color darkened by 30 on a light background and lightened by 30 on a dark one (`khroma` port in `ferrigura-model`). A color the functions cannot read fails the render, as the reference throws.
- A style applies to the key of its sorted targets joined by `|`; statements with the same key merge, property by property, the later one winning. `style A,B` styles the pair only.
- A set label is one `tspan`: its words joined by single blanks, the way `wrapText` leaves it (see Learnings). A set without a label shows its id, an intersection without one shows nothing.
- A text node is `text` with one `tspan` per line, placed in a grid inside its area (`text_area`): the center and the inner radius of the area (a zero radius becomes 0.6 times the smallest circle radius), a grid of `ceil(sqrt(n))` columns inside `max(80 * scale, 0.95 * 2r)`, moved down by a label offset when the area has a label and by `30 * scale` when it has two nodes or fewer. A node's box is 90% of its cell. A text node of an area that `layout` has no area for is dropped. With `useDebugLayout`, the debug circle and cells are drawn.
- Colors are CSS custom properties on the diagram's id, light and dark under `prefers-color-scheme` for `colorScheme: auto`, and each element carries its light color as an attribute ([0011](../decisions/0011-themes-as-css-variables.md)). The palette is `venn1` to `venn8` of the theme, with the empty ones removed.

## Differences from Mermaid

- **Text nodes are SVG text.** The reference draws each in a `foreignObject` with an `xhtml:span`, also when `htmlLabels` is false. ferrigura breaks the lines itself, with the flex item as wide as the box or its widest unbreakable piece, and each line centered ([0009](../decisions/0009-svg-text-labels.md)). The geometry run compares the lines of both sides.
- **The layout's random numbers have a seed.** The reference draws them from `Math.random`; ferrigura draws the geometry harness's generator (mulberry32) from `DEFAULT_SEED` 1, so one text gives the same SVG every time. The facade's hidden `random_seed` option lets the harness give the reference's seed.
- **`look: handDrawn` draws the classic geometry** (rough.js is out of scope; see the [limits](../ARCHITECTURE.md#limits)).
- **A set named `__proto__` is refused** with a layout error: in a plain object, that key changes the prototype, and the library's output for it is not reproduced. Other names that `Object.prototype` has (`constructor`, `toString`) are reproduced.
- **The error messages are ferrigura's own** ([0007](../decisions/0007-hand-written-parsers.md)).
- **Library parts Mermaid does not reach are not ported:** the options the renderer never passes (`distinct`, `scaleToFit`, a custom loss), the `path`, `arcs` and `distinctPath` strings of `layout()`, and the library's `console.log` warnings.

## Known gaps

- `look: handDrawn` is not drawn; the fixtures that use it are excluded from the geometry run (`EXCLUDED_LOOK`).
- On a target without fused multiply-adds (x86-64), the layout is exact only up to the last bits of `atan2`: the simplex then takes slightly different steps, and the captured cases agree within 0.1 (the largest difference measured is 0.042). The snapshots are taken on aarch64 ([0012](../decisions/0012-layout-engines-ported-exactly.md)).
- The license of `fmin`, which venn.js bundles, is unverified: venn.js's `LICENSE` lists only its own copyright (`NOTICE.md`).
- No `pending` or `unsupported` exceptions exist for this type (`tools/port/exceptions.ts`).

## Checking it

- `pnpm -C tools/port port:diff venn` compares the canonical models on every input: the fixtures, the string literals of Mermaid's specs, and the cases in `tools/port/cases.ts` (each quirk above has one). A rejection on both sides agrees. `undetectedCases.venn` lists the inputs the reference's detector does not take.
- `pnpm -C tools/port port:geometry venn` renders the fixtures and the probes in `tools/port/geometry/probes/venn` with both and compares circles, intersections, label positions and text lines within 0.05 px. The probes cover the random branch (four sets), numeric ids, zero-size sets, a disjoint labeled union, duplicated statements, the debug layout, titles with markup, blank labels, widths, paddings, styles, and a palette of more than eight sets.
- `pnpm -C tools/port port:diff config` and `port:diff theme` cover the `venn` section and the `venn1` to `venn8` tokens.
- `cargo test -p ferrigura-venn --test corpus` compares the layout with captures of the library in Chromium: every stage (greedy, constrained, `venn`, the chart, `layout()`) for the fixtures, hand-written probes and generated set systems. It is exact where the target fuses multiply-adds (CI runs it in release mode on macOS). `pnpm -C tools/port exec tsx crates/venn/capture-corpus.ts` regenerates the corpus, with the seeded generator `generate-set-systems.ts`.
- The Rust specs: `ferrigura-syntax/tests/venn_spec.rs` (Mermaid's `venn.spec.ts`) and `venn_quirks_spec.rs` for the parser; `ferrigura-render/tests/venn_renderer_spec.rs` (`vennRenderer.spec.ts` and `vennPalette.spec.ts`, for the two shipped themes) and `venn_quirks_render.rs` for the renderer; unit tests beside `data.rs`, `random.rs` and `text.rs`.
- The snapshots pin the SVG of every fixture and probe.

## Learnings

- **Two paddings.** The chart keeps the library's padding of 15, because `vennRenderer.ts` passes only the size, while `layout()` gets `config.padding` (8). The text nodes are placed from the padding-8 geometry, so they sit a few pixels off the circles the reader sees. This is the reference's behavior and ferrigura keeps it; making both equal would move every venn with text nodes.
- **Set labels are never wrapped.** `wrapText` measures with `getComputedTextLength`, which is 0 on the detached element the chart draws into, so no line is ever too wide. The label is one `tspan`. `split(/\s+/)` still collapses runs of white space, stops at the first empty word (so trailing white space goes and leading white space stays as a blank), and an empty label gives an empty `tspan`. Text nodes are not touched: their runs of spaces stay.
- **Set ids decide the layout through `Object.keys`.** Integer-like keys come first in ascending order, the others in insertion order, and every object of the library is keyed by set id. Sets `"2"`, `"10"`, `"1"` therefore lay out like `"1"`, `"2"`, `"10"`, and `x2`, `x10`, `x1` differ. The model keeps statement order; the layout applies `JsObject`'s order.
- **The random branch.** The constrained layout runs only with eight or more areas after `addMissingAreas`: four or more sets always, three with duplicated statements. It draws `2n` numbers per restart, ten restarts, so a render draws `40n` numbers (chart, then layout). The result differs for most draws when the constrained layout wins, so a port matches only with the same generator and draw order. No fixture of the e2e corpus reaches it; the probes do.
- **The simplex tolerance.** The library reads `minErrorDelta` for its `minTolerance` too, so the text-center search stops at 1e-10, not 1e-5. `venn()` itself uses the defaults (1e-6, 1e-5).
- **Zero-size sets.** The chart removes a zero-size single set and every area naming it; `layout()` does not, so a text node under `A:0` is drawn at the center of a zero-radius circle. The palette index shifts with the removal, because it counts the circles drawn.
- **A disjoint union** gets its text center at (0, -1000) with `disjoint` set, and its path is `M 0 0`: the label is off the canvas.
- **Invalid input.** A negative size gives `NaN` coordinates and no error; a bare `venn-beta` fails in the layout; an unreadable color fails in `khroma`. The renderer reproduces all three; none is a parse error, which is why the model keeps negative sizes.
- **The style sheet beats the attributes.** The title is drawn at 32px whatever its `font-size` attribute says, and the circle labels at 48px unless their inline style sets a size.
- **The library's path string starts with a line feed** and has line feeds between the commands; the rendered `d` keeps those between commands and drops the first.
- **Venn's own defaults.** The `venn` section defaults `theme` to `redux-color` and `look` to `neo`, and these reach the top level for a venn diagram only. `useMaxWidth`, `width`, `height` and `padding` are read from the venn section alone; a top-level value does not reach the renderer. The `classic` and `neo` looks give identical output.
- **Themes without `venn1` to `venn8`** give flat circles in `primaryColor`; the two shipped themes set the eight slots from their border color array.
- **A text node needs its area in the layout.** `text A,C` with no `union A,C` is stored by the DB and dropped by the renderer.
