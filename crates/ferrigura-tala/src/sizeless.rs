// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// internal/placement/{node_placement,optimization_guard,sizeless_optimizer}.go
// and internal/placementcost/{edge_length,geometry}.go.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::go_rand::GoRand;
use crate::initialize::{
    Cell, Grid, Neighbor, edge_cost, initialize, placement_adjacency, preferred_direction,
};
use crate::stress_initialize::initialize_by_graph_distance;
use crate::{Error, Graph};

mod compaction;

const ITERATIONS_PER_SQRT_NODE: f64 = 90.0;
const COMPACTION_ITERATION: usize = 9;
const MAX_OPTIMIZATION_WORK: usize = 250_000_000;
const MAX_SEARCH_DISTANCE: i32 = 100;
const PRECISION: f64 = 0.0001;

struct Work(usize);

impl Work {
    fn add(&mut self, count: usize) -> Result<(), Error> {
        self.0 = self.0.checked_sub(count).ok_or(Error::WorkLimit)?;
        Ok(())
    }
}

/// Executes the seed-dependent first sizeless placement pass for an already
/// preprocessed flat placement component. The output precedes compaction.
///
/// # Errors
/// Returns an error for unsupported component topology or invalid measurements,
/// exhausted work, or a placement grid that lacks a candidate.
pub fn first_pass(graph: &Graph, seed: i64) -> Result<Grid, Error> {
    run(graph, seed, true)
}

/// Executes the entire sizeless phase including alternating-axis compaction.
///
/// # Errors
/// Returns an error for unsupported component topology or invalid measurements,
/// exhausted work, or a placement grid that lacks a candidate.
pub fn optimize_phase(graph: &Graph, seed: i64) -> Result<Grid, Error> {
    run(graph, seed, false)
}

fn run(graph: &Graph, seed: i64, first_pass_only: bool) -> Result<Grid, Error> {
    let mut grid = initialize(graph)?;
    if seed % 2 == 0
        && let Some(points) = initialize_by_graph_distance(graph)?
    {
        for (cell, point) in grid.nodes.iter_mut().zip(points) {
            (cell.x, cell.y) = (round_cell(point.x), round_cell(point.y));
        }
    }
    let size = f64::from(u32::try_from(grid.nodes.len()).map_err(|_| Error::WorkLimit)?).sqrt();
    let mut temperature = 2.0 * size;
    let iteration_count = ITERATIONS_PER_SQRT_NODE * size;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let passes = if first_pass_only {
        1
    } else {
        (iteration_count as usize) / 2
    };
    let cooling = (0.2 / temperature).powf(1.0 / iteration_count.floor());
    let mut rng = GoRand::new(seed);
    let adjacency = placement_adjacency(graph)?;
    let direction = preferred_direction(graph)?;
    let mut positions = grid
        .nodes
        .iter()
        .map(|node| (node.x, node.y))
        .collect::<Vec<_>>();
    let mut axis = compaction::Axis::Horizontal;
    for pass in 0..passes {
        optimize_pass(&adjacency, &mut positions, direction, temperature, &mut rng)?;
        if !first_pass_only && pass % COMPACTION_ITERATION == 0 {
            compaction::compact(&adjacency, &mut positions, direction, axis)?;
            axis = axis.opposite();
        }
        temperature *= cooling;
    }
    for (node, position) in grid.nodes.iter_mut().zip(positions) {
        (node.x, node.y) = position;
    }
    Ok(grid)
}

fn optimize_pass(
    adjacency: &[Vec<Neighbor>],
    positions: &mut [Cell],
    direction: Option<i32>,
    temperature: f64,
    rng: &mut GoRand,
) -> Result<(), Error> {
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let movable: Vec<_> = adjacency.iter().map(|links| !links.is_empty()).collect();
    let mut order: Vec<_> = movable
        .iter()
        .enumerate()
        .filter_map(|(index, &can_move)| can_move.then_some(index))
        .collect();
    work.add(order.len())?;
    rng.shuffle(&mut order, || work.add(1))?;

    let mut occupied: HashMap<_, _> = positions
        .iter()
        .enumerate()
        .map(|(index, cell)| (*cell, index))
        .collect();
    for node in order {
        work.add(1)?;
        let previous = positions[node];
        // Zero and equal scores compare using the upstream tolerance. The
        // candidate search can move farther than the original node's location.
        occupied.remove(&previous);
        let median = jittered_median(&adjacency[node], positions, temperature, rng, &mut work)?;
        let distance = nearest_distance(median, &occupied, &mut work)?;
        let mut candidates = placement_points(median, distance, &occupied, &mut work)?;
        rng.shuffle(&mut candidates, || work.add(1))?;
        positions[node] = preferred_cell(
            adjacency,
            positions,
            direction,
            node,
            &candidates,
            &mut work,
        )?;

        if positions[node] == previous {
            let mut neighbors = swap_candidates(previous, &movable, &occupied);
            rng.shuffle(&mut neighbors, || work.add(1))?;
            if let Some(neighbor) =
                preferred_swap(adjacency, positions, direction, node, neighbors, &mut work)?
            {
                positions.swap(node, neighbor);
                occupied.insert(positions[neighbor], neighbor);
            }
        }
        occupied.insert(positions[node], node);
    }
    Ok(())
}

fn jittered_median(
    links: &[Neighbor],
    positions: &[Cell],
    temperature: f64,
    rng: &mut GoRand,
    work: &mut Work,
) -> Result<Cell, Error> {
    work.add(links.len())?;
    let mut xs: Vec<_> = links.iter().map(|edge| positions[edge.node].0).collect();
    let mut ys: Vec<_> = links.iter().map(|edge| positions[edge.node].1).collect();
    let median_x = axis_median(&mut xs);
    let median_y = axis_median(&mut ys);
    Ok((
        round_cell(median_x + (-temperature + rng.float64() * (2.0 * temperature))),
        round_cell(median_y + (-temperature + rng.float64() * (2.0 * temperature))),
    ))
}

fn axis_median(coordinates: &mut [i32]) -> f64 {
    coordinates.sort_unstable();
    let middle = coordinates.len() / 2;
    let right = f64::from(coordinates[middle]) + 0.5;
    if coordinates.len().is_multiple_of(2) {
        (right + f64::from(coordinates[middle - 1])) / 2.0
    } else {
        right
    }
}

fn round_cell(value: f64) -> i32 {
    // The flat placement node limit and bounded grid search keep coordinates
    // inside i32.
    #[allow(clippy::cast_possible_truncation)]
    let rounded = value.round() as i32;
    rounded
}

fn nearest_distance(
    median: Cell,
    occupied: &HashMap<Cell, usize>,
    work: &mut Work,
) -> Result<i32, Error> {
    for distance in 0..=MAX_SEARCH_DISTANCE {
        work.add(1)?;
        for x in (-distance..=distance).rev() {
            work.add(1)?;
            let y = distance - x.abs();
            if !occupied.contains_key(&(median.0 + x, median.1 - y))
                || !occupied.contains_key(&(median.0 + x, median.1 + y))
            {
                return Ok(distance);
            }
        }
    }
    Err(Error::UnsupportedGraph)
}

fn placement_points(
    median: Cell,
    nearest: i32,
    occupied: &HashMap<Cell, usize>,
    work: &mut Work,
) -> Result<Vec<Cell>, Error> {
    let distance = nearest + 1;
    let mut points = Vec::new();
    for x in (-distance..=distance).rev() {
        work.add(1)?;
        for y in 0..=distance - x.abs() {
            work.add(1)?;
            let above = (median.0 + x, median.1 - y);
            let below = (median.0 + x, median.1 + y);
            if y != 0 && !occupied.contains_key(&above) {
                points.push(above);
            }
            if !occupied.contains_key(&below) {
                points.push(below);
            }
        }
    }
    Ok(points)
}

fn preferred_cell(
    adjacency: &[Vec<Neighbor>],
    positions: &mut [Cell],
    direction: Option<i32>,
    node: usize,
    candidates: &[Cell],
    work: &mut Work,
) -> Result<Cell, Error> {
    let previous = positions[node];
    let mut best = None;
    let mut lowest = f64::INFINITY;
    for &candidate in candidates {
        work.add(1)?;
        positions[node] = candidate;
        let score = node_score(adjacency, positions, direction, node, work)?;
        let comparison = precision_compare(score, lowest);
        if comparison == Ordering::Less || comparison == Ordering::Equal && candidate == previous {
            best = Some(candidate);
            lowest = score;
        }
    }
    best.ok_or(Error::UnsupportedGraph)
}

fn swap_candidates((x, y): Cell, movable: &[bool], occupied: &HashMap<Cell, usize>) -> Vec<usize> {
    [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
        .iter()
        .filter_map(|cell| occupied.get(cell).copied())
        .filter(|&index| movable[index])
        .collect()
}

fn preferred_swap(
    adjacency: &[Vec<Neighbor>],
    positions: &mut [Cell],
    direction: Option<i32>,
    node: usize,
    candidates: Vec<usize>,
    work: &mut Work,
) -> Result<Option<usize>, Error> {
    let mut lowest = node_score(adjacency, positions, direction, node, work)?;
    let mut best = None;
    for neighbor in candidates {
        work.add(1)?;
        let current = node_score(adjacency, positions, direction, neighbor, work)?;
        positions.swap(node, neighbor);
        let score = node_score(adjacency, positions, direction, node, work)?;
        if precision_compare(score, lowest) == Ordering::Less
            && precision_compare(
                node_score(adjacency, positions, direction, neighbor, work)?,
                current,
            ) != Ordering::Greater
        {
            lowest = score;
            best = Some(neighbor);
        }
        positions.swap(node, neighbor);
    }
    Ok(best)
}

fn node_score(
    adjacency: &[Vec<Neighbor>],
    positions: &[Cell],
    direction: Option<i32>,
    node: usize,
    work: &mut Work,
) -> Result<f64, Error> {
    work.add(adjacency[node].len())?;
    Ok(adjacency[node]
        .iter()
        .map(|&edge| edge_cost(edge, positions[node], positions[edge.node], direction))
        .sum())
}

fn precision_compare(first: f64, second: f64) -> Ordering {
    if (first - second).abs() < PRECISION {
        Ordering::Equal
    } else if first < second {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

#[cfg(test)]
mod tests;
