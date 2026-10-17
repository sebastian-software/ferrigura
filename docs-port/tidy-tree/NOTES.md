# Tidy tree layout (`layout: tidy-tree`): notes for the port

Research for the layout engine that mindmaps use with `layout: tidy-tree`. Read only; nothing here is committed. The P2 decision that asks for it is in [decision 0008](../decisions/0008-force-directed-layouts.md) and [P2-PLAN.md](../P2-PLAN.md).

## Reference

- **The package.** `@mermaid-js/layout-tidy-tree` 1.0.1. The fork removed it in commit `2d85261d6`, so its source is read from the parent commit, `2d85261d6^`. The source tree at HEAD (`aa5c4338f`) has no tidy-tree code, and the built entry `packages/mermaid/dist/mermaid.esm.mjs` has no `tidy` string.
- **The library.** `non-layered-tidy-tree-layout` 2.0.2, which the package wraps. This worktree has no copy in `node_modules` (the fork removed the dependency). The copy read here is in the main checkout, at `/Users/sebastian/Workspace/ferraid/node_modules/.pnpm/non-layered-tidy-tree-layout@2.0.2/node_modules/non-layered-tidy-tree-layout/`. Its lock entry is `2d85261d6^:pnpm-lock.yaml:8372`, integrity `sha512-gkXMxRzUH+PB0ax9dUN0yYF0S25BqeAYqhgMaLUFmpXLEk7Fcu8f4emJuOAY0V8kjDICxROIKsTAKsV/v355xw==`.

Citations: `pkg:` is `packages/mermaid-layout-tidy-tree/src/` at `2d85261d6^`. `lib:` is the library directory above, `src/` unless it says otherwise. `mermaid:` is `packages/mermaid/src/` at HEAD. A bare `render.ts` line in sections 2 to 7 means the package's `src/render.ts`.

Marks: **probed** means the reference code was run (section 7 says how). **read** means read in the source and not run. **unverified** means neither.

## 0. Summary

Mermaid's tidy tree is a layout for mindmaps that the host page has to register. The fork's bundle cannot supply it by itself. Without registration, `layout: tidy-tree` becomes `cose-bilkent`, with no message in the page console (probed, section 7.3).

The layout takes each node's measured box, which is its DOM width and height. It splits the root's children into a left and a right tree by index parity, lays each tree out with the library (van der Ploeg's non-layered tidy tree algorithm), places the root at `(0, 20)`, and returns node centres and four-point edge paths. The layout is arithmetic over `id`, `width`, `height`, `shape`, `start` and `end`. It reads no DOM and nothing it draws reaches the output (section 4).

Size: `layout.ts` is 653 lines, about 520 of them code. The parts of the library that Mermaid uses are about 150 lines of `algorithm.js` and 70 lines of `helpers.js`. The package's `render.ts` (180 lines) is renderer glue that the unified renderer replaces. A Rust port is roughly 700 lines of layout code with a similar amount of tests.

Riskiest parts, in order:

1. The contour algorithm with threads and the list of left siblings (`lib:src/algorithm.js:41-151`). It is mutable and pointer-linked, and the library's own tests are three small files. The differential in section 7.2 is the real test.
2. Four quirks of the reference that a port must reproduce to match its output (section 3.6). All four are probed.
3. Node sizes come from text measurement in the DOM, and the layout uses them directly. A small width error moves every node on its side.
4. Edge paths pass through the shared `insertEdge` and `fixCorners` (section 3.7), so the renderer part of the port must use the same path.

## 1. Sources

| File                                                       | Lines                     | What it holds                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pkg:src/layout.ts`                                        | 1-653                     | `executeTidyTreeLayout` (23-72); the dual tree built from the edges (80-138); subtrees and the transposition (144-184); the placement of the two trees and the root (189-292, 298-360); the circle and rectangle clipping (369-448); the edge points (455-626); `validateLayoutData` (635-652, not called by the renderer)                      |
| `pkg:src/render.ts`                                        | 1-180                     | the renderer the loader returns: groups (47-50), node insertion and `getBBox` (54-81), the layout call (93-105), `translate` (109-117), edges (128-174)                                                                                                                                                                                         |
| `pkg:src/layouts.ts`                                       | 1-13                      | the registration object: name `tidy-tree`, `algorithm: 'tidy-tree'`, loader `import('./render.js')`                                                                                                                                                                                                                                             |
| `pkg:src/index.ts`                                         | 1-50                      | the package entry: default export, `render`, re-exports                                                                                                                                                                                                                                                                                         |
| `pkg:src/types.ts`                                         | 1-70                      | `PositionedNode`, `PositionedEdge`, `TidyTreeNode`. `TidyTreeLayoutConfig` (314-317) is unused                                                                                                                                                                                                                                                  |
| `pkg:src/layout.test.ts`                                   | 1-447                     | mocks the library (7-46), so it never checks geometry. Expects the root at `(0, 20)` (287-288, 414-415) and the first intermediate point of a root edge at `±(root width / 2 + 30)` (437-444)                                                                                                                                                   |
| `pkg:README.md`, `pkg:CHANGELOG.md`, `pkg:package.json`    |                           | usage with `registerLayoutLoaders`; 0.2.1 inlined the library; 0.2.2 kept root edges on a rectangular root (`5ab469382`); `package.json` lists `d3` as a dependency, which no source file imports                                                                                                                                               |
| `lib:src/algorithm.js`                                     | 1-208                     | `Tree` (1-22), the contours and threads (24-108), `seperate` (110-151), `positionRoot` (153-163), `firstWalk` (165-181), `addChildSpacing` (183-191), `secondWalk` (193-201), `layout` (203-206)                                                                                                                                                |
| `lib:src/helpers.js`                                       | 1-128                     | `BoundingBox` (3-23; `addBoundingBox` 13-15, `removeBoundingBox` 20-22), `Layout.layout` (36-42), `convert` (47-62), `getSize` (84-101, not used by Mermaid), `assignLayout` (106-125)                                                                                                                                                          |
| `lib:src/index.js`                                         | 1-4                       | exports `layout`, `Tree`, `BoundingBox`, `Layout`                                                                                                                                                                                                                                                                                               |
| `lib:dist/non-layered-tidy-tree-layout.js`                 | one line, 5599 bytes      | the UMD build, minified. Same operations as `src` (read: `seperate`, `updateIYL`, `positionRoot`, `addChildSpacing`). It needs a global `window` to load in Node                                                                                                                                                                                |
| `lib:test/tree-layout.test.js`, `lib:test/helpers.test.js` | 1-62, 1-100               | known answers (section 7.1)                                                                                                                                                                                                                                                                                                                     |
| `mermaid:rendering-util/render.ts`                         | 29-129                    | the registry: `registerLayoutLoaders` (33-37), `getLayoutLoaderDefinition` (45-51), defaults (65-79), `render` (81-103), `getRegisteredLayoutAlgorithm` with its fallback (113-129)                                                                                                                                                             |
| `mermaid:mermaid.ts`                                       | 19, 393, 407              | exposes `mermaid.registerLayoutLoaders`                                                                                                                                                                                                                                                                                                         |
| `mermaid:diagrams/mindmap/mindmapRenderer.ts`              | 33-116                    | chooses the layout (46-49; fallback `cose-bilkent`), presets node widths for some shapes (58-74, overwritten by the DOM measure), calls `render` (77)                                                                                                                                                                                           |
| `mermaid:diagrams/mindmap/mindmapDb.ts`                    | 227-303, 313-349, 357-421 | flattening in pre-order (`flattenNodes`); edges per parent in child order (`generateEdges`, curve `basis` at 336); `getData` (line 366 sets `cose-bilkent` unless the text sets `layout`; line 418 sets `diagramId` from `v4()`); shape mapping (248-270; `mindmapCircle` at 253; `rounded` for the default shape under a redux theme, 264-265) |
| `mermaid:rendering-util/rendering-elements/edges.js`       | 43-45, 301, 550, 624-731  | `resolveEdgeCurveType`; `positionEdgeLabel`; `fixCorners`; `insertEdge`, whose endpoint clipping (699-703) the tidy tree's nodes skip; the `fixCorners` call (729-731)                                                                                                                                                                          |
| `mermaid:rendering-util/rendering-elements/nodes.ts`       | 14-…                      | `insertNode`, returns the node's group element                                                                                                                                                                                                                                                                                                  |
| `mermaid:internals.ts`                                     | 19-31                     | the helpers the layout receives (`internalHelpers`)                                                                                                                                                                                                                                                                                             |
| `mermaid:rendering-util/rendering-elements/shapes.ts`      | 565                       | registers `mindmapCircle`                                                                                                                                                                                                                                                                                                                       |

## 2. Input

**What Mermaid builds.** `mindmapDb.getData()` (`mindmapDb.ts:357-421`) returns:

- `nodes`, in pre-order from the root (`flattenNodes`, 227-303). Each node has `id` (the numeric id as a string), `label`, `shape`, `width` and `height` as the database holds them (the DOM box replaces them, section 2 below), `padding`, `cssClasses`, `section`, `look`, `centerLabel: true`.
- `edges`, one per parent-child pair (`generateEdges`, 313-349), in the order of the children of each parent. Each edge has `id: edge_<parent>_<child>`, `start` (parent), `end` (child), `curve: 'basis'`, `classes`.
- `config`, `direction: 'TB'`, `markers: ['point']`, `type: 'mindmap'`, `diagramId` (random, replaced by the renderer).

**Which fields the tidy tree reads.**

| Field                                                 | Read by                                         | Notes                                                                                                                                                                                                 |
| ----------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `nodes[].id`                                          | layout (keys and parent map), render (`nodeDb`) | the root is the first node that has no parent (`layout.ts:106-111`)                                                                                                                                   |
| `nodes[].width`, `height`                             | layout                                          | replaced by the DOM box before the layout runs (`render.ts:81-84`, `93-101`). The width and height that the mindmap renderer presets (`mindmapRenderer.ts:58-74`) never reach the layout              |
| `nodes[].shape`                                       | layout (edge clipping), render (`insertNode`)   | the names are `mindmapCircle`, `rounded`, `rect` (renamed `squareRect` or `roundedRect` by `insertNode`, `nodes.ts:23-28`), `cloud`, `bang`, `hexagon`, `defaultMindmapNode` (`mindmapDb.ts:248-270`) |
| `nodes[].isGroup`                                     | render                                          | always `false` for mindmaps                                                                                                                                                                           |
| `edges[].id`, `start`, `end`                          | layout, render                                  | in the order of `getData`                                                                                                                                                                             |
| `edges[].curve`, `classes`, `look`                    | render (`insertEdge`, `insertEdgeLabel`)        | `curve: 'basis'` for every mindmap edge                                                                                                                                                               |
| `config`, `direction`, `markers`, `type`, `diagramId` | render                                          | `direction` goes to `insertNode` as `dir` (`render.ts:78`); `diagramId` prefixes the DOM ids (`mindmapRenderer.ts:51`, `mermaid:render.ts:88-92`)                                                     |

**Child order.** The split in section 3.2 depends on the order of the edges, which is the order of the children in the text. A port's parser has to keep that order.

**Which layout runs.** The text's `layout` value is read from the config. `getData` leaves it alone when the text sets it and sets `cose-bilkent` otherwise (`mindmapDb.ts:362-367`). `mindmapRenderer.ts:47-49` passes the value to `getRegisteredLayoutAlgorithm` with the fallback `cose-bilkent`. An unregistered name falls back with a warning (`mermaid:render.ts:113-129`). In the probe the page console showed no such message (section 7.3).

## 3. Algorithm

### 3.1 Registration and call path

1. The host page imports the package and calls `mermaid.registerLayoutLoaders(tidyTreeLayouts)` (`pkg:README.md`, usage; the removed page `e2e/platform/mindmap-layouts.html:363-368` did the same). The fork's bundle does not contain the package.
2. The package's default export is `[{ name: 'tidy-tree', loader, algorithm: 'tidy-tree' }]` (`pkg:src/layouts.ts:5-11`). `registerLayoutLoaders` stores each entry by name (`mermaid:render.ts:33-37`).
3. `mindmapRenderer.ts:47` selects `tidy-tree` when the text sets it.
4. `render` (`mermaid:render.ts:81-103`) awaits the loader, which imports the package's `render.js`, and then calls `render(data4Layout, svg, internalHelpers, { algorithm })`. The helpers are the host's own (`mermaid:internals.ts:19-31`), so the package does not import `mermaid` at run time.
5. The package's `render` (`pkg:src/render.ts:27`) inserts markers, then four groups in the svg `g`: `subgraphs`, `edgePaths`, `edgeLabels`, `nodes` (47-50).
6. Nodes: `insertNode(nodes, node, { config, dir })` for each node inside a `Promise.all` (54-91). The box comes from `getBBox()` (81). The layout runs once, after all boxes are known (93-105).
7. Positions: each positioned node gets `transform = translate(x, y)` on its element (109-117).
8. Edges: after `insertEdgeLabel` (130), the edge's points are looked up by id (136). The edge is drawn with `insertEdge(edgePaths, { ...edge, points }, clusterDb, type, startNode, endNode, diagramId)` and its label is placed with `positionEdgeLabel` (144-154). An edge that the layout did not return is drawn as a straight line between the two node centres (156-174).

Probed (section 7.3): the fork's bundle plus the registered loader renders all six removed fixtures with one tidy call each and no page error. The root's `transform` is `translate(0, 20)`.

### 3.2 The bidirectional split

- Children map and parent map, built from the edges in data order: `children[start].push(end)`, `parents[end] = start` (`layout.ts:93-104`).
- Root: the first node without a parent (`106-111`).
- Root children, by index in their list: index 0, 2, 4, ... go to the left tree and 1, 3, 5, ... go to the right (`120-130`). So the first child is on the left, and a single child is on the left too (probed).
- Each side is a virtual root with `id: virtual-root-${Math.random()}`, `width: 1`, `height: 1`, and the side's children (`buildSubTree`, 144-160). A child's converted node is transposed: `width = node.height`, `height = node.width` (`convertNodeToTidyTreeTransposed`, 166-184). So the library's breadth is the node's height, and its depth is the node's width.
- The root keeps its real size: `width = node.width ?? 100`, `height = node.height ?? 50` (113-118).

### 3.3 Constants

| Constant                         | Value                                             | Where                                |
| -------------------------------- | ------------------------------------------------- | ------------------------------------ |
| `gap` (added to breadth)         | 20                                                | `layout.ts:38`, `helpers.js:14`      |
| `bottomPadding` (added to depth) | 40                                                | `layout.ts:39`, `helpers.js:14`      |
| virtual root size                | 1 by 1, so its box is 21 by 41                    | `layout.ts:151-152`, `helpers.js:14` |
| `treeSpacing`                    | root width / 2 + 30                               | `layout.ts:199`                      |
| root `y`                         | 20, whatever the root's height                    | `layout.ts:261`                      |
| `intersectionShift`              | 30 (the value 50 at line 24 is overwritten at 40) | `layout.ts:24`, `40`                 |
| round shapes                     | `circle`, `cloud`, `bang`                         | `layout.ts:493-498`                  |
| returned node position           | centre of the box                                 | `layout.ts:270-271`, `280-281`       |

### 3.4 The library: van der Ploeg's non-layered tidy tree

The README cites A.J. van der Ploeg, "Drawing Non-layered Tidy Trees in Linear Time" (**unverified**: the paper was not read). The library follows that algorithm in a JavaScript port of the Java project at `cwi-swat/non-layered-tidy-trees` (README).

Mermaid uses the `BoundingBox(20, 40)` and `Layout.layout` path only.

1. **Convert** (`helpers.js:47-62`). Each node becomes a `Tree` with `w = width + 20`, `h = height + 40`, and `y = depth`. A child's depth is its parent's depth plus the parent's `h` (`helpers.js:57`).
2. **First walk** (`algorithm.js:165-181`). Bottom-up. Each parent places its children with `seperate` (110-151): each new subtree is moved right until its left contour clears the right contour of the subtrees on its left. The contours are compared by `bottom = y + h` (37-39). The left-sibling list `IYL` (41-58) holds the lowest bottom of each sibling that is still visible. Threads (`setLeftThread`, `setRightThread`, 86-108) let the walk find the next contour node in constant time. `positionRoot` (153-163) centres a parent over its first and last child, and `setExtremes` (24-35) keeps the extreme nodes.
3. **Second walk** (`algorithm.js:193-201`). `x = prelim + sum of modifiers` (`secondWalk`), then `addChildSpacing` (183-191) adds the child shifts and changes to the modifiers.
4. **Assign** (`helpers.js:106-125`). The returned node `x` is the tree's `x` plus `gap / 2`, and `y` is the tree's `y`. So the returned `x` is the left edge in breadth, and the returned `y` is the depth. Mermaid does not read the library's bounding box (`layout.ts:49, 55` keep only `.result`).

In Mermaid's frame the library's breadth is the vertical axis and its depth is the horizontal axis (`layout.ts:305-315`, `332-349`).

### 3.5 Where the nodes end up

For the right tree (`layout.ts:207-214`, `332-360`, `280-281`):

- centre `x = treeSpacing + depth + w / 2`, with `w` the original width;
- centre `y = breadth + h / 2 + offset`, with `h` the original height.

For the left tree (`204-205`, `298-326`, `270-271`): centre `x = -treeSpacing - depth - w / 2`, and the same `y`.

The offset (`216-256`). For each side, take the first-level nodes (`firstLevelLeftX` is the largest `x` on the left, `firstLevelRightX` the smallest on the right; equality is exact). Then `c = (min(top - h/2) + max(top + h/2)) / 2`, with the min and max over those nodes, and `offset = -c`. `top` is the library's breadth edge, not the centre, so the offset centres the first level on the wrong point (quirk Q1).

Consequences (probed, section 7.1 and 7.3):

- Gap from the root's side to a first-level node's side: **71 px**, which is 30 (from `treeSpacing`, `layout.ts:199`) plus 41 (the virtual root's box height, `layout.ts:151-152` and `helpers.js:14`). Example: root 120 wide, child 60 wide: the root's left edge is at -60 and the child's right edge at -131.
- Gap from a parent's side to a child's side deeper down: **40 px**, the parent's width is included. Example, Chromium fixture 4: `child1`'s left edge at -203.89 and `grandchild2`'s right edge at -243.89.
- Clearance between siblings on the same side: **20 px** in the vertical direction, at least (the contours can push them further apart).
- The root is at `(0, 20)`.
- Each child's depth is fixed by its parent's width, so the horizontal layout does not depend on the breadth algorithm.

### 3.6 The edge geometry and the quirks

`calculateEdgePositions` (`layout.ts:455-626`) builds every edge in data order:

1. **First pass.** `startPos = intersection(source, sourceCenter, targetCenter)` and `endPos = intersection(target, targetCenter, sourceCenter)`. For round shapes the circle version is used (500-524). The points `midX` and `midY` come from these first-pass values (526-527).
2. **Shift points.** One point 30 px outside the source's side, and one point 30 px outside the target's side (529-577). A root source gets the shift on the side facing the target. A left target gets its right edge plus 30, a right target its left edge minus 30. So every mindmap edge has exactly four points (probed on 59,012 edges, all with 4).
3. **Second pass.** `startPos = intersection(source, points[1], sourceCenter)` and `endPos = intersection(target, points[n-2], targetCenter)` (579-607). The first and last points are replaced by these.
4. **Return.** `startX/startY/endX/endY` are the second-pass values; `midX/midY` are the first-pass midpoint (609-626).

`intersection(node, outside, inside)` (`389-448`) returns the point where the segment from `inside` to `outside` leaves the box. A zero width or height returns `outside` (393-395). The top-bottom branch is taken when `|y - outside.y| * w > |x - outside.x| * h`, with the guards at 413-422, and the side branch otherwise, with the guards at 435-444. `computeCircleEdgeIntersection` (369-387) uses radius `min(w, h) / 2`.

The quirks. A port must reproduce each of them to match the reference's output. Each was probed.

| Quirk | Where                                        | What the reference does                                                                                                                                                   | Probe                                                                                                                                                                                                                 |
| ----- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1    | `layout.ts:216-233`, `255-256`, `271`, `281` | The first-level group is centred on the top edges, so a group of one node of height `h` has its centre at `y = h / 2`, not at the root's `y = 20`.                        | Node: two leaves of height 30 sit at `y = 15`. Chromium: the only right child of fixture 4 has height 47 and sits at `y = 23.5`                                                                                       |
| Q2    | `layout.ts:526-527`                          | `midX`/`midY` are the midpoint of the first-pass points, which are not the output endpoints. Mermaid's renderer never reads them (`render.ts:136-154` use only `points`). | Node: root to `A` gives `midX = -65.5`, `midY = 17.03`, while the endpoints are `(-60, 20)` and `(-131, 15)`                                                                                                          |
| Q3    | `layout.ts:493-498`                          | The round list has `circle`, not `mindmapCircle`, so a `((…))` node is clipped as a rectangle.                                                                            | Chromium: the root box is 134.97 wide and the edge starts at `x = -67.48`, the box's edge                                                                                                                             |
| Q4    | `layout.ts:529-577`                          | An edge between two nodes on the same side first goes out past the source's side, then back in to the target's side, and so reverses its direction.                       | Node: `c1 -> g1` has points `(-211,-35) (-241,-35) (-221,-60) (-251,-60)`. Every same-side edge in 2,000 random trees does it (55,085 of 55,085). Chromium shows the same hook in the path of `edge_1_2` in fixture 4 |

Smaller points that a port must also keep:

- The first-level gap of 71 comes from the virtual root's box height of 41 (`layout.ts:151-152`), and the deeper gap of 40 from `bottomPadding` (section 3.5). Neither is a free parameter of the port: both are fixed by the reference.
- The first child goes left (`layout.ts:125`).
- The zero-size guard (393-395) and the NaN and Infinity guards (413-422, 435-444) run in the order written. For example `r = (R * q) / Q` is computed before the guards for `Q = 0`.
- `intersectionShift = 50` (24) is dead code, overwritten at line 40.
- `TidyTreeLayoutConfig` (`types.ts:314-317`) and `validateLayoutData` (635-652) are not used by the render path.

### 3.7 Edge paths in the shared renderer

`insertEdge` clips the points a second time only when both end nodes have an `intersect` function (`edges.js:699-703`). The tidy tree passes `nodeDb` entries (`render.ts:68-74`), which are copies made before `insertNode`. They have no `intersect`, so the computed points stay as they are. Probed: the Chromium path starts at the layout's start point.

Then `fixCorners` runs for every curve except `rounded` (`edges.js:729-731`). Mindmap edges use `basis`, so the path goes through the corner pre-rounding and then `curveBasis`. In the Chromium output the path starts `M-67.483,20L-72.483,20C…`, which is the `basis` start of the points `(-67.48, 20)` and `(-97.48, 20)`. The corner pre-rounding changes only right-angle corners, and how often a mindmap edge has one is **unverified**.

The port's renderer must use the same `insertEdge` path, or the edge geometry will not match.

## 4. Randomness

| Draw                                       | Where                                                                      | Generator                | Order                                    | Effect on the output                                                                                                                                                                               |
| ------------------------------------------ | -------------------------------------------------------------------------- | ------------------------ | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Math.random()` for the left virtual root  | `layout.ts:150`, called from `buildSubTree` (`144-160`) at `layout.ts:132` | the page's `Math.random` | first, when the left side is not empty   | none. The id is a key in `TidyTreeNode` and never reaches the output. Probed: the output is the same under two `Math.random` streams, and the Chromium output is the same across renders and pages |
| `Math.random()` for the right virtual root | `layout.ts:150`, called at `layout.ts:134-135`                             | the page's `Math.random` | second, when the right side is not empty | none, as above                                                                                                                                                                                     |
| `v4()` in `getData`                        | `mindmapDb.ts:418`                                                         | uuid                     | at `getData`                             | overwritten by the renderer's `diagramId` (`mindmapRenderer.ts:51`)                                                                                                                                |

**State that survives.** The page's `Math.random` state advances by 0, 1 or 2 draws per tidy render. In the harness, `renderReference` seeds `Math.random` for each render (`tools/port/geometry/common.ts`, the `renderReference` function). Any later draw in the same render would see the shift. None is known in the mindmap path. `layout-utils/scoreLayout.ts:425-426` draws too, but the tidy tree does not reach it (**read**). A ferrigura port has no need to draw, since no output depends on the draws.

No other randomness is involved. The layout does not use the clock.

## 5. Output

- **Nodes.** `translate(x, y)` on each node's group, with `x, y` the centre (`render.ts:117`). The root is at `(0, 20)`. `width` and `height` in the returned nodes are the original sizes (`layout.ts:263-264`, `317-318`).
- **Edges.** The renderer reads `points` only (`render.ts:144-154`, `164-173`). `startX`, `startY`, `endX`, `endY`, `midX`, `midY` are computed but not read by Mermaid. A port should still return them, because they are in the reference's output and the differential compares them (section 7.2).
- **Labels.** `positionEdgeLabel` places a label on the points. Mindmap edges have no labels.
- **Order.** The returned edges are in data order, and the nodes are in the order the layout pushes them: the root, then the left tree, then the right tree (`layout.ts:258-289`). The DOM order of the groups is data order (probed in Chromium).
- **Keys.** Output ids are `String(id)`.

## 6. Determinism and floating point

- All arithmetic is IEEE 754 double precision. The operations are `+ - * /`, `Math.min`, `Math.max` (some with spread arguments), `Math.abs`, `Math.sqrt` (`layout.ts:374`), numeric `Set` and `sort` (`220-222`, `238-240`), and exact `===` on doubles to pick the first level (`224`, `242`).
- Operation order matters for bit-identical output. Keep the expressions as written, for example `positionRoot` (`algorithm.js:155-162`), `distance / nr` (62-64), and the sums in `addChildSpacing` (183-191).
- `Math.min(...xs)` returns NaN when any value is NaN. Rust's `f64::min` ignores NaN, so use an explicit fold that propagates it.
- In `intersection`, `r = (R * q) / Q` can be `±Infinity` or NaN before the guards at 413-422 run. Compute the same expression and apply the guards in the same order.
- There is no iteration limit. The loops end when the contours run out (`seperate`, `117`). The reference throws a TypeError if the walk along `IYL` runs past its end (`seperate` reads `ih.lowY` and `ih.index`). That was not seen in 30,000 random trees with 1,363,435 nodes (probed), and it is **unverified** that it cannot happen. A port should return an error there, not panic.
- `JSON.stringify` prints `-0` as `0`. Compare with `==`, or keep the sign in a separate field if a sign matters.

## 7. How to test a port

### 7.1 Known answers, no reference run needed

Probed: the library's known answers all match, computed by the library's own `src/` code (bundled with esbuild, see 7.2):

- README example: node 0 at `(300, 0)`, node 1 at `(185, 60)`, node 6 at `(5, 120)`, nodes 2 to 5 at `(242.5, 60)`, `(300, 60)`, `(357.5, 60)` and `(415, 60)`; bounding box `left 5, right 455, top 0, bottom 160`.
- `tree-layout.test.js`: two nodes (root `x 5, y 0`; child `x 0, y 4`); three nodes (root `x -5`; children `(0, 10)` and `(10, 10)`); reflection (`110/0, 70/10, 90/10, 110/10, 130/10, 150/10, 0/20`).
- `helpers.test.js`: the `Layout` class example (root `(120, 0)`, the five children at `x 75, 97.5, 120, 142.5, 165`, grandchild `(5, 40)`, bounding box `5, 175, 0, 50`); big root and small child (`(-25, 0)`, child `(0, 50)`, bounding box `-25, 75, 0, 100`); `getSize` (`10, 110, 0, 120`).

Mermaid's own `layout.test.ts` checks the root at `(0, 20)` and the first intermediate point of a root edge at `±(root width / 2 + 30)`. The real algorithm satisfies both (probed with a 100 px root: `(80, 20)` and `(-80, 20)`). Its mocks mean it checks nothing about the contour layout.

### 7.2 Layout-level differential

1. **Reference build.** `esbuild` from the repository's own install (`node_modules/.bin/esbuild`, version 0.25.12) bundles `layout.ts` from `2d85261d6^` with `--bundle --platform=node --format=esm --main-fields=module,main`. The library resolves to its `src/index.js` through the `module` field. Probed: this runs in Node without a browser, and the library's known answers pass when bundled this way.
2. **Capture.** A script writes, for each case, `{ input: { nodes: [{ id, width, height, shape }], edges: [{ id, start, end }] }, output: <executeTidyTreeLayout result> }` as JSON. The input is what `render.ts:93-101` hands over, so it can come from a Chromium run (section 7.3) or from a generator.
3. **Corpus.**
   - the six removed fixtures, captured in Chromium (their DOM sizes are the realistic inputs);
   - seeded random trees (the probes used mulberry32, depth up to 4, up to 8 children, 5 percent zero sizes, all eight shapes). Probed: 30,000 trees without an exception, a non-finite value or a root that is not at `(0, 20)`;
   - a single node, one child, and a root with two leaves.
4. **Comparison.** Every field of every node and edge, with `==`: ids and their order, `x`, `y`, `section`, `width`, `height`, every point, `startX` to `endY`, `midX`, `midY`, and the section and size fields of each edge. Report the first mismatch of each case. Exact equality is expected, because the port can follow the same operation order. A tolerance would hide an operation-order difference, so use none.
5. **What the differential cannot see.** It takes the sizes as input, so it does not test text measurement. Section 7.3 does.

### 7.3 Geometry, end to end

Probed, in Chromium, with the fork's built `packages/mermaid/dist` (no changes to the repository):

- The reference build of the removed package is served beside the fork's `mermaid.esm.mjs`. The page imports both and calls `mermaid.registerLayoutLoaders(tidyTree)` before any render. The registry is a module-level map, so one registration serves every fixture on the page.
- All six removed fixtures render with one tidy call each and no page error. Node shapes in the captured inputs: `mindmapCircle` and `rounded` (redux theme), and `cloud` and `bang` in fixture 3.
- Two renders of the same fixture in one page give identical SVG and identical captured output. The same holds across two pages.
- Without the registration call, the same fixture is laid out by `cose-bilkent`: the root is not at `(0, 20)`, no tidy call is captured, and the positions differ from one run to the next. The page console showed no message.

The harness plan that follows from this:

1. **Build step.** Like `buildRenderExample` in `tools/port/geometry/common.ts`, a build step extracts the five source files from `2d85261d6^` (`git show`) into `target/`, bundles them with esbuild as in 7.2, and writes an ESM file under `target/`. Nothing is vendored into the repository.
2. **The library** must be present at build time (section 8, question 3).
3. **Page.** The `pageHtml` function in `common.ts` imports the bundle, registers it, and serves it under a route next to `/dist/`, as the fonts already are.
4. **Check.** For a fixture whose text sets `layout: tidy-tree`, the harness fails if no tidy call happened. Without that check a fallback would be compared as if it were the tidy tree.
5. **Capture.** `--capture` needs the captured layout output. The production bundle has no hook, so the capture uses a second bundle built from a copy of `render.ts` with one line added after `executeTidyTreeLayout` (probed: this works).
6. **Fixtures.** The fork deleted the six fixtures (`2d85261d6`): `e2e/diagrams/mindmap/pr-check/1-…` to `5-…` (five files) and `e2e/platform/dev-diagrams/knsv/knsv2-16-mindmap-tidy-tree.mmd`. No file in the current tree sets `layout: tidy-tree`. Section 8, question 2.
7. **Theme.** The harness forces `redux-color` (`common.ts`, `forcedDirective`). Under that theme a default-shaped node is `rounded` (`mindmapDb.ts:264-265`), not `defaultMindmapNode`, so fixture 5 (described as having a rectangular root) has a `rounded` root in the harness.
8. **Exclusions.** None expected. `tidy-tree` is not in `NON_LAYERED_LAYOUTS` and should not be added.

### 7.4 What was probed, and how

- Node runs of esbuild bundles of the removed package's source and of the library's `src/` (sections 3.4, 3.5, 3.6, 4, 6, 7.1): the probe scripts were run from a scratch directory outside the repository. They are not committed.
- Chromium runs with the fork's built `packages/mermaid/dist` (served read-only) and a scratch bundle of the removed package (sections 3.1, 3.7, 7.3).
- Not done: the npm tarball of `@mermaid-js/layout-tidy-tree@1.0.1` was not downloaded (section 8, question 3).

## 8. Risks, effort, open questions

### Risks, in order

1. **The contour algorithm** (`lib:src/algorithm.js:41-151`). Mutable nodes, threads and the list of siblings must be transcribed one for one. A mistake shows only in some shapes, so the fuzz differential (7.2) is what catches it. Mitigation: arena indices instead of references, the same function names, one test per function.
2. **The quirks Q1 to Q4** (section 3.6). The easy mistake is to fix one, because it looks like a bug. Mitigation: a quirk table in the port's PLAN.md, with one test per quirk taken from the probe cases in section 7.
3. **Node sizes.** Every width and height is a direct input, with no smoothing. A text-measurement difference of a few hundredths of a pixel moves nodes by the same amount. Mitigation: the differential takes sizes as input, and the geometry harness tests the measurement.
4. **Edge paths.** The shared `insertEdge` and `fixCorners` decide the drawn path (3.7). A port that draws the points differently will differ from the reference even when the points match.
5. **Panics.** The reference throws in the `IYL` walk if it can run past the end (section 6). Unverified that it cannot. The port must return an error, not panic.
6. **Setup.** The library is no longer installed. The reference build needs it (section 8, question 3).

### Effort

The layout is smaller than cose-bilkent or fcose: no solver, no randomness, no iteration. The work is careful transcription of about 500 lines of logic, the intersection code, and the differential. Rough size: 700 lines of Rust for the layout and its tree type, 300 lines of tests (known answers, quirk cases, the capture and differential driver), and the renderer glue on the unified renderer.

### Open questions for the coordinator

1. **Which reference?** The fork's own bundle, with no registration, draws `layout: tidy-tree` as `cose-bilkent` (probed). The published package, once registered, draws the tidy tree. Decision 0008 and P2-PLAN say ferrigura gives the tidy tree. My reading: the reference is the registered package, and the fork's fallback is a fork artefact that ferrigura does not reproduce. Please confirm.
2. **Fixtures.** Restore the six removed fixtures into the repository's corpus (where?), or generate the corpus from the harness? The current tree has none.
3. **Tooling.** Two ways to get the reference:
   - build from the history commit `2d85261d6^` (no download; needs the library, which is no longer in the lockfile: it would become a tools dependency with a lockfile change, or be fetched from the main checkout's store at build time);
   - use `@mermaid-js/layout-tidy-tree@1.0.1` from npm (`npm view` shows the version exists, its `dist/mermaid-layout-tidy-tree.core.mjs` export and a single dependency, `d3`). The tarball was not downloaded, because a download needs your permission in this session.
     My recommendation: build from history as the reference, and use the npm tarball as a cross-check, after you approve the download.
4. **Reproduce the quirks?** My recommendation is yes: the contract is the reference's output. Fixing Q1 to Q4 would be a separate decision, and would make the port disagree with Mermaid's own tidy tree.
5. **`midX`, `midY`.** Keep them in the engine's result (the reference has them, and the differential compares them), and leave them out of the canonical model if the model has no use for them. Mermaid's renderer does not read them.
6. **Capture hook.** A separate capture bundle (my recommendation) or a hook in the production harness bundle.
7. **Mindmap `layout` in the model.** The mindmap model must carry the text's `layout` value, because `getData` decides the default from it (`mindmapDb.ts:362-367`). This is for the mindmap parser task, not this one.
