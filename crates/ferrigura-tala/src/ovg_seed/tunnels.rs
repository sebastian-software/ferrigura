// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{tunnel,ovg,tuning}.go and internal/layoutgraph/node.go.

use std::collections::{HashMap, HashSet};

use crate::Point;

use super::{Bounds, PortOwner, TunnelVisibility, Vertex, VisibilitySeed, point_key};

const SPACING: f64 = 40.0;

#[derive(Clone, Copy)]
pub(super) struct Span {
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Copy)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn axes(self, node: &Bounds) -> [Span; 2] {
        let x = Span {
            start: node.left,
            end: node.right,
        };
        let y = Span {
            start: node.top,
            end: node.bottom,
        };
        match self {
            Self::Horizontal => [x, y],
            Self::Vertical => [y, x],
        }
    }

    fn point(self, along: f64, across: f64) -> Point {
        match self {
            Self::Horizontal => Point {
                x: along,
                y: across,
            },
            Self::Vertical => Point {
                x: across,
                y: along,
            },
        }
    }
}

pub(super) fn add(
    seed: VisibilitySeed,
    bounds: &[Bounds],
    edges: &[[usize; 2]],
) -> TunnelVisibility {
    let mut graph = TunnelVisibility {
        vertices: seed.vertices,
        ports_by_node: seed.ports_by_node.into_iter().map(Vec::from).collect(),
        tunnel_links: Vec::new(),
    };
    let mut occupied: HashMap<_, _> = graph
        .vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| (point_key(vertex.point), index))
        .collect();
    let mut seen = HashSet::new();
    for node in 0..bounds.len() {
        // Go searches node order, then each node's original edge order.
        for &edge in edges {
            let other = match edge {
                [owner, other] if owner == node => other,
                [other, owner] if owner == node => other,
                _ => continue,
            };
            if node == other || !seen.insert([node.min(other), node.max(other)]) {
                continue;
            }
            let count = edges
                .iter()
                .filter(|edge| edge.contains(&node) && edge.contains(&other))
                .count();
            let points = tunnel_points(bounds, node, other, count);
            for [from, to] in points {
                let from = add_endpoint(&mut graph, &mut occupied, node, from);
                let to = add_endpoint(&mut graph, &mut occupied, other, to);
                graph.tunnel_links.push([from, to]);
            }
        }
    }
    graph
}

fn add_endpoint(
    graph: &mut TunnelVisibility,
    occupied: &mut HashMap<(u64, u64), usize>,
    owner: usize,
    point: Point,
) -> usize {
    let index = *occupied.entry(point_key(point)).or_insert_with(|| {
        let index = graph.vertices.len();
        graph.vertices.push(Vertex {
            point,
            owners: Vec::new(),
        });
        index
    });
    let metadata = &mut graph.vertices[index].owners;
    // Unrestricted tunnels have no side flag. An existing canonical port
    // still retains its original side and center status for this owner.
    if let Err(position) = metadata.binary_search_by_key(&owner, |entry| entry.node) {
        metadata.insert(
            position,
            PortOwner {
                node: owner,
                sides: 0,
                center: false,
            },
        );
    }
    graph.ports_by_node[owner].push(index);
    index
}

fn tunnel_points(bounds: &[Bounds], a: usize, b: usize, count: usize) -> Vec<[Point; 2]> {
    let Some((axis, first, second, ranges)) = tunnel_ranges(bounds, a, b, &[], SPACING) else {
        return Vec::new();
    };
    let (from, to) = if first.start > second.start {
        (first.start, second.end)
    } else {
        (first.end, second.start)
    };
    let mut remaining = f64::from(u32::try_from(count).expect("bounded graph edge count"));
    let mut points = Vec::new();
    for r in ranges {
        let length = r.end - r.start;
        let fit = (length / SPACING).floor().min(remaining);
        let mut number = 1.0;
        while number <= fit {
            let place = (r.start + number * length / (fit + 1.0)).round();
            points.push([axis.point(from, place), axis.point(to, place)]);
            number += 1.0;
            remaining -= 1.0;
        }
    }
    points
}

pub(super) fn tunnel_ranges(
    bounds: &[Bounds],
    a: usize,
    b: usize,
    excluded: &[bool],
    minimum_span: f64,
) -> Option<(Axis, Span, Span, Vec<Span>)> {
    let (axis, first, second, overlap) =
        [Axis::Horizontal, Axis::Vertical]
            .into_iter()
            .find_map(|axis| {
                let [first, a_cross] = axis.axes(&bounds[a]);
                let [second, b_cross] = axis.axes(&bounds[b]);
                let overlap = Span {
                    start: a_cross.start.max(b_cross.start),
                    end: a_cross.end.min(b_cross.end),
                };
                (overlap.start <= overlap.end).then_some((axis, first, second, overlap))
            })?;
    let mut ranges = vec![overlap];
    for (index, other) in bounds.iter().enumerate() {
        if index == a || index == b || excluded.get(index) == Some(&true) {
            continue;
        }
        let [along, across] = axis.axes(other);
        let entirely_between = along.start >= first.end && along.end <= second.start
            || along.start >= second.end && along.end <= first.start;
        if entirely_between && across.start <= overlap.start && across.end >= overlap.end {
            return None;
        }
        if first.start.min(second.start) < along.start
            && along.start < first.start.max(second.start)
        {
            ranges = ranges
                .into_iter()
                .flat_map(|range| subtract_obstacle(range, across))
                .flatten()
                .filter(|range| range.end - range.start >= minimum_span)
                .collect();
        }
    }
    Some((axis, first, second, ranges))
}

fn subtract_obstacle(range: Span, obstacle: Span) -> [Option<Span>; 2] {
    if obstacle.end <= range.start || obstacle.start >= range.end {
        return [Some(range), None];
    }
    [
        (obstacle.start > range.start).then_some(Span {
            start: range.start,
            end: obstacle.start,
        }),
        (obstacle.end < range.end).then_some(Span {
            start: obstacle.end,
            end: range.end,
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeGeometry;
    use crate::ovg_seed::rectangular_visibility_seed;

    fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            left: x,
            right: x + width,
            top: y,
            bottom: y + height,
        }
    }

    #[test]
    fn tunnel_simple() {
        let a = rectangle(0.0, 0.0, 100.0, 100.0);
        let b = rectangle(300.0, 0.0, 100.0, 100.0);
        let [from, to] = tunnel_points(&[a, b], 0, 1, 1)[0];
        assert_eq!(from, Point { x: 100.0, y: 50.0 });
        assert_eq!(to, Point { x: 300.0, y: 50.0 });
        let mut graph = TunnelVisibility {
            vertices: Vec::new(),
            ports_by_node: vec![Vec::new(), Vec::new()],
            tunnel_links: Vec::new(),
        };
        let mut occupied = HashMap::new();
        for (node, point) in [from, to].into_iter().enumerate() {
            let index = add_endpoint(&mut graph, &mut occupied, node, point);
            assert_eq!(
                graph.vertices[index].owners,
                [PortOwner {
                    node,
                    sides: 0,
                    center: false
                }]
            );
        }
        let a = rectangle(0.0, 0.0, 100.0, 100.0);
        let b = rectangle(300.0, 300.0, 100.0, 100.0);
        assert!(tunnel_points(&[a, b], 0, 1, 1).is_empty());
    }

    #[test]
    fn tunnel_obscured() {
        for (blocker_y, count) in [(0.0, 0), (1.0, 0), (SPACING + 1.0, 1), (-90.0, 1)] {
            let nodes = [
                rectangle(0.0, 0.0, 100.0, 100.0),
                rectangle(500.0, 0.0, 100.0, 100.0),
                rectangle(300.0, blocker_y, 100.0, 100.0),
            ];
            assert_eq!(tunnel_points(&nodes, 0, 1, 1).len(), count);
        }
    }

    #[test]
    fn tunnel_multiple() {
        let mut nodes = vec![
            rectangle(0.0, 0.0, 1000.0, 1000.0),
            rectangle(5000.0, 0.0, 1000.0, 1000.0),
        ];
        let count = |nodes: &[Bounds]| tunnel_points(nodes, 0, 1, 3).len();
        assert_eq!(count(&nodes), 3);
        nodes[1] = rectangle(5000.0, 1000.0 - 2.0 * SPACING, 1000.0, 1000.0);
        assert_eq!(count(&nodes), 2);
        nodes[1] = rectangle(5000.0, 0.0, 1000.0, 1000.0);
        nodes.push(rectangle(3000.0, SPACING, 1000.0, 1000.0 - 2.0 * SPACING));
        assert_eq!(count(&nodes), 2);
        nodes.push(rectangle(3000.0, 0.0, 1000.0, SPACING));
        assert_eq!(count(&nodes), 1);
    }

    #[test]
    fn tunnel_keeps_existing_port_directions() {
        let nodes = [0.0, 300.0].map(|x| NodeGeometry {
            id: String::new(),
            x,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        });
        let seed = rectangular_visibility_seed(&nodes).unwrap();
        let bounds = [0.0, 300.0].map(|x| rectangle(x, 0.0, 100.0, 100.0));
        let expected = [seed.ports_by_node[0][10], seed.ports_by_node[1][4]];
        let graph = add(seed, &bounds, &[[0, 1]]);
        assert_eq!(graph.tunnel_links, [expected]);
        for (node, side) in [8, 2].into_iter().enumerate() {
            let port = graph.ports_by_node[node][12];
            assert_eq!(port, expected[node]);
            assert_eq!(
                graph.vertices[port].owners,
                [PortOwner {
                    node,
                    sides: side,
                    center: true
                }]
            );
        }
    }
}
