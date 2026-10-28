# Mindmap

A mindmap draws a tree of nodes from an indented outline, with seven shapes, icons and classes. It starts with `mindmap`. ferrigura parses it with a hand-written scanner and recursive-descent parser in `ferrigura-syntax` (`src/mindmap/`) and draws it with a renderer of the type's own in `ferrigura-render` (`src/mindmap/`) that builds on the unified renderer's shapes, labels and edge paths. The layout is cose-bilkent by default (`ferrigura-cose`), or the tidy tree for `layout: tidy-tree` (`src/unified/tidy_tree.rs`), or ELK Layered for an ELK layout name ([0012](../decisions/0012-layout-engines-ported-exactly.md)). The shared machinery is in [flowchart.md](./flowchart.md); this page covers what is specific to mindmaps.

## Model

`ferrigura_model::mindmap::MindmapDiagram` is the Diagram DB after parsing (`mindmapDb.ts`): `root`, the tree from the first node, or `null` when the text has no node. A node has:

- `id`, the DB's counter: the node's position in the order the parser added the nodes;
- `nodeId` and `descr`, the text before the shape and the text inside it, sanitized, with the entity placeholders the DB keeps (`ﬂ°°35¶ß`), which the renderer decodes;
- `type`, the shape number: `0` default, `1` rounded rectangle, `2` rectangle, `3` circle, `4` cloud, `5` bang, `6` hexagon;
- `level`, the indentation relative to the root, in columns;
- `width` and `padding`, the configuration's values as the reference's JavaScript holds them: `mindmap.maxNodeWidth` (200) and `mindmap.padding` (10), the padding doubled for shapes 1, 2 and 6. A value of another type than a number (a string, a boolean) stays as given, except that the doubling turns it into a number; an array or an object does not replace the default number;
- `icon` and `class`, `""` when absent (the last icon and the last class win, each sanitized);
- `children`.

The model leaves out what `getData` and the layout set: `isRoot`, `section`, `x`, `y`, `height`, `labelType` and the shape names. The conventions common to all models are in [canonical-model.md](../canonical-model.md).

## Parsing

The text goes through the shared preparation (front matter, directives), comment-line removal and `encodeEntities`, and gets one line break at the end, as `Diagram.fromText` does. The configuration is read from the same text before the parse, because the DB reads `mindmap.padding` and `mindmap.maxNodeWidth` when it adds each node.

**Scanner** (`mindmap/scanner.rs`). The rules of `mindmap.jison` in their written order ([0007](../decisions/0007-hand-written-parsers.md)): first match wins and the states are exclusive, so a rule without a state belongs to INITIAL only. INITIAL has the comment (`\s*%%.*`), the keyword `mindmap` (case-insensitive, with a word boundary), `:::` and `::icon(`, blank rows, line breaks, the shape openers (`-)`, `(-`, `))`, `)`, `((`, `{{`, `(`, `[`), the indentation (`[\s]+`, whose length is the level), node ids (`[^\(\[\n\)\{\}]+`) and the end. The states are NODE (a shape's text; `"` and `` "` `` open the quoted forms), NSTR and NSTR2 (between quotes and between a quote and a backtick), ICON and CLASS. When the text is used up inside a shape, an icon or a class, the scanner yields Jison's raw end symbol, which the grammar does not accept.

**Grammar and DB** (`mindmap/mod.rs`, `mindmap/db.rs`). The grammar is read from the tokens by a recursive recognizer, and the DB calls then run in document order: `addNode(level, node)` makes the first node the root (its level is 0, whatever its indentation) and attaches each later node to the nearest earlier node with a smaller indentation; `decorateNode` sets the icon or class of the last node. A node with no parent fails with the reference's own message, "There can be only one root. No parent could be found for …". `getType(start, end)` picks the shape: `[` is a rectangle, `(` … `)` a rounded rectangle, `(` with another end a cloud, `((` a circle, `)` a cloud, `))` a bang, `{{` a hexagon, and anything else (the `(-` and `-)` starts) the default.

Quirks of the reference that are kept on purpose:

- A comment takes the blanks and line breaks before its `%%`. After a shape `%%` starts a comment; after a plain id it is text (`child %% trailing` is the id).
- White space with a line break after its first character is one `SPACELINE`: blank rows collapse, and the indentation of the row after them is its own `SPACELIST`. A lone line break is `NL`.
- `mindmap` is a keyword only as a whole word: `mindmapper` is an id, `MindMap` is the keyword.
- `:::` followed at once by a line break takes the line break with no token, so the next line's indentation follows the statement's, and `root`, `  :::`, `  child` is rejected.
- `::icon()` sets nothing, an icon may span lines, and the last icon wins.
- An id is everything up to a bracket: it keeps spaces, trailing blanks, `:`, `%`, `]` and `:::` (`child:::hot` is one id), and `root-)` has the id `root-`. `}` cannot be in an id.
- `root(-Root-)` is the default shape with the text `Root-`, since `(-` is not in `getType`.
- Descriptions: `"…"` may hold brackets and span lines; ``"`…`"`` keeps the backticks out while `[`…`]` keeps them; `(` ends a description, and a lone `}` takes the rest of the line. Text after a shape (`root(a)b`) and a quote inside a description (`root["a"b]`) are errors.
- The header takes a statement and a statement needs a stop: `mindmap`, a blank row and `root` is rejected, `mindmap` alone is rejected, and `mindmap` followed by one line break is a text with no node.
- An icon or a class before any node is rejected, because the reference's `decorateNode` throws on `undefined`.
- Levels are columns; a tab is one column.
- Entities stay as the DB keeps them. `<`, `>` and `&` are sanitized as the flowchart sanitizes them (`<script>x</script>y` is `y`, `<br/>` is `<br>`, `a < b` is `a &lt; b`); one DOMPurify pass of `sanitize.rs` gives the same result as the reference's two on every checked input.

## Layout and rendering

`mindmap/mod.rs` is `mindmapRenderer.ts`: it builds the layout data with `mindmap/data.rs`, measures every node with its shape, lays the nodes out with the chosen engine, paints (`mindmap/paint.rs`) and writes the root with `setupViewPortForSVG` and `mindmap.padding` as the padding. The root is `<svg class="mindmapDiagram" aria-roledescription="mindmap">`. A tree without a root draws only the placeholder `<svg><g></g></svg>`, with no class, viewBox or style.

**Layout data** (`mindmap/data.rs`, `getData` with `flattenNodes`, `generateEdges` and the shape overrides of `draw`). Nodes are flattened in pre-order. A node's class list is `mindmap-node`, then `section-root section--1` for the root or `section-<n>` otherwise, then its own class. Sections: with `colorBy: branch` (the default) a child of the root gets `index % 11` and its descendants inherit it, with `colorBy: depth` every node gets `(depth − 1) % 11`. Edges are one per child, in pre-order, with the id `edge_<parent>_<child>` and the classes `edge`, `section-edge-<section>` and `edge-depth-<parent level + 1>`; the depth comes from the indentation delta and not from the tree depth. The shapes are `rounded` (the default node and type 1, in the redux themes), `rect`, `mindmapCircle`, `cloud`, `bang` and `hexagon`. `draw` then overrides the records: `rounded` takes width 0 and padding 15, `rect` width 0 and padding 10, `hexagon` width and height 0.

**Shapes and labels.** Labels are Markdown laid out as SVG text. A shape with width 0 wraps its label at `flowchart.wrappingWidth` (120); the circle, cloud and bang keep `mindmap.maxNodeWidth` (200). A rounded node is `bw + 30` by `bh + 30` with the theme's radius; a rectangle takes a label padding of 20 by 10 (16 by 12 in the neo look); a circle has the radius `sqrt(bw² + bh²)/2 + padding`; the cloud and the bang are paths whose boxes are Chromium's bounds of their arcs (`ferrigura-path`); the hexagon is a polygon (`unified/shapes`). An icon is stored and never drawn: no glyph is written, but a node with an icon gets the label background.

**Layout engines** (`mindmap/layout.rs`). The layout is the top-level `layout`, which `getData` replaces by `cose-bilkent` unless the user set `layout` at the top level (in `initialize`, a directive or the front matter). Names that are not registered fall back to cose-bilkent, and so does `swimlane`.

- **cose-bilkent** (`ferrigura-cose`): the engine gives each node's center; each edge gets the three points (`start`, `mid`, `end`) of cytoscape's straight edge for a default 30 by 30 node style, so every endpoint lies on a circle of radius 15 around its node's center, whatever the node's real size. The edge is a `basis` curve through those points, with no clipping: the reference's copies of the nodes that `insertEdge` receives have no `intersect`. Nodes paint over the edges, which hides the overlap.
- **Tidy tree** (`layout: tidy-tree`, `unified/tidy_tree.rs`): `@mermaid-js/layout-tidy-tree` 1.0.1 on `non-layered-tidy-tree-layout` 2.0.2. The root's children alternate between a left and a right tree, each laid out with its boxes turned, and every edge gets four points.
- **ELK** (`elk` and `elk.*` names): the unified pipeline's ELK layout. No fixture sets one.

**Paint** (`mindmap/paint.rs`). The DOM order is the reference's: the four `point` markers (written, though no edge refers to them), the empty `g.subgraphs`, `g.edgePaths`, `g.edgeLabels` (an empty `g.edgeLabel` per edge, with its orphan `rect.background`), and `g.nodes`. An edge path is `basis` after `fixCorners`, its style attribute is `undefined;;;undefined`, and `data-points` is the base64 of the points as JSON.

**Style sheet** (`mindmap/style.rs`, `styles.ts`). Colors are custom properties on the diagram id ([0011](../decisions/0011-themes-as-css-variables.md)): per section `.section-<i>` fill and text from `cScale<i>` and `cScaleLabel<i>`, `.section-edge-<i>` stroke from `cScale<i>`, and `.edge-depth-<i>` stroke widths (17 − 3i classic, `max(10 − 2(i−1), 2)` neo), plus the `.section-root` colors from `git0`. The rules that match no element (`.node-icon-N`, `.section-N line`) are left out.

## Differences from Mermaid

- **Labels are SVG text,** also with `htmlLabels: true`, which is the reference's default for mindmaps ([0009](../decisions/0009-svg-text-labels.md)).
- **Only the redux themes ship,** so the default node is always the rounded rectangle; the classic themes' `defaultMindmapNode` is not drawn ([0011](../decisions/0011-themes-as-css-variables.md)).
- **`look: handDrawn`** takes the classic geometry.
- **`layout: swimlane`** falls back to cose-bilkent, where the reference's registered swimlane layout would receive the mindmap records.
- **ELK for the ELK names.** A mindmap with an ELK layout name is laid out with ELK. The reference's ELK path fails on the mindmap records, which carry `x` and `y` keys with `undefined` values (as the comment in `layout.rs` records), so there is no output to compare, and no fixture sets one.

## Known gaps

- A configuration that puts an array in place of the `mindmap` section is merged element by element by `assignWithDepth` in the reference (`mindmap: [{padding: 3}]` gives padding 3); the configuration layer's merge does not do that.
- The error texts of Jison and of the engine's `TypeError` are not reproduced; the differential compares acceptance, and the DB's own message exactly.
- Icons are not drawn, as in the reference.

## Checking it

- `pnpm -C tools/port port:diff mindmap` compares the canonical models on the fixtures, the string literals of the specs, the docs and the explicit cases of `cases.ts` (the quirks above, the configuration cases for padding and width, and `MindMap` in `undetectedCases`); the reference's configuration steps run first. `pnpm -C tools/port port:diff config` compares the `mindmap` section.
- `pnpm -C tools/port port:geometry mindmap` renders every fixture and the probes under `tools/port/geometry/probes/mindmap` with both in Chromium and compares geometry; the probes cover each shape, `colorBy`, wrapping, Markdown, classes and icons, many sections, the tidy tree, the empty tree, an unregistered layout name and a diagram-scoped layout key.
- `pnpm -C tools/port port:tidy-tree` captures the reference's tidy tree layout (fixtures, hand-picked and seeded random trees) into `crates/ferrigura-render/tests/data/tidy-tree/`; `tidy_tree_reference.rs` compares the port with it, and its ignored test takes a larger run.
- Rust: `mindmap_spec.rs` and `mindmap_colors_spec.rs` in `ferrigura-syntax/tests`, `mindmap_get_data.rs` in `ferrigura-render/tests` (the `getData` and section tests), the unit tests of `layout.rs` (cytoscape's rscratch points on the `ferrigura-cose` corpus, bit for bit), and `cargo test -p ferrigura-cose`.

## Learnings

- **The diagram-scoped layout key is ignored.** `mindmap.layout` and `updateSiteConfig({layout})` resolve into the configuration but not into the user-defined set that `getData` checks, so they fall back to cose-bilkent; only a top-level `layout` (front matter `config.layout`, a directive's `layout`, `initialize({layout})`) counts. The facade reproduces this in `View::resolve`.
- **cose-bilkent is deterministic for every input a mindmap can give it.** A mindmap is always a tree, which takes the radial placement, not the random start: no draws from `RandomSeed` or `Math.random`, so the output does not depend on the page. The engine's `layout-base` is version 1.0.2.
- **The edge endpoints are cytoscape's, not Mermaid's.** They come from the node's style size (30 by 30), so each lies 15 px from its node's center along the line between the two centers; the midpoint is the average of the four points.
- **Wrap widths differ by shape.** Rounded, rectangle and hexagon labels (width zeroed) wrap at `flowchart.wrappingWidth` (120) and ignore `mindmap.maxNodeWidth`; circle, cloud and bang keep 200.
- **The padding is changed twice.** The DB doubles it for shapes 1, 2 and 6 (10 becomes 20); `draw` then sets `rounded` to 15 and `rect` to 10, so only the hexagon keeps 20. The `radius`, `taper` and `stroke` that `draw` sets on `rounded` have no reader.
- **Cloud and bang sizes are path bounds.** The node's box is `getBBox` of an arc path, which Chromium computes by converting arcs to curves and taking extrema; `ferrigura-path` reproduces it bit for bit, and a difference would move every cloud and bang node.
- **The tidy tree has four quirks that are kept:** the first level of each side is centered on the tops of its nodes, which is off by half the nodes' height; the reported `mid` is the first pass's midpoint, not an endpoint of the edge; the round shapes are `circle`, `cloud` and `bang` (`mindmapCircle` is not one, so a `((…))` node is clipped as a rectangle); and an edge between two nodes of one side goes out past the source and back in to the target.
- **The `edge-depth` class uses the indentation delta,** so a child written two levels deeper than its parent has `edge-depth-3` although it is one level down.
- **The empty `rect` and the orphan `rect.background`.** `labelHelper` inserts an empty `rect` first in each label group, and in SVG mode the edge label's background rect stays in the `edgeLabels` group as a sibling; both are in the geometry run's structural keys.
- **`style="undefined;;;undefined"`** appears on every mindmap edge because the edge has no `style` field; the browser ignores it.
