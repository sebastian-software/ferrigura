// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/grouping/join.go and
// internal/layoutgraph/{node,graph}.go.

use super::super::precision_compare;
use super::super::transition::overlaps_at;
use super::{MAX_OPTIMIZATION_WORK, SizedOptimizer, Work};
use crate::{Error, NodeGeometry, Point};

const DISTANCE_CELLS: f64 = 3.0;
const MAX_JOIN_PASSES: usize = 1000;
const OVERLAP_PREFILTER_PADDING: f64 = 500.0;

impl SizedOptimizer {
    pub(super) fn join_distanced_clusters(&mut self) -> Result<(), Error> {
        if self.state.nodes.len() <= 1 {
            return Ok(());
        }
        let mut work = Work(MAX_OPTIMIZATION_WORK);
        let clusters = self.distance_clusters(&mut work)?;
        if clusters.is_empty() {
            return Ok(());
        }
        let target = median_center(&self.state.nodes, &clusters);
        for _ in 0..MAX_JOIN_PASSES {
            work.add(1 + clusters.len())?;
            let mut moved = false;
            for cluster in &clusters {
                moved |= self.inch_towards_target(cluster, target, &mut work)?;
            }
            if !moved {
                break;
            }
        }
        Ok(())
    }

    fn distance_clusters(&self, work: &mut Work) -> Result<Vec<Vec<usize>>, Error> {
        let nodes = &self.state.nodes;
        let threshold = DISTANCE_CELLS * self.cell;
        let mut seeds = Vec::new();
        for (node, edges) in self.adjacency.iter().enumerate() {
            work.add(1 + edges.len())?;
            for adjacent in edges {
                if precision_compare(distance(&nodes[node], &nodes[adjacent.node]), threshold)
                    .is_gt()
                {
                    seeds.extend([node, adjacent.node]);
                }
            }
        }
        if seeds.is_empty() {
            return Ok(Vec::new());
        }

        let mut cluster_ids = vec![None; nodes.len()];
        let mut next_id = 0;
        for seed in seeds {
            work.add(1)?;
            if cluster_ids[seed].is_none() {
                cluster_ids[seed] = Some(next_id);
                next_id += 1;
            }
            merge_perimeter(nodes, seed, threshold, &mut cluster_ids, work)?;
        }

        let mut clusters = vec![Vec::new(); next_id];
        for (node, id) in cluster_ids.into_iter().enumerate() {
            clusters[id.ok_or(Error::UnsupportedGraph)?].push(node);
        }
        clusters.retain(|cluster| !cluster.is_empty());
        for cluster in &mut clusters {
            cluster.sort_unstable_by_key(|&node| self.state.ranks[node]);
        }
        Ok(clusters)
    }

    fn inch_towards_target(
        &mut self,
        cluster: &[usize],
        target: Point,
        work: &mut Work,
    ) -> Result<bool, Error> {
        work.add(cluster.len())?;
        let (center, length) = cluster_center(&self.state.nodes, cluster);
        if euclidean(center.x - target.x, center.y - target.y) < length / 2.0 {
            return Ok(false);
        }
        let dx = step_towards(center.x, target.x, self.cell);
        let dy = step_towards(center.y, target.y, self.cell);
        if dx == 0.0 && dy == 0.0 {
            return Ok(false);
        }

        let mut blocked = false;
        for &node in cluster {
            let body = &self.state.nodes[node];
            let point = Point {
                x: body.x + dx,
                y: body.y + dy,
            };
            if !blocked {
                for (other, other_body) in self.state.nodes.iter().enumerate() {
                    work.add(1)?;
                    if !cluster.contains(&other)
                        && within_overlap_prefilter(body, point, other_body)
                        && overlaps_at(body, other_body, point, self.state.gaps[node][other])
                    {
                        blocked = true;
                        break;
                    }
                }
            }
            self.state.nodes[node].x = point.x;
            self.state.nodes[node].y = point.y;
        }
        if blocked {
            for &node in cluster {
                self.state.nodes[node].x += -dx;
                self.state.nodes[node].y += -dy;
            }
        }
        Ok(!blocked)
    }
}

fn merge_perimeter(
    nodes: &[NodeGeometry],
    node: usize,
    threshold: f64,
    cluster_ids: &mut [Option<usize>],
    work: &mut Work,
) -> Result<(), Error> {
    work.add(nodes.len())?;
    let previous_id = cluster_ids[node];
    for other in 0..nodes.len() {
        if other == node
            || precision_compare(distance(&nodes[node], &nodes[other]), threshold).is_gt()
        {
            continue;
        }
        let other_id = cluster_ids[other];
        if other_id.is_some() && other_id == previous_id {
            continue;
        }

        // Recursion can reassign the seed. Use its current membership for each
        // neighbor, while keeping the original ID for the comparison above.
        cluster_ids[other] = cluster_ids[node];
        if other_id.is_none() {
            merge_perimeter(nodes, other, threshold, cluster_ids, work)?;
        }
    }
    Ok(())
}

fn cluster_center(nodes: &[NodeGeometry], cluster: &[usize]) -> (Point, f64) {
    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for &node in cluster {
        let body = &nodes[node];
        left = left.min(body.x);
        top = top.min(body.y);
        right = right.max((body.x + body.width).round());
        bottom = bottom.max((body.y + body.height).round());
    }
    let width = right - left;
    let height = bottom - top;
    (
        Point {
            x: left + width / 2.0,
            y: top + height / 2.0,
        },
        width.max(height),
    )
}

fn median_center(nodes: &[NodeGeometry], clusters: &[Vec<usize>]) -> Point {
    let (mut x, mut y): (Vec<_>, Vec<_>) = clusters
        .iter()
        .map(|cluster| {
            let (center, _) = cluster_center(nodes, cluster);
            (center.x, center.y)
        })
        .unzip();
    Point {
        x: median(&mut x),
        y: median(&mut y),
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle] + values[middle - 1]) / 2.0
    } else {
        values[middle]
    }
}

fn distance(first: &NodeGeometry, second: &NodeGeometry) -> f64 {
    let dx = interval_gap(first.x, first.width, second.x, second.width);
    let dy = interval_gap(first.y, first.height, second.y, second.height);
    let center_x = ((first.x + first.width / 2.0) - (second.x + second.width / 2.0)).abs()
        / (first.width + second.width);
    let center_y = ((first.y + first.height / 2.0) - (second.y + second.height / 2.0)).abs()
        / (first.height + second.height);
    euclidean(dx, dy) + (1.0 / 20.0) * center_x.min(center_y)
}

fn interval_gap(first: f64, first_length: f64, second: f64, second_length: f64) -> f64 {
    (second - (first + first_length))
        .max(first - (second + second_length))
        .max(0.0)
}

fn euclidean(dx: f64, dy: f64) -> f64 {
    if dx == 0.0 {
        dy.abs()
    } else if dy == 0.0 {
        dx.abs()
    } else {
        (dx * dx + dy * dy).sqrt()
    }
}

fn step_towards(start: f64, end: f64, cell: f64) -> f64 {
    if start < end {
        cell
    } else if start > end {
        -cell
    } else {
        0.0
    }
}

fn within_overlap_prefilter(body: &NodeGeometry, point: Point, other: &NodeGeometry) -> bool {
    point.x <= other.x + other.width + OVERLAP_PREFILTER_PADDING
        && point.x + body.width + OVERLAP_PREFILTER_PADDING >= other.x
        && point.y <= other.y + other.height + OVERLAP_PREFILTER_PADDING
        && point.y + body.height + OVERLAP_PREFILTER_PADDING >= other.y
}
