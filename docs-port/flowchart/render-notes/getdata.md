# getData() notes for M5

Source: `packages/mermaid/src/diagrams/flowchart/flowDb.ts`, commit e8af08065. The worktree branch `sebastian/port-m5-flowchart-render` points at the same commit as `main`. Paths are relative to the repo root. Line numbers are from that commit.

Method: read-only. No tests were run and no code was executed, so every rule below comes from reading. "Unverified" marks what I could not confirm by reading.

Terms: DB = `FlowDB` (flowDb.ts). Parse time = the grammar's calls into the DB (`addVertex`, `addLink`/`addSingleLink`, `addSubGraph`, `setLink`, `setTooltip`, ...). getData time = inside `getData()` (flowDb.ts:1119-1347).

## 0. Summary

- `getData()` is flowDb.ts:1119-1347. It returns `{ nodes, edges, other: {}, config }` (1346). `config` is `getConfig()` taken at call time (1120).
- getData maps DB state built at parse time. It does not parse, and it does not sanitise. Labels, titles, tooltips, edge text and links are already sanitised when getData reads them (section 2.5).
- Output order: subgraph nodes first (reverse of the DB's subgraph array), then vertices in creation order, then edges in creation order (section 1).
- Config read at getData time: `look`, `flowchart.padding`, `flowchart.minNodeWidth`, `flowchart.curve` (section 5). In real renders `look` resolves to `neo`, not `classic` (section 5.1).

## 1. Output order

| List                   | Order                                                                                                                 | Source                                                       |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `nodes`, subgraph part | Reverse of the `subGraphs` array (reverse closing order). Hidden subgraphs skipped                                    | flowDb.ts:1232-1279 (loop `i = length-1 .. 0`, skip at 1234) |
| `nodes`, vertex part   | `this.vertices` Map insertion order, i.e. the first `addVertex` call that creates the vertex. Hidden vertices skipped | 1281-1288; creation at 181-196                               |
| `edges`                | `this.edges` array order (`addLink` loops starts outer, ends inner), minus edges dropped inside a collapsed subgraph  | 1290-1344; 353-371                                           |
| `subGraphs` array      | Closing order: the grammar reduces a subgraph when it closes. A repeated id stays at its first position               | addSubGraph 752-766; comment 1146-1147                       |

Consequences:

- All subgraph nodes come before every vertex node, even when a subgraph's vertices appear earlier in the source.
- The ELK adapter receives `nodes` and `edges` in these orders (`elk/render.ts:718-720`). Whether ELK's result depends on order is unverified here; M4 owns that.
- `subgraph.nodes` order is not used for output order. The grammar reverses it (`subgraph.spec.js:22-23`: `a2` before `a1`).

## 2. Node fields

### 2.1 Vertex nodes (`addNodeFromVertex`, flowDb.ts:1048-1103, called at 1287)

| Field                                                           | Value                                                                                                                                                                                                                                                                       | Lines           |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `id`                                                            | `vertex.id`                                                                                                                                                                                                                                                                 | 1066            |
| `label`                                                         | `vertex.text`. Sanitised for bracket and text labels, raw for `@{ label }` (section 2.5)                                                                                                                                                                                    | 1067            |
| `labelType`                                                     | `vertex.labelType`. Default `'text'` at creation (190). Replaced by the text object's type when text is given (202), or by `sanitizeNodeLabelType(doc.labelType)` for `@{ label }` (247). `sanitizeNodeLabelType` keeps `text`/`string`/`markdown`, else `markdown` (90-98) | 1068            |
| `labelStyle`                                                    | `''`                                                                                                                                                                                                                                                                        | 1069            |
| `parentId`                                                      | `parentDB.get(vertex.id)` (section 4.2). Key absent when undefined                                                                                                                                                                                                          | 1056, 1070      |
| `padding`                                                       | `config.flowchart?.padding \|\| 8`. Default 15 in real renders; `0` also becomes 8                                                                                                                                                                                          | 1071            |
| `cssStyles`                                                     | `vertex.styles` (statement order)                                                                                                                                                                                                                                           | 1072            |
| `cssCompiledStyles`                                             | `getCompiledStyles(['default', 'node', ...vertex.classes])`                                                                                                                                                                                                                 | 1073, 1105-1117 |
| `cssClasses`                                                    | `'default ' + vertex.classes.join(' ')`. Trailing space when there are no classes                                                                                                                                                                                           | 1074            |
| `dir`                                                           | `vertex.dir`. Always undefined: no grammar path passes `dir` to `addVertex` (flow.jison:403-420, 432-464 and 558 pass no sixth argument)                                                                                                                                    | 1075            |
| `domId`                                                         | `vertex.domId` = `flowchart-<id>-<n>` (section 2.6)                                                                                                                                                                                                                         | 1076, 191       |
| `look`                                                          | `config.look \|\| 'classic'`. Only vertex nodes have this fallback                                                                                                                                                                                                          | 1077, 1287      |
| `link`, `linkTarget`                                            | `vertex.link` (formatted by `formatUrl`, section 2.5), `vertex.linkTarget`                                                                                                                                                                                                  | 1078-1079       |
| `tooltip`                                                       | `getTooltip(vertex.id)` = `tooltips.get(id)` (gen-2 keys by id)                                                                                                                                                                                                             | 1080, 564-566   |
| `icon`, `pos`, `img`, `assetWidth`, `assetHeight`, `constraint` | Copied from the vertex (set in addVertex 249-275)                                                                                                                                                                                                                           | 1081-1086       |
| `isGroup`                                                       | `false`                                                                                                                                                                                                                                                                     | 1097            |
| `shape`                                                         | `getTypeFromVertex(vertex)` (section 2.2)                                                                                                                                                                                                                                   | 1098            |
| `minWidth`                                                      | `config.flowchart?.minNodeWidth` (default 120). Vertex nodes only                                                                                                                                                                                                           | 1099            |

getData does not set `colorIndex`, `metadata`, `width`, `height`, `x`, `y` or `minWidth` on subgraph nodes.

### 2.2 Shape mapping (`getTypeFromVertex`, flowDb.ts:995-1023)

Checks run in this order: image, icon, then `vertex.type`.

| Condition                               | Shape                                                                  | Lines     |
| --------------------------------------- | ---------------------------------------------------------------------- | --------- |
| `vertex.img` set                        | `imageSquare`                                                          | 996-998   |
| `vertex.icon` set, `form === 'circle'`  | `iconCircle`                                                           | 999-1001  |
| icon, `form === 'square'`               | `iconSquare`                                                           | 1002-1005 |
| icon, `form === 'rounded'`              | `iconRounded`                                                          | 1006-1008 |
| icon, any other form                    | `icon`                                                                 | 1009      |
| `vertex.type === 'square'` or undefined | `squareRect`                                                           | 1012-1014 |
| `'round'`                               | `roundedRect`                                                          | 1015-1016 |
| `'ellipse'`                             | `ellipse` (with `@ts-expect-error`: "Ellipses are broken", issue 5976) | 1017-1019 |
| any other                               | `vertex.type` unchanged                                                | 1020-1021 |

Where `vertex.type` comes from:

- Bracket syntax (flow.jison:432-464): `[t]` square; `((t))` circle; `(((t)))` doublecircle; `(-t-)` ellipse; `([t])` stadium; `[[t]]` subroutine; `[(t)]` cylinder; `(t)` round; `{t}` diamond; `{{t}}` hexagon; `>t]` odd; `[/t/]` trapezoid; `[\t\]` inv_trapezoid; `[/t\]` lean_right; `[\t/]` lean_left. A bare id has no type.
- Shape data `@{ shape: X }` sets `vertex.type = X` (flowDb.ts:236-243). `X` must be lowercase with no `_` (237-241), and `isValidShape` must accept it (239).

Registry check (`rendering-util/rendering-elements/shapes.ts`, `isValidShape` at 605-608). Every produced name is a registry key (short name, alias or internal alias) except `ellipse`, which is not a key. Locations: `squareRect` internal alias 113-114; `roundedRect` internal alias 122-123; `rect` short name 111; `circle` short name 197; `diamond` alias 223; `cylinder` alias 147; `odd` short name 521; `stadium` short name 129; undocumented shapes 538-586, including `imageSquare`, `icon`, `iconSquare`, `iconCircle`, `iconRounded`, `collapsedGroup` (551). The Rust copy `crates/ferrigura-syntax/src/flowchart/shapes.rs` has the same keys and no `ellipse`.

Unverified: how the renderer draws the unregistered `ellipse` shape.

### 2.3 Subgraph and vertex with the same id (flowDb.ts:1059-1063)

If a vertex has the same id as a subgraph node already in `nodes`, the existing node keeps all its fields. Only these are overwritten: `cssStyles = vertex.styles`, `cssCompiledStyles = getCompiledStyles(vertex.classes)` (no `default`/`node`), `cssClasses = vertex.classes.join(' ')` (no `default ` prefix).

The `isGroup` branch at 1088-1093 looks unreachable by reading. A subgraph id is in `subGraphDB` only when it is not hidden (1220-1225). Every non-hidden subgraph is pushed to `nodes` before the vertices (1232-1279). So the vertex path always finds the node first.

### 2.4 Subgraph nodes (flowDb.ts:1232-1279)

| Field                                  | Expanded (`shape 'rect'`, `isGroup true`)                             | Collapsed (`shape 'collapsedGroup'`, `isGroup false`) |
| -------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------- |
| `id`, `label`                          | `id`, `label = subGraph.title` (1258-1259)                            | same (1240-1241)                                      |
| `labelStyle`, `labelType`, `parentId`  | `''`, `subGraph.labelType`, `parentDB.get(id)`                        | same (1242-1244)                                      |
| `padding`                              | hard-coded `8`, not config (1263)                                     | hard-coded `8` (1245)                                 |
| `cssCompiledStyles`                    | `getCompiledStyles(subGraph.classes)`                                 | same (1246)                                           |
| `cssClasses`                           | `subGraph.classes.join(' ')`, no `default`                            | same (1247)                                           |
| `dir`                                  | `subGraph.dir`: raw statement value, not normalised (`TD` stays `TD`) | same (1249)                                           |
| `look`                                 | `config.look`, no fallback (1269)                                     | same (1251)                                           |
| `colorIndex`                           | `declarationIndex.get(id)` (1270)                                     | same (1254)                                           |
| `metadata`                             | All subgraph metadata keys (1276)                                     | Not set                                               |
| `domId`, `link`, `tooltip`, `minWidth` | Not set                                                               | Not set                                               |

Subgraph nodes have no `domId`, so the render prefix uses the id (render.ts:90).

### 2.5 Sanitisation: what getData already receives

| Value                                  | Sanitised at parse time?                                                                           | Lines                               | Rust model holds                              |
| -------------------------------------- | -------------------------------------------------------------------------------------------------- | ----------------------------------- | --------------------------------------------- |
| Bracket or text vertex label `A[text]` | Yes: `sanitizeText(textObj.text.trim())`, then quote strip                                         | flowDb.ts:199-207 (sanitize at 201) | Sanitised `text` (`sanitize_text` in `db.rs`) |
| `@{ label: ... }` vertex label         | **No**: assigned as is, labelType from `labelType`                                                 | 245-248                             | Raw `text`                                    |
| Empty label with icon or img           | Text set to `''` when it equals the id and the label is blank                                      | 249-254, 261-266                    | Already applied (db.rs `apply_vertex_data`)   |
| Edge label                             | Yes: `sanitizeText`, then quote strip                                                              | 298-307 (sanitize at 301)           | Sanitised `text` (`db.rs`)                    |
| Subgraph title                         | Yes: `sanitizeText`, then trim                                                                     | 735-736, 742                        | Sanitised `title` (`db.rs`)                   |
| Tooltip                                | Yes: `sanitizeText`                                                                                | 488-496 (492)                       | Raw `tooltips`                                |
| Link                                   | `utils.formatUrl`: trim; empty becomes undefined; unless `securityLevel` is `loose`, `sanitizeUrl` | 553-558; utils.ts:250-262           | Trimmed; empty is `None`; no `sanitizeUrl`    |

`sanitizeText` (common.ts:71-83) runs `DOMPurify.sanitize(sanitizeMore(text))`, with `dompurifyConfig` if set, else `{ FORBID_TAGS: ['style'] }`. `sanitizeMore` (common.ts:56-69) runs only when `getEffectiveHtmlLabels(config)` is true (config.ts:324-326, `htmlLabels` default true). For `antiscript`, `strict` or `sandbox` it calls `removeScript` (DOMPurify). For other non-`loose` levels it escapes `<`, `>` and `=`, keeping `<br>` through a placeholder.

Consequence: the DB applies the sanitising step to the first, third and fourth rows, with `sanitize.rs` (a port of the DOMPurify subset that labels reach; see its module documentation). The node label is sanitised again when it is drawn (`shapes/mod.rs`, `node_label`), as `labelHelper` does. Tooltips and links are not ported.

### 2.6 domId

- Set only at creation: `'flowchart-' + id + '-' + this.vertexCounter` (flowDb.ts:191; prefix 35). `vertexCounter` is then incremented (197) for every `addVertex` call that reaches that line, including calls for a vertex that already exists. So the suffix is the number of earlier such calls on that DB.
- `vertexCounter` is not reset by `clear()` (650-664). Each diagram gets a new DB (flowDiagram.ts:19-21 `get db() { return new FlowDB(); }`), so this matters only for reused DB instances, such as the parser specs' `new FlowDB(); clear()`.
- getData copies it (1076) and does not prefix it.
- `render()` prefixes every node: `${diagramId}-${node.domId || node.id}` (render.ts:86-93). Edges are prefixed in `insertEdge` (`rendering-elements/edges.js:837` and `:860`), not in `render()`.

## 3. Edge fields (flowDb.ts:1290-1344)

Per edge, `styles = [...(e.defaultStyle ?? [])]` (1293). Then the edge's own `style` is appended (1308-1310).

| Field                  | Value                                                                                                                                                                                                               | Lines                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `id`                   | `getEdgeId(start, end, { counter: index, prefix: 'L' }, rawEdge.id)`. addSingleLink always sets `rawEdge.id`, so the result is `rawEdge.id` (utils.ts:908-926 returns `id` when truthy). The index fallback is dead | 1312                 |
| `isUserDefinedId`      | `rawEdge.isUserDefinedId`                                                                                                                                                                                           | 1313                 |
| `start`, `end`         | Redirected through `collapsedAncestorMap`; unchanged if not redirected (section 4.3)                                                                                                                                | 1299-1300            |
| `type`                 | `rawEdge.type ?? 'normal'`: the arrow type string (`arrow_point`, `double_arrow_cross`, ...)                                                                                                                        | 1316                 |
| `label`                | `rawEdge.text`, `''` when none                                                                                                                                                                                      | 1317                 |
| `labelType`            | `rawEdge.labelType` (`'text'` default, 292)                                                                                                                                                                         | 1318                 |
| `labelpos`             | `'c'`                                                                                                                                                                                                               | 1319                 |
| `thickness`            | `rawEdge.stroke` (`normal`, `thick`, `dotted`, `invisible`)                                                                                                                                                         | 1320                 |
| `minlen`               | `rawEdge.length` (clamped to 10 at parse, db 313)                                                                                                                                                                   | 1321                 |
| `classes`              | `''` when stroke is `invisible`; otherwise `'edge-thickness-normal edge-pattern-solid flowchart-link'`. User edge classes are not in this string                                                                    | 1322-1325            |
| `arrowTypeStart`       | `'none'` when stroke is `invisible` or type is `arrow_open`; else from `destructEdgeType`                                                                                                                           | 1326-1329; 1028-1046 |
| `arrowTypeEnd`         | `'none'` when stroke is `invisible` or type is `arrow_open`; else from `destructEdgeType` (default `arrow_point`)                                                                                                   | 1330-1331; 1028-1046 |
| `arrowheadStyle`       | `'fill: #333'`                                                                                                                                                                                                      | 1332                 |
| `cssCompiledStyles`    | `getCompiledStyles(rawEdge.classes)` (classes from `class` and `click`)                                                                                                                                             | 1333                 |
| `labelStyle`           | The `styles` array                                                                                                                                                                                                  | 1334                 |
| `style`                | The same `styles` array object as `labelStyle`                                                                                                                                                                      | 1335                 |
| `pattern`              | `rawEdge.stroke`                                                                                                                                                                                                    | 1336                 |
| `look`                 | `config.look`                                                                                                                                                                                                       | 1337                 |
| `animate`, `animation` | Raw values, undefined when not set                                                                                                                                                                                  | 1338-1339            |
| `curve`                | `rawEdge.interpolate \|\| this.edges.defaultInterpolate \|\| config.flowchart?.curve`                                                                                                                               | 1340                 |

Not set by getData: `stroke`, `interpolate`, `text`, `length`. The edge literal has no `stroke` key (1311-1341), so `thickness` and `pattern` carry the value instead.

Notes:

- `destructEdgeType` (1028-1046): `arrow_point`, `arrow_circle`, `arrow_cross` set the end. `double_*` sets start and end to the same `*`. The defaults are start `none` and end `arrow_point`. `arrow_open` then overrides the end to `none` (1331).
- `invisible` (`~~~`) sets both arrows to `none` and classes to `''` (1323-1331).
- `defaultStyle` is the `linkStyle default` list (updateLink 400-401). It is prepended to every edge's `style` at getData time. The `fill:none` default is added only to per-edge styles, at parse time (db.rs:392; TS flowDb.ts:405-410).
- `curve` uses the default interpolate as it is at getData time, and the per-edge value from creation (295). An edge created while the default was `X` keeps `X` even if the default later becomes `Y`.

Lengths: `destructEndLink` gives `line.length - 1`, or the number of dots for `dotted` (flowDb.ts:917, 927-932). addSingleLink clamps to 10 (313).

## 4. Subgraph structure

### 4.1 Ids, members and merging (`addSubGraph`, flowDb.ts:674-768)

- Id: `_id.text.trim()` (679). If the title is the same token as the id and contains whitespace, the id is undefined (681-683). It then becomes `'subGraph' + subCount` (734). `subCount` increments on every call, named or not (737). Spec: `subGraph2`, `subGraph3` (flowDb.spec.ts:486-500).
- Members: flatten `list` (718); drop `dir` statements (698-700); drop empty strings and duplicates (702-714); `makeUniq` removes members that already belong to any earlier subgraph (750-754, 983-991); drop the subgraph's own id (753).
- Repeated id: members are appended to the first declaration (756-766). `subGraphLookup` points at the first declaration (766).
- Title: `_title.text`, sanitised (735-736), trimmed (742). `labelType` from the title's type (745) through `sanitizeNodeLabelType`.
- Direction: a `dir` statement wins. Else, if `flowchart.inheritDir` is true, use `getDirection()` (normalised header direction) `?? config.direction ?? undefined` (721-726). Otherwise undefined. This happens at parse time.

### 4.2 parentId and isGroup (flowDb.ts:1218-1229)

- `parentDB` is built only from non-hidden subgraphs.
- The loop runs from last to first (1218), and each assignment overwrites. So a member's parent is the subgraph with the **lowest index** in closing order: the innermost containing subgraph, since nested subgraphs close first (comment 1146-1147).
- A subgraph id that is a member of another subgraph gets `parentId` the same way (the id is in `sg.nodes`; test `subgraph.spec.js:253-300`).
- `isGroup` is true for expanded subgraphs and false for collapsed ones and for vertices.

### 4.3 Collapsed subgraphs (`view: 'collapsed'`, flowDb.ts:1136-1215 and 1237-1255)

- `view` is read as `subGraphLookup.get(id)?.metadata?.view === 'collapsed'` (1177-1178). The metadata comes from `addVertex` on the subgraph id (157-161), merged into the first declaration.
- `subGraphParent` (1136-1143): child id to the subgraph that directly contains it. Only subgraph ids are recorded.
- `outermostCollapsed(sgId)` (1179-1191): walks up the parent chain and keeps the last collapsed one, which is the outermost. A `seen` set guards the loop.
- `hiddenIds` and `collapsedAncestorMap` (1195-1215). For each subgraph with an outermost collapsed ancestor `A`:
  - the subgraph itself is hidden, unless it is `A`;
  - each direct member is hidden, unless it is `A`, and is redirected to `A`.
    Deeper members are covered by their own subgraph's entry.
- Collapsed node (1239-1255): `shape: 'collapsedGroup'`, `isGroup: false`, `padding: 8`, no `metadata`, plus the usual fields.
- Edge redirect (1299-1300): each end becomes `collapsedAncestorMap.get(x) ?? x`. Subgraph ids can be redirected too.
- Drop rule (1301-1306): drop the edge when `start === end` and at least one end was redirected. Self-loops on non-collapsed vertices are kept (comment 1295-1298).
- Hidden vertices stay in `this.vertices` and are skipped at output (1284).

### 4.4 colorIndex (flowDb.ts:1156-1175)

- Parent map over all subgraphs, hidden ones included (`subGraphParent` 1136-1143; `childrenOf` 1157-1163).
- Pre-order walk from the roots, which are subgraphs with no parent (1171-1175). Children are visited in `subGraphs` array order (closing order, which is source order for siblings). Each subgraph gets the next index as it is visited.
- Hidden and collapsed subgraphs keep their index, so colours do not shift when one is collapsed. Spec: `flowDb.spec.ts:363-379`.
- Plain vertices get no `colorIndex`. Subgraphs with no root (a parent cycle, section 10) get no index (by reading).

## 5. Configuration

### 5.1 How `look` is resolved

- getData reads top-level `config.look`. `resolveAppearance` (config.ts:79-107) sets top-level `theme`, `look` and `layout`. For each key, the first defined value wins, in this order: directive, `initialize()`, the diagram section (`flowchart.look`), top-level value, then the defaults.
- The diagram scope is set by `setDiagramConfigScope` (config.ts:149-152), called from `processAndSetConfigs` (mermaidAPI.ts:83). `render` clears it at the end (mermaidAPI.ts:689).
- Schema defaults: `flowchart.look` is `neo` (`schemas/config.schema.yaml:2458-2460`). The top-level `look` is `classic` (68-70). `.build/jsonSchema.ts` `generateDefaults` builds the per-diagram defaults with ajv; `defaultConfig.ts:19` spreads them in.
- Result: a default flowchart render has `config.look === 'neo'`. `config.appearance.spec.ts:73-77` asserts `neo` for flowchart, which matches.
- DB-level use with no scope (`new FlowDB()` plus `getData()`, as in flowDb.spec.ts and the parser specs): `resolveAppearance` returns early (config.ts:80-81), so `look` is the top-level default `classic`. Unverified by running the specs.

### 5.2 Config reads

| Key (default, schema line)                                 | Read at                                       | Effect                                                           |
| ---------------------------------------------------------- | --------------------------------------------- | ---------------------------------------------------------------- |
| `look` (`classic`, 68-70; `neo` for flowchart, 5.1)        | 1251, 1269, 1287 (fallback `'classic'`), 1337 | `look` on nodes and edges                                        |
| `flowchart.padding` (15, 2523-2529)                        | 1071 (`\|\| 8`)                               | vertex `padding`                                                 |
| hard-coded 8                                               | 1245, 1263                                    | subgraph `padding` (not config)                                  |
| `flowchart.minNodeWidth` (120, 2538-2547)                  | 1099                                          | vertex `minWidth`                                                |
| `flowchart.curve` (`basis`, 2501-2521)                     | 1340                                          | edge `curve` fallback                                            |
| `flowchart.inheritDir` (false, 2549-2551)                  | 724 (parse time)                              | subgraph `dir` when no `dir` statement                           |
| `direction` (no schema key)                                | 725 (parse time)                              | fallback for inheritDir. Whether a user can set it is unverified |
| `maxEdges` (500, 87-91)                                    | 330 (parse time)                              | edge limit error, not output                                     |
| `securityLevel`, `htmlLabels`, `dompurifyConfig`           | common.ts:56-83; utils.ts:257 (parse time)    | sanitising of labels, titles, tooltips, links                    |
| `flowchart.diagramPadding` (8, 2480-2482)                  | flowRenderer-v3-unified.ts:44 (after layout)  | viewport padding only                                            |
| `flowchart.nodeSpacing`, `rankSpacing` (50, 50; 2483-2499) | flowRenderer-v3-unified.ts:37-38              | set on LayoutData; see section 6                                 |
| `layout` (`elk`, 76-82)                                    | flowRenderer-v3-unified.ts:20, 35             | `layoutAlgorithm`                                                |

`wrappingWidth` (120) is read by shape rendering, not by getData.

## 6. Before layout: draw() and render()

In order, `flowRenderer-v3-unified.ts` draw (17-52), then `rendering-util/render.ts`:

1. `diag.db.setDiagramId(id)` (23). It affects only `lookUpDomId`, which getData does not call. No change to getData output.
2. `diag.db.getData()` (28).
3. `data4Layout.type = diag.type` (34).
4. `layoutAlgorithm = getRegisteredLayoutAlgorithm(layout)` (35). Falls back to `elk` with a warning (render.ts:113-128).
5. `direction = diag.db.getDirection()` (32, stored at 36). getDirection returns the header direction, normalised (flowDb.ts:587-589, 445-463). The root `elk.direction` is `dir2ElkDirection(direction ?? 'DOWN')` (elk/render.ts:715-716; mapping 318-331: `LR` RIGHT, `RL` LEFT, `TB`/`TD` DOWN, `BT` UP).
6. `nodeSpacing = conf?.nodeSpacing || 50` (37) and `rankSpacing = conf?.rankSpacing || 50` (38). A grep of `layout-algorithms/elk/` finds no reader. The readers are ddlt (`parseToLayoutData.ts:35-36` with `??`; `backends.ts:26-27` with 40/100 defaults) and swimlanes (`swimlanes/layoutCore.ts:22-23` with 40/100). For ELK these two fields have no effect (by grep; not run).
7. `markers = ['point', 'circle', 'cross']` (39), read by `insertMarkers` (`layout-algorithms/common/index.ts:142-146`).
8. `diagramId = id` (41).
9. `render(data4Layout, svg)` (43, render.ts:81-104):
   - Node domIds: `domId = `${diagramId}-${node.domId || node.id}`` for every node (86-93). The nodes are mutated in place.
   - Edge ids are not changed here. `insertEdge` prefixes them (edges.js:837, 860) and throws when `diagramId` is empty (edges.js:635-637).
   - `insertLookDefs(svg, config)` (98). Not a data change.
10. After layout: `padding = config.flowchart?.diagramPadding ?? 8` (44), title (45-50), viewport (51).

Also: `parseMmdFileToLayoutData` (ddlt/parseToLayoutData.ts:14-43) mirrors draw() as a test harness. It always sets `direction` (default `'TB'`). With `stampFlowchartRendererFields` it also sets `type`, `nodeSpacing` and `rankSpacing` with `??`, where draw uses `||`. The two differ only for `0`.

## 7. Spec tests that call getData

Every title below is in the file's own `describe`, with the getData assertion summarised. Port these 1:1. Labels and shapes are getData output unless marked "parse".

### 7.1 `parser/flow-node-data.spec.js`

Setup (beforeEach, 10-13): `flow.parser.yy = new FlowDB(); clear(); setGen('gen-2')`. Module-level `setConfig({ securityLevel: 'strict' })` (5-7). The file has 31 `it(` blocks plus 1 `it.skip` (line 410). 28 `it(` blocks and the skip call getData (29 call sites).

| Line       | Title                                                                          | getData assertions                                                                                                                       |
| ---------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| 16         | should handle basic shape data statements                                      | 1 node; shape `rounded`; label `D`                                                                                                       |
| 25         | should handle basic shape data statements (second form)                        | same                                                                                                                                     |
| 35         | ... with &                                                                     | 2 nodes; [0] `rounded`/`D`; [1] label `E`                                                                                                |
| 45         | should handle shape data statements with edges                                 | 2 nodes; `rounded`/`D`; label `E`                                                                                                        |
| 55         | ... with amp and edges 1                                                       | 3 nodes; `rounded`/`D`; label `E`                                                                                                        |
| 65         | ... with amp and edges 2                                                       | 3 nodes; `rounded`/`D`; label `E`                                                                                                        |
| 75         | ... with amp and edges 3                                                       | 4 nodes; `rounded`/`D`; label `E`                                                                                                        |
| 85         | ... with amp and edges 4                                                       | 4 nodes; `rounded`/`D`; label `E`                                                                                                        |
| 95         | ... with amp and edges 5, trailing space                                       | 4 nodes; `rounded`/`D`; label `E`                                                                                                        |
| 105        | should no matter of there are no leading spaces                                | 1 node; `rounded`/`D`                                                                                                                    |
| 116        | should no matter of there are many leading spaces                              | 1 node; `rounded`/`D`                                                                                                                    |
| 127        | should be forgiving with many spaces before the end                            | 1 node; `rounded`/`D`                                                                                                                    |
| 137        | should be possible to add multiple properties on the same line                 | 1 node; `rounded`; label `DD`                                                                                                            |
| 147        | should be possible to link to a node with more data                            | 2 nodes; [0] `squareRect`/`A`; [1] `circle`/`D`; 1 edge                                                                                  |
| 165        | should not disturb adding multiple nodes after each other                      | 3 nodes; [0] `squareRect`/`hello`; [1] `circle`/`B`; [2] `circle`/`Hello`                                                                |
| 187        | should use handle bracket end (}) character inside the shape data              | 1 node; `squareRect`; `This is }`                                                                                                        |
| 200        | should error on nonexistent shape                                              | parse: throws `No such shape: this-shape-does-not-exist.` (no getData)                                                                   |
| 207        | should error on a shape name that is only an object property                   | parse: throws `No such shape: constructor.` (no getData)                                                                                 |
| 214        | should error on internal-only shape                                            | parse: throws the lowercase message (no getData)                                                                                         |
| 222        | Diamond shapes should work as usual                                            | 1 node; `diamond`; `This is a label`                                                                                                     |
| 232        | Multi line strings should be supported (block `label: \|`)                     | 1 node; `squareRect`; `This is a\nmultiline string\n`                                                                                    |
| 247        | Multi line strings should be supported (quoted, duplicate title)               | 1 node; `squareRect`; `This is a<br/>multiline string`                                                                                   |
| 261        | should be possible to use } in strings                                         | `squareRect`; `This is a string with }`                                                                                                  |
| 274        | should be possible to use @ in strings                                         | `squareRect`; `This is a string with @`                                                                                                  |
| 287        | should be possible to use @ in strings (second, duplicate title)               | `squareRect`; `This is a string with}`                                                                                                   |
| 301        | should be possible to use @ syntax to add labels on multi nodes                | 3 nodes; labels `label for n2`, `label for n4`, `label for n5`                                                                           |
| 313        | should be possible to use @ syntax to add labels on multi nodes with edge/link | 5 nodes; labels `A`, `for B`, `for c`, `for E`, `for D`                                                                                  |
| 328        | should be possible to use @ syntax in labels                                   | 11 nodes; labels `@A@`, `@for@ B@`, `@for@ c@`, `@for@ E@`, `@for@ D@`, `@for@ H@` x2, `@for@ Q@` x2, `@for@ AS@` x2 (backticks removed) |
| 354        | should handle unique edge creation with using @ and &                          | 7 nodes; 6 edges; ids `L_A_C_0`, `L_A_D_0`, `e1`, `L_B_D_0`, `e2`, `L_A1_D1_0`                                                           |
| 371        | should handle redefine same edge ids again                                     | 7 nodes; 6 edges; ids `L_A_C_0`, `L_A_D_0`, `e1`, `L_B_D_0`, `L_A1_C1_0`, `L_A1_D1_0`                                                    |
| 388        | should handle overriding edge animate again                                    | 6 nodes; 3 edges; ids `e1`, `e2`, `e3`; animate `true`, `false`, `false`                                                                 |
| 410 (skip) | should be possible to use @ syntax to add labels with trail spaces             | Skipped upstream; would check 3 labels                                                                                                   |

### 7.2 `parser/subgraph.spec.js`

| Line | Title                                                                       | getData assertions                                                                                                                                                                    |
| ---- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 341  | should attach `@{ view: collapsed }` metadata to a subgraph and collapse it | parse: 1 subgraph, `metadata.view === 'collapsed'`. getData: node `one` has shape `collapsedGroup` and `isGroup false`; no `A`; no `B`; edge `C` to `one` exists; no edge ends at `A` |

The file also has one `it.skip` (line 75, parse only). Otherwise it has 24 `it(` blocks with no getData.

### 7.3 `flowDb.spec.ts`

Setup: a new `FlowDB` per test; no `setGen`; no config scope. 31 `it`/`it.each` blocks in total. 18 blocks (19 cases) call getData, as below. The other 13 are parse or DB-level (`exist`, `makeUniq`, `addClass`, `class` own-property, direction, repeated-id parse).

| Line | describe > title                                                                                                                                          | getData assertions                                                                      |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| 109  | flow db getData > should use defaultInterpolate for edges without specific interpolate                                                                    | `edges[0].curve === 'stepBefore'`                                                       |
| 119  | ... > should prioritize edge-specific interpolate over defaultInterpolate                                                                                 | `edges[0].curve === 'basis'`                                                            |
| 130  | ... > should support modifying interpolate using edge id syntax                                                                                           | curves: [0] `basis`, [1] `monotoneX`, [2] `catmullRom`, [3] `stepBefore`                |
| 184  | flow db collapsible subgraphs > renders a subgraph normally (as a group) when no collapse metadata is set                                                 | `sub1.isGroup === true`; nodes include A, B, C, sub1; edge C to A                       |
| 200  | ... > forwards subgraph metadata to the group node so layouts can read it                                                                                 | `sub1.isGroup === true`; metadata matches `{ algorithm: 'elk.box' }`                    |
| 214  | ... > renders a collapsed subgraph as a single collapsedGroup node and hides its members                                                                  | `sub1` shape `collapsedGroup`, `isGroup false`, label `My Group`; no A, no B; C present |
| 237  | ... > redirects edges crossing into a collapsed subgraph to the collapsed node                                                                            | edge C to `sub1`; no edge ends at `A`                                                   |
| 251  | ... > drops edges fully internal to a collapsed subgraph (avoids a self-loop)                                                                             | `edges` has length 0                                                                    |
| 263  | ... > redirects edges to the outermost collapsed ancestor for nested collapsed subgraphs                                                                  | `outer` shape `collapsedGroup`; no `inner`, no `A`; edge C to `outer`                   |
| 323  | flow db subgraph colour slots > numbers flat subgraphs in source order                                                                                    | colorIndex of first, second, third: 0, 1, 2                                             |
| 336  | ... > numbers nested subgraphs in source order, parent before its children                                                                                | Outer 0, InnerOne 1, InnerTwo 2, Sibling 3                                              |
| 363  | ... > keeps a collapsed subgraph on its own slot so the cycle does not shift                                                                              | first 0, second 1 (collapsed), third 2                                                  |
| 381  | ... > leaves plain vertices without a slot                                                                                                                | vertex `A` colorIndex undefined; subgraph `only` 0                                      |
| 417  | flow db repeated subgraph ids (issue #8326) > produces a single group node in getData with both members parented to it                                    | one `S`, `isGroup true`; `x` and `y` parentId `S`; ids unique                           |
| 429  | ... > applies a class set after a repeated declaration to the merged subgraph                                                                             | `S.cssClasses` contains `hot` (the parse part checks `classes`)                         |
| 438  | ... > collapses a repeated subgraph when its collapsed metadata comes {after the repeated declaration \| between the two declarations} (it.each, 2 cases) | one `S`; shape `collapsedGroup`; no `x`, no `y`                                         |
| 458  | ... > flattens a subgraph redeclared inside its own body instead of creating a cycle                                                                      | one `S`; no node with parentId equal to its own id; `x` and `y` parentId `S`            |
| 473  | ... > merges an empty first declaration with a later one containing a nested subgraph                                                                     | one `S`; `T` parentId `S`; `y` parentId `T`                                             |

### 7.4 Other getData users

| File and title                                                                                                                                                                                     | Use of getData                          | Assertions                                                                                                                  |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `flowDb-elk-duplicate-subgraph.spec.ts` (31-60): ELK layout of a flowchart with a repeated subgraph id > lays out a single compound node with finite coordinates                                   | Nodes fed to `runElkLayoutCore`         | After layout: `S` appears once in the ELK tree (53); ids unique (54); every node has finite `x` and `y` (56-59). M4/M5      |
| `flowRenderer-v3-unified.spec.ts` > calls setDiagramId with the svg element id                                                                                                                     | getData mocked                          | `setDiagramId` called with `test-id` only                                                                                   |
| `flowRenderer-elk-default.spec.ts` (53-70)                                                                                                                                                         | getData mocked (`nodes: []`, line 44)   | Layout choice only                                                                                                          |
| `rendering-util/unique-dom-ids.spec.ts` > Unique DOM element IDs > Full collision simulation > two flowcharts with identical nodes A->B->C produce zero duplicate IDs (158-)                       | Node and edge lists from getData        | The test prefixes domIds and edge ids itself, which simulates render.ts and not the real edge path (insertEdge)             |
| `layout-algorithms/swimlanes/__tests__/sourceOrder.spec.ts` (40, 49, 67, 82): two nodes in one lane (TB); five nodes in one lane (TB); unconnected nodes in two lanes; five nodes in one lane (LR) | Helper `layoutFromSource` calls getData | Source order into the swimlane layout. Swimlanes use `createFlowDiagram` (swimlanesDiagram.ts:8), so this is FlowDB.getData |
| `rendering-elements/lineJump.integration.spec.ts` (165): the routed swimlane layout has at least one edge crossing                                                                                 | getData at line 64                      | Edge crossing                                                                                                               |
| `layout-algorithms/ddlt/parseToLayoutData.ts` (14-43)                                                                                                                                              | Helper for `.mmd` fixtures, no titles   | Harness only                                                                                                                |

## 8. Rust status

### 8.1 Ignored tests

- `grep '#\[ignore' crates` gives 12 hits. None says `port: getData`.
- The ignores in ferrigura-syntax are: `flowchart_huge_spec.rs:28` (skipped upstream); `flowchart_subgraph_spec.rs:77` (= subgraph.spec.js:75, parse only); `flowchart_node_data_spec.rs:405` (= flow-node-data.spec.js:410, skipped upstream); `flowchart_db_spec.rs:135` (JS API); `flowchart_flow_spec.rs:173` (escaping, "checked in M5", not getData); and two `pie_spec.rs` ones.
- `getData` appears in `crates/ferrigura-syntax` only in comments: `flowchart_db_spec.rs:8-10, 342, 396, 410`; `flowchart_node_data_spec.rs:5-8`; `flowchart_subgraph_spec.rs:5-7`.
- PLAN.md section 10.2 asks for `#[ignore = "port: getData mapping, M5"]` on the getData expectations. The port did not do this. It dropped those lines and wrote comments. So M5 has no placeholder tests.
- `flowchart_db_spec.rs:142` (`mod flow_db_get_data`) ports the three interpolate tests at model level. The `curve` assertions became the model's `interpolate`.
- `flowchart_node_data_spec.rs` runs all 35 test functions. Each asserts raw shape names (for example `circle`, `rounded`) and vertex texts. The `squareRect` and `diamond` expectations of getData are not asserted anywhere in Rust.

### 8.2 Stale comments

- `flowchart_subgraph_spec.rs:8-9` says the branch's copy of the spec "does not have" the collapsed test. The worktree copy has it (`subgraph.spec.js:341`), and commit 764814a2d is in that file's history. Check before M5.
- `flowchart_node_data_spec.rs:7` says `@{ ... }` tests are ignored until step 7. There are no such ignores; all of them run.

## 9. Rust model coverage

Model: `crates/ferrigura-model/src/flowchart.rs`. Parser: `crates/ferrigura-syntax/src/flowchart/` (`db.rs`, `grammar.rs`).

Key: **Yes** = carried as is. **Derivable** = a pure function of the model, rule given. **Partial** = carried but raw, so sanitising is missing. **No** = not in the model, and needed.

### 9.1 Vertices

| Input (TS)                                                              | Rust field                                       | Status and note                                                                              |
| ----------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| `id`                                                                    | `FlowVertex.id` (65)                             | Yes                                                                                          |
| `text` (label)                                                          | `FlowVertex.text` (67)                           | Partial: raw. Sanitising missing (2.5). The model cannot tell bracket text from `@{ label }` |
| `labelType`                                                             | `FlowVertex.label_type` (69)                     | Yes. `@{ label }` defaults to Markdown (db.rs:613-619), as in TS (90-98)                     |
| `vertex.type` (shape)                                                   | `FlowVertex.shape` (71)                          | Yes. Same jison names (grammar.rs:664-800) and `@{ shape }` names (db.rs:640-660)            |
| `styles`                                                                | `FlowVertex.styles` (73)                         | Yes                                                                                          |
| `classes` (incl. `clickable`)                                           | `FlowVertex.classes` (75)                        | Yes                                                                                          |
| `link`                                                                  | `FlowVertex.link` (77)                           | Partial: trimmed. `sanitizeUrl` missing (config)                                             |
| `linkTarget`                                                            | `FlowVertex.link_target` (79)                    | Yes                                                                                          |
| `tooltip`                                                               | `FlowchartDiagram.tooltips` (53, struct 167-172) | Partial: raw, not sanitised                                                                  |
| `domId`                                                                 | none                                             | **No**: needs the addVertex call count (9.4)                                                 |
| `dir`                                                                   | none                                             | Dead in TS; emit `null`                                                                      |
| `icon`, `form`, `pos`, `img`, `constraint`, `assetWidth`, `assetHeight` | 83-95                                            | Yes                                                                                          |
| `props`                                                                 | `FlowVertex.props` (81)                          | Yes. Not in getData output. BTreeMap sorts keys (9.6)                                        |
| `haveCallback`                                                          | none                                             | By design (PLAN.md section 9)                                                                |
| `parentId`, `isGroup`                                                   | none                                             | Derivable (4.2)                                                                              |
| `cssCompiledStyles`, `cssClasses`, `cssStyles`                          | none                                             | Derivable from `classes` and `FlowClass`, rule in 9.3                                        |
| `padding`, `minWidth`, `look`                                           | none                                             | Config (section 5)                                                                           |

### 9.2 Subgraphs

| Input (TS)                          | Rust field                                                  | Status and note                                                    |
| ----------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------ |
| `id`, `title`, `labelType`, `nodes` | `FlowSubgraph.id`, `title`, `label_type`, `nodes` (139-145) | Yes. `title` is raw; TS sanitises it (2.5)                         |
| `dir` (explicit statement)          | `FlowSubgraph.dir` (147)                                    | Yes. Raw, so `TD` stays `TD`                                       |
| `dir` (inherited)                   | `FlowchartDiagram.direction` (43)                           | Partial: needs `inheritDir` and `config.direction` (config)        |
| `classes`                           | `FlowSubgraph.classes` (149)                                | Yes                                                                |
| `metadata`                          | `FlowSubgraph.metadata` (151)                               | Yes. BTreeMap order (9.6)                                          |
| collapsed                           | `metadata["view"] == "collapsed"`                           | Derivable (4.3)                                                    |
| `colorIndex`                        | none                                                        | Derivable (4.4). Needs closing order, which `subgraphs` keeps (48) |
| `parentId`, `isGroup`               | none                                                        | Derivable (4.2)                                                    |

### 9.3 Edges

| Input (TS)                                                             | Rust field                                      | Status and note                                                                    |
| ---------------------------------------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------- |
| `id`, `isUserDefinedId`                                                | `FlowEdge.id` (103), `is_user_defined_id` (105) | Yes. Generated ids match: `L_<s>_<e>_<n>` with the 0, 2, 3 counter (db.rs:312-314) |
| `start`, `end`                                                         | `start` (107), `end` (109)                      | Yes. Pre-redirect, as getData needs                                                |
| `type` (arrow)                                                         | `edge_type` (113)                               | Yes                                                                                |
| `stroke`                                                               | `stroke` (115)                                  | Yes                                                                                |
| `length` (minlen)                                                      | `length` (117)                                  | Yes. Clamped to 10 at parse (db.rs:292)                                            |
| `text` (label)                                                         | `text` (119)                                    | Partial: raw. TS sanitises at parse (301)                                          |
| `labelType`                                                            | `label_type` (121)                              | Yes                                                                                |
| `classes`                                                              | `classes` (123)                                 | Yes                                                                                |
| `style`                                                                | `style` (125)                                   | Yes                                                                                |
| `defaultStyle` (merged at getData)                                     | `default_style` (55)                            | Yes. `fill:none` added at parse (db.rs:392)                                        |
| `interpolate` (creation default, later changes)                        | `interpolate` (127)                             | Yes (db.rs:297, 412)                                                               |
| `defaultInterpolate`                                                   | `default_interpolate` (57)                      | Yes                                                                                |
| `animate`, `animation`                                                 | `animate` (129), `animation` (131)              | Yes                                                                                |
| `arrowTypeStart`, `arrowTypeEnd`                                       | none                                            | Derivable (3, rules at 1028-1046 and 1326-1331)                                    |
| `classes` string, `labelpos`, `thickness`, `pattern`, `arrowheadStyle` | none                                            | Derivable (constants and `stroke`)                                                 |
| `curve` fallback                                                       | none                                            | Config (`flowchart.curve`)                                                         |
| `look`                                                                 | none                                            | Config                                                                             |

### 9.4 Missing from the model for getData

1. **domId counter.** The model has no record of how many `addVertex` calls reached flowDb.ts:197. The suffix of `flowchart-<id>-<n>` (191) depends on it. Options: add a per-vertex `dom_counter` field, or keep the global counter in the model. PLAN.md section 9 currently leaves it out as "rendering".
2. **Text provenance and sanitisation.** Resolved for the labels: the DB sanitises the bracket, edge and title texts and not `@{ label }`, which it keeps as written (`db.rs`, `apply_vertex_data`). Not done: `dompurifyConfig`, the `securityLevel` branches of `sanitizeMore` (they run DOMPurify again, which gives the same text), and the tooltips.
3. **Config values.** Section 5. Not in the model by design.

### 9.5 Derived class styles

`FlowClass.textStyles` (not in Rust) is built in `addClass` (flowDb.ts:431-434). For each split style `s` that matches `/color/`, push `s.replace('fill', 'bgFill')`. Only the first `fill` is replaced. A style with `color` but no `fill` is pushed unchanged. `getCompiledStyles` (1105-1117) concatenates `styles` then `textStyles` for each class name, trimming after each step. So a `color:` style with no `fill` appears twice in `cssCompiledStyles`.

### 9.6 Map ordering

`FlowVertex.props` (81) and `FlowSubgraph.metadata` (151) are `BTreeMap`s, which sort keys. The DB keeps insertion order. This affects only key order in JSON, not values. Unverified whether the harness compares by value.

## 10. Trickiest rules

1. **Node order.** Subgraph nodes first, in reverse closing order (flowDb.ts:1232), then vertices in creation order (1281). Edges in creation order (1290).
2. **Edge ids skip a counter.** `A-->B` three times gives `L_A_B_0`, `L_A_B_2`, `L_A_B_3`, because the counter is `existing + 1` once a pair exists (flowDb.ts:319-327; db.rs:312-314). The getData index fallback is dead (1312).
3. **Sanitising is parse time and asymmetric.** Bracket and text labels (201), edge text (301), subgraph titles (736) and tooltips (492) are sanitised. `@{ label }` is not (245-248). Links go through `sanitizeUrl` unless `securityLevel` is `loose` (utils.ts:250-262).
4. **domId is global and prefixed in two places.** `flowchart-<id>-<n>` with `n` the count of addVertex calls (191, 197). Nodes are prefixed in `render()` (render.ts:86-93), edges in `insertEdge` (edges.js:837, 860).
5. **Subgraph structure.** `parentId` is the innermost container, found by lowest closing index (1218-1229). `colorIndex` walks all subgraphs, hidden ones included (1156-1175). Collapsed subgraphs redirect edges to the outermost collapsed ancestor (1179-1215, 1299-1306).
6. **`look` depends on scope.** `neo` in real renders (config.ts:79-107; schema 2458-2460). `classic` in DB-only use. Vertex nodes fall back to `'classic'` (1287). Subgraph and edge look do not (1251, 1269, 1337).

Other quirks:

- `style` and `labelStyle` on edges are the same array object (1334-1335).
- Edge `classes` never contains user class names. Those reach only `cssCompiledStyles` (1322-1325, 1333).
- `curve` never sets `interpolate`; getData sets `curve` only (1340).
- Nothing sets `stroke` on the edge; `thickness` and `pattern` carry it (1320, 1336).
- `clear()` resets neither `direction` (flowDb.ts:650-664) nor `vertexCounter`.
- Node `labelStyle` is `string` (types.ts:31) and edge `labelStyle` is `string[]` (types.ts:185). getData sets them accordingly.

## 11. Open questions and unverified points

1. **Mutual containment (possible parent cycle).** Reading only. Take `subgraph A` containing `subgraph B`, with B's body naming `A`. Then `B.nodes = ['A']` and `A.nodes = ['B']`, and no subgraph is a root. Consequences by reading: `declarationIndex` gets no entries for either (1158-1175); `parentDB` gives `A.parentId = B` and `B.parentId = A` (1218-1229). The walk at 1165-1170 has no `seen` guard, so if a root reaches the cycle it would recurse without end. No spec covers this. Check by running the JS DB before porting.
2. **`ellipse` shape.** Not a registry key. The renderer's behaviour for it is unverified.
3. **Sanitiser parity.** M5 needs DOMPurify-equivalent output for the default `strict` path. Not checked against DOMPurify.
4. **Subgraph `dir` inheritance.** `inheritDir` (default false) changes ELK geometry through `elk/render.ts:488-492`. The size of the effect is unverified. No flowDb spec tests inheritDir (grep finds only config.spec.ts and config.type.ts).
5. **Edge `interpolate` timing.** Derived from flowDb.ts:295 and 1340: edges created while the default was `X` keep `X`. Not tested by any spec found.
6. **Spec counts.** PLAN.md section 10.1 says 30 (flow-node-data), 23 (subgraph) and about 17 (flowDb getData). My recount: 28 `it(` blocks with getData plus 1 skip in flow-node-data (PLAN: 30 plus 2 skip/todo); 1 in subgraph (24 `it(` in total); 18 titles (19 cases) in flowDb.spec.ts. PLAN's repeated-ids figure (4) is one short: five titles call getData there (`it.each` is one title with two cases).
7. **Port decision for getData expectations.** PLAN.md section 10.2 wants ignored placeholders. The Rust port dropped them (8.1). M5 needs new tests for `squareRect`, `diamond`, `collapsedGroup`, `colorIndex`, `parentId` and the edge redirects.
8. **Reused DB instances.** `clear()` does not reset `vertexCounter` or `direction`. Matters only for the Rust harness if it reuses one DB. Unverified.
9. **Where `draw()`'s `nodeSpacing` goes.** No reader in ELK (by grep). Unverified whether ELK reads it indirectly through `data4Layout` spread.
