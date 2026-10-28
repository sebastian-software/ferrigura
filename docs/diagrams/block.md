# Block diagram

A block diagram draws blocks on a grid: a column count, a span per block, empty cells, nested composites and edges between blocks. It starts with `block` or `block-beta`. ferrigura parses it with a hand-written scanner and recursive-descent parser in `ferrigura-syntax` (`src/block/`) and draws it with a renderer of the type's own in `ferrigura-render` (`src/block/`). No graph engine lays it out: `layout.ts`'s arithmetic (rows, spans, padding) is ported as it is. The renderer builds on the unified renderer's shapes, markers, edge paths and labels, and adds two shapes of its own, the composite and the block arrow. The shared machinery is in [flowchart.md](./flowchart.md); this page covers what is specific to block diagrams.

## Model

`ferrigura_model::block::BlockDiagram` is the Diagram DB after parsing (`blockDB.ts`), with camelCase keys. A field the reference leaves `undefined` is absent from the JSON.

- **`columns`** is the columns of the entry registered as `root`: `-1` when the text gives none or `auto`. It is absent only when a class list names a block `root` with blanks around it, which replaces that entry.
- **`blocks`** are the root's children in order, composites with their `children`. A block has `id`, `type` (`na` for a plain id, `square`, `round`, `circle`, `doublecircle`, `diamond`, `hexagon`, `stadium`, `subroutine`, `cylinder`, `lean_right`, `lean_left`, `trapezoid`, `inv_trapezoid`, `rect_left_inv_arrow`, `block_arrow`, `composite` or `space`), `label` (sanitized, with the entity placeholders the DB keeps), `width` (on space cells), `widthInColumns` (the span), `columns` (of a composite), `directions` (of a block arrow, as written), `classes`, `styles`, `colorIndex` (a composite's palette slot) and `children`.
- **`edges`**, in declaration order: `id` (the count of its key, then `-`, then `start-end`, so `1-A-B` and `2-A-B`), `start`, `end`, `label`, `thickness` (`normal` or `thick`), `pattern` (`solid` or `dotted`), `directions`, `arrowTypeEnd` and `arrowTypeStart`.
- **`classDefs`:** `id`, `styles` and `textStyles`.

An edge's `type` is always `edge` and is left out. The model leaves out the layout, the palette slots beyond `colorIndex` and the shapes (renderer concerns). The front matter title is read and dropped, because the block DB has no title setter. Ids that the reference draws at random, those of space cells and anonymous composites, are the port's deterministic `id-<twelve digits>-<n>`; the differential replaces them on both sides by `$genK` placeholders in tree order, and a space cell's label (its generated id) takes the same placeholder. A user's node id cannot contain `-`, so every id with a `-` is a generated one. The one `undefined` that is written is the first block of a chained link, an empty object (see Parsing). The conventions common to all models are in [canonical-model.md](../canonical-model.md).

## Parsing

The text goes through the shared preparation (front matter, directives), comment-line removal and `encodeEntities`, and gets one line break at the end, as `Diagram.fromText` does.

**Scanner** (`block/scanner.rs`). The rules of `block.jison` in their written order on a state stack ([0007](../decisions/0007-hand-written-parsers.md)): first match wins. INITIAL holds `block-beta`, `block:` and `block`, whitespace, `columns`, quote openers, `space`, the style, class and accessibility keywords, `end`, the shape openers in the order of the grammar, `<[`, node ids, the links and `:` with a span. The other states (`string`, `md_string`, `NODE`, `BLOCK_ARROW`, `ARROW_DIR`, `LINK_LABEL`, `classDef`, `class`, `style`, the accessibility states) have the rules the grammar's `%x` states give. A keyword that ends in a word character gets the implicit word boundary, so `blockA`, `spaceship`, `defaults`, `columnsX`, `endpoint`, `classy` and `stylish` are node ids; a keyword followed by `\s+` needs its blank. Rules that match an empty text (`[^\n]*`, `[^"]*`, `[^\}]*`) return a token even when empty, and at the end of the input the scanner behaves as Jison's `done` flag does: the first call with an empty input reads the rules once, and every later call returns the raw end symbol, which the grammar does not accept.

**Grammar** (`block/grammar.rs`). Recursive descent over the tokens: a document of statements (node statements with an optional span and a chain of links, `columns`, `space`, `block … end` composites, `classDef`, `class`, `style`). Each action gives the objects the reference's does; a link gives three items `[left, edge, right]`, where the left block keeps id, label, type and directions but not its span.

**DB** (`block/db.rs`). The reference keeps one object per block, and the DB's map and the tree hold the same objects, so a later mention can change an object that the tree does not show. The port keeps the objects in an arena, and the map and the tree hold indices, which keeps those identities. Populating follows `populateBlockDatabase` item by item: labels are sanitized when non-empty (a label that sanitizes to empty takes the id); `classDef`, `class` and `style` apply their styles; `columns` sets the parent's columns; an edge joins the edge list; any other block takes the default label, keeps its first position when its id was seen, takes the new type unless it is `na`, and takes the new label when it differs from the id. A new composite takes the next palette index before its children, so outer composites come first, and a space with span `w` becomes `w` cells. `parse_with_warnings` returns the reference's `log.warn` of a block wider than the first `columns` of its list.

Quirks of the reference that are kept on purpose:

- A diagram with no statement is rejected, and so is a header `block:x` at the start, although the detector accepts it.
- `A --> B --> C` gives a first block with no id, label or type and the edge `1-undefined-C` with no start; a second chain's first block is dropped, because the map already holds the `undefined` key. `A:2 --> B` drops the span of `A`, and `A --> B:2` is rejected: a span follows a standalone node only.
- The first rule wins, so ids may contain `"`, `]`, `.`, `~`, `o` and `x`: `A o--o B` has the node `o`, `A ~~~ B` has the node `~~~`, and `x-y` is rejected.
- Edges: `---` has no end arrow, `====` and `==>` are thick, `-.`, `-.-` and `-.->` are dotted, `<-->` has a point at each end, and a link with no leading mark starts with `arrow_open`.
- A `style` for an id that no block has yet is an error; `style A,B …` is an error too (ids are not split), while `style A` with no definition is not.
- `class` before the declaration creates a DB entry outside the tree, so the later node is not drawn. `class A, B` with a blank before `B` looks up ` B` untrimmed and replaces an existing `B` entry. `class A` with no class name applies the empty class.
- `classDef x` followed by a line takes the style options from the next line, since the blank after the name takes the line break. `classDef DEFAULT` is rejected, `classDef default` names a class, and `classDef my-class` is a lexical error.
- Class styles lose the first `;` of each style; a style that mentions `color` also gives a text style, with the first `fill` made `bgFill` and then the first `color` made `fill`. The test is unanchored, so `background-color` gives `background-fill`.
- `root` is registered from the start: a node or a class named `root` is absorbed by the root entry.
- `columns 0` is kept in the model (the renderer rejects it), `columns auto` is `-1`, `columns 00007` is `7`, and `columns 3x` is `3` with the id `x`.
- `space:0` gives no cell, `space` is one cell, and a cell's id is `<id>-<j>`; the space itself is not in the tree.
- Block arrow directions and styles keep their blanks as written (`( right , down )` gives `[" right ", "down "]`); the renderer expands `x` and `y`.
- `accDescr {}` is accepted and ignored. A non-empty accessibility block, `accTitle:` and `accDescr:` with a value, `linkStyle`, `interpolate`, a bare `default`, markdown strings, the empty label `A[""]`, unquoted label text (`A[x]`) and a shape without a label (`A()`) are rejected: no production uses them. Labels are quoted, with no escapes. A link label must be quoted and non-empty, and a block arrow's direction must be `right`, `left`, `up`, `down`, `x` or `y`.

## Layout and rendering

`block/mod.rs` is `blockRenderer.ts`. The steps follow the reference: every block is measured with nothing wrapping (`calculateBlockSizes`), `layout.rs` places the blocks, each block is drawn at its final size where its label wraps (`insertBlocks`), the edges join the drawn blocks (`insertEdges`), and the root takes the box the drawing covers. Every size is the float32 `getBBox` of the drawn group, as the browser gives it.

**Layout** (`block/layout.rs`, `layout.ts`). Sizes come bottom up (`setBlockSizes`): a parent's children share the largest per-column width and the largest height among the non-space children, so a child of span `s` is `maxWidth·s + padding·(s−1)` wide; the grid is `columns` wide (or one row of `n` children when `columns` is absent or larger), and the parent is widened to its assigned width or its sibling's size when that is larger. Positions come top down (`layoutBlocks`): each child's row and column follow its span and the parent's `columns`, a row is as high as its tallest child, and a child is centered at its column's start plus `padding`. The padding is `block.padding` (8), read at the call. A `columns` of 0 is an error, as in the reference.

**Nodes and edges** (`block/nodes.rs`, `renderHelpers.ts`). `getNodeFromBlock` maps a block type to a unified shape (`na` and `square` to `rect`, `round` to a rounded rect, `diamond` to `question`, and so on); a composite and a block arrow use the shapes below. The measure pass gives a node no width, so its label does not wrap; the placed pass gives the final width, so the label wraps at the width the shape leaves for it. An edge is drawn only when both ends have a size, with the points `[start center, midpoint, end center]` clipped by the shapes' own intersect functions; its id is `<diagram id>-<edge id>`, its class list carries the thickness and pattern classes and `flowchart-link`, and its label is a Markdown label at the midpoint.

**Shapes of its own** (`unified/shapes/composite.rs`, `unified/shapes/block_arrow.rs`). A composite is a `rect.basic.cluster.composite.label-container` with an empty label, drawn as a sibling of its children (every block is a child of `g.block`, in pre-order), so the children paint over it; in a color theme it gets `data-color-id` from its `colorIndex`. A block arrow is a polygon from a table of sixteen point sets keyed by its expanded directions (`x` is right and left, `y` is up and down), in the order right, left, up, down; an unknown key or no direction gives the single point `(0, 0)`.

**Style sheet** (`block/style.rs`, `styles.ts`). Colors are custom properties on the diagram id ([0011](../decisions/0011-themes-as-css-variables.md)); the `classDef` rules come from `createCssStyles`, and the palette rules are one per slot, on composites only. The composite's fill and stroke are the one place with a computed `rgba`, since a custom property cannot carry the alpha of `fade`.

**Root.** The size is `bounds.height + magicFactor + 10` by `bounds.width + 10`, with `magicFactor = max(1, round(0.125·width/height))`; the viewBox is the box padded by 5 and does not include the magic factor. With `useMaxWidth` (the default) the root has `width="100%"` and `max-width`; without it, `height` and `width` attributes.

## Differences from Mermaid

- **Labels are SVG text,** also with `htmlLabels: true`, which is the reference's production default ([0009](../decisions/0009-svg-text-labels.md)).
- **Generated ids are deterministic** (see Model).
- **A drawing of only spaces** with `useMaxWidth: false` gets `height="NaN"`, as in the reference; the attribute is written as it writes it, a named exception to the reading of the root.

## Known gaps

- A site configuration's `dompurifyConfig` is not read by the sanitizer, which runs DOMPurify's default configuration once where the reference's `sanitizeMore` runs `removeScript` first; the one spec that tests it is ignored.

## Checking it

- `pnpm -C tools/port port:diff block` compares the canonical models on the fixtures, the string literals of the specs and the Markdown examples, and the explicit cases of `cases.ts` (one per quirk above); generated ids are replaced by placeholders (`normalizeGenerated` in `export-block.ts`, `normalize_block_ids` in the `canonical` example).
- `pnpm -C tools/port port:geometry block` renders every fixture with both in Chromium and compares geometry. There is no layout-data run: the layout is the type's own arithmetic, checked by the geometry run.
- Rust: `block_spec.rs` and `block_db_spec.rs` in `ferrigura-syntax/tests`; `block_layout_spec.rs`, `block_color_index_spec.rs` and `block_render_spec.rs` in `ferrigura-render/tests`. The last holds what the geometry run does not state: the `NaN` height of a drawing of only spaces, the zero-column error, the repeated classes of a composite, the first block of a chained link with no id and no edge, and the palette slots of a composite. The sixteen point tables of `blockArrow.spec.ts` are in the unit tests of `block_arrow.rs`.

## Learnings

- **Node labels do not wrap at 200.** In the measure pass a node has no width, so `wrappingWidth` is `Infinity` and a 680 px label stays on one row; only edge labels take `createText`'s default of 200. The reference's e2e test that says "200 px wrap threshold" checks overlap and its premise is wrong.
- **A hexagon label wraps on a float tie.** In two equal polygons `Opening tag` wraps and `Closing tag` does not, because the wrap test is `width <= limit` and the shape changes the node's width first. The shapes that read the node's own width or height (hexagon, lean and trapezoid shapes, double circle, cylinder) take it from the block's size; the flowchart never sets one.
- **A composite's class is `node default flowchart-label default flowchart-label`.** The classes are repeated, and the structural key of the geometry run counts them.
- **Every block edge has `style="undefined;;;undefined"`** and a doubled path id prefix (`b10-b10-1-A-E`): `edge.style` is undefined, so `edgeStyles` is `[undefined]`.
- **The root's `height` has a factor the viewBox lacks.** With `useMaxWidth: false` the `height` attribute is 117 where the viewBox says 116. The factor is JavaScript's `Math.round`, written `floor(x + 0.5)` here, and it is NaN for a drawing with no height, which `Math.max` keeps (Rust's `f64::max` would drop it).
- **Edge endpoints move back 4 px under a point marker,** and the circle and cross markers have no offset in `markerOffsets`, although the dash arithmetic of `markerOffsets2` has one.
- **Block edges have no color,** so no colored marker copies are made, and `arrow_open` (every edge without a leading mark) has no marker.
- **The measure and the placed pass differ,** so the same label can have two sizes; a shape that changes `node.width` first (the hexagon) changes the wrap limit of the second pass.
- **`styles2Map` splits on `:`,** so a style value with a second colon loses everything after it, and the node's `style` is merged with its classes' styles with `!important` appended.
