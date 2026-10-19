# Swimlanes (P3): research notes for the port

Status: research only. Nothing in the repository changed except this file, and nothing is committed. Reference: `packages/mermaid/src` at HEAD `f3a1a94b0` (branch `sebastian/port-p3`), and the built bundle `packages/mermaid/dist` from the same sources. Plan: [../P3-PLAN.md](../P3-PLAN.md). Guide: [../P2-RESEARCH.md](../P2-RESEARCH.md). Model for the layout: [../kanban/NOTES.md](../kanban/NOTES.md). The layout engine is written up in Part II (L0 to L8), as the guide asks for an engine.

Every rule cites the reference as `diagrams/<type>/<file>:<line>` or `rendering-util/...`, unless a file is named in full. **Probed** means the rule was run through the reference: in Chromium on `packages/mermaid/dist` with the harness's own page (`tools/port/geometry/common.ts`: `serve`, `pageHtml`, `patchText`, `renderReference`), or through the DDLT loader under jsdom with vite-node, or through the specs with vitest. **Unverified** means read or inferred only. Probe scripts and outputs are outside the repository in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p3-swimlanes/`: `probe-chromium.mts` (layout selection, spacing, determinism, handDrawn, lineHops, lanes, sizing), `probe-chromium2.mts` to `probe-chromium7.mts` (markers, repeat renders, the random-number callers), `probe-clock-locale.mts` (repeatability with the clock and locales), `probe-outline.mts` with `outline-fn.js` (DOM outline), `ddlt-core3.mts` (DDLT loader and swimlane core), `js-semantics.cjs` (JavaScript collation, rounding, `Math.hypot`), `schema-defaults.cjs` and `fixtures.cjs` (schema defaults and fixture front matter), and `vitest-swimlanes.json` (the spec run).

## 0. Summary

Swimlanes (`swimlane-beta`) is a flowchart with a different layout engine. The parser, the DB and the unified renderer are the flowchart's, which the port already has. The P3 plan sizes this type at 63 source lines and 2 spec files. That is the diagram folder only (`detector.ts` 24, `swimlanesDiagram.ts` 11, `styles.ts` 28 lines). The layout engine is much larger: **11,684 lines** of non-spec TypeScript under `rendering-util/layout-algorithms/swimlanes/` (the orthogonal router alone is 2,283 lines, the geometry passes in `direction/` 6,243), and it has **50 spec files**. The P3 plan needs that correction before the size is fixed.

What swimlanes adds on top of the flowchart (all probed or read, with the lines in section 1):

| Piece                                                                                                         | Where                                                                                | Size (lines)     |
| ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------- |
| Detector: one regex, `^\s*swimlane-beta\b`                                                                    | `diagrams/swimlanes/detector.ts:8`                                                   | 24               |
| Diagram: the flowchart factory with its own styles                                                            | `diagrams/swimlanes/swimlanesDiagram.ts:11`                                          | 11               |
| Stylesheet: the flowchart's, plus one lane rule                                                               | `diagrams/swimlanes/styles.ts:21`                                                    | 28               |
| Layout engine `swimlane`, its own Sugiyama-style pipeline, orthogonal router and geometry passes. **No ELK.** | `rendering-util/layout-algorithms/swimlanes/`                                        | 11,684           |
| Lane shape, the `swimlane` cluster                                                                            | `rendering-elements/clusters/swimlane.js`                                            | 246              |
| Edge clipping and the `-to-label` helper, swimlane branch only                                                | `rendering-elements/edges.js:379-400`, `654-700`                                     | about 70         |
| Line hops after paint (`lineHops`), shared with the flowchart's line-jump code                                | `layout-algorithms/swimlanes/adjustLayout.ts:5-30`, `rendering-elements/lineJump.ts` | 30 (+759 shared) |
| Layout name `swimlane` in the registry                                                                        | `rendering-util/render.ts:68-70`                                                     | 3                |
| Configuration section `swimlane` (five keys plus the `theme`, `look`, `layout` defaults)                      | `schemas/config.schema.yaml:2315-2370`, `config.ts:12-107`                           | 60 (schema)      |

**Probed (Chromium, reference bundle):** a `swimlane-beta` diagram with the default configuration makes **zero** ELK calls, and the same text with `layout: elk` makes one. The flowchart control makes one. The lanes are rendered as `g.cluster.swimlane`. So the layout is not ELK, and the port needs no elkjs.

The riskiest parts, in order (details in section 8):

1. The router and geometry passes (about 9,000 lines of TypeScript that decide edge shapes). They depend on floating-point details: JavaScript's `toFixed` on exact ties, `Math.hypot` in A\* and in length sums, and sort stability.
2. The reference is not repeatable unless `Math.random` is seeded. Rough.js draws random offsets through `Math.random` even for shapes with roughness 0 (`stadium`, `trapezoidalPentagon`, and `question`, probed by call stacks). **Probed:** 18 of the 46 fixtures give a different SVG on every render in one page. With the harness clock (seeded `Math.random` and a fixed `Date`), all 46 repeat exactly.
3. Collation: 18 `localeCompare` call sites in 10 layout files decide the order of nodes and edges. **Probed:** ICU order differs from code-point order for mixed case and punctuation.
4. The Rust config model has no `swimlane` section (`crates/ferrigura-model/src/config/ts_defaults.json` and the model have no `swimlane` entries). The port must add it, with the per-diagram appearance layer of `config.ts:79-107`, or `layout: swimlane` never takes effect.

## 1. Sources

### 1.1 The diagram (`packages/mermaid/src/diagrams/swimlanes/`)

| File                       | Lines | What it holds                                                                                                                                                                                    |
| -------------------------- | ----: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `detector.ts`              |  1-24 | `/^\s*swimlane-beta\b/` (line 8); the loader imports `swimlanesDiagram.js`.                                                                                                                      |
| `swimlanesDiagram.ts`      |  1-11 | `createFlowDiagram({ styles: swimlanesStyles })`. The comment names the flowchart's public factory as the one sanctioned cross-diagram dependency.                                               |
| `styles.ts`                |  1-28 | `getFlowchartStyles(options)` plus `.swimlane.cluster:not([data-color-id]) rect { stroke: clusterBorder !important }` (line 21) and `[data-look="neo"].cluster rect { filter: none }` (line 24). |
| `swimlanesDiagram.spec.ts` |    63 | 5 tests.                                                                                                                                                                                         |
| `lanePalette.spec.ts`      |   114 | 42 tests (palette and lane colours).                                                                                                                                                             |

The detector is only reached when the text starts with `swimlane-beta`. The flowchart detector is `/^\s*(graph|flowchart)/` (`diagrams/flowchart/flowDetector-v2.ts:11`), so the two do not overlap. The loader is registered in `diagram-api/diagram-orchestration.ts:4` and `:98`.

### 1.2 The reused flowchart (already ported)

| File                                            |     Lines | What swimlanes takes from it                                                                                                                                                                                             |
| ----------------------------------------------- | --------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `diagrams/flowchart/flowDiagram.ts`             |     15-35 | `createFlowDiagram({ styles })`; the `init` writes `flowchart.arrowMarkerAbsolute` (lines 30-31).                                                                                                                        |
| `diagrams/flowchart/parser/flow.jison`          |       116 | `"swimlane-beta"` is a `GRAPH` keyword with the same `firstGraph` action as `graph` and `flowchart`.                                                                                                                     |
| `diagrams/flowchart/flowDb.ts`                  | 1119-1347 | `getData()` returns `{ nodes, edges, other, config }` (line 1346) with `config = getConfig()` (line 1120). Nothing in `flowDb.ts` mentions swimlanes.                                                                    |
| `diagrams/flowchart/flowRenderer-v3-unified.ts` |     17-53 | `draw`: reads `securityLevel`, `flowchart`, `layout` (line 20); sets `layoutAlgorithm`, `nodeSpacing`, `rankSpacing`, `markers` (lines 35-39); calls `render`; inserts the title; calls `setupViewPortForSVG` (line 51). |
| `diagrams/flowchart/styles.ts`                  |     1-298 | The theme-variable consumer for every shape and edge (section 6.6). `genColor` (49-127) emits the palette rules, and `getStyles` (141-296) the rest.                                                                     |

### 1.3 The layout engine (`packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/`)

Line counts are the non-spec files. Default path means the code reached with the default configuration (`ignoreCrossLaneEdges: true`, `automaticLaneOrdering: false`); section 3 explains the split.

| File                          | Lines | Path        | Role                                                                                                                                                                                                                                                                                                                                             |
| ----------------------------- | ----: | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `index.ts`                    |    20 | default     | `render = createCommonLayoutRenderer({ prepareLayout, runLayoutCore, afterPaint })` (lines 14-20).                                                                                                                                                                                                                                               |
| `layoutCore.ts`               |    58 | default     | `runSwimlaneLayoutCore(data)` (lines 22-58): reads the configuration (22-26), runs `sugiyamaLayout`, writes back, deletes edge points, routes, sets curves to `rounded` (37-41), post-processes, validates. The DOM-free seam.                                                                                                                   |
| `helpers.ts`                  |   356 | default     | `prepareLayoutForSwimlanes` (98-140), `toGraphView` (142-172), `writeBackToLayoutData` (174-356, with the lane width solver at 305-352).                                                                                                                                                                                                         |
| `edgeLabelNodes.ts`           |   132 | default     | `createEdgeLabelNodes` (38-131).                                                                                                                                                                                                                                                                                                                 |
| `pipeline.ts`                 |    77 | default     | `sugiyamaLayout` (24-77): the phases in order.                                                                                                                                                                                                                                                                                                   |
| `phase0.helpers.ts`           |   222 | default     | Graph helpers: `normalizeGraph`, `topoSortIfAcyclic`, `sortedZeroInDegreeNodes`, `countInversions`.                                                                                                                                                                                                                                              |
| `phase1.cycles.ts`            |    81 | default     | `removeCycles_DFS`.                                                                                                                                                                                                                                                                                                                              |
| `phase2.laneAwareCompact.ts`  |    88 | default     | `assignLayers_LaneAwareCompact`, the default layering.                                                                                                                                                                                                                                                                                           |
| `phase2.dummies.ts`           |    71 | default     | `makeProperLayering`, dummy nodes for long edges.                                                                                                                                                                                                                                                                                                |
| `phase2.options.ts`           |    90 | default     | Top-lane resolver and lane order.                                                                                                                                                                                                                                                                                                                |
| `phase3.ordering.ts`          |   279 | default     | `orderLayers`: median sweeps and transposition.                                                                                                                                                                                                                                                                                                  |
| `phase4.coordinates.ts`       |   192 | default     | `assignCoordinates`.                                                                                                                                                                                                                                                                                                                             |
| `config.ts`                   |    37 | default     | Constants: `PRECISION.EPSILON` 1e-6; `LAYERING.GRAVITY_ITERATIONS` 8, `MAX_CROSSING_OPTIMIZATION_PASSES` 4, `DEFAULT_COMPACT_SINGLE_INPUT` true; `COORDINATES.DEFAULT_LAYER_GAP` 100, `DEFAULT_NODE_GAP` 40.                                                                                                                                     |
| `adjustLayout.ts`             |    30 | default     | `applySwimlaneLineJumps` (lines 5-30).                                                                                                                                                                                                                                                                                                           |
| `postProcessing.ts`           |   155 | default     | `postProcessSwimlaneLayout` (29-155): the chain of geometry passes (section 3.7).                                                                                                                                                                                                                                                                |
| `orthogonalRouter/router.ts`  | 2,283 | default     | `routeEdgesOrthogonal` (line 122, the only export; the body runs to the end of the file).                                                                                                                                                                                                                                                        |
| `direction/*.ts` (11 files)   | 6,243 | default     | Post-processing passes, called from `postProcessing.ts`: `materializedGeometry.ts` 2,814; `geometry.ts` 688; `endpointClip.ts` 469; `labelAnchoring.ts` 459; `sharedTrackNudging.ts` 321; `lrTransform.ts` 327; `detourSimplification.ts` 309; `terminalStub.ts` 259; `portSwap.ts` 241; `siblingSharedFaceRouting.ts` 197; `validation.ts` 159. |
| `phase2.gravity.ts`           |   122 | non-default | `assignLayers_Gravity`, used when `ignoreCrossLaneEdges` is false.                                                                                                                                                                                                                                                                               |
| `phase2.longestPath.ts`       |    65 | non-default | `assignLayers_LongestPath`, the base of gravity.                                                                                                                                                                                                                                                                                                 |
| `phase2.crossOptimization.ts` |   105 | non-default | `optimizeRanksByCrossings`.                                                                                                                                                                                                                                                                                                                      |
| `phase2.crossLaneAdjust.ts`   |    70 | non-default | `adjustCrossLaneSources`.                                                                                                                                                                                                                                                                                                                        |
| `phase2.crossCounts.ts`       |   212 | non-default | Subtree cross counts with binary lifting.                                                                                                                                                                                                                                                                                                        |
| `driving-tree.ts`             |   277 | non-default | Driving tree (spanning forest and blocks).                                                                                                                                                                                                                                                                                                       |
| `phase2.multitree.core.ts`    |   116 | non-default | Helpers for the multitree order.                                                                                                                                                                                                                                                                                                                 |
| `phase2.multitree.order.ts`   |   107 | non-default | `buildMultitreeLayerOrder`.                                                                                                                                                                                                                                                                                                                      |
| `laneOrdering.ts`             |   196 | non-default | `automaticLaneOrdering`: FNV-1a seeds, mulberry32 shuffles, greedy switching.                                                                                                                                                                                                                                                                    |

The default path adds up to 10,414 lines and the non-default files to 1,270 (sum 11,684). The non-default code is reached only through the configuration. Two fixtures use it (section 7).

Specs: 50 files under the folder: `__tests__/` 28, `orthogonalRouter/__tests__/` 6, `direction/geometry.spec.ts` 1, and 15 at the top level (14 DDLT fixture specs and `phase2.layering.spec.ts`). Section 5 has the run.

### 1.4 The renderer parts that swimlanes touches (shared code)

| File                                                     |                                   Lines | Used for                                                                                                                                                                                        |
| -------------------------------------------------------- | --------------------------------------: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rendering-util/render.ts`                               |             65-79, 81-104, 106, 113-124 | Registers `swimlane` (68-70); `render` (81) looks up the layout and calls its `render` (`LayoutAlgorithm.render`); `LAST_RESORT_LAYOUT = 'elk'` (106); `getRegisteredLayoutAlgorithm` (113).    |
| `rendering-util/layout-algorithms/common/index.ts`       |               107-204, 220-245, 328-351 | `createCommonLayoutRenderer`: insert markers, prepare, measure, run the core, paint (`paintLayoutData`), `afterPaint`. The edge label and terminal label positions.                             |
| `rendering-util/createGraph.ts`                          |                          42-71, 122-215 | `createGraphWithElements`: measures every node and label (section 6.3).                                                                                                                         |
| `rendering-util/rendering-elements/clusters.js`          |                        806-815, 841-846 | The cluster shape table (`swimlane` at line 815) and `insertCluster`. Nested subgraphs take `rect` (lines 15-143).                                                                              |
| `rendering-util/rendering-elements/clusters/swimlane.js` |                                  16-246 | The lane shape (section 6.5).                                                                                                                                                                   |
| `rendering-util/rendering-elements/edges.js`             | 44, 379-400, 624-700, 705, 899, 934-941 | `insertEdge` (624): the `layoutAlgorithm` argument (line 640) picks the swimlane clipping (654-665); `btoa` of the points as `data-points` (705, 899); the arrow marker absolute URL (934-941). |
| `rendering-util/rendering-elements/lineJump.ts`          |                                   1-759 | `applyLineJumpsToSvg` (shared with the flowchart's line jumps).                                                                                                                                 |
| `rendering-util/rendering-elements/markers.js`           |                 6-11, 323-533, 997-1022 | `insertMarkers`. The flowchart passes `point`, `circle` and `cross`.                                                                                                                            |
| `rendering-util/insertLookDefs.ts`                       |                                   25-26 | `theme` and gradient variables for the look definitions.                                                                                                                                        |
| `styles.ts` (root)                                       |                                  11-160 | The style sheet around the diagram's rules.                                                                                                                                                     |
| `mermaidAPI.ts`                                          |                   83, 114, 327-330, 689 | `setDiagramConfigScope(diagramType)` (line 83), which makes `swimlane.*` defaults apply. The stylesheet options are the resolved `themeVariables` plus `theme` and `look` (327-330).            |

### 1.5 Configuration and detection plumbing

| File                               |                                Lines | What it does                                                                                                                                                                                                                                                        |
| ---------------------------------- | -----------------------------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `config.ts`                        | 12-16, 27-35, 42-54, 79-107, 149-152 | `APPEARANCE_KEYS = ['theme', 'look', 'layout']` (12). `resolveAppearance` (79-107) takes the first of: frontmatter and directives, `initialize`, the diagram's own section, then the global default. `setDiagramConfigScope` (149-152) names the diagram's section. |
| `diagram-api/diagramConfigKeys.ts` |                                 1-20 | Maps a diagram id to its config section. `swimlane` has no alias, so the section is `swimlane`.                                                                                                                                                                     |
| `schemas/config.schema.yaml`       |                   376-377, 2315-2370 | `swimlane:` (376); `SwimlaneDiagramConfig` (2315-2370): `theme` redux-color, `look` neo, `layout` swimlane, `lineHops` arc, `ignoreCrossLaneEdges` true, `optimizeRanksByCrossings` true, `automaticLaneOrdering` false.                                            |
| `config.type.ts`                   |                     319-320, 519-580 | The TypeScript type of the section.                                                                                                                                                                                                                                 |

### 1.6 DDLT (the layout test harness)

| File                                                                          |               Lines | Use                                                                                                                                                                                                                                                                                                           |
| ----------------------------------------------------------------------------- | ------------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rendering-util/layout-algorithms/ddlt/backends.ts`                           | 23-28, 46-67, 69-84 | `topUpSwimlaneFlowchartConfig` (23-28) sets `flowchart.nodeSpacing` 40 and `rankSpacing` 100 when they are absent. `runSwimlanesDdlt` (46) prepares the lanes, applies the captured sizes, creates the label nodes and runs the core. `parseApplySizesAndLayout` (69) parses with `parseMmdFileToLayoutData`. |
| `ddlt/parseToLayoutData.ts`                                                   |                1-40 | `Diagram.fromText` and `getData()`; the layout gets the full configuration with the schema defaults.                                                                                                                                                                                                          |
| `ddlt/loadDdltFixture.ts`, `ddlt/fixtureSizes.ts`, `ddlt/discoverFixtures.ts` |                     | Load a fixture with its captured sizes (`applyFixtureContentSizesStrict`, `applyFixtureLabelSizesStrict`).                                                                                                                                                                                                    |

### 1.7 Fixtures

`e2e/diagrams/swimlanes/` holds 46 `.mmd` files (section 7). `e2e/platform/dev-diagrams/layout-tests/swimlanes/` holds 31 `.mmd` files and 28 `.sizes.json` files, the captured node and label sizes that the DDLT specs use.

## 2. Parser

Swimlanes has no grammar of its own. The text is parsed by the flowchart's `flow.jison` (`parser/flowParser.ts`), so every flowchart construct works in a swimlane, and the keyword is one more `GRAPH` token.

- The keyword `swimlane-beta` (`parser/flow.jison:116`) acts like `graph` and `flowchart` (line 117 is `graph`, line 118 is `flowchart`): on the first graph line it starts the `dir` state, and the direction (`LR`, `TB`, `BT`, `RL`) is read there.
- The ferrigura parser already accepts the keyword. `crates/ferrigura-syntax/src/flowchart/scanner.rs:809-812` lists `swimlane-beta`, and `grammar.rs:176` names it in the error message. **Unverified:** that the Rust parser covers every swimlane fixture. The flowchart parser's own spec, `flow.spec.js`, mentions the keyword (its file has 22 `it(` cases in all, not all about swimlanes; not run).
- A lane is a `subgraph` (`parser/flow.jison` 'subgraph' token, and the `end` token) with no parent. The layout turns every top-level group into a lane (section 3.1), so the parser needs no lane concept.
- A node outside every subgraph is a "loose" node. The layout puts it in a synthesised lane `__swimlane_default__` (`helpers.ts:45`, `110-139`). That lane has padding 20 and `colorIndex` one more than the largest, and its `look` comes from the configuration.

Quirks to reproduce: none specific to the parser. The keyword behaves exactly like `graph` except that the detector and the layout differ.

## 3. DB

The DB is the flowchart's `FlowDB` (`diagrams/flowchart/flowDb.ts`), unchanged. The swimlane path reads the following from it, with the flowchart's defaults from `config.schema.yaml`. These defaults were read from the schema with js-yaml (`schema-defaults.cjs`), and they match the probe.

### 3.1 What `getData` gives the layout

| Field           | Set at                                   | Swimlane effect                                                                                                                             | Default (schema)          |
| --------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| `config`        | `flowDb.ts:1120`, `1346`                 | The whole configuration, schema defaults included. The layout reads `flowchart.nodeSpacing`, `rankSpacing`, the `swimlane` keys and `look`. |                           |
| node `look`     | `flowDb.ts:1251`, `1269`, `1287`, `1337` | `config.look` (default `neo` for a swimlane through `swimlane.look`).                                                                       | `neo` (section 6.7).      |
| node `padding`  | `flowDb.ts:1071`                         | `config.flowchart?.padding \|\| 8`.                                                                                                         | `flowchart.padding` = 15. |
| node `minWidth` | `flowDb.ts:1099`                         | `config.flowchart?.minNodeWidth`.                                                                                                           | 120.                      |
| edge `curve`    | `flowDb.ts:1340`                         | `interpolate` or `config.flowchart?.curve`. The layout later sets `rounded` (`layoutCore.ts:37-41`).                                        | `basis`, then `rounded`.  |
| edge limit      | `flowDb.ts:330`                          | `maxEdges` (500).                                                                                                                           | 500.                      |
| direction       | `flowDb.ts:721-725`                      | `flowchart.inheritDir` decides whether a subgraph's direction applies.                                                                      | `inheritDir` false.       |

`getData` builds the lanes through `addNodeFromVertex`, which emits subgraphs as nodes with `isGroup: true` (`flowDb.ts:1088-1091`). The layout then assigns `shape: swimlane` to top-level groups (`helpers.ts:98-108`).

### 3.2 Sanitising

The text of nodes, edges and lanes goes through the flowchart's `sanitizeText` (`common/common.ts`), as in the flowchart. `securityLevel` matters for `click` (`flowDb.ts:500`), and the renderer reads it (section 6.9). No swimlane-specific sanitising exists (checked: `flowDb.ts` has no swimlane string).

## 4. Canonical model proposal

Use the flowchart's model for the DB side (the flowchart notes own it). Swimlanes needs a second model at the layout boundary, which is the differential seam, and it is the one to build:

- **Input** (per fixture, after the sizes are applied): `direction`; `config` (the keys of section 6.7 and the flowchart's `nodeSpacing`, `rankSpacing`); `nodes` with `id`, `parentId`, `isGroup`, `shape`, `width`, `height`, `padding`, `colorIndex`, `look`, `dir`, `isEdgeLabel`, `isDummy`, `labelStyle`; `edges` with `id`, `start`, `end`, `label` (present or cleared), `labelNodeId`, `isLayoutOnly`, `curve`, `arrowTypeStart`, `arrowTypeEnd`.
- **Output** (what the reference hands back): node `x`, `y`, `width`, `height`, `layer`, `order`; lane `x`, `y`, `width`, `height`, `swimlaneContentTop`, `groupTitleRect`; edge `points` (after routing and post-processing), `curve`, label node positions.

Left out: the SVG (differences in the path string are caught by the geometry harness, not by the model), the `labelBBox` and `diff` values that only the renderer reads, and the `data-points` attribute, which is derived from `points` (`edges.js:705`).

## 5. Specs

Run: `npx vitest run` on `packages/mermaid/src/diagrams/swimlanes`, `packages/mermaid/src/rendering-util/layout-algorithms/swimlanes`, `.../ddlt`, `.../layout-utils`, and `rendering-elements/lineJump.integration.spec.ts`, with the JSON reporter. **Probed:** 314 tests in 61 files; **311 passed, 3 skipped, 0 failed.** The skipped ones are `postProcessing.spec.ts` ("continues reroute scanning against current edge points after each fix") and two cases in `orthogonalRouter/__tests__/raykov.router.spec.ts` ("Edge label obstacle avoidance").

| Spec group                                                                     | Files | Tests run | Tests the port                                                                                                                                                               |
| ------------------------------------------------------------------------------ | ----: | --------: | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/swimlanes/`                                                          |     2 |        47 | Detection and the styles (`swimlanesDiagram.spec.ts`), the lane palette (`lanePalette.spec.ts`).                                                                             |
| `layout-algorithms/swimlanes/` (top level)                                     |    15 |        52 | The 14 DDLT fixture specs (50 tests: the layout of the captured sizes, with the validation score; the differential inputs of section 7.2) and `phase2.layering.spec.ts` (2). |
| `layout-algorithms/swimlanes/__tests__/`                                       |    28 |       101 | Layering, cycles, lane titles, `helpers.prepareLayout`, the phases, `direction.*`, `laneOrdering`, `pipeline.*`, `postProcessing`. Port: one group per phase.                |
| `layout-algorithms/swimlanes/orthogonalRouter/__tests__/`                      |     6 |        30 | The router (`raykov.*`): crossings, detours, wide nodes, issues. Port: the router.                                                                                           |
| `layout-algorithms/swimlanes/direction/`                                       |     1 |        12 | The geometry helpers (`geometry.spec.ts`).                                                                                                                                   |
| `layout-algorithms/ddlt/`                                                      |     5 |        13 | The loader, the sizes and the fixture freshness. Port: none (the test harness).                                                                                              |
| `layout-algorithms/layout-utils/` (`validateLayout`, `scoreLayout`, `helpers`) |     3 |        58 | The reference's validator and scorer. Shared, not swimlane code.                                                                                                             |
| `rendering-elements/lineJump.integration.spec.ts`                              |     1 |         1 | The line jumps on a swimlane fixture.                                                                                                                                        |

The total is 314 tests.

Shared spec files that mention swimlanes and were **not** run (file-level static counts of `it(`, which undercount `it.each` and include cases that are not about swimlanes): `diagrams/flowchart/parser/flow.spec.js` (22), `mermaidAPI.spec.ts` (56), `config.appearance.spec.ts` (30), `config.spec.ts` (23), `diagram-api/diagram-orchestration.spec.ts` (9), `rendering-util/rendering-elements/edges.spec.js` (14), `rendering-util/rendering-elements/lineJump.spec.ts` (28), `rendering-util/layoutFallback.spec.ts` (7), `rendering-util/render.spec.ts` (6), `rendering-util/multi-diagram-id-uniqueness.spec.ts` (5), `diagrams/common/colorThemeGate.spec.ts` (26), `diagrams/flowchart/flowDiagram.spec.ts` (3), `defaultAppearanceDocs.spec.ts` (1). These belong to the configuration, the flowchart and the line jumps, and they are the coordinator's to schedule.

## 6. Renderer

### 6.1 Pipeline, in order

1. `draw` (`flowRenderer-v3-unified.ts:17`) reads `securityLevel`, `flowchart` and `layout` (line 20). It takes `data4Layout = db.getData()` (flowchart) and sets `layoutAlgorithm = getRegisteredLayoutAlgorithm(layout)` (line 35), `nodeSpacing = conf?.nodeSpacing || 50` (37), `rankSpacing = conf?.rankSpacing || 50` (38), `markers = ['point', 'circle', 'cross']` (39), `diagramId` (41).
2. `render` (`render.ts:81`) checks the algorithm name is registered and calls the loader of `swimlane` (`layout-algorithms/swimlanes/index.ts`).
3. The common renderer (`common/index.ts:129-204`):
   1. `insertMarkers(element, markers, type, diagramId)` (line 141): the root `g` gets the markers.
   2. `clearLayoutRenderState` (147): clears the node, edge and cluster maps.
   3. `prepareLayout` (159) = `prepareLayoutForSwimlanes`, then `createEdgeLabelNodes` (`index.ts:8-12`).
   4. `measureLayout` (168) = `createGraphWithElements` (`createGraph.ts:122-215`): inserts every node, cluster label and edge label into the DOM and measures them (section 6.3).
   5. `runLayoutCore` (175) = `runSwimlaneLayoutCore`.
   6. `paintLayoutData` (191, `common/index.ts:220-245`): `positionNode` for each node; `insertCluster` for each group (`isGroup`); `insertEdge` for each edge (skipping `isLayoutOnly`, `common/index.ts:280-282`); `insertEdgeLabel` for labelled edges.
   7. `afterPaint` (199) = `applySwimlaneLineJumps` (`adjustLayout.ts:5-30`).
4. Back in `draw`: `insertTitle` (`flowRenderer-v3-unified.ts:45-49`), then `setupViewPortForSVG(svg, diagramPadding ?? 8, 'flowchart', useMaxWidth)` (lines 44 and 51).

### 6.2 DOM order (probed)

The outline below is from `pr-check/1-simple.mmd`, a two-lane LR chain, rendered in Chromium with the harness's settings and the clock. The tree was printed in document order with `probe-outline.mts`.

```
svg.flowchart                       (aria-roledescription="swimlane", role="graphics-document document")
  style                             (the stylesheet, section 6.6)
  g                                 (the marker group: 12 markers, class "marker swimlane" or "marker cross swimlane")
  g.root
    g.clusters
      g.cluster.swimlane            data-look="neo" data-id="Dep2" data-et="cluster"
      g.cluster.swimlane            data-look="neo" data-id="Dep1" data-et="cluster"
    g.edges.edgePaths
      path.edge-thickness-normal.edge-pattern-solid.flowchart-link   data-id="L_Start_Process1_0" data-et="edge" marker-end
      path ...                      data-id="L_Process1_Process2_0"
      path ...                      data-id="L_Process2_Process3_0"
    g.edgeLabels                    (empty here)
    g.nodes
      g.node.default                data-look="neo"  (4 nodes)
  defs > filter                     (two filter definitions)
```

Observed: the lanes are inserted in reverse source order (`Dep2`, then `Dep1`, though the source lists `Dep1` first). The layout reverses the group order in `toGraphView` (`helpers.ts:169`) and in `buildTopLaneOrder` (`phase2.options.ts:64`), but the paint loop walks `data4Layout.nodes` (`common/index.ts:229`), so the reversal must come earlier, in the node list from the DB. **Unverified** which step does it. The port has to reproduce the order of the reference's SVG, not the source order.

### 6.3 Measurements (every `getBBox` and friends)

| Call                                                                                    | Where                                              | Feeds                                                                                                                                                                                    |
| --------------------------------------------------------------------------------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `getBBox()` on each inserted non-group node (and each label node, which is a non-group) | `createGraph.ts:67-69`                             | `node.width` and `node.height` for the layout (set by `insertMeasuredNode`).                                                                                                             |
| Group label: `labelHelper` then its `bbox`, wrapped at `flowchart.wrappingWidth` (120)  | `createGraph.ts:42-59`; `shapes/util.ts:98`, `179` | `node.labelBBox`. The swimlane's measurement is wrapped (`unwrapGroupLabels` is not set by the swimlane renderer, `createGraph.ts:84-93`).                                               |
| `getBBox()` of the lane's label text (`createText`)                                     | `clusters/swimlane.js:45-53`                       | The lane width (`node.width` against `bbox.width + padding`, lines 63-69) and `node.offsetY` (line 239).                                                                                 |
| `getBoundingClientRect()` of the label div (only with `htmlLabels`)                     | `clusters/swimlane.js:55-61`                       | Not exercised by the fixtures (all 46 set `htmlLabels: false`).                                                                                                                          |
| `getComputedTextLength()` of a test span, in `computeWidthOfText`                       | `createText.ts:114-127`, the call at line 122      | The width of a line of text. The caller was not read, so the use is **unverified**.                                                                                                      |
| `getBBox()` of the SVG text element                                                     | `createText.ts:186-190`                            | The label size of `createText` (SVG labels).                                                                                                                                             |
| `getBBox()` of the root `svg`                                                           | `setupViewPortForSVG.ts:29`                        | The root `viewBox` and the `width` or `max-width` (section 6.10).                                                                                                                        |
| `getTotalLength()` of each path                                                         | `lineJump.ts:748-750`, `edges.js:879`              | The line-hop geometry, and the dash array of animated edges (`edges.js:879`, **unverified** which edges use it).                                                                         |
| `getBoundingClientRect()` and `getBBox()` of edge labels                                | `edges.js:114-124`, `152-239`                      | Edge labels and terminal labels. Edge labels become label nodes (section 3.1), so only the terminal labels remain here. The fixtures use no terminal labels (**unverified** by a probe). |

The layout's own text constant: "Captured swimlane fixtures use 21px as the stable single-line label height" (`helpers.ts:52`, `TOP_LANE_TITLE_BAND_HEIGHT = 21`, and the minimum lane padding of 20 at line 54).

### 6.4 Edges (`insertEdge`, `edges.js:624`)

- The `layoutAlgorithm` argument (`common/index.ts:301`) decides the branch. For the swimlane layout the branch is at `edges.js:658`: a straight two-point edge is clipped to both nodes' boundaries (`tail.intersect` and `head.intersect`, line 668). A longer edge keeps its inner bends, and only the first and last segments are clipped, with `TOLERANCE` 0.5 (line 675). The clipping of other layouts (`edges.js:699-704`) is not used for swimlanes.
- `orthogonalizeToLabelClippedPoints` (`edges.js:379-400`) is a no-op except for the `-to-label` edges. Those are layout-only and are never rendered, so in practice it does nothing.
- The points are written as `btoa(JSON.stringify(points))` into `data-points` (`edges.js:705`, `899`). A port must emit the same attribute if the comparison covers it.
- The edge curve is `rounded` (set by `layoutCore.ts:37-41` when the edge had `basis` or nothing). The rounded path is `generateRoundedPath` (`edges.js:968`).
- The `marker-end` URL is `url(#<diagramId>_<type>-pointEnd...)`, and the `arrowMarkerAbsolute` branch (`edges.js:934-941`) adds the page URL. **Probed:** with the `neo` look and with `classic`, setting `state.arrowMarkerAbsolute` or `flowchart.arrowMarkerAbsolute` left the marker URL as `url(#...)`. The branch is read but had no visible effect in the probe. The port should still read the key (`edges.js:934` reads `getConfig().state.arrowMarkerAbsolute` on a swimlane path, which is a cross-diagram read).

### 6.5 The lane shape (`clusters/swimlane.js`, `swimlane` at line 16)

- The lane is two rectangles: `swimlane-title` (the header band) and `swimlane-body` (the content), with class `cluster swimlane` on the group (line 27).
- TB (`isLR` false, lines 157-224): the title band is `min(bbox.height + 0, contentTop - laneTop)` high (`titlePaddingY` 0 for TB, lines 86, 159-160); the body starts where the title ends (`bodyY = laneTop + titleHeight`, line 166); the label is centred in the band (lines 221-223).
- LR (lines 92-156): the title is a vertical strip on the left of width `bbox.height + 2 * 4` (lines 86, 95); the label is rotated `-90` degrees (line 155).
- `contentTop` is the lane's `swimlaneContentTop` from the layout (line 78), or `laneTop + height / 3`.
- `node.diff`, `node.offsetY`, `node.width`, `node.height` are written back (lines 66-69, 236-239).
- The `handDrawn` look (`node.look === 'handDrawn'`, lines 99 and 171) draws with rough.js and `handDrawnSeed` (lines 107, 113, 179, 185). Section 6.8.

### 6.6 Stylesheet and theme variables read

The style sheet is built in `mermaidAPI.ts:327-330` from `{ ...config.themeVariables, theme, look }` and passed to `styles.ts` (root), which adds the diagram's own rules. The swimlane sheet is the flowchart's (`diagrams/flowchart/styles.ts`) plus `diagrams/swimlanes/styles.ts`.

Theme variables read on the swimlane path, with where they are read. "Shared" means the flowchart reads it too, so the flowchart port already has it.

| Variable                                                 | Read at                                                                                                                     | Notes                                                                                                                                                                                         |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fontFamily`                                             | `styles.ts:43`, `108`, `flowchart/styles.ts:144`, `262`                                                                     | Shared.                                                                                                                                                                                       |
| `fontSize`                                               | `styles.ts:44`, `109`                                                                                                       | Shared.                                                                                                                                                                                       |
| `textColor`                                              | `styles.ts:45`; `flowchart/styles.ts:145-146`, `158-159`, `274`                                                             | Shared. The fallback `nodeTextColor \|\| textColor`.                                                                                                                                          |
| `nodeTextColor`                                          | `flowchart/styles.ts:145`, `158-159`                                                                                        | Shared.                                                                                                                                                                                       |
| `titleColor`                                             | `flowchart/styles.ts:148`, `151`, `235`, `239`                                                                              | The lane title text (`.cluster-label`).                                                                                                                                                       |
| `errorBkgColor`, `errorTextColor`                        | `styles.ts:72`, `75-76`                                                                                                     | Shared; the error icon rules.                                                                                                                                                                 |
| `lineColor`                                              | `styles.ts:100-104`; `flowchart/styles.ts:190-192`, `200`, `205`                                                            | Shared; edges and markers.                                                                                                                                                                    |
| `arrowheadColor`                                         | `flowchart/styles.ts:196`                                                                                                   | Shared.                                                                                                                                                                                       |
| `strokeWidth`                                            | `styles.ts:80`, `131`; `flowchart/styles.ts:169`, `201`                                                                     | Shared; default 1 for nodes, 2 for edges (`flowchart/styles.ts:201`).                                                                                                                         |
| `nodeBorder`                                             | `styles.ts:117` and others; `flowchart/styles.ts:168`                                                                       | Shared.                                                                                                                                                                                       |
| `mainBkg`                                                | `flowchart/styles.ts:167`                                                                                                   | Shared; the node fill.                                                                                                                                                                        |
| `edgeLabelBackground`                                    | `flowchart/styles.ts:210-217`, `224`, `283-291`                                                                             | Shared.                                                                                                                                                                                       |
| `clusterBkg`                                             | `flowchart/styles.ts:229`; `clusters/swimlane.js:19`, `104`, `135`, `176`, `206`                                            | The lane fill.                                                                                                                                                                                |
| `clusterBorder`                                          | `flowchart/styles.ts:230`, `247`, `253`; `swimlanes/styles.ts:21`; `clusters/swimlane.js:19-20`, `136`, `146`, `207`, `217` | The lane stroke, and the rule that overrides the generic cluster border.                                                                                                                      |
| `tertiaryColor`, `border2`                               | `flowchart/styles.ts:264`, `265`                                                                                            | The tooltip. Shared.                                                                                                                                                                          |
| `dropShadow`, `useGradient`                              | `styles.ts:121-122`, `135`                                                                                                  | Shared.                                                                                                                                                                                       |
| `borderColorArray`, `bkgColorArray`, `THEME_COLOR_LIMIT` | `flowchart/styles.ts:50-58`; `clusters/swimlane.js:19`, `35` (via `stampColorSlot`)                                         | The palette. Only with the redux-color and redux-dark-color themes (`colorThemeGate.ts:17`, `37-38`). The redux-color theme sets `THEME_COLOR_LIMIT = 12` (`themes/theme-redux-color.js:19`). |
| `radius`                                                 | `shapes/roundedRect.ts:10` (default 5)                                                                                      | Shared; rounded node shapes.                                                                                                                                                                  |
| `altBackground`, `nodeBorder`                            | `clusters.js:423`                                                                                                           | Only the `divider` cluster; not on the swimlane path.                                                                                                                                         |
| `mainBkg`, `nodeBorder` (handDrawn overrides)            | `shapes/handDrawnShapeStyles.ts:109-110`                                                                                    | Shared; the handDrawn look.                                                                                                                                                                   |
| `useGradient`, `gradientStart`, `gradientStop`           | `insertLookDefs.ts:26`                                                                                                      | The gradient definitions. Shared.                                                                                                                                                             |

The flowchart's markers `point`, `circle` and `cross` read no configuration and no theme variable (`markers.js:323-533` has no `getConfig` or `themeVariables` read; the theme reads are in `barbNeo`, the neo markers and the requirement markers, lines 548-957, which swimlanes do not use). The marker colours come from the stylesheet rules `.marker` (`styles.ts:99-105`, `lineColor`). `lineJump.ts` reads no configuration: its options come from the caller (`adjustLayout.ts:25-29`, `enabled`, `jumpRadius` 6, `jumpStyle`). The palette rules come from `genColor` (`flowchart/styles.ts:49-127`): for each slot `i < colorSlotCount`, `[data-look=...][data-color-id="color-i"]` gets the border, fill and lane rules. The lane element carries `data-color-id` through `stampColorSlot(shapeSvg, node.colorIndex, theme, borderColorArray)` (`clusters/swimlane.js:35`, `colorThemeGate.ts`). The lane's `colorIndex` is set upstream in the DB (**unverified**: not traced), and the synthesised default lane takes the next index (`helpers.ts:126`).

### 6.7 Configuration keys read (typed-view input for decision 0009)

Decision 0009 turns these into a typed view per renderer. The swimlane renderer's view is the `swimlane` section, the `flowchart` keys below, and the global keys. Defaults come from `config.schema.yaml` (read with js-yaml, `schema-defaults.cjs`).

**The `swimlane` section** (read by the layout only):

| Key                                 | Read at                         | Default              | Probed effect                                                                                                              |
| ----------------------------------- | ------------------------------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `swimlane.layout`                   | `config.ts:79-107` (appearance) | `swimlane`           | Without it, `layout` falls back to the top-level `elk`. Probed: `layout: elk` in a directive gives 1 ELK call and no lane. |
| `swimlane.theme`, `swimlane.look`   | `config.ts:79-107`              | `redux-color`, `neo` | Probed: the lanes carry `data-look="neo"` by default.                                                                      |
| `swimlane.ignoreCrossLaneEdges`     | `layoutCore.ts:24`              | `true`               | `false` changes the SVG and makes no ELK call (probed on `mermaid-work.mmd`, which sets it).                               |
| `swimlane.optimizeRanksByCrossings` | `layoutCore.ts:25`              | `true`               | Read only on the non-default path (`phase2.gravity.ts`).                                                                   |
| `swimlane.automaticLaneOrdering`    | `layoutCore.ts:26`              | `false`              | `true` changes the SVG (probed on `14-messy-layout.mmd`, which sets it).                                                   |
| `swimlane.lineHops`                 | `adjustLayout.ts:9-14`          | `arc`                | `false` removes the arc commands; `gap` removes them too (probed on `7-car-sales-constr.mmd`, default arc = explicit arc). |

**The `flowchart` keys** (read by the layout and the renderer):

| Key                             | Read at                                                                                     | Default                            | Note                                                                                                                                                                                                                                          |
| ------------------------------- | ------------------------------------------------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `flowchart.nodeSpacing`         | `layoutCore.ts:22` (fallback 40); `flowRenderer-v3-unified.ts:37` (fallback 50)             | 50                                 | Probed: default equals explicit 50; a value of 100 changes the SVG. The fallback 40 in `layoutCore.ts:22` is not reached in any normal configuration (the schema default is present). The DDLT top-up (`ddlt/backends.ts:34-37`) is the same. |
| `flowchart.rankSpacing`         | `layoutCore.ts:23` (fallback 100); `flowRenderer-v3-unified.ts:38` (fallback 50)            | 50                                 | As above. The layer gap of the layout is this value (`phase4.coordinates.ts:18`, `layoutCore.ts:23`).                                                                                                                                         |
| `flowchart.diagramPadding`      | `flowRenderer-v3-unified.ts:44`                                                             | 8                                  | The root viewport padding.                                                                                                                                                                                                                    |
| `flowchart.titleTopMargin`      | `flowRenderer-v3-unified.ts:48`                                                             | 25                                 | The title position.                                                                                                                                                                                                                           |
| `flowchart.useMaxWidth`         | `flowRenderer-v3-unified.ts:51`                                                             | `true`                             | The root size (section 6.10). All fixtures set `false`.                                                                                                                                                                                       |
| `flowchart.padding`             | `flowDb.ts:1071`                                                                            | 15                                 | The node padding.                                                                                                                                                                                                                             |
| `flowchart.minNodeWidth`        | `flowDb.ts:1099`                                                                            | 120                                | The node minimum width.                                                                                                                                                                                                                       |
| `flowchart.wrappingWidth`       | `shapes/util.ts:98`, `179`                                                                  | 120                                | The wrapping width of labels.                                                                                                                                                                                                                 |
| `flowchart.curve`               | `edges.js:44`; `flowDb.ts:1340`                                                             | `basis`                            | Overridden by `rounded` in the layout for swimlanes.                                                                                                                                                                                          |
| `flowchart.subGraphTitleMargin` | `utils/subGraphTitleMargins.ts`; `common/index.ts:331-333`; `clusters.js:108`, `391`, `619` | `{top: 0, bottom: 0}`              | The lane title margins.                                                                                                                                                                                                                       |
| `flowchart.arrowMarkerAbsolute` | `flowDiagram.ts:30-31`; `edges.js:934`                                                      | from `arrowMarkerAbsolute` (false) | Section 6.4.                                                                                                                                                                                                                                  |
| `flowchart.inheritDir`          | `flowDb.ts:721-725`                                                                         | false                              | The direction of a subgraph.                                                                                                                                                                                                                  |

**Global keys** (read on the path):

| Key                         | Read at                                                                                                          | Default                                          | Note                                                                                               |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| `layout`                    | `flowRenderer-v3-unified.ts:20`, `35`; `edges.js:640`                                                            | `elk` (the top level)                            | The engine name. The per-diagram `swimlane.layout` decides it (section 1.5).                       |
| `look`                      | `flowDb.ts:1251-1337`; `layout-algorithms/swimlanes/helpers.ts:125`                                              | `classic` (top), `neo` (swimlane)                | The look.                                                                                          |
| `theme`                     | `clusters/swimlane.js:18`; `insertLookDefs.ts:25`; `colorThemeGate.ts`                                           | `default` (top), `redux-color` (swimlane)        | The theme name. `default` is not shipped by ferrigura (the harness maps it, `geometry/common.ts`). |
| `themeVariables`            | Section 6.6                                                                                                      | the theme's                                      |                                                                                                    |
| `handDrawnSeed`             | `clusters/swimlane.js:107`, `113`, `179`, `185`; `edges.js:640`; `handDrawnShapeStyles.ts:6`, `14`, `109`, `122` | 0                                                | 0 means random (section 6.8).                                                                      |
| `htmlLabels`                | `edges.js:274`; `clusters/swimlane.js:37`; `shapes/util.ts:61`, `169`; `createLabel.js`                          | `true` (the harness forces `false`)              | Every fixture sets `false`.                                                                        |
| `securityLevel`             | `flowRenderer-v3-unified.ts:20`; `selectSvgElement.ts:12`, `15`; `nodes.ts:40`; `utils.ts:257`; `flowDb.ts:500`  | `strict`                                         | `sandbox` takes another branch in `selectSvgElement` (`selectSvgElement.ts:15`).                   |
| `maxEdges`, `maxTextSize`   | `flowDb.ts:330`; `Diagram.ts` (the text cap)                                                                     | 500, 50000                                       | Limits.                                                                                            |
| `fontFamily`, `fontSize`    | `styles.ts` (root), section 6.6                                                                                  | `"trebuchet ms", verdana, arial, sans-serif`, 16 | The harness forces the Lexica family.                                                              |
| `state.arrowMarkerAbsolute` | `edges.js:934`                                                                                                   | false                                            | Section 6.4. A cross-diagram read.                                                                 |
| `logLevel`                  | not read on the render path                                                                                      |                                                  | 20 fixtures set it; no effect on output.                                                           |

### 6.8 Randomness, the clock and the DOM

- **`Math.random` (reference):** the only source on the swimlane path. Rough.js calls it (chunk `AWAWVJFC` in the bundle, the generator `p.next` at line 213, the `W` helper at 564), which is used by the `stadium`, `question`, `trapezoidalPentagon` and handDrawn `insertEdge` paths. **Probed** with call stacks: `stadium` and `trapezoidalPentagon` set `roughness = 0` for non-handDrawn looks (`stadium.ts:50-51`, `trapezoidalPentagon.ts:41`), yet rough.js still draws its curve offset from the random generator (`_doubleLine` path, `R` and `W`), so the output still varies. The random numbers are multiplied by roughness only in some terms. With the seed 0 (`handDrawnSeed` default), rough.js uses `Math.random` directly (the generator is created with `new p(t2.seed || 0)` inside `W`, chunk line 564).
- **Layout:** no `Math.random`, no `Date`, no clock (checked: the layout directory has only the `mulberry32` in `laneOrdering.ts:27-36`, which takes a seed from `hashString` (FNV-1a, lines 18-25), and that runs only with `automaticLaneOrdering`). `phase1.cycles.ts:56-59` orders its DFS starts by in-degree, which is deterministic.
- **`generateId`** (`utils.ts:442`) is used by the state diagram (`stateDb.ts:366`), not the flowchart path. The `id-…` strings of `makeRandomHex` (`utils.ts:453-464`) are not used on the swimlane path (**unverified** beyond the stack probes).
- **Probed (no clock):** 18 of the 46 fixtures give a different SVG on each of four renders in one page (`probe-chromium4.mts`). The 12 non-handDrawn ones include the eight fixtures with `notch-pent` (`issue-7802-1` to `-6`, `12-disagree-exits-b-c`, `13-disagree-exits-b-c-d`), and `14-messy-layout`, `16-intake-cycle-lr`, `sales-process` and `intake-review-complete`. The six handDrawn ones are the two coloured-lane handDrawn fixtures, three in `handdrawn/`, and `synthetic-default-lane-handdrawn`. For the handDrawn ones, `handDrawnSeed: 42` makes two renders equal (probed).
- **Probed (clock):** with `renderReference(page, id, text, { now: 1700000000000, seed: 7 })`, which seeds `Math.random` and fixes `Date`, all 46 fixtures repeat exactly, in en-US, de-DE, tr-TR and sv-SE (`probe-clock-locale.mts`). The locale changes no output for these fixtures.

### 6.9 DOM dependencies a server renderer must replace

| Dependency                                             | Where                                 | Replacement needed                                                                                                          |
| ------------------------------------------------------ | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Text measurement (`getBBox`, `getComputedTextLength`)  | section 6.3                           | The port already has font metrics (`crates/ferrigura-text`, decision 0005). The swimlane's `createText` path uses the same. |
| The root `getBBox`                                     | `setupViewPortForSVG.ts:29`           | The Rust unified pipeline must compute the same root bounds (**unverified**: the Rust side was not read).                   |
| `getTotalLength`                                       | `lineJump.ts:748-750`, `edges.js:879` | Path length of the port's path type (`crates/ferrigura-path`).                                                              |
| `window.location` (the marker URL)                     | `edges.js:935-938`                    | Only with the `arrowMarkerAbsolute` key.                                                                                    |
| `document` and the `svg` element (`getDiagramElement`) | `flowRenderer-v3-unified.ts` (`draw`) | Not needed for the output.                                                                                                  |
| DOMPurify (`sanitize`)                                 | `diagrams/common/common.ts`           | The port's sanitiser (shared with the flowchart).                                                                           |
| `btoa`                                                 | `edges.js:705`                        | Base64 of the JSON points; `data-points` must match.                                                                        |

### 6.10 Root sizing (`setupViewPortForSVG.ts:15-40`)

The viewport is the root `getBBox` plus `2 * padding` (`diagramPadding`, 8 by default), `viewBox = "x-padding y-padding width height"`, and `configureSvgSize` sets the size. **Probed** on `1-simple.mmd`:

- `useMaxWidth: true` gives `width="100%"` and `style="max-width: 1351.0244140625px;"`.
- `useMaxWidth: false` gives `width="1351.0244140625"` and `height="381"`.

Both carry `viewBox="-94 -64.5 1351.0244140625 381"` (the padding is included in the viewBox). The root has `class="flowchart"` and `aria-roledescription="swimlane"`.

## 7. Fixtures

### 7.1 The 46 fixtures

`e2e/diagrams/swimlanes/`: `nightly-check/` 36 files (three of them in `handdrawn/`), `pr-check/` 10 files, `visual-check/` none (a `.gitkeep`). All 46 set `htmlLabels: false` and `flowchart.useMaxWidth: false` in their front matter, so the fixtures exercise the SVG-text labels only and the root sizing with `useMaxWidth` false. Counts from `fixtures.cjs`:

| Feature                                          |                                                                      Fixtures |
| ------------------------------------------------ | ----------------------------------------------------------------------------: |
| `swimlane-beta LR` (direction in the first line) |                                                                            23 |
| `look` set in front matter                       |                                                       13 (handDrawn 7, neo 6) |
| `theme` set in front matter                      | 14 (the values include `redux`, `redux-color`, `redux-dark-color` and `base`) |
| `layout` set (`swimlane`)                        |                                                                             5 |
| `swimlane:` section                              |                                                                             5 |
| `swimlane.ignoreCrossLaneEdges`                  |                                           4 (3 true, 1 false: `mermaid-work`) |
| `swimlane.optimizeRanksByCrossings`              |                                                           4 (3 true, 1 false) |
| `swimlane.automaticLaneOrdering: true`           |                                                         1 (`14-messy-layout`) |
| `classDef`, `style`, `linkStyle`                 |                                                                    3, 2 and 1 |
| `themeVariables`                                 |                                    1 (`custom-theme-variables`, theme `base`) |
| `logLevel: 0`                                    |                                                      20 (no effect on output) |

Shapes in the fixtures include `{...}` (17 files), `{{...}}` (28 occurrences), `([...])` stadium (8 occurrences, the rough.js path), `[(...)]` cylinder (8 occurrences), `((...))` (2 occurrences) and `@{ shape: notch-pent }` (8 files). **Unverified:** that every shape the fixtures use is in the flowchart's shape set; the flowchart port covers it.

Handling in the geometry harness (`tools/port/geometry/common.ts`): `forcedDirective` replaces any theme name ferrigura does not ship (`redux`, `base`) with `redux-color`, and keeps the fixture's `themeVariables` (`common.ts:79-86`). The harness must render with the clock (section 6.8), or the 18 fixtures need tolerances. Nothing else is excluded by the harness rules.

### 7.2 The DDLT fixtures (layout inputs)

`e2e/platform/dev-diagrams/layout-tests/swimlanes/`: 31 `.mmd` files and 28 `.sizes.json` files (`1-simple`, `10-node-placement`, `12-disagree-exits-b-c`, … `query-process`, `simple-2`, `mermaid-work`, `pebr-*`, `issue-7802-*`, `regressions/`). The DDLT specs run them with `loadDdltFixture` (`ddlt/loadDdltFixture.ts`), which parses with the real pipeline, applies the captured sizes and runs the swimlane core. This is the cleanest differential input (section 7.3).

### 7.3 How to test a port (summary; the layout details are in L7)

- The layout's DOM-free seam is `runSwimlaneLayoutCore(data)` (`layoutCore.ts:22`). Its input is the `LayoutData` after `prepareLayoutForSwimlanes` and `createEdgeLabelNodes` (`index.ts:8-12`) with the captured sizes applied (`ddlt/backends.ts:46-67`).
- **Probed:** `parseMmdFileToLayoutData` plus `loadDdltFixture` runs under jsdom with vite-node (`ddlt-core3.mts`). For `swimlanes/simple-2` it gives 9 nodes, 5 edges and 2 lanes, the layout config `flowchart.nodeSpacing` 50 and `rankSpacing` 50, and two runs give identical positions, sizes, layers and edge points. The first load took 512 ms (the probe's `ms`, including the parse); the whole vite-node run took several minutes, mostly module transforms.

## 8. Risks, effort, open questions

### 8.1 Risks

1. **Size and density of the geometry.** `materializedGeometry.ts` (2,814 lines) and `router.ts` (2,283) are long, interdependent passes. Their decisions depend on ordering (JavaScript sort is stable, and `Map`/`Set` iteration follows insertion), on `toFixed` keys (`router.ts:225`, `1042`, `1129`; `terminalStub.ts:153`; `materializedGeometry.ts:1737`, `2319`, `2516`), on `Math.hypot` (`router.ts:1145`: the A\* f-cost; `terminalStub.ts:78`, `87`, `237`; `materializedGeometry.ts:1614`), and on the ties that follow. A port that differs in the last bit can choose another route.
2. **`toFixed` on exact ties.** JavaScript rounds exact ties up: `(0.25).toFixed(1)` is `"0.3"` and `(2.5).toFixed(0)` is `"3"` (**probed**, `js-semantics.cjs`). The Rust `format!` family rounds ties to even (from knowledge, **unverified** here). The keys and ids must use a JavaScript-compatible formatter.
3. **`Math.hypot`.** It is not `sqrt(x*x + y*y)`: the two differed in 76,511 of 200,000 random pairs (38%), in the last bit (**probed**, `js-semantics.cjs`). The port must use V8's algorithm for `Math.hypot`, or the A\* tie-breaks can change.
4. **Collation.** `localeCompare` is used at 18 call sites in 10 files: `driving-tree.ts` (5), `phase2.dummies.ts` (3), `phase0.helpers.ts` (2), `phase2.multitree.order.ts` (2), and one each in `phase1.cycles.ts`, `phase2.laneAwareCompact.ts`, `phase2.crossLaneAdjust.ts`, `phase3.ordering.ts`, `phase2.multitree.core.ts` and `laneOrdering.ts`. The topological order (`phase0.helpers.ts:65-79`) and the cycle removal (`phase1.cycles.ts:23`) depend on it. **Probed:** in V8 the order is `_x, 10, 9, a, A, a_b, a-b, ab, b, B, edge-label-a-b-e1, edge-label-A-B-e1, z, Z`, against code-point order `10, 9, A, B, Z, _x, a, a-b, a_b, ab, b, …`. The port needs an ICU-compatible comparison for the IDs used (ASCII letters, digits, `-`, `_`). The clock-seeded locale probe found no difference between en-US, de-DE, tr-TR and sv-SE for the 46 fixtures.
5. **Default sorts.** Two calls use `[...].sort()` with no comparator, which is UTF-16 code-unit order (`phase2.laneAwareCompact.ts:44-45`, `phase2.longestPath.ts:14`). Both are fallbacks that cannot run on the default path: `pipeline.ts` runs `removeCycles_DFS` before the layering, so the topological sort always succeeds there.
6. **Reference randomness** (section 6.8). The reference is not byte-repeatable without a seed. The port cannot reproduce rough.js's random draws unless it replays the same generator in the same order, and that is only possible with the harness clock (which seeds `Math.random`). For the 18 unstable fixtures, either the port replays the generator (the harness clock makes the reference stable) or the differential uses a tolerance on these shapes.
7. **Config model.** The Rust config has no `swimlane` section (`ts_defaults.json`, `crates/ferrigura-model/src/config`), and the appearance layering that makes `layout: swimlane` win over `layout: elk` (`config.ts:79-107`) is not in the port yet. Decision 0009's typed views cover the `swimlane` section once it exists.
8. **DDLT defaults.** `backends.ts:34-37` tops up 40 and 100 when `flowchart.nodeSpacing` and `rankSpacing` are absent. In the browser and in the DDLT path they are present (50 and 50), so the top-up is inert. A port must take 50 and 50 from the schema, not the 40 and 100 fallbacks of `layoutCore.ts:22-23`.
9. **Up-sweep transposition (derived, unverified by a probe).** `phase3.ordering.ts:272-275` calls `transposeImprove(layers[i+1], layers[i], edges, layers[i-1])` on the up sweep, with the upper and the next layer swapped relative to the down sweep (line 269). In the default layering every edge goes from a rank to an equal or larger rank (`phase2.laneAwareCompact.ts:75-81`), so `edgesIn` and `edgesOut` of the up sweep are empty and the transposition cannot accept a swap. The port must keep the call as it is, and not "fix" it, or the orders can differ.
10. **Lane order in the DOM.** The reference inserts lanes in reverse source order (section 6.2). It is observed, not traced.
11. **Non-default options** (`ignoreCrossLaneEdges: false`, `automaticLaneOrdering: true`) run the gravity, multitree and crossing code (1,270 lines, section 1.3). Two fixtures use them.
12. **`Math.round` with negative halves** (`phase2.gravity.ts:80`) rounds towards `+∞` in JavaScript (`Math.round(-2.5) = -2`, probed), and `f64::round` in Rust rounds away from zero. The gravity path is non-default, but its ranks depend on it.

### 8.2 Effort (rough)

- Layout engine, default path: about 10,400 lines of TypeScript, of which many are comments and debug code. Estimate 8,000 to 10,000 lines of Rust, with the router and the geometry passes taking most of it.
- Layout engine, non-default options: about 1,270 lines of TypeScript, estimate 1,000 to 1,400 lines of Rust. It can be deferred with a named exclusion, since two fixtures use it.
- Renderer additions: the lane shape (about 250 lines of TypeScript), the swimlane branch of `insertEdge` (about 50), the line-hop call (about 30), the lane rules in the stylesheet (about 40), the layout registration and the `swimlane` config section (about 150). Estimate 600 to 900 lines of Rust.
- Tests: the 46 fixtures through the geometry harness, the 28 DDLT fixtures through the core, the 28 unit spec files of `__tests__/`, and the 6 router spec files (314 tests in the swimlane paths, section 5).

These are estimates from the line counts, not from a port; treat them as ±30%.

### 8.3 Open questions for the coordinator

1. **Scope of the non-default options** (`ignoreCrossLaneEdges: false`, `automaticLaneOrdering`). Port them in P3, or name them as an exclusion in the P3 plan (two fixtures)?
2. **Randomness policy.** Replay rough.js's generator with the harness clock, or compare the 18 unstable fixtures with a tolerance? This decides whether a byte-level differential is possible for them.
3. **Collation.** Which comparison does the port use for `localeCompare`: an ICU-compatible collation crate, or a hand-written comparison for the ASCII IDs the fixtures use?
4. **The differential seam.** Use the DDLT path (`runSwimlaneLayoutCore` on captured sizes, section 7.3) as the layout's `port:diff`, with the geometry harness for the SVG? The DDLT path needs jsdom for `DOMPurify`, so the capture must be done once, in the reference, and stored as JSON.
5. **Plan corrections.** The P3 plan's "63 source lines" and "2 spec files" describe the diagram folder only. The layout is 11,684 lines and 50 spec files, and the swimlane specs run 314 tests (311 passing, 3 skipped).
6. **The `swimlane` section.** Add it to the Rust config model with decision 0009's typed view, and the per-diagram appearance layer (`config.ts:79-107`), before the layout is written?

## Part II. The swimlane layout engine

This part follows the layout-engine sections of [P2-RESEARCH.md](../P2-RESEARCH.md). The engine is in-tree, so there is no library version to pin.

### L0. Summary

`swimlane` is a layered graph layout with lanes, followed by an orthogonal router and a long chain of geometry passes. It is a Sugiyama pipeline (cycle removal, layering, dummy nodes, median ordering, coordinates) whose phases are lane-aware, then a routing phase that places each edge as an axis-aligned polyline between node borders, and a post-processing phase that cleans the polylines. It does not call ELK or elkjs: the engine folder imports no ELK module (grep found none). The shared measure step builds a dagre-d3-es graphlib graph (`createGraph.ts:2`), but the swimlane core never reads it (`layoutCore.ts` and `index.ts` use `data4Layout` and the measured groups only). The engine is deterministic, except that its optional `automaticLaneOrdering` uses a seeded generator. 44 of the 46 fixtures use only the default path; the other two set the non-default options (section 7.1).

### L1. Sources

Section 1.3 lists the files and their lines. The non-default files are the ones listed under "non-default". Imports checked by reading: `layoutCore.ts` imports `postProcessing`, `helpers`, `pipeline` and `orthogonalRouter/router`; `pipeline.ts` imports the phases and `laneOrdering`; `postProcessing.ts` imports the `direction/` modules; `router.ts` imports only the types and `config.ts`; `phase2.gravity.ts` and `phase2.longestPath.ts` import the multitree code. The non-default code is loaded by the default path's module graph (`pipeline.ts` imports it), but it runs only when `ignoreCrossLaneEdges` is false (the branch at `pipeline.ts:47-57`; the flag is read at `layoutCore.ts:24` and `pipeline.ts:35`).

### L2. Input

What Mermaid builds and hands the engine (`createCommonLayoutRenderer`, `common/index.ts:129-204`):

- `LayoutData`: `nodes`, `edges`, `direction`, `config` (the full configuration, `flowDb.ts:1346`), `layoutAlgorithm` (`swimlane`), `markers`, `diagramId`, `type`.
- After `prepareLayoutForSwimlanes` (`helpers.ts:98-140`): top-level groups have `shape: swimlane` and `direction`; loose nodes are in the synthesised lane `__swimlane_default__`.
- After `createEdgeLabelNodes` (`edgeLabelNodes.ts:38-131`): each labelled edge (not layout-only) gets a label node `edge-label-<start>-<end>-<id>`, its `labelNodeId`, and two layout-only virtual edges `<id>-to-label` and `<id>-from-label`. The label text moves to the label node, and `edge.label` is cleared.
- After `measureLayout` (`createGraph.ts:122-215`): `width` and `height` for each non-group node from `getBBox`, `labelBBox` for each group label.

`toGraphView` (`helpers.ts:142-172`) builds the engine's graph: the node order is the groups in reverse order, then the non-groups (`helpers.ts:169-170`); the edges exclude the labelled originals (`labelNodeId` set, line 159) and keep the layout-only edges; the edge ids are kept.

### L3. Algorithm

The phases, in order (`pipeline.ts:24-77`, `layoutCore.ts:17-58`), with the constants.

**3.0 Preparation.** `normalizeGraph` (`phase0.helpers.ts:5-23`): drops edges with unknown endpoints and duplicates (by `id:src->dst`), keeps the node order.

**3.1 Lanes and direction.** `prepareLayoutForSwimlanes` (`helpers.ts:98-140`); `toGraphView` as above. Lane order is `buildTopLaneOrder` (`phase2.options.ts:57-65`): the top-level groups in layout order, reversed, unique.

**3.2 Automatic lane order (optional).** When `automaticLaneOrdering` is true, `optimizeTopLaneOrder` (`laneOrdering.ts:171-196`) reorders the lanes: a greedy switch (adjacent swaps while the weighted distance drops, `greedySwitch` lines 119-150) from the source order, then 8 restarts (`AUTOMATIC_LANE_ORDERING_RESTARTS`) from shuffled orders (Fisher-Yates driven by mulberry32, lines 27-46), keeping the cheapest (ties broken by the distance from the source order, lines 152-157). The seed of each restart is FNV-1a of the source order, the weights and the restart index (lines 159-169, 18-25).

**3.3 Cycle removal** (`removeCycles_DFS`, `phase1.cycles.ts:11-81`). Adjacency sorted by destination then edge id (`localeCompare`, line 23). DFS seeds: nodes with in-degree 0 first, then the rest, in declaration order (lines 55-64). A back edge (to a grey node) is recorded and reversed; reversed edges keep their `id` and `ref`, and the source and destination swap (`phase1.cycles.ts:67-72`).

**3.4 Layering.** Default: `assignLayers_LaneAwareCompact` (`phase2.laneAwareCompact.ts:40-88`).

- Order: for `LR` the generation-based Kahn order (`topoSortByGenerationIfAcyclic`, lines 16-37: all zero in-degree nodes first, then the next generation, each sorted by `localeCompare`); for `TB`, `BT` and `RL` the queue-based Kahn order (`topoSortIfAcyclic`, `phase0.helpers.ts:158-181`: the queue is kept sorted by insertion, and successors are taken in sorted order). The fallback when there is a cycle is `[...g.nodes].sort()` (line 44-45).
- Lane of a node: its top-level ancestor, or its own id (`createTopLaneResolver`, `phase2.options.ts:52-55`, `buildTopLaneMap` 23-50).
- Rank: for each node in order, `base = max over predecessors u of rank(u) + w(u, v)`, where `w = 1` when `u` and `v` are in the same lane and `0` otherwise (`edgeWeight`, lines 56-62); then `rank(v) = max(base, nextFree[lane])`, and `nextFree[lane] = rank(v) + 1` (lines 78-82). Groups get no rank (line 66).
- Layers: `buildLayersFromRanks` skipping groups (`phase0.helpers.ts:101-123`).

Non-default (`ignoreCrossLaneEdges` false): `assignLayers_Gravity` (`phase2.gravity.ts:18-122`). Base ranks from `assignLayers_LongestPath` (`phase2.longestPath.ts:12-65`); optional crossing rank optimisation (`phase2.crossOptimization.ts:62-105`, at most `MAX_CROSSING_OPTIMIZATION_PASSES` = 4 passes, lift nodes while the total crossings drop); the gravity relaxation of `GRAVITY_ITERATIONS` = 8 rounds with `Math.round((predAvg + succAvg) / 2)` (`phase2.gravity.ts:80`) clamped to the feasible range (lines 44-60); then a feasibility pass. `adjustCrossLaneSources` (`phase2.crossLaneAdjust.ts:8-70`) runs after the longest path. Layers come from the multitree order (`phase2.multitree.order.ts:72-107`, from `driving-tree.ts:34-277` and `phase2.crossCounts.ts`).

**3.5 Proper layering** (`makeProperLayering`, `phase2.dummies.ts:4-71`). Edges are sorted by id, source and destination (`localeCompare`, lines 31-37). An edge spanning more than one rank gets dummy nodes `placeholder-<n>` on the intermediate ranks (lines 16-28, 49-54), with edge ids `<id>#<k>`. Dummies are appended to the layer lists and to `nodes`.

**3.6 Ordering** (`orderLayers`, `phase3.ordering.ts:251-279`). Three rounds (`s < 3`) of: a down sweep over layers `1..n-1`, each reordered by the median of the fixed neighbours in the layer above (`reorderLayer`, lines 110-183) and then `transposeImprove` with the layer above and the layer below (lines 185-244, adjacent swaps that strictly lower the crossing count, repeated until no swap helps); then an up sweep over layers `n-2..0` with the same two steps, using the layer below (the transposition arguments are swapped, see section 8.1 item 9). With lanes, nodes are partitioned by lane and each lane is sorted on its own, in lane order (lines 136-147), and dummies without a lane are inserted by barycentre (lines 149-180). Ties in the median are broken by the current index, then by `localeCompare` (lines 49-57). The crossing count is an inversion count (`countInversions`, `phase0.helpers.ts:197-222`, merge sort).

**3.7 Coordinates** (`assignCoordinates`, `phase4.coordinates.ts:13-192`). Constants: `layerGap` = `rankSpacing` (50 by default in the browser, `layoutCore.ts:23`); `nodeGap` = `nodeSpacing` (50); `laneGap` = 2 × `nodeGap` (line 20). For `LR`/`RL`, the layer gaps are widened so that the widths fit (lines 39-53). Each lane's column width is the sum of its nodes' widths plus `nodeGap` between them, and the lane column is the widest layer's total (lines 65-93). The columns are laid out left to right in lane order with `laneGap` between them, centred on 0 (lines 95-113). Each layer starts at `yOffset`; a node is centred in its lane column at `y = yOffset + layerH / 2` with `layerH` the tallest node of the layer (lines 115-153); nodes in a lane are spread with `nodeGap`. Dummy `x` is `Math.round` of the midpoint of the chain's endpoints (lines 155-189, `Math.round` at 174).

**3.8 Write-back** (`writeBackToLayoutData`, `helpers.ts:174-356`). Each node gets `layer` and `order` (lines 185-203) and its `x` and `y`. Groups (lanes and nested subgraphs) get the bounds of their children, padded (lines 205-251): a top lane's horizontal padding is `2 * max(20, padding)` (line 239), a nested group's is `padding` (default 20); the vertical padding is `padding`. Top lanes share the vertical extent: `verticalMargin = max(maxPad, 36)` (lines 270-273), `lane.y` is the centre and `lane.height` the common height, and `swimlaneContentTop` is the highest content (lines 274-278). The widths of the lanes are solved from the centres: an alternating recurrence `u[i+1] = 2 d[i] - u[i]` over the centre distances `d`, and an admissible offset `x` between the lower and upper bounds (lines 306-343). Each lane's title rectangle is a band of 21 px at the top (`assignTopLaneTitleRect`, lines 60-96).

**3.9 Routing** (`routeEdgesOrthogonal`, `router.ts:122-2283`). The constants (`router.ts:17-24`): `EPS` 1e-6, `NODE_PADDING` 8, `HORIZONTAL_PIPE_MARGIN` 15, `VERTICAL_PIPE_MARGIN` 15, `ROUTING_MARGIN` 25, `ANCHOR_OFFSET` 20, `TRACK_SPACING` 10. The steps, in the file's own numbering:

- Setup (lines 126-330): the edges to route (layout-only edges skipped, lines 130-146), the lane of every node (lines 165-175), the obstacles (non-group, non-label nodes, lines 177-214), the pipes (lines 215-270: grid lines, `find or create pipe`).
- Phase 1, initial routing (lines 330-396). For each edge: a straight fast path when the anchors align (lines 274-328); otherwise two L-shaped candidates (`cornerHV`, `cornerVH`, lines 1157-1190) if unobstructed; otherwise A* over the intersections of the horizontal and vertical pipes (lines 1127-1320). A* keys are `x.toFixed(1), y.toFixed(1)` (line 1129); `f = g + Math.hypot(dx, dy)` (line 1145); the open list is sorted by `f` with `Array.prototype.sort` (line 1194), which is stable.
- Step 6, port assignment (lines 397-695): 6.1 the side of each edge at each end (lines 419-463); 6.2 sibling side splitting: the strongest sibling keeps its side and weaker ones move to the secondary side, by a preference strength (the dy/dx ratio) and a side-load counter (lines 464-565); 6.2b bimodal in/out for diamonds (lines 566-610); 6.3 port groups: the offsets along each side, with ports sorted by the opposite coordinate and spread within the side (lines 611-695; diamonds get 0.3 of the side).
- Step 7, anchors (lines 696-1063): the anchor point 20 px out of each port (`ANCHOR_OFFSET`), pushed out of obstacles with waypoints (lines 846-905).
- Phase 2, crossing reduction (lines 1614-2005): a main loop over edges, trying alternative routes and keeping the one with fewer crossings.
- Phase 3, rebuild geometry (lines 2006-2282): the edge points are rebuilt from the routed segments; edge-label nodes are excluded from obstacles (Strategy 1, lines 179-183) and labels are placed later (3.10).

**3.10 Post-processing** (`postProcessSwimlaneLayout`, `postProcessing.ts:29-155`), in order:

1. Direction transforms: `LR` and `RL` (`applyLrDirectionTransform`, `lrTransform.ts:150`), `BT` (`applyBtDirectionTransform`, `lrTransform.ts:141`). The engine works in TB and rotates afterwards.
2. For each edge with at least two points: `orthogonalizePolyline` then `simplifyPolyline` (`geometry.ts:652-688`).
3. `simplifyDetouredEdges` (`detourSimplification.ts`), `straightenCollinearSiblingDetours` (`siblingSharedFaceRouting.ts`), `portSwapToLShape` (`portSwap.ts:107`).
4. `anchorLabelsToPolyline` (`labelAnchoring.ts:83`), `clipEdgeEndpointsToNodeBoundaries` (`endpointClip.ts:85`), `collapseShortTerminalStub` (`terminalStub.ts:44`), `nudgeSharedInteriorSubpaths` (`sharedTrackNudging.ts:13`), `separateSharedRenderedTerminalLanes` (`materializedGeometry.ts:45`), `collapseRedundantRectangularDoglegs` (`:357`), `liftObstacleHuggingSameSideRails` (`:511`), `swapDestinationTerminalTailsToReduceCrossings` (`:986`).
5. `finalizeRenderedEdges` (twice, lines 111-123 and 142): `resolveRenderedOrthogonalCrossings` (`:1835`), the terminal-lane split, `reassignCrossingExternalRailChannels` (`:1210`), `shortcutRedundantOrthogonalJogs` (`:1587`), label anchoring, `prepareEdgeEndpointsForRenderer` (`endpointClip.ts:447`), and the rail lift.
6. A second `nudgeSharedInteriorSubpaths` (line 136).
7. The lane title bands: `liftTopLaneTitleBandsAboveRails` (`:720`) and `shiftLeftLaneTitleBandsLeftOfRails` (`:848`), each twice (lines 147-154).

Then `validateSwimlanesLayout` (`direction/validation.ts:54`) returns issues and logs warnings (`validation.ts:149-154`); it does not throw and it does not change the output (**unverified** beyond reading), and `layoutCore.ts:55` calls it.

### L4. Randomness

Every draw, from which generator, in which order:

| Draw                                                                | Where                                       | Generator                                                                                                                      | When                                                                                                                                                                          |
| ------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lane order restarts                                                 | `laneOrdering.ts:186-193`, shuffle `:38-46` | mulberry32, seed from `hashString` (FNV-1a) of the source order, the weights and the restart index (`laneOrdering.ts:159-169`) | Only with `automaticLaneOrdering` (default false). Restart `i` uses seed `hash(... '#' i)`; the shuffle draws `j = floor(random() * (i + 1))` for `i` from `n-1` down to `1`. |
| Cycle-removal seeds                                                 | `phase1.cycles.ts:56-59`                    | None: a fixed order (the nodes with no incoming edge in declaration order, then the rest)                                      | Always. Not random.                                                                                                                                                           |
| Rough.js (the shapes and the lane's handDrawn look, not the layout) | Section 6.8                                 | `Math.random` when the seed is 0                                                                                               | Renderer only.                                                                                                                                                                |

No other `Math.random`, `Date`, or clock is read in the engine directory (checked with grep). State that survives between layouts on one page: the module-level `clearLayoutRenderState` maps (`common/index.ts:206-210`, `clusters`, `nodes`, `edges` maps), and the `cnt` counter of `generateId` is not in this path. The engine itself keeps no state between calls (each call builds its own graph).

### L5. Output

Mermaid reads back the following (`writeBackToLayoutData`, `layoutCore.ts:24-58`, `helpers.ts:174-356`):

- Nodes: `layer`, `order`, `x`, `y`; groups: `x`, `y`, `width`, `height`, `swimlaneContentTop`, `groupTitleRect`; label nodes: the same `x` and `y` (used to place the label, `common/index.ts:328-351`).
- Edges: `points` (the routed polyline after post-processing), `curve` set to `rounded` unless it was `basis` or another curve (`layoutCore.ts:37-41`), `labelNodeId` (kept), and the removed `points` are re-created by the router.
- The renderer then clips the endpoints again (`edges.js:654-665`) and places the markers.

Mermaid positions a node by its `x` and `y` (`rendering-elements/nodes.ts`, `positionNode`, called from `common/index.ts:253-259`) and draws an edge from `points` (`edges.js:624`, `insertEdge`). Lanes are drawn by `swimlane.js`, using the lane's `x`, `y`, `width` and `height`.

### L6. Determinism and floating point

The engine is deterministic given its inputs, with these exceptions and requirements:

- **Sorting.** `Array.prototype.sort` is stable in V8 (ES2019), and every comparator in the engine returns a consistent order except where `localeCompare` is used (section 8.1 item 4). A port must sort stably.
- **Iteration order.** `Map` and `Set` iterate in insertion order; `Object.create(null)` maps are iterated with `Object.entries` (`phase4.coordinates.ts:83`), which follows insertion order for non-integer keys. Integer-like keys (lane ids that are numbers) iterate first. The engine uses string ids; **unverified** whether a numeric-looking id occurs in the fixtures.
- **Floating point.** The engine uses IEEE doubles. Operations are performed in source order. A port must keep the same order of additions (for example `sum` in `phase4.coordinates.ts:84-85` and the widths in `helpers.ts:296`), because the sum of doubles is not associative.
- **Math functions.** `Math.abs`, `Math.max`, `Math.min`, `Math.sign` are exact. `Math.hypot` (5 uses) must be V8's algorithm (section 8.1 item 3). `Math.round` rounds half up (`phase4.coordinates.ts:174` on the default path, `phase2.gravity.ts:80` on the non-default path); `Math.ceil` (`router.ts:2110`, `phase2.crossCounts.ts:83`), `Math.floor` (`laneOrdering.ts:42`), `Math.log2` (`phase2.crossCounts.ts:83`), and `Math.imul` (`laneOrdering.ts`) must match too.
- **Number formatting.** `toFixed` (router keys and ids, `terminalStub.ts:153`, `materializedGeometry.ts:1737-2516`) rounds exact ties up (section 8.1 item 2). The keys are only compared for equality, so a difference changes which keys collide, and that changes the routes.
- **Iteration limits.** `GRAVITY_ITERATIONS` 8 (`config.ts:19`); `MAX_CROSSING_OPTIMIZATION_PASSES` 4 (`config.ts:22`); the Phase 3 sweep count 3 (`phase3.ordering.ts:265`); `transposeImprove` runs until no swap helps (`phase3.ordering.ts:220-242`); `greedySwitch` runs at most `order.length` sweeps (`laneOrdering.ts:128`); the router's A\* has no limit that the reader found (**unverified**: no iteration cap was seen in `router.ts:1127-1320`).

### L7. How to test a port

1. **Capture inputs.** For each DDLT fixture (`e2e/platform/dev-diagrams/layout-tests/swimlanes/*.mmd` with its `.sizes.json`, 28 of them), run `parseMmdFileToLayoutData` (`ddlt/parseToLayoutData.ts:15-40`) under jsdom (section 7.3), then `runSwimlanesDdlt`'s steps (`backends.ts:46-67`) up to but not including `runSwimlaneLayoutCore`. Store the `LayoutData` as JSON: nodes (all fields listed in section 4), edges, `direction`, and the configuration keys of section 6.7.
2. **Capture outputs.** Run `runSwimlaneLayoutCore` on the same input and store the nodes (`x`, `y`, `width`, `height`, `layer`, `order`, `groupTitleRect`, `swimlaneContentTop`), the edges (`points`, `curve`) and the returned direction. The DDLT specs already do this; the `query-process` spec shows the pattern.
3. **Compare.** Compare the ordering fields (`layer`, `order`, the lane order) exactly. Compare the coordinates with an absolute tolerance of 1e-9 first; the port should agree bit for bit where it follows the same operation order (section L6), and a mismatch at the last bit usually means a different `Math.hypot`, `toFixed` or sort.
4. **Validate.** `validateLayout` and `scoreLayout` (`layout-algorithms/layout-utils/`) are the reference's own checks; run them on the port's output (they are TypeScript, so run them in Node on the JSON), and compare the scores.
5. **Render check.** The geometry harness (`pnpm port:geometry`) compares the SVG of the 46 fixtures with the clock (section 6.8). Set `htmlLabels` false as the fixtures do.
6. **Unit tests.** Port the `__tests__/` phase tests one per phase (`phase0.helpers.spec.ts`, `phase1.cycles.spec.ts`, `phase2.layering.spec.ts`, `phase3.ordering.spec.ts`, `phase4.*.spec.ts`), and the `direction.*` and `raykov.*` tests for the post-processing and router.
7. **Probe the bundle.** Any doubt about a rule can be settled with the Chromium probe in `probe-chromium.mts` (the page, `renderReference` and the clock), which runs the reference's own code.

### L8. Risks, effort, open questions

The layout-specific risks are in section 8.1 (items 1 to 5, 8, 9, 11, 12) and the effort in section 8.2. The open questions are in section 8.3; the most urgent for the engine are the non-default scope (question 1), the randomness policy (question 2) and the collation choice (question 3).
