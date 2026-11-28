// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{swap_ports,geometry,tuning}.go.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::{Error, NodeGeometry, Point};

use super::super::Bounds;
use super::super::grid::{charge, charge_sort};
use super::super::point_key;
use super::super::slingshot::step;
use super::super::tunnels::{Axis, tunnel_ranges};
use super::flavors::segment_intersection;
use super::{OrdinaryEdge, Rectangle};

const PATH_NODE_PROXIMITY_FLOOR: f64 = 5.0;
const PORT_OBSTACLE_PADDING: f64 = 2.0 * PATH_NODE_PROXIMITY_FLOOR;

#[derive(Clone, Copy)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    const ALL: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    fn from_segment(port: Point, next: Point) -> Self {
        if port.x < next.x {
            Self::Right
        } else if port.x > next.x {
            Self::Left
        } else if port.y < next.y {
            Self::Bottom
        } else {
            Self::Top
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Top => 0,
            Self::Bottom => 1,
            Self::Left => 2,
            Self::Right => 3,
        }
    }

    fn vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    fn parallel(self, point: Point) -> f64 {
        if self.vertical() { point.x } else { point.y }
    }

    fn with_parallel(self, point: Point, value: f64) -> Point {
        if self.vertical() {
            Point { x: value, ..point }
        } else {
            Point { y: value, ..point }
        }
    }

    fn intersects(self, [p1, p2, p3]: [Point; 3], [q1, q2, q3]: [Point; 3]) -> bool {
        match self {
            Self::Top => p2.y < q2.y && q3.x < p1.x || q2.y < p2.y && p3.x > q1.x,
            Self::Bottom => p2.y > q2.y && q3.x < p1.x || q2.y > p2.y && p3.x > q1.x,
            Self::Left => p2.x < q2.x && q3.y < p1.y || q2.x < p2.x && p3.y > q1.y,
            Self::Right => p2.x > q2.x && q3.y < p1.y || q2.x > p2.x && p3.y > q1.y,
        }
    }
}

#[derive(Clone, Copy)]
struct Endpoint {
    edge: usize,
    target: bool,
}

impl Endpoint {
    fn indices(self, routes: &[Vec<Point>]) -> [usize; 3] {
        if self.target {
            let last = routes[self.edge].len() - 1;
            [last, last - 1, last - 2]
        } else {
            [0, 1, 2]
        }
    }

    fn first_three(self, routes: &[Vec<Point>]) -> [Point; 3] {
        self.indices(routes).map(|index| routes[self.edge][index])
    }

    fn arrow(self, edges: &[OrdinaryEdge]) -> &str {
        &edges[self.edge].arrowheads[usize::from(self.target)]
    }
}

#[derive(Default)]
struct PortGroups {
    by_side: [Vec<Endpoint>; 4],
    usage: HashMap<(u64, u64), usize>,
}

impl PortGroups {
    fn at(
        edges: &[OrdinaryEdge],
        paths: &[Vec<Point>],
        owner: usize,
        work: &mut usize,
    ) -> Result<Self, Error> {
        let mut ports = Self::default();
        for (route, edge) in edges.iter().enumerate() {
            let target = edge.owners[1] == owner;
            if !target && edge.owners[0] != owner {
                continue;
            }
            step(work)?;
            if edge.owners[0] == edge.owners[1] || paths[route].len() < 3 {
                continue;
            }
            let endpoint = Endpoint {
                edge: route,
                target,
            };
            let [first, next, _] = endpoint.first_three(paths);
            let side = Side::from_segment(first, next);
            ports.by_side[side.index()].push(endpoint);
            *ports.usage.entry(point_key(first)).or_default() += 1;
        }
        Ok(ports)
    }

    fn reorder_same_side(
        &mut self,
        context: &PortSwap<'_>,
        owner: usize,
        paths: &mut [Vec<Point>],
        work: &mut usize,
    ) -> Result<bool, Error> {
        let mut swapped = false;
        for side in Side::ALL {
            let endpoints = &mut self.by_side[side.index()];
            if endpoints.is_empty() {
                continue;
            }
            step(work)?;
            charge_sort(work, endpoints.len())?;
            endpoints.sort_by(|&one, &two| {
                let [p1, _, p3] = one.first_three(paths).map(|point| side.parallel(point));
                let [q1, _, q3] = two.first_three(paths).map(|point| side.parallel(point));
                let compare = |a: f64, b: f64| a.partial_cmp(&b).unwrap_or(Ordering::Equal);
                if p1 == q1 {
                    compare(p3, q3)
                } else {
                    compare(p1, q1)
                }
            });
            for index in 0..endpoints.len().saturating_sub(1) {
                step(work)?;
                let [first, second] = [endpoints[index], endpoints[index + 1]];
                let p = first.first_three(paths);
                let q = second.first_three(paths);
                if !side.intersects(p, q)
                    || !port_compatible(&self.usage, context.edges, first, second, p[0], q[0])
                {
                    continue;
                }
                let p_next = [
                    side.with_parallel(p[0], side.parallel(q[0])),
                    side.with_parallel(p[1], side.parallel(q[1])),
                ];
                let q_next = [
                    side.with_parallel(q[0], side.parallel(p[0])),
                    side.with_parallel(q[1], side.parallel(p[1])),
                ];
                if !context.clear_of_nodes(&[owner], [p_next, q_next], work)? {
                    continue;
                }
                for (endpoint, next) in [(first, p_next), (second, q_next)] {
                    let [one, two, _] = endpoint.indices(paths);
                    let path = &mut paths[endpoint.edge];
                    path[one] = next[0];
                    path[two] = next[1];
                }
                endpoints.swap(index, index + 1);
                swapped = true;
            }
        }
        Ok(swapped)
    }

    fn reorder_adjacent_sides(
        &self,
        context: &PortSwap<'_>,
        bounds: &[Bounds],
        paths: &mut [Vec<Point>],
        work: &mut usize,
    ) -> Result<bool, Error> {
        let mut swapped = false;
        let vertical = self.by_side[Side::Top.index()]
            .iter()
            .chain(&self.by_side[Side::Bottom.index()]);
        let horizontal = self.by_side[Side::Left.index()]
            .iter()
            .chain(&self.by_side[Side::Right.index()]);
        for &first in vertical {
            step(work)?;
            for &second in horizontal.clone() {
                step(work)?;
                if paths[first.edge].len() <= 2 {
                    break;
                }
                if paths[second.edge].len() <= 2 {
                    continue;
                }
                let [a1, a2, a3] = first.first_three(paths);
                let [b1, b2, b3] = second.first_three(paths);
                if !port_compatible(&self.usage, context.edges, first, second, a1, b1) {
                    continue;
                }
                let Some(intersection) = segment_intersection(a2, a3, b2, b3) else {
                    continue;
                };
                let previous = [first, second].map(|endpoint| paths[endpoint.edge].clone());
                charge(work, previous[0].len() + previous[1].len())?;
                replace_start(first, paths, [b1, b2], intersection, work)?;
                replace_start(second, paths, [a1, a2], intersection, work)?;
                let first_refined = context.refine(first.edge, bounds, paths, work)?;
                let second_refined = context.refine(second.edge, bounds, paths, work)?;
                if first_refined || second_refined {
                    swapped = true;
                } else {
                    let [first_before, second_before] = previous;
                    paths[first.edge] = first_before;
                    paths[second.edge] = second_before;
                }
            }
        }
        Ok(swapped)
    }
}

fn replace_start(
    endpoint: Endpoint,
    paths: &mut [Vec<Point>],
    [port, mut elbow]: [Point; 2],
    mut intersection: Point,
    work: &mut usize,
) -> Result<(), Error> {
    let [first, second, _] = endpoint.indices(paths);
    let path = &mut paths[endpoint.edge];
    charge(work, path.len())?;
    let offset = PATH_NODE_PROXIMITY_FLOOR / 2.0;
    if intersection.x == elbow.x {
        intersection.x += offset;
        elbow.x += offset;
    } else {
        intersection.y -= offset;
        elbow.y -= offset;
    }
    path[first] = port;
    path[second] = elbow;
    let insertion = if endpoint.target { path.len() - 2 } else { 2 };
    path.insert(insertion, intersection);
    Ok(())
}

fn port_compatible(
    usage: &HashMap<(u64, u64), usize>,
    edges: &[OrdinaryEdge],
    first: Endpoint,
    second: Endpoint,
    from: Point,
    to: Point,
) -> bool {
    usage.get(&point_key(from)) == Some(&1) && usage.get(&point_key(to)) == Some(&1)
        || first.arrow(edges) == second.arrow(edges)
}

struct PortSwap<'a> {
    nodes: &'a [NodeGeometry],
    edges: &'a [OrdinaryEdge],
    parents: &'a [Option<usize>],
}

impl PortSwap<'_> {
    fn valid(&self, paths: &[Vec<Point>]) -> bool {
        self.nodes.len() == self.parents.len()
            && self.edges.len() == paths.len()
            && self
                .edges
                .iter()
                .flat_map(|edge| edge.owners)
                .all(|node| node < self.nodes.len())
            && self
                .parents
                .iter()
                .flatten()
                .all(|&node| node < self.nodes.len())
    }

    fn clear_of_nodes(
        &self,
        exclusions: &[usize],
        paths: [[Point; 2]; 2],
        work: &mut usize,
    ) -> Result<bool, Error> {
        'nodes: for (index, node) in self.nodes.iter().enumerate() {
            step(work)?;
            for &endpoint in exclusions {
                step(work)?;
                let mut ancestor = Some(endpoint);
                for _ in 0..self.nodes.len() {
                    let Some(current) = ancestor else { break };
                    if current == index {
                        continue 'nodes;
                    }
                    ancestor = self.parents[current];
                }
                if ancestor.is_some() {
                    return Err(Error::InvalidGraph);
                }
            }
            let padded = Rectangle {
                x: node.x - PORT_OBSTACLE_PADDING,
                y: node.y - PORT_OBSTACLE_PADDING,
                width: node.width + PORT_OBSTACLE_PADDING * 2.0,
                height: node.height + PORT_OBSTACLE_PADDING * 2.0,
            };
            if paths
                .into_iter()
                .any(|[from, to]| padded.intersects(from, to))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn refine(
        &self,
        edge: usize,
        bounds: &[Bounds],
        paths: &mut [Vec<Point>],
        work: &mut usize,
    ) -> Result<bool, Error> {
        step(work)?;
        let owners = self.edges[edge].owners;
        let path = &paths[edge];
        if path.len() <= 2 || owners[0] == owners[1] {
            return Ok(false);
        }
        let occupied = self.occupied_ports(owners, paths, work)?;
        if is_s_shaped(path) || is_l_shaped(path) {
            let Some(straight) = self.straight_refinement(owners, bounds, path, &occupied, work)?
            else {
                return Ok(false);
            };
            paths[edge] = straight.to_vec();
            return Ok(true);
        }
        if is_u_shaped(path) || path.len() < 4 || !is_s_shaped(&path[..4]) {
            return Ok(false);
        }

        let bend = if path[0].x == path[1].x {
            Point {
                x: path[1].x,
                y: path[3].y,
            }
        } else {
            Point {
                x: path[3].x,
                y: path[1].y,
            }
        };
        if !self.clear_of_nodes(&owners, [[path[0], bend], [bend, path[3]]], work)? {
            return Ok(false);
        }
        // Go examines three removal indices for retained points, and one,
        // two and three respectively for the three removed points.
        charge(work, path.len() + 3 * (path.len() - 3) + 6)?;
        charge(work, path.len() - 3)?;
        paths[edge].splice(1..4, [bend]);
        Ok(true)
    }

    fn occupied_ports(
        &self,
        owners: [usize; 2],
        paths: &[Vec<Point>],
        work: &mut usize,
    ) -> Result<HashSet<(u64, u64)>, Error> {
        let mut ports = HashSet::new();
        for owner in owners {
            for (edge, path) in self.edges.iter().zip(paths) {
                let point = if edge.owners[0] == owner {
                    path.first()
                } else if edge.owners[1] == owner {
                    path.last()
                } else {
                    continue;
                };
                step(work)?;
                ports.insert(point_key(*point.ok_or(Error::InvalidGraph)?));
            }
        }
        Ok(ports)
    }

    fn straight_refinement(
        &self,
        [from, to]: [usize; 2],
        bounds: &[Bounds],
        path: &[Point],
        occupied: &HashSet<(u64, u64)>,
        work: &mut usize,
    ) -> Result<Option<[Point; 2]>, Error> {
        let mut excluded = vec![false; self.nodes.len()];
        for (node, avoid) in excluded.iter_mut().enumerate() {
            step(work)?;
            if node == from || node == to {
                continue;
            }
            let from_descendant = self.is_descendant(from, node, work)?;
            let to_descendant = self.is_descendant(to, node, work)?;
            let descends_from = self.is_descendant(node, from, work)?;
            let descends_to = self.is_descendant(node, to, work)?;
            *avoid = from_descendant || to_descendant || descends_from || descends_to;
        }
        let Some((axis, _, _, ranges)) = tunnel_ranges(bounds, from, to, &excluded, 0.0) else {
            return Ok(None);
        };
        let [first, last] = [path[0], *path.last().unwrap()];
        for range in ranges {
            step(work)?;
            let mut at = range.start + PATH_NODE_PROXIMITY_FLOOR;
            while at <= range.end - PATH_NODE_PROXIMITY_FLOOR {
                step(work)?;
                let candidate = [first, last].map(|point| match axis {
                    Axis::Horizontal => Point { y: at, ..point },
                    Axis::Vertical => Point { x: at, ..point },
                });
                if candidate
                    .into_iter()
                    .all(|port| !occupied.contains(&point_key(port)))
                {
                    return Ok(Some(candidate));
                }
                at += PATH_NODE_PROXIMITY_FLOOR;
            }
        }
        Ok(None)
    }

    fn is_descendant(&self, from: usize, other: usize, work: &mut usize) -> Result<bool, Error> {
        let mut current = Some(from);
        for _ in 0..self.nodes.len() {
            step(work)?;
            let Some(node) = current else {
                return Ok(false);
            };
            if node == other {
                return Ok(true);
            }
            current = self.parents[node];
        }
        if current.is_none() {
            Ok(false)
        } else {
            Err(Error::InvalidGraph)
        }
    }
}

fn is_l_shaped(points: &[Point]) -> bool {
    if let &[start, bend, end] = points {
        (start.x == bend.x) == (bend.y == end.y)
    } else {
        false
    }
}

fn is_s_shaped(points: &[Point]) -> bool {
    points.len() == 4 && !is_u_shaped(points)
}

fn is_u_shaped(points: &[Point]) -> bool {
    let &[start, first, second, end] = points else {
        return false;
    };
    let sign = |value: f64| value.partial_cmp(&0.0).unwrap_or(Ordering::Equal);
    if start.x == first.x {
        first.y == second.y && sign(start.y - first.y) == sign(end.y - second.y)
    } else {
        first.x == second.x && sign(start.x - first.x) == sign(end.x - second.x)
    }
}

/// Applies both reference port swapping passes to rectangular routes.
///
/// # Errors
/// Returns invalid graph for mismatched topology and a work limit if the
/// entire stage cannot finish. Input paths remain unchanged on error.
pub fn swap_ports(
    nodes: &[NodeGeometry],
    parents: &[Option<usize>],
    edges: &[OrdinaryEdge],
    paths: &mut [Vec<Point>],
    work: &mut usize,
) -> Result<(), Error> {
    let bounds = nodes
        .iter()
        .map(Bounds::try_from)
        .collect::<Result<Vec<_>, Error>>()?;
    swap_with_bounds(nodes, parents, edges, paths, Some(&bounds), work)
}

/// Applies the reference's same-side crossing swaps to ordinary rectangle paths.
///
/// # Errors
/// Returns invalid graph for mismatched topology and a work limit if the
/// entire stage cannot finish. Input paths remain unchanged on error.
pub fn swap_ports_on_same_side(
    nodes: &[NodeGeometry],
    parents: &[Option<usize>],
    edges: &[OrdinaryEdge],
    paths: &mut [Vec<Point>],
    work: &mut usize,
) -> Result<(), Error> {
    swap_with_bounds(nodes, parents, edges, paths, None, work)
}

fn swap_with_bounds(
    nodes: &[NodeGeometry],
    parents: &[Option<usize>],
    edges: &[OrdinaryEdge],
    paths: &mut [Vec<Point>],
    bounds: Option<&[Bounds]>,
    work: &mut usize,
) -> Result<(), Error> {
    let context = PortSwap {
        nodes,
        parents,
        edges,
    };
    if !context.valid(paths) {
        return Err(Error::InvalidGraph);
    }
    let mut staged = paths.to_vec();
    for owner in 0..nodes.len() {
        step(work)?;
        let mut groups = PortGroups::at(edges, &staged, owner, work)?;
        groups.reorder_same_side(&context, owner, &mut staged, work)?;
        if let Some(bounds) = bounds {
            groups.reorder_adjacent_sides(&context, bounds, &mut staged, work)?;
        }
    }
    paths.clone_from_slice(&staged);
    Ok(())
}

#[cfg(test)]
mod tests;
