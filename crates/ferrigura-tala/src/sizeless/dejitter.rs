// SPDX-License-Identifier: MPL-2.0
// Ported from pinned D2 TALA: internal/placement/{dejitter,tuning}.go.

use super::compaction::Axis;
use super::refinement::cell_size;
use super::sized_cost::{Bounds, EdgeCost};
use super::transition::{Spacing, edge_gaps, overlaps_at};
use super::{MAX_OPTIMIZATION_WORK, Work, order_ranks};
use crate::{Error, Graph, NodeGeometry, Point};

const JITTER_THRESHOLD: f64 = 80.0;
const SIGN_FLIP_PADDING: f64 = 5.0;

#[derive(Clone, Copy)]
struct Endpoint {
    edge: usize,
    reverse: bool,
}

impl Endpoint {
    fn index(self, length: usize, offset: usize) -> usize {
        if self.reverse {
            length - 1 - offset
        } else {
            offset
        }
    }

    fn point(self, path: &[Point], offset: usize) -> Point {
        path[self.index(path.len(), offset)]
    }

    fn nearest(self, path: &[Point]) -> [Point; 2] {
        [self.point(path, 0), self.point(path, 1)]
    }
}

/// Runs the node adjustment between the first and second routing passes.
/// Geometry is committed together if the complete stage succeeds.
pub(crate) fn dejitter(
    graph: &Graph,
    owners: &[[usize; 2]],
    node_order: &[usize],
    nodes: &mut [NodeGeometry],
    paths: &mut [Vec<Point>],
) -> Result<bool, Error> {
    order_ranks(nodes.len(), node_order)?;
    if nodes.len() != graph.nodes.len()
        || owners.len() != paths.len()
        || paths.len() != graph.edges.len()
        || paths.iter().any(|path| path.len() < 2)
    {
        return Err(Error::InvalidGraph);
    }
    let mut adjacent = vec![Vec::new(); nodes.len()];
    for (edge, &[first, last]) in owners.iter().enumerate() {
        if first >= nodes.len() || last >= nodes.len() || first == last {
            return Err(Error::InvalidGraph);
        }
        adjacent[first].push(Endpoint {
            edge,
            reverse: false,
        });
        adjacent[last].push(Endpoint {
            edge,
            reverse: true,
        });
    }
    let mut workspace = Workspace {
        nodes: nodes.to_vec(),
        paths: paths.to_vec(),
        owners,
        gaps: edge_gaps(graph)?,
        work: Work(MAX_OPTIMIZATION_WORK),
    };
    let scorer = EdgeCost::new(graph, nodes, cell_size(nodes), node_order)?;
    let mut changed = false;
    for &node in node_order {
        let previous = scorer.symmetry(node, &workspace.nodes, &mut workspace.work)?;
        let endpoints = &adjacent[node];
        for &endpoint in endpoints {
            let path = &workspace.paths[endpoint.edge];
            let Some((axis, amount)) = jitter(endpoint, path) else {
                continue;
            };
            if let Some(updates) = workspace.propose(node, endpoint, endpoints, axis, amount)?
                && workspace.move_node(node, axis, amount, previous, &scorer)?
            {
                workspace.apply(endpoint, updates);
                changed = true;
            }
        }
    }
    if changed {
        nodes.clone_from_slice(&workspace.nodes);
        paths.clone_from_slice(&workspace.paths);
    }
    Ok(changed)
}

struct Workspace<'a> {
    nodes: Vec<NodeGeometry>,
    paths: Vec<Vec<Point>>,
    owners: &'a [[usize; 2]],
    gaps: Vec<Vec<Spacing>>,
    work: Work,
}

struct Update {
    endpoint: Endpoint,
    port: Point,
    bend: Point,
}

impl Workspace<'_> {
    fn propose(
        &mut self,
        node: usize,
        current: Endpoint,
        adjacent: &[Endpoint],
        axis: Axis,
        delta: f64,
    ) -> Result<Option<Vec<Update>>, Error> {
        let padded_delta = if delta < 0.0 {
            delta - SIGN_FLIP_PADDING
        } else {
            delta + SIGN_FLIP_PADDING
        };
        let mut updates = Vec::with_capacity(adjacent.len());
        for &endpoint in adjacent {
            self.work.add(1)?;
            let path = &self.paths[endpoint.edge];
            let [port, bend] = endpoint.nearest(path);
            let tangent = coordinate(axis, port) == coordinate(axis, bend);
            let diagonal = path.len() == 2 && port.x != bend.x && port.y != bend.y;
            let shifted_port = shifted(axis, port, delta);
            if diagonal {
                updates.push(Update {
                    endpoint,
                    port: shifted_port,
                    bend,
                });
                continue;
            }
            if tangent && path.len() == 2 {
                return Ok(None);
            }
            let shifted_bend = if tangent {
                shifted(axis, bend, delta)
            } else {
                bend
            };
            let before = if tangent { bend } else { port };
            let pivot = if tangent {
                endpoint.point(path, 2)
            } else {
                bend
            };
            if (!tangent || endpoint.edge != current.edge)
                && flips(
                    coordinate(axis, before),
                    coordinate(axis, pivot),
                    padded_delta,
                )
            {
                return Ok(None);
            }
            let owners = self.owners[endpoint.edge];
            for (index, obstacle) in self.nodes.iter().enumerate() {
                self.work.add(1)?;
                if index != node
                    && !owners.contains(&index)
                    && Bounds::from(obstacle).blocks(shifted_port, shifted_bend)
                {
                    return Ok(None);
                }
            }
            updates.push(Update {
                endpoint,
                port: shifted_port,
                bend: shifted_bend,
            });
        }
        Ok(Some(updates))
    }

    fn move_node(
        &mut self,
        node: usize,
        axis: Axis,
        delta: f64,
        previous: f64,
        scorer: &EdgeCost,
    ) -> Result<bool, Error> {
        let original = self.nodes[node].clone();
        let mut trial = original.clone();
        match axis {
            Axis::Horizontal => trial.x += delta,
            Axis::Vertical => trial.y += delta,
        }
        for (index, other) in self.nodes.iter().enumerate() {
            self.work.add(1)?;
            if index == node {
                continue;
            }
            let spacing = self.gaps[node][index];
            let violates_spacing = |body: &NodeGeometry| {
                overlaps_at(
                    body,
                    other,
                    Point {
                        x: body.x,
                        y: body.y,
                    },
                    spacing,
                )
            };
            if violates_spacing(&trial) && !violates_spacing(&original) {
                return Ok(false);
            }
        }
        let old = Bounds::from(&original);
        let moved = Bounds::from(&trial);
        let mut prior_hits = 0;
        let mut trial_hits = 0;
        for (owners, path) in self.owners.iter().zip(&self.paths) {
            self.work.add(path.len())?;
            if owners.contains(&node) {
                continue;
            }
            for segment in path.windows(2) {
                prior_hits += usize::from(old.blocks(segment[0], segment[1]));
                trial_hits += usize::from(moved.blocks(segment[0], segment[1]));
            }
        }
        if trial_hits > prior_hits {
            return Ok(false);
        }
        self.nodes[node] = trial;
        if scorer.symmetry(node, &self.nodes, &mut self.work)? < previous {
            self.nodes[node] = original;
            return Ok(false);
        }
        Ok(true)
    }

    fn apply(&mut self, current: Endpoint, updates: Vec<Update>) {
        for Update {
            endpoint,
            port,
            bend,
        } in updates
        {
            let path = &mut self.paths[endpoint.edge];
            let first = endpoint.index(path.len(), 0);
            let second = endpoint.index(path.len(), 1);
            path[first] = port;
            path[second] = bend;
        }
        let path = &mut self.paths[current.edge];
        let first_bend = current.index(path.len(), if current.reverse { 2 } else { 1 });
        path.drain(first_bend..first_bend + 2);
    }
}

fn jitter(endpoint: Endpoint, path: &[Point]) -> Option<(Axis, f64)> {
    if path.len() < 4 {
        return None;
    }
    let [port, first, second, ahead] = [0, 1, 2, 3].map(|i| endpoint.point(path, i));
    let dx = second.x - first.x;
    let dy = second.y - first.y;
    if (dx * dx + dy * dy).sqrt() > JITTER_THRESHOLD {
        return None;
    }
    let axis = if first.y == second.y {
        Axis::Horizontal
    } else {
        Axis::Vertical
    };
    let across = axis.opposite();
    let (start, end, lane) = (
        coordinate(across, port),
        coordinate(across, ahead),
        coordinate(across, first),
    );
    if (start > lane && end > lane) || (start < lane && end < lane) {
        return None;
    }
    let delta = coordinate(axis, second) - coordinate(axis, first);
    (delta != 0.0).then(|| (axis, delta.round()))
}

fn coordinate(axis: Axis, point: Point) -> f64 {
    match axis {
        Axis::Horizontal => point.x,
        Axis::Vertical => point.y,
    }
}

fn shifted(axis: Axis, mut point: Point, delta: f64) -> Point {
    match axis {
        Axis::Horizontal => point.x += delta,
        Axis::Vertical => point.y += delta,
    }
    point
}

fn flips(before: f64, pivot: f64, delta: f64) -> bool {
    (before < pivot && before + delta > pivot) || (before > pivot && before + delta < pivot)
}

#[cfg(test)]
mod tests;
