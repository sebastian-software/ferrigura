//! The ordering of the nodes in each layer (`phase3.ordering.ts`): median sweeps, lane-aware
//! reordering, and adjacent transpositions.
//!
//! Ported: `median` (7-17), `barycenter` (19-25), `neighborPositionsFor` (27-47),
//! `currentOrderTieBreak` (49-57), `countCrossingsBetweenAdjacent` (59-75), `sortByHeuristic` (89-108),
//! `reorderLayer` (110-183), `transposeImprove` (185-244) and `orderLayers` (251-279).
//!
//! Not ported: `totalCrossings` (77-83), which no code on the default path calls. The `!topLaneOf` branch of
//! `reorderLayer` is not ported: `orderLayers` always passes the top-lane resolver.

use std::cmp::Ordering;
use std::collections::HashSet;

use indexmap::IndexMap;

use crate::helpers::{EdgeRef, Graph, Layering, NodeId, OrderedLayers};
use crate::js::locale_compare;
use crate::math::js_sign_order;
use crate::phase0::{build_layer_index, count_inversions};
use crate::phase2_options::{build_top_lane_map, resolve_top_lane_order, top_lane_of};

/// The sweep direction: the fixed layer is the one above (`Down`) or the one below (`Up`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sweep {
    Down,
    Up,
}

/// `median`: the median of the values, `Infinity` for none. An even count takes the mean of the two middle
/// values, as `0.5 * (a + b)`.
fn median(values: &[f64]) -> f64 {
    let n = values.len();
    if n == 0 {
        return f64::INFINITY;
    }
    let mut a = values.to_vec();
    a.sort_by(|x, y| js_sign_order(x - y));
    if n % 2 == 1 {
        return a[(n - 1) / 2];
    }
    0.5 * (a[n / 2 - 1] + a[n / 2])
}

/// `barycenter`: the mean of the values, summed in order from zero; `Infinity` for none.
fn barycenter(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::INFINITY;
    }
    let sum = values.iter().fold(0.0_f64, |acc, v| acc + v);
    sum / values.len() as f64
}

/// The positions in the fixed layer of the neighbours of each target node, in the order of the edges.
fn neighbor_positions_for(
    targets: &[NodeId],
    fixed_index: &IndexMap<NodeId, usize>,
    edges: &[EdgeRef],
    direction: Sweep,
) -> IndexMap<NodeId, Vec<f64>> {
    let mut neighbor_positions: IndexMap<NodeId, Vec<f64>> = IndexMap::new();
    for v in targets {
        neighbor_positions.insert(v.clone(), Vec::new());
    }
    for e in edges {
        match direction {
            Sweep::Down => {
                if let (Some(&pos), true) = (
                    fixed_index.get(&e.src),
                    neighbor_positions.contains_key(&e.dst),
                ) {
                    neighbor_positions
                        .get_mut(&e.dst)
                        .expect("the target was inserted")
                        .push(pos as f64);
                }
            }
            Sweep::Up => {
                if let (Some(&pos), true) = (
                    fixed_index.get(&e.dst),
                    neighbor_positions.contains_key(&e.src),
                ) {
                    neighbor_positions
                        .get_mut(&e.src)
                        .expect("the target was inserted")
                        .push(pos as f64);
                }
            }
        }
    }
    neighbor_positions
}

/// `currentOrderTieBreak`: the current position, then `localeCompare`.
fn current_order_tie_break(a: &str, b: &str, current: &IndexMap<NodeId, usize>) -> Ordering {
    let ia = current.get(a).copied().unwrap_or(0);
    let ib = current.get(b).copied().unwrap_or(0);
    if ia != ib {
        ia.cmp(&ib)
    } else {
        locale_compare(a, b)
    }
}

/// `countCrossingsBetweenAdjacent`: the inversions among the edges between two adjacent layers, counted
/// on the positions of the lower ends, sorted by the positions of the upper ends.
fn count_crossings_between_adjacent(
    upper: &[NodeId],
    lower: &[NodeId],
    edges: &[&EdgeRef],
) -> usize {
    let upper_set: HashSet<&str> = upper.iter().map(String::as_str).collect();
    let lower_set: HashSet<&str> = lower.iter().map(String::as_str).collect();
    let upper_index = build_layer_index(upper);
    let lower_index = build_layer_index(lower);
    let mut pairs: Vec<(usize, usize)> = edges
        .iter()
        .filter(|e| upper_set.contains(e.src.as_str()) && lower_set.contains(e.dst.as_str()))
        .map(|e| (upper_index[&e.src], lower_index[&e.dst]))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut vs: Vec<usize> = pairs.iter().map(|pair| pair.1).collect();
    count_inversions(&mut vs)
}

/// `sortByHeuristic`: the nodes by the median of their fixed neighbours' positions. A node without
/// neighbours sorts after the others; ties keep the current order.
fn sort_by_heuristic(
    nodes: &[NodeId],
    neighbor_positions: &IndexMap<NodeId, Vec<f64>>,
    current: &IndexMap<NodeId, usize>,
) -> Vec<NodeId> {
    let positions = |id: &str| neighbor_positions.get(id).map_or(&[][..], Vec::as_slice);
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|a, b| {
        let sa = median(positions(a));
        let sb = median(positions(b));
        if sa == sb {
            return current_order_tie_break(a, b, current);
        }
        if !sa.is_finite() {
            return Ordering::Greater;
        }
        if !sb.is_finite() {
            return Ordering::Less;
        }
        js_sign_order(sa - sb)
    });
    sorted
}

/// `reorderLayer`: the target layer in a new order. With lanes, the nodes of each lane keep together, in
/// the lane order; the nodes without a lane (the dummies) go where their barycentre fits.
fn reorder_layer(
    fixed_layer: &[NodeId],
    target_layer: &[NodeId],
    edges: &[EdgeRef],
    direction: Sweep,
    top_lanes: &IndexMap<NodeId, Option<NodeId>>,
    lane_order: &[NodeId],
) -> Vec<NodeId> {
    let fixed_index = build_layer_index(fixed_layer);
    let curr_index = build_layer_index(target_layer);
    let neighbor_positions = neighbor_positions_for(target_layer, &fixed_index, edges, direction);

    if lane_order.is_empty() {
        return sort_by_heuristic(target_layer, &neighbor_positions, &curr_index);
    }

    let mut by_lane: IndexMap<Option<NodeId>, Vec<NodeId>> = IndexMap::new();
    for id in target_layer {
        by_lane
            .entry(top_lane_of(top_lanes, id))
            .or_default()
            .push(id.clone());
    }

    let mut result: Vec<NodeId> = Vec::new();
    for lane in lane_order {
        if let Some(nodes_in_lane) = by_lane
            .get(&Some(lane.clone()))
            .filter(|nodes| !nodes.is_empty())
        {
            result.extend(sort_by_heuristic(
                nodes_in_lane,
                &neighbor_positions,
                &curr_index,
            ));
        }
    }

    // The nodes without a lane go where their barycentre fits: before the first placed node whose
    // barycentre is larger.
    if let Some(null_nodes) = by_lane.get(&None).filter(|nodes| !nodes.is_empty()) {
        let sorted = sort_by_heuristic(null_nodes, &neighbor_positions, &curr_index);
        for nid in sorted {
            let bc = barycenter(neighbor_positions.get(&nid).map_or(&[][..], Vec::as_slice));
            let mut best_idx = result.len();
            if bc.is_finite() {
                for (i, rid) in result.iter().enumerate() {
                    let r_bc =
                        barycenter(neighbor_positions.get(rid).map_or(&[][..], Vec::as_slice));
                    if bc < r_bc {
                        best_idx = i;
                        break;
                    }
                }
            }
            result.insert(best_idx, nid);
        }
    }

    result
}

/// `transposeImprove`: swaps adjacent nodes of the same lane while a swap lowers the crossings with the
/// upper layer and, when there is one, the next layer. Repeats until no swap helps.
fn transpose_improve(
    upper: &[NodeId],
    current: &[NodeId],
    edges: &[EdgeRef],
    next: Option<&[NodeId]>,
    top_lanes: &IndexMap<NodeId, Option<NodeId>>,
) -> Vec<NodeId> {
    let mut best: Vec<NodeId> = current.to_vec();
    let upper_set: HashSet<&str> = upper.iter().map(String::as_str).collect();
    let layer_set: HashSet<&str> = current.iter().map(String::as_str).collect();
    let next_set: Option<HashSet<&str>> =
        next.map(|layer| layer.iter().map(String::as_str).collect());

    let edges_in: Vec<&EdgeRef> = edges
        .iter()
        .filter(|e| upper_set.contains(e.src.as_str()) && layer_set.contains(e.dst.as_str()))
        .collect();
    let edges_out: Option<Vec<&EdgeRef>> = next_set.as_ref().map(|next_set| {
        edges
            .iter()
            .filter(|e| layer_set.contains(e.src.as_str()) && next_set.contains(e.dst.as_str()))
            .collect()
    });

    let crossing_score = |order: &[NodeId]| -> usize {
        let mut score = count_crossings_between_adjacent(upper, order, &edges_in);
        if let (Some(edges_out), Some(next)) = (&edges_out, next) {
            score += count_crossings_between_adjacent(order, next, edges_out);
        }
        score
    };

    let mut improved = true;
    let mut best_score = crossing_score(&best);
    while improved {
        improved = false;
        for i in 0..best.len().saturating_sub(1) {
            // Only nodes of the same lane swap; a node without a lane swaps with those without one.
            if top_lane_of(top_lanes, &best[i]) != top_lane_of(top_lanes, &best[i + 1]) {
                continue;
            }
            let prev = best_score;
            best.swap(i, i + 1);
            let next_score = crossing_score(&best);
            if next_score < prev {
                best_score = next_score;
                improved = true;
            } else {
                best.swap(i, i + 1);
            }
        }
    }
    best
}

/// `orderLayers`: three rounds of a down sweep then an up sweep. Each layer is reordered by the median of
/// its fixed neighbours, then improved by transpositions.
///
/// The up sweep passes the upper and the next layers in the reverse roles of the down sweep (`phase3`
/// lines 272-275); the reference does this, and the port keeps it.
pub(crate) fn order_layers(
    layering: &Layering,
    g_with_dummies: &Graph,
    lane_order: Option<&[NodeId]>,
) -> OrderedLayers {
    let mut layers: Vec<Vec<NodeId>> = layering.layers.clone();
    let edges = &g_with_dummies.edges;

    let top_lanes = build_top_lane_map(g_with_dummies);
    let lane_order = resolve_top_lane_order(g_with_dummies, lane_order);

    for _ in 0..3 {
        // Top-down: reorder layer i by its neighbours in layer i-1.
        for i in 1..layers.len() {
            let reordered = reorder_layer(
                &layers[i - 1],
                &layers[i],
                edges,
                Sweep::Down,
                &top_lanes,
                &lane_order,
            );
            layers[i] = reordered;
            let next = layers.get(i + 1).map(Vec::as_slice);
            let improved = transpose_improve(&layers[i - 1], &layers[i], edges, next, &top_lanes);
            layers[i] = improved;
        }
        // Bottom-up: reorder layer i by its neighbours in layer i+1.
        if layers.len() >= 2 {
            for i in (0..=layers.len() - 2).rev() {
                let reordered = reorder_layer(
                    &layers[i + 1],
                    &layers[i],
                    edges,
                    Sweep::Up,
                    &top_lanes,
                    &lane_order,
                );
                layers[i] = reordered;
                let next = if i >= 1 {
                    Some(layers[i - 1].as_slice())
                } else {
                    None
                };
                let improved =
                    transpose_improve(&layers[i + 1], &layers[i], edges, next, &top_lanes);
                layers[i] = improved;
            }
        }
    }

    OrderedLayers { layers }
}
