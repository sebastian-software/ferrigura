// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg_edge_router,turns,tuning}.go and lib/geo/math.go.

use std::collections::HashSet;

use crate::{Error, NodeGeometry, Point};

use super::priority_queue::{EntryId, Popped, PriorityQueue};
use super::slingshot::{Incident, VisibilityAdjacency, step};

pub const BASICALLY_INFINITY: f64 = 10_000_000.0;
const IDEAL_TURN_AXIS_TOLERANCE: f64 = 4.0;
const IDEAL_TURN_MULTIPLIER: f64 = 0.98;
const IDEAL_TURN_EVEN_CLUSTER_MULTIPLIER: f64 = 0.10;
const PRECISION: f64 = 0.0001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TurnAxis {
    X(f64),
    Y(f64),
}

#[must_use]
pub fn ideal_turn_axes(source: &NodeGeometry, target: &NodeGeometry) -> Vec<TurnAxis> {
    let mut result = Vec::with_capacity(2);
    let right = source.x + source.width;
    let target_right = target.x + target.width;
    if target_right < source.x {
        result.push(TurnAxis::X((source.x + target_right) / 2.0));
    } else if right < target.x {
        result.push(TurnAxis::X((target.x + right) / 2.0));
    }
    let bottom = source.y + source.height;
    let target_bottom = target.y + target.height;
    if target_bottom < source.y {
        result.push(TurnAxis::Y((source.y + target_bottom) / 2.0));
    } else if bottom < target.y {
        result.push(TurnAxis::Y((target.y + bottom) / 2.0));
    }
    result
}

/// Turn costs shared by forward search and route reconstruction.
pub struct TurnPolicy<'a> {
    pub axes: &'a [TurnAxis],
    pub cost: f64,
    pub even_arranged_source_cluster: bool,
}

impl TurnPolicy<'_> {
    /// # Errors
    /// Returns a work limit when the axis checks exhaust the search budget.
    pub fn multiplier(&self, point: Point, work: &mut usize) -> Result<f64, Error> {
        for axis in self.axes {
            step(work)?;
            let distance = match axis {
                TurnAxis::X(value) => (point.x - value).abs(),
                TurnAxis::Y(value) => (point.y - value).abs(),
            };
            if distance <= IDEAL_TURN_AXIS_TOLERANCE {
                return Ok(if self.even_arranged_source_cluster {
                    IDEAL_TURN_EVEN_CLUSTER_MULTIPLIER
                } else {
                    IDEAL_TURN_MULTIPLIER
                });
            }
        }
        Ok(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub from: usize,
    pub to: usize,
    pub link: usize,
    pub previous: Option<usize>,
    pub from_horizontal: bool,
}

#[derive(Debug, PartialEq)]
pub struct SearchRoute {
    pub vertices: Vec<usize>,
    pub cost: f64,
}

#[derive(Clone, Copy)]
struct Arrival {
    distance: f64,
    entry: Option<EntryId>,
    previous: Option<usize>,
}

impl Default for Arrival {
    fn default() -> Self {
        Self {
            distance: f64::INFINITY,
            entry: None,
            previous: None,
        }
    }
}

/// Reusable two-arrival state for one sequential routing flavor.
#[derive(Default)]
pub struct SearchScratch {
    arrivals: Vec<Option<[Arrival; 2]>>,
    touched: Vec<usize>,
    queue: PriorityQueue,
}

impl SearchScratch {
    /// Visits the two states of every reachable visibility vertex in the
    /// reference queue order. The transition callback supplies the reference
    /// cost or rejects a move under the current route and endpoint policy.
    ///
    /// # Errors
    /// Returns a work limit when the search budget is exhausted and an
    /// invalid graph error for invalid vertices or inconsistent state.
    pub fn find(
        &mut self,
        graph: &VisibilityAdjacency<'_>,
        endpoints: [usize; 2],
        turns: &TurnPolicy<'_>,
        work: &mut usize,
        mut transition_cost: impl FnMut(Transition, &mut usize) -> Result<Option<f64>, Error>,
    ) -> Result<Option<SearchRoute>, Error> {
        let [source, target] = endpoints;
        let count = graph.vertex_count();
        if source >= count || target >= count {
            return Err(Error::InvalidGraph);
        }
        self.prepare(count, work)?;
        self.context(source);
        self.queue.push(source, false, 0.0, work)?;
        self.queue.push(source, true, 0.0, work)?;
        while !self.queue.is_empty() {
            step(work)?;
            let popped = self.queue.pop(work)?;
            let Popped {
                node,
                horizontal,
                priority,
                ..
            } = popped;
            if node == target {
                let vertices = self.reconstruct(graph, endpoints, turns, work)?;
                return Ok(Some(SearchRoute {
                    vertices,
                    cost: priority,
                }));
            }
            let previous = self.context(node)[usize::from(horizontal)].previous;
            let current = graph.vertex(node).point;
            for &Incident { neighbor, link } in graph.incidents(node) {
                step(work)?;
                if Some(neighbor) == previous {
                    continue;
                }
                let next = graph.vertex(neighbor).point;
                let direction = usize::from(next.y == current.y);
                if self.arrivals[neighbor]
                    .is_some_and(|arrivals| priority >= arrivals[direction].distance)
                {
                    continue;
                }
                let transition = Transition {
                    from: node,
                    to: neighbor,
                    link,
                    previous,
                    from_horizontal: horizontal,
                };
                if let Some(cost) = transition_cost(transition, work)? {
                    self.relax(popped, neighbor, direction, priority + cost, work)?;
                }
            }
        }
        Ok(None)
    }

    fn prepare(&mut self, count: usize, work: &mut usize) -> Result<(), Error> {
        if self.arrivals.len() != count {
            reserve(work, count, 4)?;
            self.arrivals.clear();
            self.arrivals.resize(count, None);
        } else {
            reserve(work, self.touched.len(), 3)?;
            for &node in &self.touched {
                self.arrivals[node] = None;
            }
        }
        self.touched.clear();
        self.queue.reset();
        Ok(())
    }

    fn context(&mut self, index: usize) -> &mut [Arrival; 2] {
        if self.arrivals[index].is_none() {
            self.touched.push(index);
        }
        self.arrivals[index].get_or_insert_with(Default::default)
    }

    fn relax(
        &mut self,
        popped: Popped,
        neighbor: usize,
        axis: usize,
        distance: f64,
        work: &mut usize,
    ) -> Result<(), Error> {
        let old = self.context(neighbor)[axis];
        if distance.partial_cmp(&old.distance) != Some(std::cmp::Ordering::Less) {
            return Ok(());
        }
        let horizontal = axis == 1;
        if let Some(previous) = old.previous {
            let previous_distance = self.context(previous)[axis].distance;
            self.queue
                .push(previous, horizontal, previous_distance, work)?;
        }
        let entry = if let Some(entry) = old.entry {
            self.queue.decrease(entry, distance, work)?;
            entry
        } else {
            self.queue.push(neighbor, horizontal, distance, work)?
        };
        self.context(neighbor)[axis] = Arrival {
            distance,
            entry: Some(entry),
            previous: Some(popped.node),
        };
        Ok(())
    }

    fn reconstruct(
        &self,
        graph: &VisibilityAdjacency<'_>,
        endpoints: [usize; 2],
        turns: &TurnPolicy<'_>,
        work: &mut usize,
    ) -> Result<Vec<usize>, Error> {
        let [source, target] = endpoints;
        let mut sequence = Vec::new();
        let mut visited = HashSet::new();
        let mut current = Some(target);
        let mut previous = None;
        while let Some(node) = current {
            step(work)?;
            if !visited.insert(node) {
                break;
            }
            sequence.push(node);
            let arrivals = self.arrivals[node].ok_or(Error::InvalidGraph)?;
            let [vertical, horizontal] = arrivals;
            let use_vertical = vertical.previous.is_some()
                && (horizontal.previous.is_none() || vertical.distance < horizontal.distance);
            let (chosen, alternate) = if use_vertical {
                (vertical, horizontal)
            } else {
                (horizontal, vertical)
            };
            let mut next = chosen.previous;
            if let Some(before) = previous.filter(|_| next.is_some_and(|next| next != source)) {
                let point = graph.vertex(node).point;
                let before = graph.vertex(before).point;
                let would_turn = if use_vertical {
                    before.y == point.y
                } else {
                    before.x == point.x
                };
                if would_turn {
                    let with_turn = chosen.distance + turns.cost * turns.multiplier(point, work)?;
                    if precision_greater(with_turn, alternate.distance)
                        && alternate.distance < BASICALLY_INFINITY
                        && alternate.previous.is_some()
                    {
                        next = alternate.previous;
                    }
                }
            }
            previous = current;
            current = next;
        }
        let last = sequence.len();
        for index in 0..last / 2 {
            step(work)?;
            sequence.swap(index, last - index - 1);
        }
        Ok(sequence)
    }
}

fn reserve(work: &mut usize, terms: usize, units: usize) -> Result<(), Error> {
    let amount = terms.checked_mul(units).ok_or(Error::WorkLimit)?;
    *work = work.checked_sub(amount).ok_or(Error::WorkLimit)?;
    Ok(())
}

fn precision_greater(left: f64, right: f64) -> bool {
    (left - right).abs().partial_cmp(&PRECISION) != Some(std::cmp::Ordering::Less)
        && left.partial_cmp(&right) != Some(std::cmp::Ordering::Less)
}

#[cfg(test)]
mod tests;
