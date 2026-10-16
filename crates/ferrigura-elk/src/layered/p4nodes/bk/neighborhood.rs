//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/bk/NeighborhoodInformation.java
//!
//! The information the Brandes & Koepf phases use many times and that does not change while they run:
//! the index of each layer and node, and the left (upper) and right (lower) neighbours of each node.
//!
//! Java stores the neighbours as `Pair<LNode, LEdge>` lists. Here a [`Neighbor`] holds the placement
//! id of the other node and the index of the edge. The lists are sorted with `Collections.sort`, which
//! GWT implements as a merge sort, so the port uses [`sort_by`] from `crate::java::sort`.
//!
//! `buildFor` assigns `LNode.id` and `Layer.id` in Java. Here the placement id is `LNode::placement_id`,
//! and the layer index is `LNode::layer`, which the same call sets.

use crate::java::sort::sort_by;
use crate::layered::lgraph::LGraph;
use crate::layered::options::PRIORITY_STRAIGHTNESS;

use super::access::{
    incoming_edges, is_in_layer_edge, is_self_loop, outgoing_edges, source_node_id, target_node_id,
};

/// A neighbour in a neighbour list: the `Pair<LNode, LEdge>` of Java.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    /// The placement id of the neighbouring node, `getFirst()`.
    pub node: usize,
    /// The index in `LGraph::edges` of the edge between the two, `getSecond()`.
    pub edge: usize,
}

/// `NeighborhoodInformation`. Indexed by placement id where Java indexes by `LNode.id`.
#[derive(Debug, Clone, Default)]
pub struct NeighborhoodInformation {
    /// `nodeCount`: the number of nodes in the layers.
    pub node_count: usize,
    /// `layerIndex`: the index of each layer, by layer id.
    pub layer_index: Vec<usize>,
    /// `nodeIndex`: the index of each node within its layer, by placement id.
    pub node_index: Vec<usize>,
    /// The arena index of each node, by placement id. The inverse of `LNode::placement_id`.
    pub node_of_id: Vec<usize>,
    /// `leftNeighbors`: for each node, its upper neighbours (sources of incoming edges), sorted by
    /// their index in the layer.
    pub left_neighbors: Vec<Vec<Neighbor>>,
    /// `rightNeighbors`: for each node, its lower neighbours (targets of outgoing edges), sorted by
    /// their index in the layer.
    pub right_neighbors: Vec<Vec<Neighbor>>,
}

impl NeighborhoodInformation {
    /// `NeighborhoodInformation.buildFor`: assigns the placement ids in layer order, and computes the
    /// neighbours of every node.
    pub fn build_for(graph: &mut LGraph) -> Self {
        let node_count: usize = graph.layers.iter().map(Vec::len).sum();
        let mut ni = NeighborhoodInformation {
            node_count,
            layer_index: vec![0; graph.layers.len()],
            node_index: vec![0; node_count],
            node_of_id: vec![0; node_count],
            left_neighbors: Vec::new(),
            right_neighbors: Vec::new(),
        };
        let mut next_id = 0;
        for (layer_id, layer) in graph.layers.iter().enumerate() {
            ni.layer_index[layer_id] = layer_id;
            for (index_in_layer, &node) in layer.iter().enumerate() {
                ni.node_index[next_id] = index_in_layer;
                ni.node_of_id[next_id] = node;
                graph.nodes[node].placement_id = next_id;
                graph.nodes[node].layer = Some(layer_id);
                next_id += 1;
            }
        }

        let graph: &LGraph = graph;
        let left: Vec<Vec<Neighbor>> = (0..node_count)
            .map(|id| {
                let node = &graph.nodes[ni.node_of_id[id]];
                let edges = incoming_edges(graph, node);
                neighbors(graph, &ni.node_index, &edges, source_node_id)
            })
            .collect();
        let right: Vec<Vec<Neighbor>> = (0..node_count)
            .map(|id| {
                let node = &graph.nodes[ni.node_of_id[id]];
                let edges = outgoing_edges(graph, node);
                neighbors(graph, &ni.node_index, &edges, target_node_id)
            })
            .collect();
        ni.left_neighbors = left;
        ni.right_neighbors = right;
        ni
    }
}

/// The neighbours of one node along `edges`, as `determineAllLeftNeighbors` and
/// `determineAllRightNeighbors` build them: only the edges of the highest straightness priority
/// count, self-loops and in-layer edges are skipped, and the result is sorted by layer index.
fn neighbors(
    graph: &LGraph,
    node_index: &[usize],
    edges: &[usize],
    far_end: fn(&LGraph, usize) -> usize,
) -> Vec<Neighbor> {
    let mut result: Vec<Neighbor> = Vec::new();
    let mut max_priority = 0;
    for &edge in edges {
        if is_self_loop(graph, edge) || is_in_layer_edge(graph, edge) {
            continue;
        }
        let edge_prio = priority_straightness(graph, edge);
        if edge_prio > max_priority {
            max_priority = edge_prio;
            result.clear();
        }
        if edge_prio == max_priority {
            result.push(Neighbor {
                node: far_end(graph, edge),
                edge,
            });
        }
    }
    // NeighborComparator: the difference of the layer indices, reduced to its sign.
    sort_by(result.as_mut_slice(), |a, b| {
        (node_index[a.node] as i64 - node_index[b.node] as i64).cmp(&0)
    });
    result
}

/// `edge.getProperty(LayeredOptions.PRIORITY_STRAIGHTNESS)`: the registry default when unset.
fn priority_straightness(graph: &LGraph, edge: usize) -> i32 {
    graph.edges[edge]
        .properties
        .int_with_default(PRIORITY_STRAIGHTNESS)
}
