# cose-bilkent (P2): research notes for the port

Status: research only. Reference: the checkout at `aa5c4338f` (branch `sebastian/port-p2`). Nothing in the repository changed except this file. Probe scripts are outside the repository, in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/cose/`.

Labels used below: **probed** means the reference code ran (the shipped bundles in Node, or `packages/mermaid/dist` in Chromium) and the stated result was seen; **unverified** means read from the source only. Source citations are `path:line`. Library paths are under `node_modules/.pnpm/<pkg>@<ver>/node_modules/<pkg>/`, written as `<pkg>@<ver>/src/...`.

## 0. Summary

| Item                         | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it is                   | `cytoscape-cose-bilkent` 4.1.0: a CoSE spring embedder (compound-capable) driven through cytoscape. Mermaid calls it through `rendering-util/layout-algorithms/cose-bilkent/`.                                                                                                                                                                                                                                                                                                                           |
| Who reaches it               | Mindmaps only (default layout). `validateLayoutData` requires `data.rootNode` (`layout.ts:64-66`), and only `mindmapDb.getData` sets it (`mindmapDb.ts:408`). A flowchart with `layout: cose-bilkent` fails in the reference with "Root node is required" (**probed**, Chromium, `mm-run.mjs cycle`).                                                                                                                                                                                                    |
| Inputs Mermaid can produce   | Trees: one root, parent-to-child edges, no cycles, no isolated nodes. Every such input takes the flat-forest branch.                                                                                                                                                                                                                                                                                                                                                                                     |
| Random draws on those inputs | None. The forest branch uses radial placement, not the random generators (**probed**: 0 `RandomSeed` draws and 0 `Math.random` calls for every tree tried, 1 to 120 nodes).                                                                                                                                                                                                                                                                                                                              |
| Determinism on those inputs  | Yes. The reference renders the same mindmap identically twice in one page and in fresh pages (**probed**). The Node engine reproduces the reference centres exactly (**probed**, 7 nodes, difference 0).                                                                                                                                                                                                                                                                                                 |
| Library versions             | cytoscape-cose-bilkent 4.1.0 loads **cose-base 1.0.3 and layout-base 1.0.2**, not layout-base 2.0.1 (**probed** with `require.resolve`; the symlinks confirm it). layout-base 2.0.1 belongs to cose-base 2.2.0, which cytoscape-fcose uses for architecture. See section 8.                                                                                                                                                                                                                              |
| Size                         | 34 library files, 6,300 lines, about 4,700 code lines (section 1.2). The mindmap path needs about 3,000 of them.                                                                                                                                                                                                                                                                                                                                                                                         |
| Riskiest parts               | (1) `sin`, `cos`, `log` and `pow` differ between V8 and Rust's libm at the last bit (measured in section 6); (2) bit-exact float order and set iteration order; (3) a quirk in `findCenterOfTree` that changes the radial centre; (4) edge endpoints come from cytoscape's 30 x 30 default node style, not from Mermaid's sizes, so the port must reproduce cytoscape's ellipse intersection; (5) capturing the reference needs a rebuilt bundle, because the current `dist` has no capture hook at all. |

## 1. Sources

### 1.1 Mermaid's integration

| File                                                                                    |  Lines | Role                                                                                                                                                                                                                                            |
| --------------------------------------------------------------------------------------- | -----: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/mermaid/src/rendering-util/layout-algorithms/cose-bilkent/cytoscape-setup.ts` |    207 | `addNodes` 36-62, `addEdges` 70-90, `createCytoscapeInstance` 97-145 (container and removal 100-115, `layoutDimensions` override 122-127, layout config 130-136, `run` 138), `extractPositionedNodes` 152-173, `extractPositionedEdges` 180-207 |
| `.../cose-bilkent/layout.ts`                                                            |     77 | `executeCoseBilkentLayout` 21-48, `validateLayoutData` 55-77 (`rootNode` check 64-66)                                                                                                                                                           |
| `.../cose-bilkent/render.ts`                                                            |    197 | Render: node insertion and `getBBox` 54-84, layout data 90-100, layout call 102, node positions 107-123, edge points 125-135, edge insertion 137-194 (fallback 173-190)                                                                         |
| `.../cose-bilkent/index.ts`, `types.ts`                                                 | 25, 43 | Re-export, interfaces                                                                                                                                                                                                                           |
| `packages/mermaid/src/diagrams/mindmap/mindmapDb.ts`                                    | (file) | `getData` 353-410 (layout default 362-368, `rootNode` 408), `flattenNodes` 227-306, `generateEdges` 313-349, node object 102-112, padding 92-98                                                                                                 |
| `packages/mermaid/src/diagrams/mindmap/mindmapRenderer.ts`                              |    120 | `draw` 33-116; layout chosen 47-49; shape fixes 58-74; `render` 77                                                                                                                                                                              |
| `packages/mermaid/src/rendering-util/render.ts`                                         | (file) | Layout registry 28-51, 76-80 (`cose-bilkent` is registered globally)                                                                                                                                                                            |

Tests that exist: `cytoscape-setup.test.ts` (5 cases, 265 lines), `layout.test.ts` (11 cases, 236 lines), `mindmap.spec.ts` (26), `mindmapDb.getData.test.ts` (12), `mindmapLayout.spec.ts` (3), `mindmapColors.spec.ts` (6). Fixtures: `e2e/diagrams/mindmap/` (3 files).

### 1.2 The libraries, lines per file

Totals per library: cytoscape-cose-bilkent `index.js` 391 lines (279 code); cose-base 1.0.3, 6 files, 1,418 lines; layout-base 1.0.2, 27 files, 4,458 lines. "Code" counts non-blank lines with comments removed (approximate). "Mindmap" counts the code lines of named functions that the mindmap probe executed (V8 function coverage, section 7.4). "All" is the same for the full probe set (trees, cycles, leaves, isolated nodes, single node).

| File (`src/`)                                                                                                       |               Lines |                Code |                            Mindmap |                         All |
| ------------------------------------------------------------------------------------------------------------------- | ------------------: | ------------------: | ---------------------------------: | --------------------------: |
| cytoscape-cose-bilkent `index.js`                                                                                   |                 391 |                 279 |                                182 |                         182 |
| cose-base `CoSELayout.js`                                                                                           |               1,244 |                 953 |                                515 |                         865 |
| cose-base `CoSENode.js`                                                                                             |                 121 |                  98 |                                 41 |                          41 |
| cose-base `CoSEConstants.js`, `CoSEEdge.js`, `CoSEGraph.js`, `CoSEGraphManager.js`                                  |      20, 13, 13, 13 |         14, 9, 9, 9 |                         (wrappers) |                  (wrappers) |
| layout-base `Layout.js`                                                                                             |                 673 |                 451 |                                316 |                         345 |
| layout-base `LNode.js`                                                                                              |                 406 |                 317 |                                188 |                         202 |
| layout-base `LGraph.js`                                                                                             |                 478 |                 390 |                                277 |                         339 |
| layout-base `LGraphManager.js`                                                                                      |                 492 |                 391 |                                233 |                         304 |
| layout-base `LEdge.js`                                                                                              |                 154 |                 123 |                                 68 |                          68 |
| layout-base `LGraphObject.js`, `LayoutConstants.js`                                                                 |               6, 71 |               4, 20 |                        (constants) |                 (constants) |
| layout-base `fd/FDLayout.js`                                                                                        |                 530 |                 408 |                                341 |                         341 |
| layout-base `fd/FDLayoutNode.js`, `FDLayoutEdge.js`, `FDLayoutConstants.js`                                         |          43, 16, 35 |          29, 11, 28 |                            7, 0, 0 |                     7, 0, 0 |
| layout-base `util/IGeometry.js`                                                                                     |                 568 |                 403 |                                352 |                         352 |
| layout-base `util/RectangleD.js`                                                                                    |                 131 |                 105 |                                 68 |                          68 |
| layout-base `util/Transform.js`                                                                                     |                 158 |                 127 |                                 45 |                          45 |
| layout-base `util/Emitter.js`, `IMath.js`, `Integer.js`                                                             |           35, 31, 8 |           27, 23, 5 |                           6, 14, 0 |                    6, 14, 0 |
| layout-base `util/LinkedList.js`                                                                                    |                 131 |                 102 | (class; used by `updateConnected`) |                        same |
| layout-base `util/Point.js`, `PointD.js`, `DimensionD.js`                                                           |          74, 49, 31 |          60, 40, 25 |                      (small; used) |               (small; used) |
| layout-base `util/RandomSeed.js`                                                                                    |                  13 |                   9 |                                  0 |                           4 |
| layout-base `util/Quicksort.js`, `HashMap.js`, `HashSet.js`, `UniqueIDGeneretor.js`, `alignment/NeedlemanWunsch.js` | 77, 31, 56, 30, 158 | 59, 24, 43, 24, 104 |                                  0 | 0 (unused by the cose path) |

Totals: 6,300 lines and about 4,723 code lines in 34 files. Named-function code lines executed: 2,653 by the mindmap probe, 3,183 by the full probe set. The mindmap path also runs about 300 further code lines of module-level code (wrappers, constants, `index.js` defaults). Rough port size: section 8.

### 1.3 Cytoscape 3.34.0 (`node_modules/.pnpm/cytoscape@3.34.0/node_modules/cytoscape/dist/cytoscape.cjs.js`)

| Code                                     |        Line | What the layout reaches                                                                                                                                |
| ---------------------------------------- | ----------: | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `head` decision                          |       19861 | `head` is true only with a container and without `headless`; it sets the renderer                                                                      |
| `styleEnabled` default                   |       19911 | `options.styleEnabled === undefined ? head : options.styleEnabled`. Mermaid passes a container, so style is on in the browser and off in headless Node |
| `layoutDimensions`                       | 13168-13195 | Replaced by Mermaid's override (`cytoscape-setup.ts:122-127`); the default sanitises 0 to 1 (13190-13194), the override does not                       |
| `layoutPositions`                        |      13196- | Writes the final positions (animation off)                                                                                                             |
| Null renderer                            | 22583-22598 | `recalculateRenderedStyle` is a no-op                                                                                                                  |
| `recalculateRenderedStyle` (element)     | 13369-13376 | Gated on `styleEnabled`                                                                                                                                |
| Base renderer `recalculateRenderedStyle` | 25447-25520 | Calls `recalculateEdgeProjections` (25514) to `findEdgeControlPoints` (24254)                                                                          |
| Edge start point                         | 24690-24741 | `rs.startX`, `rs.startY` (24740-24741), from `intersectLine` with the node shape and the arrow gap                                                     |
| `intersectLineEllipse`                   |   4073-4084 | Ellipse boundary along the line from the centre                                                                                                        |
| Ellipse node shape                       | 27575-27584 | `intersectLine` calls `intersectLineEllipse` with `width/2 + padding` (the 30 x 30 style)                                                              |
| `shortenIntersection`                    |   4300-4308 | Moves the point toward the other end by `amount`; called with amount 0 (no arrows), so the operation still runs                                        |

## 2. Input

What Mermaid builds and hands the engine, in order.

1. **Nodes** (`mindmapDb.getData`, `flattenNodes` 227-306). Pre-order traversal. `id` is the string of the mindmap counter (0, 1, 2, ...). `x` and `y` are `undefined`, because the mindmap DB never sets them. `width` and `height` are the mindmap's own values, but the cose render replaces both with the measured box (below). `padding` is set from the mindmap config (`mindmapDb.ts:92-98`, doubled for rect-like shapes).
2. **Edges** (`generateEdges` 313-349). Depth-first: for each child, the edge `edge_<parent>_<child>` (`start` = parent id, `end` = child id), then the child's own edges. Edge order matters: it sets each node's incidence list, which the radial placement reads (section 3.3).
3. **Config** (`getData` 362-368). `config.layout` becomes `cose-bilkent` unless the user set `layout`. `rootNode` is set (408). `direction`, `markers` and `diagramId` (`'mindmap-' + v4()`, 69) are not read by the engine.
4. **Sizes** (`render.ts:54-84`). Every node is inserted into the page and measured with `getBBox()`; the group is centred on its origin (**probed**: `bx = -width/2`, `by = -height/2` for every node). These measured doubles are the engine's node sizes. The mindmap renderer's own width and padding values do not reach the engine (see below).
5. **Cytoscape set-up** (`cytoscape-setup.ts`). A hidden `div#cy` is appended to `body`, the instance is created with it as container and a single style rule, `edge { curve-style: bezier }` (102-112), then the `div` is removed (115). Nodes are added in `data.nodes` order with `data` `{id, labelText, height, width, padding}` and position `x ?? 0`, `y ?? 0` (36-62). Edges follow in `data.edges` order (70-90). Each node gets `layoutDimensions = () => {w: width, h: height}` (122-127). Config: `name: 'cose-bilkent', quality: 'proof', styleEnabled: false, animate: false` (130-136).
6. **Padding is not read.** The adapter reads `css('padding')` (`index.js:334-337`), which comes from the cytoscape style, not from `data`. In the reference `css('padding')` returns `'0px'` (**probed**, Chromium). The reference mindmap has non-zero padding in its data and still matches a run with padding 0 exactly (**probed**). So the mindmap padding never reaches the layout.
7. **Initial positions are ignored.** The forest branch overwrites every centre (section 3.3), and the mindmap sends `undefined`, which becomes 0. Random initial positions gave the same output (**probed**).
8. **Order** is insertion order for both nodes and edges, not sorted by id (**probed**: ids `b, 10, a, 2` stay in that order).

## 3. Algorithm

### 3.1 Graph construction (`cytoscape-cose-bilkent/src/index.js` 124-156, 279-368)

- The adapter (`_CoSELayout`) builds a CoSE graph. Top-most nodes (`getTopMostNodes` 289-309; for a flat graph, all nodes in order) are added to the root with `processChildrenList` (311-368).
- A node's rectangle is the centre minus half its size: `x - w/2`, `y - h/2`, size `(w, h)` (318-326). `NaN` coordinates become 0 (354-360).
- An edge is added unless it is a self loop or a parallel edge (148-156): `sourceNode !== targetNode && sourceNode.getEdgesBetween(targetNode).length == 0`. Each edge's id is set to the cytoscape id (154).
- Options are copied from defaults and the user config into module-level constants (`getUserOptions`, index.js 84-122). The values Mermaid's config leaves unset are the library defaults (index.js 11-63), except where Mermaid's config passes its own (see below).

Effective constants after `getUserOptions` (library default in brackets when it differs):

| Constant                                                            | Value                                | Where                        |
| ------------------------------------------------------------------- | ------------------------------------ | ---------------------------- |
| `LayoutConstants.QUALITY`                                           | 2 (`proof`)                          | index.js 106-111 (default 1) |
| `DEFAULT_INCREMENTAL`                                               | false (`!randomize`, randomize true) | index.js 114-115             |
| `MAX_ITERATIONS` (`numIter`)                                        | 2500                                 | index.js 95-96               |
| `DEFAULT_EDGE_LENGTH` (`idealEdgeLength`)                           | 50                                   | index.js 87-88               |
| `DEFAULT_SPRING_STRENGTH` (`edgeElasticity`)                        | 0.45                                 | index.js 89-90               |
| `DEFAULT_REPULSION_STRENGTH` (`nodeRepulsion`)                      | 4500                                 | index.js 85-86               |
| `DEFAULT_GRAVITY_STRENGTH` (`gravity`)                              | **0.25** (library 0.4)               | index.js 93-94               |
| `DEFAULT_COOLING_FACTOR_INCREMENTAL` (`initialEnergyOnIncremental`) | **0.5** (library 0.3)                | index.js 103-104             |
| `PER_LEVEL_IDEAL_EDGE_LENGTH_FACTOR` (`nestingFactor`)              | 0.1                                  | index.js 91-92               |
| `DEFAULT_GRAVITY_RANGE_FACTOR` (`gravityRange`)                     | 3.8                                  | index.js 97-98               |
| `DEFAULT_COMPOUND_GRAVITY_STRENGTH`, `..._RANGE_FACTOR`             | 1.0, 1.5                             | index.js 99-102              |
| `TILE`, `TILING_PADDING_VERTICAL`, `..._HORIZONTAL`                 | true, 10, 10                         | index.js 117-121             |

Because the library's `getUserOptions` runs for every layout and rewrites these constants, nothing from one layout leaks into the next, except `RandomSeed` (section 4.4).

### 3.2 The engine's run (`cose-base/src/CoSELayout.js` 89-149)

`runLayout` (`layout-base/src/Layout.js:97-138`) runs `tilingPreLayout` (CoSELayout 1017-1026), `initParameters` (CoSELayout 47-87), `checkLayoutSuccess`, then `layout()` (89-99, sets `level = 0`), then `doPostLayout`, then `tilingPostLayout`. `classicLayout` (101-149) does, in order:

1. `calculateNodesToApplyGravitationTo` (279-299): nodes of every graph that is not connected. For a connected tree this is the empty list (**probed**: `calcGravitationalForce` called 0 times).
2. `calcNoOfChildrenForAllNodes` (`FDLayout.js:391-401`), `calcLowestCommonAncestors`, `calcInclusionTreeDepths`, root `calcEstimatedSize` (`LGraph.js:386-408`, `LNode.js:264-277`). These feed only the gravity and inter-graph logic, so they have no effect on a flat graph (**unverified** for the flat case beyond the probe outputs).
3. `calcIdealEdgeLengths` (`FDLayout.js:43-80`): every flat edge gets `idealLength = 50`.
4. Forest or not (`getFlatForest`, layout-base `Layout.js:349-459`). A forest goes to `positionNodesRadially`; anything else goes to `reduceTrees` and `positionNodesRandomly` (section 4). Trees use the first branch (**probed**: `positionNodesRadially` once, `reduceTrees` 0 times, `positionNodesRandomly` 0 times).
5. `initSpringEmbedder` (`FDLayout.js:82-111`), then `runSpringEmbedder` (CoSELayout 260-277), which runs `tick` until it returns true.

### 3.3 Radial start for a forest (`CoSELayout.js` 355-536)

- **Trees.** `getFlatForest` runs a breadth-first search per connected component, pushing each component as `[...visited]` in visit order (layout-base `Layout.js:377-455`). A non-tree component makes the forest empty.
- **Centre of a tree.** `Layout.findCenterOfTree` (layout-base `Layout.js:587-661`) repeatedly removes leaves. It has a quirk that a port must copy: it loops over `list` while `list.splice(index, 1)` shrinks it (`Layout.js:624-631`), so every other remaining node is skipped in each round. **Probed**: the star with centre `s` and four leaves gives the leaf `x1`; the path `a-b-c-d-e` gives `b`, not the middle `c`; the 7-node mindmap of section 5.2 gives node 6, not the root 0.
- **Grid of trees.** Trees are placed row by row, `ceil(sqrt(n))` per row (`CoSELayout.js:355-405`). `DEFAULT_COMPONENT_SEPERATION = 60` (`CoSEConstants.js:13`) separates the rows (375) and the trees in a row (399). Then `transform(WORLD_CENTER_X - point.x/2, WORLD_CENTER_Y - point.y/2)` (402-404), where `point` is the last tree's bottom-right corner, not the largest (a quirk; a single tree is the Mermaid case).
- **Radial placement.** `radialLayout` (407-429): separation `radialSep = max(maxDiagonalInTree, DEFAULT_RADIAL_SEPARATION)` (408-409, `DEFAULT_RADIAL_SEPARATION = 50`); `branchRadialLayout(centre, null, 0, 359, 0, radialSep)` (410). A node at `(distance, startAngle..endAngle)` gets `halfInterval = (end - start + 1)/2` (433; the `+1` is in the source), `nodeAngle = (halfInterval + start) % 360`, `teta = nodeAngle * TWO_PI / 360` (441), centre `(distance * cos teta, distance * sin teta)` (444-448), and each child gets `stepAngle = |end - start| / childCount` and distance `+ radialSeparation` (493-515). Children are walked in `getEdges()` order starting after the parent's edge (483-491). Then the tree is moved to the start point with a `Transform` (413-423), and `LGraph.calculateBounds` gives the device origin (411, `LGraph.js:319-364`).
- `maxDiagonalInTree` is the maximum `sqrt(w^2 + h^2)` over the tree's nodes (`CoSELayout.js:521-536`, `LNode.js:104-108`).
- **Adjacency order.** Each node's `edges` list is filled in the adapter's edge order (`LGraph.js:113-121`). That order fixes the branch order, so the order of `cy.edges()` must be kept (it is insertion order, **probed**).

### 3.4 Spring embedder setup (`FDLayout.js:82-111`, `CoSELayout.js:538-541`)

| Quantity                                | Value (non-incremental, n nodes)                                                                                                                                        | Source                                                                                                        |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `coolingFactor`, `initialCoolingFactor` | 1.0 while n <= 1000; above that a linear drop that reaches 0.33 at 5,000 nodes                                                                                          | FDLayout 92-100                                                                                               |
| `maxNodeDisplacement`                   | `MAX_NODE_DISPLACEMENT` = 300                                                                                                                                           | FDLayout 101; FDLayoutConstants 26-27                                                                         |
| `maxIterations`                         | `max(5n, 2500)`                                                                                                                                                         | FDLayout 104-105                                                                                              |
| `totalDisplacementThreshold`            | `1.5 * n` (`displacementThresholdPerNode = 3 * 50 / 100`)                                                                                                               | FDLayout 107-108, constructor 18                                                                              |
| `repulsionRange`                        | `2 * (level + 1) * idealEdgeLength` = 100 at level 0                                                                                                                    | CoSELayout 538-541, FDLayout 110                                                                              |
| Cooling schedule constants              | `maxCoolingCycle = 2500/100 = 25`, `finalTemperature = 100/2500 = 0.04`, computed in `initParameters` from the value before `initSpringEmbedder` raises `maxIterations` | CoSELayout 83-84 (unverified that a port should keep the earlier value for n > 500; it is what the code does) |

### 3.5 One iteration (`CoSELayout.tick`, 151-240)

For each iteration, in order:

1. `totalIterations++`. At `maxIterations`, stop (151-161).
2. At every 100th iteration (`CONVERGENCE_CHECK_PERIOD`), check `isConverged` (163-165), which tests the displacement of the previous iteration. Then `coolingCycle++` and the cooling factor is updated (177-187):
   `coolingFactor = max(initial - pow(cycle, log(100 * (initial - final)) / log(maxCoolingCycle)) / 100 * adjuster, final)`.
   With quality `proof` (2), `coolingAdjuster` stays 1, because lines 177-184 only handle quality 0 and 1.
3. Growth branches (191-226). These run only when `prunedNodesAll` is not empty, which is not the case for a forest.
4. `totalDisplacement = 0`, then `graphManager.updateBounds()` (232), which recomputes group bounds recursively (`LGraph.updateBounds` 248-317).
5. `calcSpringForces` (233; `FDLayout.js:113-123`, `calcSpringForce` 192-234): for each edge in `getAllEdges` order, `length` from `updateLength` (`LEdge.js:105-132`, the clipped segment between the two rectangles, `getIntersection2`); if the rectangles overlap, skip the edge; otherwise `force = 0.45 * (length - 50)`, projected on `lengthX/length` and `lengthY/length`, added to the source and subtracted from the target.
6. `calcRepulsionForces` (234; `FDLayout.js:125-168`). The grid variant is on (`useFRGridVariant`). When `totalIterations % 10 == 1` (and growth is not running), `updateGrid` rebuilds the grid of cells of size `repulsionRange` over the root bounds (`FDLayout.js:407-467`). Then, for each node in `getAllNodes` order (`calculateRepulsionForceOfANode`, 469-523): when the grid was rebuilt, collect `surrounding` = the nodes in cells `startX-1 .. finishX+1` by `startY-1 .. finishY+1` (cell order, then cell list order) that are not the node itself, not yet processed, and within `repulsionRange` on both axes (the gap `|dx| - (wA + wB)/2` tested against 100); the set is turned into an array in insertion order (516). Then `calcRepulsionForce` (236-314) for each surrounding node:
   - overlapping rectangles: `calcSeparationAmount` (`IGeometry.js:21-111`) with buffer `DEFAULT_EDGE_LENGTH/2 = 25`; force `2 * overlap * childrenConstant` (childrenConstant = 1 for leaves), added with opposite signs;
   - otherwise: distances from `getIntersection2` clip points (`uniformLeafNodeSizes` is false, so the clip path runs; `IGeometry.js:168-431`); each axis distance is raised to `MIN_REPULSION_DIST = 5` in magnitude (`IMath.sign` for the sign); `force = 4500 * 1 * 1 / d^2`, projected on `dx/d` and `dy/d`, applied with opposite signs.
7. `calcGravitationalForces` (235; `FDLayout.js:170-179`) runs only on nodes of unconnected graphs, which is none for a tree.
8. `moveNodes` (236; `CoSENode.js:14-63`): for each node in `getAllNodes` order, `displacement = coolingFactor * (spring + repulsion + gravitation) / noOfChildren` (leaf: 1), clamped to `coolingFactor * 300` per axis (17-33), `moveBy` (36-44), and `totalDisplacement += |dx| + |dy|` (52-53). Then all forces are zeroed (55-62).
9. `animate()` does nothing (`FDLayout.js:375-388`, `animationDuringLayout` is false).

Termination (`isConverged`, `FDLayout.js:358-373`): converged when `totalDisplacement < 1.5 n`, or oscillating when `totalIterations > maxIterations/3` and `|totalDisplacement - oldTotalDisplacement| < 2`. `oldTotalDisplacement` is updated on every call, so the check sees the previous iteration. Iteration counts (**probed**, random trees): 100 for 7, 30 nodes (200 for one 15-node tree); 700 for 60 nodes; 1,400 for 120 nodes.

### 3.6 Finish (`Layout.js:143-151, 276-307`; `LGraph.js:200-246`; `LNode.js:345-373`)

`doPostLayout` calls `transform()`, which shifts every node so that the top-left of the bounds, with a margin of 15 (`LayoutConstants.DEFAULT_GRAPH_MARGIN`, `LayoutConstants.js:26`), becomes the origin. Node coordinates are clamped to `±WORLD_BOUNDARY` (1,000,000) before the transform (`LNode.js:345-367`). Check: the mindmap node at x = 66 has width 102, so its left edge is 15 (**probed**, Node, 8-node tree).

Then `layoutPositions` writes each node's centre (`index.js:280-283`, `getPositions` 158-169).

### 3.7 Tiling (`CoSELayout.js:1017-1033` and 546-1015)

`tilingPreLayout` groups zero-degree nodes under a dummy compound per parent when a parent has two or more of them (546-611), tiles compound children (793-848, `tileNodes` sorts by area, largest first, with `Array.prototype.sort`, which is stable in V8, then packs rows, 850-971), and `tilingPostLayout` puts them back (1028-1033, 662-674). For a mindmap nothing is grouped: a tree with two or more nodes has no zero-degree node, and the single-node mindmap has one root with no group of two. **Probed** (call counters on the shipped bundle): `groupZeroDegreeMembers` runs once per layout, and `tileNodes` is never called, for the 7-node mindmap and for the single node. For other inputs the tiling is live (section 8).

## 4. Randomness

### 4.1 The generators

| Generator                                                                                                                                 | Where                                                                                                                        | Shared state                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `RandomSeed.nextDouble` (layout-base `util/RandomSeed.js:1-12`): `x = sin(seed++) * 10000; return x - floor(x)`, with `seed = 1`, `x = 0` | Used only by `LNode.scatter` (`LNode.js:279-295`, two calls per node, X then Y, `Layout.js:309-341`)                         | Module-level in layout-base 1.0.2, so one copy per page for every cose-bilkent layout. `Layout.RANDOM_SEED = 1` (`Layout.js:50`) is never read |
| `Math.random`                                                                                                                             | `findPlaceforPrunedNode` only (`CoSELayout.js:1166` for two tied regions, `1217` for four): `floor(r * 2)` or `floor(r * 4)` | The V8 global generator (not seedable from Mermaid), shared with all of the page                                                               |

The two copies of layout-base are separate: the reference bundle has layout-base 1.0.2 for cose-bilkent and 2.0.1 for fcose, each with its own `RandomSeed` (**probed** in the bundle: `packages/mermaid/dist/mermaid.js` has both module headers, lines 198126 and 271854).

### 4.2 Every draw, in order

Trees (the mindmap case): no draw at any point. The forest branch uses only the radial code and `Math.cos`/`Math.sin`, which are deterministic.

Non-trees (the path no diagram reaches through Mermaid, but present in the library):

1. `tilingPreLayout` and `reduceTrees` (`CoSELayout.js:1039-1071`): no draw. Leaves are pruned in rounds and kept in `prunedNodesAll`.
2. `positionNodesRandomly` (`Layout.js:309-341`): for each remaining node in graph order, recursing into non-empty compounds and calling `scatter` on simple and empty nodes: **two** `RandomSeed.nextDouble` draws per node, X first then Y. Pruned leaves are not in the graph by now, so they draw nothing.
3. `initSpringEmbedder`: no draw.
4. `tick` iterations, in the growth phase (`isTreeGrowing`, starts at convergence or at `maxIterations` when pruned nodes exist, 154-173 and 191-213): `growTree` (1074-1092) takes the last pruned level, and for each pruned node in order calls `findPlaceforPrunedNode` (1095-1241). That function draws one `Math.random` in exactly two cases: two of the four neighbouring regions have count 0 (`minCount == 2`, draw at 1166), or all four do (`minCount == 4`, draw at 1217). The three-region case, the single minimum, and every other case draw nothing.

Counts per layout (**probed** with a wrapped `Math.random` and the `RandomSeed.seed` counter, `probe2.cjs`, three runs in one process):

| Graph                   | `RandomSeed` draws | `Math.random` calls | Note                                                                              |
| ----------------------- | -----------------: | ------------------: | --------------------------------------------------------------------------------- |
| Tree (any size)         |                  0 |                   0 | Mermaid's case                                                                    |
| Single node             |                  0 |                   0 |                                                                                   |
| 4-cycle                 |                  8 |                   0 | 2 per node                                                                        |
| 4-cycle plus one leaf   |                  8 |              0 or 1 | the leaf is pruned, so 4 nodes are scattered; the call count differs between runs |
| 4-cycle plus two leaves |                  8 |              0 or 2 | the calls change with the positions, which change with the seed                   |

### 4.3 What decides the number of draws

- Topology decides whether any draw happens: a forest makes none.
- For a non-forest, the number of scattered nodes decides the `RandomSeed` draws: 2 times the non-pruned simple nodes.
- The `Math.random` count depends on the grid at growth time, which depends on the scattered starting positions and on how many iterations ran before growth. It is not a function of the topology alone.

### 4.4 What survives from one layout to the next, on one page

- **`RandomSeed.seed` and `RandomSeed.x`**: never reset. Three layouts in one process moved the counter 1 to 9 to 17 to 25 (**probed**). So a second non-tree layout on a page starts from a different state than the first; this is the "state survives" effect the P2 plan talks about.
- **`Math.random`**: the page's generator state, changed by any code on the page.
- **Library constants**: rewritten by `getUserOptions` in every layout from the same values (section 3.1). They do not survive in practice.
- **The `cytoscape.use` registration** (`cytoscape-setup.ts:9`): once per page.
- **Nothing else.** Each layout makes its own `CoSELayout`, `LGraphManager` and cytoscape instance.

### 4.5 What this means for Mermaid's diagrams

For every input Mermaid can give the engine (trees), there are no draws, so the output does not depend on the page's state. The reference confirmed this: the same mindmap rendered twice in one page and in two fresh pages gives the same transforms and the same paths (**probed**, `mm-run.mjs mindmap`). The "fixed start for every render" rule in the P2 plan therefore has nothing to set for cose-bilkent mindmaps. It matters only if the port keeps the non-forest path (section 8).

## 5. Output

### 5.1 Nodes

After `layoutPositions`, each node's position is its rectangle centre. `render.ts:107-123` writes `transform="translate(x, y)"` on the node group and sets `node.x`, `node.y`. The group is centred on its origin, so the centre is what Mermaid draws (**probed**: the reference `translate` equals the engine centre for all seven nodes, difference 0 in every coordinate, with the measured sizes as input).

### 5.2 Edges: what Mermaid reads back

`extractPositionedEdges` (`cytoscape-setup.ts:180-207`) reads `edge._private.rscratch` (`startX`, `startY`, `midX`, `midY`, `endX`, `endY`). `render.ts:125-135` sets `edge.points = [start, mid, end]`, and `insertEdge` draws the path through those points (159-167).

- The values exist only when cytoscape ran its renderer code (section 1.3), which needs a container and `styleEnabled`. Headless Node leaves them `undefined` (**probed**), so Node cannot produce edge output. The browser can, and the values are set synchronously after `layout.run()` (**probed**, Chromium: read immediately after `run`).
- The endpoints come from the node's **style** size, not from Mermaid's size. The default style is 30 x 30 (`outerWidth()` is 30, **probed**). So each endpoint lies exactly 15 px from its node's centre, along the line between the two centres (**probed**: all six reference edges, distance 15 exactly). The start point is the ellipse entry (27575-27584), which calls `intersectLineEllipse` (4073) with `width/2`; the end point is the same on the target. `shortenIntersection` (4300) is then called with `arrow gap + source-distance-from-node` (24738, 24577), which is 0 here (**unverified** in the arrow-shape code, which was not read).
- The midpoint equals `(start + end) / 2` exactly for these six edges (**probed**, difference 0). The code path that produces it was not traced (**unverified**). It was checked only on mindmap edges, which have no parallel edges or loops.
- The reference path `d` starts at the start point and ends at the end point: edge `edge_0_1` in the reference starts at `M128.993,248.965`, the engine run in Chromium gives `startX = 128.9928744929661`, `startY = 248.96540294628318` (**probed**). The curve through `mid` is a d3 basis curve (renderer, not engine).
- The endpoints sit 15 px from the centres, inside the measured node boxes, so each edge starts under its node. Whether that hides the edge depends on the drawing order, which was not checked (**unverified**).

### 5.3 Fallback

`render.ts:173-190` draws a straight line between node centres when no positioned edge matches an edge id. For cose outputs every edge has a match, so the fallback is unreachable (unverified in general, probed for the six-edge mindmap).

## 6. Determinism and floating point

### 6.1 Where transcendental functions run

| Function                                                                                | Where                                          | Effect                                                                       |
| --------------------------------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------------------- |
| `Math.sin`, `Math.cos`                                                                  | `branchRadialLayout` (`CoSELayout.js:444-446`) | Every node's start position on a tree. Error here moves all later iterations |
| `Math.sin`                                                                              | `RandomSeed` (non-tree only)                   | The scattered start positions                                                |
| `Math.log`, `Math.pow`                                                                  | `tick` cooling (`CoSELayout.js:187`)           | The cooling factor, every 100 iterations, which scales every displacement    |
| `Math.sqrt`                                                                             | forces, lengths, diagonals                     | Correctly rounded in IEEE, so identical everywhere                           |
| `Math.floor`, `Math.ceil`, `Math.abs`, `Math.min`, `Math.max`, `parseInt`, `parseFloat` | grid, bounds, dimensions                       | Exact                                                                        |

Measured on this machine (macOS, Node 24.21 V8 against Rust 1.99 `std` on the same host), for the same input bits:

| Function             |                                                Inputs | Outputs that differ | Note                         |
| -------------------- | ----------------------------------------------------: | ------------------: | ---------------------------- |
| `sin`                | 100,360 (360 radial angles and 100,000 random angles) |        4,320 (4.3%) | one ulp in the sampled cases |
| `cos`                |                                               100,360 |        4,556 (4.5%) | one ulp                      |
| `log`                |                                                20,000 |          292 (1.5%) | one ulp                      |
| `pow` (cooling form) |                                                20,000 |                   0 | agreed in this sample only   |

These are one-ulp differences. A port that calls Rust's `f64::sin` and `f64::cos` gets about 4% of the radial start coordinates one ulp away from the reference, so its layout is not bit-identical from the first radial placement. Whether the difference grows over the iterations was not measured (**unverified**). A tolerance-based comparison would hide it; the ELK precedent (tolerance 0) would fail. See section 8 for the decision.

### 6.2 Operation order

The engine's results depend on the order of floating-point operations. The port must keep the source's expression trees, not simplify them. Examples: `LNode.scatter` computes `WORLD_CENTER_X + (r * (maxX - minX)) + minX` (`LNode.js:285-291`, the bundle has the same order at `layout-base.js:546`); `calcRepulsionForce` computes `repulsionForce * distanceX / distance` (`FDLayout.js:305-306`), which is `(repulsionForce * distanceX) / distance`; `shortenIntersection` computes `offset + lenRatio * disp` with `lenRatio = (length - amount) / length`, so even at amount 0 the rounding of `(intersection - offset) + offset` stays in the result (cytoscape 4300-4308).

### 6.3 Iteration order

- `Set` and `Map` iterate in insertion order: the `surrounding` set (`FDLayout.js:473-516`), `getFlatForest`'s `visited`, `getNeighborsList` (`LNode.js:184-206`), `remainingDegrees` in `findCenterOfTree`.
- The order of forces is the order of `getAllEdges` and `getAllNodes` (`LGraphManager.js:197-245`), so float sums come out in that order.
- `Object.keys` puts integer-like keys first, in ascending order. Only the tiling code uses it (`CoSELayout.js:579, 640, 666`). Its keys are parent ids, which are integers in a mindmap, so their order is numeric. The tiling runs only with two or more zero-degree nodes, which mindmaps do not produce.

### 6.4 Quirks that a port must reproduce

| Quirk                                                                    | Where                                 | Effect                                                                                                                                                    | Probed                                                                                                                                |
| ------------------------------------------------------------------------ | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `findCenterOfTree` splices `list` while it iterates                      | `Layout.js:624-631`                   | Wrong centre for many trees                                                                                                                               | Star: centre `x1`, not `s`; path of five: `b`, not `c`; the 7-node mindmap of section 5.2 (child of the root, node 6, not the root 0) |
| `positionNodesRadially`'s final `transform` uses the last tree's `point` | `CoSELayout.js:402-404`               | Affects only forests with more than one tree (Mermaid never gives one)                                                                                    | unverified                                                                                                                            |
| `shiftToLastRow` reads `node.width` and calls an undefined `instance`    | `CoSELayout.js:984, 993`              | Dead for layout nodes: sizes live in `rect`, so `node.width` is undefined, the test is NaN, and the branch never runs. Only a hand-set `width` reaches it | Probed: 0 hits in 4,998 calls over 300 random sets, and in full layouts; with a hand-set `width` the branch is reached (3 hits)       |
| `branchRadialLayout` uses `(end - start + 1) / 2`                        | `CoSELayout.js:433`                   | The radial angles                                                                                                                                         | read; it is what the output depends on                                                                                                |
| `Integer.MAX_VALUE` and `MIN_VALUE` as sentinels                         | `LGraph.js:251-254`, `LNode.js:20-21` | Bounds start at ±2,147,483,647                                                                                                                            | read                                                                                                                                  |
| The `pred1`, `next`, `processed` getters return undefined globals        | `CoSENode.js:90-118`                  | Unused                                                                                                                                                    | read                                                                                                                                  |
| Parallel edges are dropped before the engine sees them                   | `index.js:152`                        | Only the first edge of a pair                                                                                                                             | read                                                                                                                                  |

### 6.5 Input precision

Captured inputs and outputs must be read with exact doubles. The ELK port needed `float_roundtrip` in `serde_json` for this reason (`docs-port/elk/PLAN.md`, section 10, "JSON numbers"). The same applies here: the sizes are `getBBox` values with many digits.

## 7. How to test a port

### 7.1 Capture in the harness page

The ELK precedent: `render.ts` calls `window.__mermaidElkCapture(input, output)` after layout if the page installed it (`elk/render.ts:1298-1313`), `tools/port/geometry/common.ts` installs the hook in `pageHtml` before Mermaid loads, and `crates/ferrigura-elk/tools/capture-corpus.ts` writes the cases. Do the same for cose:

- Add `window.__mermaidCoseCapture` to `pageHtml`, next to `window.__elkCalls`.
- In `cytoscape-setup.ts`, after `cy.layout(layoutConfig).run()` (line 138) and before `resolve`, call the hook when it is a function, with a JSON copy (no change to the result when the hook is absent). Payload:

```text
{
  nodes:      [{ id, width, height }],        // addNodes order; width/height as returned by layoutDimensions
  edges:      [{ id, source, target }],       // addEdges order
  layoutOptions: { quality: 'proof', styleEnabled: false, animate: false, ...defaults in index.js 11-63 },
  positions:  { [id]: [x, y] },               // cy.nodes() order, after run
  edgePoints: { [id]: [startX, startY, midX, midY, endX, endY] }   // rscratch, after run
}
```

- Rebuild the bundle before capturing. The current `packages/mermaid/dist/mermaid.js` and its ESM chunks contain no `__mermaidElkCapture` at all (checked), so the dist predates `render.ts`'s hook. Rebuild, then check for the string.
- Capture the failing case too: a flowchart with `layout: cose-bilkent` produces the "Root node is required" error, and the port should decide whether to reproduce it.

### 7.2 The corpus

- Mindmaps: the 3 fixtures in `e2e/diagrams/mindmap/` and the inputs of the six spec files in section 1.1. Every case is a forest.
- Generated trees: a seeded generator, 1 to about 150 nodes, measured sizes from Chromium (as the reference measures them), random parent choice. Sizes come from the captured input, so text measurement is outside the engine's test.
- Non-trees: not from Mermaid (it errors). Use the Node engine with `Math.random` replaced by a recorded sequence and `RandomSeed.seed` set to a recorded value before each layout (section 4.2 counts the draws, so the sequence length is known). Compare positions only; edge output needs a browser.

### 7.3 What the Rust side compares

1. Node ids and order, and edge ids and order: exact.
2. Node centres: exact `f64` equality, tolerance 0, as in the ELK port.
3. Edge six-tuples: exact.
4. Trace (recommended, as the ELK port did): the number of ticks; `totalDisplacement` at each 100th tick; the forest's BFS lists; the centre node of each tree; the positions after `positionNodesRadially` before the spring embedder (exact); the grid cell lists at each refresh (for the grid code); the draw count of `RandomSeed` and `Math.random` (non-tree only).
5. The error case for non-mindmap usage.

### 7.4 How the probes were made

- `probe2.cjs`, `lib.cjs`, `sweep.cjs`, `tile-search.cjs`: the adapter run through real cytoscape 3.34.0 in headless Node, with the shipped `main` bundles (`cytoscape-cose-bilkent.js`, `cose-base.js`, `layout-base.js`). Checked: the draw counts and repeat behaviour of section 4.2; 400 random isolated-node sets (tiling), no error.
- `chromium-engine.mjs`: the same engine in Chromium through the UMD builds (container present, canvas renderer): node positions equal the Node results (exact, for a 3-node tree and for the 7-node mindmap with the reference's sizes), and `rscratch` is set after `run`.
- `mm-run.mjs`, `mm-bbox.mjs`, `mm-server.mjs`, `mm-page.mjs`: the reference, `packages/mermaid/dist/mermaid.esm.mjs` served over HTTP, with the Lexica fonts, rendering the mindmap and the cyclic flowchart in Chromium (per-page and cross-page checks; node `getBBox`; the `Root node is required` error).
- `cov-mindmap.cjs`, `cov-all.cjs`, `cov-map.cjs`, `cov-size.cjs`, `linecount.cjs`: V8 function coverage of the shipped bundles, mapped back to the source files by assigned name (section 1.2).
- `mathcmp.rs`: the Rust `f64` math compared with V8 on the same host (section 6.1).

## 8. Risks, effort, open questions

### 8.1 Risks

1. **Transcendentals.** `sin` and `cos` (radial start), `log` and `pow` (cooling) differ by one ulp for a few percent of inputs against Rust's libm on this host (section 6.1). A bit-exact port needs an fdlibm-equivalent of V8's `Math.sin`, `Math.cos`, `Math.log` and `Math.pow`. V8's source was not read here, so that it is fdlibm-derived is **unverified**; the port must be checked against the measured values in section 6.1. Until then, the trace will show divergence from the first radial position.
2. **Float and iteration order** (sections 6.2-6.3). Mistakes here usually show up as small differences that grow over the iterations.
3. **`findCenterOfTree`** (section 6.4). Easy to "fix" by accident. Copy it and say so in a comment.
4. **Edge endpoints** need cytoscape's ellipse intersection and `shortenIntersection`, which are not in the cose libraries. The port has to bring that subset along (about 150 to 250 lines of Rust, section 8.2).
5. **Reachability.** Only trees reach the engine through Mermaid. The non-tree path, the tiling, the compounds and the RNG code are dead for every Mermaid diagram. Porting them is a scope choice (section 8.4).
6. **Two library versions.** cose-bilkent (cose-base 1.0.3, layout-base 1.0.2) and fcose (cose-base 2.2.0, layout-base 2.0.1) are different code, not the same one. The P2 plan's Architecture row assumes one `cose-base`; section 8.4 asks for a decision.
7. **Stale bundle.** The current dist predates the source hook (section 7.1). Captures from it would not contain the hook.
8. **Headless Node cannot produce edges**, so edge references come from the browser only.

### 8.2 Effort (rough)

| Part                                                                                                                              |            JS code lines |  Rust estimate | Notes                                         |
| --------------------------------------------------------------------------------------------------------------------------------- | -----------------------: | -------------: | --------------------------------------------- |
| Mindmap path: adapter, options, graph build, forest radial placement, spring embedder, tick, geometry used by these paths, finish |              about 3,000 | 2,500 to 3,500 | Figures from section 1.2 and the coverage run |
| Endpoints: cytoscape `findEdgeControlPoints` subset for single bezier edges between 30 x 30 ellipses                              | (cytoscape, not counted) |     150 to 250 | Sections 5.2 and 1.3                          |
| Tests and capture (hook, corpus, trace comparison)                                                                                |                      n/a |     500 to 800 | Sections 7.1-7.3                              |
| Non-tree path, tiling, compounds, RNG replacement                                                                                 |              about 1,700 | 1,500 to 2,000 | Only if the scope includes them (section 8.4) |

The whole library (4,723 code lines of JS) comes to about 4,000 to 5,000 lines of Rust with everything. These are estimates, not measurements.

### 8.3 What the port must hold

For the mindmap case only: the adapter's options and graph build (section 3.1); `getFlatForest`, `findCenterOfTree` (with its quirk), `positionNodesRadially`, `radialLayout`, `branchRadialLayout`, `maxDiagonalInTree` (section 3.3); `initSpringEmbedder` and `tick` with convergence and cooling (sections 3.4-3.5); the force functions with the grid, `calcSeparationAmount` and `getIntersection2` (section 3.5); `updateBounds`, `updateLeftTop` and `transform` (section 3.6); and the endpoint geometry (section 5.2). Gravity and the tree-growing path are not needed for trees, but a port should either implement them or name them as unreachable.

### 8.4 Open questions for the coordinator

1. **Scope.** Port only the forest path now (the only one Mermaid reaches), or the whole engine (non-tree, tiling, compounds) as `ferrigura-cose` promises? The reference cannot give a differential for non-trees through Mermaid, only through Node, with the RNG replaced.
2. **Trig and log.** Port V8's `sin`, `cos`, `log` and `pow` (fdlibm) so the radial start is bit-exact, or accept a tolerance for the mindmap differential? The measured mismatch is about 4% of `sin` and `cos` inputs, so tolerance 0 is not reachable with Rust's libm.
3. **The P2 plan.** `P2-PLAN.md` says fcose is "on the same cose-base" (Architecture row) and that the force layouts "draw from the reference's generators with a fixed start" (Random draws row). Decision 0008 names the libraries without versions. Two corrections are needed: cose-bilkent uses cose-base 1.0.3 and layout-base 1.0.2, not 2.0.1; fcose uses cose-base 2.2.0 and layout-base 2.0.1 (3,073 and 5,345 source lines, against 1,418 and 4,458), and cose-base 2.2.0 calls `Math.random` (`cose-base@2.2.0/src/CoSELayout.js:424, 1960, 2011`), which Mermaid's `architectureSeed.ts` stubs. The "fixed start" rule has no effect on cose-bilkent mindmaps, because they draw nothing.
4. **Non-mindmap cose-bilkent.** Reproduce "Root node is required" for flowcharts that name `layout: cose-bilkent`, or let the registry reject it? The reference fails; a silent fallback to a different layout would not match.
5. **Capture.** Approve the hook (section 7.1) and a rebuild of `packages/mermaid/dist`. The current dist also lacks the existing ELK hook, so the ELK corpus needs the same rebuild.
6. **Dead branch.** The `instance` reference in `shiftToLastRow` is unreachable with cose's nodes. Drop it with a comment, or keep it as an unreachable branch?

### 8.5 Other notes

- `docs-port/sankey/`, `docs-port/packet/`, `docs-port/quadrant/` and `docs-port/tidy-tree/` appeared as untracked folders in this worktree during the session (the session began with a clean status). They are not mine and I did not touch them. This file is the only new file I wrote in the repository; the probe scripts are in the scratchpad.
- The mindmap's `diagramId` is `'mindmap-' + v4()` (`mindmapDb.ts:69`). It is random, but the engine never reads it, so it does not affect the layout (unverified beyond the reading, probed by identical output across renders).
