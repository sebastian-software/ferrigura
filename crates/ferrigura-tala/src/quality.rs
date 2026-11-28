// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/quality/{scoring,evaluation_guard}.go,
// internal/layoutgraph/{graph,node,edge}.go, and internal/limits/work.go.

use serde::{Deserialize, Serialize};

use crate::Point;
use crate::seed_selection::LayoutScore;

mod labels;
mod routes;

const EVALUATION_LIMIT: u64 = 50_000_000;
const MAX_NODES: usize = 10_000;
const MAX_EDGES: usize = 50_000;
const MAX_ROUTE_POINTS: usize = 1_000_000;
const MAX_DEPTH: usize = 256;
const LABEL_PADDING: f64 = 5.0;

/// An immutable projection of a completed candidate. Shape-specific label and
/// icon positions are resolved by the corresponding layout stage.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionedGraph {
    pub nodes: Vec<PositionedNode>,
    pub edges: Vec<PositionedEdge>,
    /// Parent links for cluster or sequence vessels outside the scored node
    /// list. Their indices start immediately after the ordinary node indices.
    #[serde(default)]
    pub detached_ancestry: Vec<Option<usize>>,
}

impl PositionedGraph {
    fn ancestry_parent(&self, index: usize) -> Option<usize> {
        if let Some(node) = self.nodes.get(index) {
            node.ancestry_parent
        } else {
            self.detached_ancestry[index - self.nodes.len()]
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Box {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Box {
    const fn right(self) -> f64 {
        self.x + self.width
    }

    const fn bottom(self) -> f64 {
        self.y + self.height
    }

    fn valid(self) -> bool {
        [
            self.x,
            self.y,
            self.width,
            self.height,
            self.right(),
            self.bottom(),
        ]
        .into_iter()
        .all(f64::is_finite)
            && self.width >= 0.0
            && self.height >= 0.0
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionedNode {
    pub body: Box,
    pub container: Option<usize>,
    pub effective_container: Option<usize>,
    pub ancestry_parent: Option<usize>,
    #[serde(default)]
    pub is_container: bool,
    #[serde(default)]
    pub clustered: bool,
    pub label: Option<Box>,
    #[serde(default)]
    pub outside_label: bool,
    pub icon: Option<Box>,
    pub outside_icon_reserve: Option<Box>,
    pub fixed_top_left: Option<Point>,
    /// Painted right and upper extensions from the 3D or multiple modifier.
    pub modifier_extension: Option<Point>,
    /// Additional space occupied by loops, in left, top, right, bottom order.
    #[serde(default)]
    pub loop_offsets: [f64; 4],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionedEdge {
    pub source: usize,
    pub target: usize,
    pub points: Vec<Point>,
    pub label: Option<Box>,
    #[serde(default)]
    pub label_is_positioned: bool,
    pub source_arrowhead_label: Option<Box>,
    pub target_arrowhead_label: Option<Box>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidGeometry,
    ResourceLimit,
    WorkLimit,
    Cancelled,
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidGeometry => "TALA Evaluate received invalid positioned geometry",
            Self::ResourceLimit => "TALA Evaluate input exceeds the engine resource limit",
            Self::WorkLimit => "TALA Evaluate work exceeds the evaluation limit",
            Self::Cancelled => "TALA Evaluate was cancelled",
        })
    }
}

impl std::error::Error for EvaluationError {}

#[derive(Debug, PartialEq, Serialize)]
pub struct Evaluation {
    pub score: LayoutScore,
    pub work: u64,
}

/// Scores a completed candidate without modifying its geometry.
///
/// # Errors
/// Returns invalid geometry, an upstream resource or work limit violation, or
/// cancellation reported by the caller.
pub fn evaluate(
    graph: &PositionedGraph,
    cancelled: Option<&dyn Fn() -> bool>,
) -> Result<Evaluation, EvaluationError> {
    let mut work = Work::new(cancelled, EVALUATION_LIMIT)?;
    evaluate_with_work(graph, &mut work)
}

fn evaluate_with_work(
    graph: &PositionedGraph,
    work: &mut Work<'_>,
) -> Result<Evaluation, EvaluationError> {
    validate(graph)?;
    let mut penalty = 0.0;
    for edge in &graph.edges {
        work.step()?;
        if graph.nodes[edge.source].clustered || graph.nodes[edge.target].clustered {
            continue;
        }
        penalty += f64::from(route_count(edge.points.len().saturating_sub(2))) * 0.5;
        for segment in edge.points.windows(2) {
            work.step()?;
            if segment[0].x != segment[1].x && segment[0].y != segment[1].y {
                penalty += 3.0;
            }
        }
    }
    let crossings = routes::count_crossings(&graph.edges, work)?;
    penalty += f64::from(
        u32::try_from(crossings).expect("crossings are bounded by the evaluation budget"),
    );
    penalty += 1.0 - labels::score(graph, work)?;
    charge_area(graph, work)?;
    let area = area(graph);
    work.check()?;
    Ok(Evaluation {
        score: LayoutScore { penalty, area },
        work: work.used,
    })
}

// The positioned graph preflight bounds these individual counts. The scoring
// budget also keeps a crossing or overlap count below this conversion's range.
fn route_count(count: usize) -> u32 {
    u32::try_from(count).expect("route counts are bounded by the evaluation budget")
}

fn validate(graph: &PositionedGraph) -> Result<(), EvaluationError> {
    let node_count = graph.nodes.len();
    let ancestry_count = node_count.saturating_add(graph.detached_ancestry.len());
    if ancestry_count > MAX_NODES || graph.edges.len() > MAX_EDGES {
        return Err(EvaluationError::ResourceLimit);
    }
    let mut route_points = 0_usize;
    for edge in &graph.edges {
        route_points = route_points.saturating_add(edge.points.len());
        if route_points > MAX_ROUTE_POINTS {
            return Err(EvaluationError::ResourceLimit);
        }
        if edge.source >= node_count
            || edge.target >= node_count
            || !edge.points.iter().copied().all(valid_point)
            || ![
                edge.label,
                edge.source_arrowhead_label,
                edge.target_arrowhead_label,
            ]
            .into_iter()
            .flatten()
            .all(Box::valid)
        {
            return Err(EvaluationError::InvalidGeometry);
        }
    }
    for node in &graph.nodes {
        if !node.body.valid()
            || ![node.label, node.icon, node.outside_icon_reserve]
                .into_iter()
                .flatten()
                .all(Box::valid)
            || ![node.modifier_extension, node.fixed_top_left]
                .into_iter()
                .flatten()
                .all(valid_point)
            || !node.loop_offsets.into_iter().all(f64::is_finite)
            || node
                .ancestry_parent
                .is_some_and(|index| index >= ancestry_count)
            || [node.container, node.effective_container]
                .into_iter()
                .flatten()
                .any(|index| index >= node_count)
        {
            return Err(EvaluationError::InvalidGeometry);
        }
    }
    // Completed candidates cannot contain cyclic parent relations, including
    // a grouped node's effective and ancestry parents.
    for parent in [
        direct_parent as fn(&PositionedNode) -> Option<usize>,
        effective_parent,
    ] {
        for node in &graph.nodes {
            let mut remaining = MAX_DEPTH;
            let mut current = parent(node);
            while let Some(index) = current {
                if remaining == 0 {
                    return Err(EvaluationError::InvalidGeometry);
                }
                remaining -= 1;
                current = parent(&graph.nodes[index]);
            }
        }
    }
    for &parent in &graph.detached_ancestry {
        if parent.is_some_and(|index| index >= ancestry_count) {
            return Err(EvaluationError::InvalidGeometry);
        }
    }
    for index in 0..ancestry_count {
        let mut remaining = MAX_DEPTH;
        let mut parent = graph.ancestry_parent(index);
        while let Some(index) = parent {
            if remaining == 0 {
                return Err(EvaluationError::InvalidGeometry);
            }
            remaining -= 1;
            parent = graph.ancestry_parent(index);
        }
    }
    Ok(())
}

fn direct_parent(node: &PositionedNode) -> Option<usize> {
    node.container
}

fn effective_parent(node: &PositionedNode) -> Option<usize> {
    node.effective_container
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn charge_area(graph: &PositionedGraph, work: &mut Work<'_>) -> Result<(), EvaluationError> {
    work.add(graph.nodes.len())?;
    for node in &graph.nodes {
        if node.label.is_some() && node.outside_label {
            for _ in 0..4 {
                work.add(graph.nodes.len())?;
            }
        }
    }
    for edge in &graph.edges {
        let multiplier =
            1 + usize::from(
                edge.label.is_some() && edge.label_is_positioned && !edge.points.is_empty(),
            ) + usize::from(edge.source_arrowhead_label.is_some())
                + usize::from(edge.target_arrowhead_label.is_some());
        for _ in 0..multiplier {
            work.add(edge.points.len())?;
        }
    }
    Ok(())
}

fn area(graph: &PositionedGraph) -> f64 {
    if graph.nodes.is_empty() {
        return 0.0;
    }
    let body_bounds = graph.nodes.iter().fold(Extent::EMPTY, |mut bounds, node| {
        bounds.include_box(node.body);
        bounds
    });
    let mut bounds = Extent::EMPTY;
    for node in &graph.nodes {
        let [left, top, right, bottom] = node.loop_offsets;
        let mut local = Extent {
            left: node.body.x,
            top: node.body.y,
            right: node.body.right(),
            bottom: node.body.bottom(),
        };
        if let Some(extension) = node.modifier_extension {
            local.top -= extension.y;
            local.right += extension.x;
        }
        local.left -= left;
        local.top -= top;
        local.right += right;
        local.bottom += bottom;

        if node.outside_label
            && let Some(label) = node.label
        {
            let padding = |is_outer| {
                if is_outer {
                    LABEL_PADDING
                } else {
                    2.0 * LABEL_PADDING
                }
            };
            if label.x < local.left {
                local.left = (label.x - padding(node.body.x <= body_bounds.left)).floor();
            }
            if label.y < local.top {
                local.top = (label.y - padding(node.body.y <= body_bounds.top)).floor();
            }
            if label.right() > local.right {
                local.right =
                    (label.right() + padding(node.body.right() >= body_bounds.right)).ceil();
            }
            if label.bottom() > local.bottom {
                local.bottom =
                    (label.bottom() + padding(node.body.bottom() >= body_bounds.bottom)).ceil();
            }
        }
        if let Some(icon) = node.outside_icon_reserve {
            local.include(Extent {
                left: (icon.x - 2.0 * LABEL_PADDING).floor(),
                top: (icon.y - 2.0 * LABEL_PADDING).floor(),
                right: (icon.right() + 2.0 * LABEL_PADDING).ceil(),
                bottom: (icon.bottom() + 2.0 * LABEL_PADDING).ceil(),
            });
        }
        bounds.include(local);
    }

    let effective_root = graph
        .nodes
        .iter()
        .map(|node| node.effective_container)
        .min_by_key(|&parent| {
            let mut depth = 0;
            let mut next = parent;
            while let Some(index) = next {
                depth += 1;
                next = graph.nodes[index].effective_container;
            }
            depth
        })
        .flatten();
    for node in &graph.nodes {
        if node.effective_container == effective_root
            && let Some(fixed) = node.fixed_top_left
        {
            bounds.left = node.body.x - fixed.x;
            bounds.top = node.body.y - fixed.y;
            break;
        }
    }

    for edge in &graph.edges {
        if edge.points.is_empty() {
            continue;
        }
        let mut local = Extent::EMPTY;
        for point in &edge.points {
            local.include(Extent {
                left: point.x,
                top: point.y,
                right: point.x,
                bottom: point.y,
            });
        }
        for decoration in [
            edge.label.filter(|_| edge.label_is_positioned),
            edge.source_arrowhead_label,
            edge.target_arrowhead_label,
        ]
        .into_iter()
        .flatten()
        {
            local.include_box(decoration);
        }
        bounds.include(Extent {
            left: local.left.round(),
            top: local.top.round(),
            right: local.right.round(),
            bottom: local.bottom.round(),
        });
    }
    (bounds.left - bounds.right).abs() * (bounds.top - bounds.bottom).abs()
}

#[derive(Clone, Copy)]
struct Extent {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Extent {
    const EMPTY: Self = Self {
        left: f64::INFINITY,
        top: f64::INFINITY,
        right: f64::NEG_INFINITY,
        bottom: f64::NEG_INFINITY,
    };

    fn include(&mut self, other: Self) {
        self.left = self.left.min(other.left);
        self.top = self.top.min(other.top);
        self.right = self.right.max(other.right);
        self.bottom = self.bottom.max(other.bottom);
    }

    fn include_box(&mut self, bounds: Box) {
        self.include(Self {
            left: bounds.x,
            top: bounds.y,
            right: bounds.right(),
            bottom: bounds.bottom(),
        });
    }
}

struct Work<'a> {
    cancelled: Option<&'a dyn Fn() -> bool>,
    used: u64,
    limit: u64,
}

impl<'a> Work<'a> {
    fn new(cancelled: Option<&'a dyn Fn() -> bool>, limit: u64) -> Result<Self, EvaluationError> {
        let work = Self {
            cancelled,
            used: 0,
            limit,
        };
        work.check()?;
        Ok(work)
    }

    const fn stride(&self) -> u64 {
        if self.cancelled.is_some() { 1024 } else { 64 }
    }

    fn step(&mut self) -> Result<(), EvaluationError> {
        self.used += 1;
        if self.used > self.limit {
            return Err(EvaluationError::WorkLimit);
        }
        if self.used.is_multiple_of(self.stride()) {
            self.check()?;
        }
        Ok(())
    }

    fn add(&mut self, units: usize) -> Result<(), EvaluationError> {
        let units = u64::try_from(units).map_err(|_| EvaluationError::WorkLimit)?;
        let previous = self.used;
        let stride = self.stride();
        if units > self.limit || self.used > self.limit - units {
            self.used = self.limit + 1;
            if previous <= self.limit && previous / stride != self.limit / stride {
                self.check()?;
            }
            return Err(EvaluationError::WorkLimit);
        }
        self.used += units;
        if self.used.is_multiple_of(stride) || previous / stride != self.used / stride {
            self.check()?;
        }
        Ok(())
    }

    fn check(&self) -> Result<(), EvaluationError> {
        if self.cancelled.is_some_and(|cancelled| cancelled()) {
            Err(EvaluationError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
