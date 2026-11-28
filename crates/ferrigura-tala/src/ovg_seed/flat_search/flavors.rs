// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg_edge_router,coordinator,route}.go and
// internal/layoutgraph/{edge,node}.go.

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::{Error, NodeGeometry, Point};

use super::super::grid::{charge, charge_sort};
use super::super::search::{SearchRoute, SearchScratch};
use super::super::slingshot::{Orientation, step};
use super::{Appearance, ExistingLine, FlatSearch, OrdinaryEdge, Rectangle, RouteHistory};

const MIN_ROUTE_NODE_CLEARANCE: f64 = 20.0;
const MIN_ARROWHEAD_CLEARANCE: f64 = 20.0;
const PRECISION: f64 = 0.0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    ShortestToLongest,
    LongestToShortest,
    Default,
    TopDownLeftRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteStrategy {
    Slingshot,
    Quick,
    Weighted,
    Fallback,
}

#[derive(Debug, PartialEq)]
pub struct RouteChoice {
    /// Index of the edge in the reference input order.
    pub edge: usize,
    pub strategy: RouteStrategy,
    pub route: SearchRoute,
}

#[derive(Debug, PartialEq)]
pub struct FlavorRoutes {
    pub flavor: Flavor,
    pub cost: f64,
    /// Routing order for this flavor.
    pub routes: Vec<RouteChoice>,
    detached: Vec<Point>,
}

/// Orders edges by the reference flavor and stably moves cluster edges first.
///
/// # Errors
/// Returns an invalid graph error for nonexistent endpoints and a work limit
/// when ordering the routes exceeds the remaining work.
pub fn order_routes(
    flavor: Flavor,
    nodes: &[NodeGeometry],
    edges: &[[usize; 2]],
    cluster_nodes: &HashSet<usize>,
    work: &mut usize,
) -> Result<Vec<usize>, Error> {
    let mut order: Vec<_> = (0..edges.len()).collect();
    charge(work, order.len())?;
    let mut degree = vec![0; nodes.len()];
    for &[from, to] in edges {
        *degree.get_mut(from).ok_or(Error::InvalidGraph)? += 1;
        *degree.get_mut(to).ok_or(Error::InvalidGraph)? += 1;
    }
    for &[from, to] in edges {
        step(work)?;
        charge(work, degree[from] + degree[to])?;
    }
    charge_sort(work, edges.len())?;
    charge_sort(work, edges.len())?;
    match flavor {
        Flavor::ShortestToLongest | Flavor::LongestToShortest => {
            let distance = |index| {
                let [from, to] = edges[index];
                routing_distance(&nodes[from], &nodes[to])
            };
            order.sort_by(|&first, &second| {
                let a: f64 = distance(first);
                let b: f64 = distance(second);
                let cmp = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
                if flavor == Flavor::ShortestToLongest {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }
        Flavor::TopDownLeftRight => {
            order.sort_by(|&first, &second| compare_top_left(nodes, edges[first], edges[second]));
        }
        Flavor::Default => {}
    }
    order.sort_by_key(|&index| !edges[index].iter().any(|node| cluster_nodes.contains(node)));
    Ok(order)
}

fn compare_top_left(
    nodes: &[NodeGeometry],
    mut first: [usize; 2],
    mut second: [usize; 2],
) -> Ordering {
    for endpoints in [&mut first, &mut second] {
        if nodes[endpoints[0]].y > nodes[endpoints[1]].y {
            endpoints.swap(0, 1);
        }
    }
    let [first_top, first_bottom] = first.map(|index| &nodes[index]);
    let [second_top, second_bottom] = second.map(|index| &nodes[index]);
    let compare = |a: f64, b: f64| a.partial_cmp(&b).unwrap_or(Ordering::Equal);
    if first_top.y != second_top.y {
        compare(first_top.y, second_top.y)
    } else if first[0] != second[0] {
        compare(first_top.x, second_top.x)
    } else if first_bottom.y == second_bottom.y {
        compare(first_bottom.x, second_bottom.x)
    } else {
        compare(first_bottom.y, second_bottom.y)
    }
}

fn routing_distance(first: &NodeGeometry, second: &NodeGeometry) -> f64 {
    let dx = ((first.x + first.width / 2.0) - (second.x + second.width / 2.0)).abs()
        / (first.width + second.width);
    let dy = ((first.y + first.height / 2.0) - (second.y + second.height / 2.0)).abs()
        / (first.height + second.height);
    if dx.is_nan() || dy.is_nan() {
        return f64::NAN;
    }
    Rectangle::from(first).distance_to(Rectangle::from(second)) + (1.0 / 20.0) * dx.min(dy)
}

impl FlatSearch<'_> {
    /// Produces raw paths for the reference ordering of an ordinary flavor.
    /// This stage tries slingshot, quick straight lines, and weighted paths,
    /// with a direct fallback when the visibility graph has no usable path.
    ///
    /// # Errors
    /// Returns an invalid graph or work limit for malformed input or an
    /// exhausted routing budget.
    pub fn route_flavor(
        &self,
        flavor: Flavor,
        node_ids: &[i64],
        edges: &[OrdinaryEdge],
        work: &mut usize,
    ) -> Result<Option<FlavorRoutes>, Error> {
        if node_ids.len() != self.nodes.len() {
            return Err(Error::InvalidGraph);
        }
        let pairs: Vec<_> = edges.iter().map(|edge| edge.owners).collect();
        let order = order_routes(flavor, self.nodes, &pairs, &HashSet::new(), work)?;
        let mut history = RouteHistory::new(self.graph, work)?;
        let mut scratch = SearchScratch::default();
        let mut routes = Vec::with_capacity(order.len());
        let mut cost = 0.0;
        for index in order {
            step(work)?;
            let edge = &edges[index];
            let (strategy, route) = if let Some(route) =
                self.slingshot(&mut history, edge, node_ids, edges, work)?
            {
                (RouteStrategy::Slingshot, route)
            } else if self.uses_direct_shortcut(edge)
                && let Some(route) = self.quick_route(edge, edges, &routes, &mut history, work)?
            {
                (RouteStrategy::Quick, route)
            } else {
                match self.find(&mut scratch, &mut history, edge, work) {
                    Ok(Some(route)) => (RouteStrategy::Weighted, route),
                    Ok(None) | Err(Error::InvalidGraph) => (
                        RouteStrategy::Fallback,
                        self.line_fallback(edge, edges, &routes, &mut history, work)?,
                    ),
                    Err(error) => return Err(error),
                }
            };
            cost += route.cost;
            history.accept(edge.clone(), route.clone(), work)?;
            routes.push(RouteChoice {
                edge: index,
                strategy,
                route,
            });
        }
        Ok(Some(FlavorRoutes {
            flavor,
            cost,
            routes,
            detached: history.into_detached(),
        }))
    }

    /// Selects the lowest-cost resolvable flavor. A cost difference smaller
    /// than the reference precision retains the first flavor.
    ///
    /// # Errors
    /// Returns an invalid graph or work limit from a flavor search.
    pub fn select_flavor(
        &self,
        node_ids: &[i64],
        edges: &[OrdinaryEdge],
        work: &mut usize,
    ) -> Result<Option<FlavorRoutes>, Error> {
        let mut best = None::<FlavorRoutes>;
        for flavor in [
            Flavor::ShortestToLongest,
            Flavor::LongestToShortest,
            Flavor::Default,
        ] {
            let Some(result) = self.route_flavor(flavor, node_ids, edges, work)? else {
                continue;
            };
            let least = best.as_ref().map_or(f64::INFINITY, |routes| routes.cost);
            if (result.cost - least).abs() >= PRECISION && result.cost < least {
                best = Some(result);
            }
        }
        Ok(best)
    }

    fn quick_route(
        &self,
        edge: &OrdinaryEdge,
        edges: &[OrdinaryEdge],
        routes: &[RouteChoice],
        history: &mut RouteHistory<'_>,
        work: &mut usize,
    ) -> Result<Option<SearchRoute>, Error> {
        let previous = routes
            .iter()
            .map(|choice| {
                charge(work, choice.route.vertices.len() + 1)?;
                let mut routed_edge = edges[choice.edge].clone();
                routed_edge.appearance = Appearance {
                    stroke_width: routed_edge.appearance.stroke_width,
                    ..Appearance::default()
                };
                Ok((
                    routed_edge,
                    self.materialize(&choice.route, history.detached())?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let materialized = previous
            .iter()
            .map(|(edge, points)| ExistingLine { edge, points })
            .collect::<Vec<_>>();
        let Some(line) = self.straight_line(edge, &materialized, work)? else {
            return Ok(None);
        };
        let centers = edge.owners.map(|owner| self.graph.centers_by_node[owner]);
        let ports = history.add_detached(line.ports);
        Ok(Some(route_between(centers, ports, line.cost)))
    }

    fn line_fallback(
        &self,
        edge: &OrdinaryEdge,
        edges: &[OrdinaryEdge],
        routes: &[RouteChoice],
        history: &mut RouteHistory<'_>,
        work: &mut usize,
    ) -> Result<SearchRoute, Error> {
        // Newly routed ordinary edges do not have materialized graph points
        // yet. Go passes their original edges, not quick-style copies.
        let previous = routes
            .iter()
            .map(|route| ExistingLine {
                edge: &edges[route.edge],
                points: &[],
            })
            .collect::<Vec<_>>();
        let centers = edge.owners.map(|owner| self.graph.centers_by_node[owner]);
        if let Some(line) = self.straight_line(edge, &previous, work)? {
            let [from, to] = line.ports.map(|point| {
                self.graph
                    .vertices
                    .iter()
                    .rposition(|vertex| vertex.point == point)
                    .ok_or(Error::InvalidGraph)
            });
            return Ok(route_between(centers, [from?, to?], line.cost));
        }
        let [from, to] = centers.map(|vertex| self.graph.vertices[vertex].point);
        let [a, b] = edge.owners.map(|owner| Rectangle::from(&self.nodes[owner]));
        let start = first_border_intersection(a, from, to).unwrap_or(from);
        let end = first_border_intersection(b, start, to).unwrap_or(to);
        let ports = history.add_detached([start, end]);
        Ok(route_between(centers, ports, super::euclidean(from, to)))
    }

    fn uses_direct_shortcut(&self, edge: &OrdinaryEdge) -> bool {
        let [from, to] = edge.owners.map(|owner| &self.nodes[owner]);
        let orientation = Orientation::between(from, to);
        let distance = Rectangle::from(from).distance_to(Rectangle::from(to));
        orientation != Orientation::Overlap
            && orientation.diagonal().is_none()
            && distance < 2.0 * (MIN_ROUTE_NODE_CLEARANCE + 1.0)
            && distance > MIN_ARROWHEAD_CLEARANCE
    }

    /// Materializes the selected port and turn points in input edge order.
    ///
    /// # Errors
    /// Returns invalid graph for missing or duplicate routes, invalid edge
    /// indices, and incomplete visibility paths.
    pub fn route_points_in_input_order(
        &self,
        routes: &FlavorRoutes,
    ) -> Result<Vec<Vec<Point>>, Error> {
        let mut points = vec![None; routes.routes.len()];
        for choice in &routes.routes {
            let slot = points.get_mut(choice.edge).ok_or(Error::InvalidGraph)?;
            if slot.is_some() {
                return Err(Error::InvalidGraph);
            }
            *slot = Some(self.materialize(&choice.route, &routes.detached)?);
        }
        points
            .into_iter()
            .map(|points| points.ok_or(Error::InvalidGraph))
            .collect()
    }

    /// Copies only the port and turn points from the raw visibility path.
    ///
    /// # Errors
    /// Returns an invalid graph error for an incomplete path or nonexistent
    /// visibility vertex.
    pub fn route_points(&self, route: &SearchRoute) -> Result<Vec<Point>, Error> {
        self.materialize(route, &[])
    }

    /// Resolves all raw OVG coordinates, including detached straight ports.
    ///
    /// # Errors
    /// Returns invalid graph if any referenced vertex does not exist.
    pub fn raw_route_points(
        &self,
        routes: &FlavorRoutes,
        route: &SearchRoute,
    ) -> Result<Vec<Point>, Error> {
        self.resolve_points(&route.vertices, &routes.detached)
    }

    fn resolve_points(&self, vertices: &[usize], detached: &[Point]) -> Result<Vec<Point>, Error> {
        let size = self.graph.vertices.len();
        vertices
            .iter()
            .map(|&index| {
                if let Some(vertex) = self.graph.vertices.get(index) {
                    Ok(vertex.point)
                } else {
                    detached
                        .get(index - size)
                        .copied()
                        .ok_or(Error::InvalidGraph)
                }
            })
            .collect()
    }

    fn materialize(&self, route: &SearchRoute, detached: &[Point]) -> Result<Vec<Point>, Error> {
        let interior = route
            .vertices
            .get(1..route.vertices.len().saturating_sub(1))
            .filter(|points| points.len() >= 2)
            .ok_or(Error::InvalidGraph)?;
        let interior = self.resolve_points(interior, detached)?;
        let mut points = vec![interior[0]];
        for triplet in interior.windows(3) {
            let [last, current, next] = [triplet[0], triplet[1], triplet[2]];
            if (current.x == last.x && current.x != next.x)
                || (current.y == last.y && current.y != next.y)
            {
                points.push(current);
            }
        }
        points.push(interior[interior.len() - 1]);
        Ok(points)
    }
}

fn route_between([source, target]: [usize; 2], [from, to]: [usize; 2], cost: f64) -> SearchRoute {
    SearchRoute {
        vertices: vec![source, from, to, target],
        cost,
    }
}

fn first_border_intersection(rectangle: Rectangle, from: Point, to: Point) -> Option<Point> {
    let [[left, right], [top, bottom]] = rectangle.limits();
    let rect = [
        Point { x: left, y: top },
        Point { x: right, y: top },
        Point {
            x: right,
            y: bottom,
        },
        Point { x: left, y: bottom },
    ];
    for edge in [[0, 1], [1, 2], [2, 3], [3, 0]] {
        let [a, b] = edge.map(|index| rect[index]);
        if let Some(point) = segment_intersection(from, to, a, b) {
            return Some(point);
        }
    }
    None
}

// The fallback uses the reference's rounded segment intersections, whose
// coordinates are measured from the beginning of the center-to-center line.
fn segment_intersection(u0: Point, u1: Point, v0: Point, v1: Point) -> Option<Point> {
    let udx = u1.x - u0.x;
    let vdx = v1.x - v0.x;
    let uvdx = v0.x - u0.x;
    let udy = u1.y - u0.y;
    let vdy = v1.y - v0.y;
    let uvdy = v0.y - u0.y;
    let denominator = udy * vdx - udx * vdy;
    if denominator == 0.0 {
        return None;
    }
    let s = (vdx * uvdy - vdy * uvdx) / denominator;
    let t = (udx * uvdy - udy * uvdx) / denominator;
    if !(0.0..=1.0).contains(&s) || !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some(Point {
        x: u0.x + (s * udx).round(),
        y: u0.y + (s * udy).round(),
    })
}

#[cfg(test)]
mod tests;
