# Swimlane diagram

A swimlane diagram is a flowchart whose top-level subgraphs are lanes, drawn as bands with the nodes inside and the edges routed at right angles. It starts with `swimlane-beta` and an optional direction. ferrigura parses it with the [flowchart](./flowchart.md) parser, keeps it in the flowchart's model, and draws it with the unified renderer; the layout is Mermaid's own swimlane engine, ported as `ferrigura-swimlane`, and not ELK.

## Model

The model is the flowchart's, field for field: `SwimlanesDiagram` in `crates/ferrigura-model/src/swimlanes.rs` is an alias of `FlowchartDiagram`. The fields, the DB quirks it keeps and the text rules are on the [flowchart page](./flowchart.md#model), and [canonical-model.md](../canonical-model.md) holds the conventions. What the swimlane adds:

- **A lane is a subgraph.** The parser has no lane concept. The lane's title is the subgraph's title, a nested lane closes before its parent and is listed by it, a repeated id merges into its first position, and a lane's members list a chain's later nodes first (`A1[Start] --> A2(Next)` gives `[a2, a1]`).
- **A node outside every lane is a vertex in no subgraph.** The default lane that the layout makes for such nodes belongs to the renderer, so the model has none.
- **Detection comes first.** The exporter (`export-swimlanes.ts`) runs the reference's `detectType` on the cleaned text, as `Diagram.fromText` does, and rejects a text that no swimlane detector takes; the Rust side checks the same in `examples/canonical.rs`.
- **Left out:** the configuration and the layout inputs (`getData`, the sizes and the default lane of the lanes), which are the renderer's.

## Parsing

`crates/ferrigura-syntax/src/swimlanes.rs` is the flowchart parser reached with this header: the reference's swimlane diagram is the flowchart's factory with its own styles, and `swimlane-beta` is one more graph word in `flow.jison`. Every flowchart construct works in a lane diagram, and the quirks of the [flowchart grammar](./flowchart.md#parsing) hold.

- **The detector decides the type, not the lexer.** The detector is `/^\s*swimlane-beta\b/`: `swimlane-beta-x` is a swimlane diagram, and its `-x` is then a lexical error; `swimlane-betax`, `swimlane-beta_x`, `swimlane-beta1`, `Swimlane-beta` and `swimlane` are not swimlane diagrams. The lexer's rule for the word has no word boundary, which is why the detector guards it.
- **The header word is a graph word.** The direction after it follows the flowchart's rules: `LR`, `RL`, `TB`, `BT`, `TD` (stored as `TB`), `BR` (kept as written), `<`, `>`, `^` and `v`, and `TB` when none is given. `swimlane-beta TD;A-->B;` on one line is accepted.
- **Rejected on both sides:** a `subgraph` with no id, an unclosed lane, a stray `end`, a link with no end, an unknown shape, a `linkStyle` before the first edge, a trailing `%%` comment after an edge on the same line, and more edges than `maxEdges` (500).

## Layout and rendering

### The renderer

`crates/ferrigura-render/src/swimlanes/` and `unified/swimlane_layout.rs`, `unified/swimlane_lane.rs` port the reference's `flowRenderer-v3-unified.ts` with `layout-algorithms/swimlanes/index.ts`, `helpers.ts`, `edgeLabelNodes.ts`, `clusters/swimlane.js` and the swimlane branch of `insertEdge`. The steps are the [flowchart's](./flowchart.md#the-pipeline) with the swimlane engine in place of ELK:

1. `getData` is the flowchart's, and the layout data takes the diagram type `swimlane`.
2. **`prepare`** runs before the measure. `prepareLayoutForSwimlanes`: every top-level subgraph becomes a lane (shape `swimlane`, the diagram's direction), and the nodes outside every lane go into `__swimlane_default__`, a lane made when there is none, with padding 20 and the color slot after the largest in use. `createEdgeLabelNodes`: each labeled edge gets a label node `edge-label-<start>-<end>-<id>` of shape `labelRect`, placed in the lane of its source, or of its target when the edge crosses lanes; the edge loses its label.
3. **`pipeline::measure_for`** measures nodes, label nodes and lane titles. A lane's title is measured wrapped, whatever its kind, where ELK measures a plain subgraph title unwrapped.
4. **`swimlane_layout::lay_out`** converts the layout data to the engine's model, calls `run_swimlane_layout_core`, and writes the result back: the positions and sizes of nodes and lanes, the lanes' `swimlaneContentTop`, and the points of the edges. The engine's virtual edges, which carry each label node between the ends of its edge, are laid out and never drawn.
5. **`pipeline::paint`** draws the lanes first, then clips each edge. The edge's ends are clipped to the outlines of their nodes or lanes with a tolerance of 0.5 on both axes (`swimlane_points`), and the line hops come from `swimlane.lineHops`: an arc by default, none when `false`, and `gap` draws no arc.
6. `swimlanes::render` writes the root `svg` with the markers of the type `swimlane`, the flowchart's style sheet with the lane rules appended, the title and the viewBox, as the flowchart does.

The **lane** (`swimlane_lane.rs`) is two rectangles, a title band and a body, with the label in the band. In `TB` the band is above the body and the label is centered in it; in `LR` the band is a strip on the left, `bbox.height + 8` wide, and the label is turned by -90 degrees. A lane takes its own width from its title when the title and the padding need more room than the layout gave. The lanes are painted in the reverse of their source order, because `getData` lists the subgraphs in reverse closing order.

**The engine is chosen by `layout`.** The `swimlane` section's default, `layout: swimlane`, selects the swimlane engine. `elk`, and any name that is not registered (`elk.layered`, `dagre`), lays the diagram out with ELK as a flowchart, with no lanes. A registered layout that ferrigura does not implement (`cose-bilkent`, `elk.stress`, `elk.force`, `elk.mrtree`, `elk.sporeOverlap`, `elk.box`, `elk.rectpacking`) is an error.

**The style sheet** (`swimlanes/style.rs`) is the flowchart's plus `.swimlane.cluster:not([data-color-id]) rect { stroke: clusterBorder !important }`, the `neo` filter rule for clusters, and the palette rules of the lanes. A lane is two rectangles, so each palette slot paints both halves, and no palette rule is `!important`, so a user's inline style still wins. The flowchart's own palette rules leave lanes out (`:not(.swimlane)`).

### The engine

`ferrigura-swimlane` ports `packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/` on the default path, with no `unsafe` code and MIT-licensed as its source ([0012](../decisions/0012-layout-engines-ported-exactly.md), [0013](../decisions/0013-licenses-of-ported-code.md)). Its seam is `run_swimlane_layout_core(&mut LayoutData) -> Result<Direction, Error>` (`layoutCore.ts`), which takes the layout data after the preparation and the measure, as the reference's own tests do.

| Rust module                                                                                                                       | Reference                                                       | Role                                                                                         |
| --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `layout_core.rs`                                                                                                                  | `layoutCore.ts`                                                 | The seam                                                                                     |
| `model.rs`                                                                                                                        | `rendering-util/types.ts` (the fields the layout reads, writes) | `LayoutData`, `Node`, `Edge`, `Config`; other own properties are kept, in order, in `extra`  |
| `helpers.rs`                                                                                                                      | `helpers.ts`                                                    | `toGraphView`, `writeBackToLayoutData`, the lane width solver, the top lane title rectangles |
| `phase0.rs`, `phase1.rs`, `phase2_layering.rs`, `phase2_dummies.rs`, `phase2_options.rs`, `phase3.rs`, `phase4.rs`, `pipeline.rs` | `phase0.helpers.ts` to `phase4.coordinates.ts`, `pipeline.ts`   | The Sugiyama phases                                                                          |
| `router/`                                                                                                                         | `orthogonalRouter/router.ts`                                    | The orthogonal router                                                                        |
| `direction/`, `post_processing.rs`                                                                                                | `direction/*.ts`, `postProcessing.ts`                           | The geometry passes and their driver                                                         |
| `js.rs`, `math.rs`                                                                                                                | (none: JavaScript semantics)                                    | `toFixed`, `Math.round`, `localeCompare`, `Array.prototype.sort`, `Math.max`, `Math.min`     |

`toGraphView` lists the lanes in reverse and then the other nodes; the edges exclude the labeled originals and keep the virtual ones. The core reads `flowchart.nodeSpacing` and `rankSpacing` (50 and 50 from the schema; its fallbacks 40 and 100 are not reached in a normal configuration) and the `swimlane` section, runs the phases, writes the result back, deletes the edge points, routes the edges, sets the curve to `rounded` where it is absent or `basis`, and post-processes. A direction other than `LR`, `BT` and `RL`, such as the `BR` that the flowchart parser keeps, lays out as `TB`, as the reference's comparisons with the four names make it.

**The phases**, in the order of `pipeline.ts`:

- **Cycle removal** (`removeCycles_DFS`): depth-first from the nodes with no incoming edge in declaration order, then the rest; a back edge is reversed and keeps its `id`.
- **Layering** (`assignLayers_LaneAwareCompact`): nodes in topological order (generation by generation for `LR`, Kahn's queue otherwise); a node's rank is the largest rank of a predecessor plus 1 in the same lane and plus 0 across lanes, and at least the lane's next free rank. Groups get no rank.
- **Proper layering** (`makeProperLayering`): an edge that spans more than one rank gets dummy nodes `placeholder-<n>` on the ranks between.
- **Ordering** (`orderLayers`): three rounds of a down and an up sweep, each layer reordered by the median of its neighbors inside each lane, then adjacent swaps that strictly lower the crossings.
- **Coordinates** (`assignCoordinates`): the layer gap is `rankSpacing`, the node gap `nodeSpacing`, the lane gap twice that; each lane's column is the widest layer's total, the lanes sit left to right centered on 0, a node is centered in its column. A lane's horizontal padding is `2 * max(20, padding)`, its title band 21 px high, and the lane widths are solved from the centers.
- **Routing** (`routeEdgesOrthogonal`): a straight fast path when the anchors align, else two L-shaped candidates, else A* over the pipes' intersections; then port assignment per node side, anchors 20 px out of each port, a crossing reduction loop, and the rebuild of the points. Constants: `EPS` 1e-6, node padding 8, pipe margins 15, routing margin 25, anchor offset 20, track spacing 10.
- **Post-processing** (`postProcessSwimlaneLayout`): the `LR`, `RL` or `BT` transform (the engine works in `TB` and turns the result), orthogonalizing and simplifying every polyline, detour simplification, sibling and port-swap passes, label anchoring, endpoint clipping and the terminal stubs, shared-track nudging (twice), the materialized passes (terminal lanes, doglegs, obstacle-hugging rails, destination tails, the crossings, the rails' channels, redundant jogs), a second finalization, and the lane title bands lifted above and shifted left of the rails (each twice).

**JavaScript semantics.** Each expression keeps the operand order of its source, the sort comparisons and the insertion orders (`indexmap` where the reference uses `Map`, `Set` or object key order), and the output agrees with the reference bit for bit.

- `to_fixed` is `Number.prototype.toFixed` by the specification, deciding on the exact binary value and rounding ties up in magnitude. The router uses its results as keys.
- `js_round` is `Math.round`, half toward `+Infinity`, with V8's sign of zero.
- `locale_compare` is `localeCompare` for printable ASCII, as ICU's root collation at tertiary strength, written as a table of the 95 characters in ICU order.
- `js_sort_by` is `Array.prototype.sort` as Chromium runs it (see Learnings).
- `Math.hypot` is `ferrigura_jsmath::hypot`, V8's formula.

## Differences from Mermaid

- **Only the default path of the engine is ported.** `ignoreCrossLaneEdges: false` (the gravity layering, the multitree order and the crossing counts) and `automaticLaneOrdering: true` are refused with an error, and so is a diagram whose ids are not printable ASCII (see Known gaps).
- **Labels are SVG text** ([0009](../decisions/0009-svg-text-labels.md)). The reference's lane labels read `flowchart.htmlLabels`, which is unset and so draws HTML, which is why the harness forces it off.
- **`look: handDrawn`** draws the lanes with the classic rectangles, not with rough.js.
- **Layout names.** A registered layout that is not ported is an error here, where a mindmap falls back to another engine.
- **Themes** are `redux-color` and `redux-dark-color` ([0011](../decisions/0011-themes-as-css-variables.md)); the `swimlane` section defaults to `redux-color`, the `neo` look and `layout: swimlane`.

## Known gaps

- **The non-default options** `ignoreCrossLaneEdges: false` and `automaticLaneOrdering: true` are not ported (about 1,270 lines of the reference: the gravity layering, the driving tree, the multitree order, the crossing counts and the lane ordering). The fixtures that set them are excluded from the geometry run.
- **Non-ASCII ids.** The layout collates with a table for printable ASCII. A node or edge id outside it (`Error::NonAsciiId`, `Error::NotPrintableAscii`) fails the render until an ICU collator is ported.
- **The sort has two gaps.** `js_sort_by` follows Chromium exactly from 17 to 129 elements. For 2 to 16 elements Chromium's comparisons depend on the JIT state of the calling code, and the port takes a first call's; from 130 elements the port keeps V8 13.6's TimSort. Only the comparator calls and the order under an inconsistent comparator differ, so a consistent comparator gives the same order, and the corpus reaches neither gap. Two ignored tests in `tests/js_sort.rs` show the gaps.
- **`validateSwimlanesLayout`** is not ported: it only logs.
- **The corpus cannot be regenerated from the repository.** The captures of the layout seam were made from the reference with stage reports that the pinned reference does not write. The generator of the seeded diagrams (`tools/port/crates/swimlane/generate-swimlanes.ts`) and the sort and JavaScript-semantics data tools are here.
- **Point objects.** The reference shares point objects in places and the port holds points as values. By identity over thousands of layouts, no point object or array is shared between edges and the corpus agrees, but this is evidence and not a proof.
- **A cycle of parents** in the lanes would recurse without end in the reference; the port visits each node once. A flowchart cannot produce one.
- **`flowchart.inheritDir`** is applied when the layout data is made, as in the flowchart; no swimlane fixture sets it.

## Checking it

- `pnpm -C tools/port port:diff swimlanes` compares the canonical models on every input: the fixtures, the string literals of the reference's specs, its documentation blocks, the probes and the generated cases of `cases.ts`. A text must first pass the swimlane detector. `port:diff config` covers the `swimlane` scope.
- `pnpm -C tools/port port:geometry swimlanes` renders the fixtures and the probe under `tools/port/geometry/probes/swimlanes/` in Chromium with both and compares them to 0.05 px. Both sides get the same clock and seed, because the reference draws random offsets even for shapes with roughness 0, and so two renders of one text differ. The run excludes `look: handDrawn` and the two fixtures with a non-default option, each with its reason. `p01-direction-br.mmd` checks the `BR` direction.
- The engine's corpus under `crates/ferrigura-swimlane/tests/corpus` holds the input, the stages (`sugiyama`, `writeback`, `routed`, and a `post:` stage for each pass that changed something) and the output of captured calls of the seam: the fixtures, the layout test inputs of `e2e/platform/dev-diagrams/layout-tests/swimlanes/` with their captured sizes, and seeded generated diagrams. `tests/corpus.rs` compares every output exactly, `f64` with `==`, and the stage tests (`corpus_router.rs`, `corpus_geometry.rs`, `corpus_materialized.rs`, `corpus_post.rs`) run each pass on the state before its stage. `corpus_manifest.rs` checks the manifest against the files, and `hand_made.rs` has small layouts worked out by hand.
- `tests/js_semantics.rs` and `tests/js_sort.rs` check the JavaScript helpers against data recorded from V8 and Chromium (`tools/port/crates/swimlane/js-semantics.ts` and `js-sort.ts` write them).
- The Rust specs: `crates/ferrigura-syntax/tests/swimlanes_spec.rs` (the layout resolution, the detection and the typed view of the section), `swimlanes_parser_spec.rs`, `crates/ferrigura-render/tests/swimlanes_render_spec.rs` and `swimlanes_lane_palette.rs`.
- `cargo test -p ferrigura-swimlane -- --ignored` runs the two sort gaps, which fail by design; the whole-chain corpus tests take about a minute each in a debug build.

## Learnings

- **There is no ELK here.** A swimlane diagram with the default configuration makes no ELK call, and the same text with `layout: elk` makes one. The layout, the router and about 9,000 lines of geometry passes are the reference's own, in TypeScript.
- **The reference is not repeatable unless `Math.random` is seeded.** rough.js draws random offsets through `Math.random` even at roughness 0 (the stadium, the notched pentagon, the diamond), so many fixtures give a different SVG on every render. With a fixed clock and seed, every fixture repeats exactly, in any locale. The layout itself uses no randomness, except the seeded shuffles of `automaticLaneOrdering` (FNV-1a and mulberry32).
- **`localeCompare` decides the order of nodes and edges.** It is called at many places (topological order, cycle removal, layering, ordering, dummies), and ICU's order differs from code-point order for mixed case and punctuation: `_x, 10, 9, a, A, a_b, a-b, ab, b, B`, against `10, 9, A, B, _x, a, a-b, a_b, ab, b`. The order is the same in Chromium and Node's V8 for printable ASCII, and the locale (en-US, de-DE, tr-TR, sv-SE) changes nothing for the fixtures.
- **`toFixed` rounds exact ties up.** `(0.25).toFixed(1)` is `"0.3"` and `(2.5).toFixed(0)` is `"3"`; Rust's `format!` rounds ties to even. The router's A* keys (`x.toFixed(1)`) and several pass ids use it, and a key that collides differently changes the route.
- **`Math.hypot` is not `sqrt(x*x + y*y)`.** The two differ in the last bit for about 38% of random pairs. A*'s f-cost and the length sums use it, so the port uses `ferrigura_jsmath::hypot`.
- **`Math.round` rounds half toward `+Infinity`.** `Math.round(-2.5)` is `-2`; `f64::round` gives `-3`. The default path needs it for the dummies' `x`.
- **`Array.prototype.sort` differs between engines.** Chromium's TimSort differs from V8 13.6's in four places even for consistent comparators: fewer than 16 elements use a binary insertion sort with no run detection; from 64 elements the minimum run length is the length shifted right until below 64; the merge invariant is `run[n-2] >= run[n-1] + run[n]` with a tie merging the lower pair; a merge with a one-element run is a binary search and a rotation. The sort is stable, so a consistent comparator gives one order from every stable sort; the threshold comparators of the router and NaN keys are the inconsistent ones.
- **Keep the up-sweep's odd call.** `transposeImprove` on the up sweep is called with the upper and the next layer swapped relative to the down sweep. In the default layering every edge goes to an equal or larger rank, so the swap can never be accepted; "fixing" the call changes the orders.
- **The lane order in the SVG is the reverse of the source.** `getData` lists the subgraphs in reverse closing order, and `toGraphView` and `buildTopLaneOrder` reverse the lanes again for the layout, so the paint order is reversed and the layout order is not.
- **Direction `BR` lays out as `TB`.** The flowchart parser keeps `BR` as written, and every pass compares the direction with `LR`, `BT` and `RL` only.
- **A label node is not a label.** A labeled edge is drawn as a `labelRect` node placed by the layout, and its text starts at the node's origin and is shifted by half its width, because the swimlane's style sheet does not center it.
- **Lane labels read `flowchart.htmlLabels`.** It is unset in the reference, so lane labels are HTML unless the harness forces `flowchart.htmlLabels: false`.
- **The layout's text constant.** The engine assumes a title band of 21 px for a single-line lane title (`TOP_LANE_TITLE_BAND_HEIGHT`), and a minimum lane padding of 20.
