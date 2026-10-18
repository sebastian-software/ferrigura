//! Temporary: the primitives of `direction/geometry.ts` that the passes of this branch call, with the same operations
//! and the same defaults.
//!
//! The geometry helper (branch `sebastian/port-swimlanes-geom`) ports the module as `direction/geometry.rs`. When that
//! branch is merged, the passes import from there and this file is deleted. Until then, every function here is
//! checked by the corpus of the passes that call it.
//!
//! The TypeScript default tolerance is [`EPS`]. Every function takes its tolerance as an argument, and the callers
//! pass the value the TypeScript passes.

use indexmap::IndexMap;

use crate::math::{js_max, js_min};
use crate::model::{Edge, Node, Point, Rect};

/// `EPS` of `geometry.ts`: the default tolerance of the helpers.
pub(crate) const EPS: f64 = 1e-3;

/// The centre and the bounds of a node (`NodeBoundsInfo`, without its id).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NodeBounds {
    pub(crate) cx: f64,
    pub(crate) cy: f64,
    pub(crate) rect: Rect,
}

/// A node id with its rectangle (`RectEntry`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RectEntry {
    pub(crate) id: String,
    pub(crate) rect: Rect,
}

/// A horizontal or vertical segment of a polyline (`OrthogonalSegment`): the index of its first point and its ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OrthogonalSegment {
    pub(crate) index: usize,
    pub(crate) a: Point,
    pub(crate) b: Point,
    pub(crate) horizontal: bool,
    pub(crate) vertical: bool,
}

/// `id || undefined`: the id, unless it is absent or empty. JavaScript treats the empty string as false.
pub(crate) fn non_empty(id: Option<&str>) -> Option<&str> {
    id.filter(|id| !id.is_empty())
}

/// `samePoint(a, b, epsilon)`.
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

/// `isHorizontalSegment(a, b, epsilon)`.
pub(crate) fn is_horizontal_segment(a: Point, b: Point, epsilon: f64) -> bool {
    same_y(a, b, epsilon) && (a.x - b.x).abs() > epsilon
}

/// `isVerticalSegment(a, b, epsilon)`.
pub(crate) fn is_vertical_segment(a: Point, b: Point, epsilon: f64) -> bool {
    same_x(a, b, epsilon) && (a.y - b.y).abs() > epsilon
}

/// `overlapLength(a1, a2, b1, b2)`: the length of the common part of two ranges, never negative.
pub(crate) fn overlap_length(a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    js_max(
        0.0,
        js_min(js_max(a1, a2), js_max(b1, b2)) - js_max(js_min(a1, a2), js_min(b1, b2)),
    )
}

/// `orthogonalSegmentsForPoints(points, epsilon)`: the segments of the polyline that are horizontal or vertical.
pub(crate) fn orthogonal_segments_for_points(
    points: &[Point],
    epsilon: f64,
) -> Vec<OrthogonalSegment> {
    let mut result = Vec::new();
    for index in 0..points.len().saturating_sub(1) {
        let a = points[index];
        let b = points[index + 1];
        let horizontal = is_horizontal_segment(a, b, epsilon);
        let vertical = is_vertical_segment(a, b, epsilon);
        if horizontal || vertical {
            result.push(OrthogonalSegment {
                index,
                a,
                b,
                horizontal,
                vertical,
            });
        }
    }
    result
}

/// `dedupeConsecutivePoints(points, epsilon)`: a copy of the polyline without a point that equals the one before it.
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

/// `rectFromCenterSize(cx, cy, width, height)`.
pub(crate) fn rect_from_center_size(cx: f64, cy: f64, width: f64, height: f64) -> Rect {
    Rect {
        left: cx - width / 2.0,
        right: cx + width / 2.0,
        top: cy - height / 2.0,
        bottom: cy + height / 2.0,
    }
}

/// `measuredNodeRect(node)`: the centre and the rectangle of a node with a positive size, and `None` otherwise.
fn measured_node_rect(node: &Node) -> Option<NodeBounds> {
    let cx = node.x.unwrap_or(0.0);
    let cy = node.y.unwrap_or(0.0);
    let width = node.width.unwrap_or(0.0);
    let height = node.height.unwrap_or(0.0);
    if width > 0.0 && height > 0.0 {
        Some(NodeBounds {
            cx,
            cy,
            rect: rect_from_center_size(cx, cy, width, height),
        })
    } else {
        None
    }
}

/// `nodeBoundsInfoFor(node)`: the id and the bounds of a measured node that is not a group.
fn node_bounds_info_for(node: &Node) -> Option<(String, NodeBounds)> {
    if node.is_group == Some(true) {
        return None;
    }
    measured_node_rect(node).map(|bounds| (node.id.clone(), bounds))
}

/// `pointInsideRect(point, rect, buffer)`: whether the point lies strictly inside the rectangle shrunk by `buffer`.
pub(crate) fn point_inside_rect(point: Point, rect: Rect, buffer: f64) -> bool {
    point.x > rect.left + buffer
        && point.x < rect.right - buffer
        && point.y > rect.top + buffer
        && point.y < rect.bottom - buffer
}

/// `rectOfNodeBounds(node)`.
pub(crate) fn rect_of_node_bounds(node: &Node) -> Option<Rect> {
    measured_node_rect(node).map(|bounds| bounds.rect)
}

/// `collectNodeRectEntries(nodes)`: the rectangles of the measured nodes that are not edge labels, and of those that
/// are, in the order of `nodes`. The result is `(realNodeRects, labelNodeRects)`.
pub(crate) fn collect_node_rect_entries<'a>(
    nodes: impl IntoIterator<Item = &'a Node>,
) -> (Vec<RectEntry>, Vec<RectEntry>) {
    let mut real = Vec::new();
    let mut labels = Vec::new();
    for node in nodes {
        let Some((id, bounds)) = node_bounds_info_for(node) else {
            continue;
        };
        let entry = RectEntry {
            id,
            rect: bounds.rect,
        };
        if node.is_edge_label == Some(true) {
            labels.push(entry);
        } else {
            real.push(entry);
        }
    }
    (real, labels)
}

/// `segmentBoundsOverlapRect(a, b, rect, buffer)`: whether the bounding box of the segment overlaps the rectangle
/// grown by `buffer`.
fn segment_bounds_overlap_rect(a: Point, b: Point, rect: Rect, buffer: f64) -> bool {
    let seg_min_x = js_min(a.x, b.x);
    let seg_max_x = js_max(a.x, b.x);
    let seg_min_y = js_min(a.y, b.y);
    let seg_max_y = js_max(a.y, b.y);
    seg_max_x > rect.left - buffer
        && seg_min_x < rect.right + buffer
        && seg_max_y > rect.top - buffer
        && seg_min_y < rect.bottom + buffer
}

/// `segmentHitsAnyRect(a, b, rects, excludeIds, shrink)`: whether the segment's bounding box meets a rectangle whose id
/// is not excluded, with the rectangle shrunk by `shrink`.
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

/// `orthogonalSegmentsStrictlyCross(a1, b1, a2, b2, epsilon)`: whether two segments, one horizontal and one vertical,
/// cross at a point interior to both.
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

    let (horiz, vert) = if a_horiz {
        ((a1, b1), (a2, b2))
    } else {
        ((a2, b2), (a1, b1))
    };
    let h_y = horiz.0.y;
    let h_x_min = js_min(horiz.0.x, horiz.1.x);
    let h_x_max = js_max(horiz.0.x, horiz.1.x);
    let v_x = vert.0.x;
    let v_y_min = js_min(vert.0.y, vert.1.y);
    let v_y_max = js_max(vert.0.y, vert.1.y);
    v_x > h_x_min + epsilon
        && v_x < h_x_max - epsilon
        && h_y > v_y_min + epsilon
        && h_y < v_y_max - epsilon
}

/// `collectRealNodeBounds(nodes)`: the bounds of the measured nodes that are not edge labels and not groups, by id
/// (`nodeInfoById`, a `Map`: a repeated id keeps its first position and its last value), and their rectangles in
/// the order of `nodes`.
pub(crate) fn collect_real_node_bounds(
    nodes: &[Node],
) -> (IndexMap<String, NodeBounds>, Vec<RectEntry>) {
    let mut node_info_by_id = IndexMap::new();
    let mut real_node_rects = Vec::new();
    for node in nodes {
        if node.is_edge_label == Some(true) {
            continue;
        }
        let Some((id, bounds)) = node_bounds_info_for(node) else {
            continue;
        };
        node_info_by_id.insert(id.clone(), bounds);
        real_node_rects.push(RectEntry {
            id,
            rect: bounds.rect,
        });
    }
    (node_info_by_id, real_node_rects)
}

/// `NodePairGeometry` without the ids: the bounds of both ends of an edge, and whether their centres share a column
/// (`collinear_x`) or a row (`collinear_y`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NodePairGeometry {
    pub(crate) src: NodeBounds,
    pub(crate) dst: NodeBounds,
    pub(crate) collinear_x: bool,
    pub(crate) collinear_y: bool,
}

/// `getNodePairGeometry(edge, nodeInfoById, epsilon)`: absent when an end is missing, or is not a measured node.
pub(crate) fn get_node_pair_geometry(
    edge: &Edge,
    node_info_by_id: &IndexMap<String, NodeBounds>,
    epsilon: f64,
) -> Option<NodePairGeometry> {
    let src_id = non_empty(edge.start.as_deref())?;
    let dst_id = non_empty(edge.end.as_deref())?;
    let src = *node_info_by_id.get(src_id)?;
    let dst = *node_info_by_id.get(dst_id)?;
    Some(NodePairGeometry {
        src,
        dst,
        collinear_x: (src.cx - dst.cx).abs() < epsilon,
        collinear_y: (src.cy - dst.cy).abs() < epsilon,
    })
}

/// The kind of a three-segment route: `HVH` starts horizontal, `VHV` vertical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteKind {
    Hvh,
    Vhv,
}

/// `ThreeSegmentRoute`: a four-point polyline whose segments alternate between the two axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ThreeSegmentRoute {
    pub(crate) kind: RouteKind,
    pub(crate) p0: Point,
    pub(crate) p1: Point,
    pub(crate) p2: Point,
    pub(crate) p3: Point,
}

/// `classifyThreeSegmentRoute(points, epsilon)`: the route of a four-point polyline that alternates its axes, and
/// `None` for any other polyline, or for an absent one.
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
    is_vhv.then_some(ThreeSegmentRoute {
        kind: RouteKind::Vhv,
        p0,
        p1,
        p2,
        p3,
    })
}

/// `orthogonalSegmentsCross(a1, b1, a2, b2, epsilon, endpointTolerance)`: whether two axis-aligned segments on
/// different axes meet, except where they meet at an endpoint of both.
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

    let horiz = if s1h { (a1, b1) } else { (a2, b2) };
    let vert = if s1v { (a1, b1) } else { (a2, b2) };
    let h_y = horiz.0.y;
    let h_x1 = js_min(horiz.0.x, horiz.1.x);
    let h_x2 = js_max(horiz.0.x, horiz.1.x);
    let v_x = vert.0.x;
    let v_y1 = js_min(vert.0.y, vert.1.y);
    let v_y2 = js_max(vert.0.y, vert.1.y);
    if v_x < h_x1 || v_x > h_x2 || h_y < v_y1 || h_y > v_y2 {
        return false;
    }

    // Whether the crossing is at the point `p`, within the tolerance.
    let at =
        |p: Point| (v_x - p.x).abs() < endpoint_tolerance && (h_y - p.y).abs() < endpoint_tolerance;
    let matches_horiz_endpoint = at(horiz.0) || at(horiz.1);
    let matches_vert_endpoint = at(vert.0) || at(vert.1);
    !(matches_horiz_endpoint && matches_vert_endpoint)
}

/// `sameAxisSegmentsOverlap(a1, b1, a2, b2, epsilon)`: whether two segments on the same line share more than
/// `epsilon` of their length.
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

/// `segmentConflictsWithAnyEdge(a, b, edges, excludeEdge, { epsilon, skipDegenerateOther })`: whether the segment
/// crosses or overlaps a segment of another routed edge. `exclude` is the index of the edge the segment belongs to.
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
