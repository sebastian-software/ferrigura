// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{alignment,metrics,symmetry}.go,
// internal/placementcost/axis.go and lib/geo/orientation.go.

use super::super::precision_compare;
use super::{Axis, Bounds, Refinement, Work, end, start};
use crate::{Error, NodeGeometry, Point};

const AXIS_ALIGNMENT_TOLERANCE: f64 = 1.0;
const CONNECTED_NODE_GAP: f64 = 60.0;

impl Refinement<'_> {
    pub(super) fn align(&mut self, work: &mut Work) -> Result<(), Error> {
        for _ in 0..100 {
            let mut changed = false;
            for i in 0..self.edges.len() {
                let [from, to] = self.edges[i];
                let a = self.center(from);
                let b = self.center(to);
                if (a.x - b.x).abs() < AXIS_ALIGNMENT_TOLERANCE
                    || (a.y - b.y).abs() < AXIS_ALIGNMENT_TOLERANCE
                {
                    continue;
                }
                let delta = self.alignment_delta(from, to);
                let mut best_score = self.score(work)?;
                let mut best = None;
                for (pivot, fixed, factor) in [(from, to, 1.0), (to, from, -1.0)] {
                    let moving = self.component(pivot, &[fixed], work)?;
                    let delta = Point {
                        x: delta.x * factor,
                        y: delta.y * factor,
                    };
                    if let Some((score, attempt)) =
                        self.try_align([from, to], &moving, delta, work)?
                        && precision_compare(score, best_score).is_lt()
                    {
                        best_score = score;
                        best = Some((moving, attempt));
                    }
                }
                if let Some((moving, attempt)) = best {
                    self.translate(&moving, attempt);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(())
    }

    fn alignment_delta(&self, from: usize, to: usize) -> Point {
        let a = self.center(from);
        let b = self.center(to);
        let mut delta = Point {
            x: (b.x - a.x).round(),
            y: (b.y - a.y).round(),
        };
        match self.orientation(from, to) {
            Some(-2 | 2) => delta.x = 0.0,
            Some(0 | 4) => delta.y = 0.0,
            _ => {
                let a = &self.nodes[from];
                let b = &self.nodes[to];
                let axis_gap = |axis| {
                    (start(axis, a) - end(axis, b))
                        .max(start(axis, b) - end(axis, a))
                        .max(0.0)
                };
                if axis_gap(Axis::Horizontal) < CONNECTED_NODE_GAP {
                    delta.y = 0.0;
                }
                if axis_gap(Axis::Vertical) < CONNECTED_NODE_GAP {
                    delta.x = 0.0;
                }
            }
        }
        delta
    }

    fn try_align(
        &mut self,
        edge: [usize; 2],
        moving: &[usize],
        delta: Point,
        work: &mut Work,
    ) -> Result<Option<(f64, Point)>, Error> {
        let before = self.nodes.clone();
        let mut best: Option<(f64, Point)> = None;
        for attempt in [Point { x: 0.0, y: delta.y }, Point { x: delta.x, y: 0.0 }] {
            if attempt.x == 0.0 && attempt.y == 0.0 {
                continue;
            }
            self.translate(moving, attempt);
            let result = self.validate_move(&before, moving, work).and_then(|valid| {
                if !valid || self.edge_is_blocked(edge) {
                    return Ok(None);
                }
                self.score(work).map(Some)
            });
            self.nodes.clone_from(&before);
            if let Some(score) = result?
                && best.is_none_or(|(previous, _)| !precision_compare(score, previous).is_gt())
            {
                best = Some((score, attempt));
            }
        }
        Ok(best)
    }

    fn edge_is_blocked(&self, [first, second]: [usize; 2]) -> bool {
        let from = self.center(first);
        let to = self.center(second);
        self.nodes.iter().enumerate().any(|(index, node)| {
            index != first && index != second && Bounds::from(node).blocks(from, to)
        })
    }

    pub(super) fn balance_symmetry(&mut self, work: &mut Work) -> Result<(), Error> {
        for &node in self.order {
            let mut adjacent = Vec::new();
            let links = &self.adjacency[node];
            if links.len() < 2
                || links.windows(2).any(|pair| {
                    sides(self.orientation(node, pair[0])) & sides(self.orientation(node, pair[1]))
                        == 0
                })
            {
                continue;
            }
            for &index in links {
                if !adjacent.contains(&index) {
                    adjacent.push(index);
                }
            }
            if adjacent.len() < 2 {
                continue;
            }
            let max_area = adjacent
                .iter()
                .map(|&index| self.nodes[index].width * self.nodes[index].height)
                .fold(0.0, f64::max);
            if adjacent
                .iter()
                .any(|&index| self.nodes[index].width * self.nodes[index].height < 0.5 * max_area)
            {
                continue;
            }
            if self.axis_score(&adjacent) != 1.0 && !self.share_center_axis(&adjacent) {
                continue;
            }

            let orientation = self.orientation(adjacent[0], adjacent[1]);
            let vertical = matches!(orientation, Some(0 | 4));
            let axis = if vertical {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            let min = adjacent
                .iter()
                .map(|&index| start(axis, &self.nodes[index]))
                .fold(f64::INFINITY, f64::min);
            let max = adjacent
                .iter()
                .map(|&index| end(axis, &self.nodes[index]))
                .fold(f64::NEG_INFINITY, f64::max);
            let center = min + (max - min) / 2.0;
            let own = self.center(node);
            let delta = if vertical {
                Point {
                    x: 0.0,
                    y: (center - own.y).floor(),
                }
            } else {
                Point {
                    x: (center - own.x).floor(),
                    y: 0.0,
                }
            };
            let before = self.nodes.clone();
            self.translate(&[node], delta);
            if !self.validate_move(&before, &[node], work)? {
                self.nodes = before;
            }
        }
        Ok(())
    }

    fn share_center_axis(&self, adjacent: &[usize]) -> bool {
        let first = self.center(adjacent[0]);
        let same_x = adjacent
            .iter()
            .all(|&index| self.center(index).x == first.x);
        let same_y = adjacent
            .iter()
            .all(|&index| self.center(index).y == first.y);
        same_x != same_y
    }

    pub(super) fn axis_score(&self, adjacent: &[usize]) -> f64 {
        if let [a, b] = adjacent {
            return f64::from(
                !self
                    .orientation(*a, *b)
                    .is_some_and(|angle| angle.abs() % 2 == 1),
            );
        }
        for along in [Axis::Horizontal, Axis::Vertical] {
            let across = along.opposite();
            let size = |body: &NodeGeometry| match across {
                Axis::Horizontal => body.width,
                Axis::Vertical => body.height,
            };
            let largest = adjacent
                .iter()
                .copied()
                .reduce(|old, index| {
                    if size(&self.nodes[index]) > size(&self.nodes[old]) {
                        index
                    } else {
                        old
                    }
                })
                .expect("has adjacent boxes");
            let neighbors = || adjacent.iter().copied().filter(|&index| index != largest);
            let is_aligned = |index| match along {
                Axis::Horizontal => matches!(self.orientation(index, largest), Some(-2 | 2)),
                Axis::Vertical => matches!(self.orientation(index, largest), Some(0 | 4)),
            };
            if !neighbors().all(is_aligned) {
                continue;
            }
            let anchor = &self.nodes[largest];
            let score = neighbors()
                .map(|index| {
                    let body = &self.nodes[index];
                    let mut node_score = 0.0;
                    for fraction in [0.25, 0.5, 0.75] {
                        let position = start(across, anchor) + fraction * size(anchor);
                        if start(across, body) <= position && position <= end(across, body) {
                            node_score += 0.33;
                        }
                    }
                    if size(body) < 0.25 * size(anchor) {
                        node_score *= 3.0;
                    } else if size(body) < 0.75 * size(anchor) {
                        node_score *= 2.0;
                    }
                    if node_score == 0.99 { 1.0 } else { node_score }
                })
                .sum::<f64>();
            let count = u32::try_from(adjacent.len() - 1).expect("validated node count");
            return (score / f64::from(count)).max(0.33);
        }
        0.0
    }
}

fn sides(orientation: Option<i32>) -> u8 {
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const TOP: u8 = 4;
    const BOTTOM: u8 = 8;
    match orientation {
        Some(0) => TOP,
        Some(1) => TOP | RIGHT,
        Some(2) => RIGHT,
        Some(3) => BOTTOM | RIGHT,
        Some(4) => BOTTOM,
        Some(-3) => BOTTOM | LEFT,
        Some(-2) => LEFT,
        Some(-1) => TOP | LEFT,
        _ => 0,
    }
}
