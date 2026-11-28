// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placementcost/symmetry.go.

use super::{Bounds, EdgeCost};
use crate::{Error, NodeGeometry};

use super::super::Work;

const MAX_NEIGHBOR_DISTANCE: f64 = 1200.0;

impl EdgeCost {
    pub(in super::super) fn symmetry(
        &self,
        node: usize,
        nodes: &[NodeGeometry],
        work: &mut Work,
    ) -> Result<f64, Error> {
        self.symmetry_inner(node, nodes, true, work)
    }

    fn symmetry_inner(
        &self,
        node: usize,
        nodes: &[NodeGeometry],
        check_neighbors: bool,
        work: &mut Work,
    ) -> Result<f64, Error> {
        let center = Bounds::from(&nodes[node]);
        let mut neighbors = Vec::new();
        let mut seen = vec![false; nodes.len()];
        work.add(self.adjacency[node].len())?;
        for edge in &self.adjacency[node] {
            let index = edge.neighbor;
            if !seen[index]
                && center.distance_to(Bounds::from(&nodes[index])) <= MAX_NEIGHBOR_DISTANCE
            {
                neighbors.push(index);
                seen[index] = true;
            }
        }
        if neighbors.is_empty() {
            return Ok(0.0);
        }
        let mut matched = vec![false; neighbors.len()];
        let mut score = 0.0;
        for (index, &first) in neighbors.iter().enumerate() {
            if matched[index] {
                continue;
            }
            let a = Bounds::from(&nodes[first]);
            let mut best = None;
            let mut best_score = 0.0;
            for (other_index, &second) in neighbors.iter().enumerate().skip(index + 1) {
                work.add(1)?;
                if matched[other_index] {
                    continue;
                }
                let b = Bounds::from(&nodes[second]);
                let area_a = a.width() * a.height();
                let area_b = b.width() * b.height();
                if area_a > 2.0 * area_b || area_b > 2.0 * area_a {
                    continue;
                }
                let mirrored = if mirrored_across_x(center, a, b, self.cell_size) {
                    Some(overlaps_y(center, a) && overlaps_y(center, b))
                } else if mirrored_across_x(
                    center.transposed(),
                    a.transposed(),
                    b.transposed(),
                    self.cell_size,
                ) {
                    Some(
                        overlaps_y(center.transposed(), a.transposed())
                            && overlaps_y(center.transposed(), b.transposed()),
                    )
                } else {
                    None
                };
                let Some(aligned) = mirrored else { continue };
                work.add(nodes.len())?;
                let unobstructed = nodes.iter().enumerate().all(|(index, body)| {
                    if [node, first, second].contains(&index) {
                        return true;
                    }
                    let body = Bounds::from(body);
                    !body.blocks(center.center(), a.center())
                        && !body.blocks(center.center(), b.center())
                });
                let match_score = if aligned { 2.0 } else { 0.5 };
                if unobstructed && match_score > best_score {
                    best = Some(other_index);
                    best_score = match_score;
                    if aligned {
                        break;
                    }
                }
            }
            if let Some(other_index) = best {
                matched[index] = true;
                matched[other_index] = true;
                score += best_score;
            }
        }
        if check_neighbors {
            for (&neighbor, matched) in neighbors.iter().zip(matched) {
                if !matched {
                    score += self.symmetry_inner(neighbor, nodes, false, work)?;
                }
            }
        }
        let count = u32::try_from(neighbors.len()).map_err(|_| Error::WorkLimit)?;
        Ok(score / f64::from(count))
    }

    pub(in super::super) fn global_symmetry_credit(&self, node: usize, symmetry: f64) -> f64 {
        let count = u32::try_from(self.adjacency[node].len()).expect("validated edge count");
        symmetry * self.cell_size * f64::from(count)
    }

    pub(in super::super) fn symmetry_cost(&self, node: usize) -> f64 {
        let count = u32::try_from(self.adjacency[node].len()).expect("validated edge count");
        self.cell_size * f64::from(count)
    }
}

fn overlaps_y(first: Bounds, second: Bounds) -> bool {
    first.top <= second.bottom && first.bottom >= second.top
}

fn mirrored_across_x(center: Bounds, first: Bounds, second: Bounds, tolerance: f64) -> bool {
    if first.left == second.left {
        return false;
    }
    let (left, right) = if first.left < second.left {
        (first, second)
    } else {
        (second, first)
    };
    let axis = center.center().x;
    left.right < axis
        && right.left > axis
        && ((right.left - axis) - (axis - left.right)).abs() <= tolerance
        && ((first.top + first.bottom) / 2.0 - (second.top + second.bottom) / 2.0).abs()
            <= tolerance
}
