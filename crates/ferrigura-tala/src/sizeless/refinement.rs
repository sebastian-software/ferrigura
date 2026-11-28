// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/stages.go,
// internal/placementcost/graph.go and internal/layoutgraph/segment_crossing.go.

use std::collections::HashMap;

use serde::Serialize;

use super::compaction::Axis;
use super::sized_cost::{Bounds, EdgeCost};
use super::transition::{Spacing, edge_gaps, end, overlaps_at, start};
use super::{MAX_OPTIMIZATION_WORK, Work};
use crate::{Error, Graph, MAX_EDGES, MAX_NODES, NodeGeometry, Point, valid_dimension};

mod axis;
mod equidistance;

#[cfg(test)]
mod tests;

const MAX_GRAPH_SIZE: f64 = 30_000.0;
const CROSSING_COST_WEIGHT: f64 = 0.110_592;
const TURN_PENALTY_MULTIPLIER: f64 = 0.125;
const ROUTING_PADDING: f64 = 1000.0;

#[derive(Debug, Serialize)]
pub struct RefinementSnapshot {
    stage: Stage,
    nodes: Vec<NodeGeometry>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum Stage {
    AlignAxes,
    BalanceSymmetry,
    Equidistance,
    FinalAlignAxes,
    Rescale,
}

/// Runs rectangular alignment, symmetry, and equidistance, then adds routing
/// padding. Feed it the body placement for one component that has finished
/// swap, transpose, and gap normalization. The order indexes authored siblings
/// into the prepared node order.
///
/// # Errors
/// Returns an error for invalid bodies or exhausted refinement work.
pub fn refine_flat_component(
    graph: &Graph,
    bodies: &[NodeGeometry],
    obstacle_order: &[usize],
) -> Result<Vec<NodeGeometry>, Error> {
    refine(graph, bodies, obstacle_order, |_, _| {})
}

/// Records each geometric refinement stage and the routing padding.
///
/// # Errors
/// Returns the same errors as the rectangular refinements.
pub fn refinement_snapshots(
    graph: &Graph,
    bodies: &[NodeGeometry],
    obstacle_order: &[usize],
) -> Result<Vec<RefinementSnapshot>, Error> {
    let mut snapshots = Vec::new();
    refine(graph, bodies, obstacle_order, |stage, nodes| {
        snapshots.push(RefinementSnapshot {
            stage,
            nodes: nodes.to_vec(),
        });
    })?;
    Ok(snapshots)
}

fn refine(
    graph: &Graph,
    bodies: &[NodeGeometry],
    obstacle_order: &[usize],
    mut observe: impl FnMut(Stage, &[NodeGeometry]),
) -> Result<Vec<NodeGeometry>, Error> {
    let mut pass = Refinement::new(graph, bodies, obstacle_order)?;
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    pass.align(&mut work)?;
    observe(Stage::AlignAxes, &pass.nodes);
    pass.balance_symmetry(&mut work)?;
    observe(Stage::BalanceSymmetry, &pass.nodes);
    let moved = pass.equidistance(&mut work)?;
    observe(Stage::Equidistance, &pass.nodes);
    if moved {
        pass.align(&mut work)?;
    }
    observe(Stage::FinalAlignAxes, &pass.nodes);
    for node in &mut pass.nodes {
        node.x += ROUTING_PADDING;
        node.y += ROUTING_PADDING;
    }
    observe(Stage::Rescale, &pass.nodes);
    Ok(pass.nodes)
}

struct Refinement<'a> {
    nodes: Vec<NodeGeometry>,
    order: &'a [usize],
    edges: Vec<[usize; 2]>,
    adjacency: Vec<Vec<usize>>,
    gaps: Vec<Vec<Spacing>>,
    scorer: EdgeCost,
    crossing_cost: f64,
}

impl<'a> Refinement<'a> {
    fn new(
        graph: &Graph,
        bodies: &[NodeGeometry],
        obstacle_order: &'a [usize],
    ) -> Result<Self, Error> {
        if graph.edges.len() > MAX_EDGES || graph.nodes.len() > MAX_NODES {
            return Err(Error::WorkLimit);
        }
        if graph.nodes.len() != bodies.len()
            || bodies.is_empty()
            || graph
                .nodes
                .iter()
                .zip(bodies)
                .any(|(node, body)| node.id != body.id || node.is_group || node.parent_id.is_some())
        {
            return Err(Error::UnsupportedGraph);
        }
        if bodies.iter().any(|node| {
            !node.x.is_finite()
                || !node.y.is_finite()
                || !valid_dimension(node.width)
                || !valid_dimension(node.height)
        }) {
            return Err(Error::InvalidGraph);
        }
        let indices: HashMap<_, _> = bodies
            .iter()
            .enumerate()
            .map(|(i, body)| (body.id.as_str(), i))
            .collect();
        if indices.len() != bodies.len() {
            return Err(Error::InvalidGraph);
        }
        let mut adjacency = vec![Vec::new(); bodies.len()];
        let mut edges = Vec::with_capacity(graph.edges.len());
        for edge in &graph.edges {
            let a = *indices
                .get(edge.source.as_str())
                .ok_or(Error::InvalidGraph)?;
            let b = *indices
                .get(edge.target.as_str())
                .ok_or(Error::InvalidGraph)?;
            if a == b {
                return Err(Error::UnsupportedGraph);
            }
            adjacency[a].push(b);
            adjacency[b].push(a);
            edges.push([a, b]);
        }
        let gaps = edge_gaps(graph)?;
        let cell = cell_size(bodies);
        let mut scorer = EdgeCost::new(graph, bodies, cell, obstacle_order)?;
        scorer.restore_full_turn_cost();
        let crossing_cost = CROSSING_COST_WEIGHT * (scorer.turn_cost() / TURN_PENALTY_MULTIPLIER);
        Ok(Self {
            nodes: bodies.to_vec(),
            order: obstacle_order,
            edges,
            adjacency,
            gaps,
            scorer,
            crossing_cost,
        })
    }

    fn score(&self, work: &mut Work) -> Result<f64, Error> {
        let mut total = 0.0;
        for &node in self.order {
            total += self.scorer.score(node, &self.nodes, work)?;
        }
        let crossings = self.crossings(work)?;
        Ok(total + self.crossing_cost * f64::from(crossings))
    }

    fn crossings(&self, work: &mut Work) -> Result<u32, Error> {
        let mut count = 0;
        for (i, &[a, b]) in self.edges.iter().enumerate() {
            for &[c, d] in &self.edges[i + 1..] {
                work.add(1)?;
                if [c, d].contains(&a) || [c, d].contains(&b) {
                    continue;
                }
                if segments_cross(
                    self.center(a),
                    self.center(b),
                    self.center(c),
                    self.center(d),
                ) {
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    fn component(
        &self,
        start: usize,
        blocked: &[usize],
        work: &mut Work,
    ) -> Result<Vec<usize>, Error> {
        let mut stack = vec![start];
        let mut seen = vec![false; self.nodes.len()];
        for &node in blocked {
            seen[node] = true;
        }
        let mut nodes = Vec::new();
        while let Some(node) = stack.pop() {
            work.add(1)?;
            if seen[node] {
                continue;
            }
            seen[node] = true;
            nodes.push(node);
            stack.extend(self.adjacency[node].iter().rev().copied());
        }
        Ok(nodes)
    }

    fn center(&self, index: usize) -> Point {
        Bounds::from(&self.nodes[index]).center()
    }

    fn orientation(&self, a: usize, b: usize) -> Option<i32> {
        Bounds::from(&self.nodes[a]).direction_relative_to(Bounds::from(&self.nodes[b]))
    }

    fn translate(&mut self, moving: &[usize], delta: Point) {
        for &index in moving {
            self.nodes[index].x += delta.x;
            self.nodes[index].y += delta.y;
        }
    }

    fn validate_move(
        &self,
        before: &[NodeGeometry],
        moving: &[usize],
        work: &mut Work,
    ) -> Result<bool, Error> {
        for &a in moving {
            for b in 0..self.nodes.len() {
                work.add(1)?;
                if a == b || moving.contains(&b) {
                    continue;
                }
                let previous = &before[a];
                let old = &before[b];
                let current = &self.nodes[a];
                let other = &self.nodes[b];
                let spacing = self.gaps[a][b];
                if !overlaps_at(
                    current,
                    other,
                    Point {
                        x: current.x,
                        y: current.y,
                    },
                    spacing,
                ) {
                    continue;
                }
                if !overlaps_at(
                    previous,
                    old,
                    Point {
                        x: previous.x,
                        y: previous.y,
                    },
                    spacing,
                ) {
                    return Ok(false);
                }
                if exact_overlap(current, other) && !exact_overlap(previous, old) {
                    return Ok(false);
                }
            }
        }
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let min = self
                .nodes
                .iter()
                .map(|node| start(axis, node))
                .fold(f64::INFINITY, f64::min)
                .round();
            let max = self
                .nodes
                .iter()
                .map(|node| end(axis, node))
                .fold(f64::NEG_INFINITY, f64::max)
                .round();
            if max - min > MAX_GRAPH_SIZE {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn cell_size(nodes: &[NodeGeometry]) -> f64 {
    let mut shortest = f64::INFINITY;
    let mut longest = f64::NEG_INFINITY;
    for n in nodes {
        shortest = shortest.min(n.width).min(n.height);
        longest = longest.max(n.width).max(n.height);
    }
    let cell = if longest < 3.0 * shortest {
        longest
    } else {
        3.0 * shortest / 2.0
    };
    cell.ceil().max(10.0)
}

fn exact_overlap(a: &NodeGeometry, b: &NodeGeometry) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn segments_cross(u0: Point, u1: Point, v0: Point, v1: Point) -> bool {
    let denominator = (u1.y - u0.y) * (v1.x - v0.x) - (u1.x - u0.x) * (v1.y - v0.y);
    if denominator == 0.0 {
        return false;
    }
    let s = ((v1.x - v0.x) * (v0.y - u0.y) - (v1.y - v0.y) * (v0.x - u0.x)) / denominator;
    let t = ((u1.x - u0.x) * (v0.y - u0.y) - (u1.y - u0.y) * (v0.x - u0.x)) / denominator;
    (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t)
}
