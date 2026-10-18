//! The geometry of the direction passes: `direction/geometry.ts` of Mermaid 12.2.0, ported function by function.
//!
//! The tolerances, the orthogonal segments and their bends, the rectangles and the node bounds, the port
//! paths, and the polyline cleanup (`orthogonalizePolyline`, `simplifyPolyline`, and the edge loop of
//! `postProcessing.ts` that runs them, [`orthogonalize_and_simplify_edges`]).
//!
//! Conventions:
//!
//! - Where the TypeScript has a default epsilon, the port takes the epsilon as a parameter, and the caller
//!   passes the value the TypeScript caller passes. No default is hidden.
//! - `Math.min` and `Math.max` are [`js_min`] and [`js_max`]: they return NaN when an operand is NaN, and they
//!   order `-0` before `+0`.
//! - The `Node`, `Edge` and `Rect` of the model stand for the TypeScript objects: a missing `x`, `y`, `width`
//!   or `height` is `0` where the TypeScript reads `?? 0`.
//!
//! The module allows dead code while the direction files that call these functions are not ported yet
//! (`materializedGeometry.ts`, `portSwap.ts`, `terminalStub.ts`, `siblingSharedFaceRouting.ts`,
//! `sharedTrackNudging.ts`, `validation.ts`). Remove the attribute as they land.

#![allow(dead_code)]

use indexmap::IndexMap;

use crate::math::{js_max, js_min};
use crate::model::{Edge, LayoutData, Node, Point, Rect};

/// `EPS` of `geometry.ts`, and of the other direction files: the tolerance of the orthogonal tests, in layout
/// units.
pub(crate) const EPS: f64 = 1e-3;

/// `RectSide`: the side of a rectangle that a port is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RectSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// `RectEntry`: a node id with its rectangle.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RectEntry {
    pub id: String,
    pub rect: Rect,
}

/// `NodeBoundsInfo`: a measured node, with its centre.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeBoundsInfo {
    pub id: String,
    pub cx: f64,
    pub cy: f64,
    pub rect: Rect,
}

/// `NodePairGeometry`: the two measured ends of an edge, and whether their centres are aligned.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodePairGeometry {
    pub src_id: String,
    pub dst_id: String,
    pub src_info: NodeBoundsInfo,
    pub dst_info: NodeBoundsInfo,
    pub collinear_x: bool,
    pub collinear_y: bool,
}

/// `LayoutNodeRect`: the rectangle of a node of the layout, from its centre and size.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutNodeRect {
    pub node_id: String,
    pub rect: Rect,
}

/// The kind of a three-segment route: `HVH` starts and ends horizontal, `VHV` vertical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteKind {
    Hvh,
    Vhv,
}

/// `ThreeSegmentRoute`: four points that make three orthogonal segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ThreeSegmentRoute {
    pub kind: RouteKind,
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

/// `OrthogonalSegment`: a horizontal or vertical segment of a polyline, with the index of its first point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OrthogonalSegment {
    pub index: usize,
    pub a: Point,
    pub b: Point,
    pub horizontal: bool,
    pub vertical: bool,
}

/// `Math.sign` on a number: `1`, `-1`, or the argument itself for a zero or a NaN, so that `-0` stays `-0`.
pub(crate) fn js_sign_value(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else if x > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// A JavaScript truthy text: a string that is not empty. `None` for an absent or an empty string.
pub(crate) fn truthy_text(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.is_empty())
}

/// `measuredNodeRect(node)`: the centre and the rectangle of a node with a positive size, or `None`.
fn measured_node_rect(node: &Node) -> Option<(f64, f64, Rect)> {
    let cx = node.x.unwrap_or(0.0);
    let cy = node.y.unwrap_or(0.0);
    let width = node.width.unwrap_or(0.0);
    let height = node.height.unwrap_or(0.0);
    if width > 0.0 && height > 0.0 {
        Some((cx, cy, rect_from_center_size(cx, cy, width, height)))
    } else {
        None
    }
}

/// `nodeBoundsInfoFor(node)`: the bounds of a node that is not a group and has a positive size.
fn node_bounds_info_for(node: &Node) -> Option<NodeBoundsInfo> {
    if node.is_group == Some(true) {
        return None;
    }
    let (cx, cy, rect) = measured_node_rect(node)?;
    Some(NodeBoundsInfo {
        id: node.id.clone(),
        cx,
        cy,
        rect,
    })
}

/// `samePoint(a, b, epsilon)`: both coordinates differ by less than `epsilon`.
pub(crate) fn same_point(a: Point, b: Point, epsilon: f64) -> bool {
    (a.x - b.x).abs() < epsilon && (a.y - b.y).abs() < epsilon
}

/// `sameX(a, b, epsilon)`.
pub(crate) fn same_x(a: Point, b: Point, epsilon: f64) -> bool {
    (a.x - b.x).abs() < epsilon
}

/// `sameY(a, b, epsilon)`.
pub(crate) fn same_y(a: Point, b: Point, epsilon: f64) -> bool {
    (a.y - b.y).abs() < epsilon
}

/// `isHorizontalSegment(a, b, epsilon)`: the same `y` and a length above `epsilon`.
pub(crate) fn is_horizontal_segment(a: Point, b: Point, epsilon: f64) -> bool {
    same_y(a, b, epsilon) && (a.x - b.x).abs() > epsilon
}

/// `isVerticalSegment(a, b, epsilon)`: the same `x` and a length above `epsilon`.
pub(crate) fn is_vertical_segment(a: Point, b: Point, epsilon: f64) -> bool {
    same_x(a, b, epsilon) && (a.y - b.y).abs() > epsilon
}

/// `overlapLength(a1, a2, b1, b2)`: the length of the common part of two intervals, zero when they are apart.
pub(crate) fn overlap_length(a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    js_max(
        0.0,
        js_min(js_max(a1, a2), js_max(b1, b2)) - js_max(js_min(a1, a2), js_min(b1, b2)),
    )
}

/// `sameAxisSegmentOverlapLength(a, b, epsilon)`: the overlap of two horizontal or two vertical segments on
/// the same line, zero otherwise.
pub(crate) fn same_axis_segment_overlap_length(
    a: &OrthogonalSegment,
    b: &OrthogonalSegment,
    epsilon: f64,
) -> f64 {
    if a.horizontal && b.horizontal && same_y(a.a, b.a, epsilon) {
        return overlap_length(a.a.x, a.b.x, b.a.x, b.b.x);
    }
    if a.vertical && b.vertical && same_x(a.a, b.a, epsilon) {
        return overlap_length(a.a.y, a.b.y, b.a.y, b.b.y);
    }
    0.0
}

/// `orthogonalSegmentsForPoints(points, epsilon)`: the horizontal and vertical segments of a polyline, in order.
/// A diagonal segment is skipped.
pub(crate) fn orthogonal_segments_for_points(
    points: &[Point],
    epsilon: f64,
) -> Vec<OrthogonalSegment> {
    let mut result = Vec::new();
    for i in 0..points.len().saturating_sub(1) {
        let a = points[i];
        let b = points[i + 1];
        let horizontal = is_horizontal_segment(a, b, epsilon);
        let vertical = is_vertical_segment(a, b, epsilon);
        if horizontal || vertical {
            result.push(OrthogonalSegment {
                index: i,
                a,
                b,
                horizontal,
                vertical,
            });
        }
    }
    result
}

/// `countOrthogonalBends(points, epsilon)`: the changes of direction between consecutive orthogonal segments.
pub(crate) fn count_orthogonal_bends(points: &[Point], epsilon: f64) -> usize {
    let segments = orthogonal_segments_for_points(points, epsilon);
    segments
        .windows(2)
        .filter(|pair| pair[0].horizontal != pair[1].horizontal)
        .count()
}

/// `dedupeConsecutivePoints(points, epsilon)`: a copy of the points without a point that is the same as the
/// point before it. Each point of the result is a new value.
pub(crate) fn dedupe_consecutive_points(points: &[Point], epsilon: f64) -> Vec<Point> {
    let mut result: Vec<Point> = Vec::new();
    for &point in points {
        let keep = match result.last() {
            Some(&last) => !same_point(last, point, epsilon),
            None => true,
        };
        if keep {
            result.push(Point {
                x: point.x,
                y: point.y,
            });
        }
    }
    result
}

/// `classifyThreeSegmentRoute(points, epsilon)`: a route of exactly four points whose segments alternate
/// horizontal and vertical, starting with either direction.
pub(crate) fn classify_three_segment_route(
    points: Option<&[Point]>,
    epsilon: f64,
) -> Option<ThreeSegmentRoute> {
    let points = points?;
    if points.len() != 4 {
        return None;
    }
    let (p0, p1, p2, p3) = (points[0], points[1], points[2], points[3]);
    let is_hvh = is_horizontal_segment(p0, p1, epsilon)
        && is_vertical_segment(p1, p2, epsilon)
        && is_horizontal_segment(p2, p3, epsilon);
    if is_hvh {
        return Some(ThreeSegmentRoute {
            kind: RouteKind::Hvh,
            p0,
            p1,
            p2,
            p3,
        });
    }
    let is_vhv = is_vertical_segment(p0, p1, epsilon)
        && is_horizontal_segment(p1, p2, epsilon)
        && is_vertical_segment(p2, p3, epsilon);
    if is_vhv {
        Some(ThreeSegmentRoute {
            kind: RouteKind::Vhv,
            p0,
            p1,
            p2,
            p3,
        })
    } else {
        None
    }
}

/// `segmentBoundsOverlapRect(a, b, rect, buffer)`: the bounding box of the segment meets the rectangle grown
/// by `buffer`. The test is on the boxes, not on the segment itself.
pub(crate) fn segment_bounds_overlap_rect(a: Point, b: Point, rect: Rect, buffer: f64) -> bool {
    let seg_min_x = js_min(a.x, b.x);
    let seg_max_x = js_max(a.x, b.x);
    let seg_min_y = js_min(a.y, b.y);
    let seg_max_y = js_max(a.y, b.y);
    seg_max_x > rect.left - buffer
        && seg_min_x < rect.right + buffer
        && seg_max_y > rect.top - buffer
        && seg_min_y < rect.bottom + buffer
}

/// `pointInsideRect(point, rect, buffer)`: strictly inside the rectangle shrunk by `buffer`.
pub(crate) fn point_inside_rect(point: Point, rect: Rect, buffer: f64) -> bool {
    point.x > rect.left + buffer
        && point.x < rect.right - buffer
        && point.y > rect.top + buffer
        && point.y < rect.bottom - buffer
}

/// `rectContainsRect(outer, inner)`: every side of `inner` is within `outer`, borders included.
pub(crate) fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    outer.left <= inner.left
        && outer.right >= inner.right
        && outer.top <= inner.top
        && outer.bottom >= inner.bottom
}

/// `rectsOverlap(a, b)`: the interiors meet. Rectangles that only share a border do not overlap.
pub(crate) fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
}

/// `inflateRect(rect, margin)`: the rectangle grown by `margin` on every side.
pub(crate) fn inflate_rect(rect: Rect, margin: f64) -> Rect {
    Rect {
        left: rect.left - margin,
        right: rect.right + margin,
        top: rect.top - margin,
        bottom: rect.bottom + margin,
    }
}

/// `rectFromCenterSize(cx, cy, width, height)`.
pub(crate) fn rect_from_center_size(cx: f64, cy: f64, width: f64, height: f64) -> Rect {
    Rect {
        left: cx - width / 2.0,
        right: cx + width / 2.0,
        top: cy - height / 2.0,
        bottom: cy + height / 2.0,
    }
}

/// `rectOfNodeBounds(node)`: the rectangle of a node with a positive size, or `None`. A group counts too: the
/// TypeScript does not check the group flag here.
pub(crate) fn rect_of_node_bounds(node: &Node) -> Option<Rect> {
    measured_node_rect(node).map(|(_, _, rect)| rect)
}

/// `portForRectSide(node, side)`: the middle of the side of the node's rectangle.
pub(crate) fn port_for_rect_side(node: &NodeBoundsInfo, side: RectSide) -> Point {
    match side {
        RectSide::Top => Point {
            x: node.cx,
            y: node.rect.top,
        },
        RectSide::Bottom => Point {
            x: node.cx,
            y: node.rect.bottom,
        },
        RectSide::Left => Point {
            x: node.rect.left,
            y: node.cy,
        },
        RectSide::Right => Point {
            x: node.rect.right,
            y: node.cy,
        },
    }
}

/// `buildOrthogonalPortPath(src, srcSide, dst, dstSide, anchor, epsilon)`: the orthogonal path from one port to
/// another, with at most two bends and the given offset from the ports, or `None` when no such path has the
/// shape of the sides.
pub(crate) fn build_orthogonal_port_path(
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
    anchor: f64,
    epsilon: f64,
) -> Option<Vec<Point>> {
    let src_h = matches!(src_side, RectSide::Left | RectSide::Right);
    let dst_h = matches!(dst_side, RectSide::Left | RectSide::Right);

    if src_h && dst_h {
        let opposing_dir =
            (src_side == RectSide::Right && dst_side == RectSide::Left && src.x < dst.x)
                || (src_side == RectSide::Left && dst_side == RectSide::Right && src.x > dst.x);
        if opposing_dir {
            if same_y(src, dst, epsilon) {
                return Some(vec![src, dst]);
            }
            let mid_x = (src.x + dst.x) / 2.0;
            return Some(vec![
                src,
                Point { x: mid_x, y: src.y },
                Point { x: mid_x, y: dst.y },
                dst,
            ]);
        }
        if src_side == dst_side {
            if same_y(src, dst, epsilon) {
                return None;
            }
            let int_x = if src_side == RectSide::Left {
                js_min(src.x, dst.x) - anchor
            } else {
                js_max(src.x, dst.x) + anchor
            };
            return Some(vec![
                src,
                Point { x: int_x, y: src.y },
                Point { x: int_x, y: dst.y },
                dst,
            ]);
        }
        return None;
    }

    if !src_h && !dst_h {
        if src_side == dst_side {
            if same_x(src, dst, epsilon) {
                return None;
            }
            let int_y = if src_side == RectSide::Top {
                js_min(src.y, dst.y) - anchor
            } else {
                js_max(src.y, dst.y) + anchor
            };
            return Some(vec![
                src,
                Point { x: src.x, y: int_y },
                Point { x: dst.x, y: int_y },
                dst,
            ]);
        }
        let same_dir = (src_side == RectSide::Bottom && dst_side == RectSide::Top && src.y < dst.y)
            || (src_side == RectSide::Top && dst_side == RectSide::Bottom && src.y > dst.y);
        if !same_dir {
            return None;
        }
        if same_x(src, dst, epsilon) {
            return Some(vec![src, dst]);
        }
        let mid_y = (src.y + dst.y) / 2.0;
        return Some(vec![
            src,
            Point { x: src.x, y: mid_y },
            Point { x: dst.x, y: mid_y },
            dst,
        ]);
    }

    if src_h && !dst_h {
        let same_dir_src = (src_side == RectSide::Right && dst.x > src.x)
            || (src_side == RectSide::Left && dst.x < src.x);
        let same_dir_dst = (dst_side == RectSide::Top && src.y < dst.y)
            || (dst_side == RectSide::Bottom && src.y > dst.y);
        return if same_dir_src && same_dir_dst {
            Some(vec![src, Point { x: dst.x, y: src.y }, dst])
        } else {
            None
        };
    }

    let same_dir_src = (src_side == RectSide::Bottom && dst.y > src.y)
        || (src_side == RectSide::Top && dst.y < src.y);
    let same_dir_dst = (dst_side == RectSide::Left && src.x < dst.x)
        || (dst_side == RectSide::Right && src.x > dst.x);
    if same_dir_src && same_dir_dst {
        Some(vec![src, Point { x: src.x, y: dst.y }, dst])
    } else {
        None
    }
}

/// `buildSameSideTrackPath(src, side, dst, track)`: a path that runs along a track outside both ports, on the
/// side they share.
pub(crate) fn build_same_side_track_path(
    src: Point,
    side: RectSide,
    dst: Point,
    track: f64,
) -> Vec<Point> {
    if matches!(side, RectSide::Left | RectSide::Right) {
        vec![
            src,
            Point { x: track, y: src.y },
            Point { x: track, y: dst.y },
            dst,
        ]
    } else {
        vec![
            src,
            Point { x: src.x, y: track },
            Point { x: dst.x, y: track },
            dst,
        ]
    }
}

/// `collectRealNodeBounds(nodes)`: the measured nodes that are not edge labels, by id (the last node with an
/// id wins), and their rectangles in node order.
pub(crate) fn collect_real_node_bounds(
    nodes: &[Node],
) -> (IndexMap<String, NodeBoundsInfo>, Vec<RectEntry>) {
    let mut node_info_by_id = IndexMap::new();
    let mut real_node_rects = Vec::new();
    for node in nodes {
        if node.is_edge_label == Some(true) {
            continue;
        }
        let Some(info) = node_bounds_info_for(node) else {
            continue;
        };
        real_node_rects.push(RectEntry {
            id: info.id.clone(),
            rect: info.rect,
        });
        node_info_by_id.insert(info.id.clone(), info);
    }
    (node_info_by_id, real_node_rects)
}

/// `collectNodeRectEntries(nodes)`: the measured nodes, split into the real nodes and the edge-label nodes.
pub(crate) fn collect_node_rect_entries(nodes: &[Node]) -> (Vec<RectEntry>, Vec<RectEntry>) {
    let mut real_node_rects = Vec::new();
    let mut label_node_rects = Vec::new();
    for node in nodes {
        let Some(info) = node_bounds_info_for(node) else {
            continue;
        };
        let entry = RectEntry {
            id: info.id,
            rect: info.rect,
        };
        if node.is_edge_label == Some(true) {
            label_node_rects.push(entry);
        } else {
            real_node_rects.push(entry);
        }
    }
    (real_node_rects, label_node_rects)
}

/// `collectLayoutNodeRects(nodes, { includeEdgeLabels })`: the rectangle of every node that is not a group,
/// from its centre and size (a missing size is 0).
pub(crate) fn collect_layout_node_rects(
    nodes: &[Node],
    include_edge_labels: bool,
) -> Vec<LayoutNodeRect> {
    let mut result = Vec::new();
    for node in nodes {
        if node.is_group == Some(true) || (!include_edge_labels && node.is_edge_label == Some(true))
        {
            continue;
        }
        let cx = node.x.unwrap_or(0.0);
        let cy = node.y.unwrap_or(0.0);
        let width = node.width.unwrap_or(0.0);
        let height = node.height.unwrap_or(0.0);
        result.push(LayoutNodeRect {
            node_id: node.id.clone(),
            rect: rect_from_center_size(cx, cy, width, height),
        });
    }
    result
}

/// `getNodePairGeometry(edge, nodeInfoById, epsilon)`: the measured ends of an edge, or `None` when an end is
/// missing or not measured.
pub(crate) fn get_node_pair_geometry(
    edge: &Edge,
    node_info_by_id: &IndexMap<String, NodeBoundsInfo>,
    epsilon: f64,
) -> Option<NodePairGeometry> {
    let src_id = truthy_text(edge.start.as_deref())?;
    let dst_id = truthy_text(edge.end.as_deref())?;
    let src_info = node_info_by_id.get(src_id)?;
    let dst_info = node_info_by_id.get(dst_id)?;
    Some(NodePairGeometry {
        src_id: src_id.to_owned(),
        dst_id: dst_id.to_owned(),
        src_info: src_info.clone(),
        dst_info: dst_info.clone(),
        collinear_x: (src_info.cx - dst_info.cx).abs() < epsilon,
        collinear_y: (src_info.cy - dst_info.cy).abs() < epsilon,
    })
}

/// `segmentHitsAnyRect(a, b, rects, excludeIds, shrink)`: the segment's box meets a rectangle that is not
/// excluded, with the rectangle shrunk by `shrink` (grown when negative).
pub(crate) fn segment_hits_any_rect(
    a: Point,
    b: Point,
    rects: &[RectEntry],
    exclude_ids: &[&str],
    shrink: f64,
) -> bool {
    rects.iter().any(|entry| {
        !exclude_ids.contains(&entry.id.as_str())
            && segment_bounds_overlap_rect(a, b, entry.rect, -shrink)
    })
}

/// `orthogonalSegmentsCross(a1, b1, a2, b2, epsilon, endpointTolerance)`: two orthogonal segments cross, or
/// touch at a point that is not an endpoint of both. Two parallel segments never cross.
pub(crate) fn orthogonal_segments_cross(
    a1: Point,
    b1: Point,
    a2: Point,
    b2: Point,
    epsilon: f64,
    endpoint_tolerance: f64,
) -> bool {
    let s1h = same_y(a1, b1, epsilon);
    let s1v = same_x(a1, b1, epsilon);
    let s2h = same_y(a2, b2, epsilon);
    let s2v = same_x(a2, b2, epsilon);
    if (s1h && s2h) || (s1v && s2v) {
        return false;
    }
    if !(s1h || s1v) || !(s2h || s2v) {
        return false;
    }

    let (horiz_a, horiz_b) = if s1h { (a1, b1) } else { (a2, b2) };
    let (vert_a, vert_b) = if s1v { (a1, b1) } else { (a2, b2) };
    let h_y = horiz_a.y;
    let h_x1 = js_min(horiz_a.x, horiz_b.x);
    let h_x2 = js_max(horiz_a.x, horiz_b.x);
    let v_x = vert_a.x;
    let v_y1 = js_min(vert_a.y, vert_b.y);
    let v_y2 = js_max(vert_a.y, vert_b.y);
    if v_x < h_x1 || v_x > h_x2 || h_y < v_y1 || h_y > v_y2 {
        return false;
    }

    let matches_horiz_endpoint = ((v_x - horiz_a.x).abs() < endpoint_tolerance
        && (h_y - horiz_a.y).abs() < endpoint_tolerance)
        || ((v_x - horiz_b.x).abs() < endpoint_tolerance
            && (h_y - horiz_b.y).abs() < endpoint_tolerance);
    let matches_vert_endpoint = ((v_x - vert_a.x).abs() < endpoint_tolerance
        && (h_y - vert_a.y).abs() < endpoint_tolerance)
        || ((v_x - vert_b.x).abs() < endpoint_tolerance
            && (h_y - vert_b.y).abs() < endpoint_tolerance);
    !(matches_horiz_endpoint && matches_vert_endpoint)
}

/// `sameAxisSegmentsOverlap(a1, b1, a2, b2, epsilon)`: two segments on the same line overlap by more than
/// `epsilon`.
pub(crate) fn same_axis_segments_overlap(
    a1: Point,
    b1: Point,
    a2: Point,
    b2: Point,
    epsilon: f64,
) -> bool {
    let s1h = same_y(a1, b1, epsilon);
    let s1v = same_x(a1, b1, epsilon);
    let s2h = same_y(a2, b2, epsilon);
    let s2v = same_x(a2, b2, epsilon);
    if s1v && s2v && same_x(a1, a2, epsilon) {
        return overlap_length(a1.y, b1.y, a2.y, b2.y) > epsilon;
    }
    if s1h && s2h && same_y(a1, a2, epsilon) {
        return overlap_length(a1.x, b1.x, a2.x, b2.x) > epsilon;
    }
    false
}

/// `segmentConflictsWithAnyEdge(a, b, edges, excludeEdge, { epsilon, skipDegenerateOther })`: the segment
/// crosses or overlaps a segment of another edge. `exclude` is the index of the edge to leave out, and the
/// layout-only edges and the edges with fewer than two points are left out.
pub(crate) fn segment_conflicts_with_any_edge(
    a: Point,
    b: Point,
    edges: &[Edge],
    exclude: Option<usize>,
    epsilon: f64,
    skip_degenerate_other: bool,
) -> bool {
    for (index, other) in edges.iter().enumerate() {
        if Some(index) == exclude || other.is_layout_only == Some(true) {
            continue;
        }
        let Some(points) = other.points.as_deref() else {
            continue;
        };
        if points.len() < 2 {
            continue;
        }
        for pair in points.windows(2) {
            let (oa, ob) = (pair[0], pair[1]);
            if skip_degenerate_other && same_point(oa, ob, epsilon) {
                continue;
            }
            if orthogonal_segments_cross(a, b, oa, ob, epsilon, 1e-6)
                || same_axis_segments_overlap(a, b, oa, ob, epsilon)
            {
                return true;
            }
        }
    }
    false
}

/// `orthogonalSegmentsStrictlyCross(a1, b1, a2, b2, epsilon)`: a horizontal and a vertical segment cross at a
/// point strictly inside both of them.
pub(crate) fn orthogonal_segments_strictly_cross(
    a1: Point,
    b1: Point,
    a2: Point,
    b2: Point,
    epsilon: f64,
) -> bool {
    let a_horiz = same_y(a1, b1, epsilon);
    let a_vert = same_x(a1, b1, epsilon);
    let b_horiz = same_y(a2, b2, epsilon);
    let b_vert = same_x(a2, b2, epsilon);
    if !((a_horiz && b_vert) || (a_vert && b_horiz)) {
        return false;
    }

    let (horiz_a, horiz_b, vert_a, vert_b) = if a_horiz {
        (a1, b1, a2, b2)
    } else {
        (a2, b2, a1, b1)
    };
    let h_y = horiz_a.y;
    let h_xmin = js_min(horiz_a.x, horiz_b.x);
    let h_xmax = js_max(horiz_a.x, horiz_b.x);
    let v_x = vert_a.x;
    let v_ymin = js_min(vert_a.y, vert_b.y);
    let v_ymax = js_max(vert_a.y, vert_b.y);
    v_x > h_xmin + epsilon
        && v_x < h_xmax - epsilon
        && h_y > v_ymin + epsilon
        && h_y < v_ymax - epsilon
}

/// `strictlyBetween(value, a, b)`: `value` is inside the open interval between `a` and `b`, by more than `EPS`
/// at each end.
fn strictly_between(value: f64, a: f64, b: f64) -> bool {
    let lo = js_min(a, b);
    let hi = js_max(a, b);
    value > lo + EPS && value < hi - EPS
}

/// `isCollinearIntermediate(prev, cur, next)`: `cur` lies strictly between `prev` and `next` on one line.
fn is_collinear_intermediate(prev: Point, cur: Point, next: Point) -> bool {
    if same_x(prev, cur, EPS) && same_x(cur, next, EPS) {
        return strictly_between(cur.y, prev.y, next.y);
    }
    if same_y(prev, cur, EPS) && same_y(cur, next, EPS) {
        return strictly_between(cur.x, prev.x, next.x);
    }
    false
}

/// `simplifyPolylineOnce(points)`: one pass that drops a point that returns to the point before it, and a point
/// that lies on the line between its neighbours. `changed` tells whether it dropped one.
fn simplify_polyline_once(points: &[Point]) -> (Vec<Point>, bool) {
    let mut changed = false;
    let mut out: Vec<Point> = Vec::new();
    let mut i = 0;
    while i < points.len() {
        let cur = points[i];
        let next = points.get(i + 1).copied();
        if let (Some(prev), Some(next)) = (out.last().copied(), next) {
            if same_point(prev, next, EPS) {
                // The TypeScript skips `cur` and `next` together.
                i += 2;
                changed = true;
                continue;
            }
            if is_collinear_intermediate(prev, cur, next) {
                changed = true;
                i += 1;
                continue;
            }
        }
        out.push(cur);
        i += 1;
    }
    (out, changed)
}

/// `orthogonalizePolyline(pts)`: inserts an L-bend between two consecutive points that differ in both
/// coordinates, then drops the consecutive duplicates. The input must have at least one point; the
/// TypeScript returns a list with one `undefined` for none, and no caller passes an empty list.
pub(crate) fn orthogonalize_polyline(pts: &[Point]) -> Vec<Point> {
    let Some(&first) = pts.first() else {
        return Vec::new();
    };
    let mut cleaned: Vec<Point> = vec![first];
    for &curr in &pts[1..] {
        let prev = cleaned[cleaned.len() - 1];
        if !same_x(prev, curr, EPS) && !same_y(prev, curr, EPS) {
            let incoming_vertical = cleaned
                .len()
                .checked_sub(2)
                .map(|index| cleaned[index])
                .is_some_and(|prev_prev| same_x(prev_prev, prev, EPS));
            let corner = if incoming_vertical {
                Point {
                    x: prev.x,
                    y: curr.y,
                }
            } else {
                Point {
                    x: curr.x,
                    y: prev.y,
                }
            };
            cleaned.push(corner);
        }
        cleaned.push(curr);
    }
    let mut deduped: Vec<Point> = Vec::new();
    for point in cleaned {
        let keep = match deduped.last() {
            Some(&last) => !same_point(last, point, EPS),
            None => true,
        };
        if keep {
            deduped.push(point);
        }
    }
    deduped
}

/// `simplifyPolyline(pts)`: repeats [`simplify_polyline_once`] until nothing changes, at most 32 times. A
/// polyline of fewer than three points is returned as it is.
pub(crate) fn simplify_polyline(pts: &[Point]) -> Vec<Point> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut work = pts.to_vec();
    for _ in 0..32 {
        let (next, changed) = simplify_polyline_once(&work);
        work = next;
        if !changed {
            break;
        }
    }
    work
}

/// The edge loop of `postProcessSwimlaneLayout` (`postProcessing.ts`, the loop before `simplifyDetouredEdges`),
/// stage `post:orthogonalizePolyline+simplifyPolyline`: every edge that is not layout-only and has at least two
/// points gets `simplifyPolyline(orthogonalizePolyline(points))`.
pub fn orthogonalize_and_simplify_edges(layout: &mut LayoutData) {
    for edge in &mut layout.edges {
        if edge.is_layout_only == Some(true) {
            continue;
        }
        let Some(points) = edge.points.as_deref() else {
            continue;
        };
        if points.len() < 2 {
            continue;
        }
        let cleaned = simplify_polyline(&orthogonalize_polyline(points));
        edge.points = Some(cleaned);
    }
}

#[cfg(test)]
mod tests {
    use crate::js::js_sort_by;

    /// The keys of arrays of fewer than 64 items, with NaN and infinite keys, and the indices of the items in the
    /// order that `Array.prototype.sort((a, b) => a.k - b.k)` leaves them in V8 (Node v24.21.0).
    #[rustfmt::skip]
    const V8_NAN_ORDERS: &[(&[f64], &[usize])] = &[
        (
            &[
                0.0, 3.0, 7.0, 3.0, 3.0, f64::NEG_INFINITY, 2.5, -1.0, 2.5, -1.0, 0.0, -1.0, f64::NEG_INFINITY, 2.0, f64::INFINITY, f64::INFINITY,
                -1.0, 2.5, 2.5, f64::NEG_INFINITY, 1.0, -1.0, 0.0, f64::INFINITY, 0.0, 7.0, 2.5, 1.0, 1.0, 2.5, f64::NEG_INFINITY, f64::NEG_INFINITY,
                0.0, 0.0, 2.0, 7.0, 1.0, -1.0, f64::INFINITY, 1.0, 2.5, 2.5,
            ],
            &[
                5, 12, 19, 30, 31, 7, 9, 11, 16, 21, 37, 0, 10, 22, 24, 32, 33, 20, 27, 28, 36, 39, 13, 34,
                6, 8, 17, 18, 26, 29, 40, 41, 1, 3, 4, 2, 25, 35, 14, 15, 23, 38,
            ],
        ),
        (
            &[
                2.0, 2.5, -1.0, f64::NEG_INFINITY, 2.0, f64::NAN, 3.0, 0.0, 3.0, f64::INFINITY, 0.0, 1.0, 1.0, 7.0, 1.0, f64::NAN,
                f64::INFINITY, f64::NEG_INFINITY, 3.0, f64::NAN, f64::INFINITY,
            ],
            &[
                3, 17, 2, 7, 10, 11, 12, 14, 0, 4, 1, 5, 6, 8, 18, 13, 9, 15, 16, 19, 20,
            ],
        ),
        (
            &[
                1.0, f64::INFINITY, 2.5, 1.0, 2.0, 2.5, 3.0, 3.0, f64::NAN, 2.0, 1.0, 2.0, 3.0, f64::INFINITY, 1.0, 7.0,
                f64::INFINITY, 2.0, 7.0,
            ],
            &[
                0, 3, 10, 14, 4, 9, 11, 17, 2, 5, 6, 7, 15, 1, 8, 12, 18, 13, 16,
            ],
        ),
        (
            &[
                f64::NAN, 3.0, 3.0, 0.0, 2.5, 3.0, f64::INFINITY, -1.0, 7.0, 1.0, f64::NEG_INFINITY, 2.0, 3.0, 2.5, 2.0, 3.0,
                f64::NAN, 7.0, f64::NAN, f64::INFINITY, 1.0, 2.5, 0.0, 2.5, f64::INFINITY, 2.5, 1.0, 1.0, f64::NAN, 0.0, -1.0, 2.5,
                -1.0, 7.0, 2.0, 2.5, 0.0, f64::NAN, f64::INFINITY,
            ],
            &[
                0, 10, 7, 30, 32, 3, 22, 29, 36, 9, 20, 26, 27, 11, 14, 34, 4, 13, 21, 23, 25, 31, 35, 1,
                2, 5, 12, 15, 8, 17, 6, 16, 18, 33, 19, 24, 28, 37, 38,
            ],
        ),
        (
            &[
                3.0, 3.0, 3.0, 1.0, 3.0, 7.0, 3.0, f64::NEG_INFINITY, 2.0, f64::INFINITY, 3.0, 1.0, 0.0, 3.0, 0.0, 0.0,
                f64::INFINITY, 0.0, f64::NEG_INFINITY, 2.0, 7.0, -1.0, 2.0, 2.5, 2.5, 0.0, 1.0, 1.0, f64::NEG_INFINITY, 2.5, 0.0, f64::NAN,
                3.0, 0.0,
            ],
            &[
                7, 18, 28, 21, 12, 14, 15, 17, 25, 30, 33, 3, 11, 26, 27, 8, 19, 22, 23, 24, 29, 0, 1, 2,
                4, 6, 10, 13, 32, 5, 20, 9, 16, 31,
            ],
        ),
        (
            &[
                f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, -1.0, -1.0, -1.0, 0.0, f64::INFINITY,
            ],
            &[
                0, 1, 2, 3, 4, 5, 6, 7,
            ],
        ),
        (
            &[
                7.0, 0.0, f64::NAN, 2.5, 0.0, 7.0, f64::INFINITY, f64::INFINITY, 2.0, 7.0, f64::INFINITY, -1.0, 0.0, 2.5, f64::INFINITY, 2.0,
                -1.0, 2.0, 3.0, 3.0, f64::INFINITY, f64::NEG_INFINITY, -1.0, f64::NAN, f64::NEG_INFINITY, 1.0, 1.0, 0.0, 3.0, 3.0, 2.0,
            ],
            &[
                21, 24, 11, 16, 22, 1, 4, 12, 27, 25, 26, 3, 0, 2, 8, 15, 17, 30, 13, 18, 19, 28, 29, 5,
                9, 6, 7, 10, 14, 20, 23,
            ],
        ),
        (
            &[
                1.0, 1.0, 2.0, 0.0, 7.0, 7.0, 3.0, 1.0, 2.5, f64::INFINITY, f64::NAN, 1.0, 0.0, 7.0, 7.0, f64::NAN,
                1.0, 0.0, f64::NEG_INFINITY, 2.5, f64::INFINITY, 7.0, -1.0,
            ],
            &[
                18, 22, 3, 12, 17, 0, 1, 7, 11, 16, 2, 8, 19, 6, 4, 5, 14, 21, 9, 10, 13, 15, 20,
            ],
        ),
        (
            &[
                -1.0, 1.0, 3.0, 2.5, 0.0, -1.0, 2.0, 3.0, 2.0, f64::NAN, 7.0, -1.0, f64::NAN, 3.0, f64::INFINITY, 2.5,
                -1.0, 7.0, 2.0, 2.5, f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, 7.0, -1.0, f64::INFINITY, 0.0, 2.5, -1.0, 3.0, -1.0, f64::NEG_INFINITY,
                2.5, 2.5, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, f64::INFINITY, 0.0, 1.0, 1.0, f64::NAN, 0.0, 3.0, 7.0, f64::NEG_INFINITY, f64::NAN,
                f64::INFINITY, -1.0, 1.0, 3.0, 2.0, f64::NEG_INFINITY, 2.5, 1.0, 3.0, 3.0, 0.0, f64::NEG_INFINITY, 2.0, 2.0, -1.0,
            ],
            &[
                21, 31, 36, 46, 53, 59, 0, 5, 11, 16, 24, 28, 30, 34, 49, 62, 4, 26, 39, 43, 58, 1, 40, 41,
                50, 55, 6, 8, 18, 52, 60, 61, 3, 15, 19, 33, 2, 7, 9, 10, 12, 27, 32, 54, 13, 29, 44, 51,
                56, 57, 17, 23, 45, 14, 20, 22, 25, 35, 37, 38, 42, 47, 48,
            ],
        ),
        (
            &[
                f64::NAN, 7.0, -1.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, 3.0, 7.0, 1.0, 1.0, 7.0, 3.0, 1.0, 3.0, 2.0, 2.0,
                -1.0, -1.0, f64::INFINITY, f64::INFINITY, f64::NAN, f64::NEG_INFINITY, 2.5, 1.0, 0.0, f64::NAN, f64::NEG_INFINITY, 7.0,
            ],
            &[
                0, 5, 21, 26, 2, 3, 16, 17, 24, 8, 9, 12, 23, 14, 15, 22, 6, 11, 13, 1, 7, 10, 27, 4,
                18, 19, 20, 25,
            ],
        ),
        (
            &[
                2.0, 7.0, -1.0, -1.0, 2.5, 7.0, 3.0, 3.0, f64::NEG_INFINITY, -1.0, 0.0, 3.0, 1.0, 7.0, 1.0, f64::NAN,
                3.0, f64::NEG_INFINITY, f64::NEG_INFINITY, 7.0, 1.0, 1.0, f64::NAN, 7.0, 1.0, 3.0, 7.0, -1.0, -1.0, 0.0, f64::INFINITY, -1.0,
                f64::NAN, f64::NAN, 2.5, 1.0, 2.5, 3.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 2.0, 2.0, 3.0, f64::NAN, f64::INFINITY, f64::INFINITY, f64::NAN,
                f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY,
            ],
            &[
                8, 17, 18, 39, 48, 49, 2, 3, 9, 27, 28, 31, 10, 29, 12, 14, 20, 21, 24, 35, 0, 41, 42, 4,
                6, 7, 11, 16, 25, 37, 43, 1, 5, 13, 15, 19, 22, 34, 36, 23, 26, 30, 32, 33, 38, 40, 44, 45,
                46, 47, 50,
            ],
        ),
        (
            &[
                f64::NEG_INFINITY, 1.0, 7.0, f64::NAN, -1.0, 3.0, 1.0, f64::NAN, f64::NEG_INFINITY, f64::NEG_INFINITY, 3.0, 2.5, 3.0, f64::INFINITY, 1.0, f64::INFINITY,
                -1.0, f64::NAN, f64::INFINITY, 0.0, 0.0, 1.0, 1.0, f64::NAN, f64::NAN, 2.5, f64::NAN, 2.0, 7.0, 2.5, f64::NEG_INFINITY, f64::NAN,
                f64::INFINITY, 7.0, -1.0, 0.0, 7.0, 0.0, f64::NAN, 0.0, 3.0, 1.0, 1.0, 1.0, 3.0, 2.0, f64::NAN, 2.0,
                2.5, -1.0, 2.5, 0.0, 1.0, f64::NEG_INFINITY,
            ],
            &[
                0, 8, 30, 1, 2, 3, 9, 53, 4, 16, 34, 49, 19, 20, 35, 37, 39, 51, 6, 14, 21, 22, 41, 42,
                43, 52, 27, 47, 11, 5, 7, 45, 25, 29, 48, 50, 10, 12, 40, 44, 28, 33, 36, 13, 15, 17, 18, 23,
                24, 26, 31, 32, 38, 46,
            ],
        ),
        (
            &[
                0.0, 1.0, 2.5, 1.0, 1.0, 2.5, 7.0, 1.0, 0.0, 1.0, 2.5, 2.0, 7.0, 7.0, 7.0, f64::NEG_INFINITY,
                2.0, -1.0, f64::INFINITY, 1.0, f64::NEG_INFINITY, 2.5, f64::INFINITY, f64::NAN, f64::NEG_INFINITY, 2.5, f64::NEG_INFINITY, f64::NAN, -1.0, f64::INFINITY, 2.5, f64::INFINITY,
                7.0, 0.0, f64::INFINITY,
            ],
            &[
                15, 20, 24, 26, 17, 28, 0, 8, 33, 1, 3, 4, 7, 9, 19, 11, 16, 2, 5, 10, 21, 25, 30, 6,
                12, 13, 14, 18, 22, 23, 27, 32, 29, 31, 34,
            ],
        ),
        (
            &[
                f64::INFINITY, 0.0, -1.0, 1.0, -1.0, 3.0, 3.0, 7.0, -1.0, 0.0, -1.0, f64::NAN, 2.0, f64::INFINITY, 1.0, f64::NAN,
                7.0, 2.0, -1.0, 2.5, 1.0, -1.0, 2.0, f64::NAN, 7.0, f64::NEG_INFINITY, f64::NAN, 0.0,
            ],
            &[
                25, 2, 4, 8, 10, 18, 21, 1, 9, 27, 3, 14, 20, 12, 17, 22, 19, 5, 6, 7, 16, 0, 11, 24,
                13, 15, 23, 26,
            ],
        ),
        (
            &[
                f64::NEG_INFINITY, 0.0, 1.0, 2.0, -1.0, 2.0, f64::NAN, 2.5, f64::NAN, 2.0, f64::NAN, 0.0, 0.0, f64::NAN, f64::NEG_INFINITY, 0.0,
                7.0, 2.0, 2.5, 1.0, 2.5, f64::NAN, -1.0, 1.0, 1.0,
            ],
            &[
                0, 14, 4, 1, 11, 12, 15, 2, 19, 24, 3, 5, 6, 9, 7, 8, 10, 13, 22, 23, 17, 18, 20, 16,
                21,
            ],
        ),
        (
            &[
                -1.0, 7.0, 0.0, 3.0, f64::NEG_INFINITY, 3.0, 1.0, 2.0, f64::NAN, 0.0, 0.0, 2.5, f64::INFINITY, f64::NEG_INFINITY, 2.0, 2.5,
                2.5, 3.0, 2.5, f64::NEG_INFINITY,
            ],
            &[
                4, 13, 19, 0, 2, 9, 10, 6, 7, 14, 11, 15, 16, 18, 3, 5, 1, 8, 17, 12,
            ],
        ),
        (
            &[
                f64::NEG_INFINITY, 3.0, -1.0, f64::NEG_INFINITY, 2.0, 1.0, 2.5, 3.0, f64::INFINITY, 2.5, f64::INFINITY, 7.0, 3.0, 2.5, -1.0, f64::NAN,
                2.5, 7.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, 2.5, 0.0, -1.0, f64::INFINITY, f64::INFINITY, 3.0, 2.0, 1.0,
                1.0, 1.0, f64::INFINITY, f64::INFINITY, 7.0, -1.0, 0.0, f64::NEG_INFINITY, 3.0, 2.5, 1.0, -1.0, f64::NAN, 0.0, 1.0, 7.0,
                2.5, 2.5, f64::NEG_INFINITY, 3.0,
            ],
            &[
                0, 3, 20, 21, 22, 39, 50, 2, 14, 18, 26, 37, 43, 23, 25, 38, 45, 5, 31, 32, 33, 42, 46, 4,
                30, 6, 9, 13, 16, 24, 41, 48, 49, 1, 7, 12, 29, 40, 51, 11, 17, 47, 8, 10, 15, 36, 19, 27,
                28, 34, 35, 44,
            ],
        ),
        (
            &[
                1.0, 7.0, 7.0, 7.0, f64::NAN, 7.0, f64::NEG_INFINITY, 3.0, -1.0, 2.5, 3.0, 0.0, f64::INFINITY, 7.0, 3.0, -1.0,
                -1.0, f64::INFINITY, 0.0, -1.0, 7.0, 3.0, 0.0, 1.0, 2.5, 2.5, -1.0, 2.0, f64::NEG_INFINITY, 7.0,
            ],
            &[
                6, 28, 8, 15, 16, 19, 26, 11, 18, 22, 0, 23, 27, 9, 24, 25, 7, 10, 14, 21, 1, 2, 3, 4,
                5, 13, 20, 29, 12, 17,
            ],
        ),
        (
            &[
                1.0, 1.0, f64::NEG_INFINITY, 3.0, 7.0, 7.0, f64::INFINITY, -1.0, -1.0, f64::NAN, f64::INFINITY, 2.0, 3.0, 1.0, f64::NAN, 2.0,
                1.0, 0.0, f64::INFINITY, 2.0, 7.0, f64::NAN, 3.0, 2.5, f64::NEG_INFINITY, 3.0, 7.0, -1.0, -1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 2.5, f64::NEG_INFINITY, 7.0, f64::INFINITY, 2.5, -1.0, 2.0,
            ],
            &[
                2, 24, 35, 7, 8, 27, 28, 39, 17, 29, 30, 31, 32, 0, 1, 13, 16, 33, 11, 15, 19, 40, 23, 38,
                3, 12, 4, 5, 20, 6, 9, 34, 26, 10, 14, 22, 18, 21, 25, 36, 37,
            ],
        ),
        (
            &[
                2.0, 7.0, 2.5, 0.0, f64::INFINITY, f64::NAN, f64::NAN, f64::NEG_INFINITY, -1.0, f64::INFINITY, -1.0, 0.0, 3.0, f64::INFINITY, -1.0, 2.0,
                7.0, f64::INFINITY, 3.0, 1.0, f64::INFINITY, f64::NAN, 7.0, -1.0, -1.0, 1.0, 7.0, 2.5, 2.5, -1.0, 2.0, 2.5,
                3.0, -1.0, 2.5, f64::NAN, f64::NAN, 1.0, 1.0, f64::NAN, 2.0, f64::INFINITY, f64::NEG_INFINITY, -1.0, f64::INFINITY, 2.5, f64::NEG_INFINITY, 2.0,
                0.0,
            ],
            &[
                7, 42, 46, 8, 10, 14, 23, 24, 29, 33, 43, 3, 11, 48, 19, 25, 37, 38, 0, 15, 30, 40, 47, 2,
                27, 28, 31, 34, 45, 18, 32, 1, 4, 5, 6, 12, 16, 22, 26, 9, 13, 17, 20, 21, 35, 36, 39, 41,
                44,
            ],
        ),
        (
            &[
                0.0, -1.0, f64::NAN, f64::INFINITY, 1.0, f64::INFINITY, 2.0, 3.0, 2.0, f64::NEG_INFINITY, f64::INFINITY, 2.0,
            ],
            &[
                1, 0, 2, 9, 4, 6, 8, 11, 7, 3, 5, 10,
            ],
        ),
        (
            &[
                1.0, 0.0, f64::INFINITY, f64::INFINITY, 2.0, 2.5, f64::NEG_INFINITY, 2.0, 7.0, 2.0, 2.0, f64::NEG_INFINITY, 2.0, f64::NAN, 7.0, f64::NAN,
                f64::INFINITY, f64::NAN, 2.5, f64::NAN, 0.0, 3.0, f64::NAN, 2.0, -1.0, f64::NEG_INFINITY, 3.0, 2.5, f64::NEG_INFINITY, 1.0, 2.0, f64::NAN,
                2.0, 2.0, 2.5, -1.0, f64::NEG_INFINITY, f64::NAN,
            ],
            &[
                6, 11, 25, 28, 36, 24, 35, 1, 20, 0, 29, 4, 7, 9, 10, 12, 23, 30, 32, 33, 5, 34, 8, 14,
                2, 3, 13, 15, 18, 16, 17, 19, 27, 21, 22, 26, 31, 37,
            ],
        ),
        (
            &[
                2.5, f64::NEG_INFINITY, 2.0, -1.0, 3.0, 3.0, -1.0, 2.5, 2.0, f64::NAN, 2.0, f64::NEG_INFINITY, f64::NAN, f64::NEG_INFINITY, 1.0, f64::INFINITY,
                -1.0, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 2.0, 1.0,
            ],
            &[
                1, 11, 13, 3, 6, 16, 14, 22, 2, 8, 10, 21, 0, 7, 4, 5, 9, 12, 15, 17, 18, 19, 20,
            ],
        ),
        (
            &[
                2.5, 1.0, 3.0, f64::INFINITY, f64::NAN, -1.0, f64::NEG_INFINITY, 0.0, f64::NEG_INFINITY, -1.0, 7.0, f64::INFINITY, f64::NAN, f64::NAN, f64::INFINITY, 2.5,
                1.0, f64::NEG_INFINITY, 1.0, 3.0, 2.5, 1.0, 7.0, 0.0, 2.0, f64::INFINITY, f64::NEG_INFINITY, 1.0, 1.0, f64::NAN, 3.0, 2.5,
                f64::INFINITY, 2.0, 7.0, 0.0, f64::INFINITY, -1.0, f64::INFINITY, 2.0, 0.0, 3.0, f64::NEG_INFINITY, 3.0, 2.5, f64::INFINITY, f64::NEG_INFINITY,
            ],
            &[
                6, 8, 17, 26, 42, 46, 5, 9, 37, 7, 23, 35, 40, 1, 16, 18, 21, 27, 28, 24, 33, 39, 0, 15,
                20, 31, 44, 2, 30, 41, 10, 3, 4, 43, 22, 34, 11, 12, 13, 19, 14, 25, 29, 32, 36, 38, 45,
            ],
        ),
        (
            &[
                1.0, 0.0, 7.0, -1.0,
            ],
            &[
                3, 1, 0, 2,
            ],
        ),
        (
            &[
                0.0, f64::NEG_INFINITY, f64::NEG_INFINITY, -1.0, 7.0, 2.5, 3.0, f64::INFINITY, 3.0, 1.0, -1.0, 3.0, f64::NAN, 2.0, 2.5, 2.0,
                1.0, 7.0, f64::INFINITY, 0.0, f64::NEG_INFINITY, f64::NAN, -1.0, f64::NEG_INFINITY, -1.0, 3.0, 7.0, 3.0, 0.0, f64::NAN, f64::NEG_INFINITY, f64::NEG_INFINITY,
                f64::NEG_INFINITY, f64::NEG_INFINITY, 2.0, 1.0, 0.0, -1.0, f64::INFINITY, 3.0, 2.0, f64::NEG_INFINITY, f64::NAN, f64::INFINITY, 2.0, f64::NAN, 2.5, f64::INFINITY,
                2.5, f64::NAN, f64::NAN, 2.5, 3.0, 1.0, f64::INFINITY, 7.0, -1.0, 7.0, 7.0,
            ],
            &[
                1, 2, 20, 23, 30, 31, 32, 33, 41, 3, 10, 22, 24, 37, 56, 0, 19, 28, 36, 9, 16, 35, 53, 13,
                15, 34, 40, 44, 5, 14, 46, 48, 51, 6, 8, 11, 25, 27, 39, 52, 4, 17, 55, 57, 58, 7, 12, 18,
                21, 26, 29, 38, 42, 43, 45, 47, 49, 50, 54,
            ],
        ),
        (
            &[
                7.0, -1.0, -1.0, -1.0, f64::NAN, 2.0, f64::INFINITY, 3.0, f64::NAN, 2.0, f64::INFINITY, 7.0, f64::NEG_INFINITY, 2.0, 3.0, -1.0,
                7.0, 2.5, 0.0, 1.0, 0.0, 3.0, 2.0, 7.0, 2.5, 1.0, f64::NEG_INFINITY, 0.0, 3.0, f64::NAN, -1.0, f64::INFINITY,
                7.0, 7.0, 3.0, f64::NAN, 2.5, 7.0,
            ],
            &[
                26, 1, 2, 3, 30, 18, 20, 27, 25, 9, 7, 0, 4, 12, 15, 19, 5, 13, 22, 17, 24, 36, 14, 21,
                28, 34, 11, 33, 37, 6, 8, 16, 23, 10, 29, 32, 31, 35,
            ],
        ),
        (
            &[
                2.0, 3.0, 2.0, 0.0, 1.0, 2.0, f64::NEG_INFINITY, f64::NAN,
            ],
            &[
                6, 3, 4, 0, 2, 5, 1, 7,
            ],
        ),
        (
            &[
                7.0, f64::NEG_INFINITY, 3.0, 2.0, 3.0, 3.0, 1.0, -1.0, 3.0, f64::NAN, 3.0, 3.0, 1.0, 1.0,
            ],
            &[
                1, 7, 6, 12, 13, 3, 2, 4, 5, 8, 10, 0, 9, 11,
            ],
        ),
        (
            &[
                3.0, 1.0, 3.0, f64::NAN, f64::INFINITY, 0.0, 2.5, 1.0, f64::NAN, 3.0, f64::NAN, 2.0, 7.0, 2.5, 1.0, f64::INFINITY,
                0.0, 2.5, 3.0, 2.0, 2.0, 1.0, 3.0, -1.0, 2.0, 2.0, 1.0, f64::NAN, 7.0, 0.0, f64::INFINITY, 7.0,
                2.0, f64::INFINITY, 0.0, -1.0, 2.0, 2.0, 0.0, 2.0, 2.5, 3.0, -1.0, 1.0, 2.0, -1.0, f64::INFINITY, f64::INFINITY,
                f64::NEG_INFINITY, 0.0, 7.0, 2.0, f64::INFINITY, f64::NAN, 7.0, 0.0, f64::NAN, 2.5,
            ],
            &[
                48, 23, 35, 42, 45, 5, 16, 29, 34, 38, 49, 55, 1, 7, 14, 21, 26, 43, 11, 19, 20, 24, 25, 32,
                36, 37, 39, 44, 51, 6, 13, 17, 40, 0, 2, 3, 9, 18, 22, 41, 12, 4, 8, 10, 57, 31, 15, 27,
                28, 50, 54, 30, 33, 46, 47, 52, 53, 56,
            ],
        ),
        (
            &[
                2.5, 7.0, 1.0, f64::NEG_INFINITY, 2.0, 7.0, 2.0, -1.0, 1.0, f64::NEG_INFINITY, 2.5, 1.0, 3.0, 3.0, f64::INFINITY, f64::NEG_INFINITY,
                f64::NAN, f64::INFINITY, 2.5, 0.0, 2.5, f64::NAN, 7.0, 1.0, 0.0, f64::NAN, 2.0, f64::NAN, 0.0, f64::NAN, f64::NEG_INFINITY, 2.0,
                7.0, -1.0, f64::INFINITY, 3.0, f64::NAN, 2.0, 2.5, f64::INFINITY, -1.0,
            ],
            &[
                3, 9, 15, 30, 7, 33, 40, 19, 24, 28, 2, 8, 11, 23, 4, 6, 26, 31, 37, 0, 10, 18, 20, 12,
                13, 35, 1, 5, 32, 14, 16, 22, 17, 21, 25, 27, 29, 38, 34, 36, 39,
            ],
        ),
        (
            &[
                f64::NAN, 1.0, f64::INFINITY, f64::INFINITY, f64::INFINITY, 0.0, 3.0, 7.0, f64::INFINITY, 7.0, 7.0, 2.5, 1.0, f64::INFINITY, -1.0, 1.0,
                2.0, 2.5, 2.0, -1.0, 0.0, f64::NEG_INFINITY, f64::NEG_INFINITY, 1.0, -1.0, f64::INFINITY, -1.0, 2.5, f64::NAN, 3.0, 1.0, 2.5,
                2.5, 2.0, f64::NAN, 1.0, f64::NEG_INFINITY, 1.0, 3.0, 7.0, 2.0, 0.0, 0.0, 1.0,
            ],
            &[
                0, 21, 22, 36, 14, 19, 24, 26, 5, 20, 41, 42, 1, 12, 15, 23, 30, 35, 37, 43, 16, 18, 33, 40,
                11, 17, 27, 31, 32, 6, 29, 38, 7, 9, 10, 39, 2, 3, 4, 8, 13, 25, 28, 34,
            ],
        ),
        (
            &[
                7.0, -1.0, 1.0, 1.0, f64::NEG_INFINITY, 1.0, f64::NEG_INFINITY, 2.5, 7.0, f64::NAN, 2.0, 2.5, 7.0, f64::INFINITY, 7.0, 2.5,
                3.0, 0.0, f64::NAN, f64::INFINITY, 7.0, f64::INFINITY, f64::NAN, 2.0, f64::NEG_INFINITY, 0.0,
            ],
            &[
                4, 6, 24, 1, 17, 25, 2, 3, 5, 10, 23, 7, 11, 0, 8, 9, 15, 16, 12, 14, 13, 18, 20, 19,
                21, 22,
            ],
        ),
        (
            &[
                2.0, 2.5, 0.0, f64::NAN, 2.0, f64::INFINITY, 7.0, 7.0, -1.0, 2.0, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NEG_INFINITY, 1.0,
                -1.0, f64::NEG_INFINITY, 3.0, f64::NAN, 3.0, 7.0, 2.5, 3.0, f64::NAN, f64::INFINITY, 1.0, 7.0, 3.0, -1.0, f64::INFINITY, 0.0,
                f64::NAN, 2.0, 2.0, f64::INFINITY, 7.0, f64::NEG_INFINITY, f64::INFINITY, 1.0, 1.0, 3.0, f64::NEG_INFINITY, f64::NEG_INFINITY, 2.0, 2.0, 3.0, f64::NAN,
                7.0, 2.5, 0.0, 3.0, f64::NAN, 2.0, f64::INFINITY, 2.5, f64::NAN, 3.0,
            ],
            &[
                14, 17, 37, 42, 43, 16, 29, 2, 31, 50, 15, 26, 39, 40, 0, 4, 1, 3, 8, 9, 33, 34, 44, 45,
                53, 22, 49, 55, 28, 6, 7, 5, 10, 11, 12, 13, 18, 19, 20, 23, 51, 21, 24, 41, 46, 27, 36, 48,
                25, 30, 32, 35, 38, 47, 52, 54, 56, 57,
            ],
        ),
        (
            &[
                f64::INFINITY, 1.0, 3.0, f64::NAN, 7.0, f64::INFINITY, -1.0, 3.0, 2.5, 7.0, f64::NEG_INFINITY, 1.0, 2.0, -1.0, 0.0, -1.0,
                3.0, -1.0, 1.0, f64::INFINITY, 1.0, f64::NAN, 2.5, 7.0, 3.0, f64::INFINITY, 2.5, f64::NEG_INFINITY, f64::NEG_INFINITY, 2.0, 7.0, -1.0,
                2.5, 0.0, -1.0, f64::NEG_INFINITY, f64::NAN, f64::NAN, f64::NAN, -1.0, f64::NEG_INFINITY, 2.0, f64::NEG_INFINITY, f64::NAN, f64::NAN, -1.0, 2.5, f64::INFINITY,
                2.5, -1.0, f64::NAN, -1.0, f64::NEG_INFINITY, f64::NAN, 3.0, f64::NAN, 3.0, 0.0, f64::NAN, 7.0, f64::NAN, 1.0,
            ],
            &[
                10, 27, 28, 35, 40, 42, 52, 6, 13, 15, 17, 31, 34, 39, 45, 49, 51, 14, 33, 57, 1, 11, 18, 20,
                61, 12, 29, 41, 8, 26, 32, 46, 48, 2, 7, 16, 54, 56, 4, 0, 3, 22, 24, 9, 23, 30, 5, 19,
                21, 25, 36, 37, 38, 43, 44, 47, 50, 53, 55, 58, 59, 60,
            ],
        ),
        (
            &[
                f64::NAN, 7.0, 2.5, 3.0, 2.0, 1.0, 1.0, 1.0, 2.0, f64::NAN, f64::NEG_INFINITY, 1.0, 0.0,
            ],
            &[
                0, 10, 12, 5, 6, 7, 11, 4, 8, 2, 3, 1, 9,
            ],
        ),
        (
            &[
                f64::INFINITY, 0.0, 1.0, f64::NEG_INFINITY, 1.0, 3.0, f64::NEG_INFINITY, 2.5, f64::NAN,
            ],
            &[
                3, 6, 1, 2, 4, 7, 5, 0, 8,
            ],
        ),
        (
            &[
                7.0, f64::INFINITY, 2.5, f64::INFINITY, -1.0, f64::NEG_INFINITY, f64::NAN, -1.0, 7.0, 2.0, 2.5, 2.5, 1.0, f64::NEG_INFINITY, f64::NEG_INFINITY, 7.0,
                2.5, -1.0, f64::NAN, 7.0, 0.0, f64::INFINITY, 7.0, 7.0, -1.0, 7.0, -1.0, -1.0, 0.0, 0.0, 7.0, f64::NEG_INFINITY,
                2.0, f64::NEG_INFINITY, 1.0, 1.0, f64::INFINITY, 3.0, f64::NEG_INFINITY, 3.0, 0.0, 2.0, 1.0, f64::NAN, 3.0, 1.0, 2.0, 0.0,
                f64::NEG_INFINITY, 1.0, f64::INFINITY, 3.0, 7.0, f64::NEG_INFINITY,
            ],
            &[
                5, 13, 14, 31, 33, 38, 48, 53, 4, 7, 17, 24, 26, 27, 20, 28, 29, 40, 47, 12, 34, 35, 42, 45,
                49, 9, 32, 41, 46, 2, 10, 11, 16, 37, 39, 44, 51, 0, 8, 15, 22, 23, 25, 1, 3, 6, 18, 19,
                30, 52, 21, 36, 43, 50,
            ],
        ),
        (
            &[
                3.0, 1.0, 7.0, 0.0, f64::NEG_INFINITY, 2.5, f64::NAN, f64::NEG_INFINITY, 7.0, f64::NEG_INFINITY, 1.0,
            ],
            &[
                4, 7, 9, 3, 1, 10, 5, 0, 2, 6, 8,
            ],
        ),
        (
            &[
                -1.0, f64::NAN, 2.5, f64::NAN, f64::NEG_INFINITY, 3.0, 3.0, 0.0, 1.0, f64::NEG_INFINITY, 7.0, f64::NEG_INFINITY, -1.0, 3.0, 2.5, 0.0,
                1.0, f64::NAN, -1.0, f64::INFINITY, -1.0, f64::NAN, -1.0, 3.0, 2.5, 0.0, 1.0, 2.0, 2.5, 2.5, -1.0, -1.0,
                0.0, 1.0, 3.0, f64::NAN, f64::NEG_INFINITY, 3.0, -1.0, 3.0, 3.0, 3.0, 2.0, 1.0, 0.0, 7.0, -1.0, -1.0,
                -1.0, f64::INFINITY, -1.0, -1.0, 2.5, 1.0, f64::INFINITY, f64::INFINITY, 7.0,
            ],
            &[
                0, 1, 2, 3, 4, 9, 11, 36, 12, 18, 20, 22, 30, 31, 38, 46, 47, 48, 50, 51, 7, 15, 25, 32,
                44, 8, 16, 26, 33, 43, 53, 27, 42, 14, 24, 28, 29, 52, 5, 6, 13, 41, 10, 17, 19, 21, 23, 34,
                35, 37, 39, 40, 45, 56, 49, 54, 55,
            ],
        ),
        (
            &[
                7.0, 2.0, -1.0, f64::NAN, 3.0, 2.5, 0.0, 2.5, f64::NAN, f64::INFINITY, 7.0, -1.0, 0.0, -1.0,
            ],
            &[
                2, 11, 13, 6, 12, 1, 5, 7, 4, 0, 3, 8, 10, 9,
            ],
        ),
        (
            &[
                f64::INFINITY, f64::NAN, 0.0, f64::NAN, 1.0, f64::INFINITY, f64::INFINITY, -1.0, 2.5, 2.5, -1.0, f64::NEG_INFINITY, 2.0, 7.0, 0.0, 1.0,
                f64::NAN, 3.0, 3.0, 3.0, 0.0, 2.5, 7.0, 7.0, 2.5, -1.0, -1.0, 2.0, 1.0, 2.0, 1.0,
            ],
            &[
                0, 1, 11, 10, 2, 3, 7, 25, 26, 14, 20, 4, 15, 28, 30, 12, 27, 29, 8, 9, 21, 24, 17, 18,
                19, 13, 22, 23, 5, 6, 16,
            ],
        ),
        (
            &[
                f64::INFINITY, 2.0, 3.0, 2.0, 1.0, f64::INFINITY, 2.5, 0.0, 2.5, -1.0, -1.0, -1.0, 0.0, f64::NAN, f64::NAN, f64::NEG_INFINITY,
                f64::INFINITY, f64::INFINITY, 3.0, 0.0, f64::NEG_INFINITY, -1.0, 3.0, 1.0, f64::INFINITY, 0.0, 7.0, f64::INFINITY, 3.0, 2.0, 0.0, -1.0,
                0.0, 7.0, f64::NAN, -1.0, 0.0, 3.0, f64::NAN, 3.0,
            ],
            &[
                15, 20, 9, 10, 11, 21, 31, 35, 7, 12, 19, 25, 30, 32, 36, 4, 23, 1, 3, 29, 6, 8, 2, 0,
                5, 13, 14, 18, 22, 28, 37, 39, 26, 33, 16, 17, 24, 27, 34, 38,
            ],
        ),
        (
            &[
                f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, -1.0, f64::INFINITY, 2.0, 7.0, 0.0, 7.0, 0.0, 0.0, 3.0, f64::INFINITY, 2.5, 1.0, f64::NEG_INFINITY,
                f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, 2.5, f64::INFINITY, 1.0, 2.5, 0.0, f64::NEG_INFINITY, f64::NAN,
            ],
            &[
                2, 15, 16, 17, 24, 3, 7, 9, 10, 18, 23, 14, 21, 5, 13, 19, 22, 11, 6, 8, 0, 1, 4, 12,
                20, 25,
            ],
        ),
        (
            &[
                0.0, 7.0, 2.5, 0.0, 2.5, f64::NEG_INFINITY, -1.0, 1.0, 0.0, 0.0, -1.0, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NAN, f64::NEG_INFINITY,
                2.5, -1.0, 1.0, 0.0, f64::NEG_INFINITY, 2.5, 0.0, f64::NEG_INFINITY, 3.0, f64::NEG_INFINITY, 2.5, 0.0, f64::INFINITY, -1.0, 0.0, f64::NAN,
                -1.0, 2.5, f64::NAN, 2.0, f64::INFINITY, 2.0, 0.0, 7.0, 1.0, 2.0, f64::NAN, 1.0, 2.5, 7.0, f64::NAN, f64::NEG_INFINITY,
                f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY, 7.0, 3.0, f64::NAN, f64::NEG_INFINITY, 3.0, 3.0, 2.5,
            ],
            &[
                5, 11, 12, 15, 20, 23, 25, 47, 48, 49, 50, 55, 6, 10, 17, 29, 32, 0, 3, 8, 9, 19, 22, 27,
                30, 38, 7, 18, 40, 43, 35, 37, 41, 2, 4, 16, 21, 26, 33, 58, 24, 53, 56, 57, 1, 13, 14, 28,
                31, 34, 44, 39, 45, 36, 42, 46, 52, 51, 54,
            ],
        ),
        (
            &[
                f64::NAN, 2.5, 7.0, 1.0, -1.0, -1.0, 7.0, 3.0, f64::NEG_INFINITY, f64::INFINITY, f64::INFINITY, 2.5, 1.0, f64::NEG_INFINITY, 7.0, 3.0,
                1.0, 0.0, 1.0, 2.5, 3.0, 3.0, 3.0, 3.0, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY, 2.5, 7.0, -1.0, 7.0, 0.0,
                1.0, 0.0, f64::NAN, 7.0, -1.0, f64::NAN, 3.0, f64::NEG_INFINITY, 3.0, 7.0, 7.0, 1.0, f64::NEG_INFINITY, 1.0, f64::INFINITY, f64::NAN,
                -1.0,
            ],
            &[
                0, 8, 13, 24, 26, 39, 44, 4, 5, 29, 36, 48, 17, 31, 33, 3, 12, 16, 18, 32, 43, 45, 1, 11,
                19, 27, 7, 15, 20, 21, 22, 23, 38, 40, 2, 6, 14, 28, 30, 35, 41, 42, 9, 10, 25, 34, 37, 46,
                47,
            ],
        ),
        (
            &[
                f64::INFINITY, f64::NEG_INFINITY, -1.0, 2.5, f64::NEG_INFINITY, 3.0, f64::NAN, 2.5, 0.0, 1.0, 7.0, -1.0, f64::NEG_INFINITY, -1.0,
            ],
            &[
                1, 4, 12, 2, 11, 13, 8, 9, 3, 7, 5, 10, 0, 6,
            ],
        ),
        (
            &[
                1.0, 2.5, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY, 7.0, 0.0, f64::NAN, f64::NEG_INFINITY, 3.0, -1.0, 0.0, f64::NEG_INFINITY, f64::NAN, 7.0, 2.5,
                2.0, f64::NAN, 1.0, 0.0, f64::NAN, 2.5, f64::NAN, 2.0, 0.0, 0.0, f64::NEG_INFINITY, 0.0, -1.0, 2.5,
            ],
            &[
                2, 3, 8, 12, 26, 10, 28, 6, 11, 19, 24, 25, 27, 0, 18, 16, 23, 1, 15, 21, 29, 9, 5, 14,
                4, 7, 13, 17, 20, 22,
            ],
        ),
    ];

    #[test]
    fn js_sort_by_orders_nan_keys_as_v8_does() {
        for (keys, expected) in V8_NAN_ORDERS {
            let mut items: Vec<(f64, usize)> = keys.iter().copied().zip(0..).collect();
            js_sort_by(&mut items, |a, b| a.0 - b.0);
            let ids: Vec<usize> = items.iter().map(|item| item.1).collect();
            assert_eq!(&ids, expected, "keys {keys:?}");
        }
    }

    /// With a consistent comparator, the order is the stable order, as `slice::sort_by` gives it.
    #[test]
    fn js_sort_by_is_stable_for_a_consistent_comparator() {
        let items: Vec<(f64, usize)> = (0..90).map(|i| (((i * 37) % 11) as f64, i)).collect();
        let mut expected = items.clone();
        expected.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut actual = items;
        js_sort_by(&mut actual, |a, b| a.0 - b.0);
        assert_eq!(actual, expected);
    }

    /// A NaN key makes the comparator inconsistent, and the sort still returns a permutation without a panic.
    #[test]
    fn js_sort_by_returns_a_permutation_on_long_nan_input() {
        let items: Vec<(f64, usize)> = (0..300)
            .map(|i| {
                (
                    if i % 7 == 3 {
                        f64::NAN
                    } else {
                        (i % 13) as f64
                    },
                    i,
                )
            })
            .collect();
        let mut actual = items.clone();
        js_sort_by(&mut actual, |a, b| a.0 - b.0);
        let mut ids: Vec<usize> = actual.iter().map(|item| item.1).collect();
        ids.sort_unstable();
        assert_eq!(ids, (0..300).collect::<Vec<usize>>());
    }
}
