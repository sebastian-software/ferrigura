// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{sized_optimizer,transpose}.go.

use super::super::sized_cost::Bounds;
use super::super::transition::oriented_gap;
use super::super::{precision_compare, rotate_box_around};
use super::{CandidateSearch, SizedOptimizer, Work};
use crate::{Error, NodeGeometry, Point};

impl SizedOptimizer {
    pub(super) fn swap(&mut self, first: usize, second: usize) {
        let a = self.position(first);
        let b = self.position(second);
        self.move_to(first, b);
        self.move_to(second, a);
    }

    pub(super) fn best_swap(
        &mut self,
        node: usize,
        work: &mut Work,
    ) -> Result<Option<usize>, Error> {
        let mut best_score = f64::INFINITY;
        let mut best = None;
        let initial = self.move_score(node, work)?;
        let order = self.shuffled_indices(work)?;
        for other in order {
            let own = &self.state.nodes[node];
            let theirs = &self.state.nodes[other];
            if node == other || Bounds::from(own).distance_to(Bounds::from(theirs)) > self.cell {
                continue;
            }
            if self.overlaps(node, self.position(other), Some(other), work)?
                || self.overlaps(other, self.position(node), Some(node), work)?
            {
                continue;
            }
            let initial_other = self.move_score(other, work)?;
            self.swap(node, other);
            let candidate = self.swapped_score(node, other, initial, work);
            self.swap(node, other);
            if let Some(candidate) = candidate?
                && precision_compare(candidate, initial + initial_other).is_lt()
                && precision_compare(candidate, best_score).is_lt()
            {
                best_score = candidate;
                best = Some(other);
            }
        }
        Ok(best)
    }

    fn swapped_score(
        &self,
        node: usize,
        other: usize,
        initial: f64,
        work: &mut Work,
    ) -> Result<Option<f64>, Error> {
        if self.overlaps(node, self.position(node), None, work)?
            || self.overlaps(other, self.position(other), None, work)?
        {
            return Ok(None);
        }
        let moved_score = self.move_score(node, work)?;
        if !precision_compare(moved_score, initial).is_lt() {
            return Ok(None);
        }
        Ok(Some(moved_score + self.move_score(other, work)?))
    }

    fn overlaps(
        &self,
        node: usize,
        point: Point,
        exception: Option<usize>,
        work: &mut Work,
    ) -> Result<bool, Error> {
        let search = CandidateSearch {
            bodies: &self.state.nodes,
            index: node,
            cell: self.cell,
            gaps: &self.state.gaps[node],
        };
        search.overlaps(point, exception, work)
    }

    pub(super) fn transpose(&mut self, node: usize, work: &mut Work) -> Result<bool, Error> {
        let Some((pivot, rotating)) = self.transpose_nodes(node, work)? else {
            return Ok(false);
        };
        let original = self.state.nodes.clone();
        let mut best = None;
        let mut best_score = self.transpose_score(node, work)?;
        for turns in 1..=3 {
            for &other in &rotating {
                let to = rotate_box_around(
                    &self.state.nodes[other],
                    &self.state.nodes[pivot],
                    turns,
                    Some(self.cell),
                );
                self.move_to(other, to);
            }
            let candidate = self
                .transpose_collides(&rotating, &original, work)
                .and_then(|blocked| {
                    if blocked {
                        return Ok(None);
                    }
                    let score = self.transpose_score(node, work)?;
                    Ok(Some((
                        score,
                        rotating
                            .iter()
                            .map(|&index| self.position(index))
                            .collect::<Vec<_>>(),
                    )))
                });
            self.state.nodes.clone_from(&original);
            if let Some((score, positions)) = candidate?
                && precision_compare(score, best_score).is_lt()
            {
                best_score = score;
                best = Some(positions);
            }
        }
        if let Some(positions) = best {
            for (index, point) in rotating.into_iter().zip(positions) {
                self.move_to(index, point);
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn transpose_nodes(
        &self,
        node: usize,
        work: &mut Work,
    ) -> Result<Option<(usize, Vec<usize>)>, Error> {
        let own = Bounds::from(&self.state.nodes[node]);
        let neighbors = &self.adjacency[node];
        let diagonal = |index: usize| {
            own.direction_relative_to(Bounds::from(&self.state.nodes[index]))
                .is_some_and(|direction| direction.abs() % 2 == 1)
        };
        match neighbors.as_slice() {
            [edge] if !diagonal(edge.node) => Ok(Some((edge.node, vec![node]))),
            [a, b] if !diagonal(a.node) && !diagonal(b.node) => {
                let side_a = self.reachable_without(a.node, node, work)?;
                if side_a.contains(&b.node) {
                    return Ok(None);
                }
                let side_b = self.reachable_without(b.node, node, work)?;
                let (pivot, mut rotating) = if side_a.len() >= side_b.len() {
                    (a.node, side_b)
                } else {
                    (b.node, side_a)
                };
                rotating.push(node);
                Ok(Some((pivot, rotating)))
            }
            _ => Ok(None),
        }
    }

    fn reachable_without(
        &self,
        start: usize,
        exclude: usize,
        work: &mut Work,
    ) -> Result<Vec<usize>, Error> {
        let mut queue = vec![start];
        let mut seen = vec![false; self.adjacency.len()];
        seen[start] = true;
        seen[exclude] = true;
        let mut i = 0;
        while let Some(&current) = queue.get(i) {
            for neighbor in &self.adjacency[current] {
                work.add(1)?;
                if !seen[neighbor.node] {
                    seen[neighbor.node] = true;
                    queue.push(neighbor.node);
                }
            }
            i += 1;
        }
        Ok(queue)
    }

    fn transpose_score(&self, node: usize, work: &mut Work) -> Result<f64, Error> {
        let mut score = self.scorer.score(node, &self.state.nodes, work)?;
        for edge in &self.adjacency[node] {
            score += self.scorer.score(edge.node, &self.state.nodes, work)?;
        }
        Ok(score)
    }

    fn transpose_collides(
        &self,
        moving: &[usize],
        before: &[NodeGeometry],
        work: &mut Work,
    ) -> Result<bool, Error> {
        for &node in moving {
            for other in 0..before.len() {
                work.add(1)?;
                if node == other {
                    continue;
                }
                let spacing = self.state.gaps[node][other].gap();
                let overlaps = |bodies: &[NodeGeometry]| {
                    let a = &bodies[node];
                    let b = &bodies[other];
                    let gap = oriented_gap(a.x, a.y, a.width, a.height, b, spacing);
                    a.x < b.x + b.width + gap
                        && a.x + a.width + gap > b.x
                        && a.y < b.y + b.height + gap
                        && a.y + a.height + gap > b.y
                };
                if overlaps(&self.state.nodes) && !overlaps(before) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
