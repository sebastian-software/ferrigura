// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use crate::routing::SIDE_EDGE_SPACING;

use super::{LayoutGraph, Shape};

pub(super) fn apply(graph: &mut LayoutGraph) {
    for (index, node) in graph.nodes.iter_mut().enumerate() {
        if node.shape == Shape::Circle {
            let side = node.width.max(node.height);
            node.width = side;
            node.height = side;
        }
        if node.incident_edges.is_empty() {
            continue;
        }
        let mut counts = HashMap::<usize, u32>::new();
        for &edge in &node.incident_edges {
            let edge = &graph.edges[edge];
            let adjacent = if edge.from == index {
                edge.to
            } else {
                edge.from
            };
            if adjacent != index {
                *counts.entry(adjacent).or_default() += 1;
            }
        }
        let busiest_neighbor = counts.values().copied().max().unwrap_or_default();
        let sides = u32::try_from(counts.len().min(4)).expect("at most four sides");
        let edges_per_side =
            busiest_neighbor.max(counts.values().copied().sum::<u32>().div_ceil(sides.max(1)));
        (node.width, node.height) = port_dimensions(node.width, node.height, edges_per_side);
    }
}

// Ferrigura has already measured text at an explicit size. The Go prescale
// path for that input reserves geometric space and leaves label metrics alone.
pub(crate) fn port_dimensions(width: f64, height: f64, edges_per_side: u32) -> (f64, f64) {
    if edges_per_side == 1 {
        return (width, height);
    }
    let side = (f64::from(edges_per_side) + 1.0) * SIDE_EDGE_SPACING;
    (width.max(side), height.max(side))
}
