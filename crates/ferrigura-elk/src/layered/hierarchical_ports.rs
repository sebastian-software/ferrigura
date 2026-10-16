//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/HierarchicalPortConstraintProcessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/HierarchicalPortDummySizeProcessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/HierarchicalPortPositionProcessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/HierarchicalPortOrthogonalEdgeRouter.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/InvertedPortProcessor.java
//!
//! The processors of a graph with external port dummies, and the inverted port processor of a graph
//! with non-free ports.
//!
//! The external port dummies of the compound graph preprocessor take their sides from the flow when
//! the parent node's port constraints are free, which leaves them in the first and the last layer
//! of the graph's internal frame. The parts these processors need for fixed port constraints are not
//! ported and are reported as `Unsupported`: the in-layer order of eastern and western dummies,
//! northern and southern dummies (which are replaced by dummies in every layer and routed around the
//! graph), and port ratios and positions. The inverted port processor reports any port it would
//! change, as the dummies it makes are not ported.

use crate::core::data::enums::{NodeType, PortConstraints, PortSide};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::ports::{is_order_fixed, is_side_fixed};
use crate::layered::spacings::SPACING_EDGE_EDGE_BETWEEN_LAYERS;

/// `LayeredOptions.PORT_CONSTRAINTS` of the graph, which the preprocessor sets for its dummies.
fn graph_port_constraints(graph: &LGraph) -> PortConstraints {
    graph
        .properties
        .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS)
}

/// The side of the external port an external port dummy stands for, `UNDEFINED` for other nodes.
pub fn ext_port_side(graph: &LGraph, node: usize) -> PortSide {
    graph.nodes[node]
        .external_port
        .as_ref()
        .and_then(|external| external.side)
        .unwrap_or(PortSide::UNDEFINED)
}

/// `HierarchicalPortConstraintProcessor.process`: with a fixed port order, the eastern and western
/// dummies of the first and the last layer would be chained by in-layer constraints; with fixed
/// sides, the northern and southern dummies would be replaced. Neither is ported, so such graphs
/// are reported; without those dummies the processor changes nothing.
pub fn process_port_constraints(graph: &mut LGraph) -> Result<(), LayoutError> {
    let constraints = graph_port_constraints(graph);
    let has = |sides: &[PortSide]| {
        graph.layers.iter().flatten().any(|&node| {
            graph.nodes[node].node_type == NodeType::EXTERNAL_PORT
                && sides.contains(&ext_port_side(graph, node))
        })
    };
    if is_order_fixed(constraints) && has(&[PortSide::EAST, PortSide::WEST]) {
        return Err(LayoutError::Unsupported(
            "a fixed order of eastern and western external ports \
             (HierarchicalPortConstraintProcessor)"
                .to_string(),
        ));
    }
    if is_side_fixed(constraints) && has(&[PortSide::NORTH, PortSide::SOUTH]) {
        return Err(LayoutError::Unsupported(
            "northern and southern external ports with fixed sides \
             (HierarchicalPortConstraintProcessor)"
                .to_string(),
        ));
    }
    Ok(())
}

/// `HierarchicalPortDummySizeProcessor.process`: the northern and southern external port dummies of
/// each layer get widths that grow by twice the edge spacing, so that their edges are spaced.
pub fn process_dummy_sizes(graph: &mut LGraph) {
    let delta = graph
        .properties
        .double_with_default(SPACING_EDGE_EDGE_BETWEEN_LAYERS)
        * 2.0;
    for layer in graph.layers.clone() {
        let mut northern = Vec::new();
        let mut southern = Vec::new();
        for &node in &layer {
            if graph.nodes[node].node_type == NodeType::EXTERNAL_PORT {
                match ext_port_side(graph, node) {
                    PortSide::NORTH => northern.push(node),
                    PortSide::SOUTH => southern.push(node),
                    _ => {}
                }
            }
        }
        set_widths(graph, &northern, true, delta);
        set_widths(graph, &southern, false, delta);
    }
}

/// `HierarchicalPortDummySizeProcessor.setWidths`.
fn set_widths(graph: &mut LGraph, nodes: &[usize], top_down: bool, delta: f64) {
    let mut current_width = 0.0;
    let mut step = delta;
    if !top_down {
        current_width = delta * (nodes.len() as f64 - 1.0);
        step *= -1.0;
    }
    for &node in nodes {
        graph.nodes[node].properties.set(
            options::ALIGNMENT,
            Some(crate::core::data::OptionValue::Enum {
                class: "Alignment",
                name: "CENTER".to_string(),
            }),
        );
        graph.nodes[node].size.x = current_width;
        for port in graph.nodes[node].ports.clone() {
            if graph.ports[port].side == PortSide::EAST {
                graph.ports[port].position.x = current_width;
            }
        }
        current_width += step;
    }
}

/// `HierarchicalPortPositionProcessor.process`: the eastern and western dummies keep the y
/// coordinates the node placement gave them unless the port ratios or positions are fixed, which
/// is not ported.
pub fn process_positions(graph: &mut LGraph) -> Result<(), LayoutError> {
    let constraints = graph_port_constraints(graph);
    if matches!(
        constraints,
        PortConstraints::FIXED_RATIO | PortConstraints::FIXED_POS
    ) {
        return Err(LayoutError::Unsupported(
            "fixed port ratios or positions on a node with a nested graph \
             (HierarchicalPortPositionProcessor)"
                .to_string(),
        ));
    }
    Ok(())
}

/// `HierarchicalPortOrthogonalEdgeRouter.process` for a graph without northern and southern
/// dummies: the eastern and western dummies move to the graph's border (step 5) and the edge
/// segments at them are straightened (step 6).
pub fn route_hierarchical_port_edges(graph: &mut LGraph) -> Result<(), LayoutError> {
    let north_south = graph
        .layers
        .iter()
        .flatten()
        .chain(graph.layerless_nodes.iter())
        .any(|&node| {
            graph.nodes[node].node_type == NodeType::EXTERNAL_PORT
                && matches!(
                    ext_port_side(graph, node),
                    PortSide::NORTH | PortSide::SOUTH
                )
        });
    if north_south {
        return Err(LayoutError::Unsupported(
            "northern and southern external port dummies (HierarchicalPortOrthogonalEdgeRouter)"
                .to_string(),
        ));
    }
    let constraints = graph_port_constraints(graph);
    if matches!(
        constraints,
        PortConstraints::FIXED_RATIO | PortConstraints::FIXED_POS
    ) {
        return Err(LayoutError::Unsupported(
            "fixed port ratios or positions on a node with a nested graph \
             (HierarchicalPortOrthogonalEdgeRouter)"
                .to_string(),
        ));
    }
    if graph.layers.is_empty() {
        return Ok(());
    }
    // Step 5, `fixCoordinates`: the first and the last layer.
    let last = graph.layers.len() - 1;
    fix_coordinates(graph, 0);
    fix_coordinates(graph, last);
    // Step 6, `correctSlantedEdgeSegments`.
    correct_slanted_edge_segments(graph, 0);
    correct_slanted_edge_segments(graph, last);
    Ok(())
}

/// `fixCoordinates(layer, constraints, graph)` for free port positions: eastern and western dummies
/// go to the graph's border. The graph keeps its height, as no position is fixed.
fn fix_coordinates(graph: &mut LGraph, layer: usize) {
    let padding = graph.padding;
    let offset = graph.offset;
    let width = graph.size.x;
    for node in graph.layers[layer].clone() {
        if graph.nodes[node].node_type != NodeType::EXTERNAL_PORT {
            continue;
        }
        match ext_port_side(graph, node) {
            PortSide::EAST => graph.nodes[node].position.x = width + padding.right - offset.x,
            PortSide::WEST => graph.nodes[node].position.x = -offset.x - padding.left,
            _ => {}
        }
    }
}

/// `correctSlantedEdgeSegments(layer)`: an edge with bend points that meets an eastern or western
/// dummy has its first (or last) bend point moved to the height of the dummy's port.
fn correct_slanted_edge_segments(graph: &mut LGraph, layer: usize) {
    for node in graph.layers[layer].clone() {
        if graph.nodes[node].node_type != NodeType::EXTERNAL_PORT
            || !matches!(ext_port_side(graph, node), PortSide::EAST | PortSide::WEST)
        {
            continue;
        }
        // `getConnectedEdges()`: the incoming then the outgoing edges of each port.
        let edges: Vec<usize> = graph.nodes[node]
            .ports
            .iter()
            .flat_map(|&port| {
                let data = &graph.ports[port];
                data.incoming.iter().chain(data.outgoing.iter()).copied()
            })
            .collect();
        for edge in edges {
            if graph.edges[edge].bend_points.is_empty() {
                continue;
            }
            let source = graph.edges[edge].source;
            if graph.ports[source].node == node {
                let y = absolute_anchor_y(graph, source);
                if let Some(first) = graph.edges[edge].bend_points.first_mut() {
                    first.y = y;
                }
            }
            let target = graph.edges[edge].target;
            if graph.ports[target].node == node {
                let y = absolute_anchor_y(graph, target);
                if let Some(last) = graph.edges[edge].bend_points.last_mut() {
                    last.y = y;
                }
            }
        }
    }
}

/// The y coordinate of `LPort.getAbsoluteAnchor()`.
fn absolute_anchor_y(graph: &LGraph, port: usize) -> f64 {
    let data = &graph.ports[port];
    graph.nodes[data.node].position.y + data.position.y + data.anchor.y
}

/// `InvertedPortProcessor.process`: input ports on the east side and output ports on the west side
/// of side-fixed normal nodes would get dummies that turn their edges around. Those dummies are not
/// ported; a graph that needs one is reported, and any other graph is left as it is.
pub fn process_inverted_ports(graph: &mut LGraph) -> Result<(), LayoutError> {
    for &node in graph.layers.iter().flatten() {
        let data = &graph.nodes[node];
        if data.node_type != NodeType::NORMAL
            || !is_side_fixed(
                data.properties
                    .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS),
            )
        {
            continue;
        }
        for &port in &data.ports {
            let port_data = &graph.ports[port];
            let inverted_input = port_data.side == PortSide::EAST
                && port_data
                    .incoming
                    .iter()
                    .any(|&edge| graph.ports[graph.edges[edge].source].node != node);
            let inverted_output = port_data.side == PortSide::WEST
                && port_data
                    .outgoing
                    .iter()
                    .any(|&edge| graph.ports[graph.edges[edge].target].node != node);
            if inverted_input || inverted_output {
                return Err(LayoutError::Unsupported(format!(
                    "an inverted port on {} (InvertedPortProcessor dummies)",
                    graph.node_name(node)
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::math::{KVector, Spacing};
    use crate::layered::lgraph::{ExternalPortDummy, LEdge, LNode, LPort};

    fn dummy(side: PortSide) -> LNode {
        LNode {
            node_type: NodeType::EXTERNAL_PORT,
            external_port: Some(ExternalPortDummy {
                side: Some(side),
                ..ExternalPortDummy::default()
            }),
            ..LNode::default()
        }
    }

    /// A western dummy in the first layer with an edge to a node in the second, whose route has a
    /// bend point off the dummy's height, and an eastern dummy in the last layer.
    #[test]
    fn eastern_and_western_dummies_go_to_the_border_and_straighten_their_edges() {
        let mut graph = LGraph {
            nodes: vec![
                dummy(PortSide::WEST),
                LNode::default(),
                dummy(PortSide::EAST),
            ],
            layers: vec![vec![0], vec![1], vec![2]],
            ..LGraph::default()
        };
        graph.size = KVector { x: 200.0, y: 80.0 };
        graph.offset = KVector { x: 2.0, y: 0.0 };
        graph.padding = Spacing {
            top: 0.0,
            left: 10.0,
            bottom: 0.0,
            right: 12.0,
        };
        graph.nodes[0].position.y = 30.0;
        graph.ports = vec![LPort::new(0, PortSide::EAST), LPort::new(1, PortSide::WEST)];
        graph.nodes[0].ports = vec![0];
        graph.nodes[1].ports = vec![1];
        graph.edges.push(LEdge {
            source: 0,
            target: 1,
            bend_points: vec![KVector { x: 40.0, y: 31.0 }, KVector { x: 40.0, y: 50.0 }],
            ..LEdge::default()
        });
        graph.ports[0].outgoing.push(0);
        graph.ports[1].incoming.push(0);
        route_hierarchical_port_edges(&mut graph).unwrap();
        assert_eq!(graph.nodes[0].position.x, -12.0);
        assert_eq!(graph.nodes[2].position.x, 210.0);
        assert_eq!(graph.edges[0].bend_points[0], KVector { x: 40.0, y: 30.0 });
        assert_eq!(graph.edges[0].bend_points[1], KVector { x: 40.0, y: 50.0 });
    }

    #[test]
    fn northern_dummies_are_unsupported() {
        let mut graph = LGraph {
            nodes: vec![dummy(PortSide::NORTH)],
            layers: vec![vec![0]],
            ..LGraph::default()
        };
        assert!(matches!(
            route_hierarchical_port_edges(&mut graph),
            Err(LayoutError::Unsupported(_))
        ));
    }
}
