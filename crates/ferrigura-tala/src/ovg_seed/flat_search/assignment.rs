// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/{coordinator,route}.go.

use std::cmp::Ordering;

use crate::{Error, Point};

use super::super::grid::{charge, charge_sort};
use super::super::slingshot::{Orientation, step};
use super::{FlatSearch, FlavorRoutes, OrdinaryEdge};

#[derive(Debug, PartialEq)]
pub struct MaterializedRoute {
    pub edge: usize,
    pub points: Vec<Point>,
}

struct Candidate {
    original_edge: usize,
    assigned_edge: usize,
    owners: [usize; 2],
    ports: [Point; 2],
    orientation: Orientation,
}

impl FlatSearch<'_> {
    /// Assigns parallel paths to authored edges in the winning flavor,
    /// then copies the ports and route turns in selected flavor order.
    ///
    /// # Errors
    /// Returns invalid graph for inconsistent or incomplete selection and a
    /// work limit when reassignment or materialization cannot finish.
    pub fn materialize_selected_routes(
        &self,
        flavor: &FlavorRoutes,
        edges: &[OrdinaryEdge],
        work: &mut usize,
    ) -> Result<Vec<MaterializedRoute>, Error> {
        if flavor.routes.len() != edges.len() {
            return Err(Error::InvalidGraph);
        }
        charge(work, edges.len())?;
        let mut candidates = Vec::with_capacity(flavor.routes.len());
        let mut found = vec![false; edges.len()];
        for route in &flavor.routes {
            step(work)?;
            let duplicate = found.get_mut(route.edge).ok_or(Error::InvalidGraph)?;
            if *duplicate {
                return Err(Error::InvalidGraph);
            }
            *duplicate = true;
            let owners = edges[route.edge].owners;
            let [source, target] = owners.map(|index| self.nodes.get(index));
            let orientation = Orientation::between(
                source.ok_or(Error::InvalidGraph)?,
                target.ok_or(Error::InvalidGraph)?,
            );
            let path = self.raw_route_points(flavor, &route.route)?;
            if path.len() < 4 {
                return Err(Error::InvalidGraph);
            }
            charge(work, path.len())?;
            candidates.push(Candidate {
                original_edge: route.edge,
                assigned_edge: route.edge,
                owners,
                ports: [path[1], path[path.len() - 2]],
                orientation,
            });
        }
        reassign_parallel(&mut candidates, work)?;
        charge(work, candidates.len())?;
        candidates
            .into_iter()
            .zip(&flavor.routes)
            .map(|(candidate, route)| {
                step(work)?;
                charge(work, route.route.vertices.len())?;
                let starts_on_original_side =
                    candidate.owners[0] == edges[candidate.assigned_edge].owners[0];
                let points = if starts_on_original_side {
                    self.materialize(&route.route, &flavor.detached)?
                } else {
                    charge(work, route.route.vertices.len())?;
                    let mut reversed = route.route.clone();
                    reversed.vertices.reverse();
                    self.materialize(&reversed, &flavor.detached)?
                };
                Ok(MaterializedRoute {
                    edge: candidate.assigned_edge,
                    points,
                })
            })
            .collect()
    }
}

fn reassign_parallel(candidates: &mut [Candidate], work: &mut usize) -> Result<(), Error> {
    let mut grouped = vec![false; candidates.len()];
    let mut buckets = Vec::new();
    for index in 0..candidates.len() {
        step(work)?;
        if grouped[index] {
            continue;
        }
        grouped[index] = true;
        let mut bucket = vec![index];
        for (other, seen) in grouped.iter_mut().enumerate().skip(index + 1) {
            step(work)?;
            if !*seen && can_exchange(candidates, index, other, work)? {
                bucket.push(other);
                *seen = true;
            }
        }
        if bucket.len() > 1 {
            buckets.push(bucket);
        }
    }
    charge(work, candidates.len())?;
    charge(work, candidates.len())?;
    for mut bucket in buckets {
        step(work)?;
        charge_sort(work, bucket.len())?;
        bucket.sort_by(|&one, &two| compare_ports(&candidates[one], &candidates[two]));
        charge(work, bucket.len())?;
        let mut assigned = Vec::with_capacity(bucket.len());
        for &index in &bucket {
            step(work)?;
            assigned.push(candidates[index].original_edge);
        }
        charge_sort(work, assigned.len())?;
        assigned.sort_unstable();
        for (route, edge) in bucket.into_iter().zip(assigned) {
            step(work)?;
            candidates[route].assigned_edge = edge;
        }
    }
    Ok(())
}

fn can_exchange(
    candidates: &[Candidate],
    first_index: usize,
    second_index: usize,
    work: &mut usize,
) -> Result<bool, Error> {
    let first = &candidates[first_index];
    let second = &candidates[second_index];
    let [source, target] = first.owners;
    let same_orientation = first.owners == second.owners;
    if source == target
        || second.owners[0] == second.owners[1]
        || (!same_orientation && [target, source] != second.owners)
        || first.orientation.diagonal().is_some()
    {
        return Ok(false);
    }
    if same_orientation {
        let same_side = match first.orientation {
            Orientation::Top | Orientation::Bottom => {
                first.ports[0].y == second.ports[0].y && first.ports[1].y == second.ports[1].y
            }
            Orientation::Left | Orientation::Right => {
                first.ports[0].x == second.ports[0].x && first.ports[1].x == second.ports[1].x
            }
            _ => true,
        };
        if !same_side {
            return Ok(false);
        }
    }
    for (index, other) in candidates.iter().enumerate() {
        step(work)?;
        if index == first_index || index == second_index {
            continue;
        }
        if first
            .ports
            .into_iter()
            .chain(second.ports)
            .any(|port| other.ports.contains(&port))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn compare_ports(first: &Candidate, second: &Candidate) -> Ordering {
    let side = usize::from(first.owners[0] != second.owners[0]);
    let a = first.ports[0];
    let b = second.ports[side];
    let (a, b) = if matches!(first.orientation, Orientation::Top | Orientation::Bottom) {
        (a.x, b.x)
    } else {
        (a.y, b.y)
    };
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests;
