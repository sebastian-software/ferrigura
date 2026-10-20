//! `straightenCollinearSiblingDetours` (`direction/siblingSharedFaceRouting.ts`, lines 47-197): the co-route rescue.
//! A four-point edge whose ends share a column or a row, and whose detour is around a blocker, becomes the straight
//! two-point line between the two faces, when that line is clear of the nodes and of the other edges. The line is
//! shifted along its face by 4, or by the half-extent of a label on a sibling edge plus 3, to keep the label off it.
//!
//! The edges are visited in order, and each edge sees the routes of the edges before it.

use std::collections::HashMap;

use indexmap::IndexMap;

use super::geometry::{
    NodeBoundsInfo, RectEntry, classify_three_segment_route, collect_real_node_bounds,
    get_node_pair_geometry, segment_conflicts_with_any_edge, segment_hits_any_rect, truthy_text,
};
use crate::model::{Edge, LayoutData, Node, Point};

const EPS: f64 = 1e-6;
const MIN_PORT_SPACING: f64 = 8.0;
const PORT_SHIFT: f64 = MIN_PORT_SPACING / 2.0;
const LABEL_CLEARANCE_BUFFER: f64 = 3.0;

/// The width and the height of an edge label node (`labelDimById`).
#[derive(Debug, Clone, Copy)]
struct LabelDim {
    w: f64,
    h: f64,
}

/// The axis along which a rescued line is shifted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

/// `pairKey(a, b)`: the key of an unordered pair of node ids, as the TypeScript builds it.
fn pair_key(a: &str, b: &str) -> String {
    if a < b {
        format!("{a}::{b}")
    } else {
        format!("{b}::{a}")
    }
}

/// `labelDimById`: the label nodes of the layout, by id, with their sizes. Groups are skipped, and a repeated id
/// keeps its last size.
fn label_dims_by_id(nodes: &[Node]) -> HashMap<String, LabelDim> {
    let mut dims = HashMap::new();
    for node in nodes {
        if node.is_group == Some(true) {
            continue;
        }
        if node.is_edge_label == Some(true) {
            dims.insert(
                node.id.clone(),
                LabelDim {
                    w: node.width.unwrap_or(0.0),
                    h: node.height.unwrap_or(0.0),
                },
            );
        }
    }
    dims
}

/// `consider(labelId)` of `labelClearanceFor`: raises the largest half-extent along `axis` to that of the label.
fn consider(
    label_id: Option<&str>,
    dims: &HashMap<String, LabelDim>,
    axis: Axis,
    max_half: &mut f64,
) {
    let Some(dim) = truthy_text(label_id).and_then(|id| dims.get(id)) else {
        return;
    };
    let half = match axis {
        Axis::X => dim.w / 2.0,
        Axis::Y => dim.h / 2.0,
    };
    if half > *max_half {
        *max_half = half;
    }
}

/// `labelClearanceFor(thisEdge, srcId, dstId, axis)`: the shift that clears the labels of this edge and of the
/// siblings that join the same pair of nodes, or 0 when none of them has a label.
fn label_clearance_for(
    edges: &[Edge],
    dims: &HashMap<String, LabelDim>,
    this: usize,
    src_id: &str,
    dst_id: &str,
    axis: Axis,
) -> f64 {
    let target_pair = pair_key(src_id, dst_id);
    let mut max_half = 0.0;
    consider(
        edges[this].label_node_id.as_deref(),
        dims,
        axis,
        &mut max_half,
    );
    for (index, other) in edges.iter().enumerate() {
        if index == this || other.is_layout_only == Some(true) {
            continue;
        }
        let (Some(other_src), Some(other_dst)) = (
            truthy_text(other.start.as_deref()),
            truthy_text(other.end.as_deref()),
        ) else {
            continue;
        };
        if pair_key(other_src, other_dst) != target_pair {
            continue;
        }
        consider(other.label_node_id.as_deref(), dims, axis, &mut max_half);
    }
    if max_half > 0.0 {
        max_half + LABEL_CLEARANCE_BUFFER
    } else {
        0.0
    }
}

/// The new polyline of the edge at `index`, when its detour can be straightened, and `None` otherwise.
fn straightened(
    edges: &[Edge],
    index: usize,
    node_info_by_id: &IndexMap<String, NodeBoundsInfo>,
    real_node_rects: &[RectEntry],
    dims: &HashMap<String, LabelDim>,
) -> Option<Vec<Point>> {
    let edge = &edges[index];
    if edge.is_layout_only == Some(true) {
        return None;
    }
    classify_three_segment_route(edge.points.as_deref(), EPS)?;

    let pair = get_node_pair_geometry(edge, node_info_by_id, EPS)?;
    if pair.collinear_x == pair.collinear_y {
        return None;
    }
    let src_id = truthy_text(edge.start.as_deref())?;
    let dst_id = truthy_text(edge.end.as_deref())?;
    let (src, dst) = (&pair.src_info, &pair.dst_info);

    let (target_src, target_dst) = if pair.collinear_x {
        let dst_below = dst.cy > src.cy;
        (
            Point {
                x: src.cx,
                y: if dst_below {
                    src.rect.bottom
                } else {
                    src.rect.top
                },
            },
            Point {
                x: dst.cx,
                y: if dst_below {
                    dst.rect.top
                } else {
                    dst.rect.bottom
                },
            },
        )
    } else {
        let dst_east = dst.cx > src.cx;
        (
            Point {
                x: if dst_east {
                    src.rect.right
                } else {
                    src.rect.left
                },
                y: src.cy,
            },
            Point {
                x: if dst_east {
                    dst.rect.left
                } else {
                    dst.rect.right
                },
                y: dst.cy,
            },
        )
    };

    if segment_hits_any_rect(
        target_src,
        target_dst,
        real_node_rects,
        &[src_id, dst_id],
        1.0,
    ) {
        return None;
    }

    // The rescued line moves perpendicular to itself: a vertical line shifts in x, by its label's width.
    let shift_axis = if pair.collinear_x { Axis::X } else { Axis::Y };
    let label_shift = label_clearance_for(edges, dims, index, src_id, dst_id, shift_axis);
    let effective_shift = if label_shift > PORT_SHIFT {
        label_shift
    } else {
        PORT_SHIFT
    };
    for delta in [0.0, effective_shift, -effective_shift] {
        let mut shifted_src = target_src;
        let mut shifted_dst = target_dst;
        if pair.collinear_x {
            shifted_src.x += delta;
            shifted_dst.x += delta;
            if shifted_src.x <= src.rect.left || shifted_src.x >= src.rect.right {
                continue;
            }
            if shifted_dst.x <= dst.rect.left || shifted_dst.x >= dst.rect.right {
                continue;
            }
        } else {
            shifted_src.y += delta;
            shifted_dst.y += delta;
            if shifted_src.y <= src.rect.top || shifted_src.y >= src.rect.bottom {
                continue;
            }
            if shifted_dst.y <= dst.rect.top || shifted_dst.y >= dst.rect.bottom {
                continue;
            }
        }

        if segment_hits_any_rect(
            shifted_src,
            shifted_dst,
            real_node_rects,
            &[src_id, dst_id],
            1.0,
        ) {
            continue;
        }
        if segment_conflicts_with_any_edge(shifted_src, shifted_dst, edges, Some(index), EPS, false)
        {
            continue;
        }
        return Some(vec![shifted_src, shifted_dst]);
    }
    None
}

/// `straightenCollinearSiblingDetours(edges, nodes)`.
pub(crate) fn straighten_collinear_sibling_detours(data: &mut LayoutData) {
    let LayoutData { nodes, edges, .. } = data;
    let nodes: &[Node] = nodes;
    let (node_info_by_id, real_node_rects) = collect_real_node_bounds(nodes);
    let dims = label_dims_by_id(nodes);
    for index in 0..edges.len() {
        if let Some(points) = straightened(edges, index, &node_info_by_id, &real_node_rects, &dims)
        {
            edges[index].points = Some(points);
        }
    }
}
