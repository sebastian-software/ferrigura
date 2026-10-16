# Flowchart edge drawing (M5 research notes)

Status: research notes for the Rust port. Read-only: no reference file or repo file was changed; this file is the only file written.

Reference: `packages/mermaid/src` at commit `e8af08065` (branch `sebastian/port-m5-flowchart-render`). The four core files (`edges.js`, `edgeMarker.ts`, `markers.js`, `lineJump.ts`) are byte-identical in the main checkout.

Scope: flowchart edges and edge labels, ELK layout, `htmlLabels: false`. Two looks are covered:

- `neo`: the effective default for flowcharts (section 0.1). Primary.
- `classic`: second. Shown where it differs.
  `handDrawn` is out of scope. Edge points, label centres and `curve` values come from the ELK post-processing, which another note covers; this note cites only the parts of `elk/render.ts` that affect drawing.

Tags: `[R]` read from source. `[V]` confirmed by running the code in Node 24 with inline `-e`/stdin scripts (nothing written to disk). `[U]` unverified. `[D]` derived by hand from source, not executed.

Notation: `file:line`. Paths are relative to `packages/mermaid/src/` unless shown otherwise.

---

## 0. Which code runs

### 0.1 Which look is used

| Layer                            | Value                                                                                                                                                                                    | Source                                                                 |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Global default `look`            | `classic`                                                                                                                                                                                | `schemas/config.schema.yaml:68-70`                                     |
| Flowchart section default `look` | `neo`                                                                                                                                                                                    | `schemas/config.schema.yaml:2458-2460` (flowchart block starts `2440`) |
| Resolution                       | highest first: frontmatter or directive, then `initialize()` (`siteConfigDelta`), then diagram default (`defaultConfig`); within a layer the diagram section key beats the top-level key | `config.ts:79-107`, `readAppearance` `42-53`                           |
| Edge look                        | `edge.look = config.look` (resolved)                                                                                                                                                     | `diagrams/flowchart/flowDb.ts:1337`, `getData` config `flowDb.ts:1120` |

Consequence: a flowchart with no user setting renders with `look = neo`. A top-level `look: classic` in `initialize()` beats the flowchart default, because `siteConfigDelta` is checked before `defaultConfig` `[R]`.

### 0.2 Look-dependent code on the edge path

| Where                                                                 | What depends on look                                                                      |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `edges.js:883-892`                                                    | neo only (non-animated): dash-array prefix on `style`                                     |
| `edges.js:946`                                                        | `useMargin = !animatedEdge && look == 'neo'`: margin marker variants                      |
| `edgeMarker.ts:96-100`                                                | `-margin` suffix in marker id, from `useMargin`                                           |
| `edges.js:901`                                                        | `data-look` attribute value                                                               |
| `lineJump.ts:725-757`                                                 | neo dash rewrite (only when the style contains a dash array)                              |
| `edges.js:824-848`                                                    | `handDrawn` only (out of scope)                                                           |
| `common/index.ts`, `elk/render.ts`, `createText.ts`, `createLabel.js` | no look check (grep `[R]`)                                                                |
| `styles.ts:116-157`                                                   | `[data-look="neo"]` rules target nodes, clusters and icons. None targets edge paths `[R]` |

The path `d` has no look branch: `insertEdge` builds `d` the same way for both looks (`edges.js:777-811`). Only `style`, marker ids and `data-look` differ between neo and classic.

---

## 1. Rules that matter most

1. Path geometry (`d`) is the same for neo and classic. Routed ELK edges use `rounded` (`generateRoundedPath`, radius 5), set at `elk/render.ts:2078`. Only edges with no ELK sections get `linear` (`elk/render.ts:2027`). So `flowchart.curve`, `interpolate` and `linkStyle … interpolate` have no effect on routed ELK edges `[R]`. The d3 basis/cardinal/... branches in `insertEdge` are reached only by edges that skip the ELK apply step `[U]`.
2. Neo (primary) changes the edge `style` attribute: it prefixes `stroke-dasharray: …; stroke-dashoffset: 0;`, which hides the first `oS` and last `oE` pixels of the stroke under the arrow heads (`edges.js:883-891`). Its numbers depend on the browser's `getTotalLength()` `[U]`.
3. Neo (primary) uses margin marker variants (`edges.js:946`, `markers.js:356-532`). Neo arrow heads sit about 3-4 px off the node boundary. Classic point arrows touch the boundary; classic circle and cross arrows leave a 1-2 px gap (section 4.3) `[D]`.
4. Only `arrow_point` shortens the path (4 px, `lineWithOffset.ts:12`), for both looks. `arrow_cross` and `arrow_circle` never shorten geometry `[R]`.
5. Two number formats coexist in one SVG. `rounded` paths (and the ELK `data-points` JSON) use full-precision JS number text. d3 paths (`linear`) round every coordinate to 3 decimals `[V]`.
6. The main edge label is placed at the ELK centre `(edge.x, edge.y)`, plus `subGraphTitleTotalMargin/2` (0 by default). The length-midpoint routine `calcLabelPosition` contributes a zero delta on this path `[V]`/`[D]`. Terminal labels use ELK centres when they exist. Otherwise they are placed along the path, and `calcTerminalLabelPosition` throws when the path is shorter than `25 + markerSize` `[V]`.
7. Class and style strings contain duplicated tokens (`edge-thickness-normal edge-pattern-solid` appears twice; classic `style=";"` for an unstyled edge; `stroke:#f00;;;stroke:#f00` for a styled one). Byte-level parity needs these `[V]`.
8. Neo dotted edges that get a line hop lose their dotted pattern. `lineJump.ts:735-740` reads the wrong group for the end gap, so the dash array is rebuilt as a solid-type array `[V]` (section 7.5).
9. The path `d` is built from points after any node clipping. For ELK that is no clipping at all (`skipIntersect`). Cluster cutting (`toCluster`/`fromCluster`) has no producer in `src/` and is dead `[R]`.

---

## 2. SVG structure

### 2.1 Edge path

Skeleton (attribute insertion order is the order below `[R]`, from `edges.js:857-901` and `edgeMarker.ts:22-36`):

```
<g class="edges edgePaths">                       createGraph.ts:35 (default class)
  <path
    d="{linePath}"                                edges.js:859 (same for both looks)
    id="{diagramId}-{edge.id}"                    edges.js:860
    class=" {strokeClasses}[ {edge.classes}][ {animationClass}]"   edges.js:861-867 (leading space)
    style="{styleAttr}"                           edges.js:868 (neo prefix: section 2.2)
    data-edge="true"                              edges.js:896
    data-et="edge"                                edges.js:897
    data-id="{edge.id}"                           edges.js:898
    data-points="{base64(JSON(points))}"          edges.js:899, 705
    data-look="{edge.look}"                       edges.js:901 (handleUndefinedAttr, utils.ts:934-939)
    marker-start="url({url}#{markerStartId})"     edgeMarker.ts:22-33 (only if arrowTypeStart not none)
    marker-end="url({url}#{markerEndId})"         edgeMarker.ts:34-36
  />
</g>
```

`url` is `''` unless `flowchart.arrowMarkerAbsolute` (default `false`, `config.schema.yaml:332`) or `state.arrowMarkerAbsolute` is set. When set, it is `protocol//host/pathname/search` with `(` and `)` escaped (`edges.js:933-942`).

Class string, by `flowDb` stroke value. `flowDb.ts:1320-1325`, `1336`; `insertEdge` class logic `edges.js:780-806`, `861-867`. Identical for both looks:

| `rawEdge.stroke` (flowDb) | `strokeClasses` (insertEdge)                  | `edge.classes` (flowDb `1322-1325`)                       | Final `class` attribute                                                                              |
| ------------------------- | --------------------------------------------- | --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `normal`                  | `edge-thickness-normal edge-pattern-solid`    | `edge-thickness-normal edge-pattern-solid flowchart-link` | ` edge-thickness-normal edge-pattern-solid edge-thickness-normal edge-pattern-solid flowchart-link`  |
| `thick`                   | `edge-thickness-thick edge-pattern-solid`     | same                                                      | ` edge-thickness-thick edge-pattern-solid edge-thickness-normal edge-pattern-solid flowchart-link`   |
| `dotted`                  | `edge-thickness-normal edge-pattern-dotted`   | same                                                      | ` edge-thickness-normal edge-pattern-dotted edge-thickness-normal edge-pattern-solid flowchart-link` |
| `invisible`               | `edge-thickness-invisible edge-pattern-solid` | `''`                                                      | ` edge-thickness-invisible edge-pattern-solid`                                                       |

Mapping (`edges.js:780-806`): thickness `normal|thick|invisible` maps to `edge-thickness-*`, anything else to `edge-thickness-normal`. Pattern `solid|dotted|dashed` maps to `edge-pattern-*`, anything else to `edge-pattern-solid`. `flowDb` sets `pattern = rawEdge.stroke`, so `thick` and `invisible` patterns fall to solid, and `dotted` thickness falls to normal. `flowDb` only produces `normal`, `thick`, `dotted` and `invisible` (`flowDb.ts:863-934`), so `dashed` is never produced for flowcharts `[R]`.

Animation token (`edges.js:815-821`, appended at `866`): `edge.animate` (true) → `edge-animation-fast`; `edge.animation` (string) → `edge-animation-<value>` (overrides).

### 2.2 `style` attribute, neo and classic

Let `S` = joined compiled class styles (non-label, `isLabelStyle` filtered, `edges.js:647-652`), `A` = `edge.style` array (flowDb `styles` = defaultStyle + linkStyle, `flowDb.ts:1293,1308-1310,1335`).

Classic `pathStyle` (`edges.js:850-856`, `[V]` by running those lines with mocks):

```
styles     = concat(a + ';' for a in A)
pathStyle  = (S ? S + ';' + styles + ';' : styles) + ';' + concat(';' + a for a in A)
```

Neo: when the edge is not animated (`animatedEdge` false, `edges.js:876-877`), the `style` attribute becomes `mOffset + pathStyle` (`edges.js:883-891`):

```
mOffset    = 'stroke-dasharray: ' + dashArray + '; stroke-dashoffset: 0;'
styleAttr  = mOffset + pathStyle
```

Neo dash array (`edges.js:880-887`, `generateDashArray` `601-622`):

```
oS = markerOffsets2[arrowTypeStart] || 0          // lineWithOffset.ts:21-25: arrow_point 4, arrow_cross 12.5, arrow_circle 12.5, else 0
oE = markerOffsets2[arrowTypeEnd]   || 0
L  = path.getTotalLength()                         // BROWSER value [U]; see section 10
if pattern is 'dotted' or 'dashed':                // flowchart: only 'dotted'
    k = floor((L - oS - oE) / 4), clamped to >= 0 and finite (edges.js:612-613)
    dashArray = '0 ' + oS + ' ' + join(' ', ['2 2' repeated k times]) + ' ' + oE    // edges.js:616-619; empty middle gives a double space
else:
    dashArray = '0 ' + oS + ' ' + (L - oS - oE) + ' ' + oE                        // edges.js:887
```

The dash array hides the first `oS` px and the last `oE` px of the stroke. It runs under the arrow heads.

Verified `[V]` (path length `L` passed in as a mock, `edges.js:601-622` and `880-892`):

| Input                                                                 | `style` attribute                                                               |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| classic, any edge                                                     | `;` (unstyled)                                                                  |
| neo, unstyled, `arrow_point` end, `L = 100`                           | `stroke-dasharray: 0 0 96 4; stroke-dashoffset: 0;;`                            |
| neo, unstyled, `arrow_circle` at both ends, `L = 50`                  | `stroke-dasharray: 0 12.5 25 12.5; stroke-dashoffset: 0;;`                      |
| neo, `invisible`, no arrows, `L = 40`                                 | `stroke-dasharray: 0 0 40 0; stroke-dashoffset: 0;;`                            |
| neo, `dotted`, `arrow_point` end, `L = 100`                           | `stroke-dasharray: 0 0 2 2 … 2 2 4; stroke-dashoffset: 0;;` (24 pairs of `2 2`) |
| neo, `dotted`, `L = 5`, no arrows                                     | dash `0 0 2 2 0`                                                                |
| neo, `dotted`, `L = 3`, `arrow_point` end                             | dash `0 0  4` (room below zero: no pairs, double space)                         |
| classic, `linkStyle` `stroke:#f00`                                    | `stroke:#f00;;;stroke:#f00`                                                     |
| classic, `S = ['fill:red']`, `A = ['stroke:#f00','stroke-width:4px']` | `fill:red;stroke:#f00;stroke-width:4px;;;;stroke:#f00;stroke-width:4px`         |
| `A = undefined` (not reachable from flowDb)                           | `undefined;;;undefined`                                                         |

Neo `style` for a `linkStyle` edge is the dash prefix followed by the same `pathStyle` text, for example `stroke-dasharray: 0 0 96 4; stroke-dashoffset: 0;stroke:#f00;;;stroke:#f00` `[D]`.

Stroke colour for markers: `strokeColor = pathStyle.match(/stroke:([^;]+)/)?.[1]` (`edges.js:871`). It is taken from `pathStyle`, not from the neo prefix, and it is the first match only.

Animated edges (`edge.animate === true`, `edge.animation`, or the `animation` class token, `edges.js:876-877`): neo dash and neo margins are both switched off, so the edge looks like classic `[R]`.

Data-points: `btoa(JSON.stringify(points))` (`edges.js:705`), where `points` are the points after the (skipped for ELK) node clipping and before the cluster cut and before marker offsets. JSON key order is the insertion order of each point object. Coordinates that are `NaN` become `null` in JSON `[U: spec behaviour, not executed]`.

### 2.3 Edge label group

Built by `insertEdgeLabel` (`edges.js:74-253`). Look-independent. Final DOM (plain text, non-empty label):

```
<g class="edgeLabel" transform="translate({X}, {Y + m/2})">     edges.js:81 (created); transform: common/index.ts:337 (m = subGraph margin total, default 0)
  <g class="label" data-id="{edge.id}" transform="translate({-(tx + tw/2)}, {-(ty + th/2)})">   edges.js:84, 130
    <g>                                                         createText.ts:160 (labelGroup, no attrs)
      <rect class="background" style="{edge label rect style}" x="{bx-2}" y="{by-2}" width="{bw+4}" height="{bh+4}"/>
           createText.ts:161 (rect, style "stroke: none" then overwritten at 399-401), 192-196 (geometry)
      <text y="-10.1" text-anchor="middle" style="{edge label text style}">   createText.ts:162-165, 409
        <tspan class="text-outer-tspan row" x="0" y="{i*1.1-0.1}em" dy="1.1em" text-anchor="middle">   createText.ts:96-111, 415
          <tspan font-style="normal" class="text-inner-tspan" font-weight="normal">{word}</tspan>      createText.ts:244-254
          <tspan ...> {word2}</tspan>      (second and later words are prefixed with a space)
        </tspan>
      </text>
    </g>
  </g>
</g>
```

Details:

- `(tx, ty, tw, th)` = `getBBox()` of the `<text>` element, not of the group (`edges.js:121-127`). `[R]`; the value depends on browser text metrics `[U]`.
- `(bx, by, bw, bh)` = text bbox; rect = text bbox expanded by 2 on each side (`createText.ts:191-196`).
- `edge.width = bbox.width`, `edge.height = bbox.height` use the group bbox including the rect (`edges.js:122,136-137`). `[R]`
- Wrap width defaults to 200 (`createText.ts:319`) because `insertEdgeLabel` passes `width: undefined` (`edges.js:98`).
- Empty `style` values: an unstyled edge has `labelStyle = ''` (`styles2String` returns only label-type compiled styles, `shapes/handDrawnShapeStyles.ts:75-98`), so `rect` and `text` get `style=""` `[D]`.
- `insertEdgeLabel` overwrites `edge.labelStyle` with that compiled label style (`edges.js:77-78`), so linkStyle label colours do not reach the label `[D]`.
- CSS for the label rect: `.edgeLabel rect { opacity: 0.5; fill: edgeLabelBackground }` (`diagrams/flowchart/styles.ts:209-220`).

### 2.4 Terminal labels (`startLeft`, `startRight`, `endLeft`, `endRight`)

Created in `insertEdgeLabel` (`edges.js:139-251`) via `createLabel` (`rendering-elements/createLabel.js:31-53`, `width: Infinity`, `isNode: false`, `addSvgBackground` false). Look-independent:

```
<g class="edgeTerminals" transform="translate({x}, {y})">       edges.js:142, common/index.ts:348
  <g class="inner" transform="translate({-(tx+tw/2)}, {-(ty+th/2)})">   edges.js:143, 160
    <g>
      <rect class="background" style="stroke: none"/>            createText.ts:161 (no geometry, so not drawn)
      <text y="-10.1" text-anchor="middle">                      no style attribute (createText.ts:409 selects nothing on a <text>) [D]
        <tspan class="text-outer-tspan row" x="0" y="…em" dy="1.1em" text-anchor="middle">…</tspan>
      </text>
    </g>
  </g>
</g>
```

`createFormattedText` returns the `<text>` (not the group) when `addBackground` is false (`createText.ts:199-200`), so the translate is computed from the text bbox.

### 2.5 Marker elements in the DOM

`insertMarkers(element, ['point','circle','cross'], type, diagramId)` (`common/index.ts:141-146`, flowchart list at `flowRenderer-v3-unified.ts:39`). `element` is the first `g` of the svg (`common/index.ts:135`). The markers are appended as children of that root `g`, not into `<defs>` (`markers.js:323-464`). The barb helper uses `<defs>` but is not called for flowcharts `[R]`.

Each of `point`, `circle`, `cross` inserts four markers, so 12 `<marker>` elements exist for both looks. Neo references the six margin variants; classic references the six non-margin variants (at most, depending on arrow types) `[R]`.

---

## 3. From points to `d` (both looks)

### 3.1 Pseudo-code of `insertEdge`

```
insertEdge(elem, edge, clusterDb, diagramType, startNode, endNode, diagramId, skipIntersect, layoutAlgorithm):
  if !diagramId: throw                                              edges.js:635-639
  layout = layoutAlgorithm ?? config.layout                          640-641
  points = edge.points
  // node clipping: swimlane branch (658-698) or generic (699-704), both gated by !skipIntersect.
  //   ELK passes skipIntersect = true (elk/render.ts:761), so NO clipping for ELK.
  pointsStr = base64(JSON.stringify(points))                         705
  if edge.toCluster:   points = cutPathAtIntersect(edge.points, cluster(toCluster).node)   706-711   [dead, see 9]
  if edge.fromCluster: points = reverse(cutPathAtIntersect(reverse(points), cluster.node))  713-722  [dead]
  lineData = points.filter(p => !Number.isNaN(p.y))                  724   (x is not checked)
  curveType = resolveEdgeCurveType(edge.curve)                       726; edges.js:43-45
  if curveType != 'rounded': lineData = fixCorners(lineData)        729-731
  curve = d3curve(curveType)                                         732-775
  lineFunction = d3.line().x(accX).y(accY).curve(curve)              777-778 (accessors: lineWithOffset.ts:56-184)
  linePath = (curveType == 'rounded')
       ? generateRoundedPath(applyMarkerOffsetsToPoints(lineData, edge), 5)   809-810
       : lineFunction(lineData)                                       811
  path = elem.append('path').attr(d, linePath).attr(id, diagramId+'-'+edge.id).attr(class,...).attr(style, pathStyle)   857-868
  data-* attributes                                                  896-901
  if neo and !animated:  style = mOffset + style                     883-891 (section 2.2)
  if edge.showPoints: debug circles                                  912-922 (off by default)
  url = arrowMarkerAbsolute ? location-based : ''                    933-942
  useMargin = !animated && look == 'neo'                             946
  addEdgeMarkers(path, edge, url, diagramId, diagramType, useMargin, strokeColor)   947
  midIndex = floor(points.length / 2)                                948-952
  pointsHasChanged = !isLabelCoordinateInPath(points[midIndex], d)   (utils.ts:949-957)
  paths = { updatedPath: points if pointsHasChanged, originalPath: edge.points }   954-959
  return paths
```

Notes on `paths`:

- With no clipping and no cluster cut (ELK), `points` is `edge.points` itself. When `pointsHasChanged` is true, `updatedPath` is that same array. Otherwise `updatedPath` is undefined. Either way the label delta is exactly 0 (see 6.1).
- For `rounded`, `d` comes only from `generateRoundedPath`. The d3 `lineFunction` is built (`edges.js:777-778`) but not called in that branch (`edges.js:809-811`) `[R]`.

### 3.2 Node and cluster clipping (non-ELK only, plus dead cluster branch)

Generic clip (runs only if `!skipIntersect`, `edges.js:699-704`):

```
inner = points.slice(1, edge.points.length - 1)
points = [tail.intersect(inner[0])] + inner + [head.intersect(inner[last])]
```

The original first and last points are discarded. The boundary is found along the line from the node toward the first and last inner points. For a two-point edge the slice is empty, so `intersect(undefined)` is called `[U: not executed; intersect implementation not traced]`.

Cluster cut (`edges.js:706-722`): `cutPathAtIntersect(edge.points, clusterNode)` (`478-512`) keeps points outside the cluster, replaces the first inside point by `intersection(...)` (`413-476`). The `intersection` rectangle formula is kept in the source and is not part of the ELK path `[R]`.

### 3.3 `fixCorners` (non-rounded curves only)

Applied when `curveType != 'rounded'` (`edges.js:729-731`), so to `linear`, `basis`, etc. Procedure from `edges.js:514-599`:

```
corners = []
for i in 1..n-2:
  prev, curr, next = P[i-1], P[i], P[i+1]
  A = prev.x == curr.x && curr.y == next.y && |curr.x-next.x| > 5 && |curr.y-prev.y| > 5    (521-527)
  B = prev.y == curr.y && curr.x == next.x && |curr.x-prev.x| > 5 && |curr.y-next.y| > 5    (530-536)
  if A or B: corners += i

out = []
for i in 0..n-1:
  if i in corners:
    newPrev = point at distance 5 before curr on segment prev->curr       (findAdjacentPoint 542-548: B - (5/|B-A|)*(B-A) with B = curr)
    newNext = point at distance 5 before curr on segment next->curr       (same with A = next)
    xDiff = newNext.x - newPrev.x;  yDiff = newNext.y - newPrev.y
    a = 2*sqrt(2)                                                          (566) ≈ 2.8284
    newCorner = curr
    if |next.x - prev.x| > 10 and |next.y - prev.y| >= 10:                  (568)
       r = 5
       if curr.x == newPrev.x:            // incoming vertical
          newCorner = (xDiff<0 ? newPrev.x - r + a : newPrev.x + r - a,  yDiff<0 ? newPrev.y - a : newPrev.y + a)   (575-579)
       else:                              // incoming horizontal
          newCorner = (xDiff<0 ? newPrev.x - a : newPrev.x + a,  yDiff<0 ? newPrev.y - r + a : newPrev.y + r - a)   (581-585)
    out += [newPrev, newCorner, newNext]                                   (564, 593)
  else:
    out += [P[i]]
```

Verified `[V]`: `[(0,0),(0,40),(60,40)]` becomes `[(0,0),(0,35),(2.1715728752538097,37.82842712474619),(5,40),(60,40)]`.

### 3.4 Straight (`linear`) path: accessors with marker offsets

Used for the d3 `line()` in the non-rounded branch (`edges.js:777-778`). Both looks use this only for straight-fallback edges. The accessors are in `utils/lineWithOffset.ts`.

Marker offset table (`lineWithOffset.ts:6-16`, `markerOffsets`):

| arrow                                     | offset (px)                                               |
| ----------------------------------------- | --------------------------------------------------------- |
| `aggregation`, `extension`, `composition` | 17.25                                                     |
| `dependency`                              | 6                                                         |
| `lollipop`                                | 13.5                                                      |
| `arrow_point`                             | 4                                                         |
| `arrow_barb`                              | 0                                                         |
| `arrow_barb_neo`                          | 5.5                                                       |
| `arrow_cross`, `arrow_circle`, `none`     | absent (0). `arrow_cross: 24` is commented out at line 15 |

`markerOffsets2` (`lineWithOffset.ts:21-25`) is `arrow_point: 4, arrow_cross: 12.5, arrow_circle: 12.5`. It is used only by the neo dash array (`edges.js:880-881`). It is not used for geometry in either look `[R]`.

Accessor rule (exact procedure, per coordinate `c` in {x, y}, applied to each point `d` with index `i` in array `D` of length `n`):

```
offset = 0
if i == 0 and markerOffsets[arrowTypeStart] exists:
    (dx, dy) = D[1] - D[0];  L = hypot(dx, dy)
    offset_x = m_s * dx / L     // lineWithOffset.ts:69-74 uses atan(dy/dx), cos/sin: equal to dx/L, dy/L [V]
    offset_y = m_s * dy / L     // lineWithOffset.ts:131-136
if i == n-1 and markerOffsets[arrowTypeEnd] exists:
    (dx, dy) = D[n-2] - D[n-1];  L = hypot(dx, dy)
    offset_x = m_e * dx / L     // lineWithOffset.ts:75-84
    offset_y = m_e * dy / L     // lineWithOffset.ts:137-146

// proximity push, applied to EVERY point d (including interior points), using the endpoint markers:
E = D[n-1];  S = D[0]
DIRx = (S.x < E.x) ? 'left' : 'right'        // lineWithOffset.ts:67-68
DIRy = (S.y < E.y) ? 'down' : 'up'           // 129-130
// x accessor (lineWithOffset.ts:86-118)
dXE = |d.x - E.x|;  dYE = |d.y - E.y|
if 0 < dXE < m_e and dYE < m_e:
    adj = (m_e + 1 - dXE) * (DIRx == 'right' ? -1 : +1);  offset -= adj       (99-108)
dXS = |d.x - S.x|;  dYS = |d.y - S.y|
if 0 < dXS < m_s and dYS < m_s:
    adj = (m_s + 1 - dXS) * (DIRx == 'right' ? -1 : +1);  offset += adj       (110-118)
x_out = d.x + offset_x_total
// y accessor (lineWithOffset.ts:148-180): same with x<->y, DIRy, and sign (DIRy == 'up' ? -1 : +1)
```

`m_s`/`m_e` are `undefined` for `arrow_cross`/`arrow_circle`/`none`, so the proximity rule is skipped for them. `extraRoom = 1`.

Effect: an endpoint with `arrow_point` moves back along the first or last segment by 4 px. An interior point whose x distance to that endpoint is below 4 px, and whose y distance is below 4 px, is moved so its x distance becomes 5 px. The y coordinate is not changed by the x rule. Only the endpoints get the base offset.

Verified `[V]`:

- `[(0,0),(50,0),(50,60)]`, end `arrow_point`: last point becomes `(50, 56)`.
- `[(0,0),(0,100)]`, both ends `arrow_point`: `(0,4)` and `(0,96)`.

### 3.5 Rounded path (`rounded`, the ELK routed case, both looks)

```
points' = applyMarkerOffsetsToPoints(lineData, edge)               edges.js:1053-1091
  P0' = P0 + M[arrowTypeStart] * u(P0 -> P1)                        u = (cos, sin)(atan2(P1-P0))  (1058-1071, atan2 at 1048)
  Pn' = Pn - M[arrowTypeEnd]   * u(P[n-2] -> Pn)                    (1073-1088)
  (only when M[...] is truthy; arrow_barb = 0 is skipped; no proximity push on this path)

generateRoundedPath(points', r = 5)                                 edges.js:968-1040
  out = ''
  for i in 0..n-1:
    if i == 0:      out += 'M' + x + ',' + y                        984
    elif i == n-1:  out += 'L' + x + ',' + y                        987
    else:
      v1 = C - Prev;  v2 = Next - C;  len1 = |v1|;  len2 = |v2|     990-997
      if len1 < 1e-5 or len2 < 1e-5: out += 'L' + C; continue       999-1002
      n1 = v1/len1;  n2 = v2/len2
      theta = acos(clamp(n1·n2, -1, 1))                              1005-1014 (turning angle)
      if theta < 1e-5 or |PI - theta| < 1e-5: out += 'L' + C; continue   1017-1020
      cut = min(r / sin(theta/2), len1/2, len2/2)                    1023
      S = C - n1*cut;  E = C + n2*cut                                1026-1029
      out += 'L' + S                                                 1032
      out += 'Q' + C + ' ' + E                                       1035  (control = C)
```

Output number text is JS `Number.prototype.toString` on full-precision doubles (no rounding), and no space between the two numbers of a coordinate. `[V]`

Verified `[V]`:

- `generateRoundedPath([(0,0),(0,40),(60,40)], 5)` gives `M0,0L0,32.928932188134524Q0,40 7.0710678118654755,40L60,40`.
- With `arrow_point` at the end: `…L56,40` (last point moved back 4 px).
- A near-zero segment (`0.1` long) between collinear points gives `M0,0L0,0.1L0,30`.

The cut length for a right angle is `5 / sin(π/4) = 7.0710678…`, as in the output.

### 3.6 Curve name → d3 factory (`insertEdge` `733-775`)

| `edge.curve` string                 | d3 factory                                                                                                       | Reached in ELK output?                                                             |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `rounded`                           | none: `generateRoundedPath` (`809-810`); `curveLinear` assigned but unused (`771`)                               | yes, all routed edges (`elk/render.ts:2078`)                                       |
| `linear`                            | `curveLinear` (`733-734`)                                                                                        | only edges with no ELK sections, i.e. the straight fallback (`elk/render.ts:2027`) |
| `basis`                             | `curveBasis` (`737-738`)                                                                                         | no for routed edges `[U: fallback edge cases]`                                     |
| `cardinal`                          | `curveCardinal` (`740-741`)                                                                                      | no                                                                                 |
| `bumpX` / `bumpY`                   | `curveBumpX/Y` (`743-748`)                                                                                       | no                                                                                 |
| `catmullRom`                        | `curveCatmullRom` (`749-751`)                                                                                    | no                                                                                 |
| `monotoneX` / `monotoneY`           | `curveMonotoneX/Y` (`752-757`)                                                                                   | no                                                                                 |
| `natural`                           | `curveNatural` (`758-760`)                                                                                       | no                                                                                 |
| `step` / `stepAfter` / `stepBefore` | `curveStep` / `curveStepAfter` / `curveStepBefore` (`761-769`)                                                   | no                                                                                 |
| anything else, or non-string        | `curveBasis` (`773-774`), after `resolveEdgeCurveType` falls back to `config.flowchart.curve` (`edges.js:43-45`) | no                                                                                 |

Where the string comes from:

- `flowDb.ts:1340`: `curve = rawEdge.interpolate || this.edges.defaultInterpolate || config.flowchart?.curve`.
- `config.schema.yaml:2501-2521`: `flowchart.curve`, enum of the 13 names above (`2506-2520`), default `'basis'` (`2521`). The runtime default object is generated from the schema (`defaultConfig.ts:6-9`).
- `interpolate` comes from `linkStyle … interpolate` (`flowDb.ts:377-383`) or from edge metadata `curve` (`flowDb.ts:173-175`).

ELK side (`elk/render.ts`):

- `getCurve(edge.curve ?? edge.interpolate, defaultInterpolate, confCurve)` (`2705-2713`), where `confCurve = conf.curve` (`2662`; top-level config key, `config.type.ts:430`, not in the schema `[U]`).
- `buildEdgeData` stores `edgeData.curve = interpolateToCurve(...)` (`2745-2750`). This is a d3 factory function, not a string.
- `Object.assign(edge, edgeData)` (`2666`) puts that function onto the layout edge. It is overwritten for every laid-out edge by `'rounded'` (`2078`) or `'linear'` (`2027`).
- If an edge is skipped by the early return for missing start or end nodes (`2000-2002`), its curve stays a function. `resolveEdgeCurveType` then uses the config string (`edges.js:43-45`). `[U]` whether such an edge exists in practice.

`interpolateToCurve` (`utils.ts:230-245`) maps `name` to `curve` + capitalised name in `d3CurveTypes` (`utils.ts:39-60`), falling back to the default factory. `rounded` is not in that map.

### 3.7 d3 curve text output (for `linear`, and `basis`/`step` if they become reachable)

d3-shape line serialisation (installed `d3-shape@3.2.0`, `d3-path@3.1.0`, via `d3@7.9.0`):

- `d3-shape/src/path.js:3-19`: `withPath` sets `digits = 3`, so every path is created with `new Path(3)`.
- `d3-path/src/path.js:13-24`: `appendRound(3)` writes `Math.round(v * 1000) / 1000` for every numeric argument.
- `d3-path/src/path.js:33-50`: `moveTo` → `M{x},{y}`, `lineTo` → `L{x},{y}`, `bezierCurveTo` → `C{x1},{y1},{x2},{y2},{x},{y}`. No spaces between commands `[R]`.
- `d3-shape/src/line.js:26-32`: for each point, `curve.point(+x(d,i,data), +y(d,i,data))`, with `lineStart`/`lineEnd` around the run.

Rounding details `[V]`:

- JS `Math.round` rounds ties towards +∞ (`Math.round(-2.5) = -2`). Rust `f64::round` rounds away from zero. Port with `js_round(x) = let f = x.floor(); if x - f >= 0.5 { f + 1 } else { f }`.
- `-0` prints as `0` (`30,0` from `-0.0004`).

**linear** (`d3-shape/src/curve/linear.js:19-26`): `M P0`, then `L Pi` for `i >= 1`.

- Example `[(0,0),(10.12345,0),(20,20.5555),(30,-0.0004)]` → `M0,0L10.123,0L20,20.556L30,0` `[V]`.

**basis** (`d3-shape/src/curve/basis.js:1-51`). Points `P0..P(n-1)`, `n >= 2`. Output:

```
M P0
if n == 2: L P1
else:
  L ( (5*P0 + P1)/6 )                                             basis.js:41 (case 2)
  for i in 2..n-1:  C  (2*P[i-2] + P[i-1])/3,  (P[i-2] + 2*P[i-1])/3,  (P[i-2] + 4*P[i-1] + P[i])/6     basis.js:42 → 1-10
  C  (2*P[n-2] + P[n-1])/3,  (P[n-2] + 2*P[n-1])/3,  (P[n-2] + 5*P[n-1])/6        basis.js:30 (lineEnd, case 3)
  L P[n-1]                                                        basis.js:31
```

(Each point is an (x, y) pair; the formulas apply to x and y separately. `[V]`: the example output below matches these formulas.)

- Example `[(0,0),(10.12345,0),(20,20.5555),(30,-0.0004)]` → `M0,0L1.687,0C3.374,0,6.749,0,10.082,3.426C13.416,6.852,16.708,13.704,20.021,13.704C23.333,13.704,26.667,6.852,28.333,3.426L30,0` `[V]`.
- Two points → `M0,0L10.123,0` `[V]`.

**step** (`d3-shape/src/curve/step.js:22-40`, `lineEnd` at `17-21`), `t = 0.5` (`curveStep`, line 43-45), `t = 0` (`stepBefore`, 47-49), `t = 1` (`stepAfter`, 51-53). For each point after the first:

```
if t <= 0:  L (x_prev, y),  L (x, y)                                                  step.js:28-30
else:       x1 = x_prev*(1-t) + x*t;  L (x1, y_prev);  L (x1, y)                       step.js:32-34
at lineEnd: if 0 < t < 1 and _point === 2 (true once >= 2 points): L (x_last, y_last)  step.js:18
            (_point is set to 2 at the second point and never changes, so this holds for every n >= 2)
```

- Example `[V]` (`t = 0.5`, three points) → `M0,0L5.062,0L5.062,0L15.062,0L15.062,20.556L20,20.556`.
- Example `[V]` (`t = 0`, `stepBefore`, three points) → `M0,0L0,5L10,5L10,20L20,20`. Example `[V]` (`t = 1`, `stepAfter`) → `M0,0L10,0L10,5L20,5L20,20`.

**Other curves** (`cardinal`, `bump`, `catmullRom`, `monotone`, `natural`): files are `d3-shape/src/curve/{cardinal,bump,catmullRom,monotone,natural}.js`. Not transcribed: they are reached only by the fallback edges in 3.6 `[U]`. Transcribe them only if that reachability is confirmed.

### 3.8 Number text in other code paths

| Writer                          | Format                                                          | Source                                         |
| ------------------------------- | --------------------------------------------------------------- | ---------------------------------------------- |
| d3 `line` (linear, basis, step) | round to 3 dp via `Math.round(v*1000)/1000`, then JS `toString` | `d3-path` `path.js:13-24`                      |
| `generateRoundedPath`           | full precision JS `toString`                                    | `edges.js:984-1035`                            |
| neo dash array                  | JS `toString` of `L - oS - oE` (`L` from `getTotalLength`)      | `edges.js:887`                                 |
| `lineJump` `fmt()`              | `Math.round(n*1000)/1000` then template text (3 dp)             | `lineJump.ts:266-274`                          |
| `data-points`                   | `JSON.stringify` of the point list (full precision)             | `edges.js:705`                                 |
| `translate(...)` attributes     | template text of numbers (full precision)                       | `edges.js:319,335…`, `common/index.ts:337,348` |

JS number text (for the port): shortest round-trip digits; exponent form for `|x| < 1e-6` or `|x| >= 1e21` (for example `1e-7`); `-0` prints `0`; `NaN` prints `NaN` in path text and `null` in JSON `[U: spec, not executed]`.

---

## 4. Marker offsets and marker placement

### 4.1 Where each offset table is used

| Offset table                                 | Used by                                                            | Effect                                                                                                 |
| -------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| `markerOffsets` (`lineWithOffset.ts:6-16`)   | linear accessors (3.4), rounded path (3.5), lineJump rewrite (7.4) | endpoint pulled back along the segment (only `arrow_point` = 4 for flowchart); same in neo and classic |
| `markerOffsets2` (`lineWithOffset.ts:21-25`) | neo dash array only (`edges.js:880-892`)                           | dash hidden at each end: `arrow_point` 4, `arrow_cross` 12.5, `arrow_circle` 12.5                      |

Note the mismatch in neo: a circle or cross arrow hides 12.5 px of stroke, but the geometry is not shortened (`markerOffsets` has no entry for them).

### 4.2 Marker placement (SVG semantics, `[U]` not checked in a renderer)

- Let `s = min(markerWidth / viewBoxWidth, markerHeight / viewBoxHeight)` (`preserveAspectRatio` default `xMidYMid meet`, `markerUnits = userSpaceOnUse`).
- Let `ref = (refX, refY)` in marker viewBox coordinates, and `V` = path endpoint, `θ` = path tangent at that endpoint (`orient="auto"`).
- A marker point `p` maps to `V + R(θ) · s · (p - ref)`. Centring translations cancel out, so they do not matter.
- Stroke widths inside the marker are scaled by `s` too.
- Along the tangent, the extent of each marker relative to `V` is `s·(x_min - ref.x)` to `s·(x_max - ref.x)`. Section 4.3 gives these numbers `[D]`.

### 4.3 Along-tangent extent of each flowchart marker

`V` is the path endpoint after the offset in 4.1 (`endpoint - 4` for `arrow_point`, `endpoint` for circle and cross). Offsets are in px along the tangent; positive is further into the edge.

| Marker                   | `s`   | Extent `[x_min, x_max]` relative to `V` | Tip or edge relative to the true endpoint `Q`         |
| ------------------------ | ----- | --------------------------------------- | ----------------------------------------------------- |
| classic `pointEnd`       | 0.8   | `[-4, +4]`                              | tip at `Q`; path ends mid-arrow                       |
| classic `pointStart`     | 0.8   | `[-3.6, +4.4]`                          | tip about 0.4 px inside the boundary                  |
| classic `circleEnd`      | 1.1   | `[-12.1, -1.1]`                         | circle ends 1.1 px before `Q`                         |
| classic `circleStart`    | 1.1   | `[1.1, 12.1]`                           | circle starts 1.1 px after `Q`                        |
| classic `crossEnd`       | 1     | `[-11, -2]`                             | cross ends 2 px before `Q`                            |
| classic `crossStart`     | 1     | `[2, 11]`                               | cross starts 2 px after `Q`                           |
| neo `pointEnd-margin`    | 0.913 | `[-10.5, 0]`                            | tip at `Q - 4`; arrow spans `[Q-14.5, Q-4]`           |
| neo `pointStart-margin`  | 1     | `[-1, +10.5]`                           | tip about 3 px after `Q`; arrow spans `[Q+3, Q+14.5]` |
| neo `circleEnd-margin`   | 1.4   | `[-17.15, -3.15]`                       | circle ends 3.15 px before `Q`                        |
| neo `circleStart-margin` | 1.4   | `[2.8, 16.8]`                           | circle starts 2.8 px after `Q`                        |
| neo `crossEnd-margin`    | 0.8   | `[-13.36, -2.96]`                       | cross ends 2.96 px before `Q`                         |
| neo `crossStart-margin`  | 0.8   | `[3.6, 14]`                             | cross starts 3.6 px after `Q`                         |

For `pointStart`, `V` is `Q + 4` (the start offset), so classic tip = `Q + 4 - 3.6 = Q + 0.4`. For neo `pointStart-margin`, tip = `Q + 4 - 1 = Q + 3`.

---

## 5. Marker definitions (flowchart-reachable)

`markers.js` `point` (`323-388`), `circle` (`389-464`), `cross` (`465-533`). All markers: `markerUnits="userSpaceOnUse"`, `orient="auto"`, `class="marker {type}"` (cross: `marker cross {type}`), `class="arrowMarkerPath"` on the child element, child style `stroke-width` and `stroke-dasharray` as listed.

### 5.1 Margin variants (neo, primary)

Referenced when `useMargin` is true (`edges.js:946`), i.e. neo and not animated. Marker id gets `-margin` only for `point`, `circle` and `cross` (`edgeMarker.ts:59-69`, `99`), which is every flowchart arrow type.

| Marker id suffix     | Lines     | viewBox       | refX  | refY | markerWidth × markerHeight | Child                                  | stroke-width | dasharray |
| -------------------- | --------- | ------------- | ----- | ---- | -------------------------- | -------------------------------------- | ------------ | --------- |
| `pointEnd-margin`    | `356-371` | `0 0 11.5 14` | 11.5  | 7    | 10.5 × 14                  | `path d="M 0 0 L 11.5 7 L 0 14 z"`     | 0            | `1,0`     |
| `pointStart-margin`  | `372-387` | `0 0 11.5 14` | 1     | 7    | 11.5 × 14                  | `polygon points="0,7 11.5,14 11.5,0"`  | 0            | `1,0`     |
| `circleEnd-margin`   | `427-444` | `0 0 10 10`   | 12.25 | 5    | 14 × 14                    | `circle cx=5 cy=5 r=5`                 | 0            | `1,0`     |
| `circleStart-margin` | `446-463` | `0 0 10 10`   | -2    | 5    | 14 × 14                    | `circle cx=5 cy=5 r=5`                 | 0            | `1,0`     |
| `crossEnd-margin`    | `501-515` | `0 0 15 15`   | 17.7  | 7.5  | 12 × 12                    | `path d="M 1,1 L 14,14 M 1,14 L 14,1"` | 2.5          | (not set) |
| `crossStart-margin`  | `517-532` | `0 0 15 15`   | -3.5  | 7.5  | 12 × 12                    | `path d="M 1,1 L 14,14 M 1,14 L 14,1"` | 2.5          | `1,0`     |

### 5.2 Non-margin variants (classic)

| Marker id suffix | Lines     | viewBox     | refX | refY | markerWidth × markerHeight | Child                                | stroke-width | dasharray |
| ---------------- | --------- | ----------- | ---- | ---- | -------------------------- | ------------------------------------ | ------------ | --------- |
| `pointEnd`       | `324-339` | `0 0 10 10` | 5    | 5    | 8 × 8                      | `path d="M 0 0 L 10 5 L 0 10 z"`     | 1            | `1,0`     |
| `pointStart`     | `340-355` | `0 0 10 10` | 4.5  | 5    | 8 × 8                      | `path d="M 0 5 L 10 10 L 10 0 z"`    | 1            | `1,0`     |
| `circleEnd`      | `390-407` | `0 0 10 10` | 11   | 5    | 11 × 11                    | `circle cx=5 cy=5 r=5`               | 1            | `1,0`     |
| `circleStart`    | `409-426` | `0 0 10 10` | -1   | 5    | 11 × 11                    | `circle cx=5 cy=5 r=5`               | 1            | `1,0`     |
| `crossEnd`       | `466-482` | `0 0 11 11` | 12   | 5.2  | 11 × 11                    | `path d="M 1,1 l 9,9 M 10,1 l -9,9"` | 2            | `1,0`     |
| `crossStart`     | `484-500` | `0 0 11 11` | -1   | 5.2  | 11 × 11                    | `path d="M 1,1 l 9,9 M 10,1 l -9,9"` | 2            | `1,0`     |

### 5.3 Marker id and selection

Source: `edgeMarker.ts`.

```
arrowTypesMap (39-57):  arrow_point → point (fill)   arrow_circle → circle (no fill)   arrow_cross → cross (no fill)
  arrow_open / none: no marker (84-86)
  unknown type: warn, no marker (91-94)
marginSupport: cross, point, circle (59-69)
suffix = start ? 'Start' : 'End'
id = `${diagramId}_${diagramType}-${type}${suffix}${useMargin && marginSupport ? '-margin' : ''}`   (96-100)
  e.g. neo: "mermaid-svg_flowchart-v2-pointEnd-margin";  classic: "mermaid-svg_flowchart-v2-pointEnd"
if strokeColor non-empty:                                             (102-134)
  colorId = strokeColor.replace(/[^\dA-Za-z]/g, '_')
  coloredId = `${id}_${colorId}`
  if no element with coloredId in document:
     clone the original marker element (must exist)
     set attribute stroke = strokeColor on every path/circle/line child
     if arrowTypeInfo.fill (point, but not circle/cross): set fill = strokeColor
     append the clone to the original's parent (root g)                     (129)
  use url(${url}#${coloredId})
else: use url(${url}#${id})
```

Arrow type to edge end (flowchart):

- `destructEdgeType` (`flowDb.ts:1028-1046`): default end `arrow_point`, start `none`. Double forms set both ends to the base type.
- `flowDb.ts:1326-1331`: start and end become `none` when the stroke is `invisible` or the type is `arrow_open`.
- So flowchart uses only `arrow_point`, `arrow_circle`, `arrow_cross`, `none`. Markers such as `extension`, `aggregation`, `composition`, `dependency`, `lollipop`, `barb` and the ER/requirement markers are not reached from the flowchart `[R]`.

Colour effects (CSS, `[D]`, not browser-checked):

- `styles.ts:99-105`: `.marker { fill: lineColor; stroke: lineColor }`, `.marker.cross { stroke: lineColor }`.
- Uncoloured circle: fill inherited from `.marker` (filled dot). Coloured circle: stroke = strokeColor, fill inherited (line colour).
- Coloured point: path `fill` and `stroke` attributes set to strokeColor.
- Coloured cross: path `stroke` attribute set, which beats inherited CSS.

---

## 6. Label placement (both looks)

Look does not change label placement. Only the edge path, its style and the markers differ.

### 6.1 Main label (ELK path)

```
resolveEdgeLabelPosition(edge, paths):                         edges.js:284-299
  x = edge.x;  y = edge.y
  if paths.updatedPath:
     updatedMid  = calcLabelPosition(paths.updatedPath)
     if paths.originalPath:
         originalMid = calcLabelPosition(paths.originalPath)
         x = edge.x + (updatedMid.x - originalMid.x)
         y = edge.y + (updatedMid.y - originalMid.y)
     else x, y = updatedMid
  return (x, y)

common positionRenderedEdgeLabel(edge, paths):                 common/index.ts:328-338
  if edge.label:
     (x, y) = resolveEdgeLabelPosition(edge, paths)
     edgeLabel_group.attr('transform', translate(x, y + subGraphTitleTotalMargin/2))
```

For ELK, `updatedPath` is either undefined or the same array as `originalPath` (3.1). Either way the delta is exactly 0. The main label is at `(edge.x, edge.y)`, which ELK sets to the label centre (`elk/render.ts:2080-2084`):

```
edge.x = label.x + offset.x + label.width / 2
edge.y = label.y + offset.y + label.height / 2
```

Then the inner `g.label` is centred by its own transform (`computeLabelTransform`, `labelTransform.ts:27-34`):

```
translate(-(bx + bw/2), -(by + bh/2))      where (bx, by, bw, bh) = bbox of the <text> element (edges.js:121-128)
```

`subGraphTitleTotalMargin` = `flowchart.subGraphTitleMargin.top + bottom` (`utils/subGraphTitleMargins.ts:12-14`; schema default 0/0, `config.schema.yaml:2475-2477`).

### 6.2 Length midpoint (inert for ELK, used by other layouts)

```
calcLabelPosition(points):                                     utils.ts:324-329
  if len == 1: return points[0]
  return traverseEdge(points)

traverseEdge(points):                                          utils.ts:307-319
  total = Σ_{i>=1} hypot(P[i]-P[i-1])                          distance() = 0 if either point missing (295-300)
  return calculatePoint(points, total / 2)

calculatePoint(points, D):                                     utils.ts:336-368
  prev = undefined; remaining = D
  for p in points:
    if prev:
      v = hypot(p - prev)
      if v == 0: return prev
      if v < remaining: remaining -= v
      else:
        r = remaining / v
        if r <= 0: return prev
        if r >= 1: return p                                    (unrounded)
        return { x: round5((1-r)*prev.x + r*p.x), y: round5((1-r)*prev.y + r*p.y) }
    prev = p
  throw Error('Could not find a suitable point for the given distance')   (367)

round5(v) = Math.round(v * 1e5) / 1e5                          utils.ts:331-334, 359-360
```

Verified `[V]`: L-path `(0,0)→(30,0)→(30,40)` gives `(30,5)`; two-point `(0,0)→(7,0)` gives `(3.5,0)`.

### 6.3 `pointsHasChanged` and `isLabelCoordinateInPath` (edges.js:948-952, utils.ts:949-957)

```
midIndex = floor(points.length / 2)           // a vertex, not a point at half length
p = points[midIndex]
s = d.replace(/(\d+\.\d+)/g, m => String(Math.round(parseFloat(m))))    // decimals only, integers untouched
ok = s.includes(String(Math.round(p.x))) || s.includes(String(Math.round(p.y)))   // plain substring test
pointsHasChanged = !ok
```

Note the substring test: `"12"` matches inside `"123"`. The result only matters if `updatedPath` and `originalPath` differ, which does not happen for ELK (3.1, 6.1). Port it only if non-ELK layouts need it.

### 6.4 Terminal labels

ELK provides a centre per terminal (`elk/render.ts:2085-2096`):

```
terminalLabelCenters[key] = (terminal.x + offset.x + terminal.width/2,  terminal.y + offset.y + terminal.height/2)
```

Set only for routed edges, and only when `terminal.terminal` is set and `x` or `y` is non-zero (`2088`).

`terminalLabelTranslate` (`common/index.ts:363-378`):

```
center = edge.terminalLabelCenters[key];   if present: translate(center)
path = updatedPath ?? originalPath            (for ELK: edge.points, the raw polyline)
if !path: translate(edge.x, edge.y)
arrowType = key starts with 'start' ? arrowTypeStart : arrowTypeEnd
translate( calcTerminalLabelPosition(arrowType ? 10 : 0, side, path) )
```

`calcTerminalLabelPosition(markerSize, side, points)` (`utils.ts:378-415`):

```
pts = structuredClone(points)
if side is not start_*: pts = reversed(pts)                     385-387
D = 25 + markerSize                                              390
C = calculatePoint(pts, D)                                       391   (throws if polyline length < D)
d = 10 + 0.5 * markerSize                                        394
a = atan2(pts[0].y - C.y, pts[0].x - C.x)                        396
mid = ((pts[0].x + C.x)/2, (pts[0].y + C.y)/2)
start_left, end_right:   x = -sin(a)*d + mid.x;   y = cos(a)*d + mid.y     401-406 (sin(a±π) = -sin a, cos(a±π) = -cos a)
end_left, start_right:   x =  sin(a)*d + mid.x;   y = -cos(a)*d + mid.y   407-412
```

Throw condition `[V]`: with an arrow (`markerSize = 10`), a polyline shorter than 35 throws. Without an arrow (`0`), shorter than 25 throws. Length exactly 35 gives `{x: 17.5, y: -15}`. Nothing catches this in the call chain (`common/index.ts:328-351`, `utils.ts:367`).

Edge label fallback in the non-ELK `edges.js:301-374` uses the same function with `edge.arrowTypeStart ? 10 : 0` (`331`, `343`, `357`, `368`).

### 6.5 Label origin edge case

`elk/render.ts:2028-2032` notes that if ELK gives no `x/y` for a labelled edge, `translate(undefined, NaN)` is written. The straight fallback (`2033-2036`) sets the midpoint instead.

---

## 7. Line hops (`lineJump.ts`), both looks

### 7.1 Trigger and config

- Called from `elk/render.ts:750` as `afterPaint` (run after all edges are painted, `common/index.ts:199`). Look-independent trigger.
- `elk/lineHops.ts:43-46`: if `config.elk.lineHops === false`, return. Default `true` (`defaultConfig.ts:29`, schema `config.schema.yaml:177-190`).
- `jumpStyle = (lineHops === 'gap') ? 'gap' : 'arc'` (`lineHops.ts:64`). `jumpRadius = 6` (`lineHops.ts:13`, `63`), fixed, not configurable (comment at `lineHops.ts:4-12`).
- Swimlanes use the same code with radius 6 and `swimlane.lineHops` (`swimlanes/adjustLayout.ts:9-27`). Not flowchart.

### 7.2 Inputs

- Geometry: `data-points` decoded from each `path[data-id]` (`lineJump.ts:617-637`, `674-691`). Fallback: `edge.points`. These are the points before marker offsets, so the rewrite is consistent.
- `curve` from the layout edge (`lineHops.ts:53-58`): `rounded` or `linear` for ELK.
- `look` is not read here. Only the dash-array text in the style is looked at (7.5).
- Only edges that have at least one crossing are rewritten (`lineJump.ts:693-726`).

### 7.3 Crossing detection (`findEdgeIntersections`, `205-264`)

```
for each edge pair (i < j):
  for each segment A of i, segment B of j:
     hit = segmentIntersection(A, B)      parametric; denom = 0 → none; tA, tB must be in (1e-6, 1-1e-6)  (118-154, ENDPOINT_EPSILON 91)
     if crossingSitsInRoundedCorner(i, segA, tA) or crossingSitsInRoundedCorner(j, segB, tB): skip   (224-229)
     horizontal(X) = |dx| >= |dy|      (159-161)
     orthogonalPair = horizontal(A) != horizontal(B)                   (236-238)
     jumpOnA = orthogonalPair ? horizontal(A) : false                  (239)
     jump edge = jumpOnA ? i : j       // so a non-orthogonal crossing always jumps the later edge
```

`crossingSitsInRoundedCorner` (`179-203`) is true when the crossing lies within `cutLen` of a rounded bend on either side. It uses `computeRoundedCorner` with radius 5 (`379-414`).

### 7.4 Rewrite (`rewriteEdgePath`, `416-534`)

```
pts = applyMarkerOffsets(points)              310-335  (same rule as 3.5, using markerOffsets and atan2)
rounded = curve == 'rounded'
parts = ['M' + fmt(pts[0])]
for i in 0..segments-1:
  (a, b) = (pts[i], pts[i+1]);  L = |b-a|;  u = (b-a)/L
  sweep = horizontal(a,b) ? (dx >= 0 ? 1 : 0) : (dy >= 0 ? 1 : 0)              281-294
  consumed = rounded && i > 0 ? cutLen(corner at pts[i]) : 0                     462-472
  stop     = rounded && i < last ? L - cutLen(corner at pts[i+1]) : L            475-487
  jumps on this segment, sorted by t:
     d_k = t_k * L
     room_k = min(d_k - consumed, stop - d_k) - 2              (CORNER_JUMP_CLEARANCE = 2, line 32)
     r_k = min(6, room_k)
     keep if r_k >= 3.6                                         (MIN_USEFUL_RADIUS_RATIO 0.6 × 6, lines 49, 493-500)
  adjacency: for consecutive k, k+1:
     gap = d_{k+1} - d_k
     if r_k + r_{k+1} > gap: r_k = min(r_k, gap/2); r_{k+1} = min(r_{k+1}, gap/2)     501-508
  for each jump with r >= 3.6 (checked again, 515-517):
     pre  = P - u*r;   post = P + u*r
     arc: parts += 'L' + fmt(pre), 'A' + fmt(r) + ',' + fmt(r) + ' 0 0 ' + sweep + ' ' + fmt(post)    352-354
     gap: parts += 'L' + fmt(pre), 'M' + fmt(post)                                                 355-357
  end of segment:
     if rounded and a corner follows: parts += 'L' + S', 'Q' + C' + ' ' + E'                       523-527
     else: parts += 'L' + fmt(b)                                                                  529
return parts.join(' ')                         533  (joined with spaces, unlike generateRoundedPath)
```

`fmt(n) = Math.round(n*1000)/1000` as text (`266-270`). Output uses 3 dp and spaces between commands.

Important: for `rounded` edges the rewrite keeps the `Q` corners (`523-527`). The doc comment at `593-595` says corner rounding is dropped, which contradicts the code `[R]`.

### 7.5 Dash-array update (neo only, and the dotted bug)

`applyLineJumpsToSvg` (`646-759`):

- Writes the new `d` (`742-743`).
- Reads the original style (`735`). Matches `/stroke-dasharray\s*:\s*0\s+([\d.]+)\s+[\d.]+\s+([\d.]+)/` (`736-738`), which only fits the solid neo form `0 S L E`. Captures `S` and `E` (`739-740`).
- If there is a match, it recomputes `onLen = max(0, getTotalLength() − S − E)` (`750-752`), writes `0 S onLen E` (`752`), and replaces the dash array in the style (`753-755`). The `;\s*;+` → `;` collapse at `755` also shortens the neo style's `;;` tail to `;`.
- Classic has no dash array, so the match fails and this step is skipped `[R]`.

Verified `[V]` (regex run on the neo strings from 2.2):

- Solid, `0 0 96 4`: captures `S = 0`, `E = 4`. Correct.
- Dotted, `0 0 2 2 2 2 … 2 2 4`: captures `S = 0`, `E = 2`. Wrong: `E` is the first `2` of the pattern, not the real end gap `4`. The rewrite then writes `0 0 onLen 2`, which drops the dotted pattern and leaves a solid-type line `[D]`.
- The same mis-capture happens for any dotted neo edge with `oS` non-zero: the captured `S` is correct, but `E` still picks up a pattern value.

Impact: dotted neo edges that receive a line hop are drawn with a solid dash, not dotted (`[D]`, not rendered).

### 7.6 Verified examples `[V]`

Horizontal `A = [(0,50),(100,50)]`, vertical `B = [(50,0),(50,100)]`, both `linear`, `A` end `arrow_point`:

- crossing `[{jumpEdgeId: A, segIndex: 0, t: 0.5, point: (50,50)}]`
- arc on A: `M0,50 L44,50 A6,6 0 0 1 56,50 L96,50`
- gap on A: `M0,50 L44,50 M56,50 L96,50`
- B is unchanged (`processEdgesWithJumps` writes plain paths for non-jumping edges, but the pipeline only rewrites jumping edges)

Rounded A `[(0,50),(100,50),(100,90)]`, arrow end `arrow_point`:

- `M0,50 L44,50 A6,6 0 0 1 56,50 L92.929,50 Q100,50 100,57.071 L100,86`

Crossing 4 px from start (`x = 4`): room = 2 < 3.6, so the hop is dropped and the path is unchanged `M0,50 L100,50`.

---

## 8. Edge ids and attributes (cross-reference)

| Item                 | Value                                                                      | Source                                                                                  |
| -------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ---------------------- |
| Edge id (no user id) | `L_{start}_{end}_{index}`, `index` is the position in the parsed edge list | `flowDb.ts:1291,1312`, `utils.ts:908-931`                                               |
| User-given id        | used as is                                                                 | `utils.ts:908-931`                                                                      |
| Path element id      | `{diagramId}-{edge.id}`                                                    | `edges.js:860`                                                                          |
| Label `data-id`      | `edge.id`                                                                  | `edges.js:84`                                                                           |
| Path `data-id`       | `edge.id`                                                                  | `edges.js:898`                                                                          |
| Marker id            | `{diagramId}\_{diagramType}-{type}{End                                     | Start}[-margin]` (`-margin` for neo)                                                    | `edgeMarker.ts:96-100` |
| `diagramId`          | the `id` passed to `draw`                                                  | `flowRenderer-v3-unified.ts:23,41`                                                      |
| `diagramType`        | `diag.type`, expected `'flowchart-v2'`                                     | `flowRenderer-v3-unified.ts:34`, `flowDetector-v2.ts:7` `[U: runtime value not traced]` |
| Coloured marker id   | `{markerId}_{colour with non-alnum → '_'}`                                 | `edgeMarker.ts:102-106`                                                                 |
| `data-look`          | `neo` (default) or `classic`                                               | `edges.js:901`                                                                          |
| Label `class`        | `edgeLabel`; `label`                                                       | `edges.js:81,84`                                                                        |
| Terminals `class`    | `edgeTerminals`; `inner`                                                   | `edges.js:142-143`                                                                      |

---

## 9. Dead, inert or unused code (for the port: skip, or keep for non-ELK)

| Item                                                                                                                    | Status                                                                                        | Evidence                  |
| ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------- |
| `toCluster` / `fromCluster` branches (`edges.js:706-722`), `cutPathAtIntersect` (`478-512`), `intersection` (`413-476`) | Dead. No code in `src/` sets `toCluster` or `fromCluster`; `intersection` has no importer     | grep `[R]`                |
| Node clipping in `insertEdge` (`699-704`)                                                                               | Not run for ELK (`skipIntersect: true`, `elk/render.ts:761`). Used by other callers           | `[R]`                     |
| Swimlane clip branch (`658-698`) and `orthogonalizeToLabelClippedPoints` (`379-401`)                                    | Swimlanes only                                                                                | `[R]`                     |
| `edges.js` `positionEdgeLabel` (`301-374`)                                                                              | Not on the ELK path                                                                           | section 0                 |
| `processEdgesWithJumps` (`547-578`)                                                                                     | Exported, not used in the pipeline (only `applyLineJumpsToSvg` is)                            | grep `[R]`                |
| `calcLabelPosition` midpoint for the main label                                                                         | Zero delta on ELK path                                                                        | 6.1 `[V]`/`[D]`           |
| `handDrawn` branch (`824-848`)                                                                                          | Out of scope                                                                                  | `[R]`                     |
| `showPoints` debug circles (`912-922`)                                                                                  | Off by default                                                                                | `[R]`                     |
| `pattern === 'dashed'` branch (`edges.js:886`)                                                                          | Not produced by flowDb for flowcharts (section 2.1)                                           | `flowDb.ts:863-934` `[R]` |
| `.edgePaths .path` CSS rule (`flowchart/styles.ts:199-202`)                                                             | No emitted element has a `path` class token; rule matches nothing for insertEdge output `[D]` | `edges.js:861-867`        |

---

## 10. Open questions and unverified items

1. `[U]` `getTotalLength()` (neo dash array at `edges.js:879`, lineJump neo rewrite at `750`) is browser-computed. The port must compute the arc length of its own `L`, `Q` and `A` segments. Exact agreement with browser output digits is not guaranteed, so compare with a tolerance. Browsers round through their own path flattening.
2. `[U]` Text metrics. Label `translate`s, `edge.width/height` and rect geometry all use browser `getBBox`. The port needs font metrics that match the reference renderer.
3. `[U]` Runtime value of `diag.type` (expected `'flowchart-v2'`), which becomes part of every marker id.
4. `[U]` Whether any flowchart edge reaches `insertEdge` with a function-valued `curve` (the early return at `elk/render.ts:2000-2002`). If so, the drawing falls back to `config.flowchart.curve` (typically `basis`).
5. `[U]` Two-point edges through the non-ELK clip branch: `points.slice(1, n-1)` is empty and `intersect(undefined)` is called (`edges.js:701-703`). Not executed. Relevant only for callers that pass `skipIntersect = false` (for example `diagrams/block/renderHelpers.ts:266-281`).
6. `[U]` Marker overflow (clipping to viewport) and `preserveAspectRatio` behaviour are assumed to follow the SVG spec, not checked in a renderer. The neo tip positions in 4.3 are `[D]` from those assumptions.
7. `[U]` Coloured circle and cross appearance depends on CSS inheritance (`styles.ts:99-105`) and was not rendered.
8. `[U]` `NaN` coordinates: `generateRoundedPath` prints `NaN`; `JSON.stringify` prints `null`; `lineData` drops only `NaN` y (`edges.js:724`). Reachable only with degenerate ELK output.
9. `[U]` Duplicate endpoints in the linear accessors: `atan(dy/0)` returns ±π/2 and `atan(0/0)` returns NaN (`lineWithOffset.ts:46`), giving a tiny offset or NaN. The rounded path uses `atan2` and is safe (`edges.js:1048`).
10. `[U]` `conf.curve` (top-level config key, `config.type.ts:430`) feeds the ELK internal default (`elk/render.ts:2662`). It is overwritten, so it should not affect output, but I did not check every consumer.
11. `[U]` Attribute order in the SVG is insertion order. This note assumes byte-level comparison may depend on it.
12. `[D]` Whether the neo "gap" arrows (3-4 px off the boundary, section 4.3) are the intended design or a side effect of `refX`. The reference output is the only arbiter; the port should match the numbers, not the intent.

---

## 11. Verification log (read-only; nothing written to disk)

Each item was run with inline Node 24 code (`node -`, `node --input-type=module -e`, `node --experimental-strip-types -`), either on source extracted from the reference files with `sed` into stdin or on the installed d3 packages:

1. `edges.js` 514-599 (`fixCorners`, `extractCornerPoints`, `findAdjacentPoint`), 968-1091 (`generateRoundedPath`, `applyMarkerOffsetsToPoints`): outputs in 3.3 and 3.5.
2. `edges.js` 850-856 (`pathStyle` formula) with mocked inputs: outputs in 2.2 (classic).
3. `edges.js` 601-622 (`generateDashArray`) and 880-892 (neo style prefix), with mocked path lengths: outputs in 2.2 (neo).
4. `utils/lineWithOffset.ts` 1-184 (accessors) with stdin type stripping: outputs in 3.4.
5. `d3-shape@3.2.0` (`src/curve/*.js`, `src/line.js`, `src/path.js`) and `d3-path@3.1.0`: linear, basis, step (including `stepBefore`/`stepAfter`) outputs in 3.7.
6. `lineJump.ts` (imports replaced by constants, otherwise verbatim): `findEdgeIntersections`, `processEdgesWithJumps`, `isStraightPath` outputs in 7.6.
7. The neo `lineJump` dash regex (`lineJump.ts:736`) run on the neo style strings from item 3: dotted capture `E = 2` (section 7.5).
8. `utils.ts` 295-368 and 378-415 (`distance`, `traverseEdge`, `calculatePoint`, `calcTerminalLabelPosition`): outputs in 6.2 and 6.4, including the throw.

Not run: the full renderer, `getBBox`/text metrics, any browser SVG path or marker rendering, `getTotalLength`, and the ELK layout itself.
