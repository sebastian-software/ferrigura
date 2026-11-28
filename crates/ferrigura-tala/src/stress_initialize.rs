// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// d2layouts/d2talalayout/internal/placement/stress_initialize.go.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

use crate::{Error, Graph, Point};

const MIN_NODES: usize = 4;
const MAX_NODES: usize = 64;
const MAX_EDGES: usize = 256;
const SWEEPS: u32 = 48;
const START_LEARNING_RATE: f64 = 0.7;
const END_LEARNING_RATE: f64 = 0.02;

/// Returns the starting grid positions for one unpinned placement subgraph.
///
/// The even-seed pipeline uses this stage before its ordinary optimization.
/// The points follow the supplied node order. Disconnected and out-of-size
/// inputs return `Ok(None)` and use the ordinary initializer upstream.
///
/// # Errors
/// Returns an error for duplicate node IDs or edge endpoints outside the
/// supplied placement subgraph.
pub fn initialize_by_graph_distance(graph: &Graph) -> Result<Option<Vec<Point>>, Error> {
    let count = graph.nodes.len();
    if !(MIN_NODES..=MAX_NODES).contains(&count) || graph.edges.len() > MAX_EDGES {
        return Ok(None);
    }

    let indices: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    if indices.len() != count {
        return Err(Error::InvalidGraph);
    }

    let mut distances = vec![vec![f64::INFINITY; count]; count];
    for (index, row) in distances.iter_mut().enumerate() {
        row[index] = 0.0;
    }
    let mut degrees = vec![0; count];
    for edge in &graph.edges {
        let from = *indices
            .get(edge.source.as_str())
            .ok_or(Error::InvalidGraph)?;
        let to = *indices
            .get(edge.target.as_str())
            .ok_or(Error::InvalidGraph)?;
        degrees[from] += 1;
        if from != to {
            degrees[to] += 1;
            distances[from][to] = 1.0;
            distances[to][from] = 1.0;
        }
    }

    // Connectedness is measured using authored edges, regardless of direction.
    for via in 0..count {
        for from in 0..count {
            for to in 0..count {
                distances[from][to] =
                    distances[from][to].min(distances[from][via] + distances[via][to]);
            }
        }
    }
    if distances
        .iter()
        .flatten()
        .any(|distance| distance.is_infinite())
    {
        return Ok(None);
    }

    let count_integer = u32::try_from(count).expect("subgraph size is bounded");
    let count_f64 = f64::from(count_integer);
    let ring_radius = count_f64.sqrt();
    let mut points: Vec<_> = (0..count_integer)
        .map(|index| {
            let angle = 2.0 * PI * f64::from(index) / count_f64;
            Point {
                x: ring_radius * angle.cos(),
                y: ring_radius * angle.sin(),
            }
        })
        .collect();

    // Minimize squared relative error between Euclidean and graph distances
    // with rotated pair order, keeping the layout RNG untouched.
    for sweep in 0..SWEEPS {
        let progress = f64::from(sweep) / f64::from(SWEEPS - 1);
        let rate = START_LEARNING_RATE * (END_LEARNING_RATE / START_LEARNING_RATE).powf(progress);
        for first in 0..count {
            let i = (first + sweep as usize) % count;
            for second in first + 1..count {
                let j = (second + sweep as usize) % count;
                let mut dx = points[i].x - points[j].x;
                let mut dy = points[i].y - points[j].y;
                let mut length = dx.hypot(dy);
                if length < 1e-9 {
                    dx = 1e-6;
                    dy = 1e-6;
                    length = std::f64::consts::SQRT_2 * 1e-6;
                }
                let distance = distances[i][j];
                let weight = (rate / (distance * distance)).min(1.0);
                let amount = 0.5 * weight * (length - distance) / length;
                let move_x = dx * amount;
                let move_y = dy * amount;
                points[i].x -= move_x;
                points[i].y -= move_y;
                points[j].x += move_x;
                points[j].y += move_y;
            }
        }
    }

    let mut order: Vec<_> = (0..count).collect();
    order.sort_by_key(|&index| Reverse(degrees[index]));
    let mut occupied = HashSet::with_capacity(count);
    for index in order {
        let point = &mut points[index];
        let target_x = point.x * 2.0 + count_f64;
        let target_y = point.y * 2.0 + count_f64;
        let cell = nearest_cell(target_x, target_y, &occupied, count as i32);
        occupied.insert(cell);
        *point = Point {
            x: f64::from(cell.0),
            y: f64::from(cell.1),
        };
    }
    Ok(Some(points))
}

fn nearest_cell(
    target_x: f64,
    target_y: f64,
    occupied: &HashSet<(i32, i32)>,
    max_radius: i32,
) -> (i32, i32) {
    let cx = target_x.round() as i32;
    let cy = target_y.round() as i32;
    let mut best = (cx, cy);
    let mut cost = f64::INFINITY;
    for radius in 0..=max_radius {
        let left = cx - radius;
        let right = cx + radius;
        let top = cy - radius;
        let bottom = cy + radius;
        for x in left..=right {
            for y in top..=bottom {
                if (x != left && x != right && y != top && y != bottom)
                    || occupied.contains(&(x, y))
                {
                    continue;
                }
                let distance =
                    (f64::from(x) - target_x).powi(2) + (f64::from(y) - target_y).powi(2);
                if distance < cost {
                    best = (x, y);
                    cost = distance;
                }
            }
        }
        if cost.is_finite() {
            break;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::{Edge, Node};

    use super::*;

    #[derive(Deserialize)]
    struct Case {
        name: String,
        count: usize,
        edges: Vec<(usize, usize)>,
        expected: Option<Vec<[f64; 2]>>,
    }

    #[test]
    fn matches_the_pinned_go_grid_and_fallback_cases() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("stress_initialize/positions.json")).unwrap();
        for case in cases {
            let graph = Graph {
                direction: None,
                nodes: (0..case.count)
                    .map(|index| Node {
                        id: index.to_string(),
                        parent_id: None,
                        direction: None,
                        is_group: false,
                        shape: "squareRect".into(),
                        width: 100.0,
                        height: 50.0,
                        label: String::new(),
                        label_width: 0.0,
                        label_height: 0.0,
                    })
                    .collect(),
                edges: case
                    .edges
                    .into_iter()
                    .enumerate()
                    .map(|(index, (from, to))| Edge {
                        id: index.to_string(),
                        source: from.to_string(),
                        target: to.to_string(),
                        source_arrow: false,
                        target_arrow: true,
                        label: String::new(),
                        label_width: 0.0,
                        label_height: 0.0,
                    })
                    .collect(),
            };
            let actual = initialize_by_graph_distance(&graph)
                .unwrap()
                .map(|points| points.into_iter().map(|p| [p.x, p.y]).collect());
            assert_eq!(actual, case.expected, "{}", case.name);
        }
    }
}
