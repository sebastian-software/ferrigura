//! The graph view of the layout, and the write-back of the phases' result into the layout data.
//!
//! Ports `helpers.ts`: `toGraphView` (lines 142-172), `writeBackToLayoutData` (174-356) with the lane width
//! solver (305-352), and `assignTopLaneTitleRect` (60-96). `prepareLayoutForSwimlanes` (98-140) is the
//! prepare step before the seam and is not ported here.
//!
//! The TypeScript `Graph` holds node objects. The port holds a [`GraphNode`] per id, with the properties
//! the phases read, and the index of the node in `LayoutData::nodes` (`None` for a dummy, which is not in
//! the layout). Those properties do not change while the phases run, so the phases read the copies, and
//! only [`write_back_to_layout_data`] touches the layout.

use indexmap::{IndexMap, IndexSet};

use crate::js::js_sort_by;
use crate::math::{js_max, js_min};
use crate::model::{LayoutData, Node, Rect};

/// A node id. Ids are ASCII (see the entry check of the core), so byte order is code-unit order.
pub type NodeId = String;

/// A node as the phases see it through `nodeById`: a node of the layout, or a dummy node of
/// `makeProperLayering`.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    /// The index of the node in `LayoutData::nodes`. `None` for a dummy.
    pub layout_index: Option<usize>,
    /// Whether `isGroup` is truthy.
    pub is_group: bool,
    /// Whether `isDummy` is truthy.
    pub is_dummy: bool,
    /// `parentId`, as the node has it. An empty string is kept, and the phases treat it as absent.
    pub parent_id: Option<String>,
    /// `width`, or `None` when the node has none (`?? 0` at the uses).
    pub width: Option<f64>,
    /// `height`, or `None` when the node has none (`?? 0` at the uses).
    pub height: Option<f64>,
}

impl GraphNode {
    fn from_layout(index: usize, node: &Node) -> GraphNode {
        GraphNode {
            layout_index: Some(index),
            is_group: node.is_group == Some(true),
            is_dummy: node.is_dummy == Some(true),
            parent_id: node.parent_id.clone(),
            width: node.width,
            height: node.height,
        }
    }

    /// The dummy node that `makeProperLayering` creates (`phase2.dummies.ts:18`).
    pub(crate) fn dummy() -> GraphNode {
        GraphNode {
            layout_index: None,
            is_group: false,
            is_dummy: true,
            parent_id: None,
            width: Some(0.0),
            height: Some(0.0),
        }
    }
}

/// The layout edge that a graph edge stands for: the `ref` of the TypeScript edge.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeOrigin {
    /// `ref.id`. The id of the layout edge, which the split chain edges share.
    pub id: String,
    /// `ref.start`, a string for every edge that `toGraphView` keeps.
    pub start: String,
    /// `ref.end`, a string for every edge that `toGraphView` keeps.
    pub end: String,
}

/// An edge of the graph view (`EdgeRef` in the TypeScript). `weight` is never set on the default path,
/// so it is not carried.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRef {
    /// `id`. A split chain edge has `<layout id>#<k>`.
    pub id: String,
    /// `src`.
    pub src: String,
    /// `dst`.
    pub dst: String,
    /// `ref`: the layout edge.
    pub origin: EdgeOrigin,
}

/// The graph of the phases: the TypeScript `Graph` without its `layout` field.
///
/// `toGraphView` keeps the layout's top-level groups in `top_level_groups`, which is the only part of
/// `layout` that the phases read (`buildTopLaneOrder`, `phase2.options.ts:57-65`).
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    /// `nodes`: the node ids in the order of the array. After `normalizeGraph` this is the order of the
    /// node map, which is the order of the layout's nodes.
    pub nodes: Vec<NodeId>,
    /// `edges`.
    pub edges: Vec<EdgeRef>,
    /// `nodeById`, in insertion order. A duplicate id keeps the position of its first node and the
    /// properties of its last, as `Map.set` does.
    pub node_by_id: IndexMap<NodeId, GraphNode>,
    /// The ids of the top-level groups of the layout, in the layout's order, duplicates included.
    pub top_level_groups: Vec<NodeId>,
}

/// The layering of the phases (`Layering`): the layers, the rank of each node, and the dummy nodes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layering {
    /// `layers`.
    pub layers: Vec<Vec<NodeId>>,
    /// `rankOf`. Groups have no rank; dummies have theirs.
    pub rank_of: IndexMap<NodeId, i64>,
    /// `dummy`: the dummy nodes.
    pub dummy: IndexSet<NodeId>,
}

/// The ordering of the phases (`OrderedLayers`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderedLayers {
    /// `layers`, the node ids of each layer in order.
    pub layers: Vec<Vec<NodeId>>,
}

/// The coordinates of the phases (`Coordinates`): the centre of each node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Coordinates {
    /// `x`.
    pub x: IndexMap<NodeId, f64>,
    /// `y`.
    pub y: IndexMap<NodeId, f64>,
}

/// The title band height of a top lane, in pixels (`TOP_LANE_TITLE_BAND_HEIGHT`, `helpers.ts:53`).
const TOP_LANE_TITLE_BAND_HEIGHT: f64 = 21.0;
/// The least horizontal padding of a top lane (`MIN_TOP_LANE_HORIZONTAL_PADDING`, `helpers.ts:54`).
const MIN_TOP_LANE_HORIZONTAL_PADDING: f64 = 20.0;
/// The padding of a group without one (`group.padding ?? 20`), and the padding of a top lane's
/// vertical margin (`helpers.ts:238`, `258`).
const DEFAULT_PADDING: f64 = 20.0;
/// The least vertical margin of the top lanes (`minHeaderMargin`, `helpers.ts:271`).
const MIN_HEADER_MARGIN: f64 = 36.0;

/// `Math.max(lane.padding ?? 20, 20)` (`topLaneHorizontalPadding`, `helpers.ts:56-58`).
fn top_lane_horizontal_padding(padding: Option<f64>) -> f64 {
    js_max(
        padding.unwrap_or(DEFAULT_PADDING),
        MIN_TOP_LANE_HORIZONTAL_PADDING,
    )
}

/// The truthy value of a string-valued property: `None` for an absent or empty one.
fn truthy(value: Option<&str>) -> Option<&str> {
    value.filter(|text| !text.is_empty())
}

/// Builds the graph view of the layout (`toGraphView`, `helpers.ts:142-172`).
///
/// The edges with a missing or empty endpoint are dropped, and so are the labelled edges (those with a
/// `labelNodeId`): their routing is carried by the layout-only virtual edges, which are kept.
pub fn to_graph_view(layout: &LayoutData) -> Graph {
    let mut node_by_id = IndexMap::new();
    for (index, node) in layout.nodes.iter().enumerate() {
        node_by_id.insert(node.id.clone(), GraphNode::from_layout(index, node));
    }

    let mut edges = Vec::new();
    for edge in &layout.edges {
        let (Some(src), Some(dst)) = (truthy(edge.start.as_deref()), truthy(edge.end.as_deref()))
        else {
            continue;
        };
        if truthy(edge.label_node_id.as_deref()).is_some() {
            continue;
        }
        edges.push(EdgeRef {
            id: edge.id.clone(),
            src: src.to_owned(),
            dst: dst.to_owned(),
            origin: EdgeOrigin {
                id: edge.id.clone(),
                start: src.to_owned(),
                end: dst.to_owned(),
            },
        });
    }

    // The groups in reverse order, then the other nodes (`helpers.ts:169-170`).
    let groups: Vec<NodeId> = layout
        .nodes
        .iter()
        .filter(|node| node.is_group == Some(true))
        .map(|node| node.id.clone())
        .rev()
        .collect();
    let others = layout
        .nodes
        .iter()
        .filter(|node| node.is_group != Some(true))
        .map(|node| node.id.clone());
    let nodes = groups.into_iter().chain(others).collect();

    let top_level_groups = layout
        .nodes
        .iter()
        .filter(|node| node.is_group == Some(true) && truthy(node.parent_id.as_deref()).is_none())
        .map(|node| node.id.clone())
        .collect();

    Graph {
        nodes,
        edges,
        node_by_id,
        top_level_groups,
    }
}

/// The top of a top lane's title band (`assignTopLaneTitleRect`, `helpers.ts:60-96`). The rectangle is
/// removed when the lane has no size or content top, or when the band would be empty.
fn assign_top_lane_title_rect(lane: &mut Node) {
    let (Some(x), Some(y), Some(width), Some(height), Some(content_top)) = (
        lane.x,
        lane.y,
        lane.width,
        lane.height,
        lane.swimlane_content_top,
    ) else {
        lane.group_title_rect = None;
        return;
    };
    let finite = [x, y, width, height, content_top]
        .iter()
        .all(|value| value.is_finite());
    if !finite || width <= 0.0 || height <= 0.0 {
        lane.group_title_rect = None;
        return;
    }

    let top = y - height / 2.0;
    let header_bottom = js_min(content_top, y + height / 2.0);
    let title_height = js_min(TOP_LANE_TITLE_BAND_HEIGHT, js_max(0.0, header_bottom - top));
    let bottom = top + title_height;
    if bottom <= top {
        lane.group_title_rect = None;
        return;
    }

    lane.group_title_rect = Some(Rect {
        left: x - width / 2.0,
        right: x + width / 2.0,
        top,
        bottom,
    });
}

/// The bounds of a group's children (`groupBounds`, `helpers.ts:206`).
#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

/// Writes the result of the phases into the layout: the layer, the order, and the centre of each node,
/// then the size and the position of each group, and the widths of the top lanes
/// (`writeBackToLayoutData`, `helpers.ts:174-356`).
///
/// Nodes of `ordered` that are not in the layout (the dummies) take part in the order index, and they
/// are not written. A node without a coordinate falls back to its order index times `node_gap` and its
/// layer index times `layer_gap`. A group takes the bounds of its children, padded.
pub fn write_back_to_layout_data(
    g: &Graph,
    layout: &mut LayoutData,
    ordered: &OrderedLayers,
    coords: &Coordinates,
    layer_gap: f64,
    node_gap: f64,
) {
    // The nodes of the layers.
    for (layer_index, layer) in ordered.layers.iter().enumerate() {
        for (order_index, id) in layer.iter().enumerate() {
            let Some(index) = g.node_by_id.get(id).and_then(|node| node.layout_index) else {
                continue;
            };
            let x = coords
                .x
                .get(id)
                .copied()
                .unwrap_or(order_index as f64 * node_gap);
            let y = coords
                .y
                .get(id)
                .copied()
                .unwrap_or(layer_index as f64 * layer_gap);
            let node = &mut layout.nodes[index];
            node.layer = Some(layer_index as f64);
            node.order = Some(order_index as f64);
            node.x = Some(x);
            node.y = Some(y);
        }
    }

    // The groups, in the order of the layout. A group's children are all the nodes whose parent it is,
    // groups included, and a child group that comes earlier in the order already has its bounds.
    let mut top_level: Vec<usize> = Vec::new();
    let mut group_bounds: IndexMap<NodeId, Bounds> = IndexMap::new();
    for group_index in 0..layout.nodes.len() {
        if layout.nodes[group_index].is_group != Some(true) {
            continue;
        }
        let group_id = layout.nodes[group_index].id.clone();
        if truthy(layout.nodes[group_index].parent_id.as_deref()).is_none() {
            top_level.push(group_index);
        }

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for child_index in 0..layout.nodes.len() {
            let child = &layout.nodes[child_index];
            if child.parent_id.as_deref() != Some(group_id.as_str()) {
                continue;
            }
            let cx = child.x.or_else(|| coords.x.get(&child.id).copied());
            let cy = child.y.or_else(|| coords.y.get(&child.id).copied());
            let cw = child.width.unwrap_or(0.0);
            let ch = child.height.unwrap_or(0.0);
            if let (Some(cx), Some(cy)) = (cx, cy) {
                min_x = js_min(min_x, cx - cw / 2.0);
                max_x = js_max(max_x, cx + cw / 2.0);
                min_y = js_min(min_y, cy - ch / 2.0);
                max_y = js_max(max_y, cy + ch / 2.0);
            }
        }

        let group = &mut layout.nodes[group_index];
        if min_x == f64::INFINITY || min_y == f64::INFINITY {
            group.x = Some(group.x.unwrap_or(0.0));
            group.y = Some(group.y.unwrap_or(0.0));
            group.width = Some(group.width.unwrap_or(0.0));
            group.height = Some(group.height.unwrap_or(0.0));
        } else {
            let pad = group.padding.unwrap_or(DEFAULT_PADDING);
            let horizontal_pad = if truthy(group.parent_id.as_deref()).is_some() {
                pad
            } else {
                2.0 * top_lane_horizontal_padding(group.padding)
            };
            let vertical_pad = pad;
            let w = js_max(0.0, max_x - min_x) + horizontal_pad;
            let h = js_max(0.0, max_y - min_y) + vertical_pad;
            group.x = Some((min_x + max_x) / 2.0);
            group.y = Some((min_y + max_y) / 2.0);
            group.width = Some(w);
            group.height = Some(h);
            group_bounds.insert(
                group_id,
                Bounds {
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                },
            );
        }
    }

    if top_level.is_empty() || group_bounds.is_empty() {
        return;
    }

    // The top lanes share their vertical extent.
    let mut global_min_y = f64::INFINITY;
    let mut global_max_y = f64::NEG_INFINITY;
    let mut max_pad = 0.0_f64;
    for &lane_index in &top_level {
        let lane = &layout.nodes[lane_index];
        let pad = lane.padding.unwrap_or(DEFAULT_PADDING);
        if pad > max_pad {
            max_pad = pad;
        }
        let Some(bounds) = group_bounds.get(&lane.id) else {
            continue;
        };
        global_min_y = js_min(global_min_y, bounds.min_y);
        global_max_y = js_max(global_max_y, bounds.max_y);
    }
    if global_min_y == f64::INFINITY || global_max_y == f64::NEG_INFINITY {
        return;
    }

    let content_height = js_max(0.0, global_max_y - global_min_y);
    let vertical_margin = js_max(max_pad, MIN_HEADER_MARGIN);
    let lane_height = content_height + 2.0 * vertical_margin;
    let center_y = (global_min_y + global_max_y) / 2.0;
    for &lane_index in &top_level {
        let lane = &mut layout.nodes[lane_index];
        lane.y = Some(center_y);
        lane.height = Some(lane_height);
        lane.swimlane_content_top = Some(global_min_y);
    }

    // The lane widths, from the centres of the lanes in order of x.
    let x_of = |index: usize| layout.nodes[index].x.unwrap_or(0.0);
    let mut sorted_lanes = top_level.clone();
    js_sort_by(&mut sorted_lanes, |a, b| x_of(a) - x_of(b));

    let mut lane_ids: Vec<NodeId> = Vec::new();
    let mut centers: Vec<f64> = Vec::new();
    let mut base_widths: Vec<f64> = Vec::new();
    for &lane_index in &sorted_lanes {
        let lane = &layout.nodes[lane_index];
        let Some(bounds) = group_bounds.get(&lane.id) else {
            continue;
        };
        let content_width = js_max(0.0, bounds.max_x - bounds.min_x)
            + 2.0 * top_lane_horizontal_padding(lane.padding);
        let cx = (bounds.min_x + bounds.max_x) / 2.0;
        lane_ids.push(lane.id.clone());
        centers.push(cx);
        base_widths.push(content_width);
    }

    let count = lane_ids.len();
    if count == 0 {
        return;
    }
    let mut lane_widths: IndexMap<NodeId, f64> = IndexMap::new();
    if count == 1 {
        lane_widths.insert(lane_ids[0].clone(), base_widths[0]);
    } else {
        let d: Vec<f64> = (0..count - 1)
            .map(|i| centers[i + 1] - centers[i])
            .collect();
        let mut u = vec![0.0_f64; count];
        for i in 0..count - 1 {
            u[i + 1] = 2.0 * d[i] - u[i];
        }

        let mut lower_bound = 0.0_f64;
        let mut upper_bound = f64::INFINITY;
        for i in 0..count {
            let base_w = base_widths[i];
            if i % 2 == 0 {
                lower_bound = js_max(lower_bound, base_w - u[i]);
            } else {
                upper_bound = js_min(upper_bound, u[i] - base_w);
            }
        }

        let x = if lower_bound <= upper_bound {
            (lower_bound + upper_bound) / 2.0
        } else {
            lower_bound
        };

        for i in 0..count {
            let w = u[i] + if i % 2 == 0 { x } else { -x };
            let final_width = js_max(base_widths[i], w);
            lane_widths.insert(lane_ids[i].clone(), final_width);
        }
    }

    for &lane_index in &top_level {
        let lane = &mut layout.nodes[lane_index];
        if let Some(&w) = lane_widths.get(&lane.id) {
            lane.width = Some(w);
        }
        assign_top_lane_title_rect(lane);
    }
}
