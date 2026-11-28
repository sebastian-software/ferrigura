// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA at b463ea5: internal/routing/{edge_simplify,geometry}.go.

use std::collections::HashMap;

use crate::{Error, Graph, Layout, NodeGeometry, Point};

const MAX_WORK: usize = 250_000;
const MAX_ENTITIES: usize = 256;

struct Topology<'a> {
    nodes: &'a [NodeGeometry],
    parents: Vec<Option<usize>>,
    endpoints: Vec<(usize, usize)>,
}

impl<'a> Topology<'a> {
    fn new(graph: &Graph, layout: &'a Layout) -> Result<Self, Error> {
        if graph.nodes.len() > MAX_ENTITIES || graph.edges.len() > MAX_ENTITIES {
            return Err(Error::WorkLimit);
        }
        if graph.nodes.len() != layout.nodes.len() || graph.edges.len() != layout.edges.len() {
            return Err(Error::InvalidGraph);
        }
        let indices: HashMap<_, _> = layout
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        if indices.len() != layout.nodes.len()
            || layout.nodes.iter().any(|node| {
                ![node.x, node.y, node.width, node.height]
                    .iter()
                    .all(|value| value.is_finite())
            })
        {
            return Err(Error::InvalidGraph);
        }
        let mut parents = vec![None; layout.nodes.len()];
        let mut seen = vec![false; layout.nodes.len()];
        for node in &graph.nodes {
            let index = *indices.get(node.id.as_str()).ok_or(Error::InvalidGraph)?;
            if std::mem::replace(&mut seen[index], true) {
                return Err(Error::InvalidGraph);
            }
            if let Some(parent) = node.parent_id.as_deref() {
                parents[index] = Some(*indices.get(parent).ok_or(Error::InvalidGraph)?);
            }
        }
        for mut current in 0..parents.len() {
            for depth in 0..parents.len() {
                let Some(parent) = parents[current] else {
                    break;
                };
                if depth + 1 == parents.len() {
                    return Err(Error::InvalidGraph);
                }
                current = parent;
            }
        }
        let endpoints = graph
            .edges
            .iter()
            .zip(&layout.edges)
            .map(|(edge, route)| {
                if edge.id != route.id {
                    return Err(Error::InvalidGraph);
                }
                Ok((
                    *indices
                        .get(edge.source.as_str())
                        .ok_or(Error::InvalidGraph)?,
                    *indices
                        .get(edge.target.as_str())
                        .ok_or(Error::InvalidGraph)?,
                ))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            nodes: &layout.nodes,
            parents,
            endpoints,
        })
    }

    fn ancestor(&self, ancestor: usize, mut node: usize) -> bool {
        while let Some(parent) = self.parents[node] {
            if parent == ancestor {
                return true;
            }
            node = parent;
        }
        false
    }
}

/// Replays the Go route simplification stage on an already-routed graph.
///
/// Input geometry is absolute and uses the same edge order as `graph`.
/// Placement, primary routing, and relabeling run at other pipeline stages.
/// Geometry is committed only when the entire pass succeeds.
///
/// # Errors
/// Refuses inconsistent topology and work beyond the experimental stage budget.
pub fn simplify_routes(graph: &Graph, layout: &mut Layout) -> Result<(), Error> {
    let topology = Topology::new(graph, layout)?;
    let mut work = MAX_WORK;
    let mut routes = Vec::with_capacity(layout.edges.len());
    for edge in &layout.edges {
        spend(&mut work, edge.points.len())?;
        spend(&mut work, 1)?;
        if edge
            .points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(Error::InvalidGraph);
        }
        routes.push(edge.points.clone());
    }
    for index in 0..routes.len() {
        while let Some(route) = simplify_once(&topology, &routes, index, &mut work)? {
            routes[index] = route;
        }
    }
    for (edge, route) in layout.edges.iter_mut().zip(routes) {
        edge.points = route;
    }
    Ok(())
}

fn spend(work: &mut usize, units: usize) -> Result<(), Error> {
    *work = work.checked_sub(units).ok_or(Error::WorkLimit)?;
    Ok(())
}

fn simplify_once(
    topology: &Topology<'_>,
    routes: &[Vec<Point>],
    index: usize,
    work: &mut usize,
) -> Result<Option<Vec<Point>>, Error> {
    let points = &routes[index];
    let (source, target) = topology.endpoints[index];
    for (start, window) in points.windows(5).enumerate() {
        spend(work, 1)?;
        let [a, b, c, d, e] = [window[0], window[1], window[2], window[3], window[4]];
        let Some(turn) = shortened_turn(a, b, c, d, e) else {
            continue;
        };
        if start == 0 && contains(&topology.nodes[source], a) && !same_direction(a, b, a, turn)
            || start + 5 == points.len()
                && contains(&topology.nodes[target], e)
                && !same_direction(d, e, turn, e)
        {
            continue;
        }
        if blocked(topology, routes, index, a, turn, work)?
            || blocked(topology, routes, index, turn, e, work)?
        {
            continue;
        }
        spend(work, points.len() - 2)?;
        let mut replacement = Vec::with_capacity(points.len() - 2);
        replacement.extend_from_slice(&points[..=start]);
        replacement.push(turn);
        replacement.extend_from_slice(&points[start + 4..]);
        return Ok(Some(replacement));
    }
    Ok(None)
}

fn shortened_turn(a: Point, b: Point, c: Point, d: Point, e: Point) -> Option<Point> {
    let first = delta(a, b);
    let second = delta(b, c);
    let third = delta(c, d);
    let fourth = delta(d, e);
    if first.y == 0.0
        && third.y == 0.0
        && same_sign(first.x, third.x)
        && second.x == 0.0
        && fourth.x == 0.0
        && same_sign(second.y, -fourth.y)
    {
        Some(Point { x: e.x, y: a.y })
    } else if first.x == 0.0
        && third.x == 0.0
        && same_sign(first.y, third.y)
        && second.y == 0.0
        && fourth.y == 0.0
        && same_sign(second.x, -fourth.x)
    {
        Some(Point { x: a.x, y: e.y })
    } else {
        None
    }
}

fn same_sign(a: f64, b: f64) -> bool {
    a > 0.0 && b > 0.0 || a < 0.0 && b < 0.0
}

fn delta(from: Point, to: Point) -> Point {
    Point {
        x: to.x - from.x,
        y: to.y - from.y,
    }
}

fn same_direction(a: Point, b: Point, c: Point, d: Point) -> bool {
    let first = delta(a, b);
    let second = delta(c, d);
    first.x * second.x + first.y * second.y > 0.0 && first.x * second.y == first.y * second.x
}

fn contains(node: &NodeGeometry, point: Point) -> bool {
    node.x - 1.0 <= point.x
        && point.x <= node.x + node.width + 1.0
        && node.y - 1.0 <= point.y
        && point.y <= node.y + node.height + 1.0
}

fn blocked(
    topology: &Topology<'_>,
    routes: &[Vec<Point>],
    index: usize,
    a: Point,
    b: Point,
    work: &mut usize,
) -> Result<bool, Error> {
    let (source, target) = topology.endpoints[index];
    for (id, node) in topology.nodes.iter().enumerate() {
        spend(work, 1)?;
        if node.width <= 0.0 || node.height <= 0.0 {
            continue;
        }
        if id != source
            && id != target
            && (topology.ancestor(id, source) || topology.ancestor(id, target))
        {
            continue;
        }
        if enters_node(node, a, b) {
            return Ok(true);
        }
    }
    for (other, route) in routes.iter().enumerate() {
        spend(work, 1)?;
        if other == index {
            continue;
        }
        for pair in route.windows(2) {
            spend(work, 1)?;
            if intersects(a, b, pair[0], pair[1]) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn enters_node(node: &NodeGeometry, a: Point, b: Point) -> bool {
    const EPSILON: f64 = 1e-6;
    let (left, right) = (node.x + EPSILON, node.x + node.width - EPSILON);
    let (top, bottom) = (node.y + EPSILON, node.y + node.height - EPSILON);
    if a.y == b.y {
        top < a.y && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
    } else if a.x == b.x {
        left < a.x && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
    } else {
        true
    }
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    let first = delta(a, b);
    let second = delta(a, c);
    first.y * second.x - first.x * second.y
}

fn intersects(a: Point, b: Point, c: Point, d: Point) -> bool {
    let sides = [
        orientation(a, b, c),
        orientation(a, b, d),
        orientation(c, d, a),
        orientation(c, d, b),
    ];
    if sides.iter().all(|&side| side == 0.0) {
        let overlaps =
            |a: f64, b: f64, c: f64, d: f64| a.min(b) <= c.max(d) && c.min(d) <= a.max(b);
        overlaps(a.x, b.x, c.x, d.x) && overlaps(a.y, b.y, c.y, d.y)
    } else {
        let straddles = |first: f64, second: f64| {
            first == 0.0 || second == 0.0 || (first < 0.0) != (second < 0.0)
        };
        straddles(sides[0], sides[1]) && straddles(sides[2], sides[3])
    }
}

#[cfg(test)]
mod tests;
