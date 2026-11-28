// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{gapreduction,stages,tuning}.go.

use super::super::transition::oriented_body_gap;
use super::{Axis, Refinement, Work, cell_size, end, start};
use crate::{Error, NodeGeometry, Point};

const LARGE_GAP_THRESHOLD: f64 = 0.5;
const IDEAL_GAP_SIZE: f64 = 150.0;

#[derive(Clone, Copy)]
struct Traversal {
    axis: Axis,
    forwards: bool,
}

impl Traversal {
    fn reverse(self) -> Self {
        Self {
            forwards: !self.forwards,
            ..self
        }
    }

    fn entry(self, body: &NodeGeometry) -> f64 {
        if self.forwards {
            start(self.axis, body)
        } else {
            -end(self.axis, body)
        }
    }

    fn exit(self, body: &NodeGeometry) -> f64 {
        if self.forwards {
            end(self.axis, body)
        } else {
            -start(self.axis, body)
        }
    }

    fn delta(self, distance: f64) -> Point {
        let distance = if self.forwards { distance } else { -distance };
        match self.axis {
            Axis::Horizontal => Point {
                x: distance,
                y: 0.0,
            },
            Axis::Vertical => Point {
                x: 0.0,
                y: distance,
            },
        }
    }
}

enum GapTrial {
    Blocked,
    NoImprovement,
    Accepted(f64),
}

impl Refinement<'_> {
    pub(super) fn normalize_gaps(&mut self, work: &mut Work) -> Result<bool, Error> {
        let mut order = self.order.to_vec();
        order.sort_by_key(|&node| std::cmp::Reverse(self.adjacency[node].len()));
        let mut accepted = false;
        for axis in [Axis::Horizontal, Axis::Vertical] {
            for forwards in [true, false] {
                let traversal = Traversal { axis, forwards };
                for &node in &order {
                    self.reduce_gap(node, traversal, true, work)?;
                    // A valid no-op has the same transaction outcome as an
                    // accepted move. Both ask the pipeline to align again.
                    accepted = true;
                }
            }
        }
        Ok(accepted)
    }

    fn reduce_gap(
        &mut self,
        behind: usize,
        traversal: Traversal,
        recover_symmetry: bool,
        work: &mut Work,
    ) -> Result<Option<f64>, Error> {
        let Some(initial_ahead) = self.nearest_connected_ahead(behind, traversal) else {
            return Ok(None);
        };
        let connected = self.component(initial_ahead, &[behind], work)?;
        let nearest = self
            .nearest_between(behind, initial_ahead, &connected, traversal)
            .unwrap_or(initial_ahead);
        let before = self.nodes.clone();
        let baseline = self.score_with_direction(work, false)?;
        let threshold = LARGE_GAP_THRESHOLD * cell_size(&self.nodes);
        let candidates = std::iter::once(behind)
            .chain(self.order.iter().copied().filter(|&node| {
                node != behind
                    && node != nearest
                    && !connected.contains(&node)
                    && self.is_gap_blocker(node, behind, nearest, traversal)
            }))
            .collect::<Vec<_>>();
        for candidate in candidates {
            let gap =
                traversal.entry(&self.nodes[nearest]) - traversal.exit(&self.nodes[candidate]);
            if gap <= threshold || gap == IDEAL_GAP_SIZE {
                continue;
            }
            let delta = traversal.delta(IDEAL_GAP_SIZE - gap);
            self.translate(&connected, delta);
            let attempt =
                self.score_gap_trial(behind, &before, baseline, traversal, recover_symmetry, work);
            match attempt {
                Ok(GapTrial::Accepted(score)) => return Ok(Some(score)),
                Ok(GapTrial::Blocked) => self.nodes.clone_from(&before),
                Ok(GapTrial::NoImprovement) => {
                    self.nodes.clone_from(&before);
                    break;
                }
                Err(error) => {
                    self.nodes.clone_from(&before);
                    return Err(error);
                }
            }
        }
        Ok(None)
    }

    fn score_gap_trial(
        &mut self,
        behind: usize,
        before: &[NodeGeometry],
        baseline: f64,
        traversal: Traversal,
        recover_symmetry: bool,
        work: &mut Work,
    ) -> Result<GapTrial, Error> {
        let score = self.score_with_direction(work, false)?;
        let mut accepted = score < baseline;
        if recover_symmetry {
            let forward = self.nodes.clone();
            let reverse = self.reduce_gap(behind, traversal.reverse(), false, work)?;
            if reverse.is_some_and(|reverse| reverse < score && reverse < baseline)
                && self.validate_gap_moves(&forward, work)?
            {
                accepted = true;
            } else {
                self.nodes.clone_from(&forward);
            }
        }
        if !accepted {
            return Ok(GapTrial::NoImprovement);
        }
        if !self.validate_gap_moves(before, work)? {
            return Ok(GapTrial::Blocked);
        }
        // The upstream routine returns the forward score even if reversing
        // the other side also became part of the committed move.
        Ok(GapTrial::Accepted(score))
    }

    fn validate_gap_moves(&self, before: &[NodeGeometry], work: &mut Work) -> Result<bool, Error> {
        let moving = self
            .nodes
            .iter()
            .zip(before)
            .enumerate()
            .filter_map(|(index, (new, old))| (new.x != old.x || new.y != old.y).then_some(index))
            .collect::<Vec<_>>();
        self.validate_move(before, &moving, work)
    }

    fn nearest_connected_ahead(&self, behind: usize, traversal: Traversal) -> Option<usize> {
        let trailing = traversal.exit(&self.nodes[behind]);
        self.adjacency[behind]
            .iter()
            .copied()
            .filter(|&node| traversal.entry(&self.nodes[node]) >= trailing)
            .min_by(|&a, &b| {
                traversal
                    .entry(&self.nodes[a])
                    .total_cmp(&traversal.entry(&self.nodes[b]))
            })
    }

    fn nearest_between(
        &self,
        behind: usize,
        ahead: usize,
        connected: &[usize],
        traversal: Traversal,
    ) -> Option<usize> {
        let trailing = &self.nodes[behind];
        let leading = &self.nodes[ahead];
        let across = traversal.axis.opposite();
        connected
            .iter()
            .copied()
            .filter(|&node| {
                if node == behind || node == ahead {
                    return false;
                }
                let body = &self.nodes[node];
                let gap = oriented_body_gap(trailing, body, self.gaps[behind][node]);
                end(across, body) >= start(across, trailing) - gap
                    && start(across, body) <= end(across, trailing) + gap
                    && traversal.exit(body) >= traversal.exit(trailing)
                    && traversal.entry(body) <= traversal.entry(leading)
            })
            .min_by(|&a, &b| {
                traversal
                    .entry(&self.nodes[a])
                    .total_cmp(&traversal.entry(&self.nodes[b]))
            })
    }

    fn is_gap_blocker(
        &self,
        node: usize,
        behind: usize,
        ahead: usize,
        traversal: Traversal,
    ) -> bool {
        let body = &self.nodes[node];
        let trailing = &self.nodes[behind];
        let leading = &self.nodes[ahead];
        let across = traversal.axis.opposite();
        traversal.entry(body) >= traversal.exit(trailing)
            && traversal.exit(body) <= traversal.entry(leading)
            && start(across, body) <= start(across, trailing).max(start(across, leading))
            && end(across, body) >= end(across, trailing).min(end(across, leading))
    }
}
