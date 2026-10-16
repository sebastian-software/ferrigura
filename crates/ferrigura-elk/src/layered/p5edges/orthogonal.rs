//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/orthogonal/OrthogonalRoutingGenerator.java
//!
//! The orthogonal routing generator: it routes the edges between two adjacent layers. For each
//! east output port of the left layer it makes a hyperedge segment, orders the segments by their
//! dependencies (breaking cycles, and splitting segments where critical cycles remain), numbers
//! them into routing slots, and sets the bend points of their edges.
//!
//! Only the west-to-east direction is ported, the one `OrthogonalEdgeRouter` instantiates. The
//! submodules hold the segments ([`segment`]), the cycle detector ([`cycles`]), the segment
//! splitter ([`splitter`]) and the bend points and junction points ([`direction`]).
//!
//! Java's `HashMap` from ports to segments is a `HashMap` here; it is only looked up, never
//! iterated. The debug output of `routeEdges` is not ported.

pub(in crate::layered) mod cycles;
mod direction;
pub(in crate::layered) mod segment;
mod splitter;

use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::data::enums::PortSide;
use crate::java::random::Random;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;

use segment::{Arena, CRITICAL_DEPENDENCY_WEIGHT, DependencyType, add_port_positions, java_min};

/// `OrthogonalRoutingGenerator.TOLERANCE`: differences below this are zero.
pub const TOLERANCE: f64 = 1e-3;

/// `CRITICAL_CONFLICTS_DETECTED`: what `countConflicts` returns for a critical conflict.
const CRITICAL_CONFLICTS_DETECTED: i32 = -1;

/// `CONFLICT_THRESHOLD_FACTOR`: the conflict threshold as a fraction of the edge spacing.
const CONFLICT_THRESHOLD_FACTOR: f64 = 0.5;
/// `CRITICAL_CONFLICT_THRESHOLD_FACTOR`: the critical threshold as a fraction of the closest
/// distance between two connections.
const CRITICAL_CONFLICT_THRESHOLD_FACTOR: f64 = 0.2;

/// `CONFLICT_PENALTY`: the weight of a non-critical conflict.
const CONFLICT_PENALTY: i32 = 1;
/// `CROSSING_PENALTY`: the weight of a crossing, which is worse than a conflict.
const CROSSING_PENALTY: i32 = 16;

/// `OrthogonalRoutingGenerator`. The edge spacing is fixed; the critical conflict threshold is
/// set for each pair of layers that is routed.
#[derive(Debug)]
pub struct OrthogonalRoutingGenerator {
    /// `edgeSpacing`: the space between edges.
    edge_spacing: f64,
    /// `conflictThreshold`: two horizontal segments closer than this conflict.
    conflict_threshold: f64,
    /// `criticalConflictThreshold`: two horizontal segments closer than this overlap. It depends
    /// on how close the connections of the two layers come to each other.
    critical_conflict_threshold: f64,
}

impl OrthogonalRoutingGenerator {
    /// `new OrthogonalRoutingGenerator(WEST_TO_EAST, edgeSpacing, debugPrefix)`.
    pub fn new(edge_spacing: f64) -> Self {
        OrthogonalRoutingGenerator {
            edge_spacing,
            conflict_threshold: CONFLICT_THRESHOLD_FACTOR * edge_spacing,
            critical_conflict_threshold: 0.0,
        }
    }

    /// `routeEdges`: routes the edges from the nodes of the left layer (`None` if there is none)
    /// to the nodes of the right layer (`None` if there is none). The first routing slot is at
    /// `start_pos`. Returns the number of routing slots.
    pub fn route_edges(
        &mut self,
        graph: &mut LGraph,
        source_layer: Option<&[usize]>,
        target_layer: Option<&[usize]>,
        start_pos: f64,
        random: &mut Option<Random>,
    ) -> Result<i32, LayoutError> {
        let mut arena = Arena::default();
        let mut port_to_segment: HashMap<usize, usize> = HashMap::new();
        let mut edge_segments: Vec<usize> = Vec::new();

        // Segments for the east output ports of the left layer, and for the west output ports of
        // the right layer.
        create_hyper_edge_segments(
            &mut arena,
            graph,
            source_layer,
            direction::SOURCE_PORT_SIDE,
            &mut edge_segments,
            &mut port_to_segment,
        );
        create_hyper_edge_segments(
            &mut arena,
            graph,
            target_layer,
            direction::TARGET_PORT_SIDE,
            &mut edge_segments,
            &mut port_to_segment,
        );

        // The critical threshold is a fraction of the smallest distance between two horizontal
        // segments.
        self.critical_conflict_threshold = CRITICAL_CONFLICT_THRESHOLD_FACTOR
            * minimum_horizontal_segment_distance(&arena, &edge_segments);

        // The dependencies between the segments, counting the critical ones.
        let mut critical_dependency_count = 0;
        for (index, &first) in edge_segments.iter().enumerate() {
            for &second in &edge_segments[index + 1..] {
                critical_dependency_count +=
                    self.create_dependency_if_necessary(&mut arena, first, second);
            }
        }

        // Two or more critical dependencies may form critical cycles, which need splitting.
        if critical_dependency_count >= 2 {
            self.break_critical_cycles(&mut arena, &mut edge_segments, random)?;
        }

        break_non_critical_cycles(&mut arena, &edge_segments, random)?;

        topological_numbering(&mut arena, &edge_segments);

        // Set the bend points with the coordinates of the routing slots.
        let mut rank_count = -1;
        let mut created_junctions: HashSet<direction::JunctionKey> = HashSet::new();
        for &segment in &edge_segments {
            // Straight edges take no slot and need no bend points.
            let data = &arena.segments[segment];
            if (data.start - data.end).abs() < TOLERANCE {
                continue;
            }

            rank_count = rank_count.max(data.routing_slot);

            direction::calculate_bend_points(
                graph,
                &arena,
                segment,
                start_pos,
                self.edge_spacing,
                &mut created_junctions,
            );
        }

        Ok(rank_count + 1)
    }

    /// `breakCriticalCycles`: finds the critical cycles and splits the segments on them.
    fn break_critical_cycles(
        &self,
        arena: &mut Arena,
        segments: &mut Vec<usize>,
        random: &mut Option<Random>,
    ) -> Result<(), LayoutError> {
        let cycle_dependencies = cycles::detect_cycles(arena, segments, true, random)?;
        splitter::split_segments(
            self,
            arena,
            &cycle_dependencies,
            segments,
            self.critical_conflict_threshold,
        );
        Ok(())
    }

    /// `createDependencyIfNecessary`: adds the dependencies that two segments need, if any.
    /// Returns the number of critical dependencies it added. The splitter calls it as well.
    fn create_dependency_if_necessary(&self, arena: &mut Arena, he1: usize, he2: usize) -> i32 {
        // A straight segment takes no slot, so it creates no dependency.
        let (he1_start, he1_end) = (arena.segments[he1].start, arena.segments[he1].end);
        let (he2_start, he2_end) = (arena.segments[he2].start, arena.segments[he2].end);
        if (he1_start - he1_end).abs() < TOLERANCE || (he2_start - he2_end).abs() < TOLERANCE {
            return 0;
        }

        // Compare the number of conflicts of both orders.
        let conflicts1 =
            self.count_conflicts(&arena.segments[he1].outgoing, &arena.segments[he2].incoming);
        let conflicts2 =
            self.count_conflicts(&arena.segments[he2].outgoing, &arena.segments[he1].incoming);

        let critical_conflicts_detected =
            conflicts1 == CRITICAL_CONFLICTS_DETECTED || conflicts2 == CRITICAL_CONFLICTS_DETECTED;
        let mut critical_dependency_count = 0;

        if critical_conflicts_detected {
            // Add the critical dependencies that the conflicts require.
            if conflicts1 == CRITICAL_CONFLICTS_DETECTED {
                // he1 must not be left of he2, since that would cause critical conflicts.
                arena.add_dependency(
                    DependencyType::Critical,
                    he2,
                    he1,
                    CRITICAL_DEPENDENCY_WEIGHT,
                );
                critical_dependency_count += 1;
            }

            if conflicts2 == CRITICAL_CONFLICTS_DETECTED {
                // he2 must not be left of he1, since that would cause critical conflicts.
                arena.add_dependency(
                    DependencyType::Critical,
                    he1,
                    he2,
                    CRITICAL_DEPENDENCY_WEIGHT,
                );
                critical_dependency_count += 1;
            }
        } else {
            // No critical conflicts: count the crossings of both orders.
            let mut crossings1 =
                Self::count_crossings(&arena.segments[he1].outgoing, he2_start, he2_end);
            crossings1 += Self::count_crossings(&arena.segments[he2].incoming, he1_start, he1_end);
            let mut crossings2 =
                Self::count_crossings(&arena.segments[he2].outgoing, he1_start, he1_end);
            crossings2 += Self::count_crossings(&arena.segments[he1].incoming, he2_start, he2_end);

            // Crossings are worse than conflicts.
            let dep_value1 = CONFLICT_PENALTY * conflicts1 + CROSSING_PENALTY * crossings1;
            let dep_value2 = CONFLICT_PENALTY * conflicts2 + CROSSING_PENALTY * crossings2;

            if dep_value1 < dep_value2 {
                // he1 wants to be left of he2.
                arena.add_dependency(DependencyType::Regular, he1, he2, dep_value2 - dep_value1);
            } else if dep_value1 > dep_value2 {
                // he2 wants to be left of he1.
                arena.add_dependency(DependencyType::Regular, he2, he1, dep_value1 - dep_value2);
            } else if dep_value1 > 0 && dep_value2 > 0 {
                // Two dependencies of zero weight.
                arena.add_dependency(DependencyType::Regular, he1, he2, 0);
                arena.add_dependency(DependencyType::Regular, he2, he1, 0);
            }
        }

        critical_dependency_count
    }

    /// `countConflicts`: the number of positions that conflict, or `CRITICAL_CONFLICTS_DETECTED`
    /// as soon as one critical conflict is found. Both lists are sorted.
    fn count_conflicts(&self, posis1: &[f64], posis2: &[f64]) -> i32 {
        let mut conflicts = 0;

        if !posis1.is_empty() && !posis2.is_empty() {
            let mut index1 = 0;
            let mut index2 = 0;
            let mut pos1 = posis1[0];
            let mut pos2 = posis2[0];

            loop {
                if pos1 > pos2 - self.critical_conflict_threshold
                    && pos1 < pos2 + self.critical_conflict_threshold
                {
                    // One critical conflict is enough.
                    return CRITICAL_CONFLICTS_DETECTED;
                } else if pos1 > pos2 - self.conflict_threshold
                    && pos1 < pos2 + self.conflict_threshold
                {
                    conflicts += 1;
                }

                if pos1 <= pos2 && index1 + 1 < posis1.len() {
                    index1 += 1;
                    pos1 = posis1[index1];
                } else if pos2 <= pos1 && index2 + 1 < posis2.len() {
                    index2 += 1;
                    pos2 = posis2[index2];
                } else {
                    break;
                }
            }
        }

        conflicts
    }

    /// `countCrossings`: the number of sorted positions in `[start, end]`. Stops at the first
    /// position past `end`.
    fn count_crossings(positions: &[f64], start: f64, end: f64) -> i32 {
        let mut crossings = 0;
        for &position in positions {
            if position > end {
                break;
            } else if position >= start {
                crossings += 1;
            }
        }
        crossings
    }
}

/// `createHyperEdgeSegments`: a segment for each port on `side` with outgoing edges of the layer's
/// nodes, unless the port already has one. The segment takes the ports connected to it.
fn create_hyper_edge_segments(
    arena: &mut Arena,
    graph: &LGraph,
    nodes: Option<&[usize]>,
    side: PortSide,
    edge_segments: &mut Vec<usize>,
    port_to_segment: &mut HashMap<usize, usize>,
) {
    let Some(nodes) = nodes else { return };
    for &node in nodes {
        for &port in &graph.nodes[node].ports {
            // `node.getPorts(PortType.OUTPUT, side)`: the ports on the side with outgoing edges.
            if graph.ports[port].side != side || graph.ports[port].outgoing.is_empty() {
                continue;
            }
            if !port_to_segment.contains_key(&port) {
                let segment = arena.new_segment();
                edge_segments.push(segment);
                add_port_positions(arena, graph, segment, port, port_to_segment);
            }
        }
    }
}

/// `minimumHorizontalSegmentDistance`: the smallest distance between two adjacent incoming
/// connections, or two adjacent outgoing connections, of any segments.
fn minimum_horizontal_segment_distance(arena: &Arena, segments: &[usize]) -> f64 {
    let incoming: Vec<f64> = segments
        .iter()
        .flat_map(|&segment| arena.segments[segment].incoming.iter().copied())
        .collect();
    let outgoing: Vec<f64> = segments
        .iter()
        .flat_map(|&segment| arena.segments[segment].outgoing.iter().copied())
        .collect();
    java_min(minimum_difference(incoming), minimum_difference(outgoing))
}

/// `minimumDifference`: the smallest difference between two distinct numbers, or `f64::MAX` if
/// there are fewer than two.
fn minimum_difference(mut numbers: Vec<f64>) -> f64 {
    // `sorted().distinct()`: sorted by `Double.compareTo`, distinct by `Double.equals`.
    numbers.sort_by(f64::total_cmp);
    numbers.dedup_by(|a, b| a.to_bits() == b.to_bits());

    let mut min_difference = f64::MAX;
    for pair in numbers.windows(2) {
        min_difference = java_min(min_difference, pair[1] - pair[0]);
    }
    min_difference
}

/// `breakNonCriticalCycles`: removes the zero-weight dependencies on cycles and reverses the
/// others.
pub(in crate::layered) fn break_non_critical_cycles(
    arena: &mut Arena,
    segments: &[usize],
    random: &mut Option<Random>,
) -> Result<(), LayoutError> {
    let cycle_dependencies = cycles::detect_cycles(arena, segments, false, random)?;
    for dependency in cycle_dependencies {
        if arena.dependencies[dependency].weight == 0 {
            // Only two-cycles have weight zero, so removing one breaks the cycle.
            arena.remove_dependency(dependency);
        } else {
            arena.reverse_dependency(dependency);
        }
    }
    Ok(())
}

/// `topologicalNumbering`: assigns each segment its routing slot, the longest path from a source.
/// Segments that only point right are moved as far right as possible, so that back edges do not
/// end up far from their targets.
fn topological_numbering(arena: &mut Arena, segments: &[usize]) {
    // Sources have no incoming dependencies. Rightward targets have no outgoing dependencies and
    // no incoming connections: they only connect to western ports.
    let mut sources: VecDeque<usize> = VecDeque::new();
    let mut rightward_targets: VecDeque<usize> = VecDeque::new();
    for &node in segments {
        let data = &mut arena.segments[node];
        data.in_weight = data.incoming_deps.len() as i32;
        data.out_weight = data.outgoing_deps.len() as i32;

        if data.in_weight == 0 {
            sources.push_back(node);
        }
        if data.out_weight == 0 && data.incoming.is_empty() {
            rightward_targets.push_back(node);
        }
    }

    let mut max_rank = -1;

    // Assign the ranks with a topological numbering.
    while let Some(node) = sources.pop_front() {
        let node_slot = arena.segments[node].routing_slot;
        for dependency in arena.segments[node].outgoing_deps.clone() {
            let Some(target) = arena.dependencies[dependency].target else {
                continue;
            };
            let target_data = &mut arena.segments[target];
            target_data.routing_slot = target_data.routing_slot.max(node_slot + 1);
            max_rank = max_rank.max(target_data.routing_slot);

            target_data.in_weight -= 1;
            if target_data.in_weight == 0 {
                sources.push_back(target);
            }
        }
    }

    // Segments whose horizontal segments all point right move to the rightmost rank, and those
    // that connect only to them move as far right as possible.
    if max_rank > -1 {
        for &node in &rightward_targets {
            arena.segments[node].routing_slot = max_rank;
        }

        while let Some(node) = rightward_targets.pop_front() {
            // The node only has connections to western ports.
            for dependency in arena.segments[node].incoming_deps.clone() {
                let Some(source) = arena.dependencies[dependency].source else {
                    continue;
                };
                if !arena.segments[source].incoming.is_empty() {
                    continue;
                }

                let node_slot = arena.segments[node].routing_slot;
                let source_data = &mut arena.segments[source];
                source_data.routing_slot = source_data.routing_slot.min(node_slot - 1);

                source_data.out_weight -= 1;
                if source_data.out_weight == 0 {
                    rightward_targets.push_back(source);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_count_the_positions_inside_the_range() {
        assert_eq!(
            OrthogonalRoutingGenerator::count_crossings(&[1.0, 5.0, 9.0], 4.0, 10.0),
            2
        );
        assert_eq!(
            OrthogonalRoutingGenerator::count_crossings(&[1.0, 5.0, 9.0], 0.0, 4.0),
            1
        );
        assert_eq!(
            OrthogonalRoutingGenerator::count_crossings(&[], 0.0, 4.0),
            0
        );
    }

    #[test]
    fn conflicts_are_critical_within_the_critical_threshold() {
        let generator = OrthogonalRoutingGenerator {
            edge_spacing: 5.0,
            conflict_threshold: 2.5,
            critical_conflict_threshold: 2.0,
        };
        assert_eq!(
            generator.count_conflicts(&[10.0], &[11.0]),
            CRITICAL_CONFLICTS_DETECTED
        );
        assert_eq!(generator.count_conflicts(&[10.0], &[13.0]), 0);
        assert_eq!(generator.count_conflicts(&[10.0], &[12.0]), 1);
    }

    #[test]
    fn minimum_difference_is_over_distinct_values() {
        assert_eq!(minimum_difference(vec![3.0, 1.0, 3.0, 10.0]), 2.0);
        assert_eq!(minimum_difference(vec![4.0]), f64::MAX);
    }
}
