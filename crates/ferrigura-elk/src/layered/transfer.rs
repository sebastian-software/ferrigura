//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphLayoutTransferrer.java
//!
//! `ElkGraphLayoutTransferrer.applyLayout` for the root graph: the node positions and the
//! root's size go back into the ELK graph. Edges, ports and labels are not transferred yet.

use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// `applyLayout(lgraph)` for a root without nested graphs.
pub fn apply_layout(graph: &LGraph, root: &mut ElkNode) -> Result<(), LayoutError> {
    if root
        .properties
        .enum_set_contains(options::NODE_SIZE_OPTIONS, "COMPUTE_PADDING")
    {
        return Err(LayoutError::Unsupported(
            "elk.nodeSize.options COMPUTE_PADDING".to_string(),
        ));
    }
    // The nodes sit inside the padding, so the offset includes its top and left.
    let offset = KVector {
        x: graph.offset.x + graph.padding.left,
        y: graph.offset.y + graph.padding.top,
    };
    // The nodes of the graph are the root's children, in the same order.
    for (child, node) in root.children.iter_mut().zip(&graph.nodes) {
        child.shape.x = node.position.x + offset.x;
        child.shape.y = node.position.y + offset.y;
    }
    // `applyParentNodeLayout`: the root takes the actual size, unless its size is fixed.
    if !root.properties.bool_of(options::NODE_SIZE_FIXED_GRAPH_SIZE) {
        let actual = graph.actual_size();
        root.shape.width = actual.x;
        root.shape.height = actual.y;
    }
    Ok(())
}
