// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See d2layouts/d2talalayout/internal/routing/{postprocess,balance_route_guard,tuning}.go
// and lib/geo/segment.go.

use crate::{EdgeGeometry, Error, Graph, Layout, NodeGeometry, Point};

mod crossings;

const SEGMENT_SPACING_BUFFER: f64 = 40.0;
const UNBOUNDED_SEGMENT_MOVE: f64 = 100.0;
const MAX_STAGED_POINTS: usize = 128;
const MAX_REVERSAL_WORK: usize = 50_000_000;

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn coordinates(self, point: Point) -> (f64, f64) {
        match self {
            Self::X => (point.x, point.y),
            Self::Y => (point.y, point.x),
        }
    }

    fn update(self, point: &mut Point, coordinate: f64) {
        match self {
            Self::X => point.x = coordinate,
            Self::Y => point.y = coordinate,
        }
    }

    fn body_sides(self, node: &NodeGeometry) -> [Span; 2] {
        let (position, start) = self.coordinates(Point {
            x: node.x,
            y: node.y,
        });
        let (width, height) = self.coordinates(Point {
            x: node.width,
            y: node.height,
        });
        [
            Span {
                position,
                start,
                end: start + height,
            },
            Span {
                position: position + width,
                start,
                end: start + height,
            },
        ]
    }
}

#[derive(Clone, Copy)]
struct Rail {
    edge: usize,
    segment: usize,
}

impl Rail {
    fn span(self, edges: &[EdgeGeometry], axis: Axis) -> Span {
        let points = &edges[self.edge].points;
        let (position, start) = axis.coordinates(points[self.segment]);
        let (_, end) = axis.coordinates(points[self.segment + 1]);
        Span {
            position,
            start: start.min(end),
            end: start.max(end),
        }
    }

    fn move_to(self, edges: &mut [EdgeGeometry], axis: Axis, position: f64) {
        let points = &mut edges[self.edge].points;
        axis.update(&mut points[self.segment], position);
        axis.update(&mut points[self.segment + 1], position);
    }
}

#[derive(Clone, Copy)]
struct Span {
    position: f64,
    start: f64,
    end: f64,
}

impl Span {
    fn overlap(self, other: Self, buffer: f64) -> bool {
        self.start - other.end < buffer && other.start - self.end < buffer
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Range {
    floor: f64,
    ceil: f64,
}

impl Range {
    fn len(self) -> f64 {
        self.ceil - self.floor
    }

    fn around(span: Span, locks: impl Iterator<Item = Span>) -> Self {
        let mut floor = f64::NEG_INFINITY;
        let mut ceil = f64::INFINITY;
        if span.start != span.end {
            for other in locks {
                if other.end < span.start - SEGMENT_SPACING_BUFFER
                    || other.start > span.end + SEGMENT_SPACING_BUFFER
                {
                    continue;
                }
                if other.position <= span.position {
                    floor = floor.max(other.position);
                } else {
                    ceil = ceil.min(other.position);
                }
            }
        }
        if !floor.is_finite() {
            floor = span.position - UNBOUNDED_SEGMENT_MOVE;
        }
        if !ceil.is_finite() {
            ceil = span.position + UNBOUNDED_SEGMENT_MOVE;
        }
        Self { floor, ceil }
    }
}

/// Balances rectangular corridors after initial orthogonal routing.
///
/// # Errors
/// Returns an error for unsupported stage inputs or excessive work. Geometry
/// is published only after both movement axes finish successfully.
pub fn balance_routes(graph: &Graph, layout: &mut Layout) -> Result<(), Error> {
    balance_routes_with_work_limit(graph, layout, MAX_REVERSAL_WORK)
}

fn balance_routes_with_work_limit(
    graph: &Graph,
    layout: &mut Layout,
    mut reversal_work: usize,
) -> Result<(), Error> {
    validate_stage_input(graph, layout)?;
    let mut edges = layout.edges.clone();
    let fixed: Vec<_> = edges
        .iter()
        .map(|edge| {
            let first = edge.points[0];
            let last = edge.points[edge.points.len() - 1];
            (first.x - last.x).abs() == 1.0 || (first.y - last.y).abs() == 1.0
        })
        .collect();
    for axis in [Axis::X, Axis::Y] {
        balance_axis(&layout.nodes, &mut edges, &fixed, axis, &mut reversal_work)?;
    }
    for (edge, fixed) in edges.iter_mut().zip(fixed) {
        if !fixed {
            edge.points.dedup();
            if let [point] = edge.points.as_slice() {
                edge.points.push(Point {
                    x: point.x + 1.0,
                    y: point.y,
                });
            }
        }
    }
    layout.edges = edges;
    Ok(())
}

fn validate_stage_input(graph: &Graph, layout: &Layout) -> Result<(), Error> {
    if graph.nodes.len() > super::MAX_NODES
        || graph.edges.len() > super::MAX_EDGES
        || layout
            .edges
            .iter()
            .map(|edge| edge.points.len())
            .sum::<usize>()
            > MAX_STAGED_POINTS
    {
        return Err(Error::WorkLimit);
    }
    if graph.nodes.len() != layout.nodes.len() || graph.edges.len() != layout.edges.len() {
        return Err(Error::InvalidGraph);
    }
    for (source, node) in graph.nodes.iter().zip(&layout.nodes) {
        if source.is_group
            || source.parent_id.is_some()
            || !matches!(source.shape.as_str(), "rect" | "rectangle" | "squareRect")
            || (node.width == 1.0 && node.height == 1.0)
        {
            return Err(Error::UnsupportedGraph);
        }
        if source.id != node.id
            || !node.x.is_finite()
            || !node.y.is_finite()
            || !node.width.is_finite()
            || !node.height.is_finite()
            || !(node.x + node.width).is_finite()
            || !(node.y + node.height).is_finite()
            || node.width <= 0.0
            || node.height <= 0.0
        {
            return Err(Error::InvalidGraph);
        }
    }
    for (source, edge) in graph.edges.iter().zip(&layout.edges) {
        if source.source == source.target
            || edge.label.is_some()
            || edge
                .points
                .windows(2)
                .any(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y)
        {
            return Err(Error::UnsupportedGraph);
        }
        if source.id != edge.id
            || edge.points.len() < 2
            || edge
                .points
                .iter()
                .any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return Err(Error::InvalidGraph);
        }
    }
    Ok(())
}

fn balance_axis(
    nodes: &[NodeGeometry],
    edges: &mut [EdgeGeometry],
    fixed_edges: &[bool],
    axis: Axis,
    reversal_work: &mut usize,
) -> Result<(), Error> {
    let node_sides: Vec<_> = nodes
        .iter()
        .flat_map(|node| axis.body_sides(node))
        .collect();
    let mut movable = Vec::new();
    let mut special = Vec::new();
    for (edge, route) in edges.iter().enumerate() {
        let rails = route
            .points
            .windows(2)
            .enumerate()
            .filter_map(|(segment, points)| {
                (axis.coordinates(points[0]).0 == axis.coordinates(points[1]).0)
                    .then_some(Rail { edge, segment })
            });
        if fixed_edges[edge] {
            special.extend(rails);
        } else {
            movable.extend(rails);
        }
    }

    let mut locks = special.clone();
    let mut done = vec![false; movable.len()];
    for (index, &rail) in movable.iter().enumerate() {
        let span = rail.span(edges, axis);
        if locks
            .iter()
            .any(|other| shared(span, other.span(edges, axis)))
        {
            done[index] = true;
            locks.push(rail);
        }
    }

    while done.iter().any(|locked| !locked) {
        let ranges: Vec<_> = movable
            .iter()
            .zip(&done)
            .map(|(&rail, &done)| {
                (!done).then(|| {
                    Range::around(
                        rail.span(edges, axis),
                        node_sides
                            .iter()
                            .copied()
                            .chain(locks.iter().map(|&lock| lock.span(edges, axis))),
                    )
                })
            })
            .collect();
        let mut minimum = Range {
            floor: f64::NEG_INFINITY,
            ceil: f64::INFINITY,
        };
        for &range in ranges.iter().flatten() {
            let length = range.len();
            if !length.is_finite() {
                return Err(Error::InvalidGraph);
            }
            if length < minimum.len() {
                minimum = range;
            }
        }
        for (index, &range) in ranges.iter().enumerate() {
            if done[index] || range != Some(minimum) {
                continue;
            }
            let rail = movable[index];
            let span = rail.span(edges, axis);
            let mut batch = vec![index];
            for (other_index, &other) in movable.iter().enumerate() {
                if other_index == index || done[other_index] {
                    continue;
                }
                let other_span = other.span(edges, axis);
                let buffer = if rail.edge == other.edge {
                    0.0
                } else {
                    SEGMENT_SPACING_BUFFER
                };
                if (ranges[other_index] == range && span.overlap(other_span, buffer))
                    || shared(span, other_span)
                {
                    batch.push(other_index);
                }
            }

            batch.sort_by(|&a, &b| {
                let a = movable[a].span(edges, axis);
                let b = movable[b].span(edges, axis);
                a.position
                    .partial_cmp(&b.position)
                    .unwrap()
                    .then_with(|| a.start.partial_cmp(&b.start).unwrap())
            });
            let proposed = distribute(edges, axis, &movable, &batch, minimum);
            if accepts_proposal(
                edges,
                axis,
                &movable,
                &special,
                &batch,
                &proposed,
                reversal_work,
            )? {
                for (&index, position) in batch.iter().zip(proposed) {
                    movable[index].move_to(edges, axis, position);
                }
            }
            for index in batch {
                done[index] = true;
                locks.push(movable[index]);
            }
        }
    }
    Ok(())
}

fn shared(a: Span, b: Span) -> bool {
    (a.position - b.position).abs() <= 1.0 && a.overlap(b, 1.0)
}

fn distribute(
    edges: &[EdgeGeometry],
    axis: Axis,
    rails: &[Rail],
    batch: &[usize],
    range: Range,
) -> Vec<f64> {
    let mut positions: Vec<_> = batch
        .iter()
        .map(|&index| rails[index].span(edges, axis).position)
        .collect();
    // The batch is already sorted by position. D2 counts rounded distinct
    // coordinates before grouping nearby coordinates to keep a trunk shared.
    let mut distinct: Vec<_> = positions.iter().map(|position| position.round()).collect();
    distinct.dedup();
    let count = f64::from(u32::try_from(distinct.len()).expect("staged points are bounded"));
    let increment = (range.len() / (count + 1.0)).floor();
    if increment <= 0.0 {
        return positions;
    }
    let mut slot = range.floor + increment;
    let mut rest = positions.as_mut_slice();
    while let Some(&anchor) = rest.first() {
        let group = rest.partition_point(|position| (*position - anchor).abs() <= 1.0);
        let (aligned, remaining) = rest.split_at_mut(group);
        aligned.fill(slot);
        slot += increment;
        rest = remaining;
    }
    positions
}

fn accepts_proposal(
    edges: &[EdgeGeometry],
    axis: Axis,
    movable: &[Rail],
    special: &[Rail],
    batch: &[usize],
    positions: &[f64],
    reversal_work: &mut usize,
) -> Result<bool, Error> {
    let others = movable
        .iter()
        .enumerate()
        .filter_map(|(index, &rail)| (!batch.contains(&index)).then_some(rail))
        .chain(special.iter().copied());
    let mut reversal = false;
    for other in others {
        let other = (other.edge, other.span(edges, axis));
        for (&index, &new) in batch.iter().zip(positions) {
            let rail = movable[index];
            let old = rail.span(edges, axis);
            if rail.edge == other.0
                || old.position == new
                || old.start > other.1.end
                || other.1.start > old.end
            {
                continue;
            }
            if old.position == other.1.position || new == other.1.position {
                return Ok(false);
            }
            reversal |= (old.position < other.1.position) != (new < other.1.position);
        }
    }
    if !reversal {
        return Ok(true);
    }
    let moves = batch
        .iter()
        .zip(positions)
        .map(|(&index, &position)| (movable[index], position));
    crossings::reversal_removes_crossings(edges, axis, moves, reversal_work)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Case {
        name: String,
        graph: Graph,
        before: Layout,
        after: Vec<EdgeGeometry>,
    }

    #[test]
    fn balances_reference_corridors_across_both_axes() {
        let cases: Vec<Case> = serde_json::from_str(include_str!("balance/routes.json")).unwrap();
        for mut case in cases {
            let original_nodes = case.before.nodes.clone();
            balance_routes(&case.graph, &mut case.before).unwrap();
            assert_eq!(case.before.nodes, original_nodes, "{}", case.name);
            assert_eq!(case.before.edges, case.after, "{}", case.name);
        }
    }

    #[test]
    fn balances_the_pinned_go_observatory_routes() {
        let mut case: Case =
            serde_json::from_str(include_str!("balance/observatory.json")).unwrap();
        let original_nodes = case.before.nodes.clone();
        assert_eq!(
            case.before
                .edges
                .iter()
                .zip(&case.after)
                .filter(|(before, after)| before != after)
                .count(),
            8
        );
        balance_routes(&case.graph, &mut case.before).unwrap();
        assert_eq!(case.before.nodes, original_nodes, "{}", case.name);
        assert_eq!(case.before.edges, case.after, "{}", case.name);
    }

    fn reversal_case() -> Case {
        serde_json::from_str(include_str!("balance/reversal.json")).unwrap()
    }

    #[test]
    fn balances_the_pinned_go_reversal_case() {
        let mut case = reversal_case();
        balance_routes(&case.graph, &mut case.before).unwrap();
        assert_eq!(case.before.edges, case.after, "{}", case.name);
    }

    #[test]
    fn balance_edge_segments_still_uncrosses_different_ranges() {
        let mut case: Case =
            serde_json::from_str(include_str!("balance/different-ranges.json")).unwrap();
        balance_routes(&case.graph, &mut case.before).unwrap();
        assert_eq!(case.before.edges, case.after, "{}", case.name);
    }

    #[test]
    fn reversal_work_limit_leaves_all_layout_coordinates_unchanged() {
        let mut case = reversal_case();
        let original_nodes = case.before.nodes.clone();
        let original_edges = case.before.edges.clone();
        assert_eq!(
            balance_routes_with_work_limit(&case.graph, &mut case.before, 0),
            Err(Error::WorkLimit),
        );
        assert_eq!(case.before.nodes, original_nodes);
        assert_eq!(case.before.edges, original_edges);
    }
}

#[cfg(test)]
#[path = "balance/upstream_tests.rs"]
mod upstream_tests;
