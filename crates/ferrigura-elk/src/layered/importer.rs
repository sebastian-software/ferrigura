//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphImporter.java
//!
//! The layered graph from the ELK graph: `ElkGraphImporter` and `ElkGraphTransformer.importGraph`.
//!
//! Only graphs with nodes and no edges, ports or labels are imported so far.
//! Anything else is reported as unsupported.

use crate::core::data::OptionValue;
use crate::core::data::enums::{ComponentOrderingStrategy, OrderingStrategy};
use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::lgraph::{LGraph, LNode};
use crate::layered::options;
use crate::layered::spacings::{SPACING_BASE_VALUE, apply_base_value};

/// `importGraph` for a root whose children are plain nodes.
pub fn import_root(root: &ElkNode) -> Result<LGraph, LayoutError> {
    if !root.edges.is_empty() {
        return Err(LayoutError::Unsupported("edges".to_string()));
    }
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
        graph.nodes.push(LNode {
            id: child.id.clone(),
            designation: format!("{}.{}", root.id, child.id),
            properties: child.properties.clone(),
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
    Ok(graph)
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
