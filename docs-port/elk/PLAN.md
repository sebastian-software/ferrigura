# ELK Layered port: plan

Stage A of M4 (see [STATUS.md](../STATUS.md)). It fixes the source version ([SOURCES.md](./SOURCES.md)), the option set Mermaid reaches, the Java that option set needs, and the order of the port. The mapping table at the end is filled in during Stage C.

Scope rule from [0006](../decisions/0006-elk-layered-port.md): only ELK Layered is ported, from the Java sources, with its own crate `ferrigura-elk` under EPL-2.0.

## 1. Scope

**What is ported:** the ELK Layered algorithm for the options Mermaid can reach, plus the minimum of ELK core, `alg.common` and the graph model that this needs. Everything else in ELK is out of scope.

**What "reach" means:** a value is in scope if a Mermaid user can make Mermaid pass it to elkjs, through `config.elk.*` (schema enums included) or through the live option keys in `render.ts`. The catalogue `elkOptionCatalogue.ts` lists further options, but every line there is commented out, and `elkOptionCatalogue.spec.ts` asserts that the live overrides stay empty, so the catalogue adds nothing to the set.

**Proposed for confirmation by the coordinator:** this reading of "port only ELK Layered" (see open questions, §8). The alternative, porting all of Layered, adds about 17 k Java lines (with comments) of splines, wrapping, compaction, comments and validation, which Mermaid cannot select (§4.4).

## 2. Mermaid's ELK option set

Source: `packages/mermaid/src/rendering-util/layout-algorithms/elk/render.ts` (live keys, no comments) and `config.schema.yaml` (`elk` section, lines 93–264). Line numbers below refer to `render.ts`.

### 2.1 Live keys set by Mermaid

Values are as written in `render.ts`; `config.*` entries take the value from the user's config, which carries the schema defaults of §2.2 when unset.

| Key                                                                  | Set where (line)                       | Value                                                                                                                                                        |
| -------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `elk.hierarchyHandling`                                              | root 914, containers 472/493/1265–1273 | `INCLUDE_CHILDREN` on root; `SEPARATE_CHILDREN` on containers with their own algorithm or direction; `INCLUDE_CHILDREN` on ancestors of cross-boundary edges |
| `elk.algorithm`                                                      | root 915, containers 471/491           | `elk.layered` for `layout: elk`; other names fall back to layered (ADR 0006)                                                                                 |
| `elk.direction`                                                      | 716, 492, 921                          | `DOWN`, `UP`, `LEFT`, `RIGHT` from the diagram direction                                                                                                     |
| `spacing.baseValue`                                                  | root 922 (40), containers 397 (24)     |                                                                                                                                                              |
| `elk.layered.nodePlacement.strategy`                                 | root 917, containers 441               | `preset.placement` / `preset.containerPlacement`, or `config.nodePlacementStrategy`                                                                          |
| `elk.layered.nodePlacement.bk.fixedAlignment`                        | root 918, containers 432               | `preset.alignment`, or `config.nodePlacementAlignment`                                                                                                       |
| `elk.layered.nodePlacement.networkSimplex.nodeFlexibility`           | containers 452                         | `PORT_POSITION`                                                                                                                                              |
| `elk.layered.cycleBreaking.strategy`                                 | root 928, containers 449               | `preset.cycleBreaking`, or `config.cycleBreakingStrategy`                                                                                                    |
| `elk.layered.layering.strategy`                                      | root 930                               | `preset.layering`, or `config.layeringStrategy`                                                                                                              |
| `elk.layered.layering.coffmanGraham.layerBound`                      | root 932                               | `config.layeringLayerBound` (schema default 4)                                                                                                               |
| `elk.layered.layering.layerConstraint`                               | node 672                               | `FIRST`, only with `keepEntryNodeOnTop`                                                                                                                      |
| `elk.layered.crossingMinimization.forceNodeModelOrder`               | root 924                               | `config.forceNodeModelOrder` (default `false`)                                                                                                               |
| `elk.layered.considerModelOrder.strategy`                            | root 926                               | `config.considerModelOrder` (default `NODES_AND_EDGES`)                                                                                                      |
| `elk.layered.mergeEdges`                                             | root 920, containers 431               | `config.mergeEdges` (default `false`)                                                                                                                        |
| `elk.layered.mergeHierarchyEdges`                                    | root 946                               | `true`                                                                                                                                                       |
| `elk.layered.unnecessaryBendpoints`                                  | root 927                               | `true`                                                                                                                                                       |
| `elk.layered.edgeRouting.selfLoopDistribution`                       | root 945                               | `EQUALLY`                                                                                                                                                    |
| `elk.layered.wrapping.multiEdge.improveCuts`, `…improveWrappedEdges` | root 943–944                           | `true` (inert: wrapping is off)                                                                                                                              |
| `elk.spacing.portsSurrounding`                                       | root 958, containers 456               | `[top=12,left=12,bottom=12,right=12]`                                                                                                                        |
| `elk.layered.spacing.edgeNodeBetweenLayers`                          | containers 418                         | `30`                                                                                                                                                         |
| `elk.spacing.edgeEdge`                                               | containers 421                         | `20`                                                                                                                                                         |
| `spacing.nodeNode`                                                   | containers 423                         | `50`                                                                                                                                                         |
| `elk.padding`                                                        | containers 428, 477, 484               | `[top=24,left=24,bottom=24,right=24]`; `[top=padTop,left=15,…]` for algorithm containers                                                                     |
| `nodeLabels.placement`                                               | containers 429                         | `[H_CENTER V_TOP, INSIDE]`                                                                                                                                   |
| `nodeSize.constraints`, `nodeSize.minimum`                           | 364–365, 466, 470                      | title floor for groups; algorithm containers                                                                                                                 |
| `elk.aspectRatio`, `elk.contentAlignment`, `elk.expandNodes`         | 473–475, 967                           | algorithm containers and rectpacking root                                                                                                                    |
| `elk.rectpacking.*`, rectpacking spacing                             | 232–242, 966                           | only when the algorithm is `elk.rectpacking`, which falls back to layered                                                                                    |
| `elk.portAlignment.default`                                          | node 1060                              | `CENTER`, for nodes with `spreadPorts`                                                                                                                       |
| `edgeLabels.inline`, `edgeLabels.placement`                          | edges 1127–1128, 1170                  | `true`; `CENTER` for the label, `TAIL`/`HEAD` for terminal labels                                                                                            |

Options not set by Mermaid keep ELK's defaults. On the root, that includes `spacing.nodeNode` (20), `elk.spacing.edgeEdge` (10), `elk.layered.spacing.edgeNodeBetweenLayers` (10), `elk.separateConnectedComponents` (unset, which means true), and the greedy switch (see §3).

### 2.2 `config.elk.*` keys that reach ELK

| Config key                    | Schema default    | Values                                                                                                                  | Reaches                                               |
| ----------------------------- | ----------------- | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `preset`                      | `default`         | `default`, `legacy`, `modelOrder`, `depthFirst`                                                                         | the preset table below                                |
| `nodePlacementStrategy`       | (preset)          | `SIMPLE`, `NETWORK_SIMPLEX`, `LINEAR_SEGMENTS`, `BRANDES_KOEPF`                                                         | root and container placement                          |
| `nodePlacementAlignment`      | (preset)          | `NONE`, `LEFTUP`, `LEFTDOWN`, `RIGHTUP`, `RIGHTDOWN`, `BALANCED`                                                        | `bk.fixedAlignment`                                   |
| `layeringStrategy`            | (preset)          | `NETWORK_SIMPLEX`, `LONGEST_PATH`, `LONGEST_PATH_SOURCE`, `COFFMAN_GRAHAM`, `MIN_WIDTH`, `STRETCH_WIDTH`, `INTERACTIVE` | P2                                                    |
| `layeringLayerBound`          | 4                 | number                                                                                                                  | `coffmanGraham.layerBound`                            |
| `cycleBreakingStrategy`       | (preset)          | `GREEDY`, `DEPTH_FIRST`, `INTERACTIVE`, `MODEL_ORDER`, `GREEDY_MODEL_ORDER`                                             | P1                                                    |
| `considerModelOrder`          | `NODES_AND_EDGES` | `NONE`, `NODES_AND_EDGES`, `PREFER_EDGES`, `PREFER_NODES`                                                               | `considerModelOrder.strategy`                         |
| `forceNodeModelOrder`         | `false`           | boolean                                                                                                                 | `crossingMinimization.forceNodeModelOrder`            |
| `mergeEdges`                  | `false`           | boolean                                                                                                                 | `mergeEdges`                                          |
| `keepEntryNodeOnTop`          | `false`           | boolean                                                                                                                 | `layerConstraint = FIRST` on entry nodes              |
| `orientFeedbackEdges`         | `true`            | boolean                                                                                                                 | Mermaid-side reversal of feedback edges (not ELK)     |
| `straightenEdges`, `lineHops` | `true`            | boolean / string                                                                                                        | Mermaid-side post-processing (not ELK), belongs to M5 |

Presets (`ELK_PRESETS`, `render.ts` 845–888), used where the config leaves a value unset:

| Preset       | Layering          | Root placement    | Container placement | Alignment  | Cycle breaking       |
| ------------ | ----------------- | ----------------- | ------------------- | ---------- | -------------------- |
| `default`    | `NETWORK_SIMPLEX` | `BRANDES_KOEPF`   | `BRANDES_KOEPF`     | `BALANCED` | `DEPTH_FIRST`        |
| `legacy`     | `NETWORK_SIMPLEX` | `BRANDES_KOEPF`   | `BRANDES_KOEPF`     | `NONE`     | `GREEDY`             |
| `modelOrder` | `NETWORK_SIMPLEX` | `NETWORK_SIMPLEX` | `BRANDES_KOEPF`     | `NONE`     | `GREEDY_MODEL_ORDER` |
| `depthFirst` | `NETWORK_SIMPLEX` | `NETWORK_SIMPLEX` | `BRANDES_KOEPF`     | `NONE`     | `DEPTH_FIRST`        |

### 2.3 Values reachable per phase

| Phase             | Reachable values                                                                                                           | Not reachable (not ported)                                                     |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| P1 cycle breaking | all 5                                                                                                                      | none                                                                           |
| P2 layering       | 7: `NETWORK_SIMPLEX`, `LONGEST_PATH`, `LONGEST_PATH_SOURCE`, `COFFMAN_GRAHAM`, `MIN_WIDTH`, `STRETCH_WIDTH`, `INTERACTIVE` | `BreadthFirstModelOrderLayerer`, `DepthFirstModelOrderLayerer`                 |
| P3 node ordering  | `LAYER_SWEEP` (the default, never set by Mermaid)                                                                          | `InteractiveCrossingMinimizer`, `NoCrossingMinimizer`                          |
| P4 node placement | 4: `SIMPLE`, `NETWORK_SIMPLEX`, `LINEAR_SEGMENTS`, `BRANDES_KOEPF`, plus all 6 BK alignments                               | `InteractiveNodePlacer`                                                        |
| P5 edge routing   | `ORTHOGONAL` (default)                                                                                                     | `PolylineEdgeRouter`, `SplineEdgeRouter` (`elk.edgeRouting` is catalogue-only) |

Options that stay at their melk default, and therefore must have that default: `layering.nodePromotion.strategy` (NONE), `compaction.postCompaction.strategy` (NONE), `wrapping.strategy` (OFF), `edgeLabels.sideSelection` (SMART_DOWN), `edgeLabels.centerLabelPlacementStrategy` (MEDIAN_LAYER), `crossingMinimization.greedySwitch.type` (TWO_SIDED, threshold 40), `crossingMinimization.greedySwitchHierarchical.type` (OFF), `feedbackEdges` (false), `highDegreeNodes.treatment` (false), `crossingMinimization.semiInteractive` (false), `interactiveReferencePoint` (CENTER), `separateConnectedComponents` (unset, read as true by `ComponentsProcessor`), `favorStraightEdges` (computed: true for ORTHOGONAL).

## 3. Processors that run for Mermaid's options

Composed from `GraphConfigurator.java` (`BASELINE_PROCESSING_CONFIGURATION`, `HIERARCHICAL_ADDITIONS`, `getPhaseIndependentLayoutProcessorConfiguration`) and the `getLayoutProcessorConfiguration()` of each selected phase class. The order inside a phase is set by `AlgorithmAssembler.build` (core `alg/`), and is checked in Stage C.

**Phase-independent (root, `INCLUDE_CHILDREN`, direction DOWN):**

- Always: `INNERMOST_NODE_MARGIN_CALCULATOR`, `LABEL_AND_NODE_SIZE_PROCESSOR` (before P4), `LAYER_SIZE_AND_GRAPH_HEIGHT_CALCULATOR` (before P5), `END_LABEL_SORTER` (after P5).
- `INCLUDE_CHILDREN`: `HIERARCHICAL_NODE_RESIZER` (after P5).
- `feedbackEdges` is false, so `PORT_SIDE_PROCESSOR` runs before P3.
- Direction DOWN, UP or LEFT: `GraphTransformer` pre (before P1) and post (after P5). RIGHT adds neither.
- `considerModelOrder` is not `NONE`: `SORT_BY_INPUT_ORDER_OF_MODEL` (before P3).
- Not added with these options: `NODE_PROMOTION`, `PARTITION_*`, `COMMENT_*`, `CENTER_LABEL_MANAGEMENT` / `END_NODE_PORT_LABEL_MANAGEMENT`, `HIGH_DEGREE_NODE_LAYER_PROCESSOR`, `SEMI_INTERACTIVE_CROSSMIN_PROCESSOR`, `SINGLE_EDGE_GRAPH_WRAPPER` / `BREAKING_POINT_*`, `HORIZONTAL_COMPACTOR`, `CONSTRAINTS_POSTPROCESSOR`.
- Greedy switch: `greedySwitchHierarchical.type` defaults to OFF, so the switch is off on a hierarchical root and in nested graphs. In a `SEPARATE_CHILDREN` container, `greedySwitch.type` (TWO_SIDED) applies while the container has fewer than 40 layerless nodes.

**Phase processors, one row per selected strategy:**

| Phase | Strategy (default preset)                     | Processors it adds                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P1    | `DEPTH_FIRST` (`DepthFirstCycleBreaker`)      | `REVERSED_EDGE_RESTORER`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| P2    | `NETWORK_SIMPLEX` (`NetworkSimplexLayerer`)   | `EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER`, `LAYER_CONSTRAINT_PREPROCESSOR`, `LAYER_CONSTRAINT_POSTPROCESSOR`                                                                                                                                                                                                                                                                                                                                                                                                                         |
| P3    | `LAYER_SWEEP` (`LayerSweepCrossingMinimizer`) | `IN_LAYER_CONSTRAINT_PROCESSOR`, `LONG_EDGE_JOINER`, `LONG_EDGE_SPLITTER`, `PORT_LIST_SORTER`                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| P4    | `BRANDES_KOEPF` (`BKNodePlacer`)              | `HIERARCHICAL_PORT_POSITION_PROCESSOR`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| P5    | `ORTHOGONAL` (`OrthogonalEdgeRouter`)         | `END_LABEL_PREPROCESSOR`, `END_LABEL_POSTPROCESSOR`, `HIERARCHICAL_PORT_CONSTRAINT_PROCESSOR`, `HIERARCHICAL_PORT_DUMMY_SIZE_PROCESSOR`, `HIERARCHICAL_PORT_ORTHOGONAL_EDGE_ROUTER`, `HYPEREDGE_DUMMY_MERGER`, `HYPERNODE_PROCESSOR`, `INVERTED_PORT_PROCESSOR`, `LABEL_DUMMY_INSERTER`, `LABEL_DUMMY_REMOVER`, `LABEL_DUMMY_SWITCHER`, `LABEL_SIDE_SELECTOR`, `NORTH_SOUTH_PORT_PREPROCESSOR`, `NORTH_SOUTH_PORT_POSTPROCESSOR`, `SELF_LOOP_PREPROCESSOR`, `SELF_LOOP_ROUTER`, `SELF_LOOP_POSTPROCESSOR`, `SELF_LOOP_PORT_RESTORER` |

The rows are the processors each phase class names in its `getLayoutProcessorConfiguration()`. Some may be added conditionally inside the processor, so the exact run list is confirmed against `AlgorithmAssembler` in Stage C.

Other strategies the config can select: the `INTERACTIVE` cycle breaker and layerer add `INTERACTIVE_EXTERNAL_PORT_POSITIONER`; every layerer adds the layer-constraint pair, and all but `INTERACTIVE` add `EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER`. Every cycle breaker adds `REVERSED_EDGE_RESTORER`. Every placer adds `HIERARCHICAL_PORT_POSITION_PROCESSOR`.

Connected components: `ComponentsProcessor` splits the graph before the processors run and combines the result afterwards, because `separateConnectedComponents` is unset and therefore read as true. Mermaid's option set reaches this path, so it is ported.

## 4. Java inventory

Counts from the pinned checkout (`.elk-src/elk`, commit `9bc93474e`). "Closure" is the set of files reached from Mermaid's entry points, see §4.2.

### 4.1 Modules as elkjs builds them

| Module                        | Files | Lines (incl. comments) | Role                                                                                 |
| ----------------------------- | ----: | ---------------------: | ------------------------------------------------------------------------------------ |
| `org.eclipse.elk.alg.layered` |   245 |                 59,876 | the algorithm (`src/` only)                                                          |
| `org.eclipse.elk.core`        |   108 |                 19,552 | options, math, util, alg interfaces, recursive engine (`src/` only)                  |
| `org.eclipse.elk.alg.common`  |    82 |                 14,270 | node spacing, network simplex, compaction, polyomino (`src/` only)                   |
| `org.eclipse.elk.graph`       |    40 |                 11,249 | EMF graph model (`ElkNode`, `ElkEdge`, …), generated `ElkGraphPackage` (`src/` only) |
| `org.eclipse.elk.graph.json`  |     1 |                    239 | JSON import and export (`ElkGraphJson`)                                              |

`alg.layered` subpackages (all files / lines):

| Package                               | Files |  Lines | Role                                                                                         |
| ------------------------------------- | ----: | -----: | -------------------------------------------------------------------------------------------- |
| `layered` (root)                      |     9 |  3,243 | `ElkLayered`, `LayeredLayoutProvider`, `GraphConfigurator`, `LayeredPhases`, debug utilities |
| `graph` (+ `transform`)               |    16 |  5,427 | `LGraph`, `LNode`, `LEdge`, `LPort`, `LLabel`, `ElkGraphTransformer`                         |
| `components`                          |    10 |  2,564 | connected-component split and combine                                                        |
| `compound`                            |     4 |  1,486 | hierarchical graphs: pre- and postprocessor                                                  |
| `intermediate` (+ 5 subpackages)      |    87 | 22,099 | processors shared by all phases                                                              |
| `options`                             |    33 |  2,180 | enums and internal property holders (option ids are generated)                               |
| `p1cycles`                            |     5 |    736 | cycle breaking                                                                               |
| `p2layers`                            |     9 |  2,451 | layering                                                                                     |
| `p3order` (+ `counting`)              |    21 |  5,127 | layer sweep crossing minimisation                                                            |
| `p4nodes` (+ `bk`)                    |    11 |  4,737 | node placement                                                                               |
| `p5edges` (+ `orthogonal`, `splines`) |    19 |  6,703 | edge routing                                                                                 |
| `compaction`                          |    21 |  3,123 | post-compaction (not reached)                                                                |

### 4.2 Closure for Mermaid's option set

Built by a name-based walk over the Java sources (`scratchpad` script, not committed): start from the entry points (driver, configuration, graph import, the phase classes and processors listed in §3, `ComponentsProcessor`, `RecursiveGraphLayoutEngine`), and follow every class name mentioned in code. Cut at debug utilities, the splines router, wrapping, compaction, comments, validation, testing and the EMF-generated classes. Name clashes over-approximate the result.

| Module                        |   Files |      Lines | Code lines (no comments, no blanks) |
| ----------------------------- | ------: | ---------: | ----------------------------------: |
| `org.eclipse.elk.alg.layered` |     163 |     40,646 |                                     |
| `org.eclipse.elk.core`        |      68 |     13,313 |                                     |
| `org.eclipse.elk.alg.common`  |      37 |      8,679 |                                     |
| `org.eclipse.elk.graph`       |      35 |     10,014 |                                     |
| `org.eclipse.elk.graph.json`  |       1 |        239 |                                     |
| **Total**                     | **304** | **72,891** |          **≈ 35,600** (upper bound) |

Layered closure by subpackage: `intermediate` 50 files / 12,978 lines; `p3order` 19 / 4,833; `p4nodes` 10 / 4,610; `graph` 16 / 5,427; `p5edges` 11 / 2,716; `options` 26 / 1,991; `p2layers` 7 / 1,998; `components` 10 / 2,564; `compound` 4 / 1,486; `p1cycles` 5 / 736; root 5 / 1,297.

Core closure: `util` 22 / 4,851; `math` 7 / 2,978; `options` 16 / 1,442; `data` 8 / 2,030; `alg` 7 / 846; `labels` 1 / 41; top-level engine and providers 7 / 1,125. `alg.common` closure: `nodespacing` 26 files, `networksimplex` 4, `overlaps` 3, `polyomino` 1, `spore` 1, plus two root files. The `alg.common` `compaction` package is not in the closure. Graph closure: `ElkGraphPackage` 2,105 lines, `impl` 4,940, `properties` 578, `util` 999, the other model classes about 1,400 (all with comments; about 8.9 k code lines in total).

Some closure members are name clashes and not real dependencies, for example the root `alg.common` `RectilinearConvexHull`, which the walk reached through the layered compaction code. The closure is an upper bound.

### 4.3 Dependencies on core, graph and alg.common

Import lines from `alg.layered` into the other modules (counts of import lines, not of distinct uses):

- `org.eclipse.elk.core.options`: 181 (of which `PortSide` 62).
- `org.eclipse.elk.core.util`: 123. `org.eclipse.elk.core.alg`: 121 (`AlgorithmAssembler`, `LayoutProcessorConfiguration`, processor and phase interfaces). `org.eclipse.elk.core.math`: 109 (`KVector`, `KVectorChain`, `ElkMargin`, `ElkPadding`).
- `org.eclipse.elk.graph.properties`: 27. `org.eclipse.elk.graph`: 20. `org.eclipse.elk.core.labels`: 8. `org.eclipse.elk.core.testing`: 3 (not ported).
- `org.eclipse.elk.alg.common.nodespacing`: 5 and `nodespacing.cellsystem`: 6, `networksimplex`: 12. `org.eclipse.elk.alg.common.compaction.oned`: 20, imported only by the compaction code that is not ported.
- Layered's own compaction (`org.eclipse.elk.alg.layered.compaction.*`) is used by the horizontal compactor, which is not ported. `options/ConstraintCalculationStrategy` names classes from that package; the enum stays because the melk metadata declares it, and the referenced classes do not.

The Rust port keeps property semantics (`IPropertyHolder`, `MapPropertyHolder`, option defaults and inheritance) and the option lookup by full id and by suffix, which elkjs uses for JSON `layoutOptions`. It does not keep the EMF model: `org.eclipse.elk.graph` becomes plain Rust structs, and the JSON reader and writer use serde.

### 4.4 Java lines that are not ported

- Not reached by Mermaid's option set: `p5edges/splines` (3,420 lines), the polyline router, `intermediate/wrapping` (2,123), `compaction` (`layered` 3,123, `alg.common` compaction and polyomino about 4,000), `core/comments` (2,545), `core/validation` (473), `core/testing` (243), the interactive node placer and crossing minimiser, `HorizontalCompactor`, partition processors, label management, high-degree-node processing, semi-interactive crossing minimisation, `InteractiveLayeredGraphVisitor`, `BreadthFirst`/`DepthFirstModelOrderLayerer`.
- Debug and log output: `DebugUtil`, `JsonDebugUtil`, `DotDebugUtil` and the `DebugUtil.logDebugGraph` calls, which become no-ops.
- Other algorithms, per ADR 0006: `alg.box`, `alg.rectpacking`, `alg.force`, `alg.stress`, `alg.mrtree`, `alg.radial`, `alg.spore`, `alg.disco`, `alg.libavoid`, `alg.graphviz`, `alg.topdownpacking`.
- The melk metacompiler, and `org.eclipse.elk.graph.json.text` / `.ui` / `.text.ide`.
- Java iterator and visitor plumbing is replaced by idiomatic Rust where that does not change results. Each replacement is noted in the mapping table.

## 5. Size estimate

- Closure, upper bound: 304 Java files, 72.9 k lines, about 35.6 k code lines.
- Without the graph module (about 8.9 k code lines, replaced by plain structs), about 26.7 k Java code lines remain. Rust needs less boilerplate but more explicit types, so the estimate is **20–28 k lines of Rust**, including a generated option registry of roughly 1.5 k lines.
- These are estimates from the name-based closure. The mapping table in §9 gives real numbers once Stage C is under way.

## 6. Porting order

The order follows the call graph and gives something runnable early. Each step is tested against the differential corpus restricted to what the step can already handle (Stage B).

1. **Input and options.** Read the elkjs JSON graph (serde), option ids and defaults generated from `Core.melk`, `Layered.melk`, `Labels.melk`, option lookup by full id and by suffix, the property holder.
2. **Graph model.** `LGraph`, `LNode`, `LEdge`, `LPort`, `LLabel` and the transformer from the input graph, plus the reverse direction that writes coordinates back.
3. **Driver and configuration.** `LayeredLayoutProvider`, `ElkLayered` (`doLayout`, `doCompoundLayout`, the hierarchical loop), `GraphConfigurator`, `LayeredPhases`, `AlgorithmAssembler` and `LayoutProcessorConfiguration` (core `alg/`), `RecursiveGraphLayoutEngine` (for `SEPARATE_CHILDREN`).
4. **Components.** `ComponentsProcessor`, split and combine, the simple row placer.
5. **Shared intermediate processors.** Margin and size calculators, `LabelAndNodeSizeProcessor`, `LayerSizeAndGraphHeightCalculator`, `PortSideProcessor`, the direction `GraphTransformer`, `EndLabelSorter`, `HierarchicalNodeResizingProcessor`, `ReversedEdgeRestorer`.
6. **P1 cycle breaking.** The five strategies, with `ModelOrder` and `GreedyModelOrder` sharing the model-order code.
7. **P2 layering.** `NetworkSimplexLayerer` and `alg.common/networksimplex`, then the other six layerers.
8. **P3 ordering.** `LayerSweepCrossingMinimizer`, counting, `BarycenterHeuristic`, port distributors, `SortByInputModelProcessor`, `LongEdgeSplitter` and `LongEdgeJoiner`, `PortListSorter`, then the greedy switch.
9. **P4 placement.** `SimpleNodePlacer`, `LinearSegmentsNodePlacer`, `NetworkSimplexPlacer`, then `BKNodePlacer` with its aligner, compactor and threshold strategy.
10. **P5 routing.** `OrthogonalEdgeRouter` and `p5edges/orthogonal` (routing direction, hyperedge segments), then the self-loop, label and north/south processors.
11. **Hierarchy.** `CompoundGraphPreprocessor` and `CompoundGraphPostprocessor`, cross-hierarchy edges, `HierarchicalPort*` processors (confirm whether Mermaid graphs ever reach them).
12. **Graph export.** Coordinates, edge sections, labels, JSON writer.

Increments for the corpus: first graphs with nodes only, then acyclic edges, then cycles, labels, self-loops, components, containers with `SEPARATE_CHILDREN`, cross-hierarchy edges.

## 7. Crate layout (planned)

`crates/ferrigura-elk/src/`, one module per Java package, the names kept:

```
lib.rs              public entry: layout(json graph) -> json graph
json.rs             elkjs JSON reader and writer (serde)            <- elk.graph.json
options/            ids, defaults, enums                             <- generated from *.melk
graph/              ElkNode and friends, property holder            <- elk.graph (EMF replaced)
core/               RecursiveGraphLayoutEngine, math, util, alg, labels
common/             alg.common: nodespacing, networksimplex, utils
layered/            ElkLayered, LayeredLayoutProvider, GraphConfigurator, LayeredPhases
layered/graph/      LGraph and friends, transform
layered/components/, compound/, intermediate/, p1cycles/, p2layers/, p3order/, p4nodes/, p5edges/
```

Files that are not ported keep no placeholder.

## 8. Open questions

1. **Scope rule.** Confirm the reading of §1: the Mermaid-reachable option set, not all of ELK Layered. This leaves splines, wrapping, compaction, comments and validation out, about 17 k Java lines with comments (§4.4).
2. **Source pin.** The commit `9bc93474e` is an inference (see [SOURCES.md](./SOURCES.md)). Accept it, or pin the `v0.9.1` tag instead? The difference Mermaid can see is the top-down change, which Mermaid never sets.
3. **Fallback fixtures.** Seven fixtures use `layout: elk.stress`, `elk.mrtree`, `elk.box` or `elk.rectpacking`. They fall back to layered, so the differential run cannot compare them with elkjs. Propose: exclude them from the ELK differential run and cover them in the geometry comparison (M5).

## 9. Mapping table

Java class to Rust module, as ported. `partial` means the nodes-only path works and the rest is not ported; `replaced` means the Java code is not ported as such.

| Java class (package)                                                         | Rust                                                                                     | Status                                                                              |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `ElkNode` family, `org.eclipse.elk.graph` (EMF)                              | `graph::ElkNode`, `ElkPort`, `ElkLabel`, `ElkEdge`                                       | replaced (plain structs); edges not imported                                        |
| `JsonImporter`, `ElkGraphJson`, `JsonAdapter`                                | `json` (serde types), `graph::json_import`                                               | partial: nodes, ports, labels, options; no edges                                    |
| `LayoutMetaDataService.getOptionDataBySuffix`, `LayoutOptionData.parseValue` | `core::data::option_by_suffix`, `parse_value`                                            | ported                                                                              |
| melk metadata (generated `*Options`, `*MetaDataProvider`)                    | `core::data::registry` (generated), `core::data::enums` (generated)                      | generated by `tools/`                                                               |
| `MapPropertyHolder`                                                          | `core::data::property::PropertyHolder`                                                   | ported (defaults from melk text)                                                    |
| `BasicProgressMonitor`, `collectLogs`                                        | `core::util::progress`                                                                   | ported                                                                              |
| `AlgorithmAssembler`, `LayoutProcessorConfiguration`                         | `core::alg::LayoutProcessorConfiguration`; ordering in `layered::configurator::assemble` | partial (assembly ported, not the factory cache)                                    |
| `RecursiveGraphLayoutEngine`                                                 | `core::recursive`                                                                        | partial: root level only                                                            |
| `Math`/`round_int`, `java.util.Random`, GWT merge sort                       | `java::math`, `java::random`, `java::sort`                                               | ported, checked against elkjs values                                                |
| `ElkLayered`, `LayeredLayoutProvider`                                        | `layered::driver`                                                                        | partial: compound layout only; `doLayout` (components) not ported                   |
| `GraphConfigurator`                                                          | `layered::configurator`                                                                  | partial: no greedy switch, node promotion, interactive, wrapping                    |
| `ElkGraphImporter`, `ElkGraphTransformer`                                    | `layered::importer`                                                                      | partial: nodes only                                                                 |
| `LayeredPhases`, `IntermediateProcessorStrategy`                             | `core::data::enums` (generated), `layered::intermediate::Step`                           | ported                                                                              |
| `CycleBreakingStrategy` processors                                           | `layered::phases::PhaseStep` (`DepthFirstCycleBreaker`)                                  | partial: names and config; run is nodes-only no-op                                  |
| `NetworkSimplexLayerer`                                                      | `layered::phases` (`network_simplex_layering`)                                           | partial: one subtask per component, layer 0; `alg.common/networksimplex` not ported |
| `LayerSweepCrossingMinimizer`                                                | `layered::phases::PhaseStep::CrossingMinimization`                                       | partial: name and config only                                                       |
| `BKNodePlacer` (`alg.layered/p4nodes/bk`)                                    | `layered::phases::PhaseStep::NodePlacement`                                              | partial: name only; logs and coordinates missing                                    |
| `OrthogonalEdgeRouter`                                                       | `layered::phases::PhaseStep::EdgeRouting`                                                | partial: name only; config conditions done                                          |
| Other intermediate processors                                                | `layered::intermediate::run`                                                             | partial: nodes-only names; others unsupported                                       |
| `CompoundGraphPreprocessor` / `Postprocessor`                                | `layered::driver`                                                                        | partial: names; nodes-only no-op                                                    |
| `ElkGraphLayoutTransferrer`, `ElkGraphTransformer.applyLayout`               | (not yet)                                                                                | not ported: output positions are the input's                                        |

Not yet ported, with the checkpoint in mind: `alg.common/networksimplex` (`NetworkSimplex` itself), the Brandes & Koepf placer and its logs, edge import, `doLayout` with `ComponentsProcessor`, nested graphs, and every processor the nodes-only path skips.

## 10. Stage C status

The checkpoint from the coordinator: a corpus graph runs through the Rust driver with the same processor trace as elkjs, and lays out the same. **Met for all 96 supported corpus cases**: node boxes, edge sections and label boxes match elkjs exactly (tolerance 0, `tests/differential.rs`), and the full trace matches with its logs (`tests/trace.rs`).

Corrections to §3, which lists the configurations as if they were unconditional:

- The processor configurations are **conditional on graph properties**. `OrthogonalEdgeRouter` adds processors only for hyperedges, self loops, north/south and hierarchical ports, labels, and end labels. `BKNodePlacer` (and the other placers) add `HIERARCHICAL_PORT_POSITION_PROCESSOR` only for `EXTERNAL_PORTS`. The graph properties come from the import, and a nodes-only graph has none. So the nodes-only list is the minimal one: the phase-independent baseline, the phase tables of the chosen strategies, and nothing else.
- The layering phases share one table, except `INTERACTIVE`. `NetworkSimplexLayerer` and the other layerers add `EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER` before P1, `LAYER_CONSTRAINT_PREPROCESSOR` before P2 and `LAYER_CONSTRAINT_POSTPROCESSOR` before P3.
- `DepthFirstCycleBreaker` adds `REVERSED_EDGE_RESTORER` **after P5**, not before.
- `LayerSweepCrossingMinimizer` adds `LONG_EDGE_SPLITTER` and `PORT_LIST_SORTER` before P3, `IN_LAYER_CONSTRAINT_PROCESSOR` before P4, and `LONG_EDGE_JOINER` after P5.

The assembly is ordinal-sorted within each slot, and that reproduces elkjs's order (for example "Edge splitting", "Port side processing", "Port order processing", "Sort By Input Model NODES_AND_EDGES", "Minimize Crossings BARYCENTER").

### Findings

- **Option ids.** The registry is generated from the same ten melk files that elkjs bundles, and suffix lookup runs over all of them, as `getOptionDataBySuffix` does. The first version of the generator read the melk nesting line by line and got 30 ids wrong; it now tracks brace blocks, and every registered id is found as a literal in the elkjs 0.9.3 bundle (the algorithm ids aside).
- **Mermaid's `layeringLayerBound` key is effective.** `elk.layered.layering.coffmanGraham.layerBound` is a suffix of the registered id `org.eclipse.elk.layered.layering.coffmanGraham.layerBound`. On an 8-node graph with edges and COFFMAN_GRAHAM, elkjs gives 8 layers with bound 1 and 6 with bound 2 under that key, and 5 layers (no effect) under the shorter spellings `elk.layered.coffmanGraham.layerBound` and `org.eclipse.elk.layered.coffmanGraham.layerBound`. So the Mermaid key works; the shorter forms do not.
- **Node promotion** is `org.eclipse.elk.layered.layering.nodePromotion.strategy`, under `layering` (the corrected reader).
- **elkjs mutates its input** (the Stage B agent's finding): it adds `x`, `y`, `width`, `height`, `logging` and `$H` to the object it is given. The corpus stores the input before layout, so the port reads the same graph elkjs read.
- **Output `$H`.** elkjs adds GWT identity hashes to its output. The corpus strips them; the port does not produce them.
- **Two generator and reader bugs, both fixed.** The melk reader returned no default for `new ElkPadding(12)`, so the root padding was absent; it now reads the all-sides `ElkPadding` and `ElkMargin` constructors. The enum generator let a nested `SelfHyperLoopLabels.Alignment` replace the core `Alignment`, which lost `AUTOMATIC`; a top-level enum in its own file now wins. Only that enum changed in the generated file.
- **JSON numbers.** serde_json's default float parser is not correctly rounded. The inputs and the reference outputs are read through it, so both are read with `float_roundtrip`, which makes them exact as JavaScript reads them. Until that was enabled, the differential test reported differences of about 1e-14 on cases that were otherwise identical. The one-ulp Brandes & Koepf size in `state-diagram-v2__elk-start-end-edge-centering__0` had the same cause, a width of `14.013293266296387` read one ulp away.

### Coverage

- Corpus: 96 of 136 cases pass the driver. The other 40 report `Unsupported`: 24 are nested graphs, 6 need end labels, 4 are UP or LEFT directions, 3 have self loops, 1 has layer constraints, and 2 are non-layered algorithms (`elk.box`, `elk.rectpacking`).
- All 96 match elkjs exactly in coordinates, edge sections and label boxes (`coordinates_match_elkjs`, tolerance 0, floor 96). All 96 match the full trace, names and logs (`supported_traces_match_elkjs_including_logs`, floor 96).
- Crate tests: 137 unit tests; the differential and trace tests pass, with their diagnostics still ignored; `clippy -D warnings` is clean.

### Next, in order

1. UP and LEFT directions through `GraphTransformer`: 4 cases.
2. End labels (head and tail labels): 6 cases.
3. Self loops (3) and layer constraints (1).
4. Nested graphs (24): `CompoundGraphPreprocessor` and `Postprocessor`, the hierarchical port processors, and `RecursiveGraphLayoutEngine` for `SEPARATE_CHILDREN`. The two non-layered algorithms stay excluded.
5. Indexed hash order (`indexmap` with `shift_remove`) once a processor iterates a hash collection; none does yet.

### Edge pipeline: state and order

The simplest corpus case with edges, `requirement__a-simple-requirement-diagram-elk-layout__0` (two nodes, one edge with a centred label), needs the 26 processors that its elkjs trace lists, in order: transformation to the input direction, edge and layer constraint reversal, depth-first cycle removal, label dummy insertions, layer constraint pre-processing, network simplex layering, layer constraint post-processing, edge splitting, port side processing, port order processing, sort by input model (with ports), minimize crossings, layer constraint edge reversal, node and port label placement and node sizing, node margin calculation, label dummy switching, label side selection, Brandes & Koepf, layer size calculation, orthogonal edge routing, edge joining, label dummy removal, sort end labels, restoring reversed edges, the resizer, and the transformation back.

Done on this branch, in order: edge import with a port per end; the depth-first cycle breaker and the reversed edge restorer; label dummy insertion, the long edge split and join; port sides and port order; the DOWN transform; network simplex and the layerer; the sort by input model, with the model order comparators (`preserveorder.rs`); the layer sweep with edges; the transfer of edge sections and label boxes; the orthogonal router; Brandes & Koepf with edges; the label dummy switcher, side selector and remover; node margins and the label and node sizes; the port-based alignment of `placeNodesHorizontally`; and the configurator, which now accepts edges and centred labels.

Still open: the features in the coverage list above.
