// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See d2layouts/d2talalayout/internal/routing/{ovg,ovg_fastpath,ovg_node,ovg_resource}.go,
// internal/nodeshape/{policy,snappoints}.go, and internal/layoutgraph/{geometry_policy,node}.go.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{Error, NodeGeometry, Point};

mod boundary;
mod edge_points;
mod links;
mod tunnels;

const MIN_PORT_CLEARANCE: f64 = 20.0;
const MIN_ROUTE_NODE_CLEARANCE: f64 = 20.0;
const SLOTS_PER_RECTANGLE: usize = 12;
const SIDE_FRACTIONS: [f64; 3] = [0.25, 0.5, 0.75];

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilitySeed {
    pub vertices: Vec<Vertex>,
    pub ports_by_node: Vec<[usize; SLOTS_PER_RECTANGLE]>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Vertex {
    pub point: Point,
    pub owners: Vec<PortOwner>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct PortOwner {
    /// Index of the positioned input rectangle.
    pub node: usize,
    /// Bits encode top, left, bottom, and right in that order.
    pub sides: u8,
    pub center: bool,
}

/// Generates port vertices and initial visibility intersections for the
/// visible, unpinned rectangles of one ordinary routing subgraph.
///
/// # Errors
/// Returns an error for invalid rectangle geometry or subgraphs beyond the
/// experimental work limit.
pub fn rectangular_visibility_seed(nodes: &[NodeGeometry]) -> Result<VisibilitySeed, Error> {
    if nodes.len() > super::MAX_NODES {
        return Err(Error::WorkLimit);
    }
    let bounds: Vec<_> = nodes
        .iter()
        .map(Bounds::try_from)
        .collect::<Result<_, _>>()?;
    let mut seed = VisibilitySeed {
        vertices: Vec::with_capacity(SLOTS_PER_RECTANGLE * nodes.len()),
        ports_by_node: Vec::with_capacity(nodes.len()),
    };
    let mut occupied = HashMap::with_capacity(SLOTS_PER_RECTANGLE * nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        let mut ports = [0; SLOTS_PER_RECTANGLE];
        for (side, side_slots) in ports.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            for (slot, fraction) in SIDE_FRACTIONS.into_iter().enumerate() {
                let (x, y) = match side {
                    0 => (fraction, 0.0),
                    1 => (0.0, fraction),
                    2 => (fraction, 1.0),
                    _ => (1.0, fraction),
                };
                let point = Point {
                    x: node.x + (node.width * x).round(),
                    y: node.y + (node.height * y).round(),
                };
                let port = *occupied.entry(point_key(point)).or_insert_with(|| {
                    let index = seed.vertices.len();
                    seed.vertices.push(Vertex {
                        point,
                        owners: Vec::new(),
                    });
                    index
                });
                let owners = &mut seed.vertices[port].owners;
                let side_flag = 1 << side;
                let center = slot == 1;
                if let Some(owner) = owners.last_mut().filter(|owner| owner.node == index) {
                    owner.sides |= side_flag;
                    owner.center |= center;
                } else {
                    owners.push(PortOwner {
                        node: index,
                        sides: side_flag,
                        center,
                    });
                }
                side_slots[slot] = port;
            }
        }
        seed.ports_by_node.push(ports);
    }
    add_intersections(&mut seed.vertices, &bounds);
    Ok(seed)
}

/// Adds the reference's perimeter and halfway vertices after the initial
/// port intersections for an ordinary rectangular routing subgraph.
///
/// Each edge is a pair of positioned node indices in reference routing order.
///
/// # Errors
/// Returns an error for invalid geometry or endpoints and for a subgraph
/// beyond the supported replay work limit.
pub fn rectangular_edge_visibility_seed(
    nodes: &[NodeGeometry],
    edges: &[[usize; 2]],
) -> Result<VisibilitySeed, Error> {
    if edges.len() > super::MAX_EDGES {
        return Err(Error::WorkLimit);
    }
    if edges.iter().flatten().any(|&index| index >= nodes.len()) {
        return Err(Error::InvalidGraph);
    }
    let mut seed = rectangular_visibility_seed(nodes)?;
    let bounds: Vec<_> = nodes
        .iter()
        .map(Bounds::try_from)
        .collect::<Result<_, _>>()?;
    edge_points::add(&mut seed, &bounds, edges);
    Ok(seed)
}

/// Adds routing layer, port-ray and corner vertices outside the graph bounds
/// after the reference's edge perimeter and halfway vertices.
///
/// # Errors
/// Returns an error for invalid geometry or endpoints and for a subgraph
/// beyond the supported replay work limit.
pub fn rectangular_boundary_visibility_seed(
    nodes: &[NodeGeometry],
    edges: &[[usize; 2]],
) -> Result<VisibilitySeed, Error> {
    let mut seed = rectangular_edge_visibility_seed(nodes, edges)?;
    let bounds: Vec<_> = nodes
        .iter()
        .map(Bounds::try_from)
        .collect::<Result<_, _>>()?;
    boundary::add(&mut seed, &bounds);
    Ok(seed)
}

/// The ordinary visibility graph immediately after tunnel endpoints and links
/// have been added, before the general adjacent-vertex connection pass.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelVisibility {
    pub vertices: Vec<Vertex>,
    pub ports_by_node: Vec<Vec<usize>>,
    pub tunnel_links: Vec<[usize; 2]>,
}

/// Builds the reference's ordered tunnel links and port-slot additions for
/// an already positioned ordinary rectangular routing subgraph.
///
/// # Errors
/// Returns an error for invalid geometry or endpoints and for a subgraph
/// beyond the supported replay work limit.
pub fn rectangular_tunnel_visibility(
    nodes: &[NodeGeometry],
    edges: &[[usize; 2]],
) -> Result<TunnelVisibility, Error> {
    let seed = rectangular_boundary_visibility_seed(nodes, edges)?;
    let bounds: Vec<_> = nodes
        .iter()
        .map(Bounds::try_from)
        .collect::<Result<_, _>>()?;
    Ok(tunnels::add(seed, &bounds, edges))
}

/// Fully linked flat rectangular OVG. Tunnel links occupy the ordered prefix
/// of `links`. Center and port slots retain the distinct vertex identities of
/// the reference even when coordinates coincide.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedVisibility {
    pub vertices: Vec<Vertex>,
    pub ports_by_node: Vec<Vec<usize>>,
    pub centers_by_node: Vec<usize>,
    pub links: Vec<[usize; 2]>,
    pub tunnel_link_count: usize,
    pub near_port_owners: Vec<Vec<usize>>,
}

/// Builds the connected visibility graph used to route ordinary positioned
/// rectangles, including tunnels, obstacle-aware links, and port proximity.
///
/// # Errors
/// Returns an error for invalid geometry or endpoints and for a subgraph
/// beyond the supported replay work limit.
pub fn rectangular_linked_visibility(
    nodes: &[NodeGeometry],
    edges: &[[usize; 2]],
) -> Result<LinkedVisibility, Error> {
    let graph = rectangular_tunnel_visibility(nodes, edges)?;
    let bounds: Vec<_> = nodes
        .iter()
        .map(Bounds::try_from)
        .collect::<Result<_, _>>()?;
    Ok(links::connect(graph, &bounds, nodes))
}

fn append_unowned(vertices: &mut Vec<Vertex>, occupied: &mut HashSet<(u64, u64)>, point: Point) {
    if occupied.insert(point_key(point)) {
        vertices.push(Vertex {
            point,
            owners: Vec::new(),
        });
    }
}

fn point_key(point: Point) -> (u64, u64) {
    (key(point.x), key(point.y))
}

fn key(coordinate: f64) -> u64 {
    if coordinate == 0.0 {
        0
    } else {
        coordinate.to_bits()
    }
}

#[derive(Default)]
struct PortLines {
    columns: HashMap<u64, Vec<usize>>,
    rows: HashMap<u64, Vec<usize>>,
}

impl PortLines {
    fn new(vertices: &[Vertex]) -> Self {
        let mut lines = Self::default();
        for (index, vertex) in vertices
            .iter()
            .enumerate()
            .filter(|(_, vertex)| !vertex.owners.is_empty())
        {
            let (x, y) = point_key(vertex.point);
            lines.columns.entry(x).or_default().push(index);
            lines.rows.entry(y).or_default().push(index);
        }
        lines
    }

    fn visible(
        &self,
        vertices: &[Vertex],
        bounds: &[Bounds],
        point: Point,
        required: usize,
    ) -> bool {
        let (x, y) = point_key(point);
        let column = self.columns.get(&x).map_or(&[][..], Vec::as_slice);
        let row = self.rows.get(&y).map_or(&[][..], Vec::as_slice);
        sees_owners(vertices, column, row, bounds, point, required)
    }
}

fn add_intersections(vertices: &mut Vec<Vertex>, bounds: &[Bounds]) {
    let lines = PortLines::new(vertices);
    // The reference enumerates the Cartesian product in sorted X/Y order.
    let mut xs: Vec<_> = lines.columns.keys().copied().map(f64::from_bits).collect();
    let mut ys: Vec<_> = lines.rows.keys().copied().map(f64::from_bits).collect();
    xs.sort_unstable_by(f64::total_cmp);
    ys.sort_unstable_by(f64::total_cmp);
    for x in xs {
        let column = &lines.columns[&key(x)];
        for &y in &ys {
            let row = &lines.rows[&key(y)];
            let point = Point { x, y };
            if too_close_to_port(vertices, column, y, |p| p.y)
                || too_close_to_port(vertices, row, x, |p| p.x)
                || bounds.iter().any(|node| node.near(point))
                || !sees_owners(vertices, column, row, bounds, point, 2)
            {
                continue;
            }
            vertices.push(Vertex {
                point,
                owners: Vec::new(),
            });
        }
    }
}

fn too_close_to_port(
    vertices: &[Vertex],
    line: &[usize],
    coordinate: f64,
    axis: impl Fn(Point) -> f64,
) -> bool {
    line.iter().any(|&index| {
        let nearby = axis(vertices[index].point);
        (coordinate - MIN_PORT_CLEARANCE..=coordinate + MIN_PORT_CLEARANCE).contains(&nearby)
    })
}

fn sees_owners(
    vertices: &[Vertex],
    column: &[usize],
    row: &[usize],
    bounds: &[Bounds],
    point: Point,
    required: usize,
) -> bool {
    if column.is_empty() && row.is_empty() {
        // Without any aligned port the reference accepts the candidate.
        return true;
    }
    let mut visible_owner = None;
    for &index in column.iter().chain(row) {
        let port = &vertices[index];
        for owner in &port.owners {
            if visible_owner == Some(owner.node) {
                continue;
            }
            let body = &bounds[owner.node];
            if port.point.x == point.x && (point.x == body.left || point.x == body.right)
                || port.point.y == point.y && (point.y == body.top || point.y == body.bottom)
            {
                continue;
            }
            if bounds.iter().enumerate().any(|(index, node)| {
                index != owner.node && node.blocks_axis_segment(port.point, point)
            }) {
                continue;
            }
            // Initial intersections need two owners, edge fill points one.
            if required == 1 || visible_owner.is_some() {
                return true;
            }
            visible_owner = Some(owner.node);
        }
    }
    false
}

struct Bounds {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl TryFrom<&NodeGeometry> for Bounds {
    type Error = Error;

    fn try_from(node: &NodeGeometry) -> Result<Self, Self::Error> {
        let right = node.x + node.width;
        let bottom = node.y + node.height;
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !right.is_finite()
            || !bottom.is_finite()
            || right <= node.x
            || bottom <= node.y
        {
            return Err(Error::InvalidGraph);
        }
        Ok(Self {
            left: node.x,
            right,
            top: node.y,
            bottom,
        })
    }
}

impl Bounds {
    fn near(&self, point: Point) -> bool {
        self.left - MIN_ROUTE_NODE_CLEARANCE <= point.x
            && point.x <= self.right + MIN_ROUTE_NODE_CLEARANCE
            && self.top - MIN_ROUTE_NODE_CLEARANCE <= point.y
            && point.y <= self.bottom + MIN_ROUTE_NODE_CLEARANCE
    }

    fn blocks_axis_segment(&self, start: Point, end: Point) -> bool {
        // On an axis-aligned segment, closed bounding-box intersection is
        // equivalent to the reference's endpoint checks and clipping test.
        self.left <= start.x.max(end.x)
            && start.x.min(end.x) <= self.right
            && self.top <= start.y.max(end.y)
            && start.y.min(end.y) <= self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Case {
        name: String,
        nodes: Vec<NodeGeometry>,
        expected: VisibilitySeed,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EdgePointsCase {
        edges: Vec<[usize; 2]>,
        added_points: Vec<[f64; 2]>,
    }

    fn observatory() -> (Vec<NodeGeometry>, Vec<[usize; 2]>, VisibilitySeed) {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("ovg_seed/vertices.json")).unwrap();
        let case = cases
            .into_iter()
            .find(|case| case.name == "Observatory services measured graph at initial OVG routing")
            .unwrap();
        let fixture: EdgePointsCase =
            serde_json::from_str(include_str!("ovg_seed/edge-points.json")).unwrap();
        let mut expected = case.expected;
        expected
            .vertices
            .extend(fixture.added_points.into_iter().map(|[x, y]| Vertex {
                point: Point { x, y },
                owners: Vec::new(),
            }));
        (case.nodes, fixture.edges, expected)
    }

    #[test]
    fn matches_pinned_go_observatory_edge_fill_points() {
        let (nodes, edges, expected) = observatory();
        let actual = rectangular_edge_visibility_seed(&nodes, &edges).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn matches_pinned_go_observatory_boundary_points() {
        let (nodes, edges, expected) = observatory();
        let actual = rectangular_boundary_visibility_seed(&nodes, &edges).unwrap();
        let (prefix, boundary) = actual.vertices.split_at(expected.vertices.len());
        assert_eq!(prefix, expected.vertices);
        assert_eq!(actual.ports_by_node, expected.ports_by_node);
        // Go ranges over the port owner map in this stage, so the suffix
        // has no defined vertex order. The exact coordinates are stable.
        let points: HashSet<_> = boundary
            .iter()
            .map(|vertex| point_key(vertex.point))
            .collect();
        let fixture: Vec<[f64; 2]> =
            serde_json::from_str(include_str!("ovg_seed/boundary-points.json")).unwrap();
        let expected: HashSet<_> = fixture
            .into_iter()
            .map(|[x, y]| point_key(Point { x, y }))
            .collect();
        assert!(boundary.iter().all(|vertex| vertex.owners.is_empty()));
        assert_eq!(boundary.len(), points.len());
        assert_eq!(points, expected);
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TunnelCase {
        vertices: Vec<Vertex>,
        added_ports: Vec<Vec<usize>>,
        links: Vec<[usize; 2]>,
    }

    #[test]
    fn matches_pinned_go_observatory_tunnel_links_and_ports() {
        let (nodes, edges, _) = observatory();
        let before = rectangular_boundary_visibility_seed(&nodes, &edges).unwrap();
        let actual = rectangular_tunnel_visibility(&nodes, &edges).unwrap();
        let fixture: TunnelCase =
            serde_json::from_str(include_str!("ovg_seed/tunnels.json")).unwrap();
        let expected_ports: Vec<_> = before
            .ports_by_node
            .iter()
            .zip(fixture.added_ports)
            .map(|(base, added)| base.iter().copied().chain(added).collect::<Vec<_>>())
            .collect();
        let (prefix, added) = actual.vertices.split_at(before.vertices.len());
        assert_eq!(prefix, before.vertices);
        assert_eq!(added, fixture.vertices);
        assert_eq!(actual.ports_by_node, expected_ports);
        assert_eq!(actual.tunnel_links, fixture.links);
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LinksCase {
        vertex_count: usize,
        tunnel_link_count: usize,
        links: Vec<[usize; 2]>,
        near_ports: Vec<[usize; 2]>,
    }

    #[test]
    fn matches_pinned_go_observatory_visibility_links_and_near_ports() {
        let (nodes, edges, _) = observatory();
        let graph = rectangular_linked_visibility(&nodes, &edges).unwrap();
        let fixture: LinksCase = serde_json::from_str(include_str!("ovg_seed/links.json")).unwrap();
        assert_eq!(graph.vertices.len(), fixture.vertex_count);
        assert_eq!(graph.tunnel_link_count, fixture.tunnel_link_count);
        let mut centers = vec![None; graph.vertices.len()];
        for (owner, &vertex) in graph.centers_by_node.iter().enumerate() {
            centers[vertex] = Some(owner);
        }
        // Only vertex storage order is unspecified at the Go boundary.
        // Canonical identities preserve link and adjacency ordering.
        let mut ordered: Vec<_> = (0..graph.vertices.len()).collect();
        ordered.sort_unstable_by(|&a, &b| {
            let first = graph.vertices[a].point;
            let second = graph.vertices[b].point;
            first
                .x
                .total_cmp(&second.x)
                .then_with(|| first.y.total_cmp(&second.y))
                .then_with(|| centers[a].cmp(&centers[b]))
        });
        let mut remap = vec![0; graph.vertices.len()];
        for (index, &original) in ordered.iter().enumerate() {
            remap[original] = index;
        }
        let links: Vec<_> = graph
            .links
            .iter()
            .map(|link| link.map(|vertex| remap[vertex]))
            .collect();
        assert_eq!(links, fixture.links);
        let marks: Vec<_> = ordered
            .iter()
            .enumerate()
            .flat_map(|(canonical, &original)| {
                graph.near_port_owners[original]
                    .iter()
                    .map(move |&owner| [canonical, owner])
            })
            .collect();
        assert_eq!(marks, fixture.near_ports);
    }

    #[test]
    fn matches_pinned_go_ports_and_initial_intersections() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("ovg_seed/vertices.json")).unwrap();
        for case in cases {
            let actual = rectangular_visibility_seed(&case.nodes).unwrap();
            assert_eq!(actual, case.expected, "{}", case.name);
        }
    }
}
