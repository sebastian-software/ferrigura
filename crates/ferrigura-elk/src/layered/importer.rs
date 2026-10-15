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

/// `importGraph` for a root whose children are plain nodes.
pub fn import_root(root: &ElkNode) -> Result<LGraph, LayoutError> {
    if !root.edges.is_empty() {
        return Err(LayoutError::Unsupported("edges".to_string()));
    }
    let mut graph = LGraph {
        properties: root.properties.clone(),
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
            properties: child.properties.clone(),
            size: KVector {
                x: child.shape.width,
                y: child.shape.height,
            },
            layer: None,
        });
    }
    graph.layerless_nodes = (0..graph.nodes.len()).collect();
    Ok(graph)
}
