//! Temporary: the primitives of `direction/geometry.ts` that the passes of this branch call, with the same operations
//! and the same defaults.
//!
//! The geometry helper (branch `sebastian/port-swimlanes-geom`) ports the module as `direction/geometry.rs`. When that
//! branch is merged, the passes import from there and this file is deleted. Until then, every function here is
//! checked by the corpus of the passes that call it.
//!
//! The TypeScript default tolerance is [`EPS`]. Every function takes its tolerance as an argument, and the callers
//! pass the value the TypeScript passes.

use crate::math::{js_max, js_min};
use crate::model::{Node, Point, Rect};

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
