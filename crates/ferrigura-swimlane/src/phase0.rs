//! Graph helpers of the phases (`phase0.helpers.ts`).
//!
//! Ported: `normalizeGraph` (5-23), `incoming` (26-28), `buildSuccessorMap` (35-44),
//! `buildSortedSuccessorMap` (46-52), `buildInDegreeMap` (54-63), `sortedZeroInDegreeNodes` (65-79),
//! `buildLayersFromRanks` (101-123), `topoSortIfAcyclic` (158-181), `buildLayerIndex` (187-195) and
//! `countInversions` (197-222).
//!
//! Not ported: `buildPredecessorSuccessorMaps` (used by the non-default layering only), `isAcyclic`
//! and `outgoing` (no caller in the reference).

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::helpers::{EdgeRef, Graph, NodeId};
use crate::js::locale_compare;

/// `normalizeGraph`: keeps the edges whose endpoints are nodes, and one edge per `id:src->dst`. The nodes
/// become the keys of the node map, in its order.
pub(crate) fn normalize_graph(g: &Graph) -> Graph {
    let node_by_id = g.node_by_id.clone();
    let mut seen: HashSet<String> = HashSet::new();
    let mut edges: Vec<EdgeRef> = Vec::new();
    for e in &g.edges {
        if !node_by_id.contains_key(&e.src) || !node_by_id.contains_key(&e.dst) {
            continue;
        }
        let key = format!("{}:{}->{}", e.id, e.src, e.dst);
        if !seen.insert(key) {
            continue;
        }
        edges.push(e.clone());
    }
    let nodes = node_by_id.keys().cloned().collect();
    Graph {
        nodes,
        edges,
        node_by_id,
        top_level_groups: g.top_level_groups.clone(),
    }
}

/// `incoming`: the edges whose target is `v`, in the order of the edges.
pub(crate) fn incoming<'a>(g: &'a Graph, v: &str) -> Vec<&'a EdgeRef> {
    g.edges.iter().filter(|e| e.dst == v).collect()
}

/// `buildSuccessorMap`: the targets of each node's edges, in the order of the edges. Every node of the
/// graph has an entry. The graph must be normalised.
pub(crate) fn build_successor_map(g: &Graph) -> IndexMap<NodeId, Vec<NodeId>> {
    let mut succs: IndexMap<NodeId, Vec<NodeId>> = IndexMap::new();
    for v in &g.nodes {
        succs.insert(v.clone(), Vec::new());
    }
    for e in &g.edges {
        succs
            .get_mut(&e.src)
            .expect("a normalised graph has its edge sources among its nodes")
            .push(e.dst.clone());
    }
    succs
}

/// `buildSortedSuccessorMap`: the successors of each node, sorted by `localeCompare`.
pub(crate) fn build_sorted_successor_map(g: &Graph) -> IndexMap<NodeId, Vec<NodeId>> {
    let mut succs = build_successor_map(g);
    for successors in succs.values_mut() {
        successors.sort_by(|a, b| locale_compare(a, b));
    }
    succs
}

/// `buildInDegreeMap`: the number of incoming edges of each node, keyed in the order of the nodes.
pub(crate) fn build_in_degree_map(g: &Graph) -> IndexMap<NodeId, i64> {
    let mut indeg: IndexMap<NodeId, i64> = IndexMap::new();
    for v in &g.nodes {
        indeg.insert(v.clone(), 0);
    }
    for e in &g.edges {
        *indeg.entry(e.dst.clone()).or_insert(0) += 1;
    }
    indeg
}

/// `sortedZeroInDegreeNodes`: the nodes without incoming edges, sorted by `localeCompare`. The nodes that
/// have no edge at all keep their slots in the order of the nodes, so a lane of unconnected nodes is not
/// reordered alphabetically (#8355; `phase0.helpers.ts:71-78`).
pub(crate) fn sorted_zero_in_degree_nodes(g: &Graph, indeg: &IndexMap<NodeId, i64>) -> Vec<NodeId> {
    let mut sorted: Vec<NodeId> = indeg
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    sorted.sort_by(|a, b| locale_compare(a, b));

    let linked: HashSet<&str> = g
        .edges
        .iter()
        .flat_map(|e| [e.src.as_str(), e.dst.as_str()])
        .collect();
    let is_unconnected =
        |id: &str| !linked.contains(id) && !g.node_by_id.get(id).is_some_and(|node| node.is_group);
    let unconnected: Vec<NodeId> = indeg
        .iter()
        .filter(|(id, degree)| **degree == 0 && is_unconnected(id))
        .map(|(id, _)| id.clone())
        .collect();

    let mut next = 0;
    sorted
        .into_iter()
        .map(|id| {
            if is_unconnected(&id) {
                let slot = unconnected[next].clone();
                next += 1;
                slot
            } else {
                id
            }
        })
        .collect()
}

/// `buildLayersFromRanks`: the layers of the nodes in `order`, by rank. With `skip_groups`, the groups
/// are left out. A node without a rank is on layer 0.
pub(crate) fn build_layers_from_ranks(
    g: &Graph,
    order: &[NodeId],
    rank_of: &IndexMap<NodeId, i64>,
    skip_groups: bool,
) -> Vec<Vec<NodeId>> {
    let is_skipped = |v: &str| skip_groups && g.node_by_id.get(v).is_some_and(|node| node.is_group);

    let mut max_rank = 0_i64;
    for v in &g.nodes {
        if is_skipped(v) {
            continue;
        }
        max_rank = max_rank.max(rank_of.get(v).copied().unwrap_or(0));
    }

    let mut layers: Vec<Vec<NodeId>> = vec![Vec::new(); (max_rank + 1) as usize];
    for v in order {
        if is_skipped(v) {
            continue;
        }
        let rank = rank_of.get(v).copied().unwrap_or(0).max(0);
        layers[rank as usize].push(v.clone());
    }
    layers
}

/// `topoSortIfAcyclic`: Kahn's algorithm with a queue kept sorted by insertion. The queue starts with the
/// nodes of in-degree zero, sorted by `localeCompare`; each node that becomes free is inserted before the
/// first queued node that is not less than it, compared as strings (`phase0.helpers.ts:170-175`). `None`
/// when the graph has a cycle.
pub(crate) fn topo_sort_if_acyclic(g: &Graph) -> Option<Vec<NodeId>> {
    let mut indeg = build_in_degree_map(g);
    let mut queue = sorted_zero_in_degree_nodes(g, &indeg);
    let mut order: Vec<NodeId> = Vec::new();
    let adj = build_sorted_successor_map(g);

    while !queue.is_empty() {
        let u = queue.remove(0);
        for v in adj.get(&u).map(Vec::as_slice).unwrap_or(&[]) {
            let degree = indeg.entry(v.clone()).or_insert(0);
            *degree -= 1;
            if *degree == 0 {
                let mut i = 0;
                while i < queue.len() && queue[i].as_str() < v.as_str() {
                    i += 1;
                }
                queue.insert(i, v.clone());
            }
        }
        order.push(u);
    }

    (order.len() == g.nodes.len()).then_some(order)
}

/// `buildLayerIndex`: the position of each node in a layer. A duplicate keeps its last position.
pub(crate) fn build_layer_index(layer: &[NodeId]) -> IndexMap<NodeId, usize> {
    let mut index_of: IndexMap<NodeId, usize> = IndexMap::new();
    for (index, id) in layer.iter().enumerate() {
        index_of.insert(id.clone(), index);
    }
    index_of
}

/// `countInversions`: the number of pairs out of order in `values`, by merge sort. The values are
/// positions, so they are integers and the comparisons are exact. `values` ends sorted, as the
/// reference's does.
pub(crate) fn count_inversions(values: &mut [usize]) -> usize {
    let mut tmp = vec![0_usize; values.len()];
    let len = values.len();
    count_range(values, &mut tmp, 0, len)
}

fn count_range(values: &mut [usize], tmp: &mut [usize], left: usize, right: usize) -> usize {
    if right - left <= 1 {
        return 0;
    }
    let mid = (left + right) / 2;
    let mut inversions = count_range(values, tmp, left, mid) + count_range(values, tmp, mid, right);
    let mut i = left;
    let mut j = mid;
    let mut k = left;
    while i < mid || j < right {
        if j >= right || (i < mid && values[i] <= values[j]) {
            tmp[k] = values[i];
            i += 1;
        } else {
            tmp[k] = values[j];
            j += 1;
            inversions += mid - i;
        }
        k += 1;
    }
    values[left..right].copy_from_slice(&tmp[left..right]);
    inversions
}

#[cfg(test)]
mod tests {
    use super::count_inversions;

    #[test]
    fn counts_pairs_out_of_order() {
        let mut values = vec![2, 0, 1];
        assert_eq!(count_inversions(&mut values), 2);
        assert_eq!(values, vec![0, 1, 2]);
        let mut sorted = vec![0, 1, 2, 3];
        assert_eq!(count_inversions(&mut sorted), 0);
        let mut reversed = vec![3, 2, 1, 0];
        assert_eq!(count_inversions(&mut reversed), 6);
    }
}
