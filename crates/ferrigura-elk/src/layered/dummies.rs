//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/LabelDummyInserter.java
//!   and LongEdgeSplitter.java (splitEdge, setDummyNodeProperties and moveHeadLabels)
//!
//! Label dummies. An edge with centred labels is split at a node that holds the labels, so
//! that layering and routing leave them room. `split_edge` is the splitting that the long edge
//! splitter does too.

use crate::core::data::OptionValue;
use crate::core::data::enums::{Direction, EdgeLabelPlacement, NodeType, PortSide};
use crate::core::math::KVector;
use crate::layered::lgraph::{LEdge, LGraph, LLabel, LNode, LPort};
use crate::layered::options;

/// `LabelDummyInserter.process`: every edge with centred labels that joins two different nodes
/// gets a label dummy that holds those labels. The labels leave the edge, and the edge is split
/// at the dummy. The dummies join the layerless nodes at the end.
pub fn insert_label_dummies(graph: &mut LGraph) {
    let edge_label_spacing = graph
        .properties
        .double_of(options::SPACING_EDGE_LABEL)
        .unwrap_or(0.0);
    let label_label_spacing = graph
        .properties
        .double_of(options::SPACING_LABEL_LABEL)
        .unwrap_or(0.0);
    let vertical = matches!(
        graph.properties.enum_of::<Direction>(options::DIRECTION),
        Some(Direction::UP | Direction::DOWN)
    );
    let mut new_dummies = Vec::new();
    for node in graph.layerless_nodes.clone() {
        for edge in outgoing_edges(graph, node) {
            if !needs_label_dummy(graph, edge) {
                continue;
            }
            let thickness = edge_thickness(graph, edge);
            let dummy = create_label_dummy(graph, edge, thickness);
            new_dummies.push(dummy);
            let mut size = graph.nodes[dummy].size;
            let mut represented = Vec::new();
            let mut kept = Vec::new();
            for label in std::mem::take(&mut graph.edges[edge].labels) {
                if is_centred(&label) {
                    if vertical {
                        size.x += label.size.x + label_label_spacing;
                        size.y = size.y.max(label.size.y);
                    } else {
                        size.x = size.x.max(label.size.x);
                        size.y += label.size.y + label_label_spacing;
                    }
                    represented.push(label);
                } else {
                    kept.push(label);
                }
            }
            graph.edges[edge].labels = kept;
            if vertical {
                size.x -= label_label_spacing;
                size.y += edge_label_spacing + thickness;
            } else {
                size.y += edge_label_spacing - label_label_spacing + thickness;
            }
            graph.nodes[dummy].size = size;
            graph.nodes[dummy].represented_labels = represented;
        }
    }
    graph.layerless_nodes.extend(new_dummies);
}

/// `LabelDummyInserter.edgeNeedsToBeProcessed`: the edge joins two nodes and has a centred label.
fn needs_label_dummy(graph: &LGraph, edge: usize) -> bool {
    let edge = &graph.edges[edge];
    graph.ports[edge.source].node != graph.ports[edge.target].node
        && edge.labels.iter().any(is_centred)
}

fn is_centred(label: &LLabel) -> bool {
    label
        .properties
        .enum_of::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT)
        == Some(EdgeLabelPlacement::CENTER)
}

/// `retrieveThickness`: a negative thickness becomes zero.
fn edge_thickness(graph: &mut LGraph, edge: usize) -> f64 {
    let thickness = graph.edges[edge].thickness;
    if thickness < 0.0 {
        graph.edges[edge].thickness = 0.0;
        0.0
    } else {
        thickness
    }
}

/// `LabelDummyInserter.createLabelDummy`: a label dummy in place of the edge's split, its
/// ports at half the thickness.
fn create_label_dummy(graph: &mut LGraph, edge: usize, thickness: f64) -> usize {
    let dummy = new_dummy_node(graph, NodeType::LABEL, edge);
    graph.nodes[dummy].long_edge_source = Some(graph.edges[edge].source);
    graph.nodes[dummy].long_edge_target = Some(graph.edges[edge].target);
    split_edge(graph, edge, dummy);
    let port_position = (thickness / 2.0).floor();
    for port in graph.nodes[dummy].ports.clone() {
        graph.ports[port].position.y = port_position;
    }
    dummy
}

/// `LongEdgeSplitter.splitEdge`: the edge ends at a new input port of `dummy`, and a new edge
/// leaves a new output port of `dummy` for the edge's old target. Returns the new edge.
pub fn split_edge(graph: &mut LGraph, edge: usize, dummy: usize) -> usize {
    let old_target = graph.edges[edge].target;
    let thickness = edge_thickness(graph, edge);
    graph.nodes[dummy].size.y = thickness;
    let port_position = (thickness / 2.0).floor();
    let dummy_input = push_port(graph, dummy, PortSide::WEST, port_position);
    let dummy_output = push_port(graph, dummy, PortSide::EAST, port_position);
    graph.set_edge_target(edge, dummy_input);
    let original = graph.edges[edge].clone();
    let new_edge = graph.edges.len();
    graph.edges.push(LEdge {
        id: original.id,
        source: dummy_output,
        target: old_target,
        labels: Vec::new(),
        model_order: original.model_order,
        properties: original.properties,
        reversed: original.reversed,
        bend_points: Vec::new(),
        thickness: original.thickness,
    });
    graph.ports[dummy_output].outgoing.push(new_edge);
    graph.ports[old_target].incoming.push(new_edge);
    set_dummy_node_properties(graph, dummy, edge, new_edge);
    move_head_labels(graph, edge, new_edge);
    new_edge
}

/// `LongEdgeSplitter.setDummyNodeProperties`: the long edge's endpoints and whether it has label
/// dummies, taken from the neighbouring node that already carries them, if there is one.
fn set_dummy_node_properties(graph: &mut LGraph, dummy: usize, in_edge: usize, out_edge: usize) {
    let in_source = graph.ports[graph.edges[in_edge].source].node;
    let out_target = graph.ports[graph.edges[out_edge].target].node;
    let source_type = graph.nodes[in_source].node_type;
    if source_type == NodeType::LONG_EDGE || source_type == NodeType::LABEL {
        let neighbour = &graph.nodes[in_source];
        let (source, target) = (neighbour.long_edge_source, neighbour.long_edge_target);
        let has_label_dummies =
            source_type == NodeType::LABEL || neighbour.long_edge_has_label_dummies;
        set_long_edge(graph, dummy, source, target, has_label_dummies);
    } else if graph.nodes[out_target].node_type == NodeType::LABEL {
        let neighbour = &graph.nodes[out_target];
        let (source, target) = (neighbour.long_edge_source, neighbour.long_edge_target);
        set_long_edge(graph, dummy, source, target, true);
    } else {
        let source = Some(graph.edges[in_edge].source);
        let target = Some(graph.edges[out_edge].target);
        graph.nodes[dummy].long_edge_source = source;
        graph.nodes[dummy].long_edge_target = target;
    }
}

fn set_long_edge(
    graph: &mut LGraph,
    dummy: usize,
    source: Option<usize>,
    target: Option<usize>,
    has_label_dummies: bool,
) {
    let node = &mut graph.nodes[dummy];
    node.long_edge_source = source;
    node.long_edge_target = target;
    node.long_edge_has_label_dummies = has_label_dummies;
}

/// `LongEdgeSplitter.moveHeadLabels`: the head labels move to the new edge, which is where the
/// head is now, and remember the edge they came from.
fn move_head_labels(graph: &mut LGraph, old: usize, new: usize) {
    let labels = std::mem::take(&mut graph.edges[old].labels);
    let (heads, kept): (Vec<LLabel>, Vec<LLabel>) = labels.into_iter().partition(|label| {
        label
            .properties
            .enum_of::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT)
            == Some(EdgeLabelPlacement::HEAD)
    });
    graph.edges[old].labels = kept;
    for mut label in heads {
        if label.end_label_edge.is_none() {
            label.end_label_edge = Some(old);
        }
        graph.edges[new].labels.push(label);
    }
}

/// A new node of `node_type` for `origin`, with fixed port positions.
fn new_dummy_node(graph: &mut LGraph, node_type: NodeType, origin: usize) -> usize {
    let mut node = LNode {
        node_type,
        origin_edge: Some(origin),
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
    graph.nodes.len() - 1
}

/// A port of `node` on `side`, at `y` (`new LPort()` with `setNode`, as the splitter makes them).
fn push_port(graph: &mut LGraph, node: usize, side: PortSide, y: f64) -> usize {
    let mut port = LPort::new(node, side);
    port.position = KVector { x: 0.0, y };
    graph.ports.push(port);
    let index = graph.ports.len() - 1;
    graph.nodes[node].ports.push(index);
    index
}

/// `LNode.getOutgoingEdges()`: the outgoing edges of the node's ports, port by port.
fn outgoing_edges(graph: &LGraph, node: usize) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| graph.ports[port].outgoing.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::layered::graph_util::create_port;
    use crate::layered::lgraph::PortType;

    /// Two nodes, DOWN, and one edge from the first to the second with a centred label of
    /// 83 by 21 on it.
    fn graph_with_labelled_edge() -> LGraph {
        let mut graph = LGraph {
            nodes: vec![LNode::default(), LNode::default()],
            layerless_nodes: vec![0, 1],
            ..LGraph::default()
        };
        graph.properties.set(
            options::DIRECTION,
            Some(OptionValue::Enum {
                class: "Direction",
                name: "DOWN".to_string(),
            }),
        );
        graph
            .properties
            .set(options::SPACING_EDGE_LABEL, Some(OptionValue::Double(2.0)));
        graph
            .properties
            .set(options::SPACING_LABEL_LABEL, Some(OptionValue::Double(1.0)));
        let source = create_port(&mut graph, 0, PortType::OUTPUT);
        let target = create_port(&mut graph, 1, PortType::INPUT);
        let mut label = LLabel {
            text: "x".to_string(),
            size: KVector { x: 83.0, y: 21.0 },
            ..LLabel::default()
        };
        label.properties.set(
            options::EDGE_LABELS_PLACEMENT,
            Some(OptionValue::Enum {
                class: "EdgeLabelPlacement",
                name: "CENTER".to_string(),
            }),
        );
        graph.ports[source].outgoing.push(0);
        graph.ports[target].incoming.push(0);
        graph.edges.push(LEdge {
            source,
            target,
            labels: vec![label],
            thickness: 1.0,
            ..LEdge::default()
        });
        graph
    }

    #[test]
    fn a_centred_label_gets_a_dummy_between_its_nodes() {
        let mut graph = graph_with_labelled_edge();
        insert_label_dummies(&mut graph);
        assert_eq!(graph.nodes.len(), 3);
        let dummy = 2;
        assert_eq!(graph.nodes[dummy].node_type, NodeType::LABEL);
        assert_eq!(graph.nodes[dummy].represented_labels.len(), 1);
        assert_eq!(graph.layerless_nodes, vec![0, 1, dummy]);
        // Vertical: the width is the label's, the height adds the spacing and the thickness.
        assert_eq!(
            graph.nodes[dummy].size,
            KVector {
                x: 83.0,
                y: 21.0 + 2.0 + 1.0
            }
        );
        // The edge now ends at the dummy, and a second edge leaves the dummy for the old target.
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges[0].labels.is_empty());
        assert_eq!(graph.ports[graph.edges[0].target].node, dummy);
        assert_eq!(graph.ports[graph.edges[1].source].node, dummy);
        assert_eq!(graph.ports[graph.edges[1].target].node, 1);
    }
}
