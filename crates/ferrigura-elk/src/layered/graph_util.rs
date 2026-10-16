//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/LGraphUtil.java
//!
//! The helpers of `LGraphUtil` that the layered processors share.

use crate::core::data::enums::{Alignment, Direction, GraphProperties, PortSide};
use crate::layered::lgraph::{LGraph, LPort, PortType};
use crate::layered::options;

/// `LGraphUtil.createPort(node, null, type, graph)` for a port of its own: it sits on the side
/// that edges of its type leave or enter by, in the layout direction. Returns its index in
/// `LGraph::ports`; merged ports are not ported yet.
pub fn create_port(graph: &mut LGraph, node: usize, port_type: PortType) -> usize {
    let direction = graph
        .properties
        .enum_with_default::<Direction>(options::DIRECTION);
    let default_side = side_from_direction(direction);
    let side = match port_type {
        PortType::OUTPUT => default_side,
        PortType::INPUT => opposed(default_side),
    };
    let across_flow = match direction {
        Direction::LEFT | Direction::RIGHT => matches!(side, PortSide::NORTH | PortSide::SOUTH),
        Direction::UP | Direction::DOWN => matches!(side, PortSide::EAST | PortSide::WEST),
        _ => false,
    };
    if across_flow {
        graph
            .graph_properties
            .insert(GraphProperties::NORTH_SOUTH_PORTS);
    }
    graph.ports.push(LPort::new(node, side));
    let index = graph.ports.len() - 1;
    graph.nodes[node].ports.push(index);
    index
}

/// `PortSide.fromDirection`: the side that a flow in `direction` leaves a node by.
pub fn side_from_direction(direction: Direction) -> PortSide {
    match direction {
        Direction::UP => PortSide::NORTH,
        Direction::DOWN => PortSide::SOUTH,
        Direction::LEFT => PortSide::WEST,
        Direction::RIGHT => PortSide::EAST,
        _ => PortSide::UNDEFINED,
    }
}

/// `PortSide.opposed()`.
pub fn opposed(side: PortSide) -> PortSide {
    match side {
        PortSide::NORTH => PortSide::SOUTH,
        PortSide::SOUTH => PortSide::NORTH,
        PortSide::EAST => PortSide::WEST,
        PortSide::WEST => PortSide::EAST,
        PortSide::UNDEFINED => PortSide::UNDEFINED,
    }
}

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
        let ratio = alignment_ratio(graph, index);
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
/// follows the ports: the share of the edge ports that leave the node, or the centre when the
/// node has no edge ports.
fn alignment_ratio(graph: &LGraph, index: usize) -> f64 {
    match graph.nodes[index]
        .properties
        .enum_of::<Alignment>(options::ALIGNMENT)
    {
        Some(Alignment::LEFT) => 0.0,
        Some(Alignment::RIGHT) => 1.0,
        Some(Alignment::CENTER) => 0.5,
        _ => {
            let ports = &graph.nodes[index].ports;
            let inports = ports
                .iter()
                .filter(|&&port| !graph.ports[port].incoming.is_empty())
                .count();
            let outports = ports
                .iter()
                .filter(|&&port| !graph.ports[port].outgoing.is_empty())
                .count();
            if inports + outports == 0 {
                0.5
            } else {
                outports as f64 / (inports + outports) as f64
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::core::data::property::PropertyHolder;
    use crate::core::math::KVector;
    use crate::layered::lgraph::LNode;

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
