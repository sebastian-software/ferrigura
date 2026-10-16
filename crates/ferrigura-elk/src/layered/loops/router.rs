//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/loops/routing/OrthogonalSelfLoopRouter.java
//!   and intermediate/loops/routing/AbstractSelfLoopRouter.java
//!
//! The last step of "Self-Loop routing": the bend points of each self loop edge, the labels' places
//! beside the routing slots, and the node margins that make room for both. The bend points are
//! relative to the node; the post-processor moves them to the drawing.

use crate::core::data::enums::PortSide;
use crate::core::math::{KVector, Spacing};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::loops::labels::SelfHyperLoopLabels;
use crate::layered::loops::model::{
    SIDE_COUNT, SelfHyperLoop, SelfLoopEdge, SelfLoopHolder, left, right,
};
use crate::layered::options;
use crate::layered::spacings::SPACING_EDGE_EDGE;

/// `OrthogonalSelfLoopRouter.EdgeRoutingDirection`. The variants keep the Java names.
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutingDirection {
    CLOCKWISE,
    COUNTER_CLOCKWISE,
}

/// The spacings the router reads, from the graph.
#[derive(Debug, Clone, Copy)]
struct Spacings {
    edge_edge: f64,
    edge_label: f64,
    node_self_loop: f64,
}

/// `OrthogonalSelfLoopRouter.routeSelfLoops`: routes every loop of the node, places its labels,
/// and sets the node's margins to hold the loops and their labels.
pub fn route_self_loops(
    graph: &mut LGraph,
    holder: &mut SelfLoopHolder,
) -> Result<(), LayoutError> {
    let node = holder.node;
    let node_size = graph.nodes[node].size;
    let spacings = Spacings {
        edge_edge: required(graph, SPACING_EDGE_EDGE)?,
        edge_label: required(graph, options::SPACING_EDGE_LABEL)?,
        node_self_loop: required(graph, options::SPACING_NODE_SELF_LOOP)?,
    };
    let mut new_margins: Spacing = graph.nodes[node].margin;

    // How far away from the node each routing slot on each side is, labels included.
    let positions = routing_slot_positions(graph, holder, spacings)?;

    for hyper in 0..holder.hyper_loops.len() {
        for edge in holder.hyper_loops[hyper].sl_edges.clone() {
            let sl_edge = holder.sl_edges[edge];
            let direction = edge_routing_direction(graph, holder, sl_edge)?;
            let bend_points =
                orthogonal_bend_points(graph, holder, sl_edge, direction, &positions)?;
            for &point in &bend_points {
                update_margins(node_size, &mut new_margins, point);
            }
            graph.edges[sl_edge.ledge].bend_points = bend_points;
        }
        // The labels, placed beside their routing slot (the routing made room for them).
        let loop_slots = holder.hyper_loops[hyper].routing_slot;
        if let Some(labels) = holder.hyper_loops[hyper].labels.as_mut() {
            place_labels(labels, loop_slots, &positions, spacings.edge_label)?;
            update_margins_for_labels(node_size, &mut new_margins, labels);
        }
    }

    graph.nodes[node].margin = new_margins;
    Ok(())
}

/// `LGraphUtil.getIndividualOrInherited` for a spacing: the graph's value, which is the only one
/// there is, as individual spacings are not ported.
fn required(graph: &LGraph, id: &str) -> Result<f64, LayoutError> {
    graph
        .properties
        .double_of(id)
        .ok_or_else(|| LayoutError::Unsupported(format!("a graph without {id}")))
}

/// The loop a self loop edge belongs to.
fn hyper_index(sl_edge: SelfLoopEdge) -> Result<usize, LayoutError> {
    sl_edge
        .hyper_loop
        .ok_or_else(|| LayoutError::Unsupported("a self loop edge without its loop".to_string()))
}

/// `computeRoutingSlotPositions`: the distance of each routing slot from the node, by side.
fn routing_slot_positions(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    spacings: Spacings,
) -> Result<Vec<Vec<f64>>, LayoutError> {
    let mut positions: Vec<Vec<f64>> = holder
        .routing_slot_count
        .iter()
        .map(|&count| vec![0.0; count])
        .collect();
    // The north and south slots first need the height of their largest labels.
    initialize_with_max_label_height(&mut positions, holder, PortSide::NORTH)?;
    initialize_with_max_label_height(&mut positions, holder, PortSide::SOUTH)?;
    for side in [
        PortSide::NORTH,
        PortSide::EAST,
        PortSide::SOUTH,
        PortSide::WEST,
    ] {
        compute_positions(graph, holder, &mut positions, side, spacings)?;
    }
    Ok(positions)
}

/// `initializeWithMaxLabelHeight`: each slot takes the largest label height of its loops.
fn initialize_with_max_label_height(
    positions: &mut [Vec<f64>],
    holder: &SelfLoopHolder,
    side: PortSide,
) -> Result<(), LayoutError> {
    for hyper in &holder.hyper_loops {
        let Some(labels) = &hyper.labels else {
            continue;
        };
        if labels.side != Some(side) {
            continue;
        }
        let slot = hyper.routing_slot[side as usize];
        let entry = positions[side as usize].get_mut(slot).ok_or_else(|| {
            LayoutError::Unsupported("a label on an unused routing slot".to_string())
        })?;
        *entry = entry.max(labels.size.y);
    }
    Ok(())
}

/// `computePositions`: each slot's position, moving away from the node, past its labels.
fn compute_positions(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    positions: &mut [Vec<f64>],
    side: PortSide,
    spacings: Spacings,
) -> Result<(), LayoutError> {
    let mut current = baseline_position(graph, holder.node, side, spacings.node_self_loop)?;
    // Northern and western coordinates decrease away from the node.
    let factor = if side == PortSide::NORTH || side == PortSide::WEST {
        -1.0
    } else {
        1.0
    };
    for entry in positions[side as usize].iter_mut() {
        // The slot holds the height or width of its largest label.
        let mut largest_label = *entry;
        if largest_label > 0.0 {
            largest_label += spacings.edge_label;
        }
        *entry = current;
        current += factor * (largest_label + spacings.edge_edge);
    }
    Ok(())
}

/// `computeBaselinePosition`: where the first slot of a side is, from the node and its margins.
fn baseline_position(
    graph: &LGraph,
    node: usize,
    side: PortSide,
    node_self_loop: f64,
) -> Result<f64, LayoutError> {
    let margin = graph.nodes[node].margin;
    let size = graph.nodes[node].size;
    match side {
        PortSide::NORTH => Ok(-margin.top - node_self_loop),
        PortSide::EAST => Ok(size.x + margin.right + node_self_loop),
        PortSide::SOUTH => Ok(size.y + margin.bottom + node_self_loop),
        PortSide::WEST => Ok(-margin.left - node_self_loop),
        PortSide::UNDEFINED => Err(LayoutError::Unsupported(
            "a routing slot on an undefined side".to_string(),
        )),
    }
}

/// `edgeRoutingDirection`: clockwise or counter-clockwise, from the sides of the ends.
fn edge_routing_direction(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    sl_edge: SelfLoopEdge,
) -> Result<RoutingDirection, LayoutError> {
    let source = &graph.ports[holder.sl_ports[sl_edge.source].lport];
    let target = &graph.ports[holder.sl_ports[sl_edge.target].lport];
    if source.side == target.side {
        // The same side: the port ids decide, as the Java does.
        return Ok(if source.id < target.id {
            RoutingDirection::CLOCKWISE
        } else {
            RoutingDirection::COUNTER_CLOCKWISE
        });
    }
    if right(source.side) == target.side {
        return Ok(RoutingDirection::CLOCKWISE);
    }
    if left(source.side) == target.side {
        return Ok(RoutingDirection::COUNTER_CLOCKWISE);
    }
    // Opposite sides: prefer clockwise, unless the loop does not occupy the side clockwise of the source.
    let hyper = &holder.hyper_loops[hyper_index(sl_edge)?];
    if hyper.occupied_sides.contains(&right(source.side)) {
        Ok(RoutingDirection::CLOCKWISE)
    } else {
        Ok(RoutingDirection::COUNTER_CLOCKWISE)
    }
}

/// `computeOrthogonalBendPoints`: the outer point at the source, the corners, and the outer point at
/// the target. The `modifyBendPoints` hook of the Java returns the points unchanged here.
fn orthogonal_bend_points(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    sl_edge: SelfLoopEdge,
    direction: RoutingDirection,
    positions: &[Vec<f64>],
) -> Result<Vec<KVector>, LayoutError> {
    let hyper = &holder.hyper_loops[hyper_index(sl_edge)?];
    let mut points = Vec::new();
    points.push(outer_bend_point(
        graph,
        holder,
        hyper,
        sl_edge.source,
        positions,
    )?);
    corner_bend_points(
        graph,
        holder,
        hyper,
        sl_edge,
        direction,
        positions,
        &mut points,
    )?;
    points.push(outer_bend_point(
        graph,
        holder,
        hyper,
        sl_edge.target,
        positions,
    )?);
    Ok(points)
}

/// `addOuterBendPoint`: the point where the loop leaves a port, on the port's routing slot.
fn outer_bend_point(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    hyper: &SelfHyperLoop,
    sl_port: usize,
    positions: &[Vec<f64>],
) -> Result<KVector, LayoutError> {
    let port = &graph.ports[holder.sl_ports[sl_port].lport];
    let side = port.side;
    let mut result = base_vector(side, hyper.routing_slot[side as usize], positions)?;
    let anchor_x = port.position.x + port.anchor.x;
    let anchor_y = port.position.y + port.anchor.y;
    match side {
        PortSide::NORTH | PortSide::SOUTH => result.x += anchor_x,
        PortSide::EAST | PortSide::WEST => result.y += anchor_y,
        PortSide::UNDEFINED => {
            return Err(LayoutError::Unsupported(
                "a self loop port without a side".to_string(),
            ));
        }
    }
    Ok(result)
}

/// `addCornerBendPoints`: the corners between the source's side and the target's, in the loop's
/// direction. Each corner adds the routing slot of the two sides it joins.
fn corner_bend_points(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    hyper: &SelfHyperLoop,
    sl_edge: SelfLoopEdge,
    direction: RoutingDirection,
    positions: &[Vec<f64>],
    points: &mut Vec<KVector>,
) -> Result<(), LayoutError> {
    let source_side = graph.ports[holder.sl_ports[sl_edge.source].lport].side;
    let target_side = graph.ports[holder.sl_ports[sl_edge.target].lport].side;
    if source_side == target_side {
        return Ok(());
    }
    let mut current = source_side;
    let mut steps = 0;
    while current != target_side {
        steps += 1;
        if steps > 4 {
            return Err(LayoutError::Unsupported(
                "a self loop whose sides do not join".to_string(),
            ));
        }
        let next = if direction == RoutingDirection::CLOCKWISE {
            right(current)
        } else {
            left(current)
        };
        let current_component =
            base_vector(current, hyper.routing_slot[current as usize], positions)?;
        let next_component = base_vector(next, hyper.routing_slot[next as usize], positions)?;
        // One component has its x set, the other its y; their sum is the corner.
        points.push(KVector {
            x: current_component.x + next_component.x,
            y: current_component.y + next_component.y,
        });
        current = next;
    }
    Ok(())
}

/// `getBaseVector`: the point on the side's routing slot, on the line of that side.
fn base_vector(
    side: PortSide,
    slot: usize,
    positions: &[Vec<f64>],
) -> Result<KVector, LayoutError> {
    let position = *positions
        .get(side as usize)
        .and_then(|slots| slots.get(slot))
        .ok_or_else(|| LayoutError::Unsupported("a routing slot without a position".to_string()))?;
    match side {
        PortSide::NORTH | PortSide::SOUTH => Ok(KVector {
            x: 0.0,
            y: position,
        }),
        PortSide::EAST | PortSide::WEST => Ok(KVector {
            x: position,
            y: 0.0,
        }),
        PortSide::UNDEFINED => Err(LayoutError::Unsupported(
            "a routing slot on an undefined side".to_string(),
        )),
    }
}

/// `updateNewNodeMargins` for a point: the margins grow to hold it.
fn update_margins(node_size: KVector, margins: &mut Spacing, point: KVector) {
    margins.left = margins.left.max(-point.x);
    margins.right = margins.right.max(point.x - node_size.x);
    margins.top = margins.top.max(-point.y);
    margins.bottom = margins.bottom.max(point.y - node_size.y);
}

/// `updateNewNodeMargins` for a label block: its two corners.
fn update_margins_for_labels(
    node_size: KVector,
    margins: &mut Spacing,
    labels: &SelfHyperLoopLabels,
) {
    update_margins(node_size, margins, labels.position);
    let far = KVector {
        x: labels.position.x + labels.size.x,
        y: labels.position.y + labels.size.y,
    };
    update_margins(node_size, margins, far);
}

/// `placeLabels` of the router: the labels sit beside their routing slot, with the edge label
/// spacing between them and the loop.
fn place_labels(
    labels: &mut SelfHyperLoopLabels,
    loop_slots: [usize; SIDE_COUNT],
    positions: &[Vec<f64>],
    edge_label: f64,
) -> Result<(), LayoutError> {
    let side = labels
        .side
        .ok_or_else(|| LayoutError::Unsupported("a label without a side".to_string()))?;
    let mut label_position = *positions
        .get(side as usize)
        .and_then(|slots| slots.get(loop_slots[side as usize]))
        .ok_or_else(|| LayoutError::Unsupported("a label on an unused routing slot".to_string()))?;
    match side {
        PortSide::NORTH => {
            label_position -= edge_label + labels.size.y;
            labels.position.y = label_position;
        }
        PortSide::SOUTH => {
            label_position += edge_label;
            labels.position.y = label_position;
        }
        PortSide::WEST => {
            label_position -= edge_label + labels.size.x;
            labels.position.x = label_position;
        }
        PortSide::EAST => {
            label_position += edge_label;
            labels.position.x = label_position;
        }
        PortSide::UNDEFINED => {
            return Err(LayoutError::Unsupported(
                "a label on an undefined side".to_string(),
            ));
        }
    }
    Ok(())
}
