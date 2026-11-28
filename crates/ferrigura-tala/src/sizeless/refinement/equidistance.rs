// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/equidistance.go.

use super::super::precision_compare;
use super::{Axis, Refinement, Work, end, start};
use crate::{Error, NodeGeometry, Point};

impl Refinement<'_> {
    pub(super) fn equidistance(&mut self, work: &mut Work) -> Result<bool, Error> {
        let mut moved_horizontally = false;
        let mut moved_vertically = false;
        for &node in self.order {
            moved_horizontally |= self.center_between(node, Axis::Horizontal, work)?;
            moved_vertically |= self.center_between(node, Axis::Vertical, work)?;
        }
        for (axis, moved) in [
            (Axis::Horizontal, moved_horizontally),
            (Axis::Vertical, moved_vertically),
        ] {
            if !moved {
                continue;
            }
            for _ in 0..5 {
                let mut adjusted = false;
                for &node in self.order {
                    adjusted |= self.center_between(node, axis, work)?;
                }
                if !adjusted {
                    break;
                }
            }
        }
        Ok(moved_horizontally || moved_vertically)
    }

    fn center_between(&mut self, node: usize, axis: Axis, work: &mut Work) -> Result<bool, Error> {
        let Some([back, front]) = self.nearest_along_axis(node, axis) else {
            return Ok(false);
        };
        let current = &self.nodes[node];
        let size = match axis {
            Axis::Horizontal => current.width,
            Axis::Vertical => current.height,
        };
        let coordinate = ((end(axis, &self.nodes[back]) + start(axis, &self.nodes[front])) / 2.0
            - size / 2.0)
            .round();
        let distance = coordinate - start(axis, current);
        if distance == 0.0 {
            return Ok(false);
        }
        let delta = match axis {
            Axis::Horizontal => Point {
                x: distance,
                y: 0.0,
            },
            Axis::Vertical => Point {
                x: 0.0,
                y: distance,
            },
        };
        let blockers = [node, back, front];
        let mut connected = Vec::new();
        for &adjacent in &self.adjacency[node] {
            if blockers.contains(&adjacent) {
                continue;
            }
            let perpendicular = match axis {
                Axis::Horizontal => matches!(self.orientation(adjacent, node), Some(0 | 4)),
                Axis::Vertical => matches!(self.orientation(adjacent, node), Some(-2 | 2)),
            };
            if perpendicular {
                for moved in self.component(adjacent, &blockers, work)? {
                    if !connected.contains(&moved) {
                        connected.push(moved);
                    }
                }
            }
        }
        let before = self.nodes.clone();
        let baseline = self.score(work)?;
        let solo = self.try_center(&[node], delta, &before, baseline, work)?;
        let group = if connected.is_empty() {
            None
        } else {
            connected.push(node);
            self.try_center(&connected, delta, &before, baseline, work)?
        };
        let moving = match (solo, group) {
            (None, None) => return Ok(false),
            (Some(_), None) => &[node][..],
            (Some(solo), Some(group)) if precision_compare(solo, group).is_lt() => &[node][..],
            _ => &connected,
        };
        self.translate(moving, delta);
        Ok(true)
    }

    fn nearest_along_axis(&self, node: usize, axis: Axis) -> Option<[usize; 2]> {
        let current = &self.nodes[node];
        let mut back = None::<usize>;
        let mut front = None::<usize>;
        for &index in &self.adjacency[node] {
            let other = &self.nodes[index];
            if end(axis, other) < start(axis, current)
                && back.is_none_or(|old| end(axis, other) > end(axis, &self.nodes[old]))
            {
                back = Some(index);
            }
            if start(axis, other) > end(axis, current)
                && front.is_none_or(|old| start(axis, other) < start(axis, &self.nodes[old]))
            {
                front = Some(index);
            }
        }
        Some([back?, front?])
    }

    fn try_center(
        &mut self,
        moving: &[usize],
        delta: Point,
        before: &[NodeGeometry],
        baseline: f64,
        work: &mut Work,
    ) -> Result<Option<f64>, Error> {
        self.translate(moving, delta);
        let result = self.validate_move(before, moving, work).and_then(|valid| {
            if !valid {
                return Ok(None);
            }
            let score = self.score(work)?;
            let accepted = score > 0.0 && !precision_compare(score, baseline).is_gt();
            Ok(accepted.then_some(score))
        });
        self.nodes.clone_from_slice(before);
        result
    }
}
