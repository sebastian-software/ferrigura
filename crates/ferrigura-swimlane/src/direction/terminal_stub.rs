//! `collapseShortTerminalStub` (`direction/terminalStub.ts`, lines 44-259): the short last stub of an edge is dropped
//! by moving the destination face. When the last segment of a route is shorter than 10 and perpendicular to the one
//! before it, the route ends at the face of the destination on the approach axis, and the bend before the stub is
//! moved onto the centre of the destination on the other axis. The rewrite is kept when its new segments are clear of
//! the nodes, of the labels and of the other edges, and then the label of the edge moves to the middle of its longest
//! segment that can hold it.
//!
//! The edges are visited in order, and each sees the nodes as the edges before it left them.

use std::collections::HashSet;

use indexmap::IndexMap;

use super::geometry_stopgap::{
    RectEntry, collect_node_rect_entries, dedupe_consecutive_points, is_horizontal_segment,
    is_vertical_segment, non_empty, orthogonal_segments_strictly_cross, point_inside_rect,
    rect_of_node_bounds, same_x, same_y, segment_hits_any_rect,
};
use super::node_by_id_map;
use crate::js::to_fixed;
use crate::model::{Edge, LayoutData, Node, Point};

const MIN_STUB: f64 = 10.0;
const EPS_LOCAL: f64 = 1e-3;
const BUFFER: f64 = 2.0;

/// The rewrite of one edge: its new polyline, and the new centre of its label node, as `(index, x, y)`.
struct Collapse {
    points: Vec<Point>,
    label: Option<(usize, f64, f64)>,
}

/// The key of a segment in `segmentCrossesOtherEdge`'s set: each coordinate written with `toFixed(3)`.
fn segment_key(a: Point, b: Point) -> String {
    format!(
        "{},{}|{},{}",
        to_fixed(a.x, 3),
        to_fixed(a.y, 3),
        to_fixed(b.x, 3),
        to_fixed(b.y, 3)
    )
}

/// `segmentCrossesOtherEdge(from, to)`: whether the segment crosses a segment of another routed edge, other than the
/// segments of this edge (`own`).
fn crosses_other_edge(
    edges: &[Edge],
    index: usize,
    own: &HashSet<String>,
    from: Point,
    to: Point,
) -> bool {
    for (other_index, other) in edges.iter().enumerate() {
        if other_index == index || other.is_layout_only == Some(true) {
            continue;
        }
        let Some(points) = other.points.as_deref() else {
            continue;
        };
        if points.len() < 2 {
            continue;
        }
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if own.contains(&segment_key(a, b)) {
                continue;
            }
            if orthogonal_segments_strictly_cross(from, to, a, b, EPS_LOCAL) {
                return true;
            }
        }
    }
    false
}

/// The rewrite of the edge at `index`, when its last stub is collapsed, and `None` otherwise.
fn collapsed(
    edges: &[Edge],
    index: usize,
    nodes: &[Node],
    node_by_id: &IndexMap<String, usize>,
    real_node_rects: &[RectEntry],
    label_rects: &[RectEntry],
) -> Option<Collapse> {
    let edge = &edges[index];
    if edge.is_layout_only == Some(true) {
        return None;
    }
    let raw = edge.points.as_deref()?;
    if raw.len() < 4 {
        return None;
    }

    // Dedupe the consecutive equal points, so that the last segment is measured as it is drawn.
    let pts = dedupe_consecutive_points(raw, EPS_LOCAL);
    if pts.len() < 4 {
        return None;
    }
    let n_last = pts.len() - 1;
    let end_pt = pts[n_last];
    let penult_pt = pts[n_last - 1];
    let prev_pt = pts[n_last - 2];

    // The last segment must be short, and not degenerate.
    let last_dx = end_pt.x - penult_pt.x;
    let last_dy = end_pt.y - penult_pt.y;
    let last_len = ferrigura_jsmath::hypot(last_dx, last_dy);
    if last_len >= MIN_STUB || last_len < EPS_LOCAL {
        return None;
    }

    // The penultimate segment must not be degenerate, and it must be perpendicular to the last one.
    let penult_dx = penult_pt.x - prev_pt.x;
    let penult_dy = penult_pt.y - prev_pt.y;
    let penult_len = ferrigura_jsmath::hypot(penult_dx, penult_dy);
    if penult_len < EPS_LOCAL {
        return None;
    }
    let last_is_horiz = is_horizontal_segment(penult_pt, end_pt, EPS_LOCAL);
    let last_is_vert = is_vertical_segment(penult_pt, end_pt, EPS_LOCAL);
    let penult_is_horiz = is_horizontal_segment(prev_pt, penult_pt, EPS_LOCAL);
    let penult_is_vert = is_vertical_segment(prev_pt, penult_pt, EPS_LOCAL);
    if !((last_is_horiz && penult_is_vert) || (last_is_vert && penult_is_horiz)) {
        return None;
    }

    let dst_id = non_empty(edge.end.as_deref());
    let src_id = non_empty(edge.start.as_deref());
    let dst = dst_id
        .and_then(|id| node_by_id.get(id))
        .map(|&node| &nodes[node])?;
    let dst_cx = dst.x.unwrap_or(0.0);
    let dst_cy = dst.y.unwrap_or(0.0);
    let dst_rect = rect_of_node_bounds(dst)?;

    // The new prev' keeps prev's far coordinate, and the new end is the face of the destination on the approach axis,
    // opposite to the approach.
    let (new_prev, new_end) = if penult_is_vert {
        let approach_from_below = penult_dy < 0.0;
        (
            Point {
                x: dst_cx,
                y: prev_pt.y,
            },
            Point {
                x: dst_cx,
                y: if approach_from_below {
                    dst_rect.bottom
                } else {
                    dst_rect.top
                },
            },
        )
    } else {
        let approach_from_left = penult_dx > 0.0;
        (
            Point {
                x: prev_pt.x,
                y: dst_cy,
            },
            Point {
                x: if approach_from_left {
                    dst_rect.right
                } else {
                    dst_rect.left
                },
                y: dst_cy,
            },
        )
    };

    // The new approach must not enter a node other than the destination, nor a label.
    let dst_ids: Vec<&str> = dst_id.into_iter().collect();
    if segment_hits_any_rect(new_prev, new_end, real_node_rects, &dst_ids, -BUFFER) {
        return None;
    }
    if segment_hits_any_rect(new_prev, new_end, label_rects, &[], -BUFFER) {
        return None;
    }
    // The new prev' must not lie inside the source node.
    if let Some(src_rect) = src_id
        .and_then(|id| node_by_id.get(id))
        .and_then(|&node| rect_of_node_bounds(&nodes[node]))
        && point_inside_rect(new_prev, src_rect, BUFFER)
    {
        return None;
    }

    let own: HashSet<String> = (0..n_last)
        .map(|i| segment_key(pts[i], pts[i + 1]))
        .collect();
    if crosses_other_edge(edges, index, &own, new_prev, new_end) {
        return None;
    }

    // The segment before prev moves its end to prev', and must stay clear as well. n_last is at least 3.
    let before_prev = pts[n_last - 3];
    let endpoint_ids: Vec<&str> = [src_id, dst_id].into_iter().flatten().collect();
    if segment_hits_any_rect(
        before_prev,
        new_prev,
        real_node_rects,
        &endpoint_ids,
        -BUFFER,
    ) {
        return None;
    }
    if crosses_other_edge(edges, index, &own, before_prev, new_prev) {
        return None;
    }

    let mut points: Vec<Point> = pts[..n_last - 2].to_vec();
    points.push(new_prev);
    points.push(new_end);

    // The label moves to the middle of the longest axis-aligned segment that holds it, when the label has a size.
    let label = non_empty(edge.label_node_id.as_deref())
        .and_then(|id| node_by_id.get(id))
        .and_then(|&label_index| {
            let label_node = &nodes[label_index];
            let lw = label_node.width.unwrap_or(0.0);
            let lh = label_node.height.unwrap_or(0.0);
            if !(lw > 0.0 && lh > 0.0) {
                return None;
            }
            let mut best: Option<(f64, f64)> = None;
            let mut best_len = -1.0;
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let seg_len = ferrigura_jsmath::hypot(b.x - a.x, b.y - a.y);
                let is_horiz = same_y(a, b, EPS_LOCAL);
                let is_vert = same_x(a, b, EPS_LOCAL);
                let fits = (is_horiz && seg_len >= lw + 2.0) || (is_vert && seg_len >= lh + 2.0);
                if !fits {
                    continue;
                }
                if seg_len > best_len {
                    best_len = seg_len;
                    best = Some(((a.x + b.x) / 2.0, (a.y + b.y) / 2.0));
                }
            }
            best.map(|(x, y)| (label_index, x, y))
        });

    Some(Collapse { points, label })
}

/// `collapseShortTerminalStub(edges, nodeByIdMap)`.
pub(crate) fn collapse_short_terminal_stub(data: &mut LayoutData) {
    let LayoutData { nodes, edges, .. } = data;
    let node_by_id = node_by_id_map(nodes.as_slice());
    let (real_node_rects, label_rects) =
        collect_node_rect_entries(node_by_id.values().map(|&index| &nodes[index]));
    for index in 0..edges.len() {
        let update = collapsed(
            edges,
            index,
            nodes.as_slice(),
            &node_by_id,
            &real_node_rects,
            &label_rects,
        );
        if let Some(update) = update {
            edges[index].points = Some(update.points);
            if let Some((label, x, y)) = update.label {
                nodes[label].x = Some(x);
                nodes[label].y = Some(y);
            }
        }
    }
}
