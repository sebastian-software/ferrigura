// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{compaction,moves}.go and
// internal/layoutgraph/node.go.

use super::{Cell, MAX_OPTIMIZATION_WORK, Neighbor, Work, node_score, preferred_cell};
use crate::Error;

const MIN_SPACING: i32 = 3;
pub(super) const MAX_ROUNDS: usize = 20;
pub(super) const MAX_CANDIDATES: i32 = 30_003;

#[derive(Clone, Copy)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub(super) fn opposite(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    fn value(self, position: Cell) -> i32 {
        match self {
            Self::Horizontal => position.0,
            Self::Vertical => position.1,
        }
    }

    fn across(self, position: Cell) -> i32 {
        self.opposite().value(position)
    }

    fn move_to(self, position: Cell, coordinate: i32) -> Cell {
        match self {
            Self::Horizontal => (coordinate, position.1),
            Self::Vertical => (position.0, coordinate),
        }
    }

    fn advance(self, position: Cell, delta: i32) -> Cell {
        self.move_to(position, self.value(position) + delta)
    }
}

/// Spreads nodes on visible rows or columns before shifting visible groups
/// and individual nodes toward their neighbors.
pub(super) fn compact(
    adjacency: &[Vec<Neighbor>],
    positions: &mut [Cell],
    direction: Option<i32>,
    axis: Axis,
) -> Result<(), Error> {
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let predecessors = visibility_predecessors(positions, axis);
    for node in ordered_nodes(positions, axis) {
        if let Some(anchor) = predecessors[node] {
            let floor = axis.value(positions[anchor]) + MIN_SPACING;
            if axis.value(positions[node]) < floor {
                positions[node] = axis.move_to(positions[node], floor);
            }
        }
    }
    let pass = Compaction {
        adjacency,
        direction,
        predecessors,
        axis,
    };
    for _ in 0..MAX_ROUNDS {
        if !pass.shift_groups(positions, &mut work)? {
            break;
        }
    }
    for _ in 0..MAX_ROUNDS {
        if !pass.shift_nodes(positions, &mut work)? {
            break;
        }
    }
    Ok(())
}

fn ordered_nodes(positions: &[Cell], axis: Axis) -> Vec<usize> {
    let mut order: Vec<_> = (0..positions.len()).collect();
    order.sort_unstable_by_key(|&node| (axis.value(positions[node]), node));
    order
}

fn visibility_predecessors(positions: &[Cell], axis: Axis) -> Vec<Option<usize>> {
    let mut order: Vec<_> = (0..positions.len()).collect();
    order.sort_unstable_by_key(|&node| (axis.across(positions[node]), axis.value(positions[node])));
    let mut predecessors = vec![None; positions.len()];
    for pair in order.windows(2) {
        let (first, second) = (pair[0], pair[1]);
        if axis.across(positions[first]) == axis.across(positions[second]) {
            predecessors[second] = Some(first);
        }
    }
    predecessors
}

struct Compaction<'a> {
    adjacency: &'a [Vec<Neighbor>],
    direction: Option<i32>,
    predecessors: Vec<Option<usize>>,
    axis: Axis,
}

impl Compaction<'_> {
    fn earliest(&self, positions: &[Cell]) -> usize {
        positions
            .iter()
            .enumerate()
            .min_by_key(|&(_, &position)| self.axis.value(position))
            .expect("a placement component always has a node")
            .0
    }

    fn candidate_positions(
        &self,
        positions: &[Cell],
        node: usize,
        floor_decrease: i32,
    ) -> Result<impl Iterator<Item = Cell> + use<>, Error> {
        let behind = self.predecessors[node].unwrap_or_else(|| self.earliest(positions));
        let anchor = positions[behind];
        let current = positions[node];
        let floor = if self.axis.value(anchor) == self.axis.value(current)
            || self.axis.across(anchor) != self.axis.across(current)
        {
            self.axis.value(anchor)
        } else {
            self.axis.value(anchor) + MIN_SPACING
        } - floor_decrease;
        let ceiling = self.axis.value(current);
        if ceiling - floor + 1 > MAX_CANDIDATES {
            return Err(Error::WorkLimit);
        }

        let axis = self.axis;
        Ok((floor..=ceiling)
            .map(move |coordinate| axis.move_to(current, coordinate))
            .chain(std::iter::once(current)))
    }

    fn shift_groups(&self, positions: &mut [Cell], work: &mut Work) -> Result<bool, Error> {
        let mut groups = vec![Vec::new(); positions.len()];
        let mut roots = Vec::new();
        for node in ordered_nodes(positions, self.axis) {
            if let Some(anchor) = self.predecessors[node] {
                groups[anchor].push(node);
            } else {
                roots.push(node);
                groups[node].push(node);
            }
        }
        let earliest = self.earliest(positions);
        let mut changed = false;
        for root in roots {
            let group = &groups[root];
            let decrease =
                if self.axis.value(positions[root]) == self.axis.value(positions[earliest]) {
                    2
                } else {
                    0
                };
            let before = self.axis.value(positions[root]);
            let mut best_delta = 0;
            let mut lowest = self.group_score(group, positions, work)?;
            for candidate in self.candidate_positions(positions, root, decrease)? {
                work.add(group.len() * positions.len())?;
                let delta = self.axis.value(candidate) - before;
                if group.iter().any(|&node| {
                    let next = self.axis.advance(positions[node], delta);
                    positions
                        .iter()
                        .enumerate()
                        .any(|(other, &position)| !group.contains(&other) && position == next)
                }) {
                    continue;
                }
                self.move_group(group, positions, delta);
                let score = self.group_score(group, positions, work)?;
                self.move_group(group, positions, -delta);
                if score < lowest {
                    lowest = score;
                    best_delta = delta;
                }
            }
            if best_delta != 0 {
                self.move_group(group, positions, best_delta);
                changed = true;
            }
        }
        Ok(changed)
    }

    fn move_group(&self, group: &[usize], positions: &mut [Cell], delta: i32) {
        for &node in group {
            positions[node] = self.axis.advance(positions[node], delta);
        }
    }

    fn group_score(
        &self,
        group: &[usize],
        positions: &[Cell],
        work: &mut Work,
    ) -> Result<f64, Error> {
        let mut result = 0.0;
        for &node in group {
            result += node_score(self.adjacency, positions, self.direction, node, work)?;
        }
        Ok(result)
    }

    fn shift_nodes(&self, positions: &mut [Cell], work: &mut Work) -> Result<bool, Error> {
        let mut changed = false;
        for node in ordered_nodes(positions, self.axis) {
            let previous = positions[node];
            let mut candidates = Vec::new();
            for candidate in self.candidate_positions(positions, node, 0)? {
                work.add(positions.len())?;
                if candidate == previous || !positions.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
            positions[node] = preferred_cell(
                self.adjacency,
                positions,
                self.direction,
                node,
                &candidates,
                work,
            )?;
            changed |= previous != positions[node];
        }
        Ok(changed)
    }
}
