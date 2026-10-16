//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/loops/routing/RoutingDirector.java
//!   and intermediate/loops/routing/LabelPlacer.java
//!
//! The first two steps of "Self-Loop routing": the leftmost and rightmost port of each loop, which
//! say how the loop runs around the node, and the placement of the loop's labels.

use std::collections::BTreeSet;

use crate::core::data::enums::{PortSide, SelfLoopOrderingStrategy};
use crate::java::sort::sort_by;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::loops::labels::Alignment;
use crate::layered::loops::model::{SelfLoopHolder, SelfLoopType, right};
use crate::layered::loops::ordering::sorted_two_side_port_sides;
use crate::layered::options;

/// The penalty of an unconnected port, `UNCONNECTED_PORT_PENALTY`.
const UNCONNECTED_PORT_PENALTY: i64 = 1;
/// The penalty of a connected port, `CONNECTED_PORT_PENALTY`.
const CONNECTED_PORT_PENALTY: i64 = 3;

/// `RoutingDirector.determineLoopRoutes`: gives the ports ids by their order in the node's list,
/// and chooses for each loop the leftmost and rightmost port its route runs between.
pub fn determine_loop_routes(
    graph: &mut LGraph,
    holder: &mut SelfLoopHolder,
) -> Result<(), LayoutError> {
    let node_ports = graph.nodes[holder.node].ports.clone();
    for (index, &lport) in node_ports.iter().enumerate() {
        graph.ports[lport].id = index;
    }
    // Sort each loop's ports by id.
    for hyper in 0..holder.hyper_loops.len() {
        let mut ports = holder.hyper_loops[hyper].sl_ports.clone();
        {
            let view: &LGraph = &*graph;
            let holder_view: &SelfLoopHolder = &*holder;
            sort_by(&mut ports, |&a, &b| {
                port_id(view, holder_view, a).cmp(&port_id(view, holder_view, b))
            });
        }
        holder.hyper_loops[hyper].sl_ports = ports;
    }

    let penalties = penalties_of(graph, &node_ports);
    for hyper in 0..holder.hyper_loops.len() {
        match holder.hyper_loops[hyper].loop_type {
            Some(SelfLoopType::ONE_SIDE) => {
                let side = port_side(graph, holder, first_port(holder, hyper)?);
                assign_leftmost_rightmost(graph, holder, hyper, side, side)?;
            }
            Some(SelfLoopType::TWO_SIDES_CORNER) => {
                let [first, second] = sorted_two_side_port_sides(holder, hyper)?;
                assign_leftmost_rightmost(graph, holder, hyper, first, second)?;
            }
            Some(SelfLoopType::TWO_SIDES_OPPOSING) => {
                determine_two_side_opposing(graph, holder, hyper, &penalties)?;
            }
            Some(SelfLoopType::THREE_SIDES) => {
                determine_three_side(graph, holder, hyper)?;
            }
            Some(SelfLoopType::FOUR_SIDES) => {
                determine_four_side(graph, holder, hyper, &penalties)?;
            }
            None => {
                return Err(LayoutError::Unsupported(
                    "a self loop without a type".to_string(),
                ));
            }
        }
        compute_occupied_sides(graph, holder, hyper)?;
    }
    Ok(())
}

/// The id of a self loop port's port in the node's list.
fn port_id(graph: &LGraph, holder: &SelfLoopHolder, sl_port: usize) -> usize {
    graph.ports[holder.sl_ports[sl_port].lport].id
}

/// The side of a self loop port.
fn port_side(graph: &LGraph, holder: &SelfLoopHolder, sl_port: usize) -> PortSide {
    graph.ports[holder.sl_ports[sl_port].lport].side
}

/// The first port of a loop, or an error for a loop without ports.
fn first_port(holder: &SelfLoopHolder, hyper: usize) -> Result<usize, LayoutError> {
    holder.hyper_loops[hyper]
        .sl_ports
        .first()
        .copied()
        .ok_or_else(|| LayoutError::Unsupported("a self loop without ports".to_string()))
}

/// `RoutingDirector.computePenalties` and `computeEdgePenalty`: the penalty of each prefix of the
/// node's ports, where a connected port costs more than an unconnected one.
fn penalties_of(graph: &LGraph, node_ports: &[usize]) -> Vec<i64> {
    let mut sum = 0;
    node_ports
        .iter()
        .map(|&lport| {
            let port = &graph.ports[lport];
            sum += if port.incoming.is_empty() && port.outgoing.is_empty() {
                UNCONNECTED_PORT_PENALTY
            } else {
                CONNECTED_PORT_PENALTY
            };
            sum
        })
        .collect()
}

/// `RoutingDirector.computeEdgePenalty`: the penalty of an edge between two ports, going clockwise
/// from `left_id` to `right_id`, which may wrap around the first port of the node.
fn edge_penalty(penalties: &[i64], left_id: usize, right_id: usize) -> i64 {
    let count = penalties.len();
    if count == 0 {
        return 0;
    }
    // The port left of the rightmost one, wrapping to the last port when it is the first.
    let left_of_right = if right_id == 0 {
        count - 1
    } else {
        right_id - 1
    };
    if left_id <= left_of_right {
        penalties[left_of_right] - penalties[left_id]
    } else {
        // The edge passes the corner where the indices reset, so it is the sum of two parts.
        penalties[count - 1] - penalties[left_id] + penalties[left_of_right]
    }
}

/// The penalty of an edge between two self loop ports.
fn penalty_between(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    penalties: &[i64],
    left_port: usize,
    right_port: usize,
) -> i64 {
    edge_penalty(
        penalties,
        port_id(graph, holder, left_port),
        port_id(graph, holder, right_port),
    )
}

/// The lowest-id port of a loop on a side.
fn lowest_port_on_side(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    hyper: usize,
    side: PortSide,
) -> Result<usize, LayoutError> {
    highest_or_lowest(graph, holder, hyper, side, |a, b| a < b)
}

/// The highest-id port of a loop on a side.
fn highest_port_on_side(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    hyper: usize,
    side: PortSide,
) -> Result<usize, LayoutError> {
    highest_or_lowest(graph, holder, hyper, side, |a, b| a > b)
}

/// The port of a loop on a side that is best by `better`, comparing ids.
fn highest_or_lowest(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    hyper: usize,
    side: PortSide,
    better: impl Fn(usize, usize) -> bool,
) -> Result<usize, LayoutError> {
    let ports = holder.hyper_loops[hyper]
        .ports_by_side
        .get(&side)
        .ok_or_else(|| {
            LayoutError::Unsupported("a self loop without a port on a side".to_string())
        })?;
    let mut best: Option<usize> = None;
    for &port in ports {
        match best {
            Some(current)
                if !better(
                    port_id(graph, holder, port),
                    port_id(graph, holder, current),
                ) => {}
            _ => best = Some(port),
        }
    }
    best.ok_or_else(|| LayoutError::Unsupported("a self loop without a port on a side".to_string()))
}

/// `RoutingDirector.assignLeftmostRightmostPorts`.
fn assign_leftmost_rightmost(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
    leftmost_side: PortSide,
    rightmost_side: PortSide,
) -> Result<(), LayoutError> {
    let leftmost = lowest_port_on_side(graph, holder, hyper, leftmost_side)?;
    let rightmost = highest_port_on_side(graph, holder, hyper, rightmost_side)?;
    holder.hyper_loops[hyper].leftmost = Some(leftmost);
    holder.hyper_loops[hyper].rightmost = Some(rightmost);
    Ok(())
}

/// `RoutingDirector.determineTwoSideOpposingLoopRoutes`: the cheaper of the two ways around. The
/// sides come in the multimap's key order, which decides a tie.
fn determine_two_side_opposing(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
    penalties: &[i64],
) -> Result<(), LayoutError> {
    let sides: Vec<PortSide> = holder.hyper_loops[hyper].side_order.clone();
    let [first, second] = match sides.as_slice() {
        [first, second] => [*first, *second],
        _ => {
            return Err(LayoutError::Unsupported(
                "a two-side self loop without two sides".to_string(),
            ));
        }
    };
    // Option 1: from the lowest port on the first side to the highest on the second.
    let option1_left = lowest_port_on_side(graph, holder, hyper, first)?;
    let option1_right = highest_port_on_side(graph, holder, hyper, second)?;
    let option1 = penalty_between(graph, holder, penalties, option1_left, option1_right);
    // Option 2: the other way around.
    let option2_left = lowest_port_on_side(graph, holder, hyper, second)?;
    let option2_right = highest_port_on_side(graph, holder, hyper, first)?;
    let option2 = penalty_between(graph, holder, penalties, option2_left, option2_right);
    let (leftmost, rightmost) = if option1 <= option2 {
        (option1_left, option1_right)
    } else {
        (option2_left, option2_right)
    };
    holder.hyper_loops[hyper].leftmost = Some(leftmost);
    holder.hyper_loops[hyper].rightmost = Some(rightmost);
    Ok(())
}

/// `RoutingDirector.determineThreeSideLoopRoutes`: the sides run from the side the loop misses.
fn determine_three_side(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
) -> Result<(), LayoutError> {
    let sides = &holder.hyper_loops[hyper].ports_by_side;
    let missing = [
        PortSide::NORTH,
        PortSide::EAST,
        PortSide::SOUTH,
        PortSide::WEST,
    ]
    .into_iter()
    .find(|side| !sides.contains_key(side));
    let (leftmost_side, rightmost_side) = match missing {
        Some(PortSide::NORTH) => (PortSide::EAST, PortSide::WEST),
        Some(PortSide::EAST) => (PortSide::SOUTH, PortSide::NORTH),
        Some(PortSide::SOUTH) => (PortSide::WEST, PortSide::EAST),
        Some(PortSide::WEST) => (PortSide::NORTH, PortSide::SOUTH),
        _ => {
            return Err(LayoutError::Unsupported(
                "a three-side self loop without a missing side".to_string(),
            ));
        }
    };
    assign_leftmost_rightmost(graph, holder, hyper, leftmost_side, rightmost_side)
}

/// `RoutingDirector.determineFourSideLoopRoutes`: the route is split where the penalty of an edge
/// between adjacent ports is highest.
fn determine_four_side(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
    penalties: &[i64],
) -> Result<(), LayoutError> {
    // The ports are sorted by id.
    let ports = holder.hyper_loops[hyper].sl_ports.clone();
    let (Some(&last), Some(&first)) = (ports.last(), ports.first()) else {
        return Err(LayoutError::Unsupported(
            "a self loop without ports".to_string(),
        ));
    };
    let mut worst_left = last;
    let mut worst_right = first;
    let mut worst_penalty = penalty_between(graph, holder, penalties, worst_left, worst_right);
    for pair in ports.windows(2) {
        let (current_left, current_right) = (pair[0], pair[1]);
        let current = penalty_between(graph, holder, penalties, current_left, current_right);
        if current > worst_penalty {
            worst_left = current_left;
            worst_right = current_right;
            worst_penalty = current;
        }
    }
    // The loop is not drawn between the left and right ports, so the two are switched.
    holder.hyper_loops[hyper].leftmost = Some(worst_right);
    holder.hyper_loops[hyper].rightmost = Some(worst_left);
    Ok(())
}

/// `RoutingDirector.computeOccupiedPortSides`: the sides from the leftmost port's side clockwise to
/// the rightmost port's side.
fn compute_occupied_sides(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
) -> Result<(), LayoutError> {
    let (Some(leftmost), Some(rightmost)) = (
        holder.hyper_loops[hyper].leftmost,
        holder.hyper_loops[hyper].rightmost,
    ) else {
        return Err(LayoutError::Unsupported(
            "a self loop without its leftmost and rightmost ports".to_string(),
        ));
    };
    let target = port_side(graph, holder, rightmost);
    let mut current = port_side(graph, holder, leftmost);
    let mut occupied = BTreeSet::new();
    let mut steps = 0;
    while current != target {
        occupied.insert(current);
        current = right(current);
        steps += 1;
        if steps > 4 {
            return Err(LayoutError::Unsupported(
                "a self loop whose sides do not join".to_string(),
            ));
        }
    }
    occupied.insert(current);
    holder.hyper_loops[hyper].occupied_sides = occupied;
    Ok(())
}

/// `LabelPlacer.placeLabels`: the side and alignment of each loop's labels, then their coordinates.
/// Label management needs a label manager, which no graph of this port has.
pub fn place_labels(graph: &mut LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    assign_side_and_alignment(graph, holder)?;
    for hyper in 0..holder.hyper_loops.len() {
        if holder.hyper_loops[hyper].labels.is_some() {
            compute_coordinates(graph, holder, hyper)?;
        }
    }
    Ok(())
}

/// `LabelPlacer.assignSideAndAlignment`: most loops are simple; the one-sided northern and southern
/// ones are placed together when the loops are sequenced.
fn assign_side_and_alignment(
    graph: &mut LGraph,
    holder: &mut SelfLoopHolder,
) -> Result<(), LayoutError> {
    let ordering = graph.nodes[holder.node]
        .properties
        .enum_with_default::<SelfLoopOrderingStrategy>(options::EDGE_ROUTING_SELF_LOOP_ORDERING);
    let sequenced = ordering == SelfLoopOrderingStrategy::SEQUENCED;
    let mut northern: Vec<usize> = Vec::new();
    let mut southern: Vec<usize> = Vec::new();
    for hyper in 0..holder.hyper_loops.len() {
        if holder.hyper_loops[hyper].labels.is_none() {
            continue;
        }
        match holder.hyper_loops[hyper].loop_type {
            Some(SelfLoopType::ONE_SIDE) => {
                let Some(&side) = holder.hyper_loops[hyper].occupied_sides.iter().next() else {
                    return Err(LayoutError::Unsupported(
                        "a self loop without an occupied side".to_string(),
                    ));
                };
                if sequenced && side == PortSide::NORTH {
                    northern.push(hyper);
                } else if sequenced && side == PortSide::SOUTH {
                    southern.push(hyper);
                } else {
                    assign_one_sided_simple(graph, holder, hyper, side)?;
                }
            }
            Some(SelfLoopType::TWO_SIDES_CORNER) => {
                assign_two_sides_corner(graph, holder, hyper)?;
            }
            Some(SelfLoopType::TWO_SIDES_OPPOSING | SelfLoopType::THREE_SIDES) => {
                assign_two_sides_opposing_and_three(holder, hyper)?;
            }
            Some(SelfLoopType::FOUR_SIDES) => {
                assign_four_sides(graph, holder, hyper)?;
            }
            None => {
                return Err(LayoutError::Unsupported(
                    "a self loop without a type".to_string(),
                ));
            }
        }
    }
    if sequenced {
        if !northern.is_empty() {
            assign_one_sided_sequenced(graph, holder, northern, PortSide::NORTH)?;
        }
        if !southern.is_empty() {
            assign_one_sided_sequenced(graph, holder, southern, PortSide::SOUTH)?;
        }
    }
    Ok(())
}

/// `LabelPlacer.assignOneSidedSimpleSideAndAlignment`.
fn assign_one_sided_simple(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
    side: PortSide,
) -> Result<(), LayoutError> {
    match side {
        PortSide::EAST | PortSide::WEST => {
            // Aligned with the topmost port, which is the leftmost or the rightmost one.
            let (Some(leftmost), Some(rightmost)) = (
                holder.hyper_loops[hyper].leftmost,
                holder.hyper_loops[hyper].rightmost,
            ) else {
                return Err(LayoutError::Unsupported(
                    "a self loop without its leftmost and rightmost ports".to_string(),
                ));
            };
            let mut topmost = leftmost;
            if graph.ports[holder.sl_ports[rightmost].lport].position.y
                < graph.ports[holder.sl_ports[topmost].lport].position.y
            {
                topmost = rightmost;
            }
            assign_side(holder, hyper, side, Alignment::TOP, Some(topmost));
            Ok(())
        }
        PortSide::NORTH | PortSide::SOUTH => {
            assign_side(holder, hyper, side, Alignment::CENTER, None);
            Ok(())
        }
        _ => Err(LayoutError::Unsupported(
            "a one-sided self loop without a side".to_string(),
        )),
    }
}

/// `LabelPlacer.assignOneSidedSequencedSideAndAlignment`: pairs the loops from the outside in, so
/// that their labels face away from each other.
fn assign_one_sided_sequenced(
    graph: &mut LGraph,
    holder: &mut SelfLoopHolder,
    loops: Vec<usize>,
    side: PortSide,
) -> Result<(), LayoutError> {
    // Ensure sensible port ids, by the node's port list.
    let node_ports = graph.nodes[holder.node].ports.clone();
    for (index, &lport) in node_ports.iter().enumerate() {
        graph.ports[lport].id = index;
    }
    // Sort by the leftmost port's id: ascending for the north, descending for the south.
    let mut keyed: Vec<(i64, usize)> = Vec::with_capacity(loops.len());
    for &hyper in &loops {
        let Some(leftmost) = holder.hyper_loops[hyper].leftmost else {
            return Err(LayoutError::Unsupported(
                "a self loop without a leftmost port".to_string(),
            ));
        };
        let id = port_id(graph, holder, leftmost) as i64;
        keyed.push((if side == PortSide::NORTH { id } else { -id }, hyper));
    }
    sort_by(&mut keyed, |a, b| a.0.cmp(&b.0));
    let loops: Vec<usize> = keyed.into_iter().map(|(_, hyper)| hyper).collect();
    let Some(last) = loops.len().checked_sub(1) else {
        return Ok(());
    };

    // From the outside loops towards the inside ones, in pairs.
    let (mut left, mut right) = (0usize, last);
    while left < right {
        let (left_loop, right_loop) = (loops[left], loops[right]);
        // A northern loop's leftmost port is left of its rightmost; a southern one's is the other way.
        let (left_reference, right_reference) = if side == PortSide::NORTH {
            (
                holder.hyper_loops[left_loop].rightmost,
                holder.hyper_loops[right_loop].leftmost,
            )
        } else {
            (
                holder.hyper_loops[left_loop].leftmost,
                holder.hyper_loops[right_loop].rightmost,
            )
        };
        assign_side(holder, left_loop, side, Alignment::RIGHT, left_reference);
        assign_side(holder, right_loop, side, Alignment::LEFT, right_reference);
        left += 1;
        right -= 1;
    }
    // A single loop in the middle.
    if left == right {
        assign_side(holder, loops[left], side, Alignment::CENTER, None);
    }
    Ok(())
}

/// `LabelPlacer.assignTwoSidesCornerSideAndAlignment`.
fn assign_two_sides_corner(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
) -> Result<(), LayoutError> {
    let (Some(leftmost), Some(rightmost)) = (
        holder.hyper_loops[hyper].leftmost,
        holder.hyper_loops[hyper].rightmost,
    ) else {
        return Err(LayoutError::Unsupported(
            "a self loop without its leftmost and rightmost ports".to_string(),
        ));
    };
    let left_side = port_side(graph, holder, leftmost);
    let right_side = port_side(graph, holder, rightmost);
    let (side, alignment, reference) = if left_side == PortSide::NORTH {
        (PortSide::NORTH, Alignment::LEFT, leftmost)
    } else if right_side == PortSide::NORTH {
        (PortSide::NORTH, Alignment::RIGHT, rightmost)
    } else if left_side == PortSide::SOUTH {
        (PortSide::SOUTH, Alignment::RIGHT, leftmost)
    } else if right_side == PortSide::SOUTH {
        (PortSide::SOUTH, Alignment::LEFT, rightmost)
    } else {
        return Err(LayoutError::Unsupported(
            "a corner self loop without a north or south port".to_string(),
        ));
    };
    assign_side(holder, hyper, side, alignment, Some(reference));
    Ok(())
}

/// `LabelPlacer.assignTwoSidesOpposingAndThreeSidesSideAndAlignment`: the labels go to the side the
/// loop leaves free, or to the north, aligned with the port at the end the loop does not cover.
fn assign_two_sides_opposing_and_three(
    holder: &mut SelfLoopHolder,
    hyper: usize,
) -> Result<(), LayoutError> {
    let occupied = holder.hyper_loops[hyper].occupied_sides.clone();
    let (Some(leftmost), Some(rightmost)) = (
        holder.hyper_loops[hyper].leftmost,
        holder.hyper_loops[hyper].rightmost,
    ) else {
        return Err(LayoutError::Unsupported(
            "a self loop without its leftmost and rightmost ports".to_string(),
        ));
    };
    if !occupied.contains(&PortSide::NORTH) {
        assign_side(holder, hyper, PortSide::SOUTH, Alignment::CENTER, None);
    } else if !occupied.contains(&PortSide::SOUTH) {
        assign_side(holder, hyper, PortSide::NORTH, Alignment::CENTER, None);
    } else if !occupied.contains(&PortSide::WEST) {
        assign_side(
            holder,
            hyper,
            PortSide::NORTH,
            Alignment::LEFT,
            Some(leftmost),
        );
    } else if !occupied.contains(&PortSide::EAST) {
        assign_side(
            holder,
            hyper,
            PortSide::NORTH,
            Alignment::RIGHT,
            Some(rightmost),
        );
    } else {
        return Err(LayoutError::Unsupported(
            "a self loop covering all four sides".to_string(),
        ));
    }
    Ok(())
}

/// `LabelPlacer.assignFourSidesSideAndAlignment`. The Java reads the leftmost port for both ends,
/// and the port is kept that way here.
fn assign_four_sides(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
) -> Result<(), LayoutError> {
    let Some(leftmost) = holder.hyper_loops[hyper].leftmost else {
        return Err(LayoutError::Unsupported(
            "a self loop without a leftmost port".to_string(),
        ));
    };
    let leftmost_side = port_side(graph, holder, leftmost);
    let rightmost_side = port_side(graph, holder, leftmost);
    if leftmost_side == PortSide::NORTH || rightmost_side == PortSide::NORTH {
        assign_side(holder, hyper, PortSide::SOUTH, Alignment::CENTER, None);
    } else {
        assign_side(holder, hyper, PortSide::NORTH, Alignment::CENTER, None);
    }
    Ok(())
}

/// `LabelPlacer.assignSideAndAlignment`: sets the side, alignment and reference of a loop's labels.
fn assign_side(
    holder: &mut SelfLoopHolder,
    hyper: usize,
    side: PortSide,
    alignment: Alignment,
    reference: Option<usize>,
) {
    if let Some(labels) = holder.hyper_loops[hyper].labels.as_mut() {
        labels.side = Some(side);
        labels.alignment = Some(alignment);
        labels.alignment_reference = reference;
    }
}

/// `LabelPlacer.computeCoordinates`: the labels' position along their side, from their alignment.
fn compute_coordinates(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
) -> Result<(), LayoutError> {
    let node_size = graph.nodes[holder.node].size;
    let reference_port = holder.hyper_loops[hyper]
        .labels
        .as_ref()
        .and_then(|labels| labels.alignment_reference)
        .map(|sl| {
            let lport = holder.sl_ports[sl].lport;
            let port = &graph.ports[lport];
            (
                port.position.x + port.anchor.x,
                port.position.y + port.anchor.y,
            )
        });
    let Some(labels) = holder.hyper_loops[hyper].labels.as_mut() else {
        return Ok(());
    };
    let size = labels.size;
    match labels.alignment {
        Some(Alignment::CENTER) => {
            labels.position.x = (node_size.x - size.x) / 2.0;
        }
        Some(Alignment::LEFT) => {
            let (x, _) = reference_port.ok_or_else(missing_reference)?;
            labels.position.x = x;
        }
        Some(Alignment::RIGHT) => {
            let (x, _) = reference_port.ok_or_else(missing_reference)?;
            labels.position.x = x - size.x;
        }
        Some(Alignment::TOP) => {
            let (_, y) = reference_port.ok_or_else(missing_reference)?;
            labels.position.y = y;
        }
        None => {
            return Err(LayoutError::Unsupported(
                "a self loop label without an alignment".to_string(),
            ));
        }
    }
    Ok(())
}

/// The error for a label alignment that needs a port which is not there.
fn missing_reference() -> LayoutError {
    LayoutError::Unsupported("a self loop label aligned to a missing port".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edge_penalty_is_the_sum_of_the_ports_it_passes_going_clockwise() {
        // Four ports with penalties 1, 4, 5 and 8: the prefix sums of the port penalties.
        let penalties = [1, 4, 5, 8];
        // From port 0 to port 2 passes port 1 only.
        assert_eq!(edge_penalty(&penalties, 0, 2), 3);
        // From port 2 to port 0 wraps around through port 3.
        assert_eq!(edge_penalty(&penalties, 2, 0), 3);
        // From port 3 to port 1 wraps the corner, through port 0.
        assert_eq!(edge_penalty(&penalties, 3, 1), 1);
    }
}
