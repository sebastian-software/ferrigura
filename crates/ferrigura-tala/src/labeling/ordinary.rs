// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/labeling/{placement,edge_placement,guard}.go.

use std::collections::HashMap;

use super::{LabelCandidate, LabelSelection, MAX_WORK, NodeObstacle, PADDING, Scenario};
use super::{overlaps, select_with_budget, shared_obstacles, spend};
use crate::layout_graph::Label;
use crate::quality::Box;
use crate::route_output::RoutedLabel;
use crate::{Error, Point};

const MAX_REPAIR_WORK: u32 = MAX_WORK / 100;

pub(crate) struct OrdinaryLabels {
    /// Caption boxes in the positioned node coordinate frame, in node order.
    pub node_boxes: Vec<Option<Box>>,
    /// Resolved anchor and metrics in input edge order.
    pub edge_labels: Vec<Option<RoutedLabel>>,
    /// Edge-label boxes use layout-time stroke width and original position.
    pub edge_boxes: Vec<Option<Box>>,
}

/// Places movable labels on ordinary flat flowcharts with centered node
/// captions. Node inputs follow authored order. Edges and paths share caller
/// order and remain in layout-time orientation.
pub(crate) fn place_ordinary(
    nodes: &[Box],
    node_labels: &[Option<&Label>],
    paths: &[Vec<Point>],
    edge_labels: &[Option<&Label>],
) -> Result<OrdinaryLabels, Error> {
    if nodes.len() != node_labels.len()
        || paths.len() != edge_labels.len()
        || nodes.iter().any(|body| !body.valid())
        || node_labels
            .iter()
            .chain(edge_labels)
            .flatten()
            .any(|label| {
                !label.width.is_finite()
                    || !label.height.is_finite()
                    || label.width < 0.0
                    || label.height < 0.0
            })
        || paths
            .iter()
            .any(|path| path.len() < 2 || path.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()))
    {
        return Err(Error::InvalidGraph);
    }
    let node_boxes: Vec<_> = nodes
        .iter()
        .zip(node_labels)
        .map(|(body, label)| {
            label.map(|label| Box {
                x: body.x + body.width / 2.0 - label.width / 2.0,
                y: body.y + body.height / 2.0 - label.height / 2.0,
                width: label.width,
                height: label.height,
            })
        })
        .collect();
    if node_boxes.iter().flatten().any(|body| !body.valid()) {
        return Err(Error::InvalidGraph);
    }
    let mut result = OrdinaryLabels {
        node_boxes,
        edge_labels: vec![None; paths.len()],
        edge_boxes: vec![None; paths.len()],
    };
    let mut remaining = MAX_WORK;
    let order = placement_order(paths, edge_labels, &mut remaining)?;
    if order.is_empty() {
        return Ok(result);
    }
    let mut placement = Placement {
        paths,
        edge_labels,
        order,
        others: nodes
            .iter()
            .map(|&body| NodeObstacle {
                body,
                is_container: false,
            })
            .collect(),
        shared: shared_obstacles(paths)?,
        obstacles: result.node_boxes.iter().flatten().copied().collect(),
        candidates: Vec::new(),
    };
    let (fixed, collisions) = placement.greedy(&mut remaining)?;
    let mut repair = remaining.min(MAX_REPAIR_WORK);
    placement.repair(fixed, collisions, &mut repair)?;
    for (edge, candidate) in placement.order.into_iter().zip(placement.candidates) {
        result.edge_labels[edge] = Some(RoutedLabel {
            anchor: candidate.anchor,
            percentage: candidate.percentage,
            width: candidate.body.width,
            height: candidate.body.height,
        });
        result.edge_boxes[edge] = Some(candidate.body);
    }
    Ok(result)
}

fn placement_order(
    paths: &[Vec<Point>],
    labels: &[Option<&Label>],
    remaining: &mut u32,
) -> Result<Vec<usize>, Error> {
    let mut order = Vec::new();
    for (edge, path) in paths.iter().enumerate() {
        if labels[edge].is_some() {
            spend(remaining, path.len())?;
            let length: f64 = path
                .windows(2)
                .map(|pair| {
                    let dx = pair[1].x - pair[0].x;
                    let dy = pair[1].y - pair[0].y;
                    if dx == 0.0 {
                        dy.abs()
                    } else if dy == 0.0 {
                        dx.abs()
                    } else {
                        (dx * dx + dy * dy).sqrt()
                    }
                })
                .sum();
            if !length.is_finite() || length == 0.0 {
                return Err(Error::InvalidGraph);
            }
            order.push((edge, path.len() == 2, length));
        }
    }
    order.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| left.2.total_cmp(&right.2))
    });
    Ok(order.into_iter().map(|item| item.0).collect())
}

struct Placement<'a> {
    paths: &'a [Vec<Point>],
    edge_labels: &'a [Option<&'a Label>],
    order: Vec<usize>,
    others: Vec<NodeObstacle>,
    shared: Vec<Box>,
    obstacles: Vec<Box>,
    candidates: Vec<LabelCandidate>,
}

impl Placement<'_> {
    fn search(
        &self,
        index: usize,
        labels: &[Box],
        repair_conflicts: Option<u32>,
        remaining: &mut u32,
    ) -> Result<LabelSelection, Error> {
        let route = self.order[index];
        let measured = self.edge_labels[route].ok_or(Error::InvalidGraph)?;
        select_with_budget(
            Scenario {
                route,
                routes: self.paths,
                hint: None,
                percentage: 0.0,
                width: measured.width,
                height: measured.height,
                ancestors: &[],
                others: &self.others,
                labels,
                shared: &self.shared,
                repair_conflicts,
            },
            remaining,
        )
    }

    fn greedy(&mut self, remaining: &mut u32) -> Result<(usize, Vec<usize>), Error> {
        let fixed = self.obstacles.len();
        let mut collisions = Vec::new();
        for index in 0..self.order.len() {
            spend(remaining, 1)?;
            let candidate = self.search(index, &self.obstacles, None, remaining)?.best;
            self.candidates.push(candidate);
            self.obstacles.push(candidate.body);
            if candidate.conflicts != 0 {
                collisions.push(index);
            }
        }
        Ok((fixed, collisions))
    }

    fn repair(
        &mut self,
        fixed: usize,
        collisions: Vec<usize>,
        remaining: &mut u32,
    ) -> Result<(), Error> {
        for index in collisions {
            match self.resolve(index, fixed, remaining) {
                Err(Error::WorkLimit) => break,
                result => result?,
            }
        }
        Ok(())
    }

    fn alternatives(
        &self,
        index: usize,
        partner: Option<usize>,
        conflicts: u32,
        fixed: usize,
        remaining: &mut u32,
    ) -> Result<LabelSelection, Error> {
        spend(remaining, self.obstacles.len())?;
        let mut labels = Vec::new();
        let mut positions = Vec::new();
        for (position, &body) in self.obstacles.iter().enumerate() {
            if position != fixed + index && Some(position) != partner.map(|i| fixed + i) {
                labels.push(body);
                positions.push(position);
            }
        }
        let mut selection = self.search(index, &labels, Some(conflicts), remaining)?;
        for candidate in &mut selection.choices {
            candidate.blocker = candidate.blocker.map(|blocked| positions[blocked]);
        }
        Ok(selection)
    }

    fn resolve(&mut self, index: usize, fixed: usize, remaining: &mut u32) -> Result<(), Error> {
        let selection = self.alternatives(index, None, 1, fixed, remaining)?;
        if selection.best.conflicts == 0 {
            self.update(index, fixed, selection.best);
            return Ok(());
        }
        if selection.choices.is_empty() {
            return Ok(());
        }
        let mut blocked = HashMap::<usize, Vec<LabelCandidate>>::new();
        for choice in selection.choices {
            if let Some(blocker) = choice.blocker {
                blocked.entry(blocker).or_default().push(choice);
            }
        }
        for partner in (0..self.order.len()).rev() {
            spend(remaining, 1)?;
            let Some(options) = blocked.get(&(fixed + partner)) else {
                continue;
            };
            let other = self.alternatives(partner, Some(index), 0, fixed, remaining)?;
            if let Some((next, previous)) = best_clear_pair(options, &other.choices, remaining)? {
                self.update(index, fixed, next);
                self.update(partner, fixed, previous);
                break;
            }
        }
        Ok(())
    }

    fn update(&mut self, index: usize, fixed: usize, candidate: LabelCandidate) {
        self.candidates[index] = candidate;
        self.obstacles[fixed + index] = candidate.body;
    }
}

fn best_clear_pair(
    next: &[LabelCandidate],
    previous: &[LabelCandidate],
    remaining: &mut u32,
) -> Result<Option<(LabelCandidate, LabelCandidate)>, Error> {
    let mut best = None;
    let mut score = f64::INFINITY;
    for &old in previous {
        for &new in next {
            spend(remaining, 1)?;
            if overlaps(old.body, new.body, 0.0) {
                continue;
            }
            let combined = old.score + new.score + f64::from(overlaps(old.body, new.body, PADDING));
            if combined < score {
                score = combined;
                best = Some((new, old));
            }
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests;
