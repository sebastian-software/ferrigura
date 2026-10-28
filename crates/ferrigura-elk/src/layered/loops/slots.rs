//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/loops/routing/RoutingSlotAssigner.java
//!
//! The routing slots of the loops: each loop's trunk gets a slot on each side it occupies, so that
//! trunks do not cross, and labels that overlap get different slots. The crossing graph is built on
//! the hyperedge segments of the orthogonal router, and its cycles are broken by the same code.

use std::collections::{HashSet, VecDeque};

use crate::core::data::enums::PortSide;
use crate::java::sort::sort_by;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::loops::labels::SelfHyperLoopLabels;
use crate::layered::loops::model::SelfLoopHolder;
use crate::layered::p5edges::orthogonal::break_non_critical_cycles;
use crate::layered::p5edges::orthogonal::segment::{Arena, DependencyType};

/// The sides in the order the shifts visit them, as `shiftTowardsNode` does.
const SHIFT_ORDER: [PortSide; 4] = [
    PortSide::NORTH,
    PortSide::EAST,
    PortSide::SOUTH,
    PortSide::WEST,
];

/// What the crossing counts read: the graph's port ids, each loop's activity over the ports, and
/// which pairs of labels overlap.
struct CrossingContext<'a> {
    graph: &'a LGraph,
    holder: &'a SelfLoopHolder,
    /// `slLoopActivityOverPorts`: per loop, whether it is active at each port id.
    activity: &'a [Vec<bool>],
    /// `labelCrossingMatrix`, indexed by label ids.
    label_matrix: &'a [Vec<bool>],
}

/// `RoutingSlotAssigner.assignRoutingSlots`.
pub fn assign_routing_slots(
    graph: &mut LGraph,
    holder: &mut SelfLoopHolder,
) -> Result<(), LayoutError> {
    let label_matrix = label_crossing_matrix(holder);
    let activity = loop_activity(graph, holder)?;

    // The crossing graph: one segment per loop, and a dependency for each pair of loops.
    let mut arena = Arena::default();
    let segments: Vec<usize> = (0..holder.hyper_loops.len())
        .map(|_| arena.new_segment())
        .collect();
    {
        let context = CrossingContext {
            graph: &*graph,
            holder: &*holder,
            activity: &activity,
            label_matrix: &label_matrix,
        };
        for first in 0..segments.len() {
            for second in first + 1..segments.len() {
                create_dependencies(&context, &mut arena, &segments, first, second);
            }
        }
    }

    // Break the cycles that do not matter for the crossings, as the orthogonal router does.
    let mut random = graph.random.take();
    let broken = break_non_critical_cycles(&mut arena, &segments, &mut random);
    graph.random = random;
    broken?;

    assign_raw_routing_slots_to_segments(&mut arena, &segments);
    assign_raw_routing_slots_to_loops(holder, &arena, &segments)?;
    shift_towards_node(graph, holder, &activity, &label_matrix);
    Ok(())
}

/// `computeLabelCrossingMatrix`: gives the labels their ids, and marks the pairs that overlap.
fn label_crossing_matrix(holder: &mut SelfLoopHolder) -> Vec<Vec<bool>> {
    let mut count = 0;
    for hyper in &mut holder.hyper_loops {
        if let Some(labels) = hyper.labels.as_mut() {
            labels.id = count;
            count += 1;
        }
    }
    let mut matrix = vec![vec![false; count]; count];
    let loops = &holder.hyper_loops;
    for first in 0..loops.len() {
        for second in first + 1..loops.len() {
            let (Some(first_labels), Some(second_labels)) =
                (loops[first].labels.as_ref(), loops[second].labels.as_ref())
            else {
                continue;
            };
            let overlap = labels_overlap(first_labels, second_labels);
            matrix[first_labels.id][second_labels.id] = overlap;
            matrix[second_labels.id][first_labels.id] = overlap;
        }
    }
    matrix
}

/// `labelsOverlap` of two loops' labels: on the same north or south side, with overlapping
/// horizontal extents. The positions are set by the label placer.
fn labels_overlap(first: &SelfHyperLoopLabels, second: &SelfHyperLoopLabels) -> bool {
    if first.side != second.side
        || first.side == Some(PortSide::EAST)
        || first.side == Some(PortSide::WEST)
    {
        return false;
    }
    let start1 = first.position.x;
    let end1 = start1 + first.size.x;
    let start2 = second.position.x;
    let end2 = start2 + second.size.x;
    start1 <= end2 && end1 >= start2
}

/// `createCrossingGraph`: for each loop, the activity over the node's ports. Each loop is active from
/// its leftmost port to its rightmost port, wrapping around the node's port list if needed.
fn loop_activity(graph: &LGraph, holder: &SelfLoopHolder) -> Result<Vec<Vec<bool>>, LayoutError> {
    let port_count = graph.nodes[holder.node].ports.len();
    if port_count == 0 {
        return Err(LayoutError::Unsupported(
            "a self loop node without ports".to_string(),
        ));
    }
    let mut activity = Vec::with_capacity(holder.hyper_loops.len());
    for hyper in &holder.hyper_loops {
        let (Some(leftmost), Some(rightmost)) = (hyper.leftmost, hyper.rightmost) else {
            return Err(LayoutError::Unsupported(
                "a self loop without its leftmost and rightmost ports".to_string(),
            ));
        };
        let mut loop_activity = vec![false; port_count];
        // Run from the port before the leftmost one to the rightmost port, marking each port passed.
        let target = graph.ports[holder.sl_ports[rightmost].lport].id as i64;
        let mut index = graph.ports[holder.sl_ports[leftmost].lport].id as i64 - 1;
        while index != target {
            index = (index + 1).rem_euclid(port_count as i64);
            loop_activity[index as usize] = true;
        }
        activity.push(loop_activity);
    }
    Ok(activity)
}

/// `createDependencies`: the segments of two loops are ordered by which of them crosses fewer of the
/// other's ports when placed above it. Loops with equal crossings but overlapping labels get different
/// slots.
fn create_dependencies(
    context: &CrossingContext,
    arena: &mut Arena,
    segments: &[usize],
    first: usize,
    second: usize,
) {
    let first_above = count_crossings(context, first, second);
    let second_above = count_crossings(context, second, first);
    let (first_segment, second_segment) = (segments[first], segments[second]);
    if first_above < second_above {
        arena.add_dependency(
            DependencyType::Regular,
            first_segment,
            second_segment,
            second_above - first_above,
        );
    } else if second_above < first_above {
        arena.add_dependency(
            DependencyType::Regular,
            second_segment,
            first_segment,
            first_above - second_above,
        );
    } else if first_above != 0 || labels_overlap_in_matrix(context, first, second) {
        arena.add_dependency(DependencyType::Regular, first_segment, second_segment, 0);
        arena.add_dependency(DependencyType::Regular, second_segment, first_segment, 0);
    }
}

/// `countCrossings`: how many ports of the upper loop lie inside the lower loop's activity.
fn count_crossings(context: &CrossingContext, upper: usize, lower: usize) -> i32 {
    let lower_activity = &context.activity[lower];
    context.holder.hyper_loops[upper]
        .sl_ports
        .iter()
        .filter(|&&sl_port| {
            let id = context.graph.ports[context.holder.sl_ports[sl_port].lport].id;
            lower_activity[id]
        })
        .count() as i32
}

/// `labelsOverlap` of two loops, from the matrix; loops without labels never overlap.
fn labels_overlap_in_matrix(context: &CrossingContext, first: usize, second: usize) -> bool {
    match (
        &context.holder.hyper_loops[first].labels,
        &context.holder.hyper_loops[second].labels,
    ) {
        (Some(first_labels), Some(second_labels)) => {
            context.label_matrix[first_labels.id][second_labels.id]
        }
        _ => false,
    }
}

/// `assignRawRoutingSlotsToSegments`: a segment's slot is one more than the slots of the segments
/// that must lie beneath it, found by walking the dependencies from the sinks.
fn assign_raw_routing_slots_to_segments(arena: &mut Arena, segments: &[usize]) {
    let mut sinks = VecDeque::new();
    for &segment in segments {
        let data = &mut arena.segments[segment];
        data.in_weight = data.incoming_deps.len() as i32;
        data.out_weight = data.outgoing_deps.len() as i32;
        if data.out_weight == 0 {
            data.routing_slot = 0;
            sinks.push_back(segment);
        }
    }
    while let Some(segment) = sinks.pop_front() {
        let next_routing_slot = arena.segments[segment].routing_slot + 1;
        for dependency in arena.segments[segment].incoming_deps.clone() {
            let Some(source) = arena.dependencies[dependency].source else {
                continue;
            };
            let source_data = &mut arena.segments[source];
            source_data.routing_slot = source_data.routing_slot.max(next_routing_slot);
            source_data.out_weight -= 1;
            if source_data.out_weight == 0 {
                sinks.push_back(source);
            }
        }
    }
}

/// `assignRawRoutingSlotsToLoops`: each loop takes its segment's slot on every side it occupies.
fn assign_raw_routing_slots_to_loops(
    holder: &mut SelfLoopHolder,
    arena: &Arena,
    segments: &[usize],
) -> Result<(), LayoutError> {
    for (hyper, &segment) in segments.iter().enumerate() {
        let slot = usize::try_from(arena.segments[segment].routing_slot)
            .map_err(|_| LayoutError::Unsupported("a negative routing slot".to_string()))?;
        let sides: Vec<PortSide> = holder.hyper_loops[hyper]
            .occupied_sides
            .iter()
            .copied()
            .collect();
        for side in sides {
            set_routing_slot(holder, hyper, side, slot);
        }
    }
    Ok(())
}

/// `SelfHyperLoop.setRoutingSlot`: the loop's slot on a side, and the count of slots on the side.
fn set_routing_slot(holder: &mut SelfLoopHolder, hyper: usize, side: PortSide, slot: usize) {
    holder.hyper_loops[hyper].routing_slot[side as usize] = slot;
    let count = &mut holder.routing_slot_count[side as usize];
    *count = (*count).max(slot + 1);
}

/// `shiftTowardsNode`: on each side, the loops take the lowest slot free across their ports, and a
/// label clashing with another label's slot moves on.
fn shift_towards_node(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    activity: &[Vec<bool>],
    label_matrix: &[Vec<bool>],
) {
    // For each port, the next routing slot a loop on the port's side may take.
    let mut next_free_slot = vec![0usize; graph.nodes[holder.node].ports.len()];
    for side in SHIFT_ORDER {
        shift_towards_node_on_side(
            graph,
            holder,
            side,
            &mut next_free_slot,
            activity,
            label_matrix,
        );
    }
}

/// `shiftTowardsNodeOnSide`.
fn shift_towards_node_on_side(
    graph: &LGraph,
    holder: &mut SelfLoopHolder,
    side: PortSide,
    next_free_slot: &mut [usize],
    activity: &[Vec<bool>],
    label_matrix: &[Vec<bool>],
) {
    // The loops on the side, by their slot there.
    let mut loops: Vec<usize> = (0..holder.hyper_loops.len())
        .filter(|&hyper| holder.hyper_loops[hyper].occupied_sides.contains(&side))
        .collect();
    sort_by(&mut loops, |&a, &b| {
        holder.hyper_loops[a].routing_slot[side as usize]
            .cmp(&holder.hyper_loops[b].routing_slot[side as usize])
    });

    // The first and last port ids on the side.
    let mut range: Option<(usize, usize)> = None;
    for &lport in &graph.nodes[holder.node].ports {
        let port = &graph.ports[lport];
        if port.side == side {
            range = Some(match range {
                None => (port.id, port.id),
                Some((low, high)) => (low.min(port.id), high.max(port.id)),
            });
        }
    }

    let Some((min_port, max_port)) = range else {
        // No ports on this side: the loops take consecutive slots from zero, and no label clashes.
        for (slot, &hyper) in loops.iter().enumerate() {
            set_routing_slot(holder, hyper, side, slot);
        }
        return;
    };

    let mut slot_of_label: Vec<i64> = vec![-1; label_matrix.len()];
    for &hyper in &loops {
        let active = &activity[hyper];
        // The lowest slot free on every port the loop is active at.
        let mut lowest = 0usize;
        for port in min_port..=max_port {
            if active[port] {
                lowest = lowest.max(next_free_slot[port]);
            }
        }
        // A label must not share a slot with a label that overlaps it.
        if let Some(label) = holder.hyper_loops[hyper]
            .labels
            .as_ref()
            .map(|labels| labels.id)
        {
            let conflicts: HashSet<i64> = (0..label_matrix.len())
                .filter(|&other| label_matrix[label][other])
                .map(|other| slot_of_label[other])
                .collect();
            while conflicts.contains(&(lowest as i64)) {
                lowest += 1;
            }
        }
        set_routing_slot(holder, hyper, side, lowest);
        for port in min_port..=max_port {
            if active[port] {
                next_free_slot[port] = lowest + 1;
            }
        }
        if let Some(label) = holder.hyper_loops[hyper]
            .labels
            .as_ref()
            .map(|labels| labels.id)
        {
            slot_of_label[label] = lowest as i64;
        }
    }
}
