//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/InvertedPortProcessor.java
//!
//! Inverted ports, "Inverted port preprocessing".
//!
//! An edge that enters a node at a port on its east side, or leaves it at a port on its west side,
//! runs against the layering. Before crossing minimization, each such edge of a normal node with
//! fixed port sides is turned around at a long edge dummy in the node's layer: the edge ends at the
//! dummy's western port, and an in-layer edge joins the dummy's eastern port with the inverted port
//! (or, on the west side, the inverted port with the dummy's western port, and the dummy's eastern
//! port with the edge's old target). Head labels move to the new edge. Self-loops are left alone.
//!
//! The dummies are long edge dummies, so the long edge joiner removes them after edge routing; the
//! in-layer edges they make are routed between the node's layer and its neighbour.

use crate::core::data::OptionValue;
use crate::core::data::enums::{EdgeLabelPlacement, NodeType, PortConstraints, PortSide};
use crate::layered::lgraph::{LEdge, LGraph, LNode, LPort};
use crate::layered::options;
use crate::layered::ports::is_side_fixed;

/// `InvertedPortProcessor.process`: the dummies made for a layer join it, in the order they were
/// made, once all of its nodes have been visited.
pub fn process(graph: &mut LGraph) {
    for layer in 0..graph.layers.len() {
        let mut unassigned = Vec::new();
        for node in graph.layers[layer].clone() {
            // Dummies, and nodes whose port sides are free, have no inverted ports.
            if graph.nodes[node].node_type != NodeType::NORMAL
                || !is_side_fixed(
                    graph.nodes[node]
                        .properties
                        .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS),
                )
            {
                continue;
            }
            // `getPorts(PortType.INPUT, PortSide.EAST)`: input ports on the east side. Turning an
            // edge around changes no other port of the node, so the ports can be listed first.
            for port in ports_on(graph, node, PortSide::EAST) {
                // `LGraphUtil.toEdgeArray`: a copy, as the edges leave the port.
                for edge in graph.ports[port].incoming.clone() {
                    create_east_port_side_dummies(graph, port, edge, &mut unassigned);
                }
            }
            // `getPorts(PortType.OUTPUT, PortSide.WEST)`: output ports on the west side.
            for port in ports_on(graph, node, PortSide::WEST) {
                for edge in graph.ports[port].outgoing.clone() {
                    create_west_port_side_dummies(graph, port, edge, &mut unassigned);
                }
            }
        }
        // `node.setLayer(layer)`: each dummy is appended to the layer.
        for dummy in unassigned {
            graph.nodes[dummy].layer = Some(layer);
            graph.layers[layer].push(dummy);
        }
    }
}

/// `LNode.getPorts(side)`: the node's ports on `side`, in list order.
fn ports_on(graph: &LGraph, node: usize, side: PortSide) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .copied()
        .filter(|&port| graph.ports[port].side == side)
        .collect()
}

/// `createEastPortSideDummies`: the edge into the eastern input port ends at a dummy's western port
/// instead, and a new edge runs from the dummy's eastern port to the inverted port.
fn create_east_port_side_dummies(
    graph: &mut LGraph,
    eastward_port: usize,
    edge: usize,
    layer_nodes: &mut Vec<usize>,
) {
    // Self-loops are ignored.
    if graph.ports[graph.edges[edge].source].node == graph.ports[eastward_port].node {
        return;
    }
    let (dummy, dummy_input, dummy_output) = create_dummy(graph, edge);
    layer_nodes.push(dummy);
    // Reroute the original edge, and connect the dummy with the original port.
    graph.set_edge_target(edge, dummy_input);
    let dummy_edge = copy_edge(graph, edge, dummy_output, eastward_port);
    set_long_edge_source_and_target(graph, dummy, dummy_input, dummy_output);
    // Head labels move to the new edge and remember the edge they came from, unless they know it.
    for label in take_head_labels(graph, edge) {
        if graph.labels[label].end_label_edge.is_none() {
            graph.labels[label].end_label_edge = Some(edge);
        }
        graph.edges[dummy_edge].labels.push(label);
    }
}

/// `createWestPortSideDummies`: the edge out of the western output port ends at a dummy's western
/// port instead, and a new edge runs from the dummy's eastern port to the edge's old target.
fn create_west_port_side_dummies(
    graph: &mut LGraph,
    westward_port: usize,
    edge: usize,
    layer_nodes: &mut Vec<usize>,
) {
    // Self-loops are ignored.
    if graph.ports[graph.edges[edge].target].node == graph.ports[westward_port].node {
        return;
    }
    let (dummy, dummy_input, dummy_output) = create_dummy(graph, edge);
    layer_nodes.push(dummy);
    // Reroute the original edge, and connect the dummy with the original target.
    let original_target = graph.edges[edge].target;
    graph.set_edge_target(edge, dummy_input);
    let dummy_edge = copy_edge(graph, edge, dummy_output, original_target);
    // Head labels move to the new edge and remember the edge they belonged to.
    for label in take_head_labels(graph, edge) {
        graph.labels[label].end_label_edge = Some(edge);
        graph.edges[dummy_edge].labels.push(label);
    }
    set_long_edge_source_and_target(graph, dummy, dummy_input, dummy_output);
}

/// The dummy both cases make: a long edge dummy for `edge` with fixed port positions, a western
/// input port and an eastern output port (`new LPort()`, at the dummy's origin). Returns the dummy
/// and its two ports.
fn create_dummy(graph: &mut LGraph, edge: usize) -> (usize, usize, usize) {
    let mut node = LNode {
        node_type: NodeType::LONG_EDGE,
        origin_edge: Some(edge),
        ..LNode::default()
    };
    node.properties.set(
        options::PORT_CONSTRAINTS,
        Some(OptionValue::Enum {
            class: "PortConstraints",
            name: "FIXED_POS".to_string(),
        }),
    );
    graph.nodes.push(node);
    let dummy = graph.nodes.len() - 1;
    let mut ports = [0; 2];
    for (slot, side) in ports.iter_mut().zip([PortSide::WEST, PortSide::EAST]) {
        graph.ports.push(LPort::new(dummy, side));
        *slot = graph.ports.len() - 1;
        graph.nodes[dummy].ports.push(*slot);
    }
    (dummy, ports[0], ports[1])
}

/// `new LEdge()` with `copyProperties(edge)` and no junction points, from `source` to `target`.
/// Returns the new edge.
fn copy_edge(graph: &mut LGraph, edge: usize, source: usize, target: usize) -> usize {
    let original = &graph.edges[edge];
    let copy = LEdge {
        id: original.id.clone(),
        source,
        target,
        labels: Vec::new(),
        model_order: original.model_order,
        properties: original.properties.clone(),
        reversed: original.reversed,
        bend_points: Vec::new(),
        thickness: original.thickness,
        designation: original.designation.clone(),
    };
    graph.edges.push(copy);
    let new_edge = graph.edges.len() - 1;
    graph.ports[source].outgoing.push(new_edge);
    graph.ports[target].incoming.push(new_edge);
    new_edge
}

/// The labels of `edge` placed at its head, which leave the edge; the others stay, in order.
fn take_head_labels(graph: &mut LGraph, edge: usize) -> Vec<usize> {
    let labels = std::mem::take(&mut graph.edges[edge].labels);
    let (heads, kept): (Vec<usize>, Vec<usize>) = labels.into_iter().partition(|&id| {
        graph.labels[id]
            .properties
            .enum_with_default::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT)
            == EdgeLabelPlacement::HEAD
    });
    graph.edges[edge].labels = kept;
    heads
}

/// `setLongEdgeSourceAndTarget`: the dummy's long edge starts where the edge into it starts, or
/// where that edge's long edge dummy says its long edge starts; the same for the end.
fn set_long_edge_source_and_target(
    graph: &mut LGraph,
    dummy: usize,
    dummy_input: usize,
    dummy_output: usize,
) {
    let source = graph.ports[dummy_input]
        .incoming
        .first()
        .map(|&edge| graph.edges[edge].source);
    let target = graph.ports[dummy_output]
        .outgoing
        .first()
        .map(|&edge| graph.edges[edge].target);
    let long_edge_source = source.and_then(|port| {
        let node = &graph.nodes[graph.ports[port].node];
        if node.node_type == NodeType::LONG_EDGE {
            node.long_edge_source
        } else {
            Some(port)
        }
    });
    let long_edge_target = target.and_then(|port| {
        let node = &graph.nodes[graph.ports[port].node];
        if node.node_type == NodeType::LONG_EDGE {
            node.long_edge_target
        } else {
            Some(port)
        }
    });
    graph.nodes[dummy].long_edge_source = long_edge_source;
    graph.nodes[dummy].long_edge_target = long_edge_target;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::lgraph::LLabel;

    /// `count` nodes in the given layers.
    fn graph_with_layers(layers: Vec<Vec<usize>>, count: usize) -> LGraph {
        let mut graph = LGraph {
            nodes: vec![LNode::default(); count],
            ..LGraph::default()
        };
        for (index, layer) in layers.iter().enumerate() {
            for &node in layer {
                graph.nodes[node].layer = Some(index);
            }
        }
        graph.layers = layers;
        graph
    }

    fn fix_sides(graph: &mut LGraph, node: usize) {
        graph.nodes[node].properties.set(
            options::PORT_CONSTRAINTS,
            Some(OptionValue::Enum {
                class: "PortConstraints",
                name: "FIXED_SIDE".to_string(),
            }),
        );
    }

    fn add_port(graph: &mut LGraph, node: usize, side: PortSide) -> usize {
        graph.ports.push(LPort::new(node, side));
        let port = graph.ports.len() - 1;
        graph.nodes[node].ports.push(port);
        port
    }

    fn connect(graph: &mut LGraph, source: usize, target: usize) -> usize {
        graph.edges.push(LEdge {
            id: "e".to_string(),
            source,
            target,
            thickness: 1.0,
            ..LEdge::default()
        });
        let edge = graph.edges.len() - 1;
        graph.ports[source].outgoing.push(edge);
        graph.ports[target].incoming.push(edge);
        edge
    }

    #[test]
    fn an_edge_into_an_eastern_port_turns_at_a_dummy_in_the_layer() {
        // Node 0 in layer 0 feeds node 1's eastern port in layer 1; the edge has a head label.
        let mut graph = graph_with_layers(vec![vec![0], vec![1]], 2);
        fix_sides(&mut graph, 1);
        let out = add_port(&mut graph, 0, PortSide::EAST);
        let inverted = add_port(&mut graph, 1, PortSide::EAST);
        let edge = connect(&mut graph, out, inverted);
        let mut label = LLabel::default();
        label.properties.set(
            options::EDGE_LABELS_PLACEMENT,
            Some(OptionValue::Enum {
                class: "EdgeLabelPlacement",
                name: "HEAD".to_string(),
            }),
        );
        let label = graph.add_label(label);
        graph.edges[edge].labels.push(label);

        process(&mut graph);

        let dummy = 2;
        assert_eq!(graph.layers, vec![vec![0], vec![1, dummy]]);
        assert_eq!(graph.nodes[dummy].node_type, NodeType::LONG_EDGE);
        assert_eq!(graph.nodes[dummy].layer, Some(1));
        let (west, east) = (graph.nodes[dummy].ports[0], graph.nodes[dummy].ports[1]);
        assert_eq!(graph.ports[west].side, PortSide::WEST);
        assert_eq!(graph.ports[east].side, PortSide::EAST);
        // The edge ends at the dummy; a new edge joins the dummy with the inverted port.
        assert_eq!(graph.edges[edge].target, west);
        let new_edge = graph.edges.len() - 1;
        assert_eq!(graph.edges[new_edge].source, east);
        assert_eq!(graph.edges[new_edge].target, inverted);
        assert_eq!(graph.ports[inverted].incoming, vec![new_edge]);
        // The head label moved, and remembers its edge.
        assert!(graph.edges[edge].labels.is_empty());
        assert_eq!(graph.edges[new_edge].labels, vec![label]);
        assert_eq!(graph.labels[label].end_label_edge, Some(edge));
        assert_eq!(graph.nodes[dummy].long_edge_source, Some(out));
        assert_eq!(graph.nodes[dummy].long_edge_target, Some(inverted));
    }

    #[test]
    fn an_edge_out_of_a_western_port_turns_at_a_dummy_in_the_layer() {
        // Node 0's western port in layer 0 feeds node 1 in layer 1.
        let mut graph = graph_with_layers(vec![vec![0], vec![1]], 2);
        fix_sides(&mut graph, 0);
        let inverted = add_port(&mut graph, 0, PortSide::WEST);
        let target = add_port(&mut graph, 1, PortSide::WEST);
        let edge = connect(&mut graph, inverted, target);

        process(&mut graph);

        let dummy = 2;
        assert_eq!(graph.layers, vec![vec![0, dummy], vec![1]]);
        let (west, east) = (graph.nodes[dummy].ports[0], graph.nodes[dummy].ports[1]);
        assert_eq!(graph.edges[edge].source, inverted);
        assert_eq!(graph.edges[edge].target, west);
        let new_edge = graph.edges.len() - 1;
        assert_eq!(graph.edges[new_edge].source, east);
        assert_eq!(graph.edges[new_edge].target, target);
        assert_eq!(graph.nodes[dummy].long_edge_source, Some(inverted));
        assert_eq!(graph.nodes[dummy].long_edge_target, Some(target));
    }

    #[test]
    fn free_nodes_and_self_loops_are_left_alone() {
        // Node 1's sides are free, so its eastern input port is not inverted; node 0's self loop
        // from its western port to its eastern port is ignored.
        let mut graph = graph_with_layers(vec![vec![0], vec![1]], 2);
        fix_sides(&mut graph, 0);
        let loop_out = add_port(&mut graph, 0, PortSide::WEST);
        let loop_in = add_port(&mut graph, 0, PortSide::EAST);
        connect(&mut graph, loop_out, loop_in);
        let out = add_port(&mut graph, 0, PortSide::EAST);
        let free_in = add_port(&mut graph, 1, PortSide::EAST);
        connect(&mut graph, out, free_in);

        process(&mut graph);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.layers, vec![vec![0], vec![1]]);
    }
}
