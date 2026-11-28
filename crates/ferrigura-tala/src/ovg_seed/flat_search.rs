// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg_edge_router,geometry,route_label_boxes,tuning}.go and
// internal/layoutgraph/{geometry_policy,graph,node}.go.

use std::collections::HashSet;

use crate::{Error, NodeGeometry, Point};

use super::search::{
    BASICALLY_INFINITY, SearchRoute, SearchScratch, Transition, TurnPolicy, ideal_turn_axes,
};
use super::slingshot::{Orientation, VisibilityAdjacency, step};
use super::{LinkedVisibility, PortOwner, SLOTS_PER_RECTANGLE, Vertex, point_key};

mod flavors;
mod flight;
pub use flavors::{Flavor, FlavorRoutes, RouteChoice, RouteStrategy, order_routes};
mod history;
pub use history::{Appearance, OrdinaryEdge, RouteHistory};
use history::{Passage, UsedPorts};

const CONNECTED_NODE_GAP: f64 = 60.0;
const TURN_MULTIPLIER: f64 = 0.125;
const CENTER_PORT_MULTIPLIER: f64 = 0.04287499999999999;
const CROSSING_MULTIPLIER: f64 = 0.110_592;
const TURN_ENDPOINT_CLEARANCE: f64 = 20.0;
const NODE_PROXIMITY_PENALTY: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    pub turn: f64,
    pub crossing: f64,
    pub non_center_port: f64,
}

impl Weights {
    /// Calculates the weights for one positioned ordinary component.
    ///
    /// # Errors
    /// Returns an invalid graph error if an edge endpoint is out of range.
    pub fn from_graph(nodes: &[NodeGeometry], edges: &[[usize; 2]]) -> Result<Self, Error> {
        let mut max_length = if edges.is_empty() {
            0.0
        } else {
            CONNECTED_NODE_GAP
        };
        for &[source, target] in edges {
            let source = nodes.get(source).ok_or(Error::InvalidGraph)?;
            let target = nodes.get(target).ok_or(Error::InvalidGraph)?;
            max_length =
                max_length.max(Rectangle::from(source).distance_to(Rectangle::from(target)));
        }
        let count = edges.len() as f64;
        let min_size = nodes
            .iter()
            .map(|node| node.width.min(node.height))
            .reduce(f64::min);
        let mut non_center_port = CENTER_PORT_MULTIPLIER * count * max_length;
        if count != 0.0
            && let Some(size) = min_size
        {
            non_center_port = non_center_port.max(size / 3.0);
        }
        Ok(Self {
            turn: TURN_MULTIPLIER * count * max_length,
            crossing: CROSSING_MULTIPLIER * count * max_length,
            non_center_port,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl From<&NodeGeometry> for Rectangle {
    fn from(node: &NodeGeometry) -> Self {
        Self {
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
        }
    }
}

impl Rectangle {
    fn limits(self) -> [[f64; 2]; 2] {
        [
            [self.x, self.x + self.width],
            [self.y, self.y + self.height],
        ]
    }

    fn distance_to(self, other: Self) -> f64 {
        let [x, y] = self.limits();
        let [u, v] = other.limits();
        let dx = interval_gap(x, u);
        let dy = interval_gap(y, v);
        euclidean(Point { x: 0.0, y: 0.0 }, Point { x: dx, y: dy })
    }

    fn distance_to_point(self, point: Point) -> f64 {
        self.distance_to(Self {
            x: point.x,
            y: point.y,
            width: 0.0,
            height: 0.0,
        })
    }

    fn intersects(self, start: Point, end: Point) -> bool {
        let [[x1, x2], [y1, y2]] = self.limits();
        let [left, right] = [x1.min(x2), x1.max(x2)];
        let [top, bottom] = [y1.min(y2), y1.max(y2)];
        if start.x.max(end.x) < left
            || right < start.x.min(end.x)
            || start.y.max(end.y) < top
            || bottom < start.y.min(end.y)
        {
            return false;
        }
        let contains = |point: Point| {
            left <= point.x && point.x <= right && top <= point.y && point.y <= bottom
        };
        if contains(start) || contains(end) {
            return true;
        }
        let mut t_enter: f64 = 0.0;
        let mut t_exit: f64 = 1.0;
        for (first, last, lower, upper) in
            [(start.x, end.x, left, right), (start.y, end.y, top, bottom)]
        {
            let delta = last - first;
            if delta == 0.0 {
                if first < lower || upper < first {
                    return false;
                }
                continue;
            }
            let one = (lower - first) / delta;
            let two = (upper - first) / delta;
            t_enter = t_enter.max(one.min(two));
            t_exit = t_exit.min(one.max(two));
            if t_enter > t_exit {
                return false;
            }
        }
        t_enter < t_exit
    }
}

fn interval_gap(a: [f64; 2], b: [f64; 2]) -> f64 {
    if a[1] < b[0] {
        b[0] - a[1]
    } else if b[1] < a[0] {
        a[0] - b[1]
    } else {
        0.0
    }
}

fn euclidean(a: Point, b: Point) -> f64 {
    if a.x == b.x {
        (a.y - b.y).abs()
    } else if a.y == b.y {
        (a.x - b.x).abs()
    } else {
        let x = a.x - b.x;
        let y = a.y - b.y;
        (x * x + y * y).sqrt()
    }
}

pub struct FlatSearch<'a> {
    graph: &'a LinkedVisibility,
    nodes: &'a [NodeGeometry],
    labels: &'a [Option<Rectangle>],
    weights: Weights,
    adjacency: VisibilityAdjacency<'a>,
    is_center: Vec<bool>,
}

impl<'a> FlatSearch<'a> {
    /// Binds a linked graph, ordinary node geometry, and measured label boxes
    /// in the same node order.
    ///
    /// # Errors
    /// Returns an invalid graph error if node and label counts differ.
    pub fn new(
        graph: &'a LinkedVisibility,
        nodes: &'a [NodeGeometry],
        labels: &'a [Option<Rectangle>],
        weights: Weights,
    ) -> Result<Self, Error> {
        if nodes.len() != labels.len()
            || graph.centers_by_node.len() != nodes.len()
            || graph.ports_by_node.len() != nodes.len()
            || graph.near_port_owners.len() != graph.vertices.len()
            || graph.ports_by_node.iter().any(|ports| {
                ports.len() < SLOTS_PER_RECTANGLE
                    || ports.iter().any(|&index| index >= graph.vertices.len())
            })
        {
            return Err(Error::InvalidGraph);
        }
        let mut is_center = vec![false; graph.vertices.len()];
        for &index in &graph.centers_by_node {
            *is_center.get_mut(index).ok_or(Error::InvalidGraph)? = true;
        }
        Ok(Self {
            graph,
            nodes,
            labels,
            weights,
            adjacency: VisibilityAdjacency::new(graph)?,
            is_center,
        })
    }

    /// Calculates the first weighted search of a flat routing flavor, before
    /// previous routes contribute occupancy, crossing, or port-use penalties.
    ///
    /// # Errors
    /// Returns a work limit or invalid graph error from visibility search.
    pub fn find_first(
        &self,
        scratch: &mut SearchScratch,
        owners: [usize; 2],
        work: &mut usize,
    ) -> Result<Option<SearchRoute>, Error> {
        self.find_costed(scratch, owners, None, work)
    }

    /// Searches with the ordinary routes already accepted by this flavor.
    ///
    /// # Errors
    /// Returns invalid graph for mismatched geometry or endpoints and a work
    /// limit if the remaining search work is insufficient.
    pub fn find(
        &self,
        scratch: &mut SearchScratch,
        history: &mut RouteHistory<'_>,
        edge: &OrdinaryEdge,
        work: &mut usize,
    ) -> Result<Option<SearchRoute>, Error> {
        if !std::ptr::eq(history.graph(), self.graph) {
            return Err(Error::InvalidGraph);
        }
        self.find_costed(scratch, edge.owners, Some((history, edge)), work)
    }

    fn find_costed(
        &self,
        scratch: &mut SearchScratch,
        owners: [usize; 2],
        history: Option<(&mut RouteHistory<'_>, &OrdinaryEdge)>,
        work: &mut usize,
    ) -> Result<Option<SearchRoute>, Error> {
        let [source, target] = owners.map(|index| self.nodes.get(index));
        let source = source.ok_or(Error::InvalidGraph)?;
        let target = target.ok_or(Error::InvalidGraph)?;
        let axes = ideal_turn_axes(source, target);
        let turns = TurnPolicy {
            axes: &axes,
            cost: self.weights.turn,
            even_arranged_source_cluster: false,
        };
        for _ in self.nodes {
            step(work)?;
        }
        let [source_ports, target_ports] = owners.map(|owner| &self.graph.ports_by_node[owner]);
        let overlap_checks = source_ports
            .len()
            .checked_mul(target_ports.len())
            .ok_or(Error::WorkLimit)?;
        *work = work.checked_sub(overlap_checks).ok_or(Error::WorkLimit)?;
        let blocked = self.shared_base_port_coordinates(source_ports, target_ports);
        let centers = owners.map(|owner| self.graph.centers_by_node[owner]);
        let used = if let Some((history, _)) = &history {
            history.used_ports(owners, centers, work)?
        } else {
            UsedPorts::default()
        };
        let mut transition = RouteCost {
            routing: self,
            owners,
            centers,
            overlap: Orientation::between(source, target) == Orientation::Overlap,
            turns: &turns,
            blocked,
            history,
            used,
        };
        scratch.find(
            &self.adjacency,
            transition.centers,
            &turns,
            work,
            |candidate, work| transition.cost(candidate, work),
        )
    }

    fn shared_base_port_coordinates(
        &self,
        first: &[usize],
        second: &[usize],
    ) -> HashSet<(u64, u64)> {
        let coords = |index| {
            let vertex: &Vertex = &self.graph.vertices[index];
            point_key(vertex.point)
        };
        let source: HashSet<_> = first[..SLOTS_PER_RECTANGLE]
            .iter()
            .copied()
            .map(coords)
            .collect();
        second[..SLOTS_PER_RECTANGLE]
            .iter()
            .copied()
            .map(coords)
            .filter(|point| source.contains(point))
            .collect()
    }
}

struct RouteCost<'a, 'h, 'g> {
    routing: &'a FlatSearch<'a>,
    owners: [usize; 2],
    centers: [usize; 2],
    overlap: bool,
    turns: &'a TurnPolicy<'a>,
    blocked: HashSet<(u64, u64)>,
    history: Option<(&'h mut RouteHistory<'g>, &'h OrdinaryEdge)>,
    used: UsedPorts,
}

impl RouteCost<'_, '_, '_> {
    fn cost(&mut self, candidate: Transition, work: &mut usize) -> Result<Option<f64>, Error> {
        let graph = self.routing.graph;
        let [source, target] = self.owners;
        let [source_center, target_center] = self.centers;
        let first = &graph.vertices[candidate.from];
        let second = &graph.vertices[candidate.to];
        let leaving_source = candidate.from == source_center;
        let entering_target = candidate.to == target_center;

        if port(first, source)
            .is_some_and(|metadata| !port_faces(metadata, first.point, second.point, self.overlap))
            || port(second, target).is_some_and(|metadata| {
                !port_faces(metadata, second.point, first.point, self.overlap)
            })
        {
            return Ok(None);
        }
        if !leaving_source
            && !entering_target
            && first.point.x != second.point.x
            && first.point.y != second.point.y
        {
            return Ok(None);
        }
        if (port(second, target).is_some() || port(second, source).is_some())
            && self.blocked.contains(&point_key(second.point))
        {
            return Ok(None);
        }

        let mut cost = if leaving_source {
            self.endpoint_penalty(candidate.to, 0)
        } else {
            euclidean(first.point, second.point)
        };
        if first
            .owners
            .iter()
            .any(|owner| port(second, owner.node).is_some())
        {
            cost += self.routing.weights.turn * 4.0;
        }
        if entering_target {
            cost = self.endpoint_penalty(candidate.from, 1);
        } else if self.routing.is_center[candidate.to] {
            cost = BASICALLY_INFINITY;
        } else if let Some((history, edge)) = &mut self.history {
            match history.passage(edge, candidate, source_center, work)? {
                Passage::Open => {}
                Passage::Shared => cost = BASICALLY_INFINITY,
                Passage::Crossing => cost += self.routing.weights.crossing,
            }
        } else if !leaving_source {
            // The reference charges one crossing lookup even before any
            // route has occupied the graph.
            step(work)?;
        }

        for (owner, label) in self.routing.labels.iter().enumerate() {
            if owner != source
                && owner != target
                && let Some(label) = label
            {
                step(work)?;
                let [a, b] = graph.links[candidate.link].map(|index| graph.vertices[index].point);
                if label.intersects(a, b) {
                    cost += self.routing.weights.turn;
                }
            }
        }

        if self.is_turn(candidate, first.point, second.point) {
            cost += self.routing.weights.turn * self.turns.multiplier(second.point, work)?;
            for owner in [target, source] {
                if Rectangle::from(&self.routing.nodes[owner]).distance_to_point(first.point)
                    <= TURN_ENDPOINT_CLEARANCE
                {
                    cost += self.routing.weights.turn;
                }
            }
        }
        if !leaving_source
            && !entering_target
            && graph.near_port_owners[candidate.to]
                .iter()
                .any(|owner| self.owners.contains(owner))
        {
            cost += NODE_PROXIMITY_PENALTY;
        }
        Ok(Some(cost))
    }

    fn endpoint_penalty(&self, index: usize, side: usize) -> f64 {
        let vertex = &self.routing.graph.vertices[index];
        let mut penalty = if self.used.duplicate[side].contains(&point_key(vertex.point)) {
            BASICALLY_INFINITY
        } else {
            1.0
        };
        match port(vertex, self.owners[side]) {
            Some(metadata) if metadata.center => {
                if !self.used.symmetrical.contains(&index) {
                    penalty += 1.0;
                }
            }
            Some(_) => penalty += self.routing.weights.non_center_port,
            None if side == 0 => penalty += self.routing.weights.non_center_port,
            None => {}
        }
        penalty
    }

    fn is_turn(&self, transition: Transition, from: Point, to: Point) -> bool {
        let [source, target] = self.centers;
        transition
            .previous
            .is_some_and(|previous| previous != source)
            && transition.from != source
            && transition.to != target
            && if transition.from_horizontal {
                to.y != from.y
            } else {
                to.x != from.x
            }
    }
}

fn port(vertex: &Vertex, owner: usize) -> Option<&PortOwner> {
    vertex.owners.iter().find(|metadata| metadata.node == owner)
}

fn port_faces(metadata: &PortOwner, port: Point, outside: Point, overlap: bool) -> bool {
    let sides = metadata.sides;
    if sides == 0 {
        return true;
    }
    let [top, left, bottom, right] = [1, 2, 4, 8];
    if overlap {
        return sides & (top | bottom) != 0 && outside.y != port.y
            || sides & (left | right) != 0 && outside.x != port.x;
    }
    sides & top != 0 && outside.y < port.y
        || sides & left != 0 && outside.x < port.x
        || sides & bottom != 0 && outside.y > port.y
        || sides & right != 0 && outside.x > port.x
}

#[cfg(test)]
mod tests;
