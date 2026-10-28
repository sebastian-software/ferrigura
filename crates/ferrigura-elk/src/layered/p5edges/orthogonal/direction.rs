//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/orthogonal/direction/WestToEastRoutingStrategy.java
//!   with BaseRoutingDirectionStrategy.java (`addJunctionPointIfNecessary`, `getPortPositionOnHyperNode`)
//!
//! The routing strategy for west-to-east routing, the only direction `OrthogonalEdgeRouter`
//! instantiates. The north-to-south and south-to-north strategies serve north and south ports,
//! which the router rejects before routing, so they are not ported.
//!
//! The junction points are the set `createdJunctionPoints` of `BaseRoutingDirectionStrategy`, with
//! the junction lists of the edges kept on `LGraph::junction_points`. Java's `HashSet<KVector>` is
//! only tested for membership, so it is a `HashSet` of coordinate bits here: `KVector.equals` is
//! `==` on both coordinates, and its `hashCode` differs for -0.0 and 0.0, so bit equality agrees
//! with Java for every value routing produces.

use std::collections::HashSet;

use crate::core::data::enums::PortSide;
use crate::core::math::KVector;
use crate::layered::lgraph::LGraph;

use super::TOLERANCE;
use super::segment::Arena;

/// `WestToEastRoutingStrategy.getSourcePortSide()`: the ports that edges leave from.
pub const SOURCE_PORT_SIDE: PortSide = PortSide::EAST;
/// `WestToEastRoutingStrategy.getTargetPortSide()`: the ports that edges enter.
pub const TARGET_PORT_SIDE: PortSide = PortSide::WEST;

/// The bits of a junction point's coordinates, the key of the created junction points.
pub type JunctionKey = (u64, u64);

/// `getPortPositionOnHyperNode`: the port's y coordinate, computed in Java's operand order.
pub fn port_position_on_hyper_node(graph: &LGraph, port: usize) -> f64 {
    let port = &graph.ports[port];
    graph.nodes[port.node].position.y + port.position.y + port.anchor.y
}

/// `LPort.getAbsoluteAnchor().y`, which is `KVector.sum(owner position, port position, anchor)`:
/// the sum starts at zero and adds the three in turn.
fn absolute_anchor_y(graph: &LGraph, port: usize) -> f64 {
    let port = &graph.ports[port];
    (0.0 + graph.nodes[port.node].position.y + port.position.y) + port.anchor.y
}

/// `LEdge.isSelfLoop()`. Self loops are rejected before routing, so the check only guards the
/// bend point loop as Java has it.
fn is_self_loop(graph: &LGraph, edge: usize) -> bool {
    let edge = &graph.edges[edge];
    graph.ports[edge.source].node == graph.ports[edge.target].node
}

/// The junction key of a point, or `None` for a point that is NaN in a coordinate: Java's
/// `equals` is never true for it, so it is never found.
fn junction_key(point: KVector) -> Option<JunctionKey> {
    if point.x.is_nan() || point.y.is_nan() {
        None
    } else {
        Some((point.x.to_bits(), point.y.to_bits()))
    }
}

/// `WestToEastRoutingStrategy.calculateBendPoints`: sets the bend points of the edges leaving the
/// segment's ports. The segment's split partner, if any, adds two bends. A dummy partner does
/// nothing: its partner does the work.
pub fn calculate_bend_points(
    graph: &mut LGraph,
    arena: &Arena,
    segment: usize,
    start_pos: f64,
    edge_spacing: f64,
    created: &mut HashSet<JunctionKey>,
) {
    if arena.is_dummy(segment) {
        return;
    }
    let segment_data = &arena.segments[segment];
    let segment_x = start_pos + f64::from(segment_data.routing_slot) * edge_spacing;

    for port in segment_data.ports.clone() {
        let source_y = absolute_anchor_y(graph, port);

        for edge in graph.ports[port].outgoing.clone() {
            if is_self_loop(graph, edge) {
                continue;
            }
            let target_y = absolute_anchor_y(graph, graph.edges[edge].target);
            if (source_y - target_y).abs() > TOLERANCE {
                // The bends move to the split partner's slot if this segment was split.
                let mut current_x = segment_x;
                let mut current_segment = segment;

                let bend = KVector {
                    x: current_x,
                    y: source_y,
                };
                graph.edges[edge].bend_points.push(bend);
                add_junction_point_if_necessary(graph, created, arena, edge, current_segment, bend);

                if let Some(partner) = segment_data.split_partner {
                    let split_y = arena.segments[partner].incoming[0];

                    let bend = KVector {
                        x: current_x,
                        y: split_y,
                    };
                    graph.edges[edge].bend_points.push(bend);
                    add_junction_point_if_necessary(
                        graph,
                        created,
                        arena,
                        edge,
                        current_segment,
                        bend,
                    );

                    current_x =
                        start_pos + f64::from(arena.segments[partner].routing_slot) * edge_spacing;
                    current_segment = partner;

                    let bend = KVector {
                        x: current_x,
                        y: split_y,
                    };
                    graph.edges[edge].bend_points.push(bend);
                    add_junction_point_if_necessary(
                        graph,
                        created,
                        arena,
                        edge,
                        current_segment,
                        bend,
                    );
                }

                let bend = KVector {
                    x: current_x,
                    y: target_y,
                };
                graph.edges[edge].bend_points.push(bend);
                add_junction_point_if_necessary(graph, created, arena, edge, current_segment, bend);
            }
        }
    }
}

/// `addJunctionPointIfNecessary` with `vertical` set, which is all this strategy passes: the
/// bend point's y decides whether a junction lies inside the segment, or at a boundary where
/// another edge joins it.
fn add_junction_point_if_necessary(
    graph: &mut LGraph,
    created: &mut HashSet<JunctionKey>,
    arena: &Arena,
    edge: usize,
    segment: usize,
    pos: KVector,
) {
    let p = pos.y;

    if junction_key(pos).is_some_and(|key| created.contains(&key)) {
        return;
    }

    let segment = &arena.segments[segment];
    let point_inside_edge_segment = p > segment.start && p < segment.end;

    let mut point_at_segment_boundary = false;
    if !segment.incoming.is_empty() && !segment.outgoing.is_empty() {
        let (incoming_first, incoming_last) = (
            segment.incoming[0],
            segment.incoming[segment.incoming.len() - 1],
        );
        let (outgoing_first, outgoing_last) = (
            segment.outgoing[0],
            segment.outgoing[segment.outgoing.len() - 1],
        );

        // Does the bend point join another edge at the segment's start?
        point_at_segment_boundary |=
            (p - incoming_first).abs() < TOLERANCE && (p - outgoing_first).abs() < TOLERANCE;
        // Does it join another edge at the segment's end?
        point_at_segment_boundary |=
            (p - incoming_last).abs() < TOLERANCE && (p - outgoing_last).abs() < TOLERANCE;
    }

    if point_inside_edge_segment || point_at_segment_boundary {
        graph.junction_points.entry(edge).or_default().push(pos);
        if let Some(key) = junction_key(pos) {
            created.insert(key);
        }
    }
}
