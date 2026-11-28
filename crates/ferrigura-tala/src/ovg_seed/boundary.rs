// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg,ovg_fastpath,tuning}.go.

use std::collections::HashSet;

use crate::Point;

use super::{Bounds, MIN_PORT_CLEARANCE, VisibilitySeed, append_unowned, point_key};

const LAYER_PADDING: [f64; 3] = [20.0, 40.0, 60.0];

#[derive(Clone, Copy)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    fn has_port(self, sides: u8) -> bool {
        let bit = match self {
            Self::Top => 1,
            Self::Right => 8,
            Self::Bottom => 4,
            Self::Left => 2,
        };
        sides & bit != 0
    }

    fn allows_ray(self, port: Point, end: Point) -> bool {
        match self {
            Self::Top => port.x == end.x && port.y > end.y,
            Self::Right => port.y == end.y && port.x < end.x,
            Self::Bottom => port.x == end.x && port.y < end.y,
            Self::Left => port.y == end.y && port.x > end.x,
        }
    }

    fn extend(self, port: Point, padding: f64) -> Point {
        let Point { x, y } = port;
        match self {
            Self::Top => Point { x, y: y - padding },
            Self::Right => Point { x: x + padding, y },
            Self::Bottom => Point { x, y: y + padding },
            Self::Left => Point { x: x - padding, y },
        }
    }
}

pub(super) fn add(seed: &mut VisibilitySeed, bounds: &[Bounds]) {
    let Some(graph) = graph_bounds(bounds) else {
        return;
    };
    let mut occupied: HashSet<_> = seed.vertices.iter().map(|v| point_key(v.point)).collect();
    extend_layers(seed, bounds, &graph, &mut occupied);
    connect_ports(seed, bounds, &graph, &mut occupied);
    add_corners(seed, bounds, &graph, &mut occupied);
}

fn graph_bounds(bounds: &[Bounds]) -> Option<Bounds> {
    let (first, rest) = bounds.split_first()?;
    Some(rest.iter().fold(
        Bounds {
            left: first.left,
            right: first.right,
            top: first.top,
            bottom: first.bottom,
        },
        |graph, node| Bounds {
            left: graph.left.min(node.left),
            right: graph.right.max(node.right),
            top: graph.top.min(node.top),
            bottom: graph.bottom.max(node.bottom),
        },
    ))
}

fn unobstructed(point: Point, bounds: &[Bounds]) -> bool {
    !bounds.iter().any(|node| node.near(point))
}

fn extend_layers(
    seed: &mut VisibilitySeed,
    bounds: &[Bounds],
    graph: &Bounds,
    occupied: &mut HashSet<(u64, u64)>,
) {
    for padding in LAYER_PADDING {
        // Go's slice range reads just the vertices present at this layer's start.
        for index in 0..seed.vertices.len() {
            let port = seed.vertices[index].point;
            let side = if port.x == graph.right {
                Side::Right
            } else if port.x == graph.left {
                Side::Left
            } else if port.y == graph.bottom {
                Side::Bottom
            } else if port.y == graph.top {
                Side::Top
            } else {
                continue;
            };
            let point = side.extend(port, padding);
            if unobstructed(point, bounds) {
                append_unowned(&mut seed.vertices, occupied, point);
            }
        }
    }
}

fn connect_ports(
    seed: &mut VisibilitySeed,
    bounds: &[Bounds],
    graph: &Bounds,
    occupied: &mut HashSet<(u64, u64)>,
) {
    // Port owner map iteration in Go does not define an order for this
    // stage's added vertices. Visit owners in the positioned input order.
    for padding in LAYER_PADDING {
        for owner in 0..seed.ports_by_node.len() {
            let mut added = false;
            for (port, side) in port_directions(seed, owner) {
                for point in exterior_points(port, graph, padding) {
                    if ray_is_clear(seed, bounds, side, port, point) && unobstructed(point, bounds)
                    {
                        append_unowned(&mut seed.vertices, occupied, point);
                        added = true;
                    }
                }
            }
            if !added {
                extend_isolated_ports(seed, owner, graph, occupied);
            }
        }
    }
}

fn port_directions(seed: &VisibilitySeed, owner: usize) -> Vec<(Point, Side)> {
    let mut seen = HashSet::new();
    let mut ports = Vec::new();
    for &index in &seed.ports_by_node[owner] {
        if !seen.insert(index) {
            continue;
        }
        let vertex = &seed.vertices[index];
        let sides = vertex
            .owners
            .iter()
            .find(|entry| entry.node == owner)
            .expect("canonical port has owner")
            .sides;
        // Geo orientations visit the cardinal directions in this order.
        for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
            if side.has_port(sides) {
                ports.push((vertex.point, side));
            }
        }
    }
    ports
}

fn exterior_points(point: Point, graph: &Bounds, padding: f64) -> [Point; 4] {
    let Point { x, y } = point;
    [
        Point {
            x,
            y: graph.top - padding,
        },
        Point {
            x,
            y: graph.bottom + padding,
        },
        Point {
            x: graph.left - padding,
            y,
        },
        Point {
            x: graph.right + padding,
            y,
        },
    ]
}

fn ray_is_clear(
    seed: &VisibilitySeed,
    bounds: &[Bounds],
    side: Side,
    port: Point,
    end: Point,
) -> bool {
    let allow_ports = side.allows_ray(port, end);
    !bounds.iter().enumerate().any(|(node, bounds)| {
        bounds.blocks_axis_segment(port, end)
            && !(allow_ports
                && seed.ports_by_node[node].iter().any(|&index| {
                    let point = seed.vertices[index].point;
                    point == port || point == end
                }))
    })
}

fn extend_isolated_ports(
    seed: &mut VisibilitySeed,
    owner: usize,
    graph: &Bounds,
    occupied: &mut HashSet<(u64, u64)>,
) {
    for (port, side) in port_directions(seed, owner) {
        let point = side.extend(port, MIN_PORT_CLEARANCE);
        append_unowned(&mut seed.vertices, occupied, point);
        for padding in LAYER_PADDING {
            let [top, bottom, left, right] = exterior_points(point, graph, padding);
            let points = match side {
                Side::Top | Side::Bottom => [left, right],
                Side::Left | Side::Right => [top, bottom],
            };
            for exterior in points {
                append_unowned(&mut seed.vertices, occupied, exterior);
            }
        }
    }
}

fn add_corners(
    seed: &mut VisibilitySeed,
    bounds: &[Bounds],
    graph: &Bounds,
    occupied: &mut HashSet<(u64, u64)>,
) {
    for padding in LAYER_PADDING {
        let top = graph.top - padding;
        let bottom = graph.bottom + padding;
        let left = graph.left - padding;
        let right = graph.right + padding;
        for point in [
            Point { x: left, y: top },
            Point { x: right, y: top },
            Point {
                x: right,
                y: bottom,
            },
            Point { x: left, y: bottom },
        ] {
            if unobstructed(point, bounds) {
                append_unowned(&mut seed.vertices, occupied, point);
            }
        }
    }
}
