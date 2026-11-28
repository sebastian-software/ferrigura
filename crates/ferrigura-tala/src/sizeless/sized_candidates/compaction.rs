// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{node_placement,compaction,moves,tuning}.go.

use serde::Serialize;

use super::super::COMPACTION_ITERATION;
use super::super::compaction::{Axis, MAX_CANDIDATES, MAX_ROUNDS};
use super::super::transition::{
    compaction_floor, end, inflate_axis, nearest_predecessor, ordered_along_axis,
    oriented_body_gap, overlaps_at, start, visibility_predecessors,
};
use super::{MAX_OPTIMIZATION_WORK, SizedOptimizer, Work};
use crate::{Error, Graph, NodeGeometry, Point};

const MAX_SIZED_PHASE_WORK: usize = 48_000;
const MAX_QUIET_SIZED_PASSES: usize = 20;

/// Returns the boxes immediately after the first size-aware annealing compaction.
///
/// # Errors
/// Returns an error for unsupported component topology or exhausted placement work.
pub fn first_sized_compaction(
    graph: &Graph,
    seed: i64,
    obstacle_order: &[usize],
) -> Result<Vec<NodeGeometry>, Error> {
    sized_phase(graph, seed, obstacle_order, true, |_, _, _| {})
}

/// Places an already preprocessed flat rectangular component using the entire
/// sized annealing phase, alternating compactions, and temperature-zero passes.
/// Node order drives optimization. The required obstacle-order permutation
/// projects D2's authored sibling scan order into those node indices.
///
/// # Errors
/// Returns an error for unsupported component topology or exhausted placement work.
pub fn optimize_sized_phase(
    graph: &Graph,
    seed: i64,
    obstacle_order: &[usize],
) -> Result<Vec<NodeGeometry>, Error> {
    sized_phase(graph, seed, obstacle_order, false, |_, _, _| {})
}

#[derive(Debug, Serialize)]
pub struct SizedStep {
    step: Step,
    iteration: usize,
    positions: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum Step {
    Pass,
    Compaction,
    SettlingPass,
}

/// Records ordered coordinates after every sized pass and compaction.
///
/// # Errors
/// Returns the same errors as the sized annealing phase.
pub fn sized_phase_snapshots(
    graph: &Graph,
    seed: i64,
    obstacle_order: &[usize],
) -> Result<Vec<SizedStep>, Error> {
    let mut captures = Vec::new();
    sized_phase(
        graph,
        seed,
        obstacle_order,
        false,
        |step, iteration, nodes| {
            captures.push(SizedStep {
                step,
                iteration,
                positions: nodes.iter().map(|node| [node.x, node.y]).collect(),
            });
        },
    )?;
    Ok(captures)
}

fn sized_phase(
    graph: &Graph,
    seed: i64,
    obstacle_order: &[usize],
    compaction_only: bool,
    mut observe: impl FnMut(Step, usize, &[NodeGeometry]),
) -> Result<Vec<NodeGeometry>, Error> {
    let mut optimizer = SizedOptimizer::new(graph, seed, obstacle_order)?;
    let phase = &optimizer.state.phase;
    let mut temperature = phase.temperature;
    let mut cooling = phase.cooling;
    let mut axis = phase.next_compaction;
    let iterations = phase.iterations;
    let first = iterations / 2 + 1;
    let mut end = iterations;
    if !graph.edges.is_empty() {
        let maximum = (MAX_SIZED_PHASE_WORK / (2 * graph.edges.len())).max(1);
        if end - first > maximum {
            end = first + maximum;
            cooling = (0.2 / temperature).powf(1.0 / iteration_number(maximum)?);
        }
    }
    let mut quiet = 0;
    for i in first..end {
        if optimizer.pass(temperature)? {
            quiet = 0;
        } else {
            quiet += 1;
            if quiet >= MAX_QUIET_SIZED_PASSES {
                break;
            }
        }
        observe(Step::Pass, i, &optimizer.state.nodes);
        if i % COMPACTION_ITERATION == 0 {
            let virtual_iteration = first + (i - first) * (iterations - first) / (end - first);
            let remaining = iteration_number(iterations - virtual_iteration)?;
            let factor =
                (1.0 + 2.0 * (remaining - 30.0) / (0.5 * iteration_number(iterations)?)).max(1.0);
            optimizer.compact(axis, factor)?;
            observe(Step::Compaction, i, &optimizer.state.nodes);
            if compaction_only {
                return Ok(optimizer.state.nodes);
            }
            axis = axis.opposite();
        }
        temperature *= cooling;
    }
    if compaction_only {
        return Err(Error::UnsupportedGraph);
    }
    for i in 0..10 {
        let changed = optimizer.pass(0.0)?;
        observe(Step::SettlingPass, i, &optimizer.state.nodes);
        if !changed {
            break;
        }
    }
    Ok(optimizer.state.nodes)
}

fn iteration_number(iterations: usize) -> Result<f64, Error> {
    u32::try_from(iterations)
        .map(f64::from)
        .map_err(|_| Error::WorkLimit)
}

struct SizedCompaction {
    predecessors: Vec<Vec<usize>>,
    axis: Axis,
    factor: f64,
}

impl SizedOptimizer {
    fn compact(&mut self, axis: Axis, factor: f64) -> Result<(), Error> {
        let mut work = Work(MAX_OPTIMIZATION_WORK);
        let predecessors = visibility_predecessors(&self.state.nodes, &self.state.gaps, axis);
        inflate_axis(
            &mut self.state.nodes,
            &self.state.gaps,
            &predecessors,
            axis,
            self.cell,
            factor,
            false,
        );
        let pass = SizedCompaction {
            predecessors,
            axis,
            factor,
        };
        for _ in 0..MAX_ROUNDS {
            if !pass.shift_groups(self, &mut work)? {
                break;
            }
        }
        for _ in 0..MAX_ROUNDS {
            if !pass.shift_nodes(self, &mut work)? {
                break;
            }
        }
        Ok(())
    }
}

impl SizedCompaction {
    fn earliest(&self, nodes: &[NodeGeometry]) -> usize {
        nodes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| start(self.axis, a).total_cmp(&start(self.axis, b)))
            .expect("placement component has a node")
            .0
    }

    fn candidates(
        &self,
        optimizer: &SizedOptimizer,
        node: usize,
        decrease: f64,
        work: &mut Work,
    ) -> Result<Vec<Point>, Error> {
        let nodes = &optimizer.state.nodes;
        let anchor = nearest_predecessor(nodes, &self.predecessors, self.axis, node)
            .unwrap_or_else(|| self.earliest(nodes));
        let first = &nodes[node];
        let behind = &nodes[anchor];
        let across = self.axis.opposite();
        let current = start(self.axis, first);
        let anchor_position = start(self.axis, behind);
        let floor = if anchor_position == current
            || start(across, behind) > end(across, first)
            || end(across, behind) < start(across, first)
        {
            anchor_position / optimizer.cell
        } else {
            let gap = oriented_body_gap(first, behind, optimizer.state.gaps[node][anchor]);
            compaction_floor(behind, self.axis, optimizer.cell, self.factor, gap)
        };
        let ceiling = current / optimizer.cell;
        let mut cursor = floor - decrease;
        if ceiling - cursor + 1.0 > f64::from(MAX_CANDIDATES) {
            return Err(Error::WorkLimit);
        }
        let mut candidates = Vec::new();
        let original = optimizer.position(node);
        while cursor <= ceiling {
            work.add(1)?;
            candidates.push(self.move_to(original, cursor * optimizer.cell));
            cursor += 1.0;
        }
        candidates.push(original);
        Ok(candidates)
    }

    fn shift_groups(&self, optimizer: &mut SizedOptimizer, work: &mut Work) -> Result<bool, Error> {
        let nodes = &optimizer.state.nodes;
        let mut groups = vec![Vec::new(); nodes.len()];
        let mut roots = Vec::new();
        for node in ordered_along_axis(nodes, self.axis) {
            if let Some(anchor) = nearest_predecessor(nodes, &self.predecessors, self.axis, node) {
                groups[anchor].push(node);
            } else {
                groups[node].push(node);
                roots.push(node);
            }
        }
        let global = self.earliest(nodes);
        let mut changed = false;
        for root in roots {
            let group = &groups[root];
            let before = start(self.axis, &optimizer.state.nodes[root]);
            let decrease = if before == start(self.axis, &optimizer.state.nodes[global]) {
                2.0
            } else {
                0.0
            };
            let symmetry_cost = group
                .iter()
                .map(|&node| optimizer.scorer.symmetry_cost(node))
                .sum::<f64>();
            let candidate_moves = self.candidates(optimizer, root, decrease, work)?;
            let mut best_distance = self.group_score(optimizer, group, 0.0, work)?;
            let mut best_delta = 0.0;
            let original = group
                .iter()
                .map(|&node| optimizer.position(node))
                .collect::<Vec<_>>();
            for point in candidate_moves {
                let coordinate = match self.axis {
                    Axis::Horizontal => point.x,
                    Axis::Vertical => point.y,
                };
                let delta = coordinate - before;
                if self.group_would_overlap(optimizer, group, delta, work)? {
                    continue;
                }
                for (&node, &position) in group.iter().zip(&original) {
                    optimizer.move_to(node, self.shift(position, delta));
                }
                let score = self.group_score(optimizer, group, symmetry_cost, work);
                for (&node, &position) in group.iter().zip(&original) {
                    optimizer.move_to(node, position);
                }
                let score = score?;
                if score < best_distance {
                    best_distance = score;
                    best_delta = delta;
                }
            }
            if best_delta != 0.0 {
                changed = true;
                for (&node, &position) in group.iter().zip(&original) {
                    optimizer.move_to(node, self.shift(position, best_delta));
                }
            }
        }
        Ok(changed)
    }

    fn group_would_overlap(
        &self,
        optimizer: &SizedOptimizer,
        group: &[usize],
        delta: f64,
        work: &mut Work,
    ) -> Result<bool, Error> {
        for &node in group {
            let body = &optimizer.state.nodes[node];
            let point = self.shift(optimizer.position(node), delta);
            for (other, other_body) in optimizer.state.nodes.iter().enumerate() {
                work.add(1)?;
                if !group.contains(&other)
                    && overlaps_at(body, other_body, point, optimizer.state.gaps[node][other])
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn group_score(
        &self,
        optimizer: &SizedOptimizer,
        group: &[usize],
        symmetry_cost: f64,
        work: &mut Work,
    ) -> Result<f64, Error> {
        let mut length = 0.0;
        let mut symmetry = 0.0;
        for &node in group {
            length += optimizer.scorer.score(node, &optimizer.state.nodes, work)?;
            if symmetry_cost != 0.0 {
                symmetry += optimizer
                    .scorer
                    .symmetry(node, &optimizer.state.nodes, work)?;
            }
        }
        Ok(length - symmetry * symmetry_cost)
    }

    fn shift_nodes(&self, optimizer: &mut SizedOptimizer, work: &mut Work) -> Result<bool, Error> {
        let mut changed = false;
        for node in ordered_along_axis(&optimizer.state.nodes, self.axis) {
            let candidates = self.candidates(optimizer, node, 0.0, work)?;
            let chosen = optimizer.choose_move(node, &candidates, false, work, None)?;
            changed |= chosen != optimizer.position(node);
            optimizer.move_to(node, chosen);
        }
        Ok(changed)
    }

    fn move_to(&self, point: Point, coordinate: f64) -> Point {
        match self.axis {
            Axis::Horizontal => Point {
                x: coordinate,
                y: point.y,
            },
            Axis::Vertical => Point {
                x: point.x,
                y: coordinate,
            },
        }
    }

    fn shift(&self, point: Point, delta: f64) -> Point {
        match self.axis {
            Axis::Horizontal => self.move_to(point, point.x + delta),
            Axis::Vertical => self.move_to(point, point.y + delta),
        }
    }
}
