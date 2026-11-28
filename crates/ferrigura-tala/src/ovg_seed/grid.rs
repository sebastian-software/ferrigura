// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{ovg_crossing_index,ovg_edge_set,ovg_edge_router}.go.

use std::collections::HashMap;

use crate::{Error, Point};

use super::slingshot::step;
use super::{LinkedVisibility, point_key};

const PATH_NODE_PROXIMITY_FLOOR: f64 = 5.0;

#[derive(Clone, Copy)]
struct Interval {
    link: usize,
    start: f64,
    end: f64,
    max_end: f64,
}

struct Line {
    coordinate: f64,
    intervals: Vec<Interval>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Direction {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }

    pub(super) fn other(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Segment {
    pub from: Point,
    pub to: Point,
}

impl Segment {
    #[must_use]
    pub fn direction(self) -> Option<Direction> {
        if self.from.x == self.to.x {
            Some(Direction::Vertical)
        } else if self.from.y == self.to.y {
            Some(Direction::Horizontal)
        } else {
            None
        }
    }

    #[must_use]
    pub fn shares_point(self, other: Self) -> bool {
        self.from == other.from
            || self.from == other.to
            || self.to == other.from
            || self.to == other.to
    }

    pub(super) fn ordered(self, direction: Direction) -> (f64, f64, f64) {
        let (coordinate, a, b) = match direction {
            Direction::Horizontal => (self.from.y, self.from.x, self.to.x),
            Direction::Vertical => (self.from.x, self.from.y, self.to.y),
        };
        (coordinate, a.min(b), a.max(b))
    }
}

/// Read-only grid intervals derived from all visibility edges, shared by
/// flavors. Crossing and proximity queries report existing visibility links.
pub struct Grid {
    horizontal: Vec<Line>,
    vertical: Vec<Line>,
    segments: Vec<Segment>,
}

impl Grid {
    /// Indexes visibility links in their line and interval order.
    ///
    /// # Errors
    /// Returns an invalid graph error for nonexistent link endpoints and a
    /// work limit when index construction exceeds the remaining work.
    pub fn new(graph: &LinkedVisibility, work: &mut usize) -> Result<Self, Error> {
        let segments = graph
            .links
            .iter()
            .map(|&[from, to]| {
                let from = graph.vertices.get(from).ok_or(Error::InvalidGraph)?.point;
                let to = graph.vertices.get(to).ok_or(Error::InvalidGraph)?.point;
                Ok(Segment { from, to })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let horizontal = index_lines(&segments, Direction::Horizontal, work)?;
        let vertical = index_lines(&segments, Direction::Vertical, work)?;
        Ok(Self {
            horizontal,
            vertical,
            segments,
        })
    }

    fn lines(&self, direction: Direction) -> &[Line] {
        match direction {
            Direction::Horizontal => &self.horizontal,
            Direction::Vertical => &self.vertical,
        }
    }

    /// Reports links a newly occupied segment crosses under Go's crossing
    /// rule, excluding shared endpoints and counting strict parallel overlap.
    ///
    /// # Errors
    /// Returns a work limit if probing the grid exceeds the remaining work.
    pub fn visit_crossed(
        &self,
        segment: Segment,
        work: &mut usize,
        mut mark: impl FnMut(usize),
    ) -> Result<(), Error> {
        let Some(direction) = segment.direction() else {
            return Ok(());
        };
        let (axis, start, end) = segment.ordered(direction);
        visit(
            self.lines(direction),
            [axis, axis],
            [start, end],
            work,
            |_, interval| {
                if start < interval.end
                    && interval.start < end
                    && !segment.shares_point(self.segments[interval.link])
                {
                    mark(interval.link);
                }
            },
        )?;
        visit(
            self.lines(direction.other()),
            [start, end],
            [axis, axis],
            work,
            |_, interval| {
                if !segment.shares_point(self.segments[interval.link]) {
                    mark(interval.link);
                }
            },
        )
    }

    /// Reports same-axis links within the reference proximity distance, with
    /// the overlap flag set on the exact same line.
    ///
    /// # Errors
    /// Returns a work limit if probing the grid exceeds the remaining work.
    pub fn visit_nearby(
        &self,
        segment: Segment,
        work: &mut usize,
        mut mark: impl FnMut(usize, bool),
    ) -> Result<(), Error> {
        let direction = if segment.from.x == segment.to.x {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };
        let (axis, start, end) = segment.ordered(direction);
        let first = (axis - PATH_NODE_PROXIMITY_FLOOR).next_down();
        let last = (axis + PATH_NODE_PROXIMITY_FLOOR).next_up();
        visit(
            self.lines(direction),
            [first, last],
            [start, end],
            work,
            |coordinate, interval| {
                if (coordinate - axis).abs() <= PATH_NODE_PROXIMITY_FLOOR {
                    mark(interval.link, coordinate == axis);
                }
            },
        )
    }
}

fn index_lines(
    segments: &[Segment],
    direction: Direction,
    work: &mut usize,
) -> Result<Vec<Line>, Error> {
    let mut lines: HashMap<u64, Line> = HashMap::new();
    for (link, &segment) in segments.iter().enumerate() {
        if segment.direction() != Some(direction) {
            continue;
        }
        let (coordinate, start, end) = segment.ordered(direction);
        let key = point_key(Point {
            x: coordinate,
            y: 0.0,
        })
        .0;
        lines
            .entry(key)
            .or_insert_with(|| Line {
                coordinate,
                intervals: Vec::new(),
            })
            .intervals
            .push(Interval {
                link,
                start,
                end,
                max_end: f64::NEG_INFINITY,
            });
    }
    charge_sort(work, lines.len())?;
    let mut lines: Vec<_> = lines.into_values().collect();
    lines.sort_unstable_by(|a, b| a.coordinate.partial_cmp(&b.coordinate).unwrap());
    for line in &mut lines {
        let intervals = &mut line.intervals;
        charge_sort(work, intervals.len())?;
        charge(work, intervals.len())?;
        intervals.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        let mut max_end = f64::NEG_INFINITY;
        for interval in intervals {
            step(work)?;
            max_end = max_end.max(interval.end);
            interval.max_end = max_end;
        }
    }
    Ok(lines)
}

fn visit(
    lines: &[Line],
    coordinates: [f64; 2],
    span: [f64; 2],
    work: &mut usize,
    mut visit_match: impl FnMut(f64, Interval),
) -> Result<(), Error> {
    let [first, last] = coordinates;
    let [start, end] = span;
    charge(work, bit_length(lines.len()))?;
    let begin = lines.partition_point(|line| line.coordinate < first);
    for line in &lines[begin..] {
        step(work)?;
        if line.coordinate > last {
            break;
        }
        charge(work, bit_length(line.intervals.len()))?;
        let begin = line
            .intervals
            .partition_point(|interval| interval.max_end < start);
        for &interval in &line.intervals[begin..] {
            step(work)?;
            if interval.start > end {
                break;
            }
            if interval.end >= start {
                visit_match(line.coordinate, interval);
            }
        }
    }
    Ok(())
}

fn bit_length(length: usize) -> usize {
    (usize::BITS - length.leading_zeros()) as usize
}

pub(super) fn charge_sort(work: &mut usize, count: usize) -> Result<(), Error> {
    let amount = if count < 2 {
        1
    } else {
        count
            .checked_mul(bit_length(count - 1))
            .ok_or(Error::WorkLimit)?
    };
    charge(work, amount)
}

pub(super) fn charge(work: &mut usize, amount: usize) -> Result<(), Error> {
    *work = work.checked_sub(amount).ok_or(Error::WorkLimit)?;
    Ok(())
}

#[cfg(test)]
mod tests;
