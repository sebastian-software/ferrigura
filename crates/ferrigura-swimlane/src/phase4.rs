//! The coordinates of the nodes (`phase4.coordinates.ts`, `assignCoordinates`, lines 13-192).
//!
//! The lanes are columns, from left to right in the lane order, with `laneGap` between them, and the
//! nodes of a layer are centred in their lane's column. The dummy nodes of one original edge are placed at
//! the rounded midpoint of its two ends.
//!
//! Every sum keeps the operand order of the reference, because the sum of doubles is not associative:
//! `ids.reduce(...) + nodeGap * (n - 1)`, `start += w + nodeGap`, and `yOffset += layerH + layerGap +
//! extraGap`.

use indexmap::{IndexMap, IndexSet};

use crate::helpers::{Coordinates, EdgeRef, Graph, NodeId, OrderedLayers};
use crate::js::js_round;
use crate::math::js_max;
use crate::model::Direction;
use crate::phase2_options::{build_top_lane_map, resolve_top_lane_order, top_lane_of};

/// `assignCoordinates`. The gaps are the core's `nodeGap` and `layerGap`; the lane gap is twice the node
/// gap (`laneGap = opts?.laneGap ?? nodeGap * 2`, which the core never overrides).
pub(crate) fn assign_coordinates(
    ordered: &OrderedLayers,
    g_with_dummies: &Graph,
    layer_gap: f64,
    node_gap: f64,
    direction: Direction,
    lane_order: Option<&[NodeId]>,
) -> Coordinates {
    let lane_gap = node_gap * 2.0;
    let is_horizontal = matches!(direction, Direction::LR | Direction::RL);

    let layers = &ordered.layers;
    let mut x: IndexMap<NodeId, f64> = IndexMap::new();
    let mut y: IndexMap<NodeId, f64> = IndexMap::new();

    let width_of = |id: &str| {
        g_with_dummies
            .node_by_id
            .get(id)
            .and_then(|node| node.width)
            .unwrap_or(0.0)
    };
    let height_of = |id: &str| {
        g_with_dummies
            .node_by_id
            .get(id)
            .and_then(|node| node.height)
            .unwrap_or(0.0)
    };
    let top_lanes = build_top_lane_map(g_with_dummies);
    let top_of = |id: &str| top_lane_of(&top_lanes, id);
    let lane_order_global = resolve_top_lane_order(g_with_dummies, lane_order);

    let layer_heights: Vec<f64> = layers
        .iter()
        .map(|layer| layer.iter().fold(0.0_f64, |m, v| js_max(m, height_of(v))))
        .collect();

    // The LR and RL transforms turn the widths into vertical spans, so the layer gaps are widened first.
    let mut extra_layer_gaps: Vec<f64> = Vec::new();
    if is_horizontal {
        for i in 0..layers.len().saturating_sub(1) {
            let this_layer_max_width = layers[i]
                .iter()
                .fold(0.0_f64, |m, v| js_max(m, width_of(v)));
            let next_layer_max_width = layers[i + 1]
                .iter()
                .fold(0.0_f64, |m, v| js_max(m, width_of(v)));
            let this_layer_max_height = layer_heights[i];
            let next_layer_max_height = layer_heights[i + 1];

            let normal_spacing = this_layer_max_height / 2.0 + next_layer_max_height / 2.0;
            let required_spacing = (this_layer_max_width + next_layer_max_width) / 2.0;
            let extra_needed = js_max(0.0, required_spacing - normal_spacing - layer_gap);
            extra_layer_gaps.push(extra_needed);
        }
    }

    // The lanes in use, in the lane order; the dummies (no lane) come first when there are any.
    let mut lanes_used_set: IndexSet<Option<NodeId>> = IndexSet::new();
    for layer in layers {
        for id in layer {
            lanes_used_set.insert(top_of(id));
        }
    }
    let has_null_lane = lanes_used_set.contains(&None);
    let lanes_used: Vec<NodeId> = lane_order_global
        .iter()
        .filter(|lane| lanes_used_set.contains(&Some((*lane).clone())))
        .cloned()
        .collect();
    let mut columns: Vec<Option<NodeId>> = Vec::new();
    if has_null_lane {
        columns.push(None);
    }
    columns.extend(lanes_used.iter().map(|lane| Some(lane.clone())));

    // The width of each lane. The dummies' lane is keyed `"null"`, as the reference keys it in a plain
    // object, so a lane named `null` shares that entry.
    let mut lane_width: IndexMap<String, f64> = IndexMap::new();
    for lane in &lanes_used {
        lane_width.insert(lane.clone(), 0.0);
    }
    if has_null_lane {
        lane_width.insert("null".to_owned(), 0.0);
    }
    for layer in layers {
        let mut per_lane: IndexMap<String, Vec<NodeId>> = IndexMap::new();
        let mut null_ids: Vec<NodeId> = Vec::new();
        for id in layer {
            match top_of(id) {
                None => null_ids.push(id.clone()),
                Some(lane) => per_lane.entry(lane).or_default().push(id.clone()),
            }
        }
        for (lane, ids) in &per_lane {
            let total = ids.iter().fold(0.0_f64, |s, id| s + width_of(id))
                + node_gap * js_max(0.0, ids.len() as f64 - 1.0);
            let current = lane_width.get(lane).copied().unwrap_or(0.0);
            lane_width.insert(lane.clone(), js_max(current, total));
        }
        if has_null_lane && !null_ids.is_empty() {
            let total_null = null_ids.iter().fold(0.0_f64, |s, id| s + width_of(id))
                + node_gap * js_max(0.0, null_ids.len() as f64 - 1.0);
            let current = lane_width.get("null").copied().unwrap_or(0.0);
            lane_width.insert("null".to_owned(), js_max(current, total_null));
        }
    }

    // The centre of each column, the columns laid out from the left, centred on zero.
    let width_of_column = |column: &Option<NodeId>| -> f64 {
        match column {
            None => lane_width.get("null").copied(),
            Some(lane) => lane_width.get(lane).copied(),
        }
        .unwrap_or(0.0)
    };
    let widths: Vec<f64> = columns.iter().map(width_of_column).collect();
    let total_w = widths.iter().fold(0.0_f64, |a, b| a + b)
        + lane_gap * js_max(0.0, columns.len() as f64 - 1.0);
    let mut cursor = -total_w / 2.0;
    let mut center_x: IndexMap<Option<NodeId>, f64> = IndexMap::new();
    for (i, column) in columns.iter().enumerate() {
        let w = widths[i];
        let cx = cursor + w / 2.0;
        center_x.insert(column.clone(), cx);
        cursor += w;
        if i + 1 < columns.len() {
            cursor += lane_gap;
        }
    }

    // The nodes of each layer: centred on their lane's column, or spread around its centre in order.
    let mut y_offset = 0.0_f64;
    for (li, layer) in layers.iter().enumerate() {
        let layer_h = layer_heights[li];

        let mut by_lane: IndexMap<Option<NodeId>, Vec<NodeId>> = IndexMap::new();
        for id in layer {
            by_lane.entry(top_of(id)).or_default().push(id.clone());
        }

        for column in &columns {
            let Some(nodes_in_lane) = by_lane.get(column) else {
                continue;
            };
            if nodes_in_lane.is_empty() {
                continue;
            }
            let cx = center_x[column];
            if nodes_in_lane.len() == 1 {
                let id = &nodes_in_lane[0];
                x.insert(id.clone(), cx);
                y.insert(id.clone(), y_offset + layer_h / 2.0);
            } else {
                let widths: Vec<f64> = nodes_in_lane.iter().map(|id| width_of(id)).collect();
                let total = widths.iter().fold(0.0_f64, |a, b| a + b)
                    + node_gap * (nodes_in_lane.len() - 1) as f64;
                let mut start = cx - total / 2.0;
                for (i, id) in nodes_in_lane.iter().enumerate() {
                    let w = widths[i];
                    x.insert(id.clone(), start + w / 2.0);
                    y.insert(id.clone(), y_offset + layer_h / 2.0);
                    start += w + node_gap;
                }
            }
        }

        let extra_gap = extra_layer_gaps.get(li).copied().unwrap_or(0.0);
        y_offset += layer_h + layer_gap + extra_gap;
    }

    // The dummy chain of each original edge sits at the rounded midpoint of its ends.
    let mut by_ref: IndexMap<String, Vec<&EdgeRef>> = IndexMap::new();
    for e in &g_with_dummies.edges {
        by_ref.entry(e.origin.id.clone()).or_default().push(e);
    }
    for chain in by_ref.values() {
        let origin = &chain[0].origin;
        let src = origin.start.as_str();
        let dst = origin.end.as_str();
        let mid_x = js_round(
            (x.get(src).copied().unwrap_or(0.0) + x.get(dst).copied().unwrap_or(0.0)) / 2.0,
        );
        let mut involved: IndexSet<&str> = IndexSet::new();
        for e in chain {
            involved.insert(e.src.as_str());
            involved.insert(e.dst.as_str());
        }
        for vid in involved {
            if vid == src || vid == dst {
                continue;
            }
            if g_with_dummies
                .node_by_id
                .get(vid)
                .is_some_and(|node| node.is_dummy)
            {
                x.insert(vid.to_owned(), mid_x);
            }
        }
    }

    Coordinates { x, y }
}
