//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/EdgeAndLayerConstraintEdgeReverser.java,
//!   LayerConstraintPreprocessor.java and LayerConstraintPostprocessor.java
//!
//! Layer constraints. The reverser turns the edges around the constrained nodes so that the first
//! ones leave them and the last ones enter them. The preprocessor hides the `FIRST_SEPARATE` and
//! `LAST_SEPARATE` nodes during layering. After layering, the postprocessor moves the `FIRST` and
//! `LAST` nodes into the first and last layers, with the label dummies on their edges, and puts the
//! hidden nodes into layers of their own before and after them.

use std::collections::HashMap;

use crate::core::data::JavaEnum;
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
        .enum_with_default::<LayerConstraint>(options::LAYER_CONSTRAINT)
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
            .enum_with_default::<crate::core::data::enums::PortConstraints>(
                options::PORT_CONSTRAINTS,
            );
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

/// `LayerConstraintPreprocessor.process`: the `FIRST_SEPARATE` and `LAST_SEPARATE` nodes leave the
/// graph until the layering is done, with their edges. A node left without edges by that takes the
/// constraint `FIRST` or `LAST`, when it has none of its own.
pub fn preprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    let mut hidden = Vec::new();
    let mut kept = Vec::new();
    let mut connections: HashMap<usize, HiddenNodeConnections> = HashMap::new();
    for node in graph.layerless_nodes.clone() {
        if matches!(
            constraint(graph, node),
            LayerConstraint::FIRST_SEPARATE | LayerConstraint::LAST_SEPARATE
        ) {
            hide(graph, node, &mut connections)?;
            hidden.push(node);
        } else {
            kept.push(node);
        }
    }
    graph.layerless_nodes = kept;
    graph.hidden_nodes = hidden;
    Ok(())
}

/// `LayerConstraintPreprocessor.HiddenNodeConnections`: the kinds of hidden nodes a node was
/// connected to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum HiddenNodeConnections {
    #[default]
    None,
    FirstSeparate,
    LastSeparate,
    Both,
}

impl HiddenNodeConnections {
    fn combine(self, layer_constraint: LayerConstraint) -> Self {
        let first = layer_constraint == LayerConstraint::FIRST_SEPARATE;
        match self {
            HiddenNodeConnections::None => {
                if first {
                    HiddenNodeConnections::FirstSeparate
                } else {
                    HiddenNodeConnections::LastSeparate
                }
            }
            HiddenNodeConnections::FirstSeparate => {
                if first {
                    HiddenNodeConnections::FirstSeparate
                } else {
                    HiddenNodeConnections::Both
                }
            }
            HiddenNodeConnections::LastSeparate => {
                if first {
                    HiddenNodeConnections::Both
                } else {
                    HiddenNodeConnections::LastSeparate
                }
            }
            HiddenNodeConnections::Both => HiddenNodeConnections::Both,
        }
    }
}

/// `hide(lNode)`: each edge of the node leaves the port at its other end.
fn hide(
    graph: &mut LGraph,
    node: usize,
    connections: &mut HashMap<usize, HiddenNodeConnections>,
) -> Result<(), LayoutError> {
    ensure_no_inacceptable_edges(graph, node)?;
    // `getConnectedEdges()`: port by port, the incoming edges and then the outgoing ones.
    let edges: Vec<usize> = graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| {
            let data = &graph.ports[port];
            data.incoming.iter().chain(data.outgoing.iter()).copied()
        })
        .collect();
    for edge in edges {
        let (source, target) = (graph.edges[edge].source, graph.edges[edge].target);
        let outgoing = graph.ports[source].node == node;
        // The edge leaves the list of the port at its other end, and keeps that port as its end.
        let opposite = if outgoing {
            remove_from(&mut graph.ports[target].incoming, edge);
            target
        } else {
            remove_from(&mut graph.ports[source].outgoing, edge);
            source
        };
        let opposite_node = graph.ports[opposite].node;
        update_opposite_node_layer_constraints(graph, node, opposite_node, connections);
    }
    Ok(())
}

fn remove_from(list: &mut Vec<usize>, edge: usize) {
    if let Some(index) = list.iter().position(|&found| found == edge) {
        list.remove(index);
    }
}

/// `updateOppositeNodeLayerConstraints`: a node whose last edge to the rest of the graph was hidden
/// goes to the first or the last layer, by the kind of hidden nodes it was connected to.
fn update_opposite_node_layer_constraints(
    graph: &mut LGraph,
    hidden: usize,
    opposite: usize,
    connections: &mut HashMap<usize, HiddenNodeConnections>,
) {
    if graph.nodes[opposite]
        .properties
        .has(options::LAYER_CONSTRAINT)
    {
        return;
    }
    let combined = connections
        .get(&opposite)
        .copied()
        .unwrap_or_default()
        .combine(constraint(graph, hidden));
    connections.insert(opposite, combined);
    let still_connected = graph.nodes[opposite].ports.iter().any(|&port| {
        !graph.ports[port].incoming.is_empty() || !graph.ports[port].outgoing.is_empty()
    });
    if still_connected {
        return;
    }
    let new_constraint = match combined {
        HiddenNodeConnections::FirstSeparate => LayerConstraint::FIRST,
        HiddenNodeConnections::LastSeparate => LayerConstraint::LAST,
        _ => return,
    };
    graph.nodes[opposite].properties.set(
        options::LAYER_CONSTRAINT,
        Some(crate::core::data::OptionValue::Enum {
            class: "LayerConstraint",
            name: new_constraint.java_name().to_string(),
        }),
    );
}

/// `ensureNoInacceptableEdges`: a `FIRST_SEPARATE` node must not have incoming edges, nor a
/// `LAST_SEPARATE` node outgoing ones, unless both ends are external port dummies.
fn ensure_no_inacceptable_edges(graph: &LGraph, node: usize) -> Result<(), LayoutError> {
    let layer_constraint = constraint(graph, node);
    let acceptable = |edge: usize| {
        let source = graph.ports[graph.edges[edge].source].node;
        let target = graph.ports[graph.edges[edge].target].node;
        graph.nodes[source].node_type == NodeType::EXTERNAL_PORT
            && graph.nodes[target].node_type == NodeType::EXTERNAL_PORT
    };
    let (edges, kind) = match layer_constraint {
        LayerConstraint::FIRST_SEPARATE => (node_edges(graph, node, true), "FIRST_SEPARATE"),
        LayerConstraint::LAST_SEPARATE => (node_edges(graph, node, false), "LAST_SEPARATE"),
        _ => return Ok(()),
    };
    if edges.into_iter().any(|edge| !acceptable(edge)) {
        return Err(LayoutError::Unsupported(format!(
            "node {} has its layer constraint set to {kind}, but has edges against it",
            graph.nodes[node].designation
        )));
    }
    Ok(())
}

/// `LayerConstraintPostprocessor.process`: the `FIRST` nodes go to the first layer and the `LAST`
/// nodes to the last one, with their label dummies in label layers at the two ends. Empty layers
/// are removed.
pub fn postprocess(graph: &mut LGraph) -> Result<(), LayoutError> {
    if !graph.layers.is_empty() {
        move_first_and_last_nodes(graph)?;
    }
    // `restoreHiddenNodes`: the separate nodes join new first and last layers, with their edges.
    if !graph.hidden_nodes.is_empty() {
        let mut first_separate = Vec::new();
        let mut last_separate = Vec::new();
        for hidden in std::mem::take(&mut graph.hidden_nodes) {
            match constraint(graph, hidden) {
                LayerConstraint::FIRST_SEPARATE => first_separate.push(hidden),
                LayerConstraint::LAST_SEPARATE => last_separate.push(hidden),
                _ => {}
            }
            restore_hidden_edges(graph, hidden);
        }
        if !first_separate.is_empty() {
            graph.layers.insert(0, first_separate);
        }
        if !last_separate.is_empty() {
            graph.layers.push(last_separate);
        }
    }
    for (index, layer) in graph.layers.iter().enumerate() {
        for &node in layer {
            graph.nodes[node].layer = Some(index);
        }
    }
    Ok(())
}

/// `restoreHiddenNodes` for the edges of one node: each edge that left the port at its other end
/// joins that port's list again. An edge between two hidden nodes was hidden only once.
fn restore_hidden_edges(graph: &mut LGraph, node: usize) {
    let edges: Vec<usize> = graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| {
            let data = &graph.ports[port];
            data.incoming.iter().chain(data.outgoing.iter()).copied()
        })
        .collect();
    for edge in edges {
        let (source, target) = (graph.edges[edge].source, graph.edges[edge].target);
        if !graph.ports[source].outgoing.contains(&edge) {
            graph.ports[source].outgoing.push(edge);
        } else if !graph.ports[target].incoming.contains(&edge) {
            graph.ports[target].incoming.push(edge);
        }
    }
}

/// `moveFirstAndLastNodes`: the `FIRST` nodes go to the first layer and the `LAST` nodes to the last
/// one, with their label dummies in label layers at the two ends. Empty layers are removed.
fn move_first_and_last_nodes(graph: &mut LGraph) -> Result<(), LayoutError> {
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

    /// A FIRST_SEPARATE node with one edge to a node: the node is hidden during layering, the other
    /// node takes FIRST, and the postprocessor puts the hidden node into a layer before it.
    #[test]
    fn separate_nodes_are_hidden_and_restored() {
        let mut graph = LGraph {
            nodes: vec![Default::default(), Default::default()],
            layerless_nodes: vec![0, 1],
            ..LGraph::default()
        };
        graph.nodes[0].properties.set(
            options::LAYER_CONSTRAINT,
            Some(crate::core::data::OptionValue::Enum {
                class: "LayerConstraint",
                name: "FIRST_SEPARATE".to_string(),
            }),
        );
        graph.ports = vec![
            crate::layered::lgraph::LPort::new(0, PortSide::EAST),
            crate::layered::lgraph::LPort::new(1, PortSide::WEST),
        ];
        graph.nodes[0].ports = vec![0];
        graph.nodes[1].ports = vec![1];
        graph.edges.push(crate::layered::lgraph::LEdge {
            source: 0,
            target: 1,
            ..Default::default()
        });
        graph.ports[0].outgoing.push(0);
        graph.ports[1].incoming.push(0);
        preprocess(&mut graph).unwrap();
        assert_eq!(graph.layerless_nodes, vec![1]);
        assert!(graph.ports[1].incoming.is_empty());
        assert_eq!(constraint(&graph, 1), LayerConstraint::FIRST);
        graph.layers = vec![vec![1]];
        graph.layerless_nodes.clear();
        postprocess(&mut graph).unwrap();
        assert_eq!(graph.layers, vec![vec![0], vec![1]]);
        assert_eq!(graph.ports[1].incoming, vec![0]);
    }
}
