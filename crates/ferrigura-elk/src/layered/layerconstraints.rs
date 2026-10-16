//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/EdgeAndLayerConstraintEdgeReverser.java,
//!   LayerConstraintPreprocessor.java and LayerConstraintPostprocessor.java
//!
//! Layer constraints `FIRST` and `LAST`. The reverser turns the edges around the constrained nodes
//! so that the first ones leave them and the last ones enter them. After layering, the postprocessor
//! moves the constrained nodes into the first and last layers, with the label dummies on their
//! edges. `FIRST_SEPARATE` and `LAST_SEPARATE` nodes are hidden during layering by the Java
//! preprocessor; that is not ported, so the preprocessor refuses them.

use crate::core::data::enums::{LayerConstraint, NodeType, PortSide};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::p1cycles::reverse_edge;
use crate::layered::ports::is_side_fixed;

/// `LayerConstraint` of a node, `NONE` when it has none.
fn constraint(graph: &LGraph, node: usize) -> LayerConstraint {
    graph.nodes[node]
        .properties
        .enum_of::<LayerConstraint>(options::LAYER_CONSTRAINT)
        .unwrap_or(LayerConstraint::NONE)
}

fn is_label(graph: &LGraph, node: usize) -> bool {
    graph.nodes[node].node_type == NodeType::LABEL
}

/// Which of a node's edges a reversal looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reversal {
    outgoing: bool,
    incoming: bool,
}

/// `EdgeAndLayerConstraintEdgeReverser.process`: the edges around the constrained nodes are
/// reversed where the constraints need it, and the sides of the ports that fit the flow are
/// reversed too.
pub fn reverse_edges(graph: &mut LGraph) -> Result<(), LayoutError> {
    let remaining = handle_outer_nodes(graph);
    handle_inner_nodes(graph, &remaining);
    Ok(())
}

/// `handleOuterNodes`: the constrained nodes. A `FIRST` node leaves the graph by its outgoing edges,
/// so its incoming edges are reversed; a `LAST` node takes its incoming edges, so its outgoing ones
/// are reversed. The other nodes are kept for `handleInnerNodes`.
fn handle_outer_nodes(graph: &mut LGraph) -> Vec<usize> {
    let mut remaining = Vec::new();
    for node in graph.layerless_nodes.clone() {
        let layer_constraint = constraint(graph, node);
        match layer_constraint {
            LayerConstraint::FIRST | LayerConstraint::FIRST_SEPARATE => {
                reverse_node_edges(
                    graph,
                    node,
                    layer_constraint,
                    Reversal {
                        outgoing: false,
                        incoming: true,
                    },
                );
            }
            LayerConstraint::LAST | LayerConstraint::LAST_SEPARATE => {
                reverse_node_edges(
                    graph,
                    node,
                    layer_constraint,
                    Reversal {
                        outgoing: true,
                        incoming: false,
                    },
                );
            }
            LayerConstraint::NONE => remaining.push(node),
        }
    }
    remaining
}

/// `handleInnerNodes`: an unconstrained node with fixed port sides, all of whose ports already
/// point the way the flow goes, has its edges reversed when no edge contradicts that.
fn handle_inner_nodes(graph: &mut LGraph, remaining: &[usize]) {
    for &node in remaining {
        let constraints = graph.nodes[node]
            .properties
            .enum_of::<crate::core::data::enums::PortConstraints>(options::PORT_CONSTRAINTS)
            .unwrap_or(crate::core::data::enums::PortConstraints::UNDEFINED);
        if !is_side_fixed(constraints) || graph.nodes[node].ports.is_empty() {
            continue;
        }
        let mut all_reversed = true;
        for port in graph.nodes[node].ports.clone() {
            let net_flow =
                graph.ports[port].incoming.len() as i64 - graph.ports[port].outgoing.len() as i64;
            let side = graph.ports[port].side;
            if !((side == PortSide::EAST && net_flow > 0)
                || (side == PortSide::WEST && net_flow < 0))
            {
                all_reversed = false;
                break;
            }
            for edge in graph.ports[port].outgoing.clone() {
                let target = graph.ports[graph.edges[edge].target].node;
                if matches!(
                    constraint(graph, target),
                    LayerConstraint::LAST | LayerConstraint::LAST_SEPARATE
                ) {
                    all_reversed = false;
                    break;
                }
            }
            for edge in graph.ports[port].incoming.clone() {
                let source = graph.ports[graph.edges[edge].source].node;
                if matches!(
                    constraint(graph, source),
                    LayerConstraint::FIRST | LayerConstraint::FIRST_SEPARATE
                ) {
                    all_reversed = false;
                    break;
                }
            }
        }
        if all_reversed {
            reverse_node_edges(
                graph,
                node,
                LayerConstraint::NONE,
                Reversal {
                    outgoing: true,
                    incoming: true,
                },
            );
        }
    }
}

/// `reverseEdges`: for each port of the node, in order, reverses the edges the reversal asks for
/// that the constraints allow. The lists are copied before the edges move, as in Java.
fn reverse_node_edges(
    graph: &mut LGraph,
    node: usize,
    layer_constraint: LayerConstraint,
    reversal: Reversal,
) {
    for port in graph.nodes[node].ports.clone() {
        if reversal.outgoing {
            for edge in graph.ports[port].outgoing.clone() {
                if can_reverse_outgoing(graph, layer_constraint, edge) {
                    reverse_edge(graph, edge);
                }
            }
        }
        if reversal.incoming {
            for edge in graph.ports[port].incoming.clone() {
                if can_reverse_incoming(graph, layer_constraint, edge) {
                    reverse_edge(graph, edge);
                }
            }
        }
    }
}

/// `canReverseOutgoingEdge`.
fn can_reverse_outgoing(graph: &LGraph, source_constraint: LayerConstraint, edge: usize) -> bool {
    if graph.edges[edge].reversed {
        return false;
    }
    let target = graph.ports[graph.edges[edge].target].node;
    if source_constraint == LayerConstraint::LAST && is_label(graph, target) {
        return false;
    }
    constraint(graph, target) != LayerConstraint::LAST_SEPARATE
}

/// `canReverseIncomingEdge`.
fn can_reverse_incoming(graph: &LGraph, target_constraint: LayerConstraint, edge: usize) -> bool {
    if graph.edges[edge].reversed {
        return false;
    }
    let source = graph.ports[graph.edges[edge].source].node;
    if target_constraint == LayerConstraint::FIRST && is_label(graph, source) {
        return false;
    }
    constraint(graph, source) != LayerConstraint::FIRST_SEPARATE
}

/// `LayerConstraintPreprocessor.process`. The separate constraints would hide their nodes until the
/// layering is done; that is not ported, so such a graph is refused. Without them nothing is hidden.
pub fn preprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    let separate = graph.layerless_nodes.iter().any(|&node| {
        matches!(
            constraint(graph, node),
            LayerConstraint::FIRST_SEPARATE | LayerConstraint::LAST_SEPARATE
        )
    });
    if separate {
        return Err(LayoutError::Unsupported(
            "layer constraints FIRST_SEPARATE and LAST_SEPARATE (hidden nodes)".to_string(),
        ));
    }
    Ok(())
}

/// `LayerConstraintPostprocessor.process`: the `FIRST` nodes go to the first layer and the `LAST`
/// nodes to the last one, with their label dummies in label layers at the two ends. Empty layers
/// are removed.
pub fn postprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    if graph.layers.is_empty() {
        return Ok(());
    }
    // The label layers are placeholders at both ends, so that the first and last layers keep their
    // place while the nodes move; they stay only if something ends up in them.
    let count = graph.layers.len();
    graph.layers.insert(0, Vec::new());
    graph.layers.push(Vec::new());
    let (first_label_layer, first_layer) = (0, 1);
    let (last_layer, last_label_layer) = (count, count + 1);

    for layer in first_layer..=count {
        for node in graph.layers[layer].clone() {
            match constraint(graph, node) {
                LayerConstraint::FIRST => {
                    for edge in node_edges(graph, node, true) {
                        let source = graph.ports[graph.edges[edge].source].node;
                        if !is_label(graph, source) {
                            return Err(LayoutError::Unsupported(format!(
                                "node {} has layer constraint FIRST and an incoming edge that does not come from a label",
                                graph.node_name(node)
                            )));
                        }
                    }
                    move_node(graph, node, first_layer);
                    move_labels(graph, node, true, first_label_layer);
                }
                LayerConstraint::LAST => {
                    for edge in node_edges(graph, node, false) {
                        let target = graph.ports[graph.edges[edge].target].node;
                        if !is_label(graph, target) {
                            return Err(LayoutError::Unsupported(format!(
                                "node {} has layer constraint LAST and an outgoing edge that does not go to a label",
                                graph.node_name(node)
                            )));
                        }
                    }
                    move_node(graph, node, last_layer);
                    move_labels(graph, node, false, last_label_layer);
                }
                _ => {}
            }
        }
    }
    graph.layers.retain(|layer| !layer.is_empty());
    for (index, layer) in graph.layers.iter().enumerate() {
        for &node in layer {
            graph.nodes[node].layer = Some(index);
        }
    }
    Ok(())
}

/// The incoming (or outgoing) edges of a node, port by port.
fn node_edges(graph: &LGraph, node: usize, incoming: bool) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| {
            if incoming {
                graph.ports[port].incoming.clone()
            } else {
                graph.ports[port].outgoing.clone()
            }
        })
        .collect()
}

/// `LNode.setLayer(layer)`: the node leaves its layer and joins the end of `target`.
fn move_node(graph: &mut LGraph, node: usize, target: usize) {
    for layer in graph.layers.iter_mut() {
        if let Some(position) = layer.iter().position(|&member| member == node) {
            layer.remove(position);
            break;
        }
    }
    graph.layers[target].push(node);
}

/// `moveLabelsToLabelLayer`: the label dummies at the other end of the node's edges join the label
/// layer.
fn move_labels(graph: &mut LGraph, node: usize, incoming: bool, label_layer: usize) {
    for edge in node_edges(graph, node, incoming) {
        let other = if incoming {
            graph.ports[graph.edges[edge].source].node
        } else {
            graph.ports[graph.edges[edge].target].node
        };
        if is_label(graph, other) {
            move_node(graph, other, label_layer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separate_constraints_are_refused() {
        let mut graph = LGraph::default();
        graph.nodes.push(Default::default());
        graph.layerless_nodes.push(0);
        graph.nodes[0].properties.set(
            options::LAYER_CONSTRAINT,
            Some(crate::core::data::OptionValue::Enum {
                class: "LayerConstraint",
                name: "FIRST_SEPARATE".to_string(),
            }),
        );
        assert!(preprocess(&mut graph).is_err());
    }
}
