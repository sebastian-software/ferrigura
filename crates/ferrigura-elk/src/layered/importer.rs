//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphImporter.java
//!
//! The layered graph from the ELK graph: `ElkGraphImporter` and `ElkGraphTransformer.importGraph`.
//!
//! Only graphs with nodes and no edges, ports or labels are imported so far.
//! Anything else is reported as unsupported.

use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::lgraph::{LGraph, LNode};
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
        graph.nodes.push(LNode {
            id: child.id.clone(),
            designation: format!("{}.{}", root.id, child.id),
            properties: child.properties.clone(),
            size: KVector {
                x: child.shape.width,
                y: child.shape.height,
            },
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
