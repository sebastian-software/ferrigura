// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See d2layouts/d2talalayout/internal/routing/{ovg,ovg_fastpath,ovg_node,ovg_resource}.go,
// internal/nodeshape/{policy,snappoints}.go, and internal/layoutgraph/{geometry_policy,node}.go.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Error, NodeGeometry, Point};

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
                let port = *occupied
                    .entry((key(point.x), key(point.y)))
                    .or_insert_with(|| {
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

fn key(coordinate: f64) -> u64 {
    if coordinate == 0.0 {
        0
    } else {
        coordinate.to_bits()
    }
}

fn add_intersections(vertices: &mut Vec<Vertex>, bounds: &[Bounds]) {
    let mut columns: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut rows: HashMap<u64, Vec<usize>> = HashMap::new();
    for (index, vertex) in vertices.iter().enumerate() {
        columns.entry(key(vertex.point.x)).or_default().push(index);
        rows.entry(key(vertex.point.y)).or_default().push(index);
    }
    // The reference enumerates the Cartesian product in sorted X/Y order.
    let mut xs: Vec<_> = columns.keys().copied().map(f64::from_bits).collect();
    let mut ys: Vec<_> = rows.keys().copied().map(f64::from_bits).collect();
    xs.sort_unstable_by(f64::total_cmp);
    ys.sort_unstable_by(f64::total_cmp);
    for x in xs {
        let column = &columns[&key(x)];
        for &y in &ys {
            let row = &rows[&key(y)];
            let point = Point { x, y };
            if too_close_to_port(vertices, column, y, |p| p.y)
                || too_close_to_port(vertices, row, x, |p| p.x)
                || bounds.iter().any(|node| node.near(point))
                || !sees_two_owners(vertices, column, row, bounds, point)
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

fn sees_two_owners(
    vertices: &[Vertex],
    column: &[usize],
    row: &[usize],
    bounds: &[Bounds],
    point: Point,
) -> bool {
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
            if visible_owner.is_some() {
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
