// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placementcost/{edge_length,geometry,flow_continuity}.go
// and internal/layoutgraph/{graph,node}.go.

use std::collections::HashMap;

use super::{Work, preferred_direction};
use crate::{Error, Graph, NodeGeometry, Point};

mod symmetry;

#[cfg(test)]
mod tests;

const CONNECTED_NODE_GAP: f64 = 60.0;
const TURN_PENALTY_MULTIPLIER: f64 = 0.125;
const DIRECTION_DELTA_FACTOR: f64 = 0.25;
const SIDE_EDGE_SPACING: f64 = 40.0;

#[derive(Clone, Copy)]
pub(super) struct IncidentEdge {
    neighbor: usize,
    directed: bool,
    outgoing: bool,
    labeled: bool,
}

pub(super) struct EdgeCost {
    adjacency: Vec<Vec<IncidentEdge>>,
    direction: Option<i32>,
    cell_size: f64,
    turn_cost: f64,
}

impl EdgeCost {
    pub(super) fn new(
        graph: &Graph,
        nodes: &[NodeGeometry],
        cell_size: f64,
    ) -> Result<Self, Error> {
        let indices: HashMap<_, _> = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        let mut adjacency = vec![Vec::new(); nodes.len()];
        let mut maximum_length = CONNECTED_NODE_GAP;
        for edge in &graph.edges {
            let from = *indices
                .get(edge.source.as_str())
                .ok_or(Error::InvalidGraph)?;
            let to = *indices
                .get(edge.target.as_str())
                .ok_or(Error::InvalidGraph)?;
            let directed = edge.source_arrow != edge.target_arrow;
            let reversed = directed && edge.source_arrow;
            let labeled = !edge.label.is_empty();
            adjacency[from].push(IncidentEdge {
                neighbor: to,
                directed,
                outgoing: !reversed,
                labeled,
            });
            adjacency[to].push(IncidentEdge {
                neighbor: from,
                directed,
                outgoing: reversed,
                labeled,
            });
            maximum_length = maximum_length
                .max(Bounds::from(&nodes[from]).distance_to(Bounds::from(&nodes[to])));
        }
        let count = u32::try_from(graph.edges.len()).map_err(|_| Error::WorkLimit)?;
        // The sized phase caches this using the boxes immediately after transition and halves it.
        let turn_cost = TURN_PENALTY_MULTIPLIER * f64::from(count) * maximum_length / 2.0;
        Ok(Self {
            adjacency,
            direction: preferred_direction(graph)?,
            cell_size,
            turn_cost,
        })
    }

    pub(super) fn suppress_spokes(&mut self, node: usize, spokes: &[usize]) -> Vec<IncidentEdge> {
        let originals = std::mem::take(&mut self.adjacency[node]);
        self.adjacency[node].extend(
            originals
                .iter()
                .copied()
                .filter(|edge| !spokes.contains(&edge.neighbor)),
        );
        originals
    }

    pub(super) fn restore_incident_edges(&mut self, node: usize, edges: Vec<IncidentEdge>) {
        self.adjacency[node] = edges;
    }

    pub(super) fn turn_cost(&self) -> f64 {
        self.turn_cost
    }

    pub(super) fn score(
        &self,
        node: usize,
        nodes: &[NodeGeometry],
        work: &mut Work,
    ) -> Result<f64, Error> {
        let incident = &self.adjacency[node];
        work.add(incident.len().saturating_mul(nodes.len() + incident.len()))?;
        let bounds = Bounds::from(&nodes[node]);
        let mut labeled_counts: HashMap<_, (u32, u32)> = HashMap::new();
        let mut repeated_label_neighbor = None;
        for edge in incident.iter().filter(|edge| edge.labeled) {
            let (count, outgoing_count) = labeled_counts.entry(edge.neighbor).or_default();
            *count += 1;
            *outgoing_count += u32::from(edge.outgoing);
            if *count > 2 {
                repeated_label_neighbor = Some(edge.neighbor);
            }
        }
        let (desired, direction_factor) = match (self.direction, repeated_label_neighbor) {
            (Some(direction), _) => (direction, 6.0),
            (None, Some(neighbor)) => {
                let (count, outgoing) = labeled_counts[&neighbor];
                (if outgoing > count / 2 { -2 } else { 2 }, 10.0)
            }
            (None, None) => (3, 0.3),
        };
        let mut total = 0.0;
        for &edge in incident {
            let adjacent = Bounds::from(&nodes[edge.neighbor]);
            let Some(relative) = bounds.direction_relative_to(adjacent) else {
                continue;
            };
            let obstacles = || {
                nodes
                    .iter()
                    .enumerate()
                    .filter(|&(index, _)| index != node && index != edge.neighbor)
                    .map(|(_, body)| Bounds::from(body))
            };
            let mut distance = bounds.distance_to(adjacent);
            if relative.abs() % 2 == 1 {
                let a = bounds.center();
                let b = adjacent.center();
                let corner_a = Point { x: a.x, y: b.y };
                let corner_b = Point { x: b.x, y: a.y };
                distance = bounds.distance_to(Bounds::point(corner_a))
                    + adjacent.distance_to(Bounds::point(corner_a))
                    + self.turn_cost;
                let mut blocked_a = false;
                let mut blocked_b = false;
                for obstacle in obstacles() {
                    blocked_a |= obstacle.blocks(a, corner_a) || obstacle.blocks(b, corner_a);
                    blocked_b |= obstacle.blocks(a, corner_b) || obstacle.blocks(b, corner_b);
                    if blocked_a && blocked_b {
                        distance += self.turn_cost;
                        break;
                    }
                }
            } else {
                let horizontal = relative.abs() == 2;
                let a = if horizontal {
                    bounds.transposed()
                } else {
                    bounds
                };
                let b = if horizontal {
                    adjacent.transposed()
                } else {
                    adjacent
                };
                let (upper, lower) = if a.top < b.top { (a, b) } else { (b, a) };
                let blocked_turns = orthogonal_detour_turns(
                    upper,
                    lower,
                    obstacles().map(|obstacle| {
                        if horizontal {
                            obstacle.transposed()
                        } else {
                            obstacle
                        }
                    }),
                );
                let x = (bounds.center().x - adjacent.center().x).abs()
                    / (bounds.width() + adjacent.width());
                let y = (bounds.center().y - adjacent.center().y).abs()
                    / (bounds.height() + adjacent.height());
                distance += x.min(y) / 20.0;
                distance += self.turn_cost * f64::from(blocked_turns);
            }
            if edge.directed || self.direction.is_some() {
                let compass = if edge.outgoing {
                    opposite(relative)
                } else {
                    relative
                };
                let mut delta = f64::from(((compass - desired + 12).rem_euclid(8) - 4).abs());
                if !edge.directed {
                    let axis = (compass + 4).rem_euclid(4) - (desired + 4).rem_euclid(4);
                    delta = 0.1 * delta + 0.9 * f64::from(if axis == 3 { 1 } else { axis.abs() });
                }
                distance += direction_factor * delta * self.cell_size * DIRECTION_DELTA_FACTOR;
            }
            if edge.labeled && desired.rem_euclid(4) != 0 && relative.abs() != 2 {
                let (count, _) = labeled_counts[&edge.neighbor];
                if count > 2 {
                    distance += f64::from(count) * self.turn_cost;
                }
            }
            total += distance;
        }
        Ok(total + self.flow_continuity(node, nodes))
    }

    fn flow_continuity(&self, node: usize, nodes: &[NodeGeometry]) -> f64 {
        const INCOMING: u8 = 1;
        const OUTGOING: u8 = 2;
        let incident = &self.adjacency[node];
        if !(2..=8).contains(&incident.len()) {
            return 0.0;
        }
        struct Ray {
            neighbor: usize,
            directions: u8,
            x: f64,
            y: f64,
        }
        let center = Bounds::from(&nodes[node]).center();
        let mut rays: Vec<Ray> = Vec::with_capacity(incident.len());
        for edge in incident.iter().filter(|edge| edge.directed) {
            let direction = if edge.outgoing { OUTGOING } else { INCOMING };
            if let Some(ray) = rays.iter_mut().find(|ray| ray.neighbor == edge.neighbor) {
                ray.directions |= direction;
                continue;
            }
            let adjacent = Bounds::from(&nodes[edge.neighbor]).center();
            let x = adjacent.x - center.x;
            let y = adjacent.y - center.y;
            let largest = x.abs().max(y.abs());
            if largest == 0.0 {
                continue;
            }
            let factor = x.abs().min(y.abs()) / largest;
            let length = largest * (1.0 + factor * factor).sqrt();
            rays.push(Ray {
                neighbor: edge.neighbor,
                directions: direction,
                x: x / length,
                y: y / length,
            });
        }
        let mut spine = f64::INFINITY;
        let mut branch_sum = 0.0;
        let mut branches = 0;
        for (i, a) in rays.iter().enumerate() {
            for b in &rays[i + 1..] {
                let dot = (a.x * b.x + a.y * b.y).clamp(-1.0, 1.0);
                if a.directions & INCOMING != 0 && b.directions & OUTGOING != 0
                    || a.directions & OUTGOING != 0 && b.directions & INCOMING != 0
                {
                    spine = spine.min(1.0 + dot);
                }
                if a.directions & b.directions != 0 {
                    branch_sum += (2.0 * dot - 1.0).max(0.0);
                    branches += 1;
                }
            }
        }
        let mut cost = if spine.is_finite() { spine } else { 0.0 };
        if branches > 0 {
            cost += branch_sum / f64::from(branches);
        }
        self.turn_cost * cost
    }
}

fn opposite(direction: i32) -> i32 {
    (direction + 7).rem_euclid(8) - 3
}

#[derive(Clone, Copy)]
pub(super) struct Bounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl From<&NodeGeometry> for Bounds {
    fn from(body: &NodeGeometry) -> Self {
        Self {
            left: body.x,
            top: body.y,
            right: body.x + body.width,
            bottom: body.y + body.height,
        }
    }
}

impl Bounds {
    fn point(point: Point) -> Self {
        Self {
            left: point.x,
            top: point.y,
            right: point.x,
            bottom: point.y,
        }
    }

    fn width(self) -> f64 {
        self.right - self.left
    }

    fn height(self) -> f64 {
        self.bottom - self.top
    }

    fn center(self) -> Point {
        Point {
            x: self.left + self.width() / 2.0,
            y: self.top + self.height() / 2.0,
        }
    }

    fn transposed(self) -> Self {
        Self {
            left: self.top,
            top: self.left,
            right: self.bottom,
            bottom: self.right,
        }
    }

    pub(super) fn distance_to(self, other: Self) -> f64 {
        let x = (other.left - self.right)
            .max(self.left - other.right)
            .max(0.0);
        let y = (other.top - self.bottom)
            .max(self.top - other.bottom)
            .max(0.0);
        if x == 0.0 {
            return y;
        }
        if y == 0.0 {
            return x;
        }
        (x * x + y * y).sqrt()
    }

    pub(super) fn direction_relative_to(self, other: Self) -> Option<i32> {
        let x = i32::from(self.left > other.right) - i32::from(self.right < other.left);
        let y = i32::from(self.top > other.bottom) - i32::from(self.bottom < other.top);
        match (x, y) {
            (0, 0) => None,
            (-1, -1) => Some(-1),
            (0, -1) => Some(0),
            (1, -1) => Some(1),
            (1, 0) => Some(2),
            (1, 1) => Some(3),
            (0, 1) => Some(4),
            (-1, 1) => Some(-3),
            (-1, 0) => Some(-2),
            _ => unreachable!(),
        }
    }

    fn blocks(self, a: Point, b: Point) -> bool {
        fn overlaps(floor: f64, ceiling: f64, a: f64, b: f64) -> bool {
            floor <= a && a <= ceiling
                || floor <= b && b <= ceiling
                || a.min(b) < floor && ceiling < a.max(b)
        }
        if a.x == b.x {
            return self.left <= a.x
                && a.x <= self.right
                && overlaps(self.top, self.bottom, a.y, b.y);
        }
        if a.y == b.y {
            return self.top <= a.y
                && a.y <= self.bottom
                && overlaps(self.left, self.right, a.x, b.x);
        }
        let contains = |p: Point| {
            self.left <= p.x && p.x <= self.right && self.top <= p.y && p.y <= self.bottom
        };
        if contains(a) || contains(b) {
            return true;
        }
        let cross_x = [
            (self.left - a.x) / (b.x - a.x),
            (self.right - a.x) / (b.x - a.x),
        ];
        let cross_y = [
            (self.top - a.y) / (b.y - a.y),
            (self.bottom - a.y) / (b.y - a.y),
        ];
        let enter = cross_x[0]
            .min(cross_x[1])
            .max(cross_y[0].min(cross_y[1]))
            .max(0.0);
        let exit = cross_x[0]
            .max(cross_x[1])
            .min(cross_y[0].max(cross_y[1]))
            .min(1.0);
        enter < exit
    }
}

fn orthogonal_detour_turns(
    upper: Bounds,
    lower: Bounds,
    obstacles: impl Iterator<Item = Bounds>,
) -> u32 {
    let floor = upper.left.max(lower.left);
    let ceiling = upper.right.min(lower.right);
    let middle = (floor + ceiling) / 2.0;
    let in_line = (upper.left - lower.left).abs() <= SIDE_EDGE_SPACING;
    let out_line = (upper.right - lower.right).abs() <= SIDE_EDGE_SPACING;
    let semi_diagonal = !in_line || !out_line;
    let mut lower_route_blocked = in_line;
    let mut upper_route_blocked = !in_line && out_line;
    let mut straight_blocked = false;
    for obstacle in obstacles {
        straight_blocked |= obstacle.blocks(
            Point {
                x: middle,
                y: upper.bottom,
            },
            Point {
                x: middle,
                y: lower.top,
            },
        );
        if !straight_blocked {
            continue;
        }
        if !semi_diagonal {
            return 2;
        }
        if !lower_route_blocked {
            let min = upper.left.min(lower.left);
            let cross = min + (floor - min).abs() / 2.0;
            let far = if upper.left < lower.left {
                lower
            } else {
                upper
            };
            lower_route_blocked = alternate_blocked(obstacle, upper, lower, far, cross);
        }
        if !upper_route_blocked {
            let max = upper.right.max(lower.right);
            let cross = max - (ceiling - max).abs() / 2.0;
            let near = if upper.right > lower.right {
                lower
            } else {
                upper
            };
            upper_route_blocked = alternate_blocked(obstacle, upper, lower, near, cross);
        }
        if lower_route_blocked && upper_route_blocked {
            return 2;
        }
    }
    u32::from(straight_blocked)
}

fn alternate_blocked(
    obstacle: Bounds,
    upper: Bounds,
    lower: Bounds,
    endpoint: Bounds,
    cross: f64,
) -> bool {
    let from = Point {
        x: cross,
        y: upper.center().y,
    };
    let to = Point {
        x: cross,
        y: lower.center().y,
    };
    let center = endpoint.center();
    obstacle.blocks(from, to)
        || obstacle.blocks(
            center,
            Point {
                x: cross,
                y: center.y,
            },
        )
}
