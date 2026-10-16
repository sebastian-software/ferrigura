# ELK result application and paint (M5 research notes)

Status: read-only research. Nothing was run (no tests, no browser render). Every rule below comes from reading the code. Anything that depends on runtime behaviour, or on a convention I did not check against elkjs output, is marked **unverified**.

Source: worktree `.claude/worktrees/port-m5-flowchart-render`, branch `sebastian/port-m5-flowchart-render`, HEAD `e8af08065`. `elk/render.ts` there is byte-identical to the main checkout copy.

## 0. Scope and file key

Scope: `elk/render.ts` lines 1282 to 3021 (end), `elk/geometry.ts` (320 lines), `elk/lineHops.ts` (66 lines), and the paint and afterPaint path of the `render` export. Earlier `render.ts` code is read only where it produces inputs to these steps.

All paths are relative to `packages/mermaid/src/`. Short keys used in citations:

| Key | File                                                                                             |
| --- | ------------------------------------------------------------------------------------------------ |
| R   | `rendering-util/layout-algorithms/elk/render.ts`                                                 |
| G   | `rendering-util/layout-algorithms/elk/geometry.ts`                                               |
| L   | `rendering-util/layout-algorithms/elk/lineHops.ts`                                               |
| FCA | `rendering-util/layout-algorithms/elk/find-common-ancestor.ts`                                   |
| C   | `rendering-util/layout-algorithms/common/index.ts` (the generic `render` factory and paint loop) |
| CG  | `rendering-util/createGraph.ts`                                                                  |
| E   | `rendering-util/rendering-elements/edges.js`                                                     |
| LJ  | `rendering-util/rendering-elements/lineJump.ts`                                                  |
| CL  | `rendering-util/rendering-elements/clusters.js`                                                  |
| N   | `rendering-util/rendering-elements/nodes.ts`                                                     |
| LW  | `utils/lineWithOffset.ts`                                                                        |
| U   | `utils.ts`                                                                                       |
| LT  | `rendering-util/labelTransform.ts`                                                               |
| FDB | `diagrams/flowchart/flowDb.ts`                                                                   |
| SCH | `schemas/config.schema.yaml`                                                                     |

---

## 1. Post-processing order, elkjs output to painter input

Numbers are the stage order. Each stage consumes the output of the previous one.

| #   | Stage           | Call site                                                                                         | Effect                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | --------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Prepare         | `prepareLayoutForElk` (R:678-685), which calls `applyElkEdgeRenderData` (R:683, body R:2650-2668) | Sets `edge.curve` (as a d3 factory), `arrowTypeStart/End`, `classes`, `style`, `labelStyle`, `thickness`, `pattern`, `arrowhead`, `label` on each edge. Runs before measure.                                                                                                                                                                                                                                                                                   |
| 2   | Measure         | `createGraphWithElements(..., {unwrapGroupLabels:true})` (C:166-168, CG:122-192)                  | Non-group nodes: `node.width/height` = shape `getBBox` (CG:61-70, 175-182). Groups: `labelBBox` only, measured unwrapped (CG:42-57, 161-173). Edges: label inserted, `edge.width/height` (E:136-137) and `terminalLabelSizes` (E:263-265) recorded (CG:188-191). Shape `intersect` functions are attached here (see section 6).                                                                                                                                |
| 3   | Build ELK graph | `buildElkGraphFromLayoutData` (R:704-729)                                                         | `createElkNode` makes a **shallow copy** `{...node}` into `nodeDb` (R:1041-1066). Groups get `labels`, `labelData`, `layoutOptions` (R:1191-1222). Edges go to ELK with `labels[0]` = main label (CENTER) and terminal labels (R:1089-1142, terminal labels R:1152-1173). Cross-hierarchy `INCLUDE_CHILDREN` (R:1224-1277). Feedback edges reversed (R:723-725, 736-747).                                                                                      |
| 4   | ELK layout      | `runElkLayout` (R:1282-1323)                                                                      | elkjs `layout()`. Output: `children[]` (x,y top-left, relative to parent), `edges[]` (sections, labels).                                                                                                                                                                                                                                                                                                                                                       |
| 5   | Apply result    | `applyElkLayoutResult` (R:1325-1338)                                                              | Runs 5a, 5b, 5c in this order.                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 5a  | Node positions  | `applyElkNodePositions` (R:1509-1579)                                                             | Absolute top-left, then centre. See 2.1.                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 5b  | Group frames    | `evenGroupFrames` (R:1423-1507)                                                                   | Pulls each frame in around its contents. Runs deepest-first. See 2.2.                                                                                                                                                                                                                                                                                                                                                                                          |
| 5c  | Edge layout     | `applyElkEdgeLayout` (R:1931-2126)                                                                | See 2.3 to 2.8. Order inside it: (i) read `straightenEdges` and delete old `terminalLabelCenters` (R:1939-1944); (ii) degenerate-node alignment pre-pass (R:1951-1982); (iii) per-edge routing (R:1984-2100); (iv) straightening and main-label projection (R:2102-2118); (v) `followMovedEndpoints` (R:2119-2121); (vi) `slideTerminalLabelsOffFrames` (R:2123); (vii) `putTerminalLabelsOnTheirSide` (R:2124); (viii) `separateOppositeEdgeLabels` (R:2125). |
| 6   | Paint order     | `orderNodesForElkPaint` (R:700, body R:765-779)                                                   | Sorts `data4Layout.nodes` in place: groups first, by ascending depth. Non-groups keep their order.                                                                                                                                                                                                                                                                                                                                                             |
| 7   | Paint           | `paintLayoutData` (C:220-245)                                                                     | Nodes first (C:229-233), then edges (C:238-243). See section 4.                                                                                                                                                                                                                                                                                                                                                                                                |
| 8   | afterPaint      | `applyElkLineJumps` (C:199, L:39-66)                                                              | Rewrites `d` for crossing edges. See 4.4.                                                                                                                                                                                                                                                                                                                                                                                                                      |

Stage 5b runs before 5c. That order matters: routing reads the moved frame through `boundsFor` (R:1333-1335), while `calcOffset` reads the original ELK origin through `elkOrigin` (R:2458-2462).

---

## 2. Coordinate conventions

| Quantity                                | Frame                                            | Origin / anchor                                                                     | Where set                   |
| --------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------- | --------------------------- |
| ELK `node.x, node.y`                    | ELK                                              | top-left, relative to the parent's content origin (root: absolute)                  | elkjs output                |
| `nodeDb[id].offset.posX/posY`           | absolute                                         | top-left, absolute. `posX = elk.x + parent.posX`                                    | R:1523-1534                 |
| `offset.x/offset.y`                     | absolute                                         | the parent's absolute origin (`relX/relY`). Written but unused for maths            | R:1530-1531                 |
| `node.x, node.y` (layout and nodeDb)    | absolute                                         | **centre**: `posX + width/2`                                                        | R:1535-1536, R:1542-1543    |
| `node.width/height` (non-group)         | —                                                | ELK size = measured size                                                            | R:1537-1538, R:1544-1547    |
| Group frame after `evenGroupFrames`     | absolute                                         | left, right and bottom pulled in; top is ELK's top. `x,y` = centre of the new frame | R:1459-1495                 |
| `group.elkOrigin`                       | absolute (ELK)                                   | ELK top-left, kept for `calcOffset` only                                            | R:1488                      |
| ELK edge section points and bend points | relative to the **common ancestor**'s ELK origin | add `calcOffset(src,dst)`                                                           | R:2431-2445, R:2447-2467    |
| ELK edge and terminal label `x,y`       | relative to the same offset                      | top-left. Centre = `x + off.x + w/2`                                                | R:2082-2083, R:2093-2094    |
| `edge.points` (after routing)           | absolute                                         | first and last point on the node border (or group frame)                            | R:2046-2064 and R:2469-2551 |
| `edge.x, edge.y`                        | absolute                                         | main label **centre**                                                               | R:2082-2083                 |
| `edge.terminalLabelCenters[key]`        | absolute                                         | terminal label **centre**                                                           | R:2091-2095                 |
| `edge.terminalLabelSizes[key]`          | —                                                | unpadded measured size. The +6 padding is only in the ELK input (R:1162-1169)       | E:263-265                   |

**calcOffset** (R:2447-2467): `A = findCommonAncestor(src,dst)` (FCA:6-31). If `A` is undefined or `'root'`, the offset is (0,0). Otherwise the offset is `(A.elkOrigin ?? A.offset).posX/posY`.

**findCommonAncestor** (FCA:6-31): if `a == b`, return `parent(a) || 'root'`. Otherwise walk `a`'s chain (including `a`) into a set, then walk `b`'s chain and return the first hit, else `'root'`. When one endpoint is an ancestor of the other, that ancestor is returned.

**Paint-side conventions** (section 4):

- Non-group node: `translate(node.x, node.y)`, centre (N:77-97).
- Group frame: top-left = `(x - W/2, y - H/2)` (CL:63, CL:71-72, attrs CL:99-103).
- Group title: `translate(x - bboxW/2, y - H/2 + subGraphTitleTopMargin)` (CL:108-113).
- Main edge label: `translate(x, y + (top+bottom)/2)` (C:331-337). Inner label is centred by `translate(-w/2,-h/2)` (E:130; LT:27-34).
- Terminal label: `translate(cx, cy)` (C:348, C:364-371). Inner is centred the same way (E:160 and siblings).

**Title margins.**

- Layout reserves the title strip with the measured label height (`labelData.height = labelBBox.height - 2`, R:1068-1087), `nodeLabels.placement '[H_CENTER V_TOP, INSIDE]'` (R:429), `elk.padding` top = 24 (R:428), and a width floor `nodeSize.minimum = labelW + padding` (R:359-367). The floor in `evenGroupFrames` is `labelData.width + node.padding` (R:347-356, R:1467).
- `subGraphTitleMargin` is **not read anywhere under `elk/`**. It is consumed only at paint (CL:108-112, C:331-337). The top margin moves the painted title down, but the layout does not reserve it. Schema defaults are `top: 0`, `bottom: 0` (SCH:2464-2477).

---

## 3. Geometric rules

### 3.1 Constants

| Name                                       | Value                                                                                                                                                           | Where               | Used by                                                                                  |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- | ---------------------------------------------------------------------------------------- | ----- | ---------- |
| `SUBGRAPH_PADDING`                         | 24                                                                                                                                                              | R:183               | `evenGroupFrames` insets (R:1460-1462); `elk.padding` (R:428)                            |
| `PORTS_SURROUNDING_MARGIN`                 | 12                                                                                                                                                              | R:179               | alignment threshold `2*12 = 24` (R:2403)                                                 |
| `MIN_END_MARKER_SEGMENT_LENGTH`            | 8                                                                                                                                                               | R:134               | marker guard floor (R:2601, R:2632)                                                      |
| `TERMINAL_LABEL_MARKER_CLEARANCE`          | 6                                                                                                                                                               | R:1145              | padding added to a terminal label box given to ELK if the end has a marker (R:1162-1169) |
| `TERMINAL_JOG_MAX`                         | 16                                                                                                                                                              | R:1594              | max jog size (R:1710)                                                                    |
| `TERMINAL_RUN_MAX`                         | 30                                                                                                                                                              | R:1606              | max distance `                                                                           | p1-p0 | ` (R:1706) |
| `JOG_EPS`                                  | 0.01                                                                                                                                                            | R:1619              | axis tolerance (R:1625, R:1628) and min jog (R:1710)                                     |
| `OPPOSITE_LABEL_MIN_GAP`                   | 4                                                                                                                                                               | R:2129              | label separation (R:2160)                                                                |
| `TERMINAL_LABEL_FRAME_CLEARANCE`           | 2                                                                                                                                                               | R:2268              | slide blocking (R:2304)                                                                  |
| `EPS` (shared)                             | 1                                                                                                                                                               | G:20                | `ensureTrulyOutside` trigger (G:113)                                                     |
| `PUSH_OUT`                                 | 10                                                                                                                                                              | G:21                | radial push (G:119-120)                                                                  |
| `onBorder` default tolerance               | 0.5                                                                                                                                                             | G:23                | node/frame border test (`sanitize` R:2504-2505)                                          |
| `onBorder` tolerance used by marker guards | 1                                                                                                                                                               | R:2605, R:2636      |                                                                                          |
| `OUTLINE_RAY_STEPS`                        | 20                                                                                                                                                              | G:174               | bisection steps (G:265)                                                                  |
| `DEPARTURE_AXIS_EPS`                       | 1e-6                                                                                                                                                            | G:182               | diagonal test (G:245)                                                                    |
| `insideOutline` tolerance                  | 1e-9                                                                                                                                                            | G:200               |                                                                                          |
| `replaceEndpoint` tolerance                | 0.1                                                                                                                                                             | G:290               | duplicate endpoint (G:297-318)                                                           |
| `approxEq` / `isCenterApprox`              | 1e-6                                                                                                                                                            | R:2784, R:2788-2789 | centre detection                                                                         |
| dedupe tolerance                           | 1e-6                                                                                                                                                            | R:2563              | consecutive duplicates                                                                   |
| tail trim                                  | `< 2` px                                                                                                                                                        | R:3008              | last point dropped if too close to the one before                                        |
| `CORNER_EPSILON`                           | 1e-5                                                                                                                                                            | LJ:22               | corner test; `generateRoundedPath` uses `epsilon = 1e-5` (E:968-1038)                    |
| `ENDPOINT_EPSILON`                         | 1e-6                                                                                                                                                            | LJ:91               | `t` must lie in (eps, 1-eps) on both segments for a true crossing (LJ:139-146)           |
| `ROUNDED_CORNER_RADIUS`                    | 5                                                                                                                                                               | LJ:18; E:810        | corner radius for `rounded` paths                                                        |
| `CORNER_JUMP_CLEARANCE`                    | 2                                                                                                                                                               | LJ:32, used LJ:497  | straight run kept next to a hop                                                          |
| `MIN_USEFUL_RADIUS_RATIO`                  | 0.6                                                                                                                                                             | LJ:49, used LJ:493  | hop dropped if radius < 3.6                                                              |
| `JUMP_RADIUS`                              | 6                                                                                                                                                               | L:13                | hop radius                                                                               |
| `markerOffsets`                            | aggregation 17.25, extension 17.25, composition 17.25, dependency 6, lollipop 13.5, arrow_point 4, arrow_barb 0, arrow_barb_neo 5.5 (arrow_cross commented out) | LW:6-16             | path pull-back for drawing                                                               |
| `markerOffsets2`                           | arrow_point 4, arrow_cross 12.5, arrow_circle 12.5                                                                                                              | LW:21-24            | neo dash only (E:880)                                                                    |
| `ARROW_MAP`                                | open→[none,none], cross→[none,arrow_cross], point→[none,arrow_point], circle→[none,arrow_circle], double\_\* both ends                                          | R:163-171           | fallback arrow types (R:2731-2733)                                                       |

### 3.2 Node positions (applyElkNodePositions, R:1509-1579)

```
applyNodes(children, relX, relY, depth):           // relX/relY = parent's absolute posX/posY; root = 0
  for n in children:
    posX = n.x + relX ; posY = n.y + relY           // absolute top-left
    nodeDb[n].offset = {posX, posY, x: relX, y: relY, depth,
                        width: max(n.width, n.labels[0].width), height: n.height}  // width/height written, never read
    nodeDb[n].x = posX + n.width/2 ; nodeDb[n].y = posY + n.height/2                // centre
    nodeDb[n].width = n.width ; nodeDb[n].height = n.height                          // ELK size
    layoutNode(n): x,y = centre
                   width  = n.isGroup ? max(n.width, labelData.width) : n.width
                   height = n.height
                   labelData, labels copied
    if n.isGroup: applyNodes(n.children, posX, posY, depth+1)
```

Note: `offset.width/height` are written at R:1524-1533 and R:1490-1491 and never read.

### 3.3 Group frames (evenGroupFrames, R:1423-1507)

```
evenGroupFrames(children):                            // called recursively BEFORE the group itself
  for g in children where g.isGroup:
    evenGroupFrames(g.children)                       // deepest first
    boxes = direct children c with c.offset && c.width && c.height
    if !g.offset or boxes empty: continue             // frame stays as ELK drew it
    lane  = internalEdgePoints(graph, descendants(g), g)
    xs = [c.posX, c.posX + c.width for c in boxes] + lane.x
    ys = [c.posY, c.posY + c.height for c in boxes] + lane.y
    origin = g.offset                                 // ELK's top-left
    left   = max(origin.posX, min(xs) - 24)
    right  = min(origin.posX + W_elk, max(xs) + 24)
    bottom = min(origin.posY + H_elk, max(ys) + 24)
    top    = origin.posY                              // title strip untouched
    floor  = labelData.width + padding                // groupTitleWidth, R:347-356
    x = left ; w = right - left
    if w < floor:
        x -= (floor - w)/2 ; w = floor
        x = max(origin.posX, min(x, origin.posX + W_elk - w))
        w = min(w, W_elk)
    h = bottom - top ; if h <= 0 or w <= 0: continue
    g.elkOrigin ??= origin                            // R:1488, used by calcOffset only
    g.offset.posX = x ; g.width = w ; g.height = h ; g.offset.width/height = w/h
    g.x = x + w/2 ; g.y = top + h/2                   // centre
    layoutNode(g): x,y = centre ; width = w ; height = h
```

Rule: the frame is only ever pulled in (R:1456-1458, R:1476-1478, R:1501-1502). The one exception is the floor branch. It can make `w` larger than the content-tight width, but still never larger than ELK's frame.

**internalEdgePoints** (R:1392-1421). Iterate ELK edges with `src = sources[0] ?? start`, `dst = targets[0] ?? end`:

```
inside = src in desc(g) and dst in desc(g)
attachStart = (src == g.id) ; attachEnd = (dst == g.id)
if !inside and !attachStart and !attachEnd: skip        // external edges do not widen the frame
off = calcOffset(src, dst)
for s in edge.sections:
    pts = inside ? [s.start, ...s.bends, s.end]
                 : [attachStart ? s.start : null, attachEnd ? s.end : null]
    add pts + off
```

Edges routed fully inside contribute their whole path to the lane. External edges contribute nothing, except a point that lands on the frame itself.

### 3.4 Sections to absolute points (R:2431-2467)

```
off = calcOffset(e.start, e.end)                    // common ancestor's ELK origin
pts = [s.start + off, ...s.bends + off, s.end + off]    // only sections[0] is read (R:1965, R:2046)
```

Unverified: whether elkjs can emit more than one section per edge for these graphs. `internalEdgePoints` reads all sections (R:1409) while routing reads only the first. The two disagree if ELK ever emits several.

### 3.5 Per-edge routine (applyElkEdgeLayout loop, R:1984-2100)

```
for e in ELK edges with a matching layoutEdge:
    reversed = e.layoutReversed === true              // R:1992
    (start, end) = (sources[0], targets[0]), swapped if reversed     // R:1993-1997
    if node missing for start or end: return           // no points and no curve set (see open questions)

    if no sections:                                    // R:2014-2039, straight line fallback
        from = centre(start) ; to = centre(end)        // from offset.posX/Y + width/2
        start.x,y = from ; end.x,y = to
        pts = sanitize([from, to], start, end)
        layoutEdge.points = pts ; layoutEdge.curve = 'linear'        // R:2027
        layoutEdge.x,y = midpoint(pts[0], pts[last])                 // R:2035-2036
        continue                                        // no marker guards, no terminal centres

    off = calcOffset(e.start, e.end)
    pts = createEdgePointsFromSection(sections[0], off)
    if reversed: pts.reverse()
    elkPorts = {start: pts[0], end: pts[last]}          // captured before centres are added (R:2052)
    start.x,y = offset-based centre ; end.x,y = centre  // R:2053-2056
    if start.shape != 'rect33': pts.unshift(centre(start))       // R:2058-2060
    if end.shape   != 'rect33': pts.push(centre(end))            // R:2062-2064
    clipped = sanitize(pts, start, end)                          // section 3.6
    layoutEdge.points = ensureStart(ensureEnd(clipped, boundsFor(end), endOff), boundsFor(start), startOff)  // R:2066-2077
    layoutEdge.curve = 'rounded'                                  // R:2078
    if label: layoutEdge.x,y = label.x+off.x+w/2, label.y+off.y+h/2       // R:2080-2084
    for t in e.labels with t.terminal and (t.x or t.y):           // R:2085-2096
        terminalLabelCenters[t.terminal] = (t.x+off.x+t.w/2, t.y+off.y+t.h/2)
    if terminalLabelCenters set: terminalLabelPorts[layoutEdge] = elkPorts
```

Note: `rect33` appears only in these guards (R:1966, R:1974, R:2058, R:2062). No file under `packages/` defines a shape with that name, so the guards are always true in this tree.

### 3.6 Endpoint clipping (sanitizeElkEdgePoints, R:2469-2551)

```
sanitize(pts, sNode, eNode):
    sB = boundsFor(sNode) ; eB = boundsFor(eNode)
    sCand = pts[0] unless isCenter(pts[0]) then pts[1]          // getCandidateBorderPoint, R:2792-2811
    eCand = pts[last] unless isCenter(pts[last]) then pts[len-2]
    sSkip = sNode.isGroup and onBorder(sB, sCand, 0.5)           // R:2504
    eSkip = eNode.isGroup and onBorder(eB, eCand, 0.5)           // R:2505
    if sSkip and isCenter(pts[0]):   drop pts[0]                  // R:2507 (dropAutoCenterPoint)
    if eSkip and isCenter(pts[last]): drop pts[last]              // R:2508
    if sNode.isGroup and !sSkip: sSkip = clipGroupEndpoint(pts, sB, start)   // R:2513-2515
    if eNode.isGroup and !eSkip: eSkip = clipGroupEndpoint(pts, eB, end)     // R:2516-2518
    if sSkip or eSkip:                                            // R:2520-2536
        if !sSkip: applyStartIntersectionIfNeeded(pts, sNode, sB) // centre-ray intersection
        if !eSkip: applyEndIntersectionIfNeeded(pts, eNode, eB)
        // cutter2 is NOT run
    else:
        pts = cutter2(sNode, eNode, pts)                          // R:2538
    if result invalid (len < 2 or NaN): fall back to the finite points (R:2543-2547)
    return dedupeConsecutive(pts)                                  // R:2550
```

**clipGroupEndpoint** (R:2828-2855). The frame is the group's `boundsFor` box.

```
step = +1 (start) or -1 (end)
i = start index (0 or last) ; walk while inside(B, pts[i]) : i += step   // inside = strictly inside
if i == start index or i out of range: return false       // first point already outside, or never leaves
out = pts[i] ; in = pts[i - step]
dx = out.x - in.x ; dy = out.y - in.y
tx = dx == 0 ? inf : (B.x + sign(dx)*B.w/2 - in.x)/dx
ty = dy == 0 ? inf : (B.y + sign(dy)*B.h/2 - in.y)/dy
cross = in + min(tx,ty) * (dx,dy)
start: splice(0, i, cross)            // removes the interior run, keeps out
end:   splice(i+1, n-i-1, cross)
return true
```

`outsideNode` (G:98-106) counts a point **on** the border as outside.

**cutter2** (R:2938-3021). Runs only when neither endpoint is a group clip.

```
sB, eB = boundsFor(...)
i = first index with outsideNode(sB, pts[i])                  // R:2958-2964
if i found:
    v = attachAlongDepartureAxis(sNode, sB, pts, i, +1) ?? computeNodeIntersection(sNode, sB, pts[i], pts[0])
    replaceEndpoint(pts, start, v)                           // overwrites pts[0] (the centre)
j = last index with outsideNode(eB, pts[j])                   // R:2981-2987
if none and len > 1: j = len-2 , no test                      // R:2989-2992
if j found:
    v = attachAlongDepartureAxis(eNode, eB, pts, j, -1) ?? computeNodeIntersection(eNode, eB, pts[j], pts[last])
    replaceEndpoint(pts, end, v)
if len > 1 and |pts[last] - pts[last-1]| < 2: pop last        // R:3002-3016
```

**replaceEndpoint** (G:286-320). If the new value is within 0.1 of the existing endpoint, the existing endpoint is **removed** (shift or pop) and the new value is discarded. Otherwise the existing endpoint is overwritten. See open question Q6.

### 3.7 Attach along the departure axis (G:224-274, R:2920-2936)

```
attachAlongDepartureAxis(node, B, pts, p, step):
    if node.isGroup: return null                              // R:2927-2929
    port = pts[p] ; next = pts[p + step]                      // step +1 at start, -1 at end
    if missing: return null
    return outlineAttachPoint(node, B, port, next)

outlineAttachPoint(node, B, port, next):
    if no node.intersect: return null                          // G:230
    d = next - port ; if d == 0: return null                  // G:236
    if |dx| > 1e-6 and |dy| > 1e-6: return null               // diagonal departure, G:245-247
    centre = (B.x, B.y) ; horizontal = |dx| > |dy|
    along(t) = horizontal ? (t, port.y) : (port.x, t)
    inner = centre on the free axis ; outer = port on the free axis
    if !insideOutline(along(inner)): return null               // G:257-259
    if insideOutline(along(outer)): return port                // port already on/in outline, G:260-263
    repeat 20: mid = (inner+outer)/2 ; inner = mid if insideOutline(along(mid)) else outer = mid
    return along(inner)

insideOutline(node, c, probe):
    x = node.intersect(probe) ; if null: false
    return |probe - c| <= |x - c| + 1e-9                        // G:193-201
```

For a rectangle the port is already on the outline, so `outlineAttachPoint` returns the port. For a circle or polygon the attachment slides along the departure axis to the real outline. This is the only place the departure direction is used.

**computeNodeIntersection** (G:276-284):

```
outside' = ensureTrulyOutside(B, outside)        // if |dx-w/2|<1 or |dy-h/2|<1, push radially out by 10 (G:108-125)
return tryNodeIntersect(node, B, outside') ?? fallbackIntersection(B, outside', centre)
tryNodeIntersect: node.intersect(outside'); null if: null result, x on the wrong side of centre, or dist <= 1 (G:140-158)
fallbackIntersection: makeInsidePoint (G:127-138) + rectangle formula `intersection` (G:46-96)
```

### 3.8 Marker segment guards (R:2588-2648)

`markerPathOffset(type) = max(markerOffsets[type] ?? 0, markerOffsets2[type] ?? 0)` (R:143-151).

```
ensureEnd(P, B, off):                       // off from the END marker
    if off <= 0 or len(P) < 3: return P
    seg = |P[last] - P[last-1]|
    if seg >= max(8, 2*off): return P
    if !onBorder(B, P[last-1], tol=1): return P                 // R:2636
    return [...P[0..last-2], P[last]]                           // drops the short stub's entry point (R:2640)
ensureStart(P, B, off):                     // mirror image at the start
    seg = |P[1] - P[0]| ; if seg >= max(8, 2*off) or !onBorder(B, P[1], 1): return P
    return [P[0], ...P[2..]]                                    // R:2609
```

Offset values this produces: arrow_point 4 gives a threshold of 8. arrow_cross and arrow_circle give 12.5, so the threshold is 25. arrow_barb_neo 5.5 gives 11. aggregation, extension and composition 17.25 give 34.5. The threshold for arrow_cross and arrow_circle is set by a pull-back that drawing never applies. See D1.

### 3.9 Degenerate node alignment (pre-pass R:1951-1982, body R:2387-2429)

Runs for every ELK edge with sections, for both ends, unless `shape == 'rect33'`. The anchor is ELK's port (section start or end plus offset).

```
alignDegenerate(node, anchor):
    top = posY ; bottom = top + h
    alongWidth = |anchor.y - top| <= 0.5 or |anchor.y - bottom| <= 0.5       // R:2402
    side = alongWidth ? w : h
    if side >= 2*12: return                                                   // R:2403-2405
    if node already aligned (first anchor wins): return                       // R:2406-2409
    delta = alongWidth ? anchor.x - (posX + w/2) : anchor.y - (posY + h/2)
    if |delta| < 0.01: return
    move posX (or posY) by delta ; update node.x/y and the layout node      // R:2418-2427
```

The comment at R:2383-2384 says the shift is "at most half the side". The code has no clamp on `delta`. See D3. There is also no `isGroup` guard. See U8.

### 3.10 Straightening (R:1594-1932)

**straightenFront(pts)** (R:1696-1749). Applied at one end. Front is tried first, then the reversed array (R:1666-1677).

```
if len < 5: null
axis = dir(p0->p1) in {h, v} ; require dir(p2->p3) == axis ; require dir(p1->p2) == perpendicular   // R:1700-1704
require hypot(p1 - p0) <= 30                                     // R:1706: the step sits at the node
jog = |p2 - p1| on the perpendicular axis ; require 0.01 <= jog <= 16          // R:1709-1712
require sign(p1 - p0) == sign(p3 - p2) on the axis               // R:1715-1721: route keeps moving forward
last = 3 ; while last+1 < len and dir(p[last]->p[last+1]) == axis: last++     // R:1725-1728
require last != len - 1                                          // far terminal not inside the run, R:1731
moved[i] = (h ? (pts[i].x, p0.y) : (p0.x, pts[i].y)) for i in 2..last       // channel pulled to the port's row
result = moved with indices 1 and 2 removed                       // R:1747
run = {old: {a: pts[2], b: pts[last]}, new: {a: moved[2], b: moved[last]}}
```

The port (p0) is never moved. The channel moves onto the port's row.

**straightenEdgeTerminals(edges)** (R:1874-1929). Runs once over all edges, with edges processed in order:

```
for each edge with len(points) >= 5:
    cand, runs = straightenTerminalJogs(points)      // front, then back
    if cand is unchanged: continue
    lbl = label centre ; candLbl = lbl with projectLabelOntoStraightenedRun(lbl, runs)
    before = after = coveredBefore = coveredAfter = 0
    for each other edge route:
        before += crossingCount(orig, route) ; after += crossingCount(cand, route)        // strict interior crossings, R:1752-1772
        coveredBefore += covers(route, lbl) + covers(orig, otherLbl)
        coveredAfter  += covers(route, candLbl) + covers(cand, otherLbl)                // segment-hits-label-box, R:1826-1848
    if after > before or coveredAfter > coveredBefore: reject (keep ELK route)           // R:1919-1921
    else: commit points and update routes/labels (later edges see this change)
```

Box for coverage: `centre ± size/2`, no clearance (R:1850-1855). Crossing counts are net over all other routes, and the decision is made per edge in list order.

**projectLabelOntoStraightenedRun** (R:1803-1816). For each run: `t = paramOnSegment(label, old.a, old.b)` (clamped to [0,1]). Pick the run whose `old` point is nearest the label. Return `pointAtParam(new.a, new.b, t)`.

Applied only to the main label, only when `straightenEdges` is on (R:2102-2118).

### 3.11 Label passes (R:2119-2125, helpers R:2135-2338)

**followMovedEndpoints** (R:2345-2364): `delta_start = pts[0] - elkPorts.start`, `delta_end = pts[last] - elkPorts.end`. Each terminal centre moves by the delta of its own end (keys starting with `start` use delta_start).

**blocked(box, edge, key, segment, clearance)** (R:2221-2242). True if the box overlaps any of:

- a non-group node box (`nodeBoxes`, R:2183);
- a main label box (`centreLabelBoxes`, R:2206-2210);
- a group frame, with `clearance` added on each side (`hitsFrame`, R:2212-2220);
- any segment of any edge's `points`, excluding this edge's own `segment` (R:2226-2231);
- any other terminal label box (R:2233-2242).

The group frame is a hollow outline, so frames contribute four segments each (R:2185-2205).

**slideTerminalLabelsOffFrames** (R:2275-2312). Only for terminal labels whose box hits a frame segment with no clearance (R:2288).

```
far, end = atStart ? (to, from) : (from, to)
dir = unit(end - far) ; halfExtent = (|dir.x|*w + |dir.y|*h)/2
along(c) = (c - end) . dir
onSegment(c) = along(c) + halfExtent <= 0 and along(c) - halfExtent >= -length
for step in 1..length:
    for shift in [+step, -step]:
        c' = center + shift*dir
        if onSegment(c') and !blocked(box(c'), edge, key, segment, clearance=2): set center = c' ; stop
```

If no position is found, the label stays where it was.

**putTerminalLabelsOnTheirSide** (R:2322-2338). The label is mirrored across its end segment when it sits on the wrong side.

```
normal = (-(to.y - from.y), to.x - from.x) / length        // right of travel in y-down coords
offset = (center - from) . normal
if offset == 0 or (offset > 0) == key.endsWith('Right'): keep       // R:2330
mirrored = center - 2*offset*normal
if !blocked(box(mirrored), edge, key, segment, clearance=0): set center = mirrored
```

**separateOppositeEdgeLabels** (R:2135-2170). Only for main labels.

```
bucket edges with a label, by unordered {start,end}            // R:2141: direction is NOT checked
for each bucket with exactly 2 edges (a,b):
    if |a.y - b.y| > max(a.h, b.h): continue                    // different heights
    required = a.w/2 + b.w/2 + 4
    deficit = required - |b.x - a.x|
    if deficit > 0: a.x -= sign*deficit/2 ; b.x += sign*deficit/2   // sign = sign(b.x - a.x) or 1
```

### 3.12 Edge render data before layout (buildEdgeData, R:2715-2767)

```
edgeData.curve      = interpolateToCurve(edge.curve ?? edge.interpolate, defaultInterpolate ?? conf.curve)   // R:2745-2750, U:230-241
edgeData.arrowTypeStart = edge.arrowTypeStart ?? ARROW_MAP[type][0]    // R:2732
edgeData.arrowTypeEnd   = edge.arrowTypeEnd   ?? ARROW_MAP[type][1]    // R:2733
edgeData.classes    = edge.classes ?? `flowchart-link LS_${start} LE_${end}`          // R:2670-2678
edgeData.style, thickness, pattern, labelStyle = from computeStroke (R:2680-2703)     // stroke dotted/thick/default
edgeData.arrowhead  = edge.arrowhead ?? (type == arrow_open ? 'none' : 'normal')
edgeData.arrowheadStyle = edge.arrowheadStyle ?? ('fill: #333' if text or style)
edgeData.labelpos   = edge.labelpos ?? ('c' if text)
edgeData.label      = (edge.label ?? text).replace(lineBreakRegex, '\n')
```

The flowchart DB sets `arrowTypeStart/End` (FDB:1326-1330, from `destructEdgeType` FDB:1028-1043: start defaults to `none`, end to `arrow_point`, and `arrow_open` gives `none`) and `curve` (FDB:1340). Whatever `curve` ends up on an edge is overwritten after layout (see 5, section on curve).

---

## 4. Paint and afterPaint

### 4.1 Generic render order (C:129-204)

```
insertMarkers(element, markers)                    // C:141-146
clearLayoutRenderState()                           // C:147
preparedLayout = prepareLayout()                   // C:159-161  = prepareLayoutForElk
measure        = measureLayout()                   // C:166-168  = createGraphWithElements(unwrapGroupLabels)
coreResult     = runLayoutCore()                   // C:173-175  = runElkLayoutCore (R:687-702)
paintLayoutData(data4Layout, paintContext, {skipIntersect: true})   // C:185-196
afterPaint(data4Layout, paintContext, coreResult)  // C:199      = applyElkLineJumps
```

### 4.2 Nodes (C:247-260, N:77-97, CL:15-143)

```
for node in data4Layout.nodes (already sorted groups-first):      // C:229-233
    if clusterNode: positionNode(node)
    elif node.isGroup: insertCluster(node)                         // C:255-256, CL:841-845 -> shape 'rect' by default
    else: positionNode(node)                                       // translate(node.x, node.y), N:97
```

Group frame painter (`rect`, CL:15-143):

```
bboxW = label width measured as painted (CL:37-62)
width = node.width <= bboxW + node.padding ? bboxW + node.padding : node.width       // CL:63
x = node.x - width/2 ; y = node.y - node.height/2                                   // CL:71-72
label: translate(node.x - bboxW/2, node.y - node.height/2 + subGraphTitleTopMargin)  // CL:108-113
node.width = rectBox.width ; node.height = rectBox.height     // CL:125-126: mutates the LAYOUT node after paint
node.intersect = intersectRect(node, point)                   // CL:130-132
```

Group padding comes from FDB:1245 and FDB:1263 (`padding: 8`, hard-coded for subgraphs).

Edges are painted only after every node (C:238-243). The edge loop skips `isLayoutOnly` edges (C:280-282), which the flowchart path never sets.

### 4.3 Edge paths (insertEdge, E:624 onwards, with skipIntersect=true from C:292-302)

```
points = edge.points                                        // already clipped and marker-guarded
// E:658  swimlane branch: not taken (layout is not swimlane)
// E:699  else-if (head.intersect && tail.intersect && !skipIntersect): NOT taken
// E:706  toCluster / E:713 fromCluster: edge.toCluster / fromCluster are never set in packages/mermaid/src
lineData = points without NaN y                              // E:724
if curve != 'rounded': lineData = fixCorners(lineData)       // E:729-731 (not taken for ELK routed edges)
curveType = resolveEdgeCurveType(edge.curve)                 // E:43-45: string -> itself, else config flowchart.curve
if curveType == 'rounded':
    d = generateRoundedPath(applyMarkerOffsetsToPoints(lineData, edge), 5)   // E:808-810, E:968-1038, E:1053-1085
else:
    d = lineFunction(lineData) with x/y accessors from getLineFunctionsWithOffset   // E:777, LW:56-...
data-points = base64(JSON(points))                            // E:705, E:899 (before marker offsets)
paths.originalPath = edge.points ; paths.updatedPath only if the midpoint is off the path (E:947-958)
```

For ELK edges `points === edge.points`, so `updatedPath` and `originalPath` are identical. The main-label delta is therefore zero (see 4.4). The path-building code is in E:808-811.

**generateRoundedPath** (E:968-1038). For each interior vertex with incoming `v1` and outgoing `v2`:

```
cut = min(r / sin(theta/2), len1/2, len2/2)       // theta = angle between v1 and v2 ; r = 5
emit L(vertex - n1*cut) then Q(vertex) (vertex + n2*cut)
skip rounding (plain L) if len < 1e-5 or theta < 1e-5 or |pi - theta| < 1e-5
```

LJ `computeRoundedCorner` (LJ:379-414) is a copy of this formula.

**Marker pull-back.** `applyMarkerOffsetsToPoints` (E:1053-1085) and the line accessors in `getLineFunctionsWithOffset` (LW:56-...) use only the `markerOffsets` table. `arrow_cross` and `arrow_circle` are not in it, so these markers get no pull-back. `markerOffsets2` is used only for the neo dash array (E:880).

### 4.4 Labels (C:328-377, E:284-298)

```
positionRenderedEdgeLabel(edge, paths):                 // C:328-351
    if edge.label:
        (x, y) = resolveEdgeLabelPosition(edge, paths)   // E:284-298: edge.x/y + (updatedMid - originalMid)
        el.transform = translate(x, y + (subGraphTitleTop + subGraphTitleBottom)/2)     // C:336-337
    for key in [startLeft, startRight, endLeft, endRight] with text:
        (x, y) = terminalLabelTranslate(edge, key, path)           // C:364-378
        terminalLabels[key].transform = translate(x, y)            // C:348
terminalLabelTranslate:
    if terminalLabelCenters[key]: return it                        // ELK placed it
    else calcTerminalLabelPosition(arrow ? 10 : 0, side, path)     // U:378 fallback along the path
```

For ELK the main-label delta is zero, so the label lands at `edge.x, edge.y`. The `(top+bottom)/2` margin term is zero by default.

Labels are measured centred. `insertEdgeLabel` puts `translate(-w/2,-h/2)` on the inner label group (E:130, and E:160, 187, 215, 243 for terminals). `translate(x,y)` on the outer group therefore centres the label on the point.

### 4.5 lineHops and lineJump (L:39-66, LJ:646-759)

```
applyElkLineJumps(data4Layout, paint):
    if config.elk.lineHops === false: return                       // L:43-46
    geoms = [{id, points, curve, arrowTypeStart, arrowTypeEnd}] for edges with >= 2 points   // L:48-59
    applyLineJumpsToSvg(edgePaths, geoms, {enabled: true, jumpRadius: 6,
                        jumpStyle: lineHops === 'gap' ? 'gap' : 'arc'})  // L:61-65

applyLineJumpsToSvg(group, edges, cfg):                             // LJ:646-759
    for each edge: pts = decode(path[data-id].data-points) or edge.points   // data-points written at E:899
    crossings = findEdgeIntersections(all rendered edges)              // includes edges that will not be rewritten
    for each edge with crossings:
        if curve defined and !curveSupportsLineHops(curve): skip        // LJ:712-714
        if curve undefined and d is not M/L-only: skip                  // LJ:721-724
        d = rewriteEdgePath(edge, crossings, cfg) ; set path d          // LJ:742
```

`curveSupportsLineHops` (LJ:597-611) accepts `linear`, `rounded`, `step`, `stepBefore`, `stepAfter`, or undefined. Other curves are skipped.

**findEdgeIntersections** (LJ:205-264):

```
for each pair of edges (i < j), for each segment pair:
    hit = segmentIntersection(...)                      // LJ:118-157: parallel -> none ; t in (1e-6, 1-1e-6) on both -> hit
    if hit lies in a rounded corner cut of either edge: skip        // LJ:179-203, LJ:225-229
    orthogonal = (one segment is horizontal-dominant, the other vertical-dominant)   // LJ:238
    if orthogonal: the HORIZONTAL segment's edge jumps               // LJ:239
    else:          the LATER edge in the list (j) jumps
```

**rewriteEdgePath** (LJ:416-534):

```
pts = pts with arrow pull-back applied (LJ:310-339, markerOffsets table)
rounded = edge.curve == 'rounded'                                   // LJ:424
for each segment i:
    consumed = corner.cutLen at the start of the segment if rounded and i > 0       // LJ:470
    stop     = segLen - cutLen of the upcoming corner if rounded                    // LJ:485
    for each hop on the segment, sorted by t:
        room = min(d - consumed, stop - d) - 2                                       // LJ:497
        r    = min(6, room) ; drop if r < 0.6*6 = 3.6                                 // LJ:493, LJ:515
    adjacency: gap = d[k+1] - d[k] ; if r[k] + r[k+1] > gap: both r = min(r, gap/2)
    drop again if r < 3.6
    emit: L(pre) then arc:  A r r 0 0 sweep (post)   |  gap: M(post)                // LJ:341-358
          pre  = hit - u*r ; post = hit + u*r                                       // u = unit direction of segment
          sweep: horizontal segment dx>=0 ? 1 : 0 ; vertical dy>=0 ? 1 : 0         // LJ:281-294
    end of segment: rounded and upcoming corner -> L(startX,startY) Q(ctrl) (endX,endY)
                    otherwise -> L(seg.b)
```

Only `processEdgesWithJumps` (LJ:547-584) is a pure variant. It is used only by `lineJump.spec.ts`, so it is not on the ELK path.

---

## 5. Config switches

| Key                                                                     | Default (schema)          | Read at                                                                                     | Effect in this code                                                                                                                                                                                         |
| ----------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------------- |
| `elk.straightenEdges`                                                   | `true` (SCH:159-176)      | R:1939, `!== false`                                                                         | Gates only the straightening block (R:2102-2118). `followMovedEndpoints`, the slide, mirror and separate passes always run.                                                                                 |
| `elk.lineHops`                                                          | `true` (SCH:177-190)      | L:43-46 (`=== false` disables), L:64 (`'gap'` gives gap style, anything else arc)           | Gates the afterPaint pass.                                                                                                                                                                                  |
| `elk.orientFeedbackEdges`                                               | `true` (SCH:255-263)      | R:723, `!== false`                                                                          | Enables `reverseSubgraphFeedbackEdges` (R:736-747), which sets `layoutReversed` and changes which end is the ELK source (R:1992-1997).                                                                      |
| `elk.*` preset, placement, spacing, mergeEdges, cycleBreaking, layering | ELK input only            | R:369-530, R:905-990                                                                        | Not part of post-processing.                                                                                                                                                                                |
| `flowchart.curve`                                                       | `'basis'` (SCH:2501-2521) | FDB:1340 sets `edge.curve`; R:2745 converts it in prepare; overwritten at R:2027 and R:2078 | **Routed ELK edges always end as `'rounded'`**, so the flowchart curve setting does not reach them. Only the straight-line fallback uses `'linear'`. Derived from reading; no spec asserts it (unverified). |
| per-edge `interpolate` / `defaultInterpolate`                           | —                         | FDB:1340                                                                                    | Same as `flowchart.curve`: no effect on routed ELK edges.                                                                                                                                                   |
| `flowchart.subGraphTitleMargin.top / bottom`                            | 0 / 0 (SCH:2464-2477)     | CL:108-112 (label y), C:331-337 (edge label y)                                              | Paint only. Not reserved in layout (no read under `elk/`).                                                                                                                                                  |
| `flowchart.padding`                                                     | 15 (schema)               | FDB:1071 (`                                                                                 |                                                                                                                                                                                                             | 8`, vertices only) | Subgraphs use the hard-coded 8 (FDB:1245, FDB:1263). |

Lines `R:1939` and `L:43` read the merged config directly. I did not check how the schema defaults are materialised at runtime. The code treats `undefined` as the default, which matches the schema.

---

## 6. Node intersections

Edge endpoints are clipped in two places: `sanitize` and `cutter2` (section 3.6), and `computeNodeIntersection` (G:276-284). Paint never intersects, because `skipIntersect: true` (R:760-762, C:300, E:699). So `node.intersect` is only used during layout.

| Node kind                                                                                                                                  | Where `node.intersect` is set                                     | Function                                                         | Notes                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Rectangles (roundedRect, squareRect, rectWithTitle, note, cylinder, forkJoin, most "rect-like" shapes)                                     | at measure, via `drawRect` (shapes/drawRect.ts:78-79) and similar | `intersect.rect` = `intersectRect` (intersect/intersect-rect.js) | Ray from centre through the query point, against the box.                                                                                 |
| Circles (circle, doubleCircle, filledCircle, stateStart, stateEnd, anchor, crossedCircle)                                                  | at measure                                                        | `intersect.circle`                                               |                                                                                                                                           |
| Ellipses (usecaseEllipse, usecaseBusiness)                                                                                                 | at measure                                                        | `intersect.ellipse`                                              |                                                                                                                                           |
| Polygon-like shapes (hexagon, trapezoid, triangle, stadium, hourglass, subroutine, card, question, choice, folder, bowTieRect, and others) | at measure                                                        | `intersect.polygon`                                              |                                                                                                                                           |
| Groups (subgraph `rect`)                                                                                                                   | **paint only** (CL:130-132)                                       | `intersectRect`                                                  | At layout time, group `intersect` is undefined, so every group clip falls back to `clipGroupEndpoint` and `fallbackIntersection`. See U9. |

The `intersect` closures read the **original** data4Layout node's `x, y, width, height`. These stay in sync with the nodeDb copy because `applyElkNodePositions` (R:1542-1547) and `alignDegenerateNodeToAnchor` (R:2424-2427) update both.

**When is a group edge clipped at the frame instead?** Two cases, both in `sanitize` (R:2504-2518):

1. The candidate endpoint (first ELK point after the centre) is on the frame border (`onBorder`, tolerance 0.5). The frame is then taken as the endpoint, and the auto-centre point is dropped.
2. Otherwise `clipGroupEndpoint` walks the polyline from the endpoint until it leaves the frame, and replaces everything inside with the crossing point.

In both cases `cutter2` is skipped for the whole edge. The non-group end, if there is one, then uses `applyStartIntersectionIfNeeded` or `applyEndIntersectionIfNeeded`, which is a centre-ray intersection (G:276-284), not `attachAlongDepartureAxis`. So an edge with one group endpoint never gets the departure-axis attachment on its other end.

---

## 7. What each Edge object carries into edges.js

| Field                                                                                                                    | Set by                                                                                         | Meaning for drawing                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `points`                                                                                                                 | R:2046-2077 (routed) or R:2014-2026 (fallback)                                                 | Absolute polyline. First and last point are on the border (or frame). No marker pull-back applied. Drawn with `skipIntersect`.                                                                                                   |
| `curve`                                                                                                                  | R:2078 `'rounded'` (routed), R:2027 `'linear'` (fallback). Otherwise the function from R:2745  | `resolveEdgeCurveType` (E:43-45) turns it into a string.                                                                                                                                                                         |
| `x`, `y`                                                                                                                 | R:2082-2083 (main label centre). Also set for unlabelled edges (the label entry always exists) | Used by `resolveEdgeLabelPosition` (E:284-298). Moved by the straightening projection (R:2112-2116) and by `separateOppositeEdgeLabels` (R:2166-2168, x only). `followMovedEndpoints` moves only terminal centres (R:2345-2364). |
| `width`, `height`                                                                                                        | E:136-137 (measure, unpadded)                                                                  | Label box size. Used by label obstacles and separation.                                                                                                                                                                          |
| `terminalLabelCenters`                                                                                                   | R:2085-2096, then followMovedEndpoints, slide and mirror                                       | Centre per key. Missing key means fall back to `calcTerminalLabelPosition`.                                                                                                                                                      |
| `terminalLabelSizes`                                                                                                     | E:263-265 (measure, unpadded)                                                                  | Used by obstacles and the slide.                                                                                                                                                                                                 |
| `arrowTypeStart`, `arrowTypeEnd`                                                                                         | FDB:1326-1330, or ARROW_MAP (R:2732-2733)                                                      | Marker type. Also used by ELK label padding (R:1162-1163) and marker guards (R:2575-2581).                                                                                                                                       |
| `classes`                                                                                                                | edge.classes, or `flowchart-link LS_x LE_y` (R:2670-2678)                                      | Class attribute on the path.                                                                                                                                                                                                     |
| `style`, `labelStyle`, `thickness`, `pattern`, `arrowhead`, `arrowheadStyle`, `labelpos`, `label`, `labelType`, `minlen` | R:2715-2767                                                                                    | As above.                                                                                                                                                                                                                        |
| `layoutReversed`                                                                                                         | R:744 (set on the ELK copy, not on the data4Layout edge)                                       | Used only in render.                                                                                                                                                                                                             |

Edges are passed to `insertEdge` as `{...edge}` (C:292-296). Anything `applyElkEdgeLayout` writes onto the data4Layout edge is visible to paint.

---

## 8. Discrepancies between comments or schema and code

| ID  | Where                            | Claim                                                                                              | What the code does                                                                                                                                                                                                                                              |
| --- | -------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | R:136-141                        | `markerPathOffset` reads "the same tables the edge drawing uses", so a new marker cannot be missed | Drawing pulls back only with `markerOffsets` (LW:6-16). `arrow_cross` is commented out there. `markerOffsets2` drives only the neo dash (E:880). The guard therefore uses 12.5 for `arrow_cross` and `arrow_circle`, but drawing applies no pull-back for them. |
| D2  | L:33-37; SCH `elk.lineHops` text | An edge that takes a hop "loses its corner rounding for the segment carrying the hop"              | `rewriteEdgePath` keeps the `Q` corners for `rounded` edges (LJ:424, and the corner emission at the end of each segment). Only the hop itself is straight.                                                                                                      |
| D3  | R:2383-2384                      | Shift "is at most half the side"                                                                   | No clamp on `delta` in R:2411-2423.                                                                                                                                                                                                                             |
| D4  | R:1966, 1974, 2058, 2062         | `rect33` guards                                                                                    | No shape named `rect33` exists under `packages/`. Dead guards.                                                                                                                                                                                                  |
| D5  | R:1340-1362                      | The frame JSDoc sits above `collectDescendantIds`, which is R:1363                                 | `evenGroupFrames` (R:1423) has no doc. The doc text belongs to `evenGroupFrames`.                                                                                                                                                                               |
| D6  | R:2029-2030                      | "`positionEdgeLabel` reads `edge.x` / `edge.y`"                                                    | The ELK path uses `resolveEdgeLabelPosition` through C:328-337. `positionEdgeLabel` is used only by cose-bilkent and re-exported (internals.ts).                                                                                                                |
| D7  | SCH `straightenEdges`            | An edge is left alone if moving its run "would drag the far port, or would introduce a crossing"   | Code also rejects when a label would be covered (R:1915-1921, comment R:1866).                                                                                                                                                                                  |
| D8  | R:2131-2134                      | `separateOppositeEdgeLabels` separates "opposite-direction" edges                                  | Buckets by unordered node pair and never checks direction (R:2141). Same-direction parallel labelled edges are also pushed apart.                                                                                                                               |
| D9  | R:1490-1491, R:1524-1533         | `offset.width` and `offset.height` carry the frame and node sizes                                  | Written, never read.                                                                                                                                                                                                                                            |
| D10 | R:2773-2777 `boundsFor`          | Box centre uses `node.width`, box width uses `max(width, title floor)`                             | They agree only when the frame is already at least as wide as the title floor.                                                                                                                                                                                  |
| D11 | CG:161-171 comment               | Markdown group labels are measured wrapped and painted at `node.width`                             | Not reconciled in layout. Possible mismatch with the title floor. Not run.                                                                                                                                                                                      |

---

## 9. Unverified and open questions

- **Q1.** R:2088 relies on elkjs keeping the extra `terminal` field on label objects. Not checked against elkjs source here.
- **Q2.** Only `sections[0]` is read for routing (R:1965, R:2046), while `internalEdgePoints` reads all sections (R:1409). Assumes layered output emits one section per edge. Unverified.
- **Q3.** The section coordinate convention (relative to the common ancestor's ELK origin, FCA + R:2447-2467) is inferred from this code. The Rust crate `crates/ferrigura-elk` was not read, so whether it applies the same convention is unverified.
- **Q4.** `toCluster` and `fromCluster` (E:706-722) have no producer under `packages/mermaid/src`. Other packages were not searched. The `cutPathAtIntersect` branch is treated as dead for flowcharts.
- **Q5.** Whether the flowchart `curve` setting is meant to be ignored for routed ELK edges. The code is clear. The LJ comments (LJ:33-37 in the L file) describe `rounded` as ELK's curve, which suggests intent, but no spec asserts it.
- **Q6.** `replaceEndpoint` (G:297-306, G:308-318). When the new endpoint is within 0.1 of the old one, the old endpoint is removed and the new value is discarded. The path then starts at the next point, which can be far from the border. Reachability from the ELK flow is unverified.
- **Q7.** `cutter2` replaces only `pts[0]` and `pts[last]` (R:2975, R:2999). If the first outside point has index greater than 1, inside points stay in the path. Unverified whether ELK produces that.
- **U8.** `alignDegenerateNodeToAnchor` has no `isGroup` guard (R:2387-2429), so a small group endpoint would move its frame away from its children. Unverified whether a group can be under 24px on the anchored side.
- **U9.** Group `intersect` is undefined during layout (CL:130 runs only at paint). A second render of the same data object would have it defined. Unverified in practice. Matters for any port that assumes group clips never use a shape intersection.
- **U10.** Paint recomputes frame width (CL:63) and writes `node.width` back (CL:125). If label plus padding exceeds the layout width, the painted frame grows past the frame ELK sized. Unverified whether this happens in practice.
- **U11.** If a start or end node is missing, the loop returns before setting `points` or `curve` (R:2000-2002). `insertEdge` then receives an edge with no points. Reachability unverified.
- **U12.** The terminal-centre test `(!x && !y)` (R:2088) treats an exact (0,0) ELK position as unplaced.
- **U13.** Nothing above was executed. Visual results (hops, straightening, label placement) are unverified until rendered.
