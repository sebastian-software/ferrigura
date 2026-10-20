//! The default layering, `assignLayers_LaneAwareCompact` (`phase2.laneAwareCompact.ts`, lines 16-88).
//!
//! Ported: the generation-based topological sort for `LR` (`topoSortByGenerationIfAcyclic`, 16-37), the
//! rank assignment with `ignoreCrossLaneEdges` true (the only setting the core passes), and the layers.
//! The `compactSingleInput` option is read by no code on this path, so it is not carried.
//!
//! Not ported: the non-default layering `assignLayers_Gravity` (`phase2.gravity.ts`), which runs when
//! `ignoreCrossLaneEdges` is false and is refused by the core.

use std::collections::HashMap;

use indexmap::{IndexMap, IndexSet};

use crate::helpers::{Graph, Layering, NodeId};
use crate::js::locale_compare;
use crate::model::Direction;
use crate::phase0::{
    build_in_degree_map, build_layers_from_ranks, build_sorted_successor_map, incoming,
    normalize_graph, sorted_zero_in_degree_nodes, topo_sort_if_acyclic,
};
use crate::phase2_options::{build_top_lane_map, top_lane_of};

/// `topoSortByGenerationIfAcyclic`: Kahn's algorithm by generations. Each generation is the set of nodes
/// whose last predecessor was in the previous one, sorted by `localeCompare`. `None` for a cycle.
fn topo_sort_by_generation_if_acyclic(g: &Graph) -> Option<Vec<NodeId>> {
    let mut indeg = build_in_degree_map(g);
    let adj = build_sorted_successor_map(g);
    let mut frontier = sorted_zero_in_degree_nodes(g, &indeg);
    let mut order: Vec<NodeId> = Vec::new();

    while !frontier.is_empty() {
        let mut next_frontier: Vec<NodeId> = Vec::new();
        for u in &frontier {
            order.push(u.clone());
            for v in adj.get(u).map(Vec::as_slice).unwrap_or(&[]) {
                let degree = indeg.entry(v.clone()).or_insert(0);
                *degree -= 1;
                if *degree == 0 {
                    next_frontier.push(v.clone());
                }
            }
        }
        next_frontier.sort_by(|a, b| locale_compare(a, b));
        frontier = next_frontier;
    }

    (order.len() == g.nodes.len()).then_some(order)
}

/// `assignLayers_LaneAwareCompact`: one node per layer and lane, where an edge between two lanes can stay
/// on one layer.
///
/// A node's rank is the larger of its predecessors' ranks, plus one for a predecessor in the same lane
/// (zero otherwise), and the next free rank of its lane. The order is by generation for `LR`, and by the
/// queue for the other directions. A node without a rank (a group, or a predecessor that is one) counts as
/// rank zero.
pub(crate) fn assign_layers_lane_aware_compact(
    g_acyclic: &Graph,
    direction: Direction,
) -> Layering {
    let g = normalize_graph(g_acyclic);
    let order = if direction == Direction::LR {
        topo_sort_by_generation_if_acyclic(&g).unwrap_or_else(|| sorted_nodes(&g))
    } else {
        topo_sort_if_acyclic(&g).unwrap_or_else(|| sorted_nodes(&g))
    };

    let top_lanes = build_top_lane_map(&g);
    // `topLaneOf(id) ?? id`: the top lane, or the node itself when it has none.
    let lane_of = |id: &str| top_lane_of(&top_lanes, id).unwrap_or_else(|| id.to_owned());
    // `ignoreCrossLaneEdges` is true on this path: an edge within a lane weighs 1, another 0.
    let edge_weight = |u: &str, v: &str| i64::from(lane_of(u) == lane_of(v));

    let mut rank_of: IndexMap<NodeId, i64> = IndexMap::new();
    let mut next_free: HashMap<String, i64> = HashMap::new();
    for v in &order {
        if g.node_by_id.get(v).is_some_and(|node| node.is_group) {
            // Groups get no rank and no capacity.
            continue;
        }
        let mut base = 0_i64;
        for e in incoming(&g, v) {
            let ru = rank_of.get(&e.src).copied().unwrap_or(0);
            base = base.max(ru + edge_weight(&e.src, v));
        }
        let lane = lane_of(v);
        let nf = next_free.get(&lane).copied().unwrap_or(0);
        let rank = base.max(nf);
        rank_of.insert(v.clone(), rank);
        next_free.insert(lane, rank + 1);
    }

    let layers = build_layers_from_ranks(&g, &order, &rank_of, true);
    Layering {
        layers,
        rank_of,
        dummy: IndexSet::new(),
    }
}

/// `[...g.nodes].sort()`: the fallback order when a graph has a cycle. The default sort compares UTF-16
/// code units, which for the ASCII ids of the layout is the byte order.
fn sorted_nodes(g: &Graph) -> Vec<NodeId> {
    let mut nodes = g.nodes.clone();
    nodes.sort();
    nodes
}
