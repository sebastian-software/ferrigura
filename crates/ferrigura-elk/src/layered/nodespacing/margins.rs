//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/NodeMarginCalculator.java
//!
//! `NodeMarginCalculator.processNode` as `InnermostNodeMarginCalculator` runs it, with
//! `excludeEdgeHeadTailLabels()`. The bounding box is the node's box joined with the boxes of its
//! labels and of its ports. Port labels are not ported, and edge labels are excluded by the
//! configuration and never read.

use crate::core::math::Spacing;
use crate::layered::lgraph::LGraph;

use super::{Rect, java_max};

/// The margins (top, left, bottom, right) of `node`: the distance from the node's box to the
/// bounding box of the node and its ports. Port positions are relative to the node.
pub(super) fn margin_of(graph: &LGraph, node: usize) -> Spacing {
    let lnode = &graph.nodes[node];
    // The bounding box starts as the node's own box.
    let mut bounding_box = Rect::new(
        lnode.position.x,
        lnode.position.y,
        lnode.size.x,
        lnode.size.y,
    );
    // The node's labels, relative to the node.
    for &label in &lnode.labels {
        let llabel = &graph.labels[label];
        bounding_box.union(&Rect::new(
            llabel.position.x + lnode.position.x,
            llabel.position.y + lnode.position.y,
            llabel.size.x,
            llabel.size.y,
        ));
    }
    for &port in &lnode.ports {
        let lport = &graph.ports[port];
        // The port's upper left corner, in the frame of the graph.
        let port_x = lport.position.x + lnode.position.x;
        let port_y = lport.position.y + lnode.position.y;
        bounding_box.union(&Rect::new(port_x, port_y, lport.size.x, lport.size.y));
    }
    // `new ElkMargin(node.getMargin())` and then all four sides are assigned, so every side is
    // replaced. The guard against tiny negative values is `Math.max(0, ...)`.
    Spacing {
        top: java_max(0.0, lnode.position.y - bounding_box.y),
        left: java_max(0.0, lnode.position.x - bounding_box.x),
        bottom: java_max(
            0.0,
            bounding_box.y + bounding_box.height - (lnode.position.y + lnode.size.y),
        ),
        right: java_max(
            0.0,
            bounding_box.x + bounding_box.width - (lnode.position.x + lnode.size.x),
        ),
    }
}
