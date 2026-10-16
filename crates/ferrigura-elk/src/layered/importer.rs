//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphImporter.java
//!
//! The layered graph from the ELK graph: `ElkGraphImporter` and `ElkGraphTransformer.importGraph`.
//!
//! Graphs of plain nodes, with their inside labels, and edges between them, self loops included.
//! Ports on nodes and nested graphs are reported as unsupported.

use std::collections::HashMap;

use crate::core::data::OptionValue;
use crate::core::data::enums::{
    ComponentOrderingStrategy, EdgeLabelPlacement, GraphProperties, HierarchyHandling,
    OrderingStrategy, PortConstraints,
};
use crate::core::data::property::PropertyHolder;
use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::{ElkEdge, ElkNode};
use crate::layered::graph_util::create_port;
use crate::layered::lgraph::{LEdge, LGraph, LLabel, LNode, LabelId, PortType};
use crate::layered::nodespacing;
use crate::layered::options;
use crate::layered::spacings::{SPACING_BASE_VALUE, apply_base_value};

/// `importGraph` for a root whose children are plain nodes, joined by edges.
pub fn import_root(root: &ElkNode) -> Result<LGraph, LayoutError> {
    import_level(root, None, &root.id)
}

/// `ElkGraphImporter.importGraph` for the level of `root`: its children, as plain nodes, joined by
/// edges. `parent` holds the options of `root`'s parent, which the Java reads for the spacings of
/// `root`'s inside labels and for its minimum size; it is `None` for the root of the graph. `chain`
/// is `root`'s designation prefix: the ids from the root of the graph down to `root`.
pub fn import_level(
    root: &ElkNode,
    parent: Option<&PropertyHolder>,
    chain: &str,
) -> Result<LGraph, LayoutError> {
    let mut graph = LGraph {
        properties: root.properties.clone(),
        root_id: chain.to_string(),
        ..LGraph::default()
    };
    // `ElkGraphImporter`: the root's padding is the margin the drawing keeps inside the root node.
    graph.padding = match root.properties.value_or_default(options::PADDING) {
        Some(OptionValue::Padding(spacing)) => spacing,
        _ => {
            return Err(LayoutError::Unsupported(
                "a root without a padding value".to_string(),
            ));
        }
    };
    // `createLGraph`: the padding also leaves room for the root's own inside labels. Without labels
    // that room is zero, so the node spacing code is not asked.
    if !root.labels.is_empty() {
        let label_padding = nodespacing::inside_label_padding(&label_probe(root, parent), 0, true)?;
        graph.padding.top += label_padding.top;
        graph.padding.left += label_padding.left;
        graph.padding.bottom += label_padding.bottom;
        graph.padding.right += label_padding.right;
    }
    // `calculateMinimumGraphSize`: a nested graph's node with size constraints takes a minimum size
    // from its labels, and the configured minimum, and the graph gets the MINIMUM_SIZE constraint.
    // The Java also writes the result into the node's own minimum, through a shared object. That is
    // not reproduced: the layout's size is already at least that minimum, so the node's size does
    // not change either way.
    if let Some(parent) = parent
        && !root
            .properties
            .enum_set_is_empty(options::NODE_SIZE_CONSTRAINTS)
    {
        let probe = label_probe(root, Some(parent));
        let minimum = nodespacing::minimum_size_of(&probe, 0)?;
        let configured = match graph
            .properties
            .value_or_default(options::NODE_SIZE_MINIMUM)
        {
            Some(OptionValue::Vector(vector)) => vector,
            _ => KVector::default(),
        };
        let mut names = match root.properties.get(options::NODE_SIZE_CONSTRAINTS) {
            Some(OptionValue::EnumSet { names, .. }) => names.clone(),
            _ => Vec::new(),
        };
        if !names.iter().any(|name| name == "MINIMUM_SIZE") {
            names.push("MINIMUM_SIZE".to_string());
        }
        graph.properties.set(
            options::NODE_SIZE_CONSTRAINTS,
            Some(OptionValue::EnumSet {
                class: "SizeConstraint",
                names,
            }),
        );
        graph.properties.set(
            options::NODE_SIZE_MINIMUM,
            Some(OptionValue::Vector(KVector {
                x: minimum.x.max(configured.x),
                y: minimum.y.max(configured.y),
            })),
        );
    }
    let needs_model_order = needs_model_order_based_on_parent(root);
    let mut model_order = 0;
    for child in &root.children {
        // A child with children is a leaf of this level once it was laid out on its own, which its
        // hierarchy handling says. Any other child with children is a nested graph of this level.
        if !child.children.is_empty()
            && child
                .properties
                .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
                != Some(HierarchyHandling::SEPARATE_CHILDREN)
        {
            return Err(LayoutError::Unsupported(format!(
                "nested graph in {}",
                child.id
            )));
        }
        if !child.ports.is_empty() {
            return Err(LayoutError::Unsupported(format!("ports on {}", child.id)));
        }
        // `needsModelOrder(child)`: the parent asks for the input order, and the child does not opt out.
        let ordered = needs_model_order
            && !child
                .properties
                .bool_of(options::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER);
        let node_model_order = ordered.then(|| {
            model_order += 1;
            model_order - 1
        });
        // `transformNode`: a node without port constraints is free; any other constraint is
        // not supported by the port sides yet.
        let mut properties = child.properties.clone();
        match properties.enum_of::<PortConstraints>(options::PORT_CONSTRAINTS) {
            None | Some(PortConstraints::UNDEFINED) => properties.set(
                options::PORT_CONSTRAINTS,
                Some(OptionValue::Enum {
                    class: "PortConstraints",
                    name: "FREE".to_string(),
                }),
            ),
            Some(PortConstraints::FREE) => {}
            Some(_) => {
                graph
                    .graph_properties
                    .insert(GraphProperties::NON_FREE_PORTS);
            }
        }
        let labels = import_node_labels(child, &mut graph);
        // `LNode.getDesignation()`: the first label's text when there is one, else the ids of the
        // ELK node and its parent.
        let designation = match labels.first() {
            Some(&first) => graph.labels[first].text.clone(),
            None => format!("{chain}.{}", child.id),
        };
        graph.nodes.push(LNode {
            id: child.id.clone(),
            designation,
            properties,
            size: KVector {
                x: child.shape.width,
                y: child.shape.height,
            },
            model_order: node_model_order,
            labels,
            ..LNode::default()
        });
    }
    graph.layerless_nodes = (0..graph.nodes.len()).collect();
    // `ElkGraphImporter.importGraph`: the base value derives the spacings before the graph is laid out.
    if root.properties.has(SPACING_BASE_VALUE)
        && let Some(base) = root.properties.double_of(SPACING_BASE_VALUE)
    {
        apply_base_value(&mut graph, base);
    }
    import_edges(root, &mut graph, needs_model_order)?;
    Ok(graph)
}

/// `transformNode`'s labels: those with text and laid out, each with its origin, the index among the
/// ELK node's labels, so the position can be written back to the right label.
fn import_node_labels(node: &ElkNode, graph: &mut LGraph) -> Vec<LabelId> {
    let mut labels = Vec::new();
    for (index, label) in node.labels.iter().enumerate() {
        let text = label.text.clone().unwrap_or_default();
        if text.is_empty() || label.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let id = graph.add_label(LLabel {
            text,
            size: KVector {
                x: label.shape.width,
                y: label.shape.height,
            },
            position: KVector {
                x: label.shape.x,
                y: label.shape.y,
            },
            properties: label.properties.clone(),
            end_label_edge: None,
            origin: Some(index),
        });
        labels.push(id);
    }
    labels
}

/// The graph a node's inside labels are measured in: the node alone, with all of its labels (the
/// Java's ELK node adapter gives every label, even one without text), and the properties of its
/// parent, which is where the Java reads the spacings (none for a root).
fn label_probe(node: &ElkNode, parent: Option<&PropertyHolder>) -> LGraph {
    let mut probe = LGraph {
        properties: parent.cloned().unwrap_or_default(),
        ..LGraph::default()
    };
    let mut labels = Vec::new();
    for label in &node.labels {
        labels.push(probe.add_label(LLabel {
            text: label.text.clone().unwrap_or_default(),
            size: KVector {
                x: label.shape.width,
                y: label.shape.height,
            },
            position: KVector {
                x: label.shape.x,
                y: label.shape.y,
            },
            properties: label.properties.clone(),
            end_label_edge: None,
            origin: None,
        }));
    }
    probe.nodes.push(LNode {
        id: node.id.clone(),
        properties: node.properties.clone(),
        size: KVector {
            x: node.shape.width,
            y: node.shape.height,
        },
        labels,
        ..LNode::default()
    });
    probe
}

/// `importFlatGraph` for the root's edges: each edge gets a port of its own on its source and
/// on its target node, as `transformEdge` creates them for edges that meet nodes.
fn import_edges(
    root: &ElkNode,
    graph: &mut LGraph,
    needs_model_order: bool,
) -> Result<(), LayoutError> {
    let index_of: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let mut model_order = 0;
    for edge in &root.edges {
        // The model order counts every edge of the graph, laid out or not.
        let edge_model_order = needs_model_order.then(|| {
            model_order += 1;
            model_order - 1
        });
        if edge.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
            return Err(LayoutError::Unsupported(format!(
                "edge {} without a source or a target",
                edge.id
            )));
        };
        let (Some(&source_node), Some(&target_node)) =
            (index_of.get(source.as_str()), index_of.get(target.as_str()))
        else {
            return Err(LayoutError::Unsupported(format!(
                "edge {} to a node outside the root",
                edge.id
            )));
        };
        // `LGraphUtil.computeGraphProperties`: an edge from a node to itself is a self loop.
        if source_node == target_node {
            graph.graph_properties.insert(GraphProperties::SELF_LOOPS);
        }
        let source_port = create_port(graph, source_node, PortType::OUTPUT);
        let target_port = create_port(graph, target_node, PortType::INPUT);
        let index = graph.edges.len();
        graph.ports[source_port].outgoing.push(index);
        graph.ports[target_port].incoming.push(index);
        let labels = import_edge_labels(edge, graph);
        graph.edges.push(LEdge {
            id: edge.id.clone(),
            source: source_port,
            target: target_port,
            labels,
            model_order: edge_model_order,
            properties: edge.properties.clone(),
            reversed: false,
            bend_points: Vec::new(),
            thickness: 1.0,
        });
    }
    Ok(())
}

/// `transformEdge`'s labels: those with text and laid out, with their placement recorded. A
/// centred label makes the graph need label dummies, a head or tail one end labels.
fn import_edge_labels(edge: &ElkEdge, graph: &mut LGraph) -> Vec<LabelId> {
    let mut labels = Vec::new();
    for (index, label) in edge.labels.iter().enumerate() {
        let text = label.text.clone().unwrap_or_default();
        if text.is_empty() || label.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let placement = label
            .properties
            .enum_of::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT)
            .unwrap_or(EdgeLabelPlacement::CENTER);
        let mut properties = label.properties.clone();
        match placement {
            EdgeLabelPlacement::HEAD | EdgeLabelPlacement::TAIL => {
                graph.graph_properties.insert(GraphProperties::END_LABELS);
            }
            EdgeLabelPlacement::CENTER => {
                graph
                    .graph_properties
                    .insert(GraphProperties::CENTER_LABELS);
                properties.set(
                    options::EDGE_LABELS_PLACEMENT,
                    Some(OptionValue::Enum {
                        class: "EdgeLabelPlacement",
                        name: "CENTER".to_string(),
                    }),
                );
            }
        }
        let id = graph.add_label(LLabel {
            text,
            size: KVector {
                x: label.shape.width,
                y: label.shape.height,
            },
            position: KVector::default(),
            properties,
            end_label_edge: None,
            origin: Some(index),
        });
        labels.push(id);
    }
    labels
}

/// `ElkGraphImporter.needsModelOrderBasedOnParent`: whether the graph's options use the input order.
fn needs_model_order_based_on_parent(root: &ElkNode) -> bool {
    let props = &root.properties;
    let strategy_is = |id: &str, name: &str| {
        props.value_or_default(id).is_some_and(|value| match value {
            crate::core::data::OptionValue::Enum { name: found, .. } => found == name,
            _ => false,
        })
    };
    let considers_order = props
        .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
        .is_some_and(|strategy| strategy != OrderingStrategy::NONE);
    considers_order
        || strategy_is(options::CYCLE_BREAKING_STRATEGY, "MODEL_ORDER")
        || strategy_is(options::CYCLE_BREAKING_STRATEGY, "GREEDY_MODEL_ORDER")
        || props.bool_of(options::FORCE_NODE_MODEL_ORDER)
        || props
            .enum_of::<ComponentOrderingStrategy>(options::CONSIDER_MODEL_ORDER_COMPONENTS)
            .is_some_and(|strategy| strategy != ComponentOrderingStrategy::NONE)
        || strategy_is(
            options::NODE_PROMOTION_STRATEGY,
            "MODEL_ORDER_LEFT_TO_RIGHT",
        )
        || strategy_is(
            options::NODE_PROMOTION_STRATEGY,
            "MODEL_ORDER_RIGHT_TO_LEFT",
        )
        || strategy_is(options::LAYERING_STRATEGY, "BF_MODEL_ORDER")
        || strategy_is(options::LAYERING_STRATEGY, "DF_MODEL_ORDER")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `elk.layered.edgeRouting.selfLoopDistribution` is a node option. Set on the root only, it is
    /// the root's value, and the child keeps its own, as Java's `copyProperties` gives it.
    #[test]
    fn a_root_only_self_loop_distribution_does_not_reach_the_children() {
        let mut root = ElkNode {
            id: "root".to_string(),
            ..ElkNode::default()
        };
        root.properties.set(
            options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION,
            Some(OptionValue::Enum {
                class: "SelfLoopDistributionStrategy",
                name: "EQUALLY".to_string(),
            }),
        );
        root.children.push(ElkNode {
            id: "entity".to_string(),
            ..ElkNode::default()
        });

        let graph = import_root(&root).unwrap();

        assert!(
            graph
                .properties
                .has(options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION)
        );
        assert!(
            !graph.nodes[0]
                .properties
                .has(options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION)
        );
    }
}
