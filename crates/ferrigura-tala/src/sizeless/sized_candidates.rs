// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{sized_optimizer,optimization_guard,types}.go.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::sized_cost::EdgeCost;
use super::transition::{Spacing, Transition, oriented_gap, prepared_transition};
use super::{
    MAX_OPTIMIZATION_WORK, MAX_SEARCH_DISTANCE, PlacementOrder, Work, placement_adjacency,
};
use crate::go_rand::GoRand;
use crate::{Error, Graph, NodeGeometry, Point};

mod adjustments;
mod compaction;
mod join;

pub use compaction::{first_sized_compaction, optimize_sized_phase, sized_phase_snapshots};

#[cfg(test)]
mod tests;

const MAX_PLACEMENT_CANDIDATES: usize = 1_000_000;
const MAX_MOVABLE_CELLS: f64 = 100.0;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SizedCandidates {
    pub node: String,
    pub median: Point,
    pub distance: i32,
    pub points: Vec<Point>,
}

/// Returns the first sized optimizer search, using the remaining Go random stream.
///
/// # Errors
/// Returns an error if the flat rectangular component is unsupported or outside
/// the bounded placement search.
pub fn first_sized_candidates(
    graph: &Graph,
    seed: i64,
    order: PlacementOrder<'_>,
) -> Result<SizedCandidates, Error> {
    let mut optimizer = SizedOptimizer::new(graph, seed, order)?;
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let node = optimizer.first_node(&mut work)?;
    optimizer.candidates(node, optimizer.state.phase.temperature, &mut work)
}

/// Edge lengths at the initial sized candidate positions, in search order.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SizedEdgeCosts {
    pub node: String,
    pub turn_cost: f64,
    pub edge_costs: Vec<Option<f64>>,
    pub symmetry: Vec<Option<f64>>,
}

/// Scores the accessible positions in the first sized candidate search.
///
/// # Errors
/// Returns an error if the component or its search exceeds the placement limits.
pub fn first_sized_edge_costs(
    graph: &Graph,
    seed: i64,
    order: PlacementOrder<'_>,
) -> Result<SizedEdgeCosts, Error> {
    Ok(first_sized_evaluation(graph, seed, order)?.0)
}

/// Returns the location selected by the first sized move.
///
/// # Errors
/// Returns an error if the component or its search exceeds the placement limits.
pub fn first_sized_move(
    graph: &Graph,
    seed: i64,
    order: PlacementOrder<'_>,
) -> Result<Point, Error> {
    let point = first_sized_evaluation(graph, seed, order)?.1;
    Ok(Point {
        x: point.x + 0.0,
        y: point.y + 0.0,
    })
}

fn first_sized_evaluation(
    graph: &Graph,
    seed: i64,
    order: PlacementOrder<'_>,
) -> Result<(SizedEdgeCosts, Point), Error> {
    let mut optimizer = SizedOptimizer::new(graph, seed, order)?;
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let node = optimizer.first_node(&mut work)?;
    let candidates = optimizer.candidates(node, optimizer.state.phase.temperature, &mut work)?;
    let mut evaluation = SizedEdgeCosts {
        node: candidates.node,
        turn_cost: optimizer.scorer.turn_cost(),
        edge_costs: Vec::with_capacity(candidates.points.len()),
        symmetry: Vec::with_capacity(candidates.points.len()),
    };
    let chosen = optimizer.choose_move(
        node,
        &candidates.points,
        false,
        &mut work,
        Some(&mut evaluation),
    )?;
    Ok((evaluation, chosen))
}

/// Returns the boxes after the first sized optimization pass, before compaction.
///
/// # Errors
/// Returns an error for unsupported component topology or exhausted placement work.
pub fn first_sized_pass(
    graph: &Graph,
    seed: i64,
    order: PlacementOrder<'_>,
) -> Result<Vec<NodeGeometry>, Error> {
    let mut optimizer = SizedOptimizer::new(graph, seed, order)?;
    optimizer.pass(optimizer.state.phase.temperature)?;
    Ok(optimizer.state.nodes)
}

struct SizedOptimizer {
    state: Transition,
    adjacency: Vec<Vec<super::Neighbor>>,
    scorer: EdgeCost,
    cell: f64,
}

impl SizedOptimizer {
    fn new(graph: &Graph, seed: i64, order: PlacementOrder<'_>) -> Result<Self, Error> {
        let state = prepared_transition(graph, seed, order.entities)?;
        let cell = state.phase.grid.cell_size;
        let adjacency = placement_adjacency(graph)?;
        let scorer = EdgeCost::new(graph, &state.nodes, cell, order.obstacles)?;
        Ok(Self {
            state,
            adjacency,
            scorer,
            cell,
        })
    }

    fn first_node(&mut self, work: &mut Work) -> Result<usize, Error> {
        self.shuffled_indices(work)?
            .into_iter()
            .find(|&node| self.movable(node))
            .ok_or(Error::UnsupportedGraph)
    }

    fn movable(&self, node: usize) -> bool {
        let body = &self.state.nodes[node];
        body.width <= MAX_MOVABLE_CELLS * self.cell
            && body.height <= MAX_MOVABLE_CELLS * self.cell
            && !self.adjacency[node].is_empty()
    }

    fn shuffled_indices(&mut self, work: &mut Work) -> Result<Vec<usize>, Error> {
        let mut indices = (0..self.state.nodes.len()).collect::<Vec<_>>();
        work.add(indices.len())?;
        self.state.phase.rng.shuffle(&mut indices, || work.add(1))?;
        Ok(indices)
    }

    fn candidates(
        &mut self,
        node: usize,
        temperature: f64,
        work: &mut Work,
    ) -> Result<SizedCandidates, Error> {
        let body = &self.state.nodes[node];
        let mut neighbors = self.adjacency[node]
            .iter()
            .map(|edge| edge.node)
            .collect::<Vec<_>>();
        work.add(neighbors.len())?;
        let median = neighbor_median(
            body,
            &self.state.nodes,
            &mut neighbors,
            self.cell,
            temperature,
            &mut self.state.phase.rng,
        );
        let search = CandidateSearch {
            bodies: &self.state.nodes,
            index: node,
            cell: self.cell,
            gaps: &self.state.gaps[node],
        };
        let distance = search.nearest_distance(median, work)?;
        let mut points = search.placement_points(median, distance, work)?;
        self.state.phase.rng.shuffle(&mut points, || work.add(1))?;
        Ok(SizedCandidates {
            node: body.id.clone(),
            median,
            distance,
            points,
        })
    }

    fn choose_move(
        &mut self,
        node: usize,
        candidates: &[Point],
        must_improve: bool,
        work: &mut Work,
        mut trace: Option<&mut SizedEdgeCosts>,
    ) -> Result<Point, Error> {
        let original = self.position(node);
        let mut best_score = if must_improve {
            self.move_score(node, work)?
        } else {
            f64::INFINITY
        };
        let mut chosen = original;
        for &point in candidates {
            let search = CandidateSearch {
                bodies: &self.state.nodes,
                index: node,
                cell: self.cell,
                gaps: &self.state.gaps[node],
            };
            if point != original && search.occupied(point, work)? {
                if let Some(trace) = &mut trace {
                    trace.edge_costs.push(None);
                    trace.symmetry.push(None);
                }
                continue;
            }
            self.move_to(node, point);
            let scores = self.candidate_score(node, work);
            self.move_to(node, original);
            let (edge, symmetry) = scores?;
            if let Some(trace) = &mut trace {
                trace.edge_costs.push(Some(edge));
                trace.symmetry.push(Some(symmetry));
            }
            let score = edge - symmetry * self.scorer.symmetry_cost(node);
            let comparison = super::precision_compare(score, best_score);
            if comparison.is_lt() || comparison.is_eq() && point == original {
                best_score = score;
                chosen = point;
            }
        }
        if best_score.is_finite() {
            Ok(chosen)
        } else {
            Err(Error::WorkLimit)
        }
    }

    fn candidate_score(&self, node: usize, work: &mut Work) -> Result<(f64, f64), Error> {
        Ok((
            self.scorer.score(node, &self.state.nodes, work)?,
            self.scorer.symmetry(node, &self.state.nodes, work)?,
        ))
    }

    fn move_score(&self, node: usize, work: &mut Work) -> Result<f64, Error> {
        let (edge, symmetry) = self.candidate_score(node, work)?;
        Ok(edge - symmetry * self.scorer.symmetry_cost(node))
    }

    fn position(&self, node: usize) -> Point {
        let body = &self.state.nodes[node];
        Point {
            x: body.x,
            y: body.y,
        }
    }

    fn move_to(&mut self, node: usize, point: Point) {
        // A grid move is applied as a delta to the prior position in Go.
        // Its addition canonicalizes a rounded negative zero at the origin.
        self.state.nodes[node].x = point.x + 0.0;
        self.state.nodes[node].y = point.y + 0.0;
    }

    fn pass(&mut self, temperature: f64) -> Result<bool, Error> {
        let mut work = Work(MAX_OPTIMIZATION_WORK);
        let order = self.shuffled_indices(&mut work)?;
        let mut changed = false;
        for node in order {
            if !self.movable(node) {
                continue;
            }
            let candidates = self.candidates(node, temperature, &mut work)?;
            let point = self.choose_move(
                node,
                &candidates.points,
                temperature == 0.0,
                &mut work,
                None,
            )?;
            if point != self.position(node) {
                changed = true;
                self.move_to(node, point);
            } else if let Some(other) = self.best_swap(node, &mut work)? {
                changed = true;
                self.swap(node, other);
            } else if self.transpose(node, &mut work)? {
                changed = true;
            } else if temperature != 0.0 {
                self.reconsider_hub(node, temperature, &mut work)?;
            }
        }
        Ok(changed)
    }

    fn reconsider_hub(
        &mut self,
        node: usize,
        temperature: f64,
        work: &mut Work,
    ) -> Result<(), Error> {
        let neighbors = &self.adjacency[node];
        let spokes = neighbors
            .iter()
            .filter_map(|edge| (self.adjacency[edge.node].len() == 1).then_some(edge.node))
            .collect::<Vec<_>>();
        if spokes.is_empty() || spokes.len() == neighbors.len() {
            return Ok(());
        }
        let original_neighbors = std::mem::take(&mut self.adjacency[node]);
        self.adjacency[node].extend(
            original_neighbors
                .iter()
                .copied()
                .filter(|edge| !spokes.contains(&edge.node)),
        );
        let original_edges = self.scorer.suppress_spokes(node, &spokes);
        let original_gaps = self.state.gaps[node].clone();
        for spoke in spokes {
            self.state.gaps[node][spoke].edge_count = 0;
        }
        let chosen = self
            .candidates(node, temperature, work)
            .and_then(|candidates| self.choose_move(node, &candidates.points, false, work, None));
        self.state.gaps[node] = original_gaps;
        self.adjacency[node] = original_neighbors;
        self.scorer.restore_incident_edges(node, original_edges);
        self.move_to(node, chosen?);
        // The upstream pass reports the first move, swap and transpose only.
        Ok(())
    }
}

fn neighbor_median(
    body: &NodeGeometry,
    nodes: &[NodeGeometry],
    indices: &mut [usize],
    cell: f64,
    temperature: f64,
    rng: &mut GoRand,
) -> Point {
    let median = |indices: &mut [usize], size: f64, projection: fn(&NodeGeometry) -> (f64, f64)| {
        indices.sort_unstable_by(|&a, &b| {
            let (a_start, a_width) = projection(&nodes[a]);
            let (b_start, b_width) = projection(&nodes[b]);
            (a_start + a_width / 2.0)
                .total_cmp(&(b_start + b_width / 2.0))
                .then(a.cmp(&b))
        });
        let middle = indices.len() / 2;
        let (start, width) = projection(&nodes[indices[middle]]);
        let mut median = start + width / 2.0;
        if indices.len().is_multiple_of(2) {
            let (start, width) = projection(&nodes[indices[middle - 1]]);
            median = (median + start + width / 2.0) / 2.0;
        }
        let extent = size / cell;
        let jitter = -temperature * extent + rng.float64() * (2.0 * temperature * extent);
        ((median / cell + jitter) * cell).round()
    };
    let mut median = median;
    Point {
        x: median(indices, body.width, |body| (body.x, body.width)),
        y: median(indices, body.height, |body| (body.y, body.height)),
    }
}

struct CandidateSearch<'a> {
    bodies: &'a [NodeGeometry],
    index: usize,
    cell: f64,
    gaps: &'a [Spacing],
}

impl CandidateSearch<'_> {
    fn nearest_distance(&self, median: Point, work: &mut Work) -> Result<i32, Error> {
        for distance in 0..=MAX_SEARCH_DISTANCE {
            for x in (-distance..=distance).rev() {
                let y = distance - x.abs();
                if y != 0 && self.ring_point_unoccupied(median, x, -y, work)? {
                    return Ok(distance);
                }
                if self.ring_point_unoccupied(median, x, y, work)? {
                    return Ok(distance);
                }
            }
        }
        Err(Error::WorkLimit)
    }

    fn ring_point_unoccupied(
        &self,
        median: Point,
        x: i32,
        y: i32,
        work: &mut Work,
    ) -> Result<bool, Error> {
        self.around(x, y, |offset| {
            let point = self.rounded(median, offset);
            self.occupied(point, work).map(|occupied| !occupied)
        })
    }

    fn occupied(&self, point: Point, work: &mut Work) -> Result<bool, Error> {
        work.add(self.bodies.len())?;
        if self
            .bodies
            .iter()
            .any(|body| body.x == point.x && body.y == point.y)
        {
            return Ok(true);
        }
        self.overlaps(point, None, work)
    }

    fn overlaps(
        &self,
        point: Point,
        exception: Option<usize>,
        work: &mut Work,
    ) -> Result<bool, Error> {
        work.add(self.bodies.len())?;
        let own = &self.bodies[self.index];
        let right = point.x + own.width;
        let bottom = point.y + own.height;
        for (index, other) in self.bodies.iter().enumerate() {
            if index == self.index || exception == Some(index) {
                continue;
            }
            let other_right = other.x + other.width;
            let other_bottom = other.y + other.height;
            if point.x > other_right + 500.0
                || right + 500.0 < other.x
                || point.y > other_bottom + 500.0
                || bottom + 500.0 < other.y
            {
                continue;
            }
            let gap = oriented_gap(
                point.x,
                point.y,
                own.width,
                own.height,
                other,
                self.gaps[index].gap(),
            );
            if point.x < other_right + gap
                && right + gap > other.x
                && point.y < other_bottom + gap
                && bottom + gap > other.y
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn placement_points(
        &self,
        median: Point,
        distance: i32,
        work: &mut Work,
    ) -> Result<Vec<Point>, Error> {
        let radius = distance + 1;
        let mut seen = HashSet::new();
        let mut points = Vec::new();
        let mut add = |offset| {
            work.add(1)?;
            let point = self.rounded(median, offset);
            let key = (coordinate_bits(point.x), coordinate_bits(point.y));
            if seen.insert(key) {
                if points.len() == MAX_PLACEMENT_CANDIDATES {
                    return Err(Error::WorkLimit);
                }
                points.push(point);
            }
            Ok(false)
        };
        for x in (-radius..=radius).rev() {
            for y in 0..=radius - x.abs() {
                if y != 0 {
                    self.around(x, -y, &mut add)?;
                }
                self.around(x, y, &mut add)?;
            }
        }
        if self.gaps.iter().any(|spacing| {
            spacing.edge_count >= 3 && spacing.minimum.iter().any(|&size| size > self.cell)
        }) {
            // The component's node indices preserve the authored numeric order.
            let own = &self.bodies[self.index];
            for (neighbor, spacing) in self.bodies.iter().zip(self.gaps) {
                let [width, height] = spacing.minimum;
                let x = neighbor.x - median.x;
                let y = neighbor.y - median.y;
                let cell = self.cell;
                if width > cell {
                    add(Point {
                        x: ((x - own.width - width) / cell).floor() * cell,
                        y,
                    })?;
                    add(Point {
                        x: ((x + neighbor.width + width) / cell).ceil() * cell,
                        y,
                    })?;
                }
                if height > cell {
                    add(Point {
                        x,
                        y: ((y - own.height - height) / cell).floor() * cell,
                    })?;
                    add(Point {
                        x,
                        y: ((y + neighbor.height + height) / cell).ceil() * cell,
                    })?;
                }
            }
        }
        Ok(points)
    }

    fn around(
        &self,
        x: i32,
        y: i32,
        mut test: impl FnMut(Point) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let body = &self.bodies[self.index];
        let x = f64::from(x) * self.cell;
        let y = f64::from(y) * self.cell;
        if test(Point { x, y })? {
            return Ok(true);
        }
        let mut at_x = x - body.width;
        while at_x <= x {
            let mut at_y = y - body.height;
            while at_y <= y {
                if (at_x != x || at_y != y) && test(Point { x: at_x, y: at_y })? {
                    return Ok(true);
                }
                at_y += self.cell;
            }
            at_x += self.cell;
        }
        Ok(false)
    }

    fn rounded(&self, median: Point, offset: Point) -> Point {
        Point {
            x: ((median.x + offset.x) / self.cell).round() * self.cell,
            y: ((median.y + offset.y) / self.cell).round() * self.cell,
        }
    }
}

fn coordinate_bits(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
