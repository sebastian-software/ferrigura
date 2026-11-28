// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg_edge_router,ovg_edge_set,route,route_directions,route_search_index,coordinator}.go
// and internal/layoutgraph/edge.go.

use std::collections::{HashMap, HashSet};

use crate::{Error, Point};

use super::super::grid::{Direction, Grid, Segment, charge};
use super::super::search::{SearchRoute, Transition};
use super::super::slingshot::step;
use super::super::{LinkedVisibility, point_key};

type PointKey = (u64, u64);
type LineKey = (Direction, u64);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Appearance {
    pub opacity: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<String>,
    pub stroke_dash: Option<String>,
    pub animated: Option<String>,
}

/// An ordinary edge after the reference has oriented its routing endpoints.
#[derive(Clone, Debug)]
pub struct OrdinaryEdge {
    pub owners: [usize; 2],
    pub arrowheads: [String; 2],
    pub appearance: Appearance,
    pub invisible: bool,
}

impl OrdinaryEdge {
    fn directed(&self) -> bool {
        let [source, target] = self.arrows();
        source != target
    }

    fn arrows(&self) -> [bool; 2] {
        self.arrowheads
            .each_ref()
            .map(|arrow| !arrow.is_empty() && arrow != "none")
    }
}

#[derive(Default)]
struct LinkUsage {
    nearby: bool,
    crossed: bool,
    overlapping: Vec<usize>,
}

struct AcceptedRoute {
    edge: OrdinaryEdge,
    vertices: Vec<usize>,
    directions: Option<HashMap<LineKey, Vec<DirectedInterval>>>,
}

#[derive(Default)]
pub(super) struct UsedPorts {
    pub duplicate: [HashSet<PointKey>; 2],
    pub symmetrical: HashSet<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Passage {
    Open,
    Crossing,
    Shared,
}

struct RouteLine {
    coordinate: f64,
    segments: Vec<[usize; 2]>,
}

struct RouteInsertion {
    new_segments: Vec<[usize; 2]>,
    crossed: Vec<usize>,
    nearby: Vec<(usize, bool)>,
}

/// Membership and overlap information for routes already chosen in one flavor.
pub struct RouteHistory<'a> {
    graph: &'a LinkedVisibility,
    grid: Grid,
    routes: Vec<AcceptedRoute>,
    points: HashMap<PointKey, Vec<usize>>,
    links: Vec<LinkUsage>,
    segments: HashSet<[usize; 2]>,
    lines: [Vec<RouteLine>; 2],
    tunnel_vertices: HashSet<usize>,
}

impl<'a> RouteHistory<'a> {
    /// Builds the initial occupancy state of a flavor.
    ///
    /// # Errors
    /// Returns an invalid graph error for invalid links or a work limit
    /// when index construction exceeds the remaining work.
    pub fn new(graph: &'a LinkedVisibility, work: &mut usize) -> Result<Self, Error> {
        let tunnels = graph
            .links
            .get(..graph.tunnel_link_count)
            .ok_or(Error::InvalidGraph)?;
        Ok(Self {
            graph,
            grid: Grid::new(graph, work)?,
            routes: Vec::new(),
            points: HashMap::new(),
            links: (0..graph.links.len())
                .map(|_| LinkUsage::default())
                .collect(),
            segments: HashSet::new(),
            lines: [Vec::new(), Vec::new()],
            tunnel_vertices: tunnels.iter().flatten().copied().collect(),
        })
    }

    pub(super) fn graph(&self) -> &LinkedVisibility {
        self.graph
    }

    /// Registers a raw route and marks the grid for subsequent searches.
    ///
    /// # Errors
    /// Returns a work limit or an invalid graph error for a malformed path.
    /// The history is updated only after all marks have been computed.
    pub fn accept(
        &mut self,
        edge: OrdinaryEdge,
        route: SearchRoute,
        work: &mut usize,
    ) -> Result<(), Error> {
        let mut visited = HashSet::new();
        let vertices = route.vertices;
        let [source, target] = edge
            .owners
            .map(|owner| self.graph.centers_by_node.get(owner));
        if vertices.len() < 4
            || vertices.first() != source
            || vertices.last() != target
            || source == target
        {
            return Err(Error::InvalidGraph);
        }
        for &index in &vertices {
            step(work)?;
            if index >= self.graph.vertices.len() || !visited.insert(index) {
                return Err(Error::InvalidGraph);
            }
        }
        let staged = self.stage_route(&vertices, edge.invisible, work)?;
        let route_index = self.routes.len();
        for &vertex in &vertices {
            self.points
                .entry(self.key(vertex))
                .or_default()
                .push(route_index);
        }
        for pair in staged.new_segments {
            self.segments.insert(pair);
            self.insert_line_segment(pair);
        }
        for link in staged.crossed {
            self.links[link].crossed = true;
        }
        for (link, on_line) in staged.nearby {
            let usage = &mut self.links[link];
            usage.nearby = true;
            if on_line {
                usage.overlapping.push(route_index);
            }
        }
        self.routes.push(AcceptedRoute {
            edge,
            vertices,
            directions: None,
        });
        Ok(())
    }

    fn stage_route(
        &self,
        vertices: &[usize],
        invisible: bool,
        work: &mut usize,
    ) -> Result<RouteInsertion, Error> {
        let mut new_segments = Vec::new();
        let mut active_lines = HashMap::new();
        let mut crossed = Vec::new();
        let mut nearby = Vec::new();
        let interior = &vertices[1..vertices.len() - 1];
        charge(work, vertices.len())?;
        for pair in interior.windows(2) {
            let segment = [pair[0], pair[1]];
            step(work)?;
            if self.segments.contains(&segment) {
                continue;
            }
            new_segments.push(segment);
            let segment = self.segment(segment);
            if let Some(direction) = segment.direction() {
                let axis = segment.ordered(direction).0;
                let key = line_key(direction, axis);
                let used_on_line = self
                    .line(direction, axis)
                    .map_or(0, |line| line.segments.len());
                let pending_on_line = active_lines.get(&key).copied().unwrap_or(0);
                if used_on_line + pending_on_line == 0 {
                    let used_lines = self.lines[direction.index()].len();
                    let new_lines = active_lines
                        .keys()
                        .filter(|&&(kind, axis_key)| {
                            kind == direction && self.line(kind, f64::from_bits(axis_key)).is_none()
                        })
                        .count();
                    charge(work, used_lines + new_lines + 1)?;
                }
                charge(work, used_on_line + pending_on_line + 1)?;
                *active_lines.entry(key).or_default() += 1;
            }
            self.grid
                .visit_crossed(segment, work, |link| crossed.push(link))?;
        }
        if !invisible {
            for pair in interior.windows(2) {
                step(work)?;
                let segment = self.segment([pair[0], pair[1]]);
                self.grid
                    .visit_nearby(segment, work, |link, on_line| nearby.push((link, on_line)))?;
            }
        }
        Ok(RouteInsertion {
            new_segments,
            crossed,
            nearby,
        })
    }

    fn line(&self, direction: Direction, axis: f64) -> Option<&RouteLine> {
        let lines = &self.lines[direction.index()];
        let index = lines.partition_point(|line| line.coordinate < axis);
        lines.get(index).filter(|line| line.coordinate == axis)
    }

    fn insert_line_segment(&mut self, pair: [usize; 2]) {
        let segment = self.segment(pair);
        let Some(direction) = segment.direction() else {
            return;
        };
        let (axis, start, _) = segment.ordered(direction);
        let lines = &mut self.lines[direction.index()];
        let position = lines.partition_point(|line| line.coordinate < axis);
        if lines
            .get(position)
            .is_none_or(|line| line.coordinate != axis)
        {
            lines.insert(
                position,
                RouteLine {
                    coordinate: axis,
                    segments: Vec::new(),
                },
            );
        }
        let segments = &mut lines[position].segments;
        let position = segments.partition_point(|entry| {
            segment_for_entry(self.graph, *entry).ordered(direction).1 < start
        });
        segments.insert(position, pair);
    }

    pub(super) fn has_crossing(&self, pair: [usize; 2], work: &mut usize) -> Result<bool, Error> {
        let probe = self.segment(pair);
        // Go checks horizontal first for a zero-length query.
        let Some((key, _, _)) = along_line(probe) else {
            return Ok(false);
        };
        let (direction, _) = key;
        let (axis, start, end) = probe.ordered(direction);
        if let Some(line) = self.line(direction, axis) {
            for &pair in &line.segments {
                step(work)?;
                let candidate = self.segment(pair);
                let (_, first, last) = candidate.ordered(direction);
                if end <= first {
                    break;
                }
                if start < last && !probe.shares_point(candidate) {
                    return Ok(true);
                }
            }
        }
        let lines = &self.lines[direction.other().index()];
        let first = lines.partition_point(|line| line.coordinate < start);
        charge(work, first)?;
        for line in &lines[first..] {
            step(work)?;
            if end < line.coordinate {
                break;
            }
            for &pair in &line.segments {
                step(work)?;
                let candidate = self.segment(pair);
                let (_, first, last) = candidate.ordered(direction.other());
                if axis < first {
                    break;
                }
                if axis <= last && !probe.shares_point(candidate) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub(super) fn is_occupied(
        &mut self,
        edge: &OrdinaryEdge,
        pair: [usize; 2],
        work: &mut usize,
    ) -> Result<bool, Error> {
        let probe = self.segment(pair);
        let Some(direction) = probe.direction() else {
            return Ok(false);
        };
        let (axis, start, end) = probe.ordered(direction);
        let Some(line) = self.line(direction, axis) else {
            return Ok(false);
        };
        let mut overlap = Vec::new();
        for &entry in &line.segments {
            step(work)?;
            let (_, first, last) = self.segment(entry).ordered(direction);
            if start <= last && first <= end {
                overlap.push(entry);
            }
        }
        let mut routes = Vec::new();
        let mut seen = HashSet::new();
        for [from, to] in overlap {
            step(work)?;
            for &left in self.routes_at(from) {
                step(work)?;
                for &right in self.routes_at(to) {
                    step(work)?;
                    if left == right && seen.insert(right) {
                        routes.push(right);
                    }
                }
            }
        }
        for &index in &routes {
            step(work)?;
            let route = &mut self.routes[index];
            if edge.directed() && route.edge.directed() && route.opposes(self.graph, probe, work)? {
                return Ok(true);
            }
        }
        Ok(!self.shareable(edge, &routes, work)?)
    }

    pub(super) fn is_duplicate(&self, path: &[usize], work: &mut usize) -> Result<bool, Error> {
        for route in &self.routes {
            step(work)?;
            let vertices = &route.vertices;
            if vertices.len() != path.len()
                || self.key(vertices[1]) != self.key(path[1])
                || self.key(vertices[vertices.len() - 2]) != self.key(path[path.len() - 2])
            {
                continue;
            }
            let mut equal = true;
            for i in 2..path.len() - 2 {
                step(work)?;
                if self.key(vertices[i]) != self.key(path[i]) {
                    equal = false;
                    break;
                }
            }
            if equal {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn segment(&self, indices: [usize; 2]) -> Segment {
        segment_for_entry(self.graph, indices)
    }

    fn key(&self, vertex: usize) -> PointKey {
        point_key(self.graph.vertices[vertex].point)
    }

    fn routes_at(&self, vertex: usize) -> &[usize] {
        self.points
            .get(&self.key(vertex))
            .map_or(&[], Vec::as_slice)
    }

    pub(super) fn used_ports(
        &self,
        owners: [usize; 2],
        centers: [usize; 2],
        work: &mut usize,
    ) -> Result<UsedPorts, Error> {
        let mut ports = UsedPorts::default();
        let mut used = [HashSet::new(), HashSet::new()];
        let endpoint_ports = owners.map(|owner| &self.graph.ports_by_node[owner]);
        for center in centers {
            step(work)?;
            for &route in self.routes_at(center) {
                step(work)?;
                let route = &self.routes[route];
                if !route.edge.owners.iter().any(|owner| owners.contains(owner)) {
                    continue;
                }
                if route.edge.owners == owners {
                    ports.duplicate[0].insert(self.key(route.vertices[1]));
                    ports.duplicate[1].insert(self.key(route.vertices[route.vertices.len() - 2]));
                }
                for &vertex in &route.vertices {
                    step(work)?;
                    for (ports, used) in endpoint_ports.iter().zip(&mut used) {
                        for &port in *ports {
                            step(work)?;
                            if port == vertex {
                                used.insert(self.key(port));
                                break;
                            }
                        }
                    }
                }
            }
        }
        charge(work, endpoint_ports[0].len() + endpoint_ports[1].len())?;
        for (endpoint, used) in endpoint_ports.into_iter().zip(used) {
            let mirrored = [(1, 7), (7, 1), (4, 10), (10, 4)]
                .map(|(index, other)| (self.key(endpoint[index]), self.key(endpoint[other])));
            for &vertex in endpoint {
                step(work)?;
                if mirrored.iter().any(|&(original, mirror)| {
                    original == self.key(vertex) && used.contains(&mirror)
                }) {
                    ports.symmetrical.insert(vertex);
                }
            }
        }
        Ok(ports)
    }

    pub(super) fn passage(
        &mut self,
        edge: &OrdinaryEdge,
        transition: Transition,
        source: usize,
        work: &mut usize,
    ) -> Result<Passage, Error> {
        let leaving_source = transition.from == source;
        let usage = &self.links[transition.link];
        if leaving_source {
            return Ok(Passage::Open);
        }
        let occupancy = self.routes_at(transition.from);
        charge(work, occupancy.len())?;
        let on_existing = !occupancy.is_empty();
        let on_next = !self.routes_at(transition.to).is_empty();
        let all_shareable = self.shareable(edge, occupancy, work)?;
        charge(work, usage.overlapping.len())?;
        if !self.shareable(edge, &usage.overlapping, work)? {
            return Ok(Passage::Shared);
        }
        let nearby = usage.nearby;
        let crossed = usage.crossed;
        let overlapped = usage.overlapping.clone();
        if self.traversal_blocked(&overlapped, edge.directed(), transition, work)?
            || !on_existing && !on_next && nearby
        {
            return Ok(Passage::Shared);
        }
        if on_existing && !on_next && !all_shareable {
            return Ok(Passage::Crossing);
        }
        step(work)?;
        Ok(if crossed {
            Passage::Crossing
        } else {
            Passage::Open
        })
    }

    fn shareable(
        &self,
        edge: &OrdinaryEdge,
        routes: &[usize],
        work: &mut usize,
    ) -> Result<bool, Error> {
        if routes.is_empty() {
            return Ok(true);
        }
        let passes = if edge.directed() { 1 } else { 10 };
        for _ in 0..passes {
            charge(work, routes.len())?;
        }
        let other_edges = routes.iter().map(|&route| &self.routes[route].edge);
        if edge.directed() {
            return Ok(other_edges.clone().all(|other| {
                other.directed()
                    && (edge.owners[0] == other.owners[0] || edge.owners[1] == other.owners[1])
                    && edge.arrowheads == other.arrowheads
                    && edge.appearance == other.appearance
            }));
        }
        let bidirectional = edge.arrows() == [true, true];
        let undirected = edge.arrows() == [false, false];
        if bidirectional && edge.arrowheads[0] != edge.arrowheads[1]
            || !bidirectional && !undirected
        {
            return Ok(false);
        }
        if !other_edges.clone().all(|other| {
            other.appearance == edge.appearance
                && if bidirectional {
                    other.arrowheads == edge.arrowheads
                } else {
                    other.arrows() == [false, false]
                }
        }) {
            return Ok(false);
        }
        // The reference counts point occurrences in this multiset of routes.
        // Coincident OVG vertices can record one route more than once.
        let mut counts = HashMap::new();
        for edge in std::iter::once(edge).chain(other_edges) {
            for owner in edge.owners {
                *counts.entry(owner).or_insert(0) += 1;
            }
        }
        Ok(counts.into_values().any(|count| count == routes.len() + 1))
    }

    fn traversal_blocked(
        &mut self,
        routes: &[usize],
        directed: bool,
        transition: Transition,
        work: &mut usize,
    ) -> Result<bool, Error> {
        let is_tunnel = self.tunnel_vertices.contains(&transition.to);
        if !directed && !is_tunnel {
            return Ok(false);
        }
        let candidate = self.segment([transition.from, transition.to]);
        for &index in routes {
            step(work)?;
            let route = &mut self.routes[index];
            if directed && route.edge.directed() && route.opposes(self.graph, candidate, work)? {
                return Ok(true);
            }
            if is_tunnel {
                charge(work, route.vertices.len())?;
                if route.contained_by(self.graph, candidate) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

fn line_key(direction: Direction, coordinate: f64) -> LineKey {
    (
        direction,
        point_key(Point {
            x: coordinate,
            y: 0.0,
        })
        .0,
    )
}

fn segment_for_entry(graph: &LinkedVisibility, [from, to]: [usize; 2]) -> Segment {
    Segment {
        from: graph.vertices[from].point,
        to: graph.vertices[to].point,
    }
}

struct DirectedInterval {
    from: f64,
    to: f64,
}

impl DirectedInterval {
    fn opposes(&self, from: f64, to: f64) -> Option<bool> {
        let lo = self.from.min(self.to);
        let hi = self.from.max(self.to);
        if from.max(to) < lo || hi < from.min(to) {
            return None;
        }
        if (from == lo || from == hi) && (to < lo || to > hi)
            || (to == lo || to == hi) && (from < lo || from > hi)
        {
            return Some(false);
        }
        if self.to > self.from && to < from || self.to < self.from && to > from {
            Some(true)
        } else {
            None
        }
    }
}

impl AcceptedRoute {
    fn opposes(
        &mut self,
        graph: &LinkedVisibility,
        candidate: Segment,
        work: &mut usize,
    ) -> Result<bool, Error> {
        if self.directions.is_none() {
            let mut directions: HashMap<_, Vec<_>> = HashMap::new();
            for pair in self.vertices[..self.vertices.len() - 1].windows(2) {
                step(work)?;
                let segment = segment_for_entry(graph, [pair[0], pair[1]]);
                if segment.from == segment.to {
                    continue;
                }
                if let Some((key, from, to)) = along_line(segment) {
                    directions
                        .entry(key)
                        .or_default()
                        .push(DirectedInterval { from, to });
                }
            }
            self.directions = Some(directions);
        }
        let Some((key, from, to)) = along_line(candidate) else {
            return Ok(false);
        };
        if let Some(intervals) = self
            .directions
            .as_ref()
            .and_then(|directions| directions.get(&key))
        {
            for interval in intervals {
                step(work)?;
                if let Some(opposing) = interval.opposes(from, to) {
                    return Ok(opposing);
                }
            }
        }
        Ok(false)
    }

    fn contained_by(&self, graph: &LinkedVisibility, candidate: Segment) -> bool {
        let direction = if candidate.from.y == candidate.to.y {
            Direction::Horizontal
        } else if candidate.from.x == candidate.to.x {
            Direction::Vertical
        } else {
            return false;
        };
        let (line, start, end) = candidate.ordered(direction);
        self.vertices[1..self.vertices.len() - 1]
            .windows(2)
            .all(|pair| {
                let segment = segment_for_entry(graph, [pair[0], pair[1]]);
                let aligned = match direction {
                    Direction::Horizontal => segment.from.y == line && segment.to.y == line,
                    Direction::Vertical => segment.from.x == line && segment.to.x == line,
                };
                let (_, first, last) = segment.ordered(direction);
                aligned && start <= first && last <= end
            })
    }
}

fn along_line(segment: Segment) -> Option<(LineKey, f64, f64)> {
    if segment.from.y == segment.to.y {
        Some((
            line_key(Direction::Horizontal, segment.from.y),
            segment.from.x,
            segment.to.x,
        ))
    } else if segment.from.x == segment.to.x {
        Some((
            line_key(Direction::Vertical, segment.from.x),
            segment.from.y,
            segment.to.y,
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
