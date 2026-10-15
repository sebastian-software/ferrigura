//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/LNode.java,
//!   LEdge.java and LPort.java, and the `Math.min` and `Math.max` of `double`
//!
//! The graph queries that the Brandes & Koepf classes make on Java's `LNode`, `LEdge` and `LPort`.
//! A node is its placement id here (`LNode.id`, assigned by `NeighborhoodInformation::build_for`), and
//! an edge or a port is its index in `LGraph::edges` or `LGraph::ports`.

use crate::layered::lgraph::{LGraph, LNode};

use super::neighborhood::NeighborhoodInformation;

/// The `LNode` with the given placement id.
pub fn lnode<'g>(graph: &'g LGraph, ni: &NeighborhoodInformation, id: usize) -> &'g LNode {
    &graph.nodes[ni.node_of_id[id]]
}

/// The placement id of the node a port belongs to, `port.getNode().id`.
pub fn port_node_id(graph: &LGraph, port: usize) -> usize {
    graph.nodes[graph.ports[port].node].placement_id
}

/// The placement id of the source node of an edge.
pub fn source_node_id(graph: &LGraph, edge: usize) -> usize {
    port_node_id(graph, graph.edges[edge].source)
}

/// The placement id of the target node of an edge.
pub fn target_node_id(graph: &LGraph, edge: usize) -> usize {
    port_node_id(graph, graph.edges[edge].target)
}

/// `LEdge.isSelfLoop()`: both ends of the edge are on the same node.
pub fn is_self_loop(graph: &LGraph, edge: usize) -> bool {
    graph.ports[graph.edges[edge].source].node == graph.ports[graph.edges[edge].target].node
}

/// `LEdge.isInLayerEdge()`: not a self-loop, and both nodes are in the same layer.
pub fn is_in_layer_edge(graph: &LGraph, edge: usize) -> bool {
    if is_self_loop(graph, edge) {
        return false;
    }
    let source = &graph.nodes[graph.ports[graph.edges[edge].source].node];
    let target = &graph.nodes[graph.ports[graph.edges[edge].target].node];
    source.layer == target.layer
}

/// `LNode.getIncomingEdges()`: the incoming edges of all ports, port by port.
pub fn incoming_edges(graph: &LGraph, node: &LNode) -> Vec<usize> {
    node.ports
        .iter()
        .flat_map(|&port| graph.ports[port].incoming.iter().copied())
        .collect()
}

/// `LNode.getOutgoingEdges()`: the outgoing edges of all ports, port by port.
pub fn outgoing_edges(graph: &LGraph, node: &LNode) -> Vec<usize> {
    node.ports
        .iter()
        .flat_map(|&port| graph.ports[port].outgoing.iter().copied())
        .collect()
}

/// `LNode.getConnectedEdges()`: for each port, its incoming edges and then its outgoing ones, as
/// `LPort`'s `CombineIter` gives them.
pub fn connected_edges(graph: &LGraph, node: &LNode) -> Vec<usize> {
    node.ports
        .iter()
        .flat_map(|&port| {
            let port = &graph.ports[port];
            port.incoming.iter().chain(port.outgoing.iter()).copied()
        })
        .collect()
}

/// `Math.min(double, double)` with Java's rules: NaN wins, and -0.0 is smaller than 0.0.
pub fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && b.is_sign_negative() {
        return b;
    }
    if a <= b { a } else { b }
}

/// `Math.max(double, double)` with Java's rules: NaN wins, and 0.0 is larger than -0.0.
pub fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && a.is_sign_negative() {
        return b;
    }
    if a >= b { a } else { b }
}
