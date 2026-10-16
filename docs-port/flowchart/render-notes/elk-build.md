# How Mermaid builds the elkjs graph from LayoutData

Research notes for M5 (flowchart render in Rust). Read-only study; no code changed.

- Source: `packages/mermaid/src/` in this worktree (HEAD `e8af08065`). Paths below are relative to `packages/mermaid/src/` unless absolute. `elk/render.ts` is shortened to `render.ts`, and `rendering-util/layout-algorithms/elk/` to `elk/`.
- Line numbers are from the files as they are now. `render.ts` is 3021 lines; this study covers 1–1325 in full, the `render` export, and the post-layout passes 1325–2938 at function level.
- elkjs: 0.9.3, from `packages/mermaid/node_modules/elkjs` (the worktree has no `node_modules`). The Rust port follows that build (`crates/ferrigura-elk/src/lib.rs:3-4`).
- **Probe** = a `node -e` run of the bundled elkjs on a hand-built graph. Nothing was written to disk.
- **Unverified** = read from code only; not run in a browser or in the Rust port.
- PLAN references are to `docs-port/elk/PLAN.md` §2 (option keys). Option keys are not repeated here.

---

## 0. Trickiest rules (short form)

1. A leaf's ELK size is the `getBBox()` of its rendered shape element, measured before layout (`createGraph.ts:67-69`). ELK never sees a leaf label.
2. Every edge is put in the root `edges` array (`render.ts:720`, `1113`). elkjs moves each edge into its lowest-common-ancestor container, and section coordinates are then relative to that container. Probe-confirmed; see §2.5 and §4.4.
3. A group keeps `SEPARATE_CHILDREN` (and so its own `elk.direction` or container algorithm) only if no cross-boundary edge touches it or a descendant. Otherwise `setIncludeChildrenPolicy` forces `INCLUDE_CHILDREN` on the endpoint and its ancestors up to the common ancestor (`render.ts:1224-1277`). Under `INCLUDE_CHILDREN` the group's `elk.direction` is inert (probe). A `SEPARATE_CHILDREN` group with a boundary-crossing edge makes elkjs throw (probe).
4. Feedback reversal (`orientFeedbackEdges`, default on) reverses only edges that close a cycle through a subgraph that ELK lays out as one node (not `view: collapsed`). It runs after the INCLUDE policy and before ELK; `layoutReversed` is undone in `applyElkEdgeLayout` (`render.ts:723-725`, `1992-1997`).
5. An empty subgraph reaches ELK as a childless compound node with no size and no options. elkjs gives it 0×0 (probe), so a title-only frame is painted (§4.6, unverified visually).

---

## 1. Pipeline

### 1.1 Dispatch and producer

| Step             | Where                                                 | What it does                                                                                                                                                                                                                                                                      |
| ---------------- | ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layout selection | `rendering-util/render.ts:57-63`, `81-103`            | `layout: elk` maps to `algorithm: 'elk.layered'`. `elk.stress/force/mrtree/sporeOverlap/box/rectpacking` are registered with their own name as `algorithm` (`ELK_ALGORITHMS`, `elk/algorithms.ts:5-12`). Nothing in `packages/mermaid/src` maps them to layered (see §7, item 1). |
| Domain ids       | `rendering-util/render.ts:88-93`                      | `node.domId = diagramId-domId`. `createElkNode` deletes `domId` again (`render.ts:1043`).                                                                                                                                                                                         |
| Producer         | `diagrams/flowchart/flowRenderer-v3-unified.ts:28-43` | Sets `type`, `layoutAlgorithm`, `direction`, `nodeSpacing`/`rankSpacing` (unused by ELK, §6), `markers`, `diagramId`.                                                                                                                                                             |
| LayoutData       | `diagrams/flowchart/flowDb.ts:1119-1347` (`getData`)  | See §3 for order and §2 for fields.                                                                                                                                                                                                                                               |

### 1.2 Renderer stages

`createCommonLayoutRenderer` (`layout-algorithms/common/index.ts:107-204`) runs, in order: insert markers (`:141-146`), `clearLayoutRenderState` (`:147`), then `prepareLayout` (`:159-161`), `measureLayout` (`:166-168`), `runLayoutCore` (`:173-175`), paint (`:185-196`), `afterPaint` (`:199`). The ELK export is at `render.ts:749-763`.

| Stage           | Entry                                                                                                                              | Reads (LayoutData)                                                                                                                                    | Writes                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **prepare**     | `prepareLayoutForElk` `render.ts:678-685` → `applyElkEdgeRenderData` `render.ts:2650-2668` → `buildEdgeData` `render.ts:2715-2767` | Edge: `type`, `text`/`label`, `stroke`, `arrowTypeStart/End`, `curve`/`interpolate`, `defaultInterpolate`, `defaultStyle`, `length`. Config: `curve`. | Per edge via `Object.assign`: `minlen` (`:2726`, never read by ELK, §6), `text`, `arrowhead`, `arrowTypeStart/End`, `startLabelRight`, `endLabelLeft`, `thickness`, `pattern`, `style`, `labelStyle`, `classes`, `curve` (`:2745`, overwritten later), `arrowheadStyle`, `labelpos`, `labelType`, `label` with `<br>`→`\n` (`:2761-2764`).                                                                         |
| **measure**     | `defaultMeasureLayout(…, {unwrapGroupLabels: true})` `render.ts:757-758` → `createGraphWithElements` `createGraph.ts:122-215`      | DOM element, edges, nodes (after prepare, so labels are already `\n`-normalised).                                                                     | Leaves: `width`, `height` (`createGraph.ts:68-69`). Groups: `labelBBox` (`:54`, or `{0,0}` for no title, `:57`). Edges: `width`, `height` (`edges.js:136-137`), `labelStyle` (`edges.js:78`), `terminalLabelSizes` (`edges.js:264-265`, class diagrams only). DOM is kept for leaves and edge labels and removed for group labels (`createGraph.ts:55`). The graphlib graph is built but not read by the ELK path. |
| **layout core** | `runElkLayoutCore` `render.ts:687-702`                                                                                             | ELK-side copies only, plus LayoutData edges and nodes after the run.                                                                                  | See below.                                                                                                                                                                                                                                                                                                                                                                                                         |
| build           | `buildElkGraphFromLayoutData` `render.ts:704-729`                                                                                  | LayoutData nodes/edges, config, `direction`.                                                                                                          | Builds copies (`createElkNode` `render.ts:1042` spreads the node; `addEdgesToElkGraph` `render.ts:1113` spreads the edge). LayoutData is not mutated here, apart from a no-op `edge.id = linkId` (`render.ts:1102`).                                                                                                                                                                                               |
| run             | `runElkLayout` `render.ts:1282-1323`                                                                                               | ELK graph                                                                                                                                             | elkjs mutates and returns the same object (probe). Dev-only capture hook at `:1298-1317`.                                                                                                                                                                                                                                                                                                                          |
| apply positions | `applyElkNodePositions` `render.ts:1509-1579`                                                                                      | ELK output nodes                                                                                                                                      | LayoutData node: `x`, `y` (centre), `width` (group: `max(ELK width, labelData.width)`, `:1544-1546`), `height`, `labelData` and `labels` copied back (`:1552-1553`). ELK-side `offset` (absolute top-left), `x`, `y`, `width`, `height` (`:1524-1538`).                                                                                                                                                            |
| frame fit       | `evenGroupFrames` `render.ts:1423-1507`                                                                                            | Group children boxes, edges inside the group                                                                                                          | Group `offset`, `width`, `height`, `x`, `y`; `elkOrigin` kept (`:1488`). Only ever shrinks (`:1456-1458`).                                                                                                                                                                                                                                                                                                         |
| edge layout     | `applyElkEdgeLayout` `render.ts:1931-2126`                                                                                         | ELK output edges, `nodeDb`                                                                                                                            | LayoutData edge: `points`, `curve` (`'rounded'` at `:2078`, `'linear'` for the fallback at `:2027`), `x`, `y` (ELK label centre), `terminalLabelCenters`. Node alignment prepass moves small nodes (`:1951-1982`). Straightening, label slides and separation run after (`:2102-2125`).                                                                                                                            |
| order           | `orderNodesForElkPaint` `render.ts:765-779`                                                                                        | LayoutData nodes                                                                                                                                      | Sorts `data4Layout.nodes` in place: groups first, by depth.                                                                                                                                                                                                                                                                                                                                                        |
| **paint**       | `paintLayoutData` `common/index.ts:220-245`                                                                                        | Sorted nodes, edges, `measure.groups`                                                                                                                 | Leaves: `positionNode` (`nodes.ts:77-100`, `translate(node.x,node.y)`). Groups: `insertCluster` (`clusters.js:841-846`, `rect` `:15-135`), which also writes LayoutData `node.width`/`height` from the painted rect (`clusters.js:123-126`), `offsetX`, `offsetY`, `diff`, and `intersect` (`:130`). Edges: `insertEdge` on a copy `{...edge}` (`common/index.ts:292-302`), edge label transform (`:328-351`).     |
| **afterPaint**  | `applyElkLineJumps` `elk/lineHops.ts:39-66`                                                                                        | `edge.points`, `curve`, `arrowType*`; `config.elk.lineHops`                                                                                           | Rewrites the `d` of crossing edges (`lineHops.ts:61-65`).                                                                                                                                                                                                                                                                                                                                                          |

`paintOptions.skipIntersect: true` (`render.ts:760-762`) means paint does not clip edges to node outlines. All clipping is done in layout core (§4.11).

### 1.3 Where clipping and ordering happen

- Clipping, sizing and routing of edges all happen in layout core, not paint.
- Paint order is group frames first (by depth), then leaves, then edges, all in LayoutData order (§3).

---

## 2. The elkjs JSON

### 2.1 Root graph

`createRootElkGraph` (`render.ts:905-989`) returns `{id: 'root', layoutOptions, children: [], edges: []}`. Keys and values: PLAN §2.1 rows. This section adds the assignment order and the JSON value types.

Assignment order, last writer wins:

1. Literal (`render.ts:911-962`).
2. Rectpacking overrides, only if the root algorithm is `elk.rectpacking` (`render.ts:965-970`).
3. `rootLayoutOptions` from `context.options` (`render.ts:974-976`, read at `render.ts:814-816`). Set only by the DDLT sweep. The dispatcher passes only `{algorithm}` (`rendering-util/render.ts:100-102`), so it is undefined in production.
4. `PLACEMENT_OPTIONS`, `EDGE_ROUTING_OPTIONS`, `ROOT_EXPERIMENT_OVERRIDES` (`render.ts:981-986`). All three are empty in this worktree (`elk/elkOptionCatalogue.ts:44`, `:162`, `:267`; grep finds no active entry).
5. `elk.direction` is then set from `dir2ElkDirection(data4Layout.direction ?? 'DOWN')` (`render.ts:715-716`), replacing the literal `'DOWN'` (`render.ts:921`).

Value types written into the JSON:

| Kind         | Examples                                                                                                                                                                                                                 | Source                                    |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- |
| JSON number  | `spacing.baseValue` 40 (root, `:922`) and 24 (group, `:397`); `spacing.nodeNode` 50 (`:423`); `elk.spacing.edgeEdge` 20 (`:421`); `elk.layered.spacing.edgeNodeBetweenLayers` 30 (`:418`); `layeringLayerBound` (`:932`) | literals; `config.elk.layeringLayerBound` |
| JSON boolean | `elk.layered.unnecessaryBendpoints`, `mergeHierarchyEdges`, `wrapping.*` (`:927`, `:943-946`)                                                                                                                            | literals                                  |
| JSON string  | enum values, `elk.padding` `[top=…]`, `nodeSize.minimum` `(W, 0)`, `elk.aspectRatio: '2.0'` (`:473`), `elk.expandNodes: 'true'` (`:475`)                                                                                 | literals and template strings             |
| undefined    | `mergeEdges`, `forceNodeModelOrder`, `considerModelOrder`, `layeringLayerBound`, `layeringStrategy`, … when the config is unset                                                                                          | `render.ts:920`, `924-926`, `932`         |

Notes:

- elkjs turns numbers and booleans into text before parsing (`elk-worker.js` `optsToCfg`, around line 79907-79908). `string_val` in `crates/ferrigura-elk/src/json.rs:195-207` does the same.
- A value that is JSON `null` makes the importer fail: probe gives `java.lang.Error: Severe implementation error in the Json to ElkGraph importer`. `string_val` rejects `null` as well (`json.rs:195-207`). The Mermaid `??` fallbacks prevent this for most keys. `elk.layered.layering.coffmanGraham.layerBound` (`render.ts:932`) and the other keys without `??` would fail if the config held `null` (schema validation presumably prevents that; unverified).
- An undefined value is ignored (probe: same result as omitting the key). `JSON.stringify` in the capture hook (`render.ts:1301`) drops it too. Both are equivalent for the port.

### 2.2 Groups with children (`isGroup: true`)

| Field                                                                              | Value                                                                                | Source                                |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------- |
| `id`                                                                               | LayoutData id                                                                        | `render.ts:1042` (spread)             |
| `children`                                                                         | ordered list of ELK children (§3)                                                    | `render.ts:1034-1036`, `1045-1047`    |
| `width`, `height`, `x`, `y`                                                        | **absent** (deleted after build)                                                     | `render.ts:1217-1220`                 |
| `labels`                                                                           | `[{text: node.label, width: labelData.width ?? 50, height: labelData.height ?? 50}]` | `render.ts:1203-1209`                 |
| `labelData`                                                                        | `{width, height, wrappingWidth}` (not an ELK key; read back later)                   | `render.ts:1068-1087`, `1037`         |
| `layoutOptions`                                                                    | full object from `buildSubgraphLayoutOptions` (below); replaces any earlier value    | `render.ts:1211-1216`                 |
| `padding`, `shape`, `dir`, `metadata`, `cssClasses`, `look`, `colorIndex`, `label` | spread from LayoutData (flowDb: `padding: 8`, `shape: 'rect'`, `dir`)                | `flowDb.ts:1257-1268`; ignored by ELK |
| `ports`                                                                            | none                                                                                 |                                       |

`labelData`: `width = labelBBox.width`, `height = max(0, labelBBox.height - 2)` (`render.ts:1074-1079`). The `-2` is a fixed correction. `wrappingWidth = node.wrappingWidth ?? config.flowchart.wrappingWidth`.

`buildSubgraphLayoutOptions` (`render.ts:369-501`), in the order the keys are written:

1. `groupTitleSizeOptions(node)` (`:396`, def `:359-367`). For shapes where `clusterPaintsTitle` is true (every flowchart group, `clusters.js:823-830`): `nodeSize.constraints = '[MINIMUM_SIZE, NODE_LABELS]'` and `nodeSize.minimum = '(W, 0)'` with `W = labelData.width + padding` (`:347-353`). Padding is 8 for flowchart groups (`flowDb.ts:1245`, `1263`).
2. Base: `spacing.baseValue` 24 (`:397`), `elk.layered.spacing.edgeNodeBetweenLayers` 30 (`:418`), `elk.spacing.edgeEdge` 20 (`:421`), `spacing.nodeNode` 50 (`:423`), `elk.padding` `[24,24,24,24]` (`:428`), `nodeLabels.placement` (`:429`), `elk.layered.mergeEdges` (`:431`), `bk.fixedAlignment` (`:432-433`), `nodePlacement.strategy` (`:441-442`), `cycleBreaking.strategy` (`:449`), `networkSimplex.nodeFlexibility` (`:452`), `spacing.portsSurrounding` (`:456`).
3. Container algorithm, only if `node.metadata.algorithm` is in `CONTAINER_ALGORITHMS` (`:462`, set `:287-296`). Sets `nodeSize.constraints`, `nodeSize.minimum = (labelW + 2·pad, padTop + 15)` (`:466-470`), `elk.algorithm`, `elk.hierarchyHandling = SEPARATE_CHILDREN`, `elk.aspectRatio`, `elk.contentAlignment`, `elk.expandNodes`, `elk.padding = [top=labelH+15, 15, 15, 15]` (`:471-478`). For `elk.rectpacking`, also `RECTPACKING_OPTIONS` and `padding` 10 (`:481-487`).
4. Else, if `node.dir`: `elk.algorithm` = root algorithm, `elk.direction = dir2ElkDirection(dir)`, `elk.hierarchyHandling = SEPARATE_CHILDREN` (`:488-494`).
5. Else: none of the above. A plain subgraph has no `elk.algorithm`, no `elk.direction`, and no `hierarchyHandling` in its own options.
6. `SUBGRAPH_EXPERIMENT_OVERRIDES` (`:498`), empty.

`dir2ElkDirection` (`:318-332`): `LR`→`RIGHT`, `RL`→`LEFT`, `TB`/`TD`→`DOWN`, `BT`→`UP`, else `DOWN`.

### 2.3 Groups without children (empty subgraph)

`configureSubgraphNodes` skips them (`render.ts:1198-1201`, because `childrenById` has no entry, `:1000`). The ELK object is only the spread of the LayoutData node plus `children: []` and `labelData`. There is no `labels`, no `layoutOptions`, no `width`/`height`. See §4.6.

### 2.4 Leaves (`isGroup: false`)

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                             | Source                                                    |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `id`              | LayoutData id                                                                                                                                                                                                                                                                                                                                                                     | `render.ts:1042`                                          |
| `width`, `height` | measured `node.width ?? 0`, `node.height ?? 0`                                                                                                                                                                                                                                                                                                                                    | `render.ts:1048-1049`; measured at `createGraph.ts:67-69` |
| `labels`          | none (no label object; the label is inside the measured shape)                                                                                                                                                                                                                                                                                                                    | —                                                         |
| `labelBBox`       | present on the LayoutData node only if set by the shape code (unverified); not copied by ELK code                                                                                                                                                                                                                                                                                 | —                                                         |
| `layoutOptions`   | only `elk.portAlignment.default: CENTER`, and only if `spreadPorts` (`render.ts:1050-1061`). Flowchart never sets `spreadPorts`; only `usecaseDb.ts:520` does. Plus `elk.hierarchyHandling: INCLUDE_CHILDREN` if a cross-boundary edge touches the node (`render.ts:1273`), and `elk.layered.layering.layerConstraint: FIRST` for `keepEntryNodeOnTop` entries (`render.ts:672`). |                                                           |
| `ports`           | none; edges reference node ids                                                                                                                                                                                                                                                                                                                                                    | —                                                         |
| `padding`         | `flowchart.padding` (schema default 15; `flowDb.ts:1071`). Not used by ELK.                                                                                                                                                                                                                                                                                                       | —                                                         |
| `minWidth`        | `flowchart.minNodeWidth` (default 120; `flowDb.ts:1099`). Used in measure, §5.                                                                                                                                                                                                                                                                                                    | —                                                         |

Leaf `width`/`height` depend on `look` through shape padding (§4.12). Groups and edges have no look input to ELK.

Leaves with `shape: 'collapsedGroup'` are ordinary leaves (`isGroup: false`), see §4.7.

### 2.5 Edges

For each LayoutData edge (`addEdgesToElkGraph`, `render.ts:1089-1142`), one object is pushed onto the root `edges` array (`:1113`):

```
{
  ...edge,                       // LayoutData fields, incl. id, classes, arrow types
  sources: [edge.start],         // node id; no port
  targets: [edge.end],
  sourceId, targetId,            // only if both ends exist in nodeDb (:1175-1189)
  labels: [mainLabel, ...terminals]
}
```

- **Level.** Always the root `edges` (`graph` argument is the root, `render.ts:720`). elkjs then moves each edge into its containment container (probe, §2.7). The Rust port does the same (`json_import.rs:52-103`, `edge_container`: parent of two siblings, the node that contains the other, else the lowest common ancestor; self loops go to the node's parent).
- **id.** `edge.id`. The duplicate counter at `render.ts:1096-1100` is computed but never used.
- **mainLabel** (`render.ts:1119-1129`): `{width: edge.width ?? 0, height: edge.height ?? 0, orgWidth, orgHeight (same values), text: edge.label ?? '', layoutOptions: {'edgeLabels.inline': 'true', 'edgeLabels.placement': 'CENTER'}}`. Present even with no text (0×0, probe: ELK still writes `x:0, y:0`).
- **terminals** (`render.ts:1131-1136`, `1152-1172`): only when the root algorithm is `elk.layered`, and only for entries in `terminalLabelSizes`. The flowchart LayoutData never has these (class diagrams only: `edges.js:263-266`, `classDb.ts`). Unverified in-browser effect; code path not used by flowchart.
- **Reversal.** Edges flagged by `findSubgraphFeedbackEdges` have `sources`/`targets` swapped and `layoutReversed: true` set (`render.ts:736-747`, §4.2).
- **Output read back.** Only `sections[0]` is used for the route (`render.ts:1965`, `2046`); `sections` is an array of `{id, startPoint, endPoint, bendPoints, incomingShape, outgoingShape}` (probe). `labels[0].x/y` is the label's top-left; its centre is `x + width/2` (`render.ts:2082-2083`). Terminal `labels[i].x/y` are read only when `terminal` is set.

### 2.6 Skeleton (derived from code; not a captured dump)

Input `flowchart TB; subgraph S[Title]; A --> B; end; C --> A` gives LayoutData nodes `[S, A, B, C]` and edges `[L_A_B_0, L_C_A_1]` (§3, `flowDb.ts:1312`).

```
root.layoutOptions = { …PLAN §2.1 rows…, "elk.direction": "DOWN" }
root.children = [
  { id: "S", isGroup: true, shape: "rect", padding: 8,
    labels: [{ text: "Title", width: labelW, height: labelH }],
    layoutOptions: { "nodeSize.constraints": "[MINIMUM_SIZE, NODE_LABELS]",
                     "nodeSize.minimum": "(<labelW + 8>, 0)",
                     "spacing.baseValue": 24, …, "elk.padding": "[top=24,left=24,bottom=24,right=24]",
                     "elk.hierarchyHandling": "INCLUDE_CHILDREN" },   // added: C -> A crosses S
    children: [ { id: "A", width: wA, height: hA, padding: 15, minWidth: 120,
                  layoutOptions: { "elk.hierarchyHandling": "INCLUDE_CHILDREN" } },
                { id: "B", width: wB, height: hB, padding: 15, minWidth: 120 } ] },
  { id: "C", width: wC, height: hC, padding: 15, minWidth: 120,
    layoutOptions: { "elk.hierarchyHandling": "INCLUDE_CHILDREN" } }
]
root.edges = [
  { id: "L_A_B_0", sources: ["A"], targets: ["B"], labels: [{ width: 0, height: 0, text: "", … }] },
  { id: "L_C_A_1", sources: ["C"], targets: ["A"], labels: [{ width: 0, height: 0, text: "", … }] }
]
```

### 2.7 elkjs behaviour relied on (probes, elkjs 0.9.3)

| #   | Probe                                                                       | Result                                                                                                      | Consequence                                                                                                          |
| --- | --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| P1  | `layoutOptions` value `undefined`                                           | ignored                                                                                                     | JSON-normalised capture is equivalent                                                                                |
| P2  | value `null`                                                                | importer error, whole layout fails                                                                          | port must reject `null` (it does, `json.rs:195-207`)                                                                 |
| P3  | node/edge extra keys (`isGroup`, `labelData`, `foo`, functions)             | kept on output; the output object is the input object (`o === g`)                                           | Mermaid reads `isGroup`, `labels`, `labelData`, `layoutReversed` from output (`render.ts:1544-1553`, `1430`, `1993`) |
| P4  | edges in the root `edges` array, nodes in groups                            | edge gets a `container` field equal to its LCA (`'in'` inside `G` ends up in `G.edges`; `e1` stays at root) | section coordinates are relative to that container (`render.ts:2447-2467`)                                           |
| P5  | edge with an endpoint inside a `SEPARATE_CHILDREN` group, edge from outside | `UnsupportedGraphException: The source or the target of edge … could not be found`                          | why `setIncludeChildrenPolicy` exists                                                                                |
| P6  | edge to the `SEPARATE_CHILDREN` group node itself, or from it               | works                                                                                                       | §4.5                                                                                                                 |
| P7  | group `elk.direction` = RIGHT vs DOWN, `SEPARATE_CHILDREN`                  | members laid out left→right / top→down                                                                      | the directional branch is effective                                                                                  |
| P8  | same, `INCLUDE_CHILDREN`                                                    | identical output for RIGHT and DOWN                                                                         | group direction inert under INCLUDE                                                                                  |
| P9  | group with no children, no size, no options                                 | 0×0 at a point on the edge                                                                                  | §4.6                                                                                                                 |
| P10 | group with fixed `width`/`height` and no children                           | keeps them                                                                                                  | not produced by Mermaid                                                                                              |
| P11 | edge label with `text: ''`, 0×0                                             | `labels[0]` gets `x: 0, y: 0`                                                                               | `layoutEdge.x/y` is always defined for routed edges                                                                  |
| P12 | TAIL/HEAD terminal labels                                                   | placed beside their endpoints                                                                               | not used by flowchart                                                                                                |
| P13 | `$H`, `container` added to output                                           | not read by Mermaid (grep)                                                                                  | port may omit them                                                                                                   |

The Rust port's behaviour for missing `width`/`height` is 0 (`json_import.rs:214-225`), the same as P9.

---

## 3. Order-dependent data

| Item                            | Order used                                                                                                                  | Where                                               | Sorted?                                                                                                                  |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| LayoutData `nodes`              | groups in reverse `getSubGraphs()` order, then vertices in `getVertices()` order                                            | `flowDb.ts:1218-1229`, `1232-1279`, `1281-1288`     | no                                                                                                                       |
| ELK `children` of each parent   | LayoutData order filtered by `parentId`                                                                                     | `render.ts:1014`, `1017-1019`, `1045-1047`          | no                                                                                                                       |
| LayoutData `edges`              | `getEdges()` order; id `L_<start>_<end>_<index>` (index = position in that list, before collapse dropping)                  | `flowDb.ts:1290-1344`, `:1312`                      | no                                                                                                                       |
| ELK root `edges`                | same order as LayoutData                                                                                                    | `render.ts:1098-1139`                               | no                                                                                                                       |
| elkjs model order               | the JSON order above (`considerModelOrder` default `NODES_AND_EDGES`, `defaultConfig.ts`)                                   | elkjs                                               | n/a. Changing the order changes the layout (unverified; not probed). Rust keeps document order (`json_import.rs:52-66`). |
| `findSubgraphFeedbackEdges` DFS | roots = in-degree-0 nodes in first-appearance order, then all nodes in first-appearance order; successors in edge order     | `subgraphFeedbackEdges.ts:66-110` (`:85`, `:70-80`) | no                                                                                                                       |
| `findCyclicEntryNodes`          | nodes grouped by `parentId` in declaration order; back-edges removed in declaration order; first source-like node nominated | `render.ts:536-544`, `553-569`, `608-641`           | no. Default off (`keepEntryNodeOnTop: false`).                                                                           |
| `orderNodesForElkPaint`         | `Array.prototype.sort` (stable): groups first, then by depth ascending; ties keep input order                               | `render.ts:765-800`                                 | yes, in place; affects everything after layout                                                                           |
| Paint                           | nodes in sorted order, then edges in LayoutData order                                                                       | `common/index.ts:229`, `238`                        | no                                                                                                                       |
| `applyElkEdgeLayout`            | ELK output edge order                                                                                                       | `render.ts:1984`                                    | no                                                                                                                       |
| `straightenEdgeTerminals`       | greedy: each accepted straightening replaces `routes[index]`, which later edges see                                         | `render.ts:1889-1926`                               | order-dependent                                                                                                          |
| `separateOppositeEdgeLabels`    | grouped by sorted `(start, end)` pair                                                                                       | `render.ts:2135-2170`                               | by pair                                                                                                                  |
| `evenGroupFrames`               | children first (recursive), so a parent sees already-fitted children                                                        | `render.ts:1429-1434`                               | n/a                                                                                                                      |
| Measure DOM insertion           | `Promise.all` over nodes; insertion order depends on await order                                                            | `createGraph.ts:159-185`                            | not sorted; sizes are per element (unverified that order is irrelevant)                                                  |

---

## 4. Special handling

### 4.1 Cyclic entry nodes (`keepEntryNodeOnTop`, default false)

- `findCyclicEntryNodes` (`render.ts:529-646`): per container (by `parentId`), finds weakly connected components with no in-degree-0 node. Breaks cycles greedily in declaration order (`:608-633`). Nominates the first remaining source-like node per component (`:635-642`).
- `applyCyclicEntryConstraint` (`render.ts:654-676`) sets `elk.layered.layering.layerConstraint: FIRST` on the nominated node's `layoutOptions`. It returns at once unless `config.elk.keepEntryNodeOnTop` is true (`:658`).
- Runs after the feedback reversal (`render.ts:726`). Uses the original (not reversed) LayoutData edges (`:662-665`).

### 4.2 Subgraph feedback edges (`orientFeedbackEdges`, default true)

Gate: `render.ts:723`. Run: `reverseSubgraphFeedbackEdges` (`render.ts:736-747`). Algorithm: `findSubgraphFeedbackEdges` (`subgraphFeedbackEdges.ts:42-113`).

```
for each ELK edge e (source != target):           # self loops skipped (:51-53)
    a = LCA(e.source, e.target)                   # findCommonAncestor, root fallback
    if a is e.source or e.target: skip            # edge between subgraph and own member
    from = child of a on e.source's path          # childOnPath (:13-19)
    to   = child of a on e.target's path
    collapsed = (from != e.source) or (to != e.target)
    levels[a].push({index, from, to, collapsed})
for each level:
    DFS from in-degree-0 nodes, then all nodes, in first-appearance order
    an edge whose target is on the DFS stack is a back edge:
        flag = collapsed                          # only collapsed back edges are reversed (:103)
```

Consequences:

- "Collapsed" in `subgraphFeedbackEdges.ts` means an endpoint sits inside a subgraph that ELK sees as one node at that level. It is not the `view: collapsed` feature (§4.7).
- Only such an edge is reversed. A back edge between plain nodes is left to ELK's own cycle breaking (`subgraphFeedbackEdges.ts:30-36`).
- Reversal swaps `sources`/`targets` on the ELK edge and sets `layoutReversed` (`render.ts:741-745`). `applyElkEdgeLayout` swaps the ends back and reverses the points (`render.ts:1992-1997`, `2048-2050`).
- Runs after `configureCrossHierarchyEdges`, so the INCLUDE policy was computed on the original orientation (`render.ts:722-724`).
- Uses `parentLookupDb` built from LayoutData, not from ELK (`render.ts:718`).

### 4.3 Self loops

- ELK edge with `source == target` is passed through unchanged (`render.ts:1098-1139`). ELK routes it with bend points (probe).
- `findSubgraphFeedbackEdges` skips it (`subgraphFeedbackEdges.ts:51-53`).
- `configureCrossHierarchyEdges` skips it because both ends share a `parentId` (`render.ts:1239`).
- Offset for its section: `findCommonAncestor(id, id)` returns `parentById[id] || 'root'` (`find-common-ancestor.ts:10-13`), so a loop inside a group uses that group's origin (`render.ts:2447-2467`).
- The `drop self-loop after collapse` rule is in flowDb, not here (`flowDb.ts:1301-1306`).

### 4.4 Cross-hierarchy edges and the INCLUDE policy

`configureCrossHierarchyEdges` (`render.ts:1224-1245`): for each ELK edge whose endpoints both exist and have different `parentId`, compute the LCA and call `setIncludeChildrenPolicy` on both ends (`:1239-1242`).

`setIncludeChildrenPolicy` (`render.ts:1247-1277`):

```
node = nodeDb[id]; node.layoutOptions ??= {}
if node.hierarchyHandling == SEPARATE_CHILDREN and node has metadata.algorithm (container):
    clearContainerAlgorithmOptions(node)     # render.ts:267-277: removes overrides, restores baseValue 24 / nodeNode 50
    assign groupTitleSizeOptions(node)       # render.ts:1270
node.layoutOptions['elk.hierarchyHandling'] = INCLUDE_CHILDREN   # unconditional, :1273
if node.id != LCA and node.parentId: recurse on parent          # :1274-1276
```

- The endpoint itself and all ancestors below the LCA are forced to `INCLUDE_CHILDREN`. A directional group (`dir`, SEPARATE) loses SEPARATE and its direction becomes inert (P8). Its `elk.direction` is not removed, but it has no effect.
- A container with its own algorithm loses that algorithm (`clearContainerAlgorithmOptions`). The title floor is re-applied.
- Probe P5 shows why this matters: a `SEPARATE_CHILDREN` group with a boundary-crossing edge aborts the whole layout.

### 4.5 Edges to and from subgraphs

- Edge endpoint is a group id: ELK edges reference the compound node (P6). No special case in Mermaid except: when that group's `parentId` differs from the other end's, the policy above applies to the group (`render.ts:1239-1242`).
- Layout: the group's frame is its ELK box (`boundsFor`, `render.ts:2773-2782`, uses `getEffectiveGroupWidth`, which includes the title floor).
- Clipping: an endpoint on a group is clipped to its frame, not to a centre ray (`clipGroupEndpoint` `render.ts:2828`, `attachAlongDepartureAxis` returns null for groups `:2927-2929`).
- `internalEdgePoints` (`render.ts:1392-1421`) adds the group-end point of boundary edges to the frame-fit computation, so the frame does not pull past an anchor.

### 4.6 Empty subgraphs

- LayoutData: any subgraph that is not hidden becomes a `isGroup: true` node (`flowDb.ts:1257-1268`), whether or not it has members. `subGraphDB` only affects vertices.
- ELK: childless compound node with `labelData` but no `labels`, `layoutOptions`, `width`, `height` (§2.3). P9: elkjs returns 0×0.
- Build side: `applyElkNodePositions` sets the LayoutData width to `max(0, labelData.width)` (`render.ts:1544-1546`) and height 0 (`:1547`). `evenGroupFrames` skips it (`boxes.length === 0`, `:1440-1441`).
- Paint: `rect` draws `height = node.height` (`clusters.js:70-72`), so the frame has height 0 and only the title is visible. **Unverified in browser.**

### 4.7 Collapsed groups (`view: collapsed`)

- flowDb turns a collapsed subgraph into a leaf with `shape: 'collapsedGroup'`, `isGroup: false`, `padding: 8` (`flowDb.ts:1237-1255`). Its members are hidden (`:1197-1215`) and skipped when vertices are added (`:1284`).
- Edges touching hidden members are redirected to the collapsed node; an edge that becomes a self loop purely from collapsing is dropped (`flowDb.ts:1299-1306`).
- Because the collapsed node is a leaf, the ELK graph has no group for it. Its size is the measured `collapsedGroup` shape bbox (leaf rules, §2.4).
- A collapsed subgraph is a leaf in the ELK graph. An edge ending on it has `from === source` at its level (`subgraphFeedbackEdges.ts:59-62`), so it is not `collapsed` on that account. Redirected edges can still be reversed when their other end sits inside an expanded group at that level (§4.2).

### 4.8 Containers with their own algorithm (`@{ algorithm: … }`)

- Accepted names: `CONTAINER_ALGORITHMS` (`render.ts:287-296`): `elk.layered`, `elk.box`, `elk.rectpacking`, `elk.stress`, `elk.force`, `elk.mrtree`, `elk.radial`, `elk.sporeOverlap`. Unknown names log a warning and fall back to the diagram layout (`render.ts:302-316`).
- Applied only when the group is not an ancestor of a boundary-crossing edge (§4.4). Otherwise cleared (`render.ts:1264-1271`).
- For a surviving container: `SEPARATE_CHILDREN` and its own algorithm. Mermaid's JS passes the name to elkjs unchanged.

**Rust port.** `crates/ferrigura-elk/src/core/recursive.rs`:

- `layout_recursively` calls `require_layered` on every level it lays out (`recursive.rs:52-56`). A child with an `elk.algorithm` other than layered is routed to a separate layout (`:75-86`), where line 56 refuses it. The refusal is `LayoutError::Unsupported("algorithm elk.box")` (`:194-203`).
- A level without `elk.algorithm` is also refused: `"a level without elk.algorithm (LayoutAlgorithmResolver is not ported yet)"` (`:204-206`).
- Consequence: any diagram with a surviving container algorithm, or a root algorithm other than `elk.layered`, fails in the Rust port. The root always has `elk.algorithm` (`render.ts:915`), so for the root the failure is the non-layered name itself.
- Groups without a directive are never laid out on their own (INCLUDE), so they need no algorithm (`recursive.rs:95-101`).

### 4.9 Direction per subgraph

- Source: `subgraph X direction LR` → `subGraph.dir` → `flowDb.ts:1249`, `1267`.
- Effective only when the group is SEPARATE (no algorithm, no boundary-crossing edge): `elk.direction = dir2ElkDirection(dir)` (`render.ts:492`), P7.
- Under INCLUDE it is inert (P8). It is lost for any group touched by a boundary-crossing edge (§4.4).
- Leaves also carry `dir` (`flowDb.ts:1075`). It is used only for shape decisions (`createGraph.ts:102-120`, `resolveNodeDir`), never by ELK.
- The root direction is `data4Layout.direction` (`render.ts:715-716`), default `DOWN`.

### 4.10 Title reservation and frame fitting

Chain:

1. Measure: `labelBBox` of the title with unwrapped width for non-markdown titles (`createGraph.ts:170-171`).
2. `labelData` = `{width: labelBBox.width, height: labelBBox.height - 2}` (`render.ts:1074-1079`).
3. ELK input: `labels[0]` with those sizes and `nodeLabels.placement = [H_CENTER V_TOP, INSIDE]` (`render.ts:1203-1209`, `429`). This reserves the title strip height inside the top padding.
4. ELK input: `nodeSize.minimum = (labelData.width + 8, 0)` (`render.ts:347-353`, `364-365`). This reserves the title width.
5. Output: LayoutData width = `max(ELK width, labelData.width)` (`render.ts:1544-1546`).
6. `evenGroupFrames`: pulls the frame in to 24px from the children on the left, right and bottom (`SUBGRAPH_PADDING`, `render.ts:1460-1462`); top is left as ELK set it (`:1463`); never grown (`:1456-1458`); never narrower than the title floor (`:1467-1479`).
7. Paint: rect label at `(node.x - bbox.width/2, node.y - node.height/2 + subGraphTitleTopMargin)` (`clusters.js:108-113`). `subGraphTitleMargin` (default 0/0) is paint-only; it is not in the ELK input.

### 4.11 Edge routing after ELK (layout core)

Per ELK edge (`render.ts:1984-2100`), in ELK output order:

1. Restore reversal: swap ends back, and reverse points (`:1992-1997`, `:2048-2050`).
2. Offset = `calcOffset(source, target)` (`:2043`, def `:2447-2467`): LCA container's origin, or `elkOrigin` if the frame was refit (`:2462`). Root LCA → 0.
3. Points = `[section.startPoint, ...bendPoints, section.endPoint] + offset` (`:2431-2445`).
4. Non-`rect33` shapes: prepend and append the node centres (`:2058-2064`).
5. `sanitizeElkEdgePoints` (`:2469-2551`): clip against group frames (`clipGroupEndpoint`), then `cutter2` (`:2938-3021`) clips leaves to outlines (`attachAlongDepartureAxis` → `outlineAttachPoint`, else `computeNodeIntersection`). `cutter2` also trims its last point when it is under 2px from the one before (`:3002-3015`). Points within 1e-6 of each other are deduped (`:2557-2573`).
6. Marker fix: `ensureEndMarkerSegmentLength` and `ensureStartMarkerSegmentLength` (`:2588-2648`).
7. Curve: `'rounded'` (`:2078`).
8. Label centre: `x = label.x + offset.x + label.width/2` (`:2082-2083`).
9. Edge without sections: straight line between centres, clipped, curve `'linear'`, label at its midpoint (`:2014-2039`).
10. Post passes: straighten terminal jogs (`:2102-2118`, order-dependent, §3), follow moved endpoints (`:2119-2121`), slide off frames (`:2123`), put terminal labels on their side (`:2124`), separate opposite labels (`:2125`).

Only `sections[0]` is used (`:1965`, `:2046`). `internalEdgePoints` (`:1409`) iterates all sections. Every probe gave one section per edge; this was not checked exhaustively.

---

### 4.12 Look: `neo` and `classic` (handDrawn out of scope)

- Default: flowchart `look` is `neo` (`schemas/config.schema.yaml:2458-2460`). `resolveAppearance` (`config.ts:79-107`) puts the resolved value on the top level, in this order: frontmatter or directive, `initialize()`, diagram default, global default. `flowDb` reads `config.look` (`flowDb.ts:1251`, `1269`, `1287`).
- **What reaches ELK: leaf `width` and `height` only.** Shape handlers pick `labelPaddingX/Y` by `node.look === 'neo'`. Example, `squareRect` (`shapes/squareRect.ts:9-10`), which is the shape of a default flowchart vertex (`flowDb.ts:1012-1014`). `drawRect` then sizes the outline as `max(bbox.width + 2·labelPaddingX, node.width || 0)` and the height likewise (`shapes/drawRect.ts:21-22`). The leaf `getBBox()` (§5) returns that outline.

  | `look`    | `labelPaddingX` | `labelPaddingY` | outline grows by (width, height) |
  | --------- | --------------- | --------------- | -------------------------------- |
  | `neo`     | 16              | 12              | +32, +24                         |
  | `classic` | 2·15 = 30       | 15              | +60, +30                         |

  Padding 15 is `flowchart.padding` (schema default). Derived from code, not measured in a DOM.

- About 40 shape files contain a `look === 'neo'` branch (`grep -l "look === 'neo'" shapes/*.ts`). Not all of them change size: `state.ts:7-8` changes only the corner radius. `roundedRect.ts:16-17` uses `node.padding` on both axes and has no branch, so it is the same under both looks.
- **Groups: no look input to ELK.** `buildSubgraphLayoutOptions`, `groupTitleSizeOptions`, `labelData` and the group measurement take no look. The only look branch in the cluster painter (`clusters.js:478-482`) sets the class `divider` in both arms. Otherwise `look` affects paint only: `data-look` attributes and handDrawn branches.
- **Edges: no look input to ELK.** `insertEdgeLabel` (`edges.js:74-137`) has no look branch. Look changes paint only: dash arrays (`edges.js:883-891`) and marker margins (`edges.js:946-947`).
- The ELK code never reads `look`; the only matches in `elk/*.ts` are comments. The field is still copied onto ELK nodes and edges by the spreads (`render.ts:1042`, `1113`) and ignored by elkjs (P3).
- Consequence for M5: leaf sizes are a function of `(shape, look, padding, label)`. A golden comparison must use the same look on both sides. A neo/classic difference shows up as leaf `width`/`height` (and then as layout differences), not as a different ELK option set.

## 5. Measurements needed before layout

| Quantity          | Measured in                                                   | Element / method                                                                                                                                                                                      | Feeds                                                | Notes                                                                                                                                    |
| ----------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | --- | ------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ---------------------------------------------------- |
| Leaf size         | `insertMeasuredNode` `createGraph.ts:61-71`                   | `childNodeEl.node().getBBox()` (SVG geometry of the shape group, or the `svg:a` wrapper for linked nodes, `nodes.ts:45-53`)                                                                           | `width`, `height` (`render.ts:1048-1049`)            | Size depends on the shape handler and on `look` (§4.12); not traced for every shape. Headless (no DOM): 0 (`createGraph.ts:156`, `175`). |
| Leaf label        | `labelHelper` `shapes/util.ts:50-154` (inside shape handlers) | HTML: `div.getBoundingClientRect()` (`util.ts:119-131`); SVG: `text.getBBox()` (`util.ts:134-141`). Width: `node.width                                                                                |                                                      | node.wrappingWidth                                                                                                                       |     | flowchart.wrappingWidth` (`util.ts:98`) and `withMinWidth`with`flowchart.minNodeWidth` when no explicit width (`util.ts:92`, `131`, `140`). | leaf shape, then leaf bbox | Default wrap 120px and min width 120px (schema, §6). |
| Group title       | `measureGroupLabel` `createGraph.ts:42-59`                    | `labelHelper` with width `Infinity` for non-markdown (`createGraph.ts:170-171`, `:52`); markdown: wrapped at `flowchart.wrappingWidth` (default 120).                                                 | `labelBBox` → `labelData` (`render.ts:1074-1079`)    | §7, item 4.                                                                                                                              |
| Group title width | `groupTitleWidth` `render.ts:347-353`                         | `labelData.width + node.padding` (padding 8)                                                                                                                                                          | `nodeSize.minimum`, frame floor                      | Only for `clusterPaintsTitle` shapes (`clusters.js:823-830`).                                                                            |
| Edge label        | `insertEdgeLabel` `edges.js:74-137`                           | HTML: `div.getBoundingClientRect()` (`:111-117`); SVG: `labelElement.getBBox()` (`:121-128`). Wrap width: `undefined` passed (`:98`), so `createText`'s default of 200 applies (`createText.ts:319`). | `edge.width`, `edge.height` (`:136-137`) → ELK label | Only for edges with `hasEdgeLabel` (`edges.js:55-62`).                                                                                   |
| Terminal label    | `insertEdgeLabel` `edges.js:139-266`                          | `createLabel` then `getBBox()` / `div.getBoundingClientRect()` (`:152-160`)                                                                                                                           | `terminalLabelSizes` (`:264-265`)                    | Class diagrams only.                                                                                                                     |

Paint-time measurements that differ from the ELK ones, but only affect translate:

- Group label at paint: `text.getBBox()` or `div.getBoundingClientRect()` on a fresh `rect` label (`clusters.js:53-61`), positioned at `bbox.width/2` (`:112`).
- `rect` painter writes `node.width/height` from the painted rect's `getBBox()` (`clusters.js:123-126`), so it reads back its own geometry.

---

## 6. Config inputs

Defaults: `defaultConfig.ts:25-41` (`elk` block), `schemas/config.schema.yaml` for `flowchart` (`FlowchartDiagramConfig` starts at line 2438).

| Config key                                                                                         | Default                               | Read by ELK path                                                                | Effect on ELK input                                                             | Notes                                                                                                                                           |
| -------------------------------------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `elk.preset`                                                                                       | `default`                             | `render.ts:391`, `910`                                                          | selects the preset table (PLAN §2.2)                                            |                                                                                                                                                 |
| `elk.nodePlacementStrategy`, `nodePlacementAlignment`, `cycleBreakingStrategy`, `layeringStrategy` | undefined                             | `render.ts:916-930`, `432-449`                                                  | override preset                                                                 |                                                                                                                                                 |
| `elk.layeringLayerBound`                                                                           | 4                                     | `render.ts:932`                                                                 | COFFMAN_GRAHAM only                                                             |                                                                                                                                                 |
| `elk.mergeEdges`                                                                                   | false                                 | `render.ts:431`, `920`                                                          | `elk.layered.mergeEdges`                                                        |                                                                                                                                                 |
| `elk.forceNodeModelOrder`, `considerModelOrder`                                                    | false, `NODES_AND_EDGES`              | `render.ts:924-926`                                                             | model-order options                                                             |                                                                                                                                                 |
| `elk.keepEntryNodeOnTop`                                                                           | false                                 | `render.ts:658`                                                                 | `layerConstraint: FIRST` on entries                                             | §4.1                                                                                                                                            |
| `elk.orientFeedbackEdges`                                                                          | true                                  | `render.ts:723`                                                                 | runs feedback reversal                                                          | §4.2                                                                                                                                            |
| `elk.straightenEdges`                                                                              | true                                  | `render.ts:1939`, `2102`                                                        | terminal straightening (layout core)                                            | not ELK; Mermaid post-pass                                                                                                                      |
| `elk.lineHops`                                                                                     | true                                  | `lineHops.ts:43-46`                                                             | afterPaint hops                                                                 | not ELK                                                                                                                                         |
| `flowchart.padding`                                                                                | 15 (`schema:2529`)                    | `flowDb.ts:1071`                                                                | leaf `padding`; shapes read it                                                  | leaf width itself is from the shape bbox                                                                                                        |
| `flowchart.wrappingWidth`                                                                          | 120 (`schema:2537`)                   | `shapes/util.ts:98`                                                             | leaf label wrap; group title wrap for markdown; `labelData.wrappingWidth`       | not read in ELK directly                                                                                                                        |
| `flowchart.minNodeWidth`                                                                           | 120 (`schema:2547`)                   | `flowDb.ts:1099` → `util.ts:92,131,140`                                         | leaf label min width → leaf bbox                                                |                                                                                                                                                 |
| `flowchart.nodeSpacing`                                                                            | 50 (`schema:2490`)                    | **not read** by any ELK code (grep)                                             | none                                                                            | Set on LayoutData (`flowRenderer-v3-unified.ts:37`) and read only by the DDLT tooling. Documented as spacing; has no effect with `layout: elk`. |
| `flowchart.rankSpacing`                                                                            | 50 (`schema:2499`)                    | **not read**                                                                    | none                                                                            | Same as above (`flowRenderer-v3-unified.ts:38`).                                                                                                |
| `flowchart.curve`                                                                                  | `basis` (`schema:2521`)               | `render.ts:2745-2750`                                                           | edge `curve` at prepare                                                         | overwritten by `'rounded'`/`'linear'` in layout core (`:2027`, `:2078`). No effect on routed edges.                                             |
| `flowchart.subGraphTitleMargin`                                                                    | `{top: 0, bottom: 0}` (`schema:2475`) | **not in ELK**                                                                  | paint translate only (`clusters.js:108-113`, `:391-395`; `common/index.ts:337`) |                                                                                                                                                 |
| `flowchart.titleTopMargin`                                                                         | 25 (`schema:2463`)                    | not in ELK                                                                      | diagram title (`flowRenderer-v3-unified.ts:47-49`)                              |                                                                                                                                                 |
| `flowchart.diagramPadding`                                                                         | 8 (`schema:2482`)                     | not in ELK                                                                      | viewport (`flowRenderer-v3-unified.ts:44`)                                      |                                                                                                                                                 |
| `flowchart.look` (resolved to top-level `look`)                                                    | `neo` (`schema:2458-2460`)            | `flowDb.ts:1251`, `1269`, `1287`; leaf shapes, e.g. `shapes/squareRect.ts:9-10` | leaf `width`/`height` only, through shape padding (§4.12). No ELK keys.         | Resolution: `config.ts:79-107`. Groups and edges: no ELK effect.                                                                                |
| `layout`                                                                                           | `elk`                                 | `rendering-util/render.ts:57-63`, `flowRenderer-v3-unified.ts:35`               | selects the renderer                                                            | `getRegisteredLayoutAlgorithm` falls back to `elk` for unknown names.                                                                           |

The root's `spacing.baseValue` (40), group's (24), node-node (50), edge-edge (20), edge-node between layers (30) and `portsSurrounding` (12) are fixed literals, not config (PLAN §2.1).

---

## 7. Discrepancies and dead inputs

1. **Non-layered root names.** PLAN §2.1 (lines 26, 51) and ADR 0006 say `elk.box`, `elk.rectpacking`, etc. "fall back to layered". The JS does not do that: `render.ts:57-63` and `render.ts:915` pass the name through, so elkjs lays out with it. The fallback can only be a Rust-port behaviour. The Rust port refuses these instead (`recursive.rs:194-203`), so M5 has to choose: fall back in JS too (changes output), or accept that the Rust port differs.
2. **`flowchart.nodeSpacing` / `rankSpacing`** are documented as spacing for `layout: elk` but never read (§6).
3. **`flowchart.curve`** has no effect on routed edges (`render.ts:2078`).
4. **Markdown group titles** are measured wrapped at `flowchart.wrappingWidth` (`createGraph.ts:170`, `unwrap` is false for markdown, and `node.width` is undefined at measure time). They are painted wrapped at the group's final width (`clusters.js:41-47`, `width: node.width`). Reserved height and painted height can differ. **Unverified in browser.**
5. **Plain group titles** are measured with `labelHelper` (`createGraph.ts:50`) but painted with `createLabel` (`clusters.js:49`). The comment at `createGraph.ts:162-168` assumes they match. Not checked.
6. **Stale comment** at `render.ts:753-754` says "200px flowchart wrapping width"; the schema default is 120 (`schema:2537`). 200 is only the `createText` default (`createText.ts:319`) and the c4 fallback (`c4LabelHelper.ts:48`).
7. **Misplaced comment.** The "Scratch overrides … MUST be empty on develop" block (`render.ts:827-839`) describes `ROOT_EXPERIMENT_OVERRIDES`, but sits above `ELK_PRESETS`.
8. **Dead code.** `linkIdCnt` (`render.ts:1096-1100`) does nothing; duplicate edge ids are not disambiguated (flowDb ids are index-based, but user-given ids are used as-is, `flowDb.ts:1312`). `createGraph.ts:193-196` never pushes (`edgeExists` is always true).
9. **`minlen` is never read by ELK**: `edge.minlen` is computed at `render.ts:2726` and used nowhere in the ELK path. Link length in the source (`A ---- B`) has no effect on layout.
10. **`PLAN §2.1` line 25** says INCLUDE_CHILDREN goes on "ancestors of cross-boundary edges". It also goes on the endpoints themselves (`render.ts:1273`, the recursion starts at the endpoint).
11. **Look fallback.** Vertices use `config.look || 'classic'` (`flowDb.ts:1287`), but groups use `config.look` unchanged (`:1251`, `:1269`). Same result under schema defaults (`neo`); they differ only if `look` is unset.

---

## 8. Open questions and unverified items

- Whether the Rust port needs container algorithms and non-layered roots at all for M5 (item 7, §4.8). Decision needed, not a research question.
- Whether `elk.direction` on a SEPARATE group should be kept when a boundary edge touches it (it is currently dropped; §4.4, §4.9). Behaviour change, not a bug fix.
- Empty subgraph visual result (§4.6): painted frame height 0 by code reading; not rendered.
- Markdown and plain group title mismatches (§7, items 4-5): not rendered.
- Whether elkjs output depends on children order beyond the layout itself (model order): not probed.
- Shape handlers' use of `node.padding` and `node.width` before `labelHelper` (leaf label wrap at 120 vs shape width): traced only for `squareRect` and `drawRect` (§4.12); the other shapes are not traced.
- Neo vs classic leaf sizes (§4.12): computed from code for `squareRect`; not measured in a DOM.
- Whether measured sizes are independent of the `Promise.all` insertion order (§3): assumed, not tested.
- Multi-section edges: elkjs layered gives one section here; the code uses `sections[0]` only.
- `layeringLayerBound` and other keys without `??` that could receive `null` from a config: presumably rejected by schema validation (not checked).
- No browser run was done for anything in this note; every paint-side statement is from code reading.

---

## Appendix: probes (reproducible, nothing written)

Each probe is `node -e` against `/Users/sebastian/Workspace/ferraid/packages/mermaid/node_modules/elkjs/lib/elk.bundled.js` with `root.layoutOptions` set to `elk.layered`, `INCLUDE_CHILDREN`, `spacing.baseValue: 40`. Results are in §2.7 (P1–P13) and §4.4, §4.6, §4.9.
