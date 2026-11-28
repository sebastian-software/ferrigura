// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg,ovg_fastpath,ovg_node,ovg_lines,tuning}.go.

use std::collections::{HashSet, VecDeque};

use crate::{NodeGeometry, Point};

use super::{Bounds, LinkedVisibility, TunnelVisibility, Vertex};

const CLEARANCE: f64 = 30.0;

#[derive(Clone, Copy)]
enum Line {
    Horizontal,
    Vertical,
}

impl Line {
    fn coordinates(self, point: Point) -> [f64; 2] {
        match self {
            Self::Horizontal => [point.y, point.x],
            Self::Vertical => [point.x, point.y],
        }
    }

    fn obstacle_start(self, bounds: &Bounds) -> f64 {
        match self {
            Self::Horizontal => bounds.left,
            Self::Vertical => bounds.top,
        }
    }

    fn crosses(self, location: f64, bounds: &Bounds) -> bool {
        match self {
            Self::Horizontal => bounds.top <= location && location <= bounds.bottom,
            Self::Vertical => bounds.left <= location && location <= bounds.right,
        }
    }

    fn forward_sides(self) -> [u8; 2] {
        match self {
            Self::Horizontal => [8, 2],
            Self::Vertical => [4, 1],
        }
    }
}

pub(super) fn connect(
    seed: TunnelVisibility,
    bounds: &[Bounds],
    nodes: &[NodeGeometry],
) -> LinkedVisibility {
    let tunnel_link_count = seed.tunnel_links.len();
    let tunnel_vertices: HashSet<_> = seed.tunnel_links.iter().flatten().copied().collect();
    let mut graph = LinkedVisibility {
        vertices: seed.vertices,
        ports_by_node: seed.ports_by_node,
        centers_by_node: Vec::with_capacity(nodes.len()),
        links: seed.tunnel_links,
        tunnel_link_count,
        near_port_owners: Vec::new(),
    };
    for line in [Line::Horizontal, Line::Vertical] {
        connect_lines(&mut graph, bounds, &tunnel_vertices, line);
    }
    connect_centers(&mut graph, nodes);
    remove_isolated(&mut graph);
    graph.near_port_owners = near_ports(&graph);
    graph
}

fn connect_lines(
    graph: &mut LinkedVisibility,
    bounds: &[Bounds],
    tunnel_vertices: &HashSet<usize>,
    line: Line,
) {
    let mut ordered: Vec<_> = (0..graph.vertices.len())
        .filter(|vertex| !tunnel_vertices.contains(vertex))
        .collect();
    let coords = |index: usize| line.coordinates(graph.vertices[index].point);
    ordered.sort_unstable_by(|&a, &b| {
        let [row_a, location_a] = coords(a);
        let [row_b, location_b] = coords(b);
        row_a
            .partial_cmp(&row_b)
            .unwrap()
            .then_with(|| location_a.partial_cmp(&location_b).unwrap())
    });
    let directions = line.forward_sides();
    let link_order = ordered.chunk_by(|&a, &b| coords(a)[0] == coords(b)[0]);
    for candidates in link_order {
        let location = coords(candidates[0])[0];
        let mut obstacles: Vec<_> = bounds
            .iter()
            .enumerate()
            .filter(|(_, bound)| line.crosses(location, bound))
            .collect();
        obstacles.sort_by(|(_, a), (_, b)| {
            line.obstacle_start(a)
                .partial_cmp(&line.obstacle_start(b))
                .unwrap()
        });
        let mut first_obstacle = 0;
        for segment in candidates.windows(2) {
            let [from, to] = [segment[0], segment[1]];
            let a = &graph.vertices[from];
            let b = &graph.vertices[to];
            if ports_face_away(a, b, directions) {
                continue;
            }
            let mut clear = true;
            let search_start = first_obstacle;
            for (offset, &(owner, obstacle)) in obstacles[search_start..].iter().enumerate() {
                if line.obstacle_start(obstacle) > coords(to)[1] {
                    break;
                }
                if obstacle.blocks_axis_segment(a.point, b.point)
                    && !port_faces(a, owner, directions[0])
                    && !port_faces(b, owner, directions[1])
                {
                    clear = false;
                    break;
                }
                first_obstacle = search_start + offset;
            }
            if clear {
                graph.links.push([from, to]);
            }
        }
    }
}

fn ports_face_away(a: &Vertex, b: &Vertex, directions: [u8; 2]) -> bool {
    !a.owners.is_empty()
        && !b.owners.is_empty()
        && !a.owners.iter().any(|first| {
            b.owners.iter().any(|second| {
                first.node == second.node
                    || first.sides & directions[0] != 0 && second.sides & directions[1] != 0
            })
        })
}

fn port_faces(vertex: &Vertex, owner: usize, side: u8) -> bool {
    vertex
        .owners
        .iter()
        .any(|metadata| metadata.node == owner && metadata.sides & side != 0)
}

fn connect_centers(graph: &mut LinkedVisibility, nodes: &[NodeGeometry]) {
    for (node, geometry) in nodes.iter().enumerate() {
        let index = graph.vertices.len();
        graph.vertices.push(Vertex {
            point: Point {
                x: geometry.x + geometry.width / 2.0,
                y: geometry.y + geometry.height / 2.0,
            },
            owners: Vec::new(),
        });
        graph.centers_by_node.push(index);
        for &port in &graph.ports_by_node[node] {
            graph.links.push([index, port]);
        }
    }
}

fn remove_isolated(graph: &mut LinkedVisibility) {
    let mut connected = vec![false; graph.vertices.len()];
    for &index in graph.links.iter().flatten() {
        connected[index] = true;
    }
    let mut remap = vec![0; graph.vertices.len()];
    let mut count = 0;
    for (old, present) in connected.iter().copied().enumerate() {
        remap[old] = count;
        count += usize::from(present);
    }
    let mut indices = connected.into_iter();
    graph.vertices.retain(|_| indices.next().unwrap());
    for index in graph
        .links
        .iter_mut()
        .flatten()
        .chain(graph.ports_by_node.iter_mut().flatten())
        .chain(&mut graph.centers_by_node)
    {
        *index = remap[*index];
    }
}

fn near_ports(graph: &LinkedVisibility) -> Vec<Vec<usize>> {
    let mut adjacent: Vec<Vec<usize>> = vec![Vec::new(); graph.vertices.len()];
    for &[a, b] in &graph.links {
        adjacent[a].push(b);
        adjacent[b].push(a);
    }
    let mut owners = vec![Vec::new(); graph.vertices.len()];
    for (owner, ports) in graph.ports_by_node.iter().enumerate() {
        let mut marked = vec![false; graph.vertices.len()];
        for &port in ports {
            let mut seen = HashSet::from([port]);
            let mut queue: VecDeque<_> = adjacent[port].iter().copied().collect();
            let origin = graph.vertices[port].point;
            while let Some(index) = queue.pop_front() {
                let vertex = &graph.vertices[index];
                if vertex.point.x != origin.x && vertex.point.y != origin.y || !seen.insert(index) {
                    continue;
                }
                if !vertex.owners.iter().any(|metadata| metadata.node == owner) {
                    if (vertex.point.x - origin.x).abs() >= CLEARANCE
                        || (vertex.point.y - origin.y).abs() >= CLEARANCE
                    {
                        continue;
                    }
                    marked[index] = true;
                }
                queue.extend(adjacent[index].iter().copied());
            }
        }
        for (vertex, yes) in marked.into_iter().enumerate() {
            if yes {
                owners[vertex].push(owner);
            }
        }
    }
    owners
}
