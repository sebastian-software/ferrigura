// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{coordinator,standalone_route_guard,cluster_route_guard,
// geometry,cost,tuning}.go and internal/nodeshape/shape.go.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::balance::non_shared_crossing;
use crate::{Error, NodeGeometry, Point};

use super::super::grid::{Direction, Segment, charge};
use super::super::point_key;
use super::super::slingshot::{Orientation, step};
use super::history::can_share;
use super::{CROSSING_MULTIPLIER, FlatSearch, OrdinaryEdge, Rectangle, euclidean};

const NON_ORTHOGONAL_FACTOR: f64 = 4.0;
const AXIS_ALIGNMENT_TOLERANCE: f64 = 1.0;
const MIN_STRAIGHT_EDGE_ANGLE: f64 = 30.0;
const LOWER_JITTER_THRESHOLD: f64 = 5.0;
const TURN_PENALTY: f64 = 1.4;

pub struct ExistingLine<'a> {
    pub edge: &'a OrdinaryEdge,
    pub points: &'a [Point],
}

#[derive(Debug, PartialEq)]
pub struct StraightRoute {
    pub ports: [Point; 2],
    pub cost: f64,
}

impl FlatSearch<'_> {
    /// Replaces ordinary materialized routes with less costly straight lines.
    /// Each candidate sees the preceding replacements in reference edge order.
    ///
    /// # Errors
    /// Returns invalid graph for mismatched route input and a work limit if
    /// evaluation cannot finish. Routes are published together on success.
    pub fn straight_edges_fallback(
        &self,
        edges: &[OrdinaryEdge],
        routes: &mut [Vec<Point>],
        work: &mut usize,
    ) -> Result<(), Error> {
        if edges.len() != routes.len() || routes.iter().any(|points| points.len() < 2) {
            return Err(Error::InvalidGraph);
        }
        let mut staged = routes.to_vec();
        for (index, edge) in edges.iter().enumerate() {
            step(work)?;
            let original = &staged[index];
            let previous = edges
                .iter()
                .zip(&staged)
                .enumerate()
                .filter(|&(other, _)| other != index)
                .map(|(_, (edge, points))| ExistingLine { edge, points })
                .collect::<Vec<_>>();
            step(work)?;
            let original_cost = estimated_route_cost(original, &previous, work)?;
            let Some(replacement) = self.straight_line(edge, &previous, work)? else {
                continue;
            };
            let mut cost = replacement.cost;
            if original.len() == 4 && euclidean(original[1], original[2]) <= LOWER_JITTER_THRESHOLD
            {
                cost /= NON_ORTHOGONAL_FACTOR;
            }
            step(work)?;
            if cost < original_cost {
                staged[index] = replacement.ports.to_vec();
            }
        }
        routes.clone_from_slice(&staged);
        Ok(())
    }

    /// Selects the pinned reference's lowest-cost pair of facing ports for
    /// an ordinary straight connector. Existing routes do not include the
    /// candidate edge and use their already materialized points.
    ///
    /// # Errors
    /// Returns invalid graph for nonexistent endpoint indices, or a work
    /// limit if the remaining budget cannot evaluate all port pairs.
    pub fn straight_line(
        &self,
        edge: &OrdinaryEdge,
        existing: &[ExistingLine<'_>],
        work: &mut usize,
    ) -> Result<Option<StraightRoute>, Error> {
        step(work)?;
        let [source, target] = edge.owners;
        let from = self.nodes.get(source).ok_or(Error::InvalidGraph)?;
        let to = self.nodes.get(target).ok_or(Error::InvalidGraph)?;
        let orientations = [
            Orientation::between(from, to),
            Orientation::between(to, from),
        ];
        if orientations.contains(&Orientation::Overlap) {
            return Ok(None);
        }
        let candidates = [source, target].map(|owner| &self.graph.ports_by_node[owner]);
        let [launching, landing] =
            [0, 1].map(|side| facing_ports(candidates[side], orientations[side]));
        let ports = [
            port_edges(existing, source, work)?,
            port_edges(existing, target, work)?,
        ];
        for route in existing {
            step(work)?;
            charge(work, route.points.len())?;
        }
        let selector = LineSelector {
            routing: self,
            edge,
            existing,
            ports,
            orientations,
        };
        let mut best = None;
        let mut lowest = f64::INFINITY;
        for start in launching {
            step(work)?;
            let from = self.graph.vertices[start].point;
            let Some(from_cost) = selector.port_cost(from, 0, work)? else {
                continue;
            };
            for &end in &landing {
                step(work)?;
                let to = self.graph.vertices[end].point;
                let Some(to_cost) = selector.port_cost(to, 1, work)? else {
                    continue;
                };
                let Some(candidate_cost) = selector.route_cost([from, to], work)? else {
                    continue;
                };
                let mut cost = from_cost + to_cost + candidate_cost;
                if from.x != to.x
                    && from.y != to.y
                    && (from.x - to.x).abs() >= AXIS_ALIGNMENT_TOLERANCE
                    && (from.y - to.y).abs() >= AXIS_ALIGNMENT_TOLERANCE
                {
                    cost *= NON_ORTHOGONAL_FACTOR;
                }
                if orientations
                    .iter()
                    .any(|&orientation| sharp_angle(from, to, orientation))
                {
                    continue;
                }
                let endpoints = [(source, from), (target, to)];
                for (owner, point) in endpoints {
                    if ![1, 4, 7, 10].into_iter().any(|slot| {
                        let center =
                            self.graph.vertices[self.graph.ports_by_node[owner][slot]].point;
                        point == center
                    }) {
                        cost += self.weights.non_center_port;
                    }
                }
                if cost < lowest {
                    lowest = cost;
                    best = Some(StraightRoute {
                        ports: [from, to],
                        cost,
                    });
                }
            }
        }
        Ok(best)
    }
}

fn facing_ports(ports: &[usize], orientation: Orientation) -> Vec<usize> {
    let sides: &[usize] = match orientation {
        Orientation::TopLeft => &[2, 3],
        Orientation::Top => &[2],
        Orientation::TopRight => &[2, 1],
        Orientation::Left => &[3],
        Orientation::Overlap => &[],
        Orientation::Right => &[1],
        Orientation::BottomLeft => &[0, 3],
        Orientation::Bottom => &[0],
        Orientation::BottomRight => &[0, 1],
    };
    sides
        .iter()
        .flat_map(|&side| ports[side * 3..side * 3 + 3].iter().copied())
        .collect()
}

type PortEdges = HashMap<(u64, u64), Vec<usize>>;

fn port_edges(
    existing: &[ExistingLine<'_>],
    owner: usize,
    work: &mut usize,
) -> Result<PortEdges, Error> {
    let mut ports = PortEdges::new();
    for (index, route) in existing.iter().enumerate() {
        step(work)?;
        let [from, to] = route.edge.owners;
        let port = if from == owner {
            route.points.first()
        } else if to == owner {
            route.points.last()
        } else {
            None
        };
        if let Some(&port) = port {
            ports.entry(point_key(port)).or_default().push(index);
        }
    }
    for edges in ports.values() {
        charge(work, edges.len())?;
    }
    Ok(ports)
}

struct LineSelector<'a, 'e> {
    routing: &'a FlatSearch<'a>,
    edge: &'e OrdinaryEdge,
    existing: &'e [ExistingLine<'e>],
    ports: [PortEdges; 2],
    orientations: [Orientation; 2],
}

impl LineSelector<'_, '_> {
    fn port_cost(&self, port: Point, end: usize, work: &mut usize) -> Result<Option<f64>, Error> {
        let Some(occupants) = self.ports[end].get(&point_key(port)) else {
            return Ok(Some(0.0));
        };
        let owners = self.edge.owners;
        for &index in occupants {
            step(work)?;
            let [from, to] = self.existing[index].edge.owners;
            if owners == [from, to] || owners == [to, from] {
                return Ok(None);
            }
        }
        for &index in occupants {
            step(work)?;
            let other = self.existing[index].edge;
            let other_end = if end == 0 {
                usize::from(other.owners[0] != owners[end])
            } else {
                usize::from(other.owners[1] == owners[end])
            };
            if other.arrowheads[other_end] != self.edge.arrowheads[end] {
                return Ok(None);
            }
        }
        let arrow = &self.edge.arrowheads[end];
        if end == 0 && (arrow.is_empty() || arrow == "none") {
            return Ok(Some(0.0));
        }
        let node = &self.routing.nodes[owners[end]];
        Ok(Some(side_length(node, self.orientations[end]) / 2.0))
    }

    fn route_cost(&self, pair: [Point; 2], work: &mut usize) -> Result<Option<f64>, Error> {
        let [from, to] = pair;
        for (index, node) in self.routing.nodes.iter().enumerate() {
            step(work)?;
            if !self.edge.owners.contains(&index) {
                step(work)?;
                if Rectangle::from(node).intersects(from, to) {
                    return Ok(None);
                }
            }
        }
        // The reference filters the candidate itself out of the existing set
        // for each pair. This interface receives those already distinct routes.
        charge(work, self.existing.len())?;
        let mut overlapping = Vec::new();
        for route in self.existing {
            step(work)?;
            for points in route.points.windows(2) {
                step(work)?;
                if axis_overlap(pair, [points[0], points[1]]) {
                    overlapping.push(route.edge);
                    break;
                }
            }
        }
        if !can_share(self.edge, overlapping.into_iter(), work)? {
            return Ok(None);
        }
        Ok(Some(estimated_route_cost(&pair, self.existing, work)?))
    }
}

fn estimated_route_cost(
    points: &[Point],
    others: &[ExistingLine<'_>],
    work: &mut usize,
) -> Result<f64, Error> {
    let mut length = 0.0;
    for segment in points.windows(2) {
        step(work)?;
        length += euclidean(segment[0], segment[1]);
    }
    let mut crossings = 0;
    for route in others {
        step(work)?;
        for current in points.windows(2) {
            for other in route.points.windows(2) {
                step(work)?;
                crossings += usize::from(non_shared_crossing(current, other));
            }
        }
    }
    Ok(length * cost_by_turns(points.len() - 2) + CROSSING_MULTIPLIER * crossings as f64)
}

fn cost_by_turns(mut turns: usize) -> f64 {
    let mut multiplier = 1.0;
    let mut power = TURN_PENALTY;
    // Go's integer exponent uses repeated squaring. Its power-of-two
    // normalization preserves the same products while 1.4^n is finite.
    while turns > 0 {
        if turns & 1 != 0 {
            multiplier *= power;
        }
        power *= power;
        turns >>= 1;
    }
    multiplier
}

fn axis_overlap([from, to]: [Point; 2], [other_from, other_to]: [Point; 2]) -> bool {
    let segment = Segment { from, to };
    let parallel = Segment {
        from: other_from,
        to: other_to,
    };
    for direction in [Direction::Horizontal, Direction::Vertical] {
        let aligned = match direction {
            Direction::Horizontal => {
                from.y == to.y && to.y == other_from.y && other_from.y == other_to.y
            }
            Direction::Vertical => {
                from.x == to.x && to.x == other_from.x && other_from.x == other_to.x
            }
        };
        if aligned {
            let (_, first, last) = segment.ordered(direction);
            let (_, other_first, other_last) = parallel.ordered(direction);
            if first <= other_last && other_first <= last {
                return true;
            }
        }
    }
    false
}

fn sharp_angle(from: Point, to: Point, orientation: Orientation) -> bool {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let raw = match orientation {
        Orientation::Top | Orientation::Bottom => dy.atan2(dx),
        Orientation::Left | Orientation::Right => dx.atan2(dy),
        _ => return false,
    };
    let mut angle = (180.0 * raw / PI).abs();
    if angle > 90.0 {
        angle = 180.0 - angle;
    }
    angle < MIN_STRAIGHT_EDGE_ANGLE
}

fn side_length(node: &NodeGeometry, orientation: Orientation) -> f64 {
    match orientation {
        Orientation::Top | Orientation::Bottom => node.width,
        Orientation::Left | Orientation::Right => node.height,
        _ => node.width.max(node.height),
    }
}

#[cfg(test)]
mod tests;
