//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/SelfLoopPortRestorer.java,
//!   intermediate/loops/ordering/PortSideAssigner.java and intermediate/loops/ordering/PortRestorer.java
//!
//! Phase 3 of the self loops, "Self-Loop ordering": the hidden ports of a node get their sides, and
//! are put back into the node's port list in the order that keeps the self loops from crossing.
//!
//! A northern port whose north/south port dummy has edges to the west, and a southern port whose
//! dummy has edges to the east, stay at the start of their side (`PortRestorer`'s
//! `isNorthSouthPortWith...` predicates).

use crate::core::data::enums::{
    NodeType, PortConstraints, PortSide, SelfLoopDistributionStrategy, SelfLoopOrderingStrategy,
};
use crate::java::sort::sort_by;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::loops::model::{
    SIDE_COUNT, SelfLoopHolder, SelfLoopType, port_net_flow, set_port_side,
};
use crate::layered::options;

/// `PortRestorer.PortSideArea`: the start, middle and end of a side. The variants keep the Java names.
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
enum Area {
    START,
    MIDDLE,
    END,
}

/// The number of areas of a side, `PortSideArea.values().length`.
const AREA_COUNT: usize = 3;

/// `PortSideAssigner.Target`: the first and second side each equal distribution target spans.
const TARGETS: [(PortSide, PortSide); 8] = [
    (PortSide::NORTH, PortSide::NORTH),
    (PortSide::SOUTH, PortSide::SOUTH),
    (PortSide::EAST, PortSide::EAST),
    (PortSide::WEST, PortSide::WEST),
    (PortSide::WEST, PortSide::NORTH),
    (PortSide::NORTH, PortSide::EAST),
    (PortSide::SOUTH, PortSide::WEST),
    (PortSide::EAST, PortSide::SOUTH),
];

/// `SelfLoopPortRestorer.process`: for each node with hidden self loop ports, assigns their sides
/// and restores them, and computes the types of the node's loops.
pub fn restore_hidden_ports(graph: &mut LGraph) -> Result<(), LayoutError> {
    let layer_nodes: Vec<usize> = graph.layers.iter().flatten().copied().collect();
    for node in layer_nodes {
        if graph.nodes[node].node_type != NodeType::NORMAL || graph.nodes[node].self_loops.is_none()
        {
            continue;
        }
        let Some(mut holder) = graph.nodes[node].self_loops.take() else {
            continue;
        };
        let result = process_node(graph, &mut holder);
        graph.nodes[node].self_loops = Some(holder);
        result?;
    }
    Ok(())
}

/// `SelfLoopPortRestorer.processNode`.
fn process_node(graph: &mut LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    if !holder.ports_hidden {
        return compute_self_loop_types(graph, holder);
    }
    match graph.nodes[holder.node].original_port_constraints {
        Some(PortConstraints::UNDEFINED | PortConstraints::FREE) => {
            // Assign port sides first, then restore the ports as for fixed sides.
            assign_port_sides(graph, holder)?;
            compute_self_loop_types(graph, holder)?;
            restore_node_ports(graph, holder)
        }
        Some(PortConstraints::FIXED_SIDE) => {
            compute_self_loop_types(graph, holder)?;
            restore_node_ports(graph, holder)
        }
        _ => Err(LayoutError::Unsupported(
            "hidden self loop ports on a node whose port order is fixed".to_string(),
        )),
    }
}

/// `SelfLoopPortRestorer.computeSelfLoopTypes`.
fn compute_self_loop_types(graph: &LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    for hyper in 0..holder.hyper_loops.len() {
        holder.compute_ports_per_side(graph, hyper)?;
    }
    Ok(())
}

/// The lport of a self loop port.
fn lport_of(holder: &SelfLoopHolder, sl_port: usize) -> usize {
    holder.sl_ports[sl_port].lport
}

/// `PortSideAssigner.assignPortSides`: the sides of the hidden ports, by the node's distribution.
fn assign_port_sides(graph: &mut LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    let strategy = graph.nodes[holder.node]
        .properties
        .enum_with_default::<SelfLoopDistributionStrategy>(
            options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION,
        );
    match strategy {
        SelfLoopDistributionStrategy::NORTH => {
            for hyper in 0..holder.hyper_loops.len() {
                for port in holder.hidden_ports(hyper) {
                    set_port_side(graph, lport_of(holder, port), PortSide::NORTH);
                }
            }
        }
        SelfLoopDistributionStrategy::NORTH_SOUTH => assign_to_north_or_south(graph, holder),
        SelfLoopDistributionStrategy::EQUALLY => assign_to_all_sides(graph, holder),
    }
    Ok(())
}

/// `PortSideAssigner.assignToNorthOrSouthSide`: each loop goes to the side with fewer ports.
fn assign_to_north_or_south(graph: &mut LGraph, holder: &SelfLoopHolder) {
    let (mut north_ports, mut south_ports) = (0usize, 0usize);
    for hyper in 0..holder.hyper_loops.len() {
        let hidden = holder.hidden_ports(hyper);
        let side = if north_ports <= south_ports {
            north_ports += hidden.len();
            PortSide::NORTH
        } else {
            south_ports += hidden.len();
            PortSide::SOUTH
        };
        for port in hidden {
            set_port_side(graph, lport_of(holder, port), side);
        }
    }
}

/// `PortSideAssigner.assignToAllSides`: the loops, by their port count, go to the targets in turn.
fn assign_to_all_sides(graph: &mut LGraph, holder: &mut SelfLoopHolder) {
    let mut sorted: Vec<usize> = (0..holder.hyper_loops.len()).collect();
    sort_by(&mut sorted, |&a, &b| {
        holder.hyper_loops[b]
            .sl_ports
            .len()
            .cmp(&holder.hyper_loops[a].sl_ports.len())
    });
    for (position, &hyper) in sorted.iter().enumerate() {
        let (first, second) = TARGETS[position % TARGETS.len()];
        assign_to_target(graph, holder, hyper, first, second);
    }
}

/// `PortSideAssigner.assignToTarget`: the first half of the loop's ports go to the first side, the
/// second half to the second. Corner targets sort the ports by net flow first.
fn assign_to_target(
    graph: &mut LGraph,
    holder: &mut SelfLoopHolder,
    hyper: usize,
    first: PortSide,
    second: PortSide,
) {
    if first != second {
        let mut ports = holder.hyper_loops[hyper].sl_ports.clone();
        {
            let view: &LGraph = &*graph;
            let holder_view: &SelfLoopHolder = &*holder;
            sort_by(&mut ports, |&a, &b| {
                port_net_flow(view, holder_view.sl_ports[a].lport)
                    .cmp(&port_net_flow(view, holder_view.sl_ports[b].lport))
            });
        }
        holder.hyper_loops[hyper].sl_ports = ports;
    }
    let ports = holder.hyper_loops[hyper].sl_ports.clone();
    let half = ports.len() / 2;
    for &port in &ports[..half] {
        if holder.sl_ports[port].hidden {
            set_port_side(graph, lport_of(holder, port), first);
        }
    }
    for &port in &ports[half..] {
        if holder.sl_ports[port].hidden {
            set_port_side(graph, lport_of(holder, port), second);
        }
    }
}

/// `PortRestorer.restorePorts`: collects the hidden ports into areas of their sides, then puts them,
/// with the node's other ports, into the node's port list.
fn restore_node_ports(graph: &mut LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    let mut areas: Vec<Vec<usize>> = vec![Vec::new(); SIDE_COUNT * AREA_COUNT];
    let ordering = graph.nodes[holder.node]
        .properties
        .enum_with_default::<SelfLoopOrderingStrategy>(options::EDGE_ROUTING_SELF_LOOP_ORDERING);
    process_one_side_loops(graph, holder, &mut areas, ordering)?;
    process_two_side_corner_loops(holder, &mut areas)?;
    process_three_side_loops(holder, &mut areas)?;
    process_four_side_loops(holder, &mut areas);
    process_two_side_opposing_loops(holder, &mut areas)?;

    add_ports_to_node(graph, holder, &areas);
    for &port in areas.iter().flatten() {
        holder.sl_ports[port].hidden = false;
    }
    holder.ports_hidden = false;
    Ok(())
}

/// The index of an area in the flat table of areas.
fn area_index(side: PortSide, area: Area) -> usize {
    side as usize * AREA_COUNT + area as usize
}

/// `addToTargetArea`: the hidden ports among `ports` go to the area, prepended or appended. The
/// hidden ports are added in reverse, as the Java does.
fn add_to_target_area(
    holder: &SelfLoopHolder,
    areas: &mut [Vec<usize>],
    ports: &[usize],
    side: PortSide,
    area: Area,
    prepend: bool,
) {
    let mut hidden: Vec<usize> = ports
        .iter()
        .copied()
        .filter(|&port| holder.sl_ports[port].hidden)
        .collect();
    hidden.reverse();
    let target = &mut areas[area_index(side, area)];
    if prepend {
        hidden.append(target);
        *target = hidden;
    } else {
        target.append(&mut hidden);
    }
}

/// `SelfHyperLoop.getSLPortsBySide(side)`: the loop's ports on the side, in port order.
fn side_ports(holder: &SelfLoopHolder, hyper: usize, side: PortSide) -> Vec<usize> {
    holder.hyper_loops[hyper]
        .ports_by_side
        .get(&side)
        .cloned()
        .unwrap_or_default()
}

/// The hyper loops of one type, in their order.
fn loops_of_type(holder: &SelfLoopHolder, loop_type: SelfLoopType) -> Vec<usize> {
    (0..holder.hyper_loops.len())
        .filter(|&hyper| holder.hyper_loops[hyper].loop_type == Some(loop_type))
        .collect()
}

/// `SelfLoopHolder` ports' net flow among the self loops: incoming minus outgoing.
fn sl_net_flow(holder: &SelfLoopHolder, sl_port: usize) -> i64 {
    holder.sl_ports[sl_port].incoming.len() as i64 - holder.sl_ports[sl_port].outgoing.len() as i64
}

/// `PortRestorer.processOneSideLoops`.
fn process_one_side_loops(
    graph: &LGraph,
    holder: &SelfLoopHolder,
    areas: &mut [Vec<usize>],
    ordering: SelfLoopOrderingStrategy,
) -> Result<(), LayoutError> {
    let mut loops = loops_of_type(holder, SelfLoopType::ONE_SIDE);
    if ordering == SelfLoopOrderingStrategy::REVERSE_STACKED {
        loops.reverse();
    }
    for hyper in loops {
        let Some(&first) = holder.hyper_loops[hyper].sl_ports.first() else {
            return Err(LayoutError::Unsupported(
                "a self loop without ports".to_string(),
            ));
        };
        let side = graph.ports[lport_of(holder, first)].side;
        // Ports with more outgoing edges go to the left of those with more incoming edges.
        let mut sorted = holder.hyper_loops[hyper].sl_ports.clone();
        sort_by(&mut sorted, |&a, &b| {
            sl_net_flow(holder, a).cmp(&sl_net_flow(holder, b))
        });
        match ordering {
            SelfLoopOrderingStrategy::SEQUENCED => {
                add_to_target_area(holder, areas, &sorted, side, Area::MIDDLE, false);
            }
            SelfLoopOrderingStrategy::REVERSE_STACKED | SelfLoopOrderingStrategy::STACKED => {
                let split = port_list_split_index(holder, &sorted);
                add_to_target_area(holder, areas, &sorted[..split], side, Area::MIDDLE, true);
                add_to_target_area(holder, areas, &sorted[split..], side, Area::MIDDLE, false);
            }
        }
    }
    Ok(())
}

/// `PortRestorer.computePortListSplitIndex`: the start of the second half of a list sorted by net
/// flow. It is the first port with a positive flow when that is neither the first nor the last port;
/// otherwise the Java's second search, which also looks for a positive flow, and then the centre.
fn port_list_split_index(holder: &SelfLoopHolder, sorted: &[usize]) -> usize {
    let count = sorted.len();
    // The first port with a positive net flow.
    let mut positive = 0;
    while positive < count && sl_net_flow(holder, sorted[positive]) <= 0 {
        positive += 1;
    }
    // Neither the first nor the last port.
    if positive > 0 && positive + 1 < count {
        return positive;
    }
    // The first port with a non-negative net flow, as the Java tests it.
    let mut non_negative = 0;
    while non_negative < count && sl_net_flow(holder, sorted[non_negative]) <= 0 {
        non_negative += 1;
    }
    if non_negative > 0 && positive + 1 < count {
        return non_negative;
    }
    // The centre port.
    count / 2
}

/// `PortRestorer.processTwoSideCornerLoops`.
fn process_two_side_corner_loops(
    holder: &SelfLoopHolder,
    areas: &mut [Vec<usize>],
) -> Result<(), LayoutError> {
    for hyper in loops_of_type(holder, SelfLoopType::TWO_SIDES_CORNER) {
        let [first, second] = sorted_two_side_port_sides(holder, hyper)?;
        add_loop_to_area(holder, areas, hyper, first, Area::END, true);
        add_loop_to_area(holder, areas, hyper, second, Area::START, false);
    }
    Ok(())
}

/// `PortRestorer.processTwoSideOpposingLoops`.
fn process_two_side_opposing_loops(
    holder: &SelfLoopHolder,
    areas: &mut [Vec<usize>],
) -> Result<(), LayoutError> {
    for hyper in loops_of_type(holder, SelfLoopType::TWO_SIDES_OPPOSING) {
        let [first, second] = sorted_two_side_port_sides(holder, hyper)?;
        add_loop_to_area(holder, areas, hyper, first, Area::END, true);
        add_loop_to_area(holder, areas, hyper, second, Area::START, false);
    }
    Ok(())
}

/// `PortRestorer.processThreeSideLoops`.
fn process_three_side_loops(
    holder: &SelfLoopHolder,
    areas: &mut [Vec<usize>],
) -> Result<(), LayoutError> {
    for hyper in loops_of_type(holder, SelfLoopType::THREE_SIDES) {
        let sides = three_side_constellation(holder, hyper)?;
        add_loop_to_area(holder, areas, hyper, sides[0], Area::END, true);
        add_loop_to_area(holder, areas, hyper, sides[1], Area::MIDDLE, false);
        add_loop_to_area(holder, areas, hyper, sides[2], Area::START, false);
    }
    Ok(())
}

/// `PortRestorer.processFourSideLoops`.
fn process_four_side_loops(holder: &SelfLoopHolder, areas: &mut [Vec<usize>]) {
    for hyper in loops_of_type(holder, SelfLoopType::FOUR_SIDES) {
        let sides: Vec<PortSide> = holder.hyper_loops[hyper]
            .ports_by_side
            .keys()
            .copied()
            .collect();
        for side in sides {
            add_loop_to_area(holder, areas, hyper, side, Area::MIDDLE, false);
        }
    }
}

/// `addToTargetArea(slLoop, side, area, mode)`: the loop's ports on the side.
fn add_loop_to_area(
    holder: &SelfLoopHolder,
    areas: &mut [Vec<usize>],
    hyper: usize,
    side: PortSide,
    area: Area,
    prepend: bool,
) {
    let ports = side_ports(holder, hyper, side);
    add_to_target_area(holder, areas, &ports, side, area, prepend);
}

/// `PortRestorer.sortedTwoSideLoopPortSides`: the two sides in clockwise order, with north and west
/// switched, as the Java does.
pub(super) fn sorted_two_side_port_sides(
    holder: &SelfLoopHolder,
    hyper: usize,
) -> Result<[PortSide; 2], LayoutError> {
    let sides: Vec<PortSide> = holder.hyper_loops[hyper]
        .ports_by_side
        .keys()
        .copied()
        .collect();
    match sides.as_slice() {
        [first, second] => {
            if *first == PortSide::NORTH && *second == PortSide::WEST {
                Ok([PortSide::WEST, PortSide::NORTH])
            } else {
                Ok([*first, *second])
            }
        }
        _ => Err(LayoutError::Unsupported(
            "a two-side self loop without two sides".to_string(),
        )),
    }
}

/// `PortRestorer.determineLoopConstellation`: the start, middle and end sides of a three-side loop.
fn three_side_constellation(
    holder: &SelfLoopHolder,
    hyper: usize,
) -> Result<[PortSide; 3], LayoutError> {
    let sides = &holder.hyper_loops[hyper].ports_by_side;
    let has = |side: PortSide| sides.contains_key(&side);
    if !has(PortSide::NORTH) {
        Ok([PortSide::EAST, PortSide::SOUTH, PortSide::WEST])
    } else if !has(PortSide::EAST) {
        Ok([PortSide::SOUTH, PortSide::WEST, PortSide::NORTH])
    } else if !has(PortSide::SOUTH) {
        Ok([PortSide::WEST, PortSide::NORTH, PortSide::EAST])
    } else if !has(PortSide::WEST) {
        Ok([PortSide::NORTH, PortSide::EAST, PortSide::SOUTH])
    } else {
        Err(LayoutError::Unsupported(
            "a three-side self loop covering all four sides".to_string(),
        ))
    }
}

/// `PortRestorer.restorePorts`: the node's port list, with the areas inserted between its regular
/// ports. The regular ports keep their order; each port of a side is taken while its side matches.
fn add_ports_to_node(graph: &mut LGraph, holder: &SelfLoopHolder, areas: &[Vec<usize>]) {
    let node = holder.node;
    let old: Vec<usize> = graph.nodes[node].ports.clone();
    let mut list: Vec<usize> = Vec::with_capacity(old.len() + holder.sl_ports.len());
    let mut next = 0usize;

    push_area(
        graph,
        holder,
        areas,
        PortSide::NORTH,
        Area::START,
        &mut list,
    );
    next = take_ports_that(&old, next, &mut list, |port| {
        graph.ports[port].side == PortSide::NORTH
            && north_south_connection_sides(graph, port).contains(&PortSide::WEST)
    });
    push_area(
        graph,
        holder,
        areas,
        PortSide::NORTH,
        Area::MIDDLE,
        &mut list,
    );
    next = take_side_ports(graph, &old, next, PortSide::NORTH, &mut list);
    push_area(graph, holder, areas, PortSide::NORTH, Area::END, &mut list);

    push_area(graph, holder, areas, PortSide::EAST, Area::START, &mut list);
    push_area(
        graph,
        holder,
        areas,
        PortSide::EAST,
        Area::MIDDLE,
        &mut list,
    );
    next = take_side_ports(graph, &old, next, PortSide::EAST, &mut list);
    push_area(graph, holder, areas, PortSide::EAST, Area::END, &mut list);

    push_area(
        graph,
        holder,
        areas,
        PortSide::SOUTH,
        Area::START,
        &mut list,
    );
    next = take_ports_that(&old, next, &mut list, |port| {
        graph.ports[port].side == PortSide::SOUTH
            && north_south_connection_sides(graph, port).contains(&PortSide::EAST)
    });
    push_area(
        graph,
        holder,
        areas,
        PortSide::SOUTH,
        Area::MIDDLE,
        &mut list,
    );
    next = take_side_ports(graph, &old, next, PortSide::SOUTH, &mut list);
    push_area(graph, holder, areas, PortSide::SOUTH, Area::END, &mut list);

    push_area(graph, holder, areas, PortSide::WEST, Area::START, &mut list);
    take_side_ports(graph, &old, next, PortSide::WEST, &mut list);
    push_area(
        graph,
        holder,
        areas,
        PortSide::WEST,
        Area::MIDDLE,
        &mut list,
    );
    push_area(graph, holder, areas, PortSide::WEST, Area::END, &mut list);

    graph.nodes[node].ports = list;
}

/// `addAll(targetAreas.get(side, area), lNode)`: each port of the area joins the node.
fn push_area(
    graph: &mut LGraph,
    holder: &SelfLoopHolder,
    areas: &[Vec<usize>],
    side: PortSide,
    area: Area,
    list: &mut Vec<usize>,
) {
    for &port in &areas[area_index(side, area)] {
        let lport = lport_of(holder, port);
        graph.ports[lport].node = holder.node;
        list.push(lport);
    }
}

/// `addAllThat`: takes the old ports from `from` while they are on `side`, and returns the index of
/// the first one that is not.
fn take_side_ports(
    graph: &LGraph,
    old: &[usize],
    from: usize,
    side: PortSide,
    list: &mut Vec<usize>,
) -> usize {
    take_ports_that(old, from, list, |port| graph.ports[port].side == side)
}

/// `addAllThat`: takes the old ports from `from` while `condition` holds, and returns the index of
/// the first one for which it does not.
fn take_ports_that(
    old: &[usize],
    from: usize,
    list: &mut Vec<usize>,
    condition: impl Fn(usize) -> bool,
) -> usize {
    let mut index = from;
    while index < old.len() && condition(old[index]) {
        list.push(old[index]);
        index += 1;
    }
    index
}

/// `northSouthPortConnectionSides`: the sides of the port's north/south port dummy whose ports, made
/// for this port, have edges.
fn north_south_connection_sides(graph: &LGraph, port: usize) -> Vec<PortSide> {
    let mut sides = Vec::new();
    let Some(dummy) = graph.ports[port].north_south_dummy else {
        return sides;
    };
    for &dummy_port in &graph.nodes[dummy].ports {
        let data = &graph.ports[dummy_port];
        if data.origin_port == Some(port)
            && (!data.incoming.is_empty() || !data.outgoing.is_empty())
            && !sides.contains(&data.side)
        {
            sides.push(data.side);
        }
    }
    sides
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::loops::model::SelfLoopPort;

    /// A holder whose self loop ports have the given net flows, as incoming minus outgoing edges.
    fn holder_with_flows(flows: &[i64]) -> (SelfLoopHolder, Vec<usize>) {
        let mut holder = SelfLoopHolder::default();
        let ports = flows
            .iter()
            .map(|&flow| {
                let incoming = (0..flow.max(0)).map(|_| 0).collect();
                let outgoing = (0..(-flow).max(0)).map(|_| 0).collect();
                holder.sl_ports.push(SelfLoopPort {
                    incoming,
                    outgoing,
                    ..SelfLoopPort::default()
                });
                holder.sl_ports.len() - 1
            })
            .collect();
        (holder, ports)
    }

    #[test]
    fn the_split_starts_the_second_half_at_the_first_positive_flow() {
        let (holder, ports) = holder_with_flows(&[-2, -1, 0, 2, 3]);
        assert_eq!(port_list_split_index(&holder, &ports), 3);
    }

    #[test]
    fn a_split_of_two_ports_with_opposite_flows_falls_to_the_centre() {
        let (holder, ports) = holder_with_flows(&[-1, 1]);
        assert_eq!(port_list_split_index(&holder, &ports), 1);
    }
}
