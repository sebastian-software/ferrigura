//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/OrthogonalEdgeRouter.java
//!
//! Phase 5, edge routing: `OrthogonalEdgeRouter.process`. It places the nodes of each layer,
//! routes the edges between each pair of adjacent layers with the orthogonal routing generator
//! ([`orthogonal`]), and sets the width of the graph. The layers, the node positions and the port
//! positions are the input.
//!
//! Edges must leave the east side of a layer and enter the west side of the next one. Graphs
//! with other features, which need processors that are not ported, are rejected by
//! [`check_supported`] instead of being routed wrongly.

mod orthogonal;

use crate::core::data::enums::{GraphProperties, NodeType, PortSide};
use crate::java::random::Random;
use crate::layered::LayoutError;
use crate::layered::graph_util::place_nodes_horizontally;
use crate::layered::lgraph::LGraph;
use crate::layered::spacings::{
    SPACING_EDGE_EDGE_BETWEEN_LAYERS, SPACING_EDGE_NODE_BETWEEN_LAYERS,
    SPACING_NODE_NODE_BETWEEN_LAYERS,
};
use orthogonal::OrthogonalRoutingGenerator;

/// `OrthogonalEdgeRouter.process`: routes the edges, and sets the x positions of the nodes, the
/// bend points of the edges and the width of the graph.
///
/// The Java declares `xpos` as a float. The port keeps `f64`, as elkjs does: GWT emulates `float`
/// as `double`.
pub fn route(graph: &mut LGraph) -> Result<(), LayoutError> {
    // The routing reads the graph's random source; it is taken out while the graph is borrowed.
    let mut random = graph.random.take();
    let result = route_layers(graph, &mut random);
    graph.random = random;
    result
}

/// The loop of `OrthogonalEdgeRouter.process`.
fn route_layers(graph: &mut LGraph, random: &mut Option<Random>) -> Result<(), LayoutError> {
    check_supported(graph)?;

    let node_node_spacing = required_spacing(graph, SPACING_NODE_NODE_BETWEEN_LAYERS)?;
    // The edge spacings are only read when there are edges, since nothing else uses them.
    let (edge_edge_spacing, edge_node_spacing) = if graph.edges.is_empty() {
        (0.0, 0.0)
    } else {
        (
            required_spacing(graph, SPACING_EDGE_EDGE_BETWEEN_LAYERS)?,
            required_spacing(graph, SPACING_EDGE_NODE_BETWEEN_LAYERS)?,
        )
    };

    let mut generator = OrthogonalRoutingGenerator::new(edge_edge_spacing);
    let layer_count = graph.layers.len();
    let mut xpos = 0.0;
    let mut left: Option<usize> = None;

    // Like the Java do-while, one step past the last layer, so that the last layer is placed.
    for index in 0..=layer_count {
        let right = (index < layer_count).then_some(index);

        // Place the left layer's nodes, if any.
        if let Some(layer) = left {
            place_nodes_horizontally(graph, layer, xpos);
            xpos += graph.layer_sizes[layer].x;
        }

        // Route the edges between the two layers.
        let start_pos = if left.is_none() {
            xpos
        } else {
            xpos + edge_node_spacing
        };
        let left_nodes = left.map(|layer| graph.layers[layer].clone());
        let right_nodes = right.map(|layer| graph.layers[layer].clone());
        let slots = generator.route_edges(
            graph,
            left_nodes.as_deref(),
            right_nodes.as_deref(),
            start_pos,
            random,
        )?;

        let left_external = is_external_layer(graph, left);
        let right_external = is_external_layer(graph, right);

        if slots > 0 {
            // The routing area's width: the slots, and the spacing to each layer that is there.
            let mut routing_width = f64::from(slots - 1) * edge_edge_spacing;
            if left.is_some() {
                routing_width += edge_node_spacing;
            }
            if right.is_some() {
                routing_width += edge_node_spacing;
            }

            // Between two layers, their minimal spacing is kept.
            if routing_width < node_node_spacing && !left_external && !right_external {
                routing_width = node_node_spacing;
            }
            xpos += routing_width;
        } else if !left_external && !right_external {
            // If all edges are straight, the usual spacing is used.
            xpos += node_node_spacing;
        }

        left = right;
    }

    graph.size.x = xpos;
    Ok(())
}

/// The spacing option of the router. Java reads it unconditionally; a graph without it cannot
/// be laid out the way Java would lay it out.
fn required_spacing(graph: &LGraph, id: &'static str) -> Result<f64, LayoutError> {
    graph
        .properties
        .double_of(id)
        .ok_or_else(|| LayoutError::Unsupported(format!("a graph without {id}")))
}

/// `isLeftLayerExternal` and `isRightLayerExternal`: a missing layer is external, and so is an
/// empty one, since `Iterables.all` over it holds. The Java predicate for external port nodes
/// is never true here: such nodes are rejected by [`check_supported`], because the predicate
/// needs `EXT_PORT_SIDE`, which is not ported.
fn is_external_layer(graph: &LGraph, layer: Option<usize>) -> bool {
    layer.is_none_or(|layer| graph.layers[layer].is_empty())
}

/// Rejects the features that the router needs processors for, which are not ported, and the
/// edges that the routing generator does not handle. Java would route such edges wrongly, or not
/// at all, so they are reported instead.
fn check_supported(graph: &LGraph) -> Result<(), LayoutError> {
    let unsupported =
        |feature: &str| LayoutError::Unsupported(format!("edge routing with {feature}"));

    for (property, feature) in [
        (GraphProperties::HYPEREDGES, "hyperedges"),
        (GraphProperties::HYPERNODES, "hypernodes"),
        (GraphProperties::EXTERNAL_PORTS, "hierarchical ports"),
    ] {
        if graph.graph_properties.contains(&property) {
            return Err(unsupported(feature));
        }
    }
    if graph
        .nodes
        .iter()
        .any(|node| node.node_type == NodeType::EXTERNAL_PORT)
    {
        return Err(unsupported("external port nodes"));
    }

    // Only ports with edges matter to the routing; a port without edges is not routed.
    for port in &graph.ports {
        if port.incoming.is_empty() && port.outgoing.is_empty() {
            continue;
        }
        match port.side {
            PortSide::EAST | PortSide::WEST => {}
            PortSide::NORTH | PortSide::SOUTH => {
                return Err(unsupported("north and south ports"));
            }
            PortSide::UNDEFINED => return Err(unsupported("ports without a side")),
        }
    }

    let mut layer_of: Vec<Option<usize>> = vec![None; graph.nodes.len()];
    for (index, layer) in graph.layers.iter().enumerate() {
        for &node in layer {
            layer_of[node] = Some(index);
        }
    }
    for edge in &graph.edges {
        let source = &graph.ports[edge.source];
        let target = &graph.ports[edge.target];
        if source.node == target.node {
            return Err(unsupported("self loops"));
        }
        let (Some(source_layer), Some(target_layer)) =
            (layer_of[source.node], layer_of[target.node])
        else {
            return Err(unsupported("nodes outside the layers"));
        };
        if source.side != PortSide::EAST
            || target.side != PortSide::WEST
            || target_layer != source_layer + 1
        {
            return Err(unsupported(
                "edges that do not join the east side of a layer to the west side of the next",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::core::math::KVector;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    /// A graph with the spacings the edge tests use: 20 between the layers for nodes, 5 between
    /// edges, and 10 between an edge and a node. Its random source is seeded.
    fn spaced_graph() -> LGraph {
        let mut graph = LGraph::default();
        graph.properties.set(
            SPACING_NODE_NODE_BETWEEN_LAYERS,
            Some(OptionValue::Double(20.0)),
        );
        graph.properties.set(
            SPACING_EDGE_EDGE_BETWEEN_LAYERS,
            Some(OptionValue::Double(5.0)),
        );
        graph.properties.set(
            SPACING_EDGE_NODE_BETWEEN_LAYERS,
            Some(OptionValue::Double(10.0)),
        );
        graph.random = Some(Random::new(1));
        graph
    }

    /// A node 10 by 10 at height `y`. Its x is set by the routing.
    fn node(graph: &mut LGraph, y: f64) -> usize {
        graph.nodes.push(LNode {
            size: KVector { x: 10.0, y: 10.0 },
            position: KVector { x: 0.0, y },
            ..LNode::default()
        });
        graph.nodes.len() - 1
    }

    /// A port on `side` of `node`, at `(x, y)` relative to the node, with a zero anchor.
    fn port(graph: &mut LGraph, node: usize, side: PortSide, x: f64, y: f64) -> usize {
        graph.ports.push(LPort {
            position: KVector { x, y },
            ..LPort::new(node, side)
        });
        let port = graph.ports.len() - 1;
        graph.nodes[node].ports.push(port);
        port
    }

    /// An edge from `source` to `target`, added to both ports' lists.
    fn edge(graph: &mut LGraph, source: usize, target: usize) -> usize {
        graph.edges.push(LEdge {
            source,
            target,
            ..LEdge::default()
        });
        let edge = graph.edges.len() - 1;
        graph.ports[source].outgoing.push(edge);
        graph.ports[target].incoming.push(edge);
        edge
    }

    /// A layer of the given nodes, 10 wide.
    fn layer(graph: &mut LGraph, nodes: &[usize]) {
        graph.layers.push(nodes.to_vec());
        graph.layer_sizes.push(KVector { x: 10.0, y: 10.0 });
    }

    fn bends(graph: &LGraph, edge: usize) -> Vec<(f64, f64)> {
        graph.edges[edge]
            .bend_points
            .iter()
            .map(|point| (point.x, point.y))
            .collect()
    }

    #[test]
    fn layers_are_spaced_by_node_node_between_layers() {
        let mut graph = LGraph::default();
        graph.properties.set(
            SPACING_NODE_NODE_BETWEEN_LAYERS,
            Some(OptionValue::Double(40.0)),
        );
        for (index, width) in [50.0, 30.0].into_iter().enumerate() {
            graph.nodes.push(LNode {
                size: KVector { x: width, y: 10.0 },
                ..LNode::default()
            });
            graph.layers.push(vec![index]);
            graph.layer_sizes.push(KVector { x: width, y: 10.0 });
        }
        route(&mut graph).unwrap();
        assert_eq!(graph.size.x, 50.0 + 40.0 + 30.0);
        assert_eq!(graph.nodes[0].position.x, 0.0);
        assert_eq!(graph.nodes[1].position.x, 90.0);
    }

    #[test]
    fn one_edge_takes_the_first_slot_between_its_layers() {
        // A's east port is at y 5 and B's west port at y 20. The edge runs at the first slot,
        // at the left layer's width plus the edge-node spacing: 10 + 10.
        let mut graph = spaced_graph();
        let a = node(&mut graph, 0.0);
        let b = node(&mut graph, 0.0);
        let a_out = port(&mut graph, a, PortSide::EAST, 10.0, 5.0);
        let b_in = port(&mut graph, b, PortSide::WEST, 0.0, 20.0);
        let e = edge(&mut graph, a_out, b_in);
        layer(&mut graph, &[a]);
        layer(&mut graph, &[b]);

        route(&mut graph).unwrap();

        assert_eq!(bends(&graph, e), vec![(20.0, 5.0), (20.0, 20.0)]);
        assert!(graph.junction_points.is_empty());
        assert_eq!(graph.nodes[a].position.x, 0.0);
        // The routing area is 2 * 10 wide, which is the node-node spacing of 20.
        assert_eq!(graph.nodes[b].position.x, 30.0);
        assert_eq!(graph.size.x, 40.0);
    }

    #[test]
    fn fan_out_from_one_port_is_one_segment_with_a_junction() {
        // A's east port (y 10) feeds B1 at y 0 and B2 at y 30. The segment runs from 0 to 30.
        // The first edge's bend at (20, 10) lies inside the segment, so it gets a junction there.
        // The second edge bends through the same point, which is already created, so it gets none.
        let mut graph = spaced_graph();
        let a = node(&mut graph, 0.0);
        let b1 = node(&mut graph, 0.0);
        let b2 = node(&mut graph, 30.0);
        let a_out = port(&mut graph, a, PortSide::EAST, 10.0, 10.0);
        let b1_in = port(&mut graph, b1, PortSide::WEST, 0.0, 0.0);
        let b2_in = port(&mut graph, b2, PortSide::WEST, 0.0, 0.0);
        let e1 = edge(&mut graph, a_out, b1_in);
        let e2 = edge(&mut graph, a_out, b2_in);
        layer(&mut graph, &[a]);
        layer(&mut graph, &[b1, b2]);

        route(&mut graph).unwrap();

        assert_eq!(bends(&graph, e1), vec![(20.0, 10.0), (20.0, 0.0)]);
        assert_eq!(bends(&graph, e2), vec![(20.0, 10.0), (20.0, 30.0)]);
        assert_eq!(graph.junction_points.len(), 1);
        assert_eq!(
            graph.junction_points.get(&e1),
            Some(&vec![KVector { x: 20.0, y: 10.0 }])
        );
        assert_eq!(graph.size.x, 40.0);
    }

    #[test]
    fn non_overlapping_edges_share_the_first_slot() {
        // A1 (y 0) to B1 (y 10), and A2 (y 30) to B2 (y 40): no conflicts, so both use slot 0.
        let mut graph = spaced_graph();
        let a1 = node(&mut graph, 0.0);
        let a2 = node(&mut graph, 30.0);
        let b1 = node(&mut graph, 10.0);
        let b2 = node(&mut graph, 40.0);
        let a1_out = port(&mut graph, a1, PortSide::EAST, 10.0, 0.0);
        let a2_out = port(&mut graph, a2, PortSide::EAST, 10.0, 0.0);
        let b1_in = port(&mut graph, b1, PortSide::WEST, 0.0, 0.0);
        let b2_in = port(&mut graph, b2, PortSide::WEST, 0.0, 0.0);
        let e1 = edge(&mut graph, a1_out, b1_in);
        let e2 = edge(&mut graph, a2_out, b2_in);
        layer(&mut graph, &[a1, a2]);
        layer(&mut graph, &[b1, b2]);

        route(&mut graph).unwrap();

        assert_eq!(bends(&graph, e1), vec![(20.0, 0.0), (20.0, 10.0)]);
        assert_eq!(bends(&graph, e2), vec![(20.0, 30.0), (20.0, 40.0)]);
        assert!(graph.junction_points.is_empty());
        assert_eq!(graph.size.x, 40.0);
    }

    #[test]
    fn a_critical_conflict_puts_the_second_edge_in_the_next_slot() {
        // A1 (y 0) to B1 (y 20), and A2 (y 10) to B2 (y 0). B2's end lies on A1's start: a
        // critical conflict, so A1's segment must be left of A2's. A2 takes slot 1, 5 further on.
        let mut graph = spaced_graph();
        let a1 = node(&mut graph, 0.0);
        let a2 = node(&mut graph, 10.0);
        let b1 = node(&mut graph, 20.0);
        let b2 = node(&mut graph, 0.0);
        let a1_out = port(&mut graph, a1, PortSide::EAST, 10.0, 0.0);
        let a2_out = port(&mut graph, a2, PortSide::EAST, 10.0, 0.0);
        let b1_in = port(&mut graph, b1, PortSide::WEST, 0.0, 0.0);
        let b2_in = port(&mut graph, b2, PortSide::WEST, 0.0, 0.0);
        let e1 = edge(&mut graph, a1_out, b1_in);
        let e2 = edge(&mut graph, a2_out, b2_in);
        layer(&mut graph, &[a1, a2]);
        layer(&mut graph, &[b1, b2]);

        route(&mut graph).unwrap();

        assert_eq!(bends(&graph, e1), vec![(20.0, 0.0), (20.0, 20.0)]);
        assert_eq!(bends(&graph, e2), vec![(25.0, 10.0), (25.0, 0.0)]);
        assert!(graph.junction_points.is_empty());
        // Two slots: 5 between them and 10 to each layer make 25, more than the node spacing of 20.
        assert_eq!(graph.nodes[b1].position.x, 35.0);
        assert_eq!(graph.size.x, 45.0);
    }

    #[test]
    fn a_critical_cycle_splits_a_segment() {
        // A1 (y 0) to B1 (y 10), and A2 (y 10) to B2 (y 0): each segment's ends lie on the
        // other's, so both dependencies are critical and form a cycle. The splitter cuts seg2 at
        // y 5, which gives it an upper half (seg2 itself, slot 0) and a lower half (the split
        // partner, slot 2). seg1 sits between them at slot 1, since the critical dependencies
        // are seg2 -> seg1 -> partner.
        let mut graph = spaced_graph();
        let a1 = node(&mut graph, 0.0);
        let a2 = node(&mut graph, 10.0);
        let b1 = node(&mut graph, 10.0);
        let b2 = node(&mut graph, 0.0);
        let a1_out = port(&mut graph, a1, PortSide::EAST, 10.0, 0.0);
        let a2_out = port(&mut graph, a2, PortSide::EAST, 10.0, 0.0);
        let b1_in = port(&mut graph, b1, PortSide::WEST, 0.0, 0.0);
        let b2_in = port(&mut graph, b2, PortSide::WEST, 0.0, 0.0);
        let e1 = edge(&mut graph, a1_out, b1_in);
        let e2 = edge(&mut graph, a2_out, b2_in);
        layer(&mut graph, &[a1, a2]);
        layer(&mut graph, &[b1, b2]);

        route(&mut graph).unwrap();

        // seg1 at slot 1 (x 25), seg2 at slot 0 (x 20), and the partner at slot 2 (x 30), which
        // carries no bends of its own.
        assert_eq!(bends(&graph, e1), vec![(25.0, 0.0), (25.0, 10.0)]);
        assert_eq!(
            bends(&graph, e2),
            vec![(20.0, 10.0), (20.0, 5.0), (30.0, 5.0), (30.0, 0.0)]
        );
        assert!(graph.junction_points.is_empty());
        assert_eq!(graph.nodes[b1].position.x, 40.0);
        assert_eq!(graph.size.x, 50.0);
    }

    #[test]
    fn a_long_edge_is_routed_through_its_dummy() {
        // A (out y 5) to a dummy D (in y 15), then D (out y 15) to B (in y 25). Each part takes
        // slot 0 of its layer pair; the dummy's layer is placed between them.
        let mut graph = spaced_graph();
        let a = node(&mut graph, 0.0);
        let d = node(&mut graph, 0.0);
        let b = node(&mut graph, 0.0);
        let a_out = port(&mut graph, a, PortSide::EAST, 10.0, 5.0);
        let d_in = port(&mut graph, d, PortSide::WEST, 0.0, 15.0);
        let d_out = port(&mut graph, d, PortSide::EAST, 10.0, 15.0);
        let b_in = port(&mut graph, b, PortSide::WEST, 0.0, 25.0);
        let first = edge(&mut graph, a_out, d_in);
        let second = edge(&mut graph, d_out, b_in);
        layer(&mut graph, &[a]);
        layer(&mut graph, &[d]);
        layer(&mut graph, &[b]);

        route(&mut graph).unwrap();

        assert_eq!(bends(&graph, first), vec![(20.0, 5.0), (20.0, 15.0)]);
        assert_eq!(bends(&graph, second), vec![(50.0, 15.0), (50.0, 25.0)]);
        assert_eq!(graph.nodes[d].position.x, 30.0);
        assert_eq!(graph.nodes[b].position.x, 60.0);
        assert_eq!(graph.size.x, 70.0);
    }

    #[test]
    fn features_the_router_does_not_port_are_unsupported() {
        fn route_result(build: impl FnOnce(&mut LGraph)) -> Result<(), LayoutError> {
            let mut graph = spaced_graph();
            build(&mut graph);
            route(&mut graph)
        }

        // An edge leaving a north port.
        let north = route_result(|graph| {
            let a = node(graph, 0.0);
            let b = node(graph, 0.0);
            let a_out = port(graph, a, PortSide::NORTH, 5.0, 0.0);
            let b_in = port(graph, b, PortSide::WEST, 0.0, 5.0);
            edge(graph, a_out, b_in);
            layer(graph, &[a]);
            layer(graph, &[b]);
        });
        assert!(matches!(north, Err(LayoutError::Unsupported(_))));

        // An edge that skips a layer.
        let skipping = route_result(|graph| {
            let a = node(graph, 0.0);
            let middle = node(graph, 50.0);
            let b = node(graph, 0.0);
            let a_out = port(graph, a, PortSide::EAST, 10.0, 5.0);
            let b_in = port(graph, b, PortSide::WEST, 0.0, 5.0);
            edge(graph, a_out, b_in);
            layer(graph, &[a]);
            layer(graph, &[middle]);
            layer(graph, &[b]);
        });
        assert!(matches!(skipping, Err(LayoutError::Unsupported(_))));

        // A self loop.
        let self_loop = route_result(|graph| {
            let a = node(graph, 0.0);
            let a_out = port(graph, a, PortSide::EAST, 10.0, 5.0);
            let a_in = port(graph, a, PortSide::WEST, 0.0, 5.0);
            edge(graph, a_out, a_in);
            layer(graph, &[a]);
        });
        assert!(matches!(self_loop, Err(LayoutError::Unsupported(_))));
    }
}
