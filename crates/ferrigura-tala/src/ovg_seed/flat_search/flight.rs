// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{slingshot,cluster_route_guard}.go.

use std::collections::HashMap;

use crate::{Error, NodeGeometry, Point};

use super::super::grid::{charge, charge_sort};
use super::super::point_key;
use super::super::search::{SearchRoute, TurnAxis, ideal_turn_axes};
use super::super::slingshot::{Diagonal, Flight, Orientation, step};
use super::{FlatSearch, OrdinaryEdge, Rectangle, RouteHistory, euclidean};

impl FlatSearch<'_> {
    /// Attempts TALA's L and S slingshot paths before weighted search.
    /// Node IDs and all edges must retain their pinned graph identities so
    /// the reference's launch preference sees the same ties and neighbors.
    ///
    /// # Errors
    /// Returns invalid graph for inconsistent edges, nodes, or visibility
    /// state, and a work limit if candidate evaluation cannot finish.
    pub fn slingshot(
        &self,
        history: &mut RouteHistory<'_>,
        edge: &OrdinaryEdge,
        node_ids: &[i64],
        edges: &[OrdinaryEdge],
        work: &mut usize,
    ) -> Result<Option<SearchRoute>, Error> {
        if !std::ptr::eq(history.graph(), self.graph) || node_ids.len() != self.nodes.len() {
            return Err(Error::InvalidGraph);
        }
        step(work)?;
        let [source, target] = edge.owners;
        let from = self.nodes.get(source).ok_or(Error::InvalidGraph)?;
        let to = self.nodes.get(target).ok_or(Error::InvalidGraph)?;
        let Some(diagonal) = Orientation::between(from, to).diagonal() else {
            return Ok(None);
        };
        let source_neighbors = self.neighbor_orientations(edges, source, target, work)?;
        let target_neighbors = self.neighbor_orientations(edges, target, source, work)?;
        let (vertical, strong) = diagonal.prefer_launching_vertically(
            node_ids[source],
            node_ids[target],
            source_neighbors,
            target_neighbors,
        );
        let mut search = FlightSearch {
            flat: self,
            history,
            edge,
            diagonal,
            vertical,
            strong,
            occupied: HashMap::new(),
            crossed: HashMap::new(),
        };
        if let Some(route) = search.l_shaped(work)? {
            return Ok(Some(route));
        }
        search.s_shaped(work)
    }

    fn neighbor_orientations(
        &self,
        edges: &[OrdinaryEdge],
        owner: usize,
        opposite: usize,
        work: &mut usize,
    ) -> Result<Vec<Orientation>, Error> {
        let mut orientations = Vec::new();
        let node = &self.nodes[owner];
        for edge in edges {
            let [from, to] = edge.owners;
            let neighbor = if from == owner {
                to
            } else if to == owner {
                from
            } else {
                continue;
            };
            step(work)?;
            if neighbor != opposite {
                let neighbor = self.nodes.get(neighbor).ok_or(Error::InvalidGraph)?;
                orientations.push(Orientation::between(neighbor, node));
            }
        }
        Ok(orientations)
    }
}

struct FlightSearch<'a, 'h, 'g> {
    flat: &'a FlatSearch<'a>,
    history: &'h mut RouteHistory<'g>,
    edge: &'a OrdinaryEdge,
    diagonal: Diagonal,
    vertical: bool,
    strong: bool,
    occupied: HashMap<[usize; 2], bool>,
    crossed: HashMap<[usize; 2], bool>,
}

impl FlightSearch<'_, '_, '_> {
    fn point(&self, vertex: usize) -> Point {
        self.flat.adjacency.vertex(vertex).point
    }

    fn centers(&self) -> [usize; 2] {
        self.edge
            .owners
            .map(|owner| self.flat.graph.centers_by_node[owner])
    }

    fn follow(
        &self,
        current: usize,
        flight: Flight,
        work: &mut usize,
    ) -> Result<Option<usize>, Error> {
        step(work)?;
        let from = self.point(current);
        for incident in self.flat.adjacency.incidents(current) {
            step(work)?;
            if flight.on_flight_plan(from, self.point(incident.neighbor)) {
                return Ok(Some(incident.neighbor));
            }
        }
        Ok(None)
    }

    fn is_occupied(&mut self, pair: [usize; 2], work: &mut usize) -> Result<bool, Error> {
        if let Some(&occupied) = self.occupied.get(&pair) {
            return Ok(occupied);
        }
        let occupied = self.history.is_occupied(self.edge, pair, work)?;
        self.occupied.insert(pair, occupied);
        Ok(occupied)
    }

    fn crossing_cost(&mut self, pair: [usize; 2], work: &mut usize) -> Result<f64, Error> {
        let crossed = if let Some(&crossed) = self.crossed.get(&pair) {
            crossed
        } else {
            let crossed = self.history.has_crossing(pair, work)?;
            self.crossed.insert(pair, crossed);
            crossed
        };
        Ok(if crossed {
            self.flat.weights.crossing
        } else {
            0.0
        })
    }

    fn intersects_obstacle(&self, pair: [usize; 2], work: &mut usize) -> Result<bool, Error> {
        let [from, to] = pair.map(|vertex| self.point(vertex));
        for (index, node) in self.flat.nodes.iter().enumerate() {
            step(work)?;
            if !self.edge.owners.contains(&index) {
                step(work)?;
                if Rectangle::from(node).intersects(from, to) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn ports(
        &self,
        vertical: bool,
        l_shape: bool,
        work: &mut usize,
    ) -> Result<[Vec<usize>; 2], Error> {
        let [source, target] = self.edge.owners;
        charge(work, self.flat.graph.ports_by_node[source].len())?;
        charge(work, self.flat.graph.ports_by_node[target].len())?;
        Ok([
            self.diagonal
                .flight(vertical)
                .launch_ports(&self.flat.adjacency, source),
            self.diagonal
                .flight(if l_shape { vertical } else { !vertical })
                .landing_ports(&self.flat.adjacency, target),
        ])
    }

    fn launch_bias(&self, vertical: bool) -> f64 {
        if self.strong && self.vertical != vertical {
            self.flat.weights.crossing / 2.0
        } else {
            0.0
        }
    }

    fn l_shaped(&mut self, work: &mut usize) -> Result<Option<SearchRoute>, Error> {
        step(work)?;
        let [source, target] = self.centers();
        let target_node = &self.flat.nodes[self.edge.owners[1]];
        let mut best = None;
        let mut shortest = f64::INFINITY;
        for vertical in [self.vertical, !self.vertical] {
            step(work)?;
            let flight = self.diagonal.flight(vertical);
            let [launching, landing] = self.ports(vertical, true, work)?;
            for start in launching {
                step(work)?;
                let mut current = start;
                while let Some(anchor) = self.follow(current, flight, work)? {
                    let anchor_point = self.point(anchor);
                    if flight.overshot(anchor_point, target_node, true)
                        || self.is_occupied([current, anchor], work)?
                    {
                        break;
                    }
                    current = anchor;
                    if flight.undershot(anchor_point, target_node) {
                        continue;
                    }
                    for &end in &landing {
                        step(work)?;
                        if axis(vertical, anchor_point) != axis(vertical, self.point(end)) {
                            continue;
                        }
                        let Some(cost) = self.l_cost([start, anchor, end], vertical, work)? else {
                            continue;
                        };
                        if cost >= shortest {
                            continue;
                        }
                        let Some(vertices) =
                            self.fill_path([source, target], &[start, anchor, end], 1, work)?
                        else {
                            continue;
                        };
                        if self.history.is_duplicate(&vertices, work)? {
                            continue;
                        }
                        shortest = cost;
                        best = Some(SearchRoute {
                            vertices,
                            cost: cost + self.flat.weights.turn,
                        });
                    }
                }
            }
        }
        Ok(best)
    }

    fn l_cost(
        &mut self,
        [start, anchor, end]: [usize; 3],
        vertical: bool,
        work: &mut usize,
    ) -> Result<Option<f64>, Error> {
        if self.intersects_obstacle([start, anchor], work)?
            || self.intersects_obstacle([end, anchor], work)?
        {
            return Ok(None);
        }
        let midpoint = self.point(anchor);
        let mut cost =
            euclidean(self.point(start), midpoint) + euclidean(self.point(end), midpoint);
        if cost == 0.0 {
            return Ok(None);
        }
        cost += self.crossing_cost([start, anchor], work)?;
        cost += self.crossing_cost([anchor, end], work)?;
        cost += self.launch_bias(vertical);
        Ok(Some(cost))
    }

    fn s_shaped(&mut self, work: &mut usize) -> Result<Option<SearchRoute>, Error> {
        step(work)?;
        let centers = self.centers();
        let [source, target] = self.edge.owners.map(|owner| &self.flat.nodes[owner]);
        let axes = ideal_turn_axes(source, target);
        let [TurnAxis::X(x), TurnAxis::Y(y)] = axes.as_slice() else {
            return Err(Error::InvalidGraph);
        };
        let mut best = None;
        let mut shortest = f64::INFINITY;
        for vertical in [self.vertical, !self.vertical] {
            step(work)?;
            let [launching, landing] = self.ports(vertical, false, work)?;
            let mut launches =
                self.anchors(launching, self.diagonal.flight(vertical), target, work)?;
            let landings = self.anchors(
                landing,
                self.diagonal.opposite().flight(vertical),
                source,
                work,
            )?;
            let ideal = if vertical { *y } else { *x };
            charge_sort(work, launches.order.len())?;
            launches.order.sort_by(|&first, &second| {
                let first = (axis(vertical, self.point(first)) - ideal).abs();
                let second = (axis(vertical, self.point(second)) - ideal).abs();
                first.partial_cmp(&second).unwrap()
            });
            let mut lands_by_axis = HashMap::<_, Vec<_>>::new();
            for &vertex in &landings.order {
                step(work)?;
                lands_by_axis
                    .entry(axis_key(vertical, self.point(vertex)))
                    .or_default()
                    .push(vertex);
            }
            for from in launches.order {
                step(work)?;
                let line = axis_key(vertical, self.point(from));
                let Some(anchors) = lands_by_axis.get(&line) else {
                    continue;
                };
                for &to in anchors {
                    step(work)?;
                    let via = [launches.by_vertex[&from], from, to, landings.by_vertex[&to]];
                    let Some(cost) = self.s_cost(via, vertical, work)? else {
                        continue;
                    };
                    if cost >= shortest {
                        continue;
                    }
                    let Some(vertices) = self.fill_path(centers, &via, 1, work)? else {
                        continue;
                    };
                    shortest = cost;
                    best = Some(SearchRoute {
                        vertices,
                        cost: cost + self.flat.weights.turn * 2.0,
                    });
                }
            }
        }
        Ok(best)
    }

    fn anchors(
        &mut self,
        ports: Vec<usize>,
        flight: Flight,
        boundary: &NodeGeometry,
        work: &mut usize,
    ) -> Result<Anchors, Error> {
        let mut anchors = Anchors::default();
        for port in ports {
            step(work)?;
            let mut current = port;
            while let Some(next) = self.follow(current, flight, work)? {
                if flight.overshot(self.point(next), boundary, false)
                    || self.is_occupied([current, next], work)?
                {
                    break;
                }
                anchors.by_vertex.insert(next, port);
                anchors.order.push(next);
                current = next;
            }
        }
        Ok(anchors)
    }

    fn s_cost(
        &mut self,
        vertices: [usize; 4],
        vertical: bool,
        work: &mut usize,
    ) -> Result<Option<f64>, Error> {
        let mut cost = 0.0;
        for pair in vertices.windows(2) {
            step(work)?;
            let [from, to] = [pair[0], pair[1]];
            if self.intersects_obstacle([from, to], work)? {
                return Ok(None);
            }
            cost += euclidean(self.point(from), self.point(to));
            cost += self.crossing_cost([from, to], work)?;
        }
        Ok(Some(cost + self.launch_bias(vertical)))
    }

    fn fill_path(
        &mut self,
        [source, target]: [usize; 2],
        vias: &[usize],
        sharing_leg: usize,
        work: &mut usize,
    ) -> Result<Option<Vec<usize>>, Error> {
        let mut path = vec![source];
        for (leg, pair) in vias.windows(2).enumerate() {
            let [from, to] = [pair[0], pair[1]];
            let Some(vertices) = self.flat.adjacency.fill_path(from, to, work)? else {
                return Ok(None);
            };
            if leg == sharing_leg {
                // Go omits the last incident edge into this leg's endpoint.
                for transition in vertices.windows(2) {
                    step(work)?;
                    if self.is_occupied([transition[0], transition[1]], work)? {
                        return Ok(None);
                    }
                }
            }
            path.extend(vertices);
        }
        path.push(*vias.last().unwrap());
        path.push(target);
        Ok(Some(path))
    }
}

#[derive(Default)]
struct Anchors {
    order: Vec<usize>,
    by_vertex: HashMap<usize, usize>,
}

fn axis(vertical: bool, point: Point) -> f64 {
    if vertical { point.y } else { point.x }
}

fn axis_key(vertical: bool, point: Point) -> u64 {
    point_key(Point {
        x: axis(vertical, point),
        y: 0.0,
    })
    .0
}
