// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/{compaction,node_placement}.go
// and internal/layoutgraph/{node,geometry_policy}.go.

use std::collections::HashMap;

use super::{compaction::Axis, optimize_phase};
use crate::{Error, Graph, NodeGeometry, valid_dimension};

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
    if graph.edges.iter().any(|edge| edge.source == edge.target) {
        return Err(Error::UnsupportedGraph);
    }
    let grid = optimize_phase(graph, seed)?;
    let gaps = edge_gaps(graph)?;
    let mut nodes = graph
        .nodes
        .iter()
        .zip(grid.nodes)
        .map(|(node, location)| NodeGeometry {
            id: location.id,
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
    Ok(nodes)
}

fn edge_gaps(graph: &Graph) -> Result<Vec<Vec<[f64; 2]>>, Error> {
    let indices: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    let size = graph.nodes.len();
    let mut gaps = vec![vec![[NODE_GAP; 2]; size]; size];
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
        let label = [edge.label_width, edge.label_height];
        for (axis, dimension) in label.into_iter().enumerate() {
            let gap = gaps[source][target][axis]
                .max(CONNECTED_NODE_GAP)
                .max(dimension.ceil() + EDGE_LABEL_PADDING);
            gaps[source][target][axis] = gap;
            gaps[target][source][axis] = gap;
        }
    }
    Ok(gaps)
}

fn transition_axis(nodes: &mut [NodeGeometry], gaps: &[Vec<[f64; 2]>], axis: Axis, cell: f64) {
    let predecessors = visibility_predecessors(nodes, gaps, axis);
    let mut order: Vec<_> = (0..nodes.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        start(axis, &nodes[a])
            .total_cmp(&start(axis, &nodes[b]))
            .then(a.cmp(&b))
    });
    for target in order {
        let anchor = predecessors[target]
            .iter()
            .copied()
            .fold(None, |best, index| {
                if best.is_none_or(|old| end(axis, &nodes[index]) > end(axis, &nodes[old])) {
                    Some(index)
                } else {
                    best
                }
            });
        let Some(anchor) = anchor else { continue };
        let body = &nodes[anchor];
        let mut floor =
            ((start(axis, body) + TRANSITION_FACTOR * length(axis, body)) / cell).ceil();
        while floor * cell - end(axis, body) <= CONNECTED_NODE_GAP {
            floor += 1.0;
        }
        let coordinate = match axis {
            Axis::Horizontal => &mut nodes[target].x,
            Axis::Vertical => &mut nodes[target].y,
        };
        *coordinate = coordinate.max(floor * cell);
    }
}

fn visibility_predecessors(
    nodes: &[NodeGeometry],
    gaps: &[Vec<[f64; 2]>],
    axis: Axis,
) -> Vec<Vec<usize>> {
    let across = axis.opposite();
    let mut predecessors = vec![Vec::new(); nodes.len()];
    for (first_index, first) in nodes.iter().enumerate() {
        for (second_index, second) in nodes.iter().enumerate() {
            if start(axis, first) >= start(axis, second) {
                continue;
            }
            let gap = oriented_gap(first, second, gaps[first_index][second_index]);
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

fn oriented_gap(
    first: &NodeGeometry,
    second: &NodeGeometry,
    [horizontal, vertical]: [f64; 2],
) -> f64 {
    let across =
        |axis| end(axis, first) < start(axis, second) || end(axis, second) < start(axis, first);
    match (across(Axis::Horizontal), across(Axis::Vertical)) {
        (true, false) => horizontal,
        (false, true) => vertical,
        _ => horizontal.min(vertical),
    }
}

fn start(axis: Axis, body: &NodeGeometry) -> f64 {
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

fn end(axis: Axis, body: &NodeGeometry) -> f64 {
    start(axis, body) + length(axis, body)
}
