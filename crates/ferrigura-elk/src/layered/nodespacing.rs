//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/InnermostNodeMarginCalculator.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/LabelAndNodeSizeProcessor.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/ (reduced path)
//!
//! Node margins, port positions and node sizes of the layered graph.
//!
//! This is the reduced path of ELK's node spacing code, which the layered processors call for
//! nodes with ports on the four sides and no labels:
//!
//! - [`node_margins`] is `InnermostNodeMarginCalculator`: each node's margins wrap its box and
//!   the boxes of its ports. Edge head and tail labels are excluded, as the Java excludes them.
//! - [`label_and_node_size`] is `LabelAndNodeSizeProcessor` for the normal nodes of the layers.
//!   It places the ports (`LPort::position`) and sets the node size. Without size constraints
//!   the size stays the node's own size; the cell system lays the ports out inside it.
//!
//! Not ported, and reported as `Unsupported` where the input asks for it: node and port labels
//! (a layered node has no label list, and a layered port has none either), node size
//! constraints, port constraint `FIXED_RATIO`, a port label placement without `INSIDE` or
//! `OUTSIDE` (fixed label positions), `COMPUTE_PADDING`, individual spacings on a node, and
//! external port dummies. `FIXED_POS` and `FIXED_ORDER` are ported: `FIXED_ORDER` takes the free
//! branches, and `FIXED_POS` keeps the port coordinates it was given. Port border offsets are not
//! read: `LPort` carries no property holder, and the importer gives ports no options, so no port
//! has one yet.
//!
//! Both entry points compute every node before they change the graph, so an error leaves the
//! graph as it was.

mod cells;
mod context;
mod margins;
mod sizing;
#[cfg(test)]
mod tests;

use crate::core::data::enums::{GraphProperties, NodeType};
use crate::core::math::Spacing;
use crate::layered::LayoutError;
use crate::layered::lgraph::{LGraph, LNode};

/// `org.eclipse.elk.core.math.ElkRectangle`, the parts the node spacing code uses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// `ElkRectangle.union`: the smallest rectangle that holds both rectangles.
    fn union(&mut self, other: &Rect) {
        let mut x1 = java_min(self.x, other.x);
        let mut y1 = java_min(self.y, other.y);
        let mut x2 = java_max(self.x + self.width, other.x + other.width);
        let mut y2 = java_max(self.y + self.height, other.y + other.height);
        if x2 < x1 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y2 < y1 {
            std::mem::swap(&mut y1, &mut y2);
        }
        *self = Rect::new(x1, y1, x2 - x1, y2 - y1);
    }
}

/// `Math.max(double, double)`: NaN wins, and 0.0 is greater than -0.0.
fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && a.is_sign_negative() {
        return b;
    }
    if a >= b { a } else { b }
}

/// `Math.min(double, double)`: NaN wins, and -0.0 is less than 0.0.
fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && b.is_sign_negative() {
        return b;
    }
    if a <= b { a } else { b }
}

/// The nodes of the layers, left to right, that `keep` accepts. `LGraphAdapter.getNodes()`
/// walks the layers in the same way and skips the layerless nodes.
fn layered_nodes(graph: &LGraph, keep: impl Fn(&LNode) -> bool) -> Vec<usize> {
    graph
        .layers
        .iter()
        .flatten()
        .copied()
        .filter(|&node| keep(&graph.nodes[node]))
        .collect()
}

/// `InnermostNodeMarginCalculator.process` for the graph: sets the margins of every node in a
/// layer, dummies included. The margins wrap the node and its ports.
pub fn node_margins(graph: &mut LGraph) -> Result<(), LayoutError> {
    let margins: Vec<(usize, Spacing)> = {
        let view: &LGraph = graph;
        layered_nodes(view, |_| true)
            .into_iter()
            .map(|node| (node, margins::margin_of(view, node)))
            .collect()
    };
    for (node, margin) in margins {
        graph.nodes[node].margin = margin;
    }
    Ok(())
}

/// `LabelAndNodeSizeProcessor.process` for the graph: places the ports of the normal nodes in the
/// layers and sets their sizes. The processor's external port part is not ported.
pub fn label_and_node_size(graph: &mut LGraph) -> Result<(), LayoutError> {
    if graph
        .graph_properties
        .contains(&GraphProperties::EXTERNAL_PORTS)
    {
        return Err(LayoutError::Unsupported(
            "external port dummies in the node size processor".to_string(),
        ));
    }
    let results = {
        let view: &LGraph = graph;
        layered_nodes(view, |node| node.node_type == NodeType::NORMAL)
            .into_iter()
            .map(|node| sizing::calculate(view, node))
            .collect::<Result<Vec<_>, LayoutError>>()?
    };
    for result in results {
        graph.nodes[result.node].size = result.size;
        for (port, position) in result.ports {
            graph.ports[port].position = position;
        }
    }
    Ok(())
}

#[cfg(test)]
mod java_math {
    use super::{java_max, java_min};

    #[test]
    fn signed_zero_follows_the_jdk() {
        assert!(java_max(-0.0, 0.0).is_sign_positive());
        assert!(java_max(0.0, -0.0).is_sign_positive());
        assert!(java_min(0.0, -0.0).is_sign_negative());
        assert!(java_min(-0.0, 0.0).is_sign_negative());
        assert!(java_max(f64::NAN, 1.0).is_nan());
        assert!(java_max(1.0, f64::NAN).is_nan());
        assert_eq!(java_max(2.0, 3.0), 3.0);
        assert_eq!(java_min(2.0, 3.0), 2.0);
    }
}
