//! `portSwapToLShape` (`direction/portSwap.ts`, lines 107-240): the port swap of a four-point edge to a three-point
//! L-shape. When the source port of a H-V-H or V-H-V route leaves the node parallel to its first segment, the port
//! moves to the perpendicular face, and the route becomes one bend shorter, if the new first segment is clear of the
//! nodes and of the other edges. The destination port is kept.
//!
//! The port is moved by the first of 0, 8, -8, 16 and -16 that passes the guards, and each edge sees the routes of the
//! edges before it. The swap is taken per edge in the order of the edges.

use indexmap::IndexMap;

use super::geometry_stopgap::{
    NodeBounds, RectEntry, RouteKind, classify_three_segment_route, collect_real_node_bounds,
    dedupe_consecutive_points, get_node_pair_geometry, non_empty, same_point,
    segment_conflicts_with_any_edge, segment_hits_any_rect,
};
use crate::model::{Edge, LayoutData, Node, Point};

const EPS: f64 = 1e-6;
const MIN_PORT_SPACING: f64 = 8.0;
const PORT_SHIFT: f64 = MIN_PORT_SPACING;
const TRY_DELTAS: [f64; 5] = [
    0.0,
    PORT_SHIFT,
    -PORT_SHIFT,
    2.0 * PORT_SHIFT,
    -2.0 * PORT_SHIFT,
];

/// The new polyline of the edge at `index`, when its source port can be swapped, and `None` otherwise.
fn swapped(
    edges: &[Edge],
    index: usize,
    node_info_by_id: &IndexMap<String, NodeBounds>,
    real_node_rects: &[RectEntry],
) -> Option<Vec<Point>> {
    let edge = &edges[index];
    if edge.is_layout_only == Some(true) {
        return None;
    }
    let pts = edge.points.as_deref()?;
    if pts.len() < 4 {
        return None;
    }

    let deduped = dedupe_consecutive_points(pts, EPS);
    let route = classify_three_segment_route(Some(&deduped), EPS)?;
    let p3 = route.p3;
    let is_hvh = route.kind == RouteKind::Hvh;

    let pair = get_node_pair_geometry(edge, node_info_by_id, EPS)?;
    // A collinear pair belongs to the co-route rescue.
    if pair.collinear_x || pair.collinear_y {
        return None;
    }
    let src_id = non_empty(edge.start.as_deref())?;
    let dst_id = non_empty(edge.end.as_deref())?;
    let (src, dst) = (pair.src, pair.dst);
    let src_rect = src.rect;

    for delta in TRY_DELTAS {
        let (np0, np1, np2) = if is_hvh {
            let dst_below = dst.cy > src.cy;
            let new_src_y = if dst_below {
                src_rect.bottom
            } else {
                src_rect.top
            };
            let new_src_x = src.cx + delta;
            // The new port lies strictly inside the face, away from its corners.
            if new_src_x <= src_rect.left + EPS || new_src_x >= src_rect.right - EPS {
                continue;
            }
            (
                Point {
                    x: new_src_x,
                    y: new_src_y,
                },
                Point {
                    x: new_src_x,
                    y: p3.y,
                },
                Point { x: p3.x, y: p3.y },
            )
        } else {
            let dst_east = dst.cx > src.cx;
            let new_src_x = if dst_east {
                src_rect.right
            } else {
                src_rect.left
            };
            let new_src_y = src.cy + delta;
            if new_src_y <= src_rect.top + EPS || new_src_y >= src_rect.bottom - EPS {
                continue;
            }
            (
                Point {
                    x: new_src_x,
                    y: new_src_y,
                },
                Point {
                    x: p3.x,
                    y: new_src_y,
                },
                Point { x: p3.x, y: p3.y },
            )
        };

        // A degenerate segment collapses the L to a straight line, which is accepted.
        let first_degenerate = same_point(np0, np1, EPS);
        let second_degenerate = same_point(np1, np2, EPS);
        if first_degenerate && second_degenerate {
            continue;
        }

        // The first segment may touch the source node, and the second the destination node.
        if !first_degenerate && segment_hits_any_rect(np0, np1, real_node_rects, &[src_id], 1.0) {
            continue;
        }
        if !second_degenerate && segment_hits_any_rect(np1, np2, real_node_rects, &[dst_id], 1.0) {
            continue;
        }

        let first_conflicts = !first_degenerate
            && segment_conflicts_with_any_edge(np0, np1, edges, Some(index), EPS, true);
        let second_conflicts = !second_degenerate
            && segment_conflicts_with_any_edge(np1, np2, edges, Some(index), EPS, true);
        if first_conflicts || second_conflicts {
            continue;
        }

        return Some(if first_degenerate {
            vec![np1, np2]
        } else if second_degenerate {
            vec![np0, np1]
        } else {
            vec![np0, np1, np2]
        });
    }
    None
}

/// `portSwapToLShape(edges, nodes)`.
pub(crate) fn port_swap_to_l_shape(data: &mut LayoutData) {
    let LayoutData { nodes, edges, .. } = data;
    let nodes: &[Node] = nodes;
    let (node_info_by_id, real_node_rects) = collect_real_node_bounds(nodes);
    for index in 0..edges.len() {
        if let Some(points) = swapped(edges, index, &node_info_by_id, &real_node_rects) {
            edges[index].points = Some(points);
        }
    }
}
