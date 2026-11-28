// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Stage probe ported from TALA at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// internal/placement/{initialize,metrics,sizeless_optimizer}.go,
// internal/layoutgraph/{node,graph}.go, internal/placementcost/{edge_length,geometry}.go.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{Error, Graph, MAX_EDGES, MAX_NODES, valid_dimension};

pub(crate) type Cell = (i32, i32);

#[derive(Clone, Copy)]
pub(crate) struct Neighbor {
    pub(crate) node: usize,
    authored_from_here: bool,
    directed: bool,
    incoming: bool,
}

/// Integer grid locations produced before annealing or size-aware compaction.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grid {
    pub cell_size: f64,
    pub nodes: Vec<GridNode>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct GridNode {
    pub id: String,
    pub x: i32,
    pub y: i32,
}

/// Replays ordinary sizeless initialization on a preprocessed flat component.
///
/// Supply the ordered nodes and incident edges of a plain component without
/// proximity or placement constraints, using its post-prescale measurements.
/// Odd seeds and even seeds ineligible for graph-distance initialization use
/// this stage. Its cell coordinates precede stochastic optimization and routing.
///
/// # Errors
/// Returns an error for invalid measurements, unsupported topology, or inputs
/// exceeding this replay's work limits.
pub fn initialize(graph: &Graph) -> Result<Grid, Error> {
    if graph.nodes.len() > MAX_NODES || graph.edges.len() > MAX_EDGES {
        return Err(Error::WorkLimit);
    }
    if graph.nodes.is_empty()
        || graph.nodes.iter().any(|node| {
            node.parent_id.is_some()
                || node.is_group
                || !matches!(node.shape.as_str(), "rect" | "rectangle" | "squareRect")
        })
    {
        return Err(Error::UnsupportedGraph);
    }
    if graph.nodes.iter().any(|node| {
        node.id.is_empty()
            || node.width == 0.0
            || node.height == 0.0
            || !valid_dimension(node.width)
            || !valid_dimension(node.height)
    }) {
        return Err(Error::InvalidGraph);
    }
    let direction = preferred_direction(graph)?;
    let adjacency = placement_adjacency(graph)?;

    let mut positions = vec![None; graph.nodes.len()];
    let start = i32::try_from(graph.nodes.len()).map_err(|_| Error::WorkLimit)?;
    let origin = (start, start);
    positions[0] = Some(origin);
    let mut occupied = HashSet::from([origin]);

    let mut queue = VecDeque::from([0]);
    let mut seen = vec![false; graph.nodes.len()];
    seen[0] = true;
    while let Some(index) = queue.pop_front() {
        for neighbor in &adjacency[index] {
            if !seen[neighbor.node] {
                seen[neighbor.node] = true;
                queue.push_back(neighbor.node);
            }
        }
        if index == 0 {
            continue;
        }
        let cell = place_node(index, &adjacency[index], &positions, &occupied, direction)
            .ok_or(Error::UnsupportedGraph)?;
        positions[index] = Some(cell);
        occupied.insert(cell);
    }
    let dimensions = graph
        .nodes
        .iter()
        .flat_map(|node| [node.width, node.height]);
    let smaller = dimensions.clone().fold(f64::INFINITY, f64::min);
    let larger = dimensions.fold(0.0, f64::max);
    let lattice = if larger < 3.0 * smaller {
        larger
    } else {
        1.5 * smaller
    };
    let nodes = graph
        .nodes
        .iter()
        .zip(positions)
        .map(|(node, position)| {
            let (x, y) = position.ok_or(Error::UnsupportedGraph)?;
            Ok(GridNode {
                id: node.id.clone(),
                x,
                y,
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(Grid {
        cell_size: lattice.ceil().max(10.0),
        nodes,
    })
}

pub(crate) fn preferred_direction(graph: &Graph) -> Result<Option<i32>, Error> {
    let direction = match graph.direction.as_deref() {
        Some("TB" | "TD") => Some(4),
        Some("BT") => Some(0),
        Some("LR") => Some(2),
        Some("RL") => Some(-2),
        None | Some("") => None,
        _ => return Err(Error::InvalidGraph),
    };
    Ok(direction)
}

pub(crate) fn placement_adjacency(graph: &Graph) -> Result<Vec<Vec<Neighbor>>, Error> {
    let ids: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    if ids.len() != graph.nodes.len() {
        return Err(Error::InvalidGraph);
    }

    let mut adjacency = vec![Vec::new(); graph.nodes.len()];
    for edge in &graph.edges {
        let from = *ids.get(edge.source.as_str()).ok_or(Error::InvalidGraph)?;
        let to = *ids.get(edge.target.as_str()).ok_or(Error::InvalidGraph)?;
        let directed = edge.source_arrow != edge.target_arrow;
        adjacency[from].push(Neighbor {
            node: to,
            authored_from_here: true,
            directed,
            incoming: directed && (edge.source_arrow || from == to),
        });
        if from != to {
            adjacency[to].push(Neighbor {
                node: from,
                authored_from_here: false,
                directed,
                incoming: directed && edge.target_arrow,
            });
        }
    }
    Ok(adjacency)
}

fn place_node(
    index: usize,
    neighbors: &[Neighbor],
    positions: &[Option<Cell>],
    occupied: &HashSet<Cell>,
    direction: Option<i32>,
) -> Option<Cell> {
    let positioned_neighbors: Vec<_> = neighbors
        .iter()
        .filter_map(|neighbor| positions[neighbor.node])
        .collect();
    let mut x_coordinates: Vec<_> = positioned_neighbors.iter().map(|&(x, _)| x).collect();
    let mut y_coordinates: Vec<_> = positioned_neighbors.iter().map(|&(_, y)| y).collect();
    let median = (
        coordinate_median(&mut x_coordinates)?,
        coordinate_median(&mut y_coordinates)?,
    );
    let search_limit = i32::try_from(occupied.len()).ok()?;
    let radius = (0..=search_limit).find(|&distance| {
        (-distance..=distance).any(|x| {
            let y = distance - x.abs();
            !occupied.contains(&(median.0 + x, median.1 - y))
                || !occupied.contains(&(median.0 + x, median.1 + y))
        })
    })? + 2;
    let all_deltas = -radius..=radius;
    let incoming = neighbors
        .iter()
        .filter(|neighbor| neighbor.incoming)
        .count();
    let outgoing = neighbors
        .iter()
        .filter(|neighbor| neighbor.directed && !neighbor.incoming)
        .count();
    let steps: Vec<_> = if incoming > outgoing {
        all_deltas.rev().collect()
    } else {
        all_deltas.collect()
    };

    let mut best = None;
    let mut least_cost = f64::INFINITY;
    for &x in &steps {
        for &y in &steps {
            let candidate = (median.0 + x, median.1 + y);
            if occupied.contains(&candidate) {
                continue;
            }
            let cost: f64 = neighbors
                .iter()
                .filter_map(|&neighbor| {
                    // A loop has no position in the median, but scoring sees
                    // its endpoint at the candidate being evaluated.
                    let position = if neighbor.node == index {
                        Some(candidate)
                    } else {
                        positions[neighbor.node]
                    }?;
                    Some(edge_cost(neighbor, candidate, position, direction))
                })
                .sum();
            if cost < least_cost {
                least_cost = cost;
                best = Some(candidate);
            }
        }
    }
    best
}

fn coordinate_median(coordinates: &mut [i32]) -> Option<i32> {
    coordinates.sort_unstable();
    let middle = coordinates.len() / 2;
    let right = *coordinates.get(middle)?;
    if !coordinates.len().is_multiple_of(2) {
        Some(right)
    } else {
        // Upstream adds 0.5 to the upper middle entry before taking their
        // mean and flooring it. On integer cells this gives floor((a+b)/2).
        Some((right + coordinates[middle - 1]).div_euclid(2))
    }
}

pub(crate) fn edge_cost(
    neighbor: Neighbor,
    here: Cell,
    there: Cell,
    preferred: Option<i32>,
) -> f64 {
    let goes_from_here = if neighbor.directed {
        !neighbor.incoming
    } else {
        neighbor.authored_from_here
    };
    let (source, target) = if goes_from_here {
        (here, there)
    } else {
        (there, here)
    };
    let (dx, dy) = (target.0 - source.0, target.1 - source.1);
    let axis_distance = f64::from(dx.abs().min(dy.abs())) / 20.0;
    let direct_distance = f64::from(dx * dx + dy * dy).sqrt();
    let (preferred, weight) = match preferred {
        Some(direction) => (direction, 1.5),
        // Absent an authored axis, directed edges weakly prefer bottom-right.
        None if neighbor.directed => (3, 0.3),
        None => return direct_distance + axis_distance,
    };
    let compass = compass(dx, dy);
    let mut delta = f64::from(((compass - preferred + 12).rem_euclid(8) - 4).abs());
    if !neighbor.directed {
        let difference = (compass + 4).rem_euclid(4) - (preferred + 4).rem_euclid(4);
        let axis = f64::from(if difference == 3 { 1 } else { difference.abs() });
        delta = 0.1 * delta + 0.9 * axis;
    }
    direct_distance + axis_distance + weight * delta * 0.25
}

fn compass(dx: i32, dy: i32) -> i32 {
    match (dx.signum(), dy.signum()) {
        (-1, -1) => -1,
        (-1, 0) => -2,
        (-1, 1) => -3,
        (0, 1) => 4,
        (1, 1) => 3,
        (1, 0) => 2,
        (1, -1) => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
