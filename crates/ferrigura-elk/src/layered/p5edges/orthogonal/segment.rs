//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/orthogonal/HyperEdgeSegment.java
//!   with HyperEdgeSegmentDependency.java
//!
//! The trunks of hyperedges, `HyperEdgeSegment`, and the dependencies between them,
//! `HyperEdgeSegmentDependency`. A segment is the vertical trunk of a hyperedge between two
//! layers. The connections on either side are the y coordinates where edges enter and leave it.
//!
//! Java objects are indices here. [`Arena`] holds the segments and dependencies of one
//! `routeEdges` call, and they refer to each other by index. The Java `LinkedList`s of connection
//! coordinates are `Vec`s, kept sorted as Java keeps them. Java compares segments by `mark` in
//! `equals`, and by identity with `!=`; this port compares indices, which is the identity.
//!
//! Java `float` does not occur in the arithmetic here. GWT, which compiled elkjs, treats `float`
//! as `double`, so `Double.floatValue()` in `insertSorted` changes nothing and the sorted
//! insertion compares doubles.

use std::collections::HashMap;

use crate::layered::lgraph::LGraph;

use super::direction;

/// `HyperEdgeSegmentDependency.CRITICAL_DEPENDENCY_WEIGHT`.
pub const CRITICAL_DEPENDENCY_WEIGHT: i32 = 1;

/// `HyperEdgeSegmentDependency.DependencyType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Ignoring it costs the weight in additional crossings.
    Regular,
    /// Ignoring it causes edge overlaps.
    Critical,
}

/// A dependency between two segments: the source wants a lower routing slot than the target.
/// `source` and `target` are `None` once the dependency is removed.
#[derive(Debug, Clone, Copy)]
pub struct Dependency {
    /// `type`
    pub kind: DependencyType,
    /// `source`: an index into [`Arena::segments`].
    pub source: Option<usize>,
    /// `target`: an index into [`Arena::segments`].
    pub target: Option<usize>,
    /// `weight`
    pub weight: i32,
}

/// A hyperedge segment, `HyperEdgeSegment`.
#[derive(Debug, Clone)]
pub struct Segment {
    /// `mark`: the value the cycle detector orders the segments by.
    pub mark: i32,
    /// `ports`: indices into `LGraph::ports` of the ports the segment represents, in the order
    /// they were added.
    pub ports: Vec<usize>,
    /// `routingSlot`: the horizontal distance to the preceding layer, in slots.
    pub routing_slot: i32,
    /// `startPosition`: NaN until a connection is added.
    pub start: f64,
    /// `endPosition`: NaN until a connection is added.
    pub end: f64,
    /// `incomingConnectionCoordinates`: sorted, where incoming connections enter the segment.
    pub incoming: Vec<f64>,
    /// `outgoingConnectionCoordinates`: sorted, where outgoing connections leave the segment.
    pub outgoing: Vec<f64>,
    /// `outgoingSegmentDependencies`: indices into [`Arena::dependencies`], in list order.
    pub outgoing_deps: Vec<usize>,
    /// `outDepWeight`
    pub out_weight: i32,
    /// `criticalOutDepWeight`
    pub critical_out_weight: i32,
    /// `incomingSegmentDependencies`: indices into [`Arena::dependencies`], in list order.
    pub incoming_deps: Vec<usize>,
    /// `inDepWeight`
    pub in_weight: i32,
    /// `criticalInDepWeight`
    pub critical_in_weight: i32,
    /// `splitPartner`: the other segment of a split, if any.
    pub split_partner: Option<usize>,
    /// `splitBy`: the segment that caused this one to be split. Set on one split partner only.
    pub split_by: Option<usize>,
}

impl Default for Segment {
    /// `new HyperEdgeSegment(strategy)`: no ports, no connections, NaN extent.
    fn default() -> Self {
        Segment {
            mark: 0,
            ports: Vec::new(),
            routing_slot: 0,
            start: f64::NAN,
            end: f64::NAN,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            outgoing_deps: Vec::new(),
            out_weight: 0,
            critical_out_weight: 0,
            incoming_deps: Vec::new(),
            in_weight: 0,
            critical_in_weight: 0,
            split_partner: None,
            split_by: None,
        }
    }
}

/// The segments and dependencies of one `routeEdges` call.
#[derive(Debug, Default)]
pub struct Arena {
    /// Every segment made during the call, including the ones made to simulate splits.
    pub segments: Vec<Segment>,
    /// Every dependency made during the call, including the removed ones.
    pub dependencies: Vec<Dependency>,
}

impl Arena {
    /// `new HyperEdgeSegment(...)`: returns its index.
    pub fn new_segment(&mut self) -> usize {
        self.segments.push(Segment::default());
        self.segments.len() - 1
    }

    /// `new HyperEdgeSegmentDependency(type, source, target, weight)`: the dependency is added to
    /// the end of the source's outgoing list and of the target's incoming list. Returns its index.
    pub fn add_dependency(
        &mut self,
        kind: DependencyType,
        source: usize,
        target: usize,
        weight: i32,
    ) -> usize {
        self.dependencies.push(Dependency {
            kind,
            source: None,
            target: None,
            weight,
        });
        let dependency = self.dependencies.len() - 1;
        self.set_source(dependency, Some(source));
        self.set_target(dependency, Some(target));
        dependency
    }

    /// `HyperEdgeSegmentDependency.remove()`.
    pub fn remove_dependency(&mut self, dependency: usize) {
        self.set_source(dependency, None);
        self.set_target(dependency, None);
    }

    /// `HyperEdgeSegmentDependency.reverse()`.
    pub fn reverse_dependency(&mut self, dependency: usize) {
        let old_source = self.dependencies[dependency].source;
        let old_target = self.dependencies[dependency].target;
        self.set_source(dependency, old_target);
        self.set_target(dependency, old_source);
    }

    /// `HyperEdgeSegmentDependency.setSource`: leaves the old source's list and joins the new one's.
    fn set_source(&mut self, dependency: usize, new_source: Option<usize>) {
        if let Some(old) = self.dependencies[dependency].source {
            remove_first(&mut self.segments[old].outgoing_deps, dependency);
        }
        self.dependencies[dependency].source = new_source;
        if let Some(new) = new_source {
            self.segments[new].outgoing_deps.push(dependency);
        }
    }

    /// `HyperEdgeSegmentDependency.setTarget`: leaves the old target's list and joins the new one's.
    fn set_target(&mut self, dependency: usize, new_target: Option<usize>) {
        if let Some(old) = self.dependencies[dependency].target {
            remove_first(&mut self.segments[old].incoming_deps, dependency);
        }
        self.dependencies[dependency].target = new_target;
        if let Some(new) = new_target {
            self.segments[new].incoming_deps.push(dependency);
        }
    }

    /// `HyperEdgeSegment.simulateSplit()`: two detached segments with the connections a split
    /// would give them, and no dependencies. Their extents are computed once, as in Java, so the
    /// caller's later changes to their connections do not move the extents.
    pub fn simulate_split(&mut self, segment: usize) -> (usize, usize) {
        let split = self.new_segment();
        let partner = self.new_segment();

        let incoming = self.segments[segment].incoming.clone();
        let split_by = self.segments[segment].split_by;
        self.segments[split].incoming = incoming;
        self.segments[split].split_by = split_by;
        self.segments[split].split_partner = Some(partner);
        recompute_extent(&mut self.segments[split]);

        let outgoing = self.segments[segment].outgoing.clone();
        self.segments[partner].outgoing = outgoing;
        self.segments[partner].split_partner = Some(split);
        recompute_extent(&mut self.segments[partner]);

        (split, partner)
    }

    /// `HyperEdgeSegment.splitAt(splitPosition)`: the segment keeps its incoming connections and
    /// links to a new partner at `split_position`, which takes the outgoing connections. Both
    /// lose all their dependencies. Returns the new partner.
    pub fn split_at(&mut self, segment: usize, split_position: f64) -> usize {
        let partner = self.new_segment();
        self.segments[segment].split_partner = Some(partner);
        self.segments[partner].split_partner = Some(segment);

        let moved = std::mem::take(&mut self.segments[segment].outgoing);
        self.segments[partner].outgoing = moved;
        self.segments[segment].outgoing.push(split_position);
        self.segments[partner].incoming.push(split_position);

        recompute_extent(&mut self.segments[segment]);
        recompute_extent(&mut self.segments[partner]);

        while let Some(&dependency) = self.segments[segment].incoming_deps.first() {
            self.remove_dependency(dependency);
        }
        while let Some(&dependency) = self.segments[segment].outgoing_deps.first() {
            self.remove_dependency(dependency);
        }
        partner
    }

    /// `HyperEdgeSegment.representsHyperedge()`: more than two connections in all.
    pub fn represents_hyperedge(&self, segment: usize) -> bool {
        let segment = &self.segments[segment];
        segment.incoming.len() + segment.outgoing.len() > 2
    }

    /// `HyperEdgeSegment.isDummy()`: a split partner that was not the segment that caused the split.
    pub fn is_dummy(&self, segment: usize) -> bool {
        let segment = &self.segments[segment];
        segment.split_partner.is_some() && segment.split_by.is_none()
    }

    /// `HyperEdgeSegment.getLength()`.
    pub fn length(&self, segment: usize) -> f64 {
        let segment = &self.segments[segment];
        segment.end - segment.start
    }
}

/// `ArrayList.remove(Object)`: the first occurrence goes, and the rest keep their order.
fn remove_first(list: &mut Vec<usize>, value: usize) {
    if let Some(index) = list.iter().position(|&found| found == value) {
        list.remove(index);
    }
}

/// `HyperEdgeSegment.recomputeExtent()`: the extent spans the first and last connection of each
/// side. Both sides are sorted, so those are the extremes.
pub fn recompute_extent(segment: &mut Segment) {
    let mut start = f64::NAN;
    let mut end = f64::NAN;
    for positions in [&segment.incoming, &segment.outgoing] {
        if let (Some(&first), Some(&last)) = (positions.first(), positions.last()) {
            start = if start.is_nan() {
                first
            } else {
                java_min(start, first)
            };
            end = if end.is_nan() {
                last
            } else {
                java_max(end, last)
            };
        }
    }
    segment.start = start;
    segment.end = end;
}

/// `HyperEdgeSegment.insertSorted`: inserts `value` into the sorted list, unless an equal value
/// is already there.
pub fn insert_sorted(list: &mut Vec<f64>, value: f64) {
    let mut index = 0;
    while index < list.len() {
        let next = list[index];
        if next == value {
            // An exactly equal value is already present in the list.
            return;
        }
        if next > value {
            break;
        }
        index += 1;
    }
    list.insert(index, value);
}

/// `HyperEdgeSegment.addPortPositions(port, map)`: the segment takes `port` and, depth first,
/// every port connected to it, and records each in `map`.
pub fn add_port_positions(
    arena: &mut Arena,
    graph: &LGraph,
    segment: usize,
    port: usize,
    map: &mut HashMap<usize, usize>,
) {
    map.insert(port, segment);
    arena.segments[segment].ports.push(port);
    let port_position = direction::port_position_on_hyper_node(graph, port);

    // The routing strategy's source side is east: those connections are incoming.
    if graph.ports[port].side == direction::SOURCE_PORT_SIDE {
        insert_sorted(&mut arena.segments[segment].incoming, port_position);
    } else {
        insert_sorted(&mut arena.segments[segment].outgoing, port_position);
    }
    recompute_extent(&mut arena.segments[segment]);

    for other in connected_ports(graph, port) {
        if !map.contains_key(&other) {
            add_port_positions(arena, graph, segment, other, map);
        }
    }
}

/// `LPort.getConnectedPorts()`: the sources of the incoming edges, then the targets of the
/// outgoing edges.
fn connected_ports(graph: &LGraph, port: usize) -> Vec<usize> {
    let port = &graph.ports[port];
    port.incoming
        .iter()
        .map(|&edge| graph.edges[edge].source)
        .chain(port.outgoing.iter().map(|&edge| graph.edges[edge].target))
        .collect()
}

/// `Math.min` for doubles: NaN wins, and -0.0 is less than 0.0.
pub fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        return if a.is_sign_negative() { a } else { b };
    }
    if a < b { a } else { b }
}

/// `Math.max` for doubles: NaN wins, and 0.0 is greater than -0.0.
pub fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        return if a.is_sign_positive() { a } else { b };
    }
    if a > b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_sorted_keeps_order_and_skips_equal_values() {
        let mut list = vec![1.0, 5.0];
        insert_sorted(&mut list, 3.0);
        insert_sorted(&mut list, 5.0);
        insert_sorted(&mut list, 0.5);
        insert_sorted(&mut list, 9.0);
        assert_eq!(list, vec![0.5, 1.0, 3.0, 5.0, 9.0]);
    }

    #[test]
    fn extent_spans_the_outer_connections_of_both_sides() {
        let mut segment = Segment {
            incoming: vec![10.0],
            outgoing: vec![0.0, 30.0],
            ..Segment::default()
        };
        recompute_extent(&mut segment);
        assert_eq!((segment.start, segment.end), (0.0, 30.0));
    }

    #[test]
    fn removing_a_dependency_clears_both_lists() {
        let mut arena = Arena::default();
        let a = arena.new_segment();
        let b = arena.new_segment();
        let dependency = arena.add_dependency(DependencyType::Regular, a, b, 3);
        assert_eq!(arena.segments[a].outgoing_deps, vec![dependency]);
        assert_eq!(arena.segments[b].incoming_deps, vec![dependency]);
        arena.reverse_dependency(dependency);
        assert_eq!(arena.segments[b].outgoing_deps, vec![dependency]);
        assert_eq!(arena.segments[a].incoming_deps, vec![dependency]);
        arena.remove_dependency(dependency);
        assert!(arena.segments[a].incoming_deps.is_empty());
        assert!(arena.segments[b].outgoing_deps.is_empty());
    }
}
