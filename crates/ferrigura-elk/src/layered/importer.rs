//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphImporter.java
//!
//! The layered graph from the ELK graph: `ElkGraphImporter` and `ElkGraphTransformer.importGraph`.
//!
//! Graphs of plain nodes and edges between them. Ports and labels on nodes, nested
//! graphs and self loops are reported as unsupported.

use std::collections::HashMap;

use crate::core::data::OptionValue;
use crate::core::data::enums::{
    ComponentOrderingStrategy, EdgeLabelPlacement, GraphProperties, OrderingStrategy,
    PortConstraints,
};
use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::{ElkEdge, ElkNode};
use crate::layered::graph_util::create_port;
use crate::layered::lgraph::{LEdge, LGraph, LLabel, LNode, LabelId, PortType};
use crate::layered::options;
use crate::layered::spacings::{SPACING_BASE_VALUE, apply_base_value};

/// `importGraph` for a root whose children are plain nodes, joined by edges.
pub fn import_root(root: &ElkNode) -> Result<LGraph, LayoutError> {
    let mut graph = LGraph {
        properties: root.properties.clone(),
        root_id: root.id.clone(),
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
    let needs_model_order = needs_model_order_based_on_parent(root);
    let mut model_order = 0;
    for child in &root.children {
        if !child.children.is_empty() {
            return Err(LayoutError::Unsupported(format!(
                "nested graph in {}",
                child.id
            )));
        }
        if !child.ports.is_empty() || !child.labels.is_empty() {
            return Err(LayoutError::Unsupported(format!(
                "ports or labels on {}",
                child.id
            )));
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
        graph.nodes.push(LNode {
            id: child.id.clone(),
            designation: format!("{}.{}", root.id, child.id),
            properties,
            size: KVector {
                x: child.shape.width,
                y: child.shape.height,
            },
            model_order: node_model_order,
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
        if source_node == target_node {
            return Err(LayoutError::Unsupported(format!("self loop {}", edge.id)));
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
