// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{compaction,node_placement}.go
// and internal/layoutgraph/{node,geometry_policy}.go.

use std::collections::HashMap;

use super::{PhaseState, compaction::Axis, run};
use crate::{Error, Graph, NodeGeometry, Point, valid_dimension};

const NODE_GAP: f64 = 20.0;
const CONNECTED_NODE_GAP: f64 = 60.0;
const EDGE_LABEL_PADDING: f64 = 10.0;
const TRANSITION_FACTOR: f64 = 3.0;

/// Initializes the sized optimizer's rectangular bodies on the grid.
/// The transition inflates each axis of the preceding sizeless solution.
///
/// # Errors
/// Returns an error for unsupported component topology or invalid measurements.
pub fn transition_to_sized(graph: &Graph, seed: i64) -> Result<Vec<NodeGeometry>, Error> {
    let state = prepared_transition(graph, seed)?;
    Ok(state.nodes)
}

pub(super) struct Transition {
    pub(super) nodes: Vec<NodeGeometry>,
    pub(super) gaps: Vec<Vec<Spacing>>,
    pub(super) phase: PhaseState,
}

pub(super) fn prepared_transition(graph: &Graph, seed: i64) -> Result<Transition, Error> {
    if graph.edges.iter().any(|edge| edge.source == edge.target) {
        return Err(Error::UnsupportedGraph);
    }
    let phase = run(graph, seed, false)?;
    let grid = &phase.grid;
    let gaps = edge_gaps(graph)?;
    let mut nodes = graph
        .nodes
        .iter()
        .zip(&grid.nodes)
        .map(|(node, location)| NodeGeometry {
            id: node.id.clone(),
            x: f64::from(location.x),
            y: f64::from(location.y),
            width: node.width,
            height: node.height,
        })
        .collect::<Vec<_>>();
    for axis in [Axis::Horizontal, Axis::Vertical] {
        transition_axis(&mut nodes, &gaps, axis, grid.cell_size);
    }
    for node in &mut nodes {
        for coordinate in [&mut node.x, &mut node.y] {
            if *coordinate % grid.cell_size != 0.0 {
                *coordinate = (*coordinate / grid.cell_size).floor() * grid.cell_size;
            }
        }
    }
    Ok(Transition { nodes, gaps, phase })
}

#[derive(Clone, Copy, Default)]
pub(super) struct Spacing {
    pub(super) edge_count: usize,
    pub(super) minimum: [f64; 2],
}

impl Spacing {
    pub(super) fn gap(self) -> [f64; 2] {
        if self.edge_count == 0 {
            return [NODE_GAP; 2];
        }
        self.minimum.map(|size| size.max(CONNECTED_NODE_GAP))
    }
}

pub(super) fn edge_gaps(graph: &Graph) -> Result<Vec<Vec<Spacing>>, Error> {
    let indices: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    let size = graph.nodes.len();
    let mut gaps = vec![vec![Spacing::default(); size]; size];
    for edge in &graph.edges {
        if !valid_dimension(edge.label_width) || !valid_dimension(edge.label_height) {
            return Err(Error::InvalidGraph);
        }
        let source = *indices
            .get(edge.source.as_str())
            .ok_or(Error::InvalidGraph)?;
        let target = *indices
            .get(edge.target.as_str())
            .ok_or(Error::InvalidGraph)?;
        let mut spacing = gaps[source][target];
        spacing.edge_count += 1;
        let label = [edge.label_width, edge.label_height];
        for (axis, dimension) in label.into_iter().enumerate() {
            spacing.minimum[axis] =
                spacing.minimum[axis].max(dimension.ceil() + EDGE_LABEL_PADDING);
        }
        gaps[source][target] = spacing;
        gaps[target][source] = spacing;
    }
    Ok(gaps)
}

fn transition_axis(nodes: &mut [NodeGeometry], gaps: &[Vec<Spacing>], axis: Axis, cell: f64) {
    let predecessors = visibility_predecessors(nodes, gaps, axis);
    inflate_axis(
        nodes,
        gaps,
        &predecessors,
        axis,
        cell,
        TRANSITION_FACTOR,
        true,
    );
}

pub(super) fn inflate_axis(
    nodes: &mut [NodeGeometry],
    gaps: &[Vec<Spacing>],
    predecessors: &[Vec<usize>],
    axis: Axis,
    cell: f64,
    factor: f64,
    transition: bool,
) {
    for target in ordered_along_axis(nodes, axis) {
        let Some(anchor) = nearest_predecessor(nodes, predecessors, axis, target) else {
            continue;
        };
        let spacing = if transition {
            CONNECTED_NODE_GAP
        } else {
            oriented_body_gap(&nodes[target], &nodes[anchor], gaps[target][anchor])
        };
        let floor = compaction_floor(&nodes[anchor], axis, cell, factor, spacing);
        let coordinate = match axis {
            Axis::Horizontal => &mut nodes[target].x,
            Axis::Vertical => &mut nodes[target].y,
        };
        *coordinate = coordinate.max(floor * cell);
    }
}

pub(super) fn ordered_along_axis(nodes: &[NodeGeometry], axis: Axis) -> Vec<usize> {
    let mut order: Vec<_> = (0..nodes.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        start(axis, &nodes[a])
            .total_cmp(&start(axis, &nodes[b]))
            .then(a.cmp(&b))
    });
    order
}

pub(super) fn nearest_predecessor(
    nodes: &[NodeGeometry],
    predecessors: &[Vec<usize>],
    axis: Axis,
    target: usize,
) -> Option<usize> {
    predecessors[target]
        .iter()
        .copied()
        .fold(None, |best, index| {
            if best.is_none_or(|old| end(axis, &nodes[index]) > end(axis, &nodes[old])) {
                Some(index)
            } else {
                best
            }
        })
}

pub(super) fn compaction_floor(
    anchor: &NodeGeometry,
    axis: Axis,
    cell: f64,
    factor: f64,
    gap: f64,
) -> f64 {
    let mut floor = ((start(axis, anchor) + factor * length(axis, anchor)) / cell).ceil();
    while floor * cell - end(axis, anchor) <= gap {
        floor += 1.0;
    }
    floor
}

pub(super) fn oriented_body_gap(
    first: &NodeGeometry,
    second: &NodeGeometry,
    spacing: Spacing,
) -> f64 {
    oriented_gap(
        first.x,
        first.y,
        first.width,
        first.height,
        second,
        spacing.gap(),
    )
}

pub(super) fn overlaps_at(
    first: &NodeGeometry,
    second: &NodeGeometry,
    point: Point,
    spacing: Spacing,
) -> bool {
    let gap = oriented_gap(
        point.x,
        point.y,
        first.width,
        first.height,
        second,
        spacing.gap(),
    );
    point.x < second.x + second.width + gap
        && point.x + first.width + gap > second.x
        && point.y < second.y + second.height + gap
        && point.y + first.height + gap > second.y
}

pub(super) fn visibility_predecessors(
    nodes: &[NodeGeometry],
    gaps: &[Vec<Spacing>],
    axis: Axis,
) -> Vec<Vec<usize>> {
    let across = axis.opposite();
    let mut predecessors = vec![Vec::new(); nodes.len()];
    for (first_index, first) in nodes.iter().enumerate() {
        for (second_index, second) in nodes.iter().enumerate() {
            if start(axis, first) >= start(axis, second) {
                continue;
            }
            let gap = oriented_body_gap(first, second, gaps[first_index][second_index]);
            if start(across, first) > end(across, second) + gap
                || end(across, first) + gap < start(across, second)
            {
                continue;
            }
            let blocked = nodes.iter().enumerate().any(|(index, blocker)| {
                index != first_index
                    && index != second_index
                    && start(axis, blocker) >= end(axis, first)
                    && end(axis, blocker) <= start(axis, second)
                    && start(across, blocker) <= start(across, first).max(start(across, second))
                    && end(across, blocker) >= end(across, first).min(end(across, second))
            });
            if !blocked {
                predecessors[second_index].push(first_index);
            }
        }
    }
    predecessors
}

pub(super) fn oriented_gap(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    other: &NodeGeometry,
    [horizontal, vertical]: [f64; 2],
) -> f64 {
    let horizontal_gap = x + width < other.x || other.x + other.width < x;
    let vertical_gap = y + height < other.y || other.y + other.height < y;
    match (horizontal_gap, vertical_gap) {
        (true, false) => horizontal,
        (false, true) => vertical,
        _ => horizontal.min(vertical),
    }
}

pub(super) fn start(axis: Axis, body: &NodeGeometry) -> f64 {
    match axis {
        Axis::Horizontal => body.x,
        Axis::Vertical => body.y,
    }
}

fn length(axis: Axis, body: &NodeGeometry) -> f64 {
    match axis {
        Axis::Horizontal => body.width,
        Axis::Vertical => body.height,
    }
}

pub(super) fn end(axis: Axis, body: &NodeGeometry) -> f64 {
    start(axis, body) + length(axis, body)
}
