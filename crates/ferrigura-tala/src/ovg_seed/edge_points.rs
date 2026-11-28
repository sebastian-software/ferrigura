// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg,ovg_fastpath,tuning}.go and internal/layoutgraph/node.go.

use std::collections::HashSet;

use crate::Point;

use super::{Bounds, PortLines, VisibilitySeed, append_unowned, point_key};

const OVERSHOOT: f64 = 100.0;

pub(super) fn add(seed: &mut VisibilitySeed, bounds: &[Bounds], edges: &[[usize; 2]]) {
    let lines = PortLines::new(&seed.vertices);
    let mut occupied: HashSet<_> = seed.vertices.iter().map(|v| point_key(v.point)).collect();
    for &[source, target] in edges {
        if source == target {
            continue;
        }
        let mut candidates = Vec::new();
        perimeter(seed, bounds, [source, target], &mut candidates);
        halfway(seed, bounds, [source, target], &mut candidates);
        for point in candidates {
            if occupied.contains(&point_key(point))
                || bounds.iter().any(|node| node.near(point))
                || !lines.visible(&seed.vertices, bounds, point, 1)
            {
                continue;
            }
            append_unowned(&mut seed.vertices, &mut occupied, point);
        }
    }
}

fn perimeter(seed: &VisibilitySeed, bounds: &[Bounds], edge: [usize; 2], out: &mut Vec<Point>) {
    let [a, b] = edge.map(|index| &bounds[index]);
    let top = a.top.min(b.top) - OVERSHOOT;
    let bottom = a.bottom.max(b.bottom) + OVERSHOOT;
    let left = a.left.min(b.left) - OVERSHOOT;
    let right = a.right.max(b.right) + OVERSHOOT;
    for owner in edge {
        // The stored side bits are top, left, bottom, right. Go emits
        // perimeter points top, bottom, left, right for each endpoint.
        for side in [1, 4, 2, 8] {
            for &port in &seed.ports_by_node[owner] {
                let vertex = &seed.vertices[port];
                if vertex
                    .owners
                    .iter()
                    .any(|metadata| metadata.node == owner && metadata.sides & side != 0)
                {
                    let Point { x, y } = vertex.point;
                    out.push(match side {
                        1 => Point { x, y: top },
                        4 => Point { x, y: bottom },
                        2 => Point { x: left, y },
                        _ => Point { x: right, y },
                    });
                }
            }
        }
    }
    out.extend([
        Point { x: left, y: top },
        Point { x: right, y: top },
        Point { x: left, y: bottom },
        Point {
            x: right,
            y: bottom,
        },
    ]);
}

fn halfway(seed: &VisibilitySeed, bounds: &[Bounds], edge: [usize; 2], out: &mut Vec<Point>) {
    let [a, b] = edge.map(|index| &bounds[index]);
    // The relative orientation uses strict rectangle separation. Go rounds
    // the bottom-right coordinates only when computing the halfway line.
    let between_y = if b.bottom < a.top {
        Some((a.top + b.bottom.round()) / 2.0)
    } else if a.bottom < b.top {
        Some((b.top + a.bottom.round()) / 2.0)
    } else {
        None
    };
    let between_x = if b.right < a.left {
        Some((a.left + b.right.round()) / 2.0)
    } else if a.right < b.left {
        Some((b.left + a.right.round()) / 2.0)
    } else {
        None
    };
    let ports = edge
        .into_iter()
        .flat_map(|node| seed.ports_by_node[node])
        .map(|port| seed.vertices[port].point);
    if let Some(y) = between_y {
        out.extend(ports.clone().map(|point| Point { x: point.x, y }));
    }
    if let Some(x) = between_x {
        out.extend(ports.map(|point| Point { x, y: point.y }));
    }
    if let (Some(x), Some(y)) = (between_x, between_y) {
        out.push(Point { x, y });
    }
}
