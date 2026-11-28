// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{swap,transpose,stages,direct}.go.

use super::super::{direct_flat_component, precision_compare, rotate_box_around};
use super::{Refinement, Work};
use crate::{Error, Graph, NodeGeometry, Point};

const SWAP_NEIGHBORHOOD: usize = 6;

struct SwapTrial {
    local: f64,
    global: f64,
    crossings: u32,
    positions: [Point; 2],
}

impl Refinement<'_> {
    pub(super) fn swap_stuff(&mut self, graph: &Graph, work: &mut Work) -> Result<(), Error> {
        for _ in 0..4 {
            if !self.swap_nearby(work)? {
                break;
            }
        }
        let before = self.nodes.clone();
        direct_flat_component(graph, &mut self.nodes)?;
        if self.nodes == before {
            return Ok(());
        }
        let after = self.nodes.clone();
        let result = self
            .validate_move(&before, self.order, work)
            .and_then(|valid| {
                if valid {
                    self.score(work).map(Some)
                } else {
                    Ok(None)
                }
            });
        self.nodes = before;
        if let Some(score) = result?
            && !precision_compare(score, self.score(work)?).is_gt()
        {
            self.nodes = after;
        }
        Ok(())
    }

    fn swap_nearby(&mut self, work: &mut Work) -> Result<bool, Error> {
        let mut changed = false;
        for &node in self.order {
            if self.adjacency[node].is_empty() {
                continue;
            }
            let baseline = self.local_swap_score(node, work)?;
            let global_baseline = self.score(work)?;
            let crossing_baseline = self.crossings(work)?;
            let candidates = self.swap_candidates(node);
            let before = self.nodes.clone();
            let mut best: Option<(usize, SwapTrial)> = None;
            for other in candidates {
                let pair = [node, other];
                let first = &before[node];
                let second = &before[other];
                let straight = [position(second), position(first)];
                let aligned = smart_swap(first, second);
                let regular = self.try_swap(pair, straight, &before, work)?;
                let smart = self.try_swap(pair, aligned, &before, work)?;
                let Some(candidate) = preferred_swap(regular, smart) else {
                    continue;
                };
                if candidate.crossings <= crossing_baseline
                    && (candidate.local == 0.0
                        || precision_compare(candidate.local, baseline).is_lt())
                    && precision_compare(candidate.global, global_baseline).is_lt()
                    && best.as_ref().is_none_or(|(_, prior)| {
                        precision_compare(candidate.global, prior.global).is_lt()
                    })
                {
                    best = Some((other, candidate));
                }
            }
            if let Some((other, trial)) = best {
                self.move_pair([node, other], trial.positions);
                changed = true;
            }
        }
        Ok(changed)
    }

    fn swap_candidates(&self, node: usize) -> Vec<usize> {
        let center = self.center(node);
        let mut candidates = self
            .order
            .iter()
            .copied()
            .filter(|&other| other != node)
            .collect::<Vec<_>>();
        if candidates.len() <= SWAP_NEIGHBORHOOD {
            return candidates;
        }
        let distance = |other| {
            let p = self.center(other);
            let x = p.x - center.x;
            let y = p.y - center.y;
            x * x + y * y
        };
        candidates.sort_by(|&a, &b| distance(a).total_cmp(&distance(b)));
        candidates.truncate(SWAP_NEIGHBORHOOD);
        self.order
            .iter()
            .copied()
            .filter(|node| candidates.contains(node))
            .collect()
    }

    fn local_swap_score(&self, node: usize, work: &mut Work) -> Result<f64, Error> {
        let edge = self.scorer.score(node, &self.nodes, work)?;
        let symmetry = self.scorer.symmetry(node, &self.nodes, work)?;
        Ok(edge - self.scorer.global_symmetry_credit(node, symmetry))
    }

    fn try_swap(
        &mut self,
        pair: [usize; 2],
        positions: [Point; 2],
        before: &[NodeGeometry],
        work: &mut Work,
    ) -> Result<Option<SwapTrial>, Error> {
        self.move_pair(pair, positions);
        let result = self.validate_move(before, &pair, work).and_then(|valid| {
            if !valid {
                return Ok(None);
            }
            Ok(Some(SwapTrial {
                local: self.local_swap_score(pair[0], work)?,
                global: self.score(work)?,
                crossings: self.crossings(work)?,
                positions,
            }))
        });
        self.nodes.clone_from_slice(before);
        result
    }

    fn move_pair(&mut self, pair: [usize; 2], positions: [Point; 2]) {
        for (index, position) in pair.into_iter().zip(positions) {
            self.nodes[index].x = position.x;
            self.nodes[index].y = position.y;
        }
    }

    pub(super) fn transpose_all(&mut self, work: &mut Work) -> Result<(), Error> {
        for &node in self.order {
            let Some(pivot) = self.transpose_pivot(node, work)? else {
                continue;
            };
            let moving = self.component(node, &[pivot], work)?;
            let before = self.nodes.clone();
            let mut best_score = self.score(work)?;
            let mut best = None;
            for turns in 1..=3 {
                for &index in &moving {
                    let point = rotate_box_around(&before[index], &before[pivot], turns, None);
                    self.nodes[index].x = point.x;
                    self.nodes[index].y = point.y;
                }
                let trial = self
                    .validate_move(&before, &moving, work)
                    .and_then(|valid| {
                        if !valid {
                            return Ok(None);
                        }
                        let score = self.score(work)?;
                        Ok(Some((score, self.nodes.clone())))
                    });
                self.nodes.clone_from(&before);
                if let Some((score, positions)) = trial?
                    && precision_compare(score, best_score).is_lt()
                {
                    best_score = score;
                    best = Some(positions);
                }
            }
            if let Some(positions) = best {
                self.nodes = positions;
            }
        }
        Ok(())
    }

    fn transpose_pivot(&self, node: usize, work: &mut Work) -> Result<Option<usize>, Error> {
        let diagonal = |other| {
            self.orientation(node, other)
                .is_some_and(|angle| angle.abs() % 2 == 1)
        };
        match self.adjacency[node].as_slice() {
            [only] if !diagonal(*only) => Ok(Some(*only)),
            [a, b] if !diagonal(*a) && !diagonal(*b) => {
                let first = self.component(*a, &[node], work)?;
                if first.contains(b) {
                    return Ok(None);
                }
                let second = self.component(*b, &[node], work)?;
                Ok(Some(if first.len() >= second.len() { *a } else { *b }))
            }
            _ => Ok(None),
        }
    }
}

fn preferred_swap(regular: Option<SwapTrial>, smart: Option<SwapTrial>) -> Option<SwapTrial> {
    match (regular, smart) {
        (None, None) => None,
        (Some(trial), None) | (None, Some(trial)) => (trial.global != 0.0).then_some(trial),
        (Some(regular), Some(smart)) => {
            if regular.local == 0.0 && smart.local == 0.0 {
                if regular.global != 0.0 && smart.global == 0.0
                    || regular.global != 0.0
                        && precision_compare(regular.global, smart.global).is_lt()
                {
                    Some(regular)
                } else {
                    (smart.global != 0.0).then_some(smart)
                }
            } else if regular.local == 0.0
                || smart.local != 0.0 && precision_compare(smart.local, regular.local).is_lt()
            {
                Some(smart)
            } else {
                Some(regular)
            }
        }
    }
}

fn position(body: &NodeGeometry) -> Point {
    Point {
        x: body.x,
        y: body.y,
    }
}

fn smart_swap(a: &NodeGeometry, b: &NodeGeometry) -> [Point; 2] {
    let axis = |a: f64, a_size: f64, b: f64, b_size: f64| {
        if a + a_size < b {
            [b + b_size - a_size, a]
        } else if b + b_size < a {
            [b, a + a_size - b_size]
        } else {
            [a, b]
        }
    };
    let [ax, bx] = axis(a.x, a.width, b.x, b.width);
    let [ay, by] = axis(a.y, a.height, b.y, b.height);
    [Point { x: ax, y: ay }, Point { x: bx, y: by }]
}
