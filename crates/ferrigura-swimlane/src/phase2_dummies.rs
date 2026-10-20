//! Dummy nodes for the edges that span more than one layer (`phase2.dummies.ts`, `makeProperLayering`,
//! lines 4-71).
//!
//! An edge from rank `rU` to rank `rV` with `rV - rU > 1` becomes a chain through the dummy nodes
//! `placeholder-<n>` on the ranks in between. The chain's edges are `<id>#<k>`. The edges are visited
//! sorted by id, source and destination, by `localeCompare`, so the dummies are numbered the same way.

use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};

use crate::helpers::{EdgeRef, Graph, GraphNode, Layering, NodeId};
use crate::js::locale_compare;
use crate::phase0::normalize_graph;

/// `makeProperLayering`: the layering with the dummy nodes added to their layers, and the graph whose
/// edges are the chains. The rank of each dummy is recorded in the layering's rank map.
pub(crate) fn make_proper_layering(layering: &Layering, g_acyclic: &Graph) -> (Layering, Graph) {
    let g = normalize_graph(g_acyclic);
    let mut rank_of: IndexMap<NodeId, i64> = layering.rank_of.clone();
    let mut layers: Vec<Vec<NodeId>> = layering.layers.clone();
    let mut dummy: IndexSet<NodeId> = layering.dummy.clone();
    let mut node_by_id = g.node_by_id.clone();
    let mut dummy_seq = 0_usize;

    // `addDummyAt(L)`: a dummy node on layer L.
    let mut add_dummy_at = |layer: i64,
                            layers: &mut Vec<Vec<NodeId>>,
                            rank_of: &mut IndexMap<NodeId, i64>,
                            dummy: &mut IndexSet<NodeId>,
                            node_by_id: &mut IndexMap<NodeId, GraphNode>|
     -> NodeId {
        let id = format!("placeholder-{dummy_seq}");
        dummy_seq += 1;
        node_by_id.insert(id.clone(), GraphNode::dummy());
        dummy.insert(id.clone());
        let index = layer as usize;
        while layers.len() <= index {
            layers.push(Vec::new());
        }
        layers[index].push(id.clone());
        rank_of.insert(id.clone(), layer);
        id
    };

    // The edges sorted by id, then source, then destination.
    let mut edges_sorted: Vec<&EdgeRef> = g.edges.iter().collect();
    edges_sorted.sort_by(|a, b| {
        if a.id == b.id {
            if a.src == b.src {
                locale_compare(&a.dst, &b.dst)
            } else {
                locale_compare(&a.src, &b.src)
            }
        } else {
            locale_compare(&a.id, &b.id)
        }
    });

    let mut new_edges: Vec<EdgeRef> = Vec::new();
    for e in edges_sorted {
        let r_u = rank_of.get(&e.src).copied().unwrap_or(0);
        let r_v = rank_of.get(&e.dst).copied().unwrap_or(0);
        if r_v - r_u <= 1 {
            new_edges.push(e.clone());
            continue;
        }

        // The dummies on the ranks r_u + 1 .. r_v - 1, chained from the source.
        let mut prev = e.src.clone();
        for (k, layer) in ((r_u + 1)..r_v).enumerate() {
            let d = add_dummy_at(
                layer,
                &mut layers,
                &mut rank_of,
                &mut dummy,
                &mut node_by_id,
            );
            new_edges.push(EdgeRef {
                id: format!("{}#{}", e.id, k),
                src: prev,
                dst: d.clone(),
                origin: e.origin.clone(),
            });
            prev = d;
        }
        // The last link, into the target.
        let last_index = r_v - r_u - 2;
        new_edges.push(EdgeRef {
            id: format!("{}#{}", e.id, (last_index + 1).max(0)),
            src: prev,
            dst: e.dst.clone(),
            origin: e.origin.clone(),
        });
    }

    // The nodes: the graph's own, then the dummies that it does not hold.
    let own: HashSet<&NodeId> = g.nodes.iter().collect();
    let mut nodes: Vec<NodeId> = g.nodes.clone();
    for id in &dummy {
        if !own.contains(id) {
            nodes.push(id.clone());
        }
    }

    let graph_with_dummies = Graph {
        nodes,
        edges: new_edges,
        node_by_id,
        top_level_groups: g.top_level_groups.clone(),
    };
    (
        Layering {
            layers,
            rank_of,
            dummy,
        },
        graph_with_dummies,
    )
}
