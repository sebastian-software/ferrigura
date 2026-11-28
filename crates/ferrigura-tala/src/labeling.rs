// SPDX-License-Identifier: MPL-2.0
// Ported from D2 TALA at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// internal/labeling/{model,placement,guard,edge_placement}.go.

use serde::{Deserialize, Serialize};

use crate::quality::Box;
use crate::quality::labels::{covers, overlaps};
use crate::quality::routes::box_crosses_line;
use crate::route_output::{Anchor, RoutedLabel};
use crate::{Error, Point};

const PADDING: f64 = 5.0;
const STROKE_WIDTH: f64 = 3.0;
const SEARCH_INTERVAL: f64 = 0.025;
const MAX_WORK: u32 = 50_000_000;
const PREFERRED: [Anchor; 9] = [
    Anchor::OutsideTopCenter,
    Anchor::OutsideBottomCenter,
    Anchor::OutsideTopLeft,
    Anchor::OutsideTopRight,
    Anchor::OutsideBottomLeft,
    Anchor::OutsideBottomRight,
    Anchor::InsideMiddleCenter,
    Anchor::InsideMiddleLeft,
    Anchor::InsideMiddleRight,
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeObstacle {
    pub body: Box,
    pub is_container: bool,
}

/// A non-cluster edge and the geometry present when its label is placed.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelScenario {
    pub route: usize,
    pub routes: Vec<Vec<Point>>,
    pub hint: Option<Anchor>,
    pub percentage: f64,
    pub width: f64,
    pub height: f64,
    pub ancestors: Vec<NodeObstacle>,
    pub others: Vec<NodeObstacle>,
    pub labels: Vec<Box>,
    pub shared: Vec<Box>,
    pub repair_conflicts: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LabelCandidate {
    pub anchor: Anchor,
    pub percentage: f64,
    pub body: Box,
    pub score: f64,
    pub conflicts: u32,
    pub blocker: Option<usize>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct LabelSelection {
    pub best: LabelCandidate,
    pub choices: Vec<LabelCandidate>,
}

/// Chooses the edge label with the fewest exact shape and label intersections.
/// Equal candidates are ordered by clearance penalties and source preference.
/// When repairing a collision, it also returns the admissible alternatives.
///
/// # Errors
/// Rejects invalid geometry or a search that exceeds the label work limit.
pub fn select(scenario: &LabelScenario) -> Result<LabelSelection, Error> {
    let route = scenario
        .routes
        .get(scenario.route)
        .ok_or(Error::InvalidGraph)?;
    if route.len() < 2
        || !scenario.width.is_finite()
        || !scenario.height.is_finite()
        || scenario.width < 0.0
        || scenario.height < 0.0
        || !(0.0..=1.0).contains(&scenario.percentage)
        || scenario
            .routes
            .iter()
            .flatten()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
        || scenario
            .ancestors
            .iter()
            .chain(&scenario.others)
            .any(|node| !node.body.valid())
        || scenario
            .labels
            .iter()
            .chain(&scenario.shared)
            .any(|rect| !rect.valid())
    {
        return Err(Error::InvalidGraph);
    }

    let mut search = Search {
        scenario,
        remaining: MAX_WORK,
        best: None,
        choices: Vec::new(),
    };
    for anchor in positions(scenario.hint) {
        search.spend(1)?;
        if search.found_clear_position() {
            break;
        }
        if unlocked(anchor) && scenario.percentage == 0.0 {
            let mut percentage = 0.0;
            while percentage < 1.0 {
                search.spend(1)?;
                search.consider(anchor, percentage)?;
                if search.found_clear_position() {
                    break;
                }
                percentage += SEARCH_INTERVAL;
            }
        } else {
            search.consider(anchor, scenario.percentage)?;
        }
    }
    Ok(LabelSelection {
        best: search.best.ok_or(Error::InvalidGraph)?,
        choices: search.choices,
    })
}

/// Computes the obstacles where a caption could be ambiguous because routes share a leg.
/// # Errors
/// Returns an error if the shared-segment search exceeds the work limit.
pub fn shared_obstacles(paths: &[Vec<Point>]) -> Result<Vec<Box>, Error> {
    crate::quality::routes::shared_obstacles(paths)
}

fn positions(hint: Option<Anchor>) -> Vec<Anchor> {
    let mut positions = Vec::with_capacity(12);
    match hint {
        Some(Anchor::UnlockedMiddle) => {
            positions.extend([
                Anchor::UnlockedMiddle,
                Anchor::UnlockedTop,
                Anchor::UnlockedBottom,
            ]);
            positions.extend(PREFERRED);
        }
        Some(anchor) if unlocked(anchor) => {
            positions.extend([anchor, anchor.mirrored(), Anchor::UnlockedMiddle]);
            positions.extend(PREFERRED);
        }
        _ => {
            positions.extend(PREFERRED);
            positions.extend([
                Anchor::UnlockedTop,
                Anchor::UnlockedBottom,
                Anchor::UnlockedMiddle,
            ]);
        }
    }
    positions
}

fn unlocked(anchor: Anchor) -> bool {
    matches!(
        anchor,
        Anchor::UnlockedTop | Anchor::UnlockedMiddle | Anchor::UnlockedBottom
    )
}

fn on_route(anchor: Anchor) -> bool {
    matches!(
        anchor,
        Anchor::InsideMiddleLeft
            | Anchor::InsideMiddleCenter
            | Anchor::InsideMiddleRight
            | Anchor::UnlockedMiddle
    )
}

struct Search<'a> {
    scenario: &'a LabelScenario,
    remaining: u32,
    best: Option<LabelCandidate>,
    choices: Vec<LabelCandidate>,
}

impl Search<'_> {
    fn spend(&mut self, count: usize) -> Result<(), Error> {
        let count = u32::try_from(count).map_err(|_| Error::WorkLimit)?;
        self.remaining = self.remaining.checked_sub(count).ok_or(Error::WorkLimit)?;
        Ok(())
    }

    fn found_clear_position(&self) -> bool {
        self.scenario.repair_conflicts.is_none()
            && self
                .best
                .is_some_and(|candidate| candidate.conflicts == 0 && candidate.score == 0.0)
    }

    fn consider(&mut self, anchor: Anchor, percentage: f64) -> Result<(), Error> {
        let scenario = self.scenario;
        let route = &scenario.routes[scenario.route];
        self.spend(1 + 2 * route.len())?;
        let label = RoutedLabel {
            anchor,
            percentage,
            width: scenario.width,
            height: scenario.height,
        };
        let point = label
            .top_left(route, STROKE_WIDTH)
            .ok_or(Error::InvalidGraph)?;
        let body = Box {
            x: point.x,
            y: point.y,
            width: label.width,
            height: label.height,
        };
        let area = label.width * label.height;

        self.spend(scenario.ancestors.len() + scenario.others.len())?;
        let ancestors = shape_overlaps(body, &scenario.ancestors, true);
        let others = shape_overlaps(body, &scenario.others, false);
        let edge_overlaps = self.edge_overlaps(body, anchor)?;
        let (padded_labels, exact_labels, blocker) = self.label_overlaps(body)?;
        self.spend(scenario.shared.len())?;
        let shared = scenario
            .shared
            .iter()
            .filter(|&&rect| overlaps(body, rect, PADDING))
            .count();
        let shared = u32::try_from(shared).map_err(|_| Error::WorkLimit)?;

        let mut score = 0.0;
        if area > 0.0 {
            score = (ancestors.area + others.area) / area * 2.0;
        }
        score += f64::from(exact_labels) * 10.0;
        score += f64::from(padded_labels + exact_labels.div_ceil(2));
        score += f64::from(ancestors.count + others.count) * 2.0;
        score += f64::from(edge_overlaps) * 2.0;
        score += f64::from(shared);
        let exact_shapes = ancestors.exact + others.exact;
        let mut candidate = LabelCandidate {
            anchor,
            percentage,
            body,
            score,
            conflicts: exact_shapes + exact_labels,
            blocker: None,
        };
        if self.best.is_none_or(|best| {
            candidate.conflicts < best.conflicts
                || candidate.conflicts == best.conflicts && candidate.score < best.score
        }) {
            self.best = Some(candidate);
        }
        if exact_shapes == 0 && scenario.repair_conflicts == Some(exact_labels) {
            candidate.blocker = blocker;
            self.choices.push(candidate);
        }
        Ok(())
    }

    fn edge_overlaps(&mut self, body: Box, anchor: Anchor) -> Result<u32, Error> {
        let scenario = self.scenario;
        let mut totals = [0u32; 2];
        for (count, padding) in totals.iter_mut().zip([PADDING, 0.0]) {
            for (index, route) in scenario.routes.iter().enumerate() {
                if index == scenario.route && on_route(anchor) {
                    continue;
                }
                self.spend(1 + route.len().saturating_sub(1))?;
                for pair in route.windows(2) {
                    *count += u32::from(box_crosses_line(body, pair[0], pair[1], padding));
                }
            }
        }
        Ok(totals[0] + totals[1].div_ceil(2))
    }

    fn label_overlaps(&mut self, body: Box) -> Result<(u32, u32, Option<usize>), Error> {
        let labels = &self.scenario.labels;
        self.spend(labels.len())?;
        let (mut padded, mut exact, mut blocker) = (0, 0, None);
        for (index, &rect) in labels.iter().enumerate() {
            if overlaps(body, rect, PADDING) {
                padded += 1;
                if overlaps(body, rect, 0.0) {
                    exact += 1;
                    blocker = Some(index);
                }
            }
        }
        Ok((padded, exact, blocker))
    }
}

#[derive(Default)]
struct Overlap {
    area: f64,
    count: u32,
    exact: u32,
}

fn shape_overlaps(body: Box, nodes: &[NodeObstacle], partial: bool) -> Overlap {
    let mut total = Overlap::default();
    for node in nodes {
        let other = node.body;
        if !overlaps(body, other, PADDING) || partial && covers(other, body) {
            continue;
        }
        if overlaps(body, other, 0.0) {
            let width = body.right().min(other.right()) - body.x.max(other.x);
            let height = body.bottom().min(other.bottom()) - body.y.max(other.y);
            total.area += width * height;
            total.count += 1;
            total.exact += 1;
        }
        if !node.is_container {
            total.area += body.width * body.height;
            total.count += 1;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_pinned_go_edge_label_repair_search() {
        #[derive(Deserialize)]
        struct Fixture {
            input: LabelScenario,
            expected: LabelSelection,
        }
        let fixture: Fixture =
            serde_json::from_str(include_str!("labeling/library-repair.json")).unwrap();
        assert_eq!(select(&fixture.input).unwrap(), fixture.expected);
    }
}
