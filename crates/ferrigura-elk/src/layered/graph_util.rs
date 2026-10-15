//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/LGraphUtil.java
//!
//! The helpers of `LGraphUtil` that the layered processors share.

use crate::core::data::enums::Alignment;
use crate::layered::lgraph::{LGraph, LNode};
use crate::layered::options;

/// `LGraphUtil.placeNodesHorizontally(layer, xoffset)`: places the nodes of one layer across
/// the layer's width, by their alignment, and sets their x positions.
pub fn place_nodes_horizontally(graph: &mut LGraph, layer: usize, xoffset: f64) {
    let members = graph.layers[layer].clone();
    let layer_width = graph.layer_sizes[layer].x;
    let max_left_margin = members.iter().fold(0.0, |max, &node| {
        f64::max(max, graph.nodes[node].margin.left)
    });
    let max_right_margin = members.iter().fold(0.0, |max, &node| {
        f64::max(max, graph.nodes[node].margin.right)
    });
    for &index in &members {
        let node = &graph.nodes[index];
        let ratio = alignment_ratio(node);
        let node_width = node.size.x;
        let mut xpos = (layer_width - node_width) * ratio;
        if ratio > 0.5 {
            xpos -= max_right_margin * 2.0 * (ratio - 0.5);
        } else if ratio < 0.5 {
            xpos += max_left_margin * 2.0 * (0.5 - ratio);
        }
        // The node's own margins keep it inside the layer.
        if xpos < node.margin.left {
            xpos = node.margin.left;
        }
        let max_xpos = layer_width - node.margin.right - node_width;
        if xpos > max_xpos {
            xpos = max_xpos;
        }
        graph.nodes[index].position.x = xoffset + xpos;
    }
}

/// The ratio that `placeNodesHorizontally` takes from a node's alignment. The automatic case
/// is port-based in the Java source; these nodes have no ports, so it gives the centre.
fn alignment_ratio(node: &LNode) -> f64 {
    match node.properties.enum_of::<Alignment>(options::ALIGNMENT) {
        Some(Alignment::LEFT) => 0.0,
        Some(Alignment::RIGHT) => 1.0,
        _ => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::core::data::property::PropertyHolder;
    use crate::core::math::KVector;

    /// A layer with one node of the given width, in a layer of width 50.
    fn one_node_layer(alignment: Option<&str>) -> LGraph {
        let mut node = LNode {
            size: KVector { x: 30.0, y: 10.0 },
            ..LNode::default()
        };
        if let Some(name) = alignment {
            node.properties.set(
                options::ALIGNMENT,
                Some(OptionValue::Enum {
                    class: "Alignment",
                    name: name.to_string(),
                }),
            );
        }
        LGraph {
            nodes: vec![node],
            layers: vec![vec![0]],
            layer_sizes: vec![KVector { x: 50.0, y: 10.0 }],
            ..LGraph::default()
        }
    }

    #[test]
    fn default_alignment_is_automatic() {
        let holder = PropertyHolder::new();
        assert_eq!(
            holder.enum_of::<Alignment>(options::ALIGNMENT),
            Some(Alignment::AUTOMATIC)
        );
    }

    #[test]
    fn nodes_are_centred_by_default() {
        let mut graph = one_node_layer(None);
        place_nodes_horizontally(&mut graph, 0, 100.0);
        assert_eq!(graph.nodes[0].position.x, 110.0);
    }

    #[test]
    fn left_and_right_alignment_touch_the_edges() {
        let mut left = one_node_layer(Some("LEFT"));
        place_nodes_horizontally(&mut left, 0, 100.0);
        assert_eq!(left.nodes[0].position.x, 100.0);

        let mut right = one_node_layer(Some("RIGHT"));
        place_nodes_horizontally(&mut right, 0, 100.0);
        assert_eq!(right.nodes[0].position.x, 120.0);
    }
}
