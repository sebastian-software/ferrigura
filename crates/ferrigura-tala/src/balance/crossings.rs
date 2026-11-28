// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{balance_crossings,cost,geometry}.go.

use std::collections::HashMap;

use crate::{EdgeGeometry, Error, Point};

use super::{Axis, Rail};

/// Permits reversing the order only when the whole move removes a crossing
/// without increasing crossings for any route pair or adding a shared run.
pub(super) fn reversal_removes_crossings(
    edges: &[EdgeGeometry],
    axis: Axis,
    moves: impl Iterator<Item = (Rail, f64)>,
    work: &mut usize,
) -> Result<bool, Error> {
    let mut point_copies = HashMap::new();
    for (rail, position) in moves {
        spend(work, 1)?;
        for index in [rail.segment, rail.segment + 1] {
            spend(work, 1)?;
            let mut point = edges[rail.edge].points[index];
            axis.update(&mut point, position);
            // Both adjacent legs use the same waypoint within a route.
            if point_copies
                .insert((rail.edge, index), point)
                .is_some_and(|previous| previous != point)
            {
                return Ok(false);
            }
        }
    }

    spend(work, edges.len())?;
    let mut candidates = Vec::with_capacity(edges.len());
    for (index, edge) in edges.iter().enumerate() {
        spend(work, 1)?;
        let mut changed = false;
        for (offset, point) in edge.points.iter().enumerate() {
            spend(work, 1)?;
            changed |= point_copies
                .get(&(index, offset))
                .is_some_and(|candidate| candidate != point);
        }
        if !changed {
            candidates.push(None);
            continue;
        }

        spend(work, edge.points.len())?;
        let mut points = Vec::with_capacity(edge.points.len());
        for (offset, &point) in edge.points.iter().enumerate() {
            spend(work, 1)?;
            let candidate = point_copies.get(&(index, offset)).copied().unwrap_or(point);
            if offset > 0
                && (!axis_aligned(edge.points[offset - 1], point)
                    || !axis_aligned(points[offset - 1], candidate))
            {
                return Ok(false);
            }
            points.push(candidate);
        }
        candidates.push(Some(points));
    }

    compare_routes(edges, &candidates, work)
}

fn compare_routes(
    edges: &[EdgeGeometry],
    candidates: &[Option<Vec<Point>>],
    work: &mut usize,
) -> Result<bool, Error> {
    let mut improved = false;
    for (index, selected) in candidates.iter().enumerate() {
        let Some(selected) = selected else { continue };
        spend(work, 1)?;
        for (other_index, other) in edges.iter().enumerate() {
            spend(work, 1)?;
            let other_candidate = &candidates[other_index];
            if index == other_index || other_candidate.is_some() && other_index < index {
                continue;
            }
            let before = &edges[index].points;
            let after = selected;
            let other_before = &other.points;
            let other_after = other_candidate.as_ref().unwrap_or(other_before);
            let mut old_crossings = 0;
            let mut new_crossings = 0;
            for (old, new) in before.windows(2).zip(after.windows(2)) {
                spend(work, 1)?;
                for (old_other, new_other) in other_before.windows(2).zip(other_after.windows(2)) {
                    spend(work, 1)?;
                    old_crossings += usize::from(non_shared_crossing(old, old_other));
                    new_crossings += usize::from(non_shared_crossing(new, new_other));
                    if collinear_overlap(old, old_other) == 0.0
                        && collinear_overlap(new, new_other) > 0.0
                    {
                        return Ok(false);
                    }
                }
            }
            if new_crossings > old_crossings {
                return Ok(false);
            }
            improved |= new_crossings < old_crossings;
        }
    }
    Ok(improved)
}

fn spend(work: &mut usize, units: usize) -> Result<(), Error> {
    *work = work.checked_sub(units).ok_or(Error::WorkLimit)?;
    Ok(())
}

fn axis_aligned(a: Point, b: Point) -> bool {
    a.x == b.x || a.y == b.y
}

fn non_shared_crossing(first: &[Point], second: &[Point]) -> bool {
    let (a, b, c, d) = (first[0], first[1], second[0], second[1]);
    !equal_signs(orientation(a, b, c), orientation(a, b, d))
        && !equal_signs(orientation(c, d, a), orientation(c, d, b))
        && !(a.x == b.x && (a.x == c.x || a.x == d.x))
        && !(a.y == b.y && (a.y == c.y || a.y == d.y))
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.y - a.y) * (c.x - a.x) - (b.x - a.x) * (c.y - a.y)
}

fn equal_signs(first: f64, second: f64) -> bool {
    first > 0.0 && second > 0.0 || first == 0.0 && second == 0.0 || first < 0.0 && second < 0.0
}

fn collinear_overlap(first: &[Point], second: &[Point]) -> f64 {
    let (a, b, c, d) = (first[0], first[1], second[0], second[1]);
    if a.x == b.x && c.x == d.x && a.x == c.x {
        overlap_length(a.y, b.y, c.y, d.y)
    } else if a.y == b.y && c.y == d.y && a.y == c.y {
        overlap_length(a.x, b.x, c.x, d.x)
    } else {
        0.0
    }
}

fn overlap_length(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (a.max(b).min(c.max(d)) - a.min(b).max(c.min(d))).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(points: &[(f64, f64)]) -> EdgeGeometry {
        EdgeGeometry {
            id: String::new(),
            points: points.iter().map(|&(x, y)| Point { x, y }).collect(),
            label: None,
        }
    }

    #[test]
    fn balance_reversal_does_not_trade_crossings_or_add_shared_runs() {
        // Same routes and proposals as the two pinned Go test cases.
        for (edges, segment, target) in [
            (
                [
                    route(&[(10.0, 0.0), (10.0, 20.0)]),
                    route(&[(0.0, 5.0), (20.0, 5.0)]),
                    route(&[(20.0, 15.0), (40.0, 15.0)]),
                ],
                0,
                30.0,
            ),
            (
                [
                    route(&[(-20.0, 0.0), (10.0, 0.0), (10.0, -10.0)]),
                    route(&[(-30.0, -5.0), (30.0, -5.0)]),
                    route(&[(20.0, 30.0), (20.0, 0.0), (40.0, 0.0), (40.0, -30.0)]),
                ],
                1,
                50.0,
            ),
        ] {
            let request = [(Rail { edge: 0, segment }, target)];
            // The proposal would uncross the first pair on its own.
            assert_eq!(
                reversal_removes_crossings(&edges[..2], Axis::X, request.into_iter(), &mut 10_000),
                Ok(true)
            );
            assert_eq!(
                reversal_removes_crossings(&edges, Axis::X, request.into_iter(), &mut 10_000),
                Ok(false)
            );
        }
    }
}
