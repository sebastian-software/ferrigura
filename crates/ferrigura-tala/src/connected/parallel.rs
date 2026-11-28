// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Reduced from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/placement/stages.go (scaleBasedOnEdges) and
// internal/routing/{coordinator,postprocess}.go (parallel route ordering and balancing).

use crate::layout_graph::port_dimensions;
use crate::{Edge, EdgeGeometry, Layout, Node, NodeGeometry};

use super::{
    Direction, LABEL_PADDING, MIN_LABEL_CLEARANCE, OUTPUT_STROKE_WIDTH, congruent_measurements,
    round_label_coordinate, separation,
};

const MAX_CAPTION_HEIGHT: f64 = 24.0;

pub(super) fn layout(
    direction_hint: Option<&str>,
    nodes: [&Node; 2],
    edges: &[Edge],
) -> Option<Layout> {
    let hint = direction_hint.filter(|hint| !hint.is_empty())?;
    let direction = Direction::from_hint(Some(hint))?;
    let first = edges.first()?;
    if !(2..=4).contains(&edges.len())
        || first.source == first.target
        || edges.iter().any(|edge| {
            edge.source_arrow
                || !edge.target_arrow
                || edge.source != first.source
                || edge.target != first.target
        })
    {
        return None;
    }
    let (width, height) = congruent_measurements(&nodes)?;
    if width.fract() != 0.0 || height.fract() != 0.0 {
        return None;
    }

    // Each endpoint sees every edge on one side. Prescale reserves one more
    // spacing interval than the number of incident parallel edges.
    let edges_per_side = u32::try_from(edges.len()).ok()?;
    let intervals = f64::from(edges_per_side + 1);
    let (width, height) = port_dimensions(width, height, edges_per_side);
    let (body_axis, body_cross) = direction.axes(width, height);
    let distance = separation(body_axis, body_cross);
    let rail_gap = (body_cross / intervals).floor();

    // Full cross-clearance leaves the edge order and its caption side stable.
    // This branch admits either no captions or one on every horizontal edge
    // with a common measured height that fits between adjacent rails.
    let caption_height = first.label_height.ceil();
    let has_captions = edges.iter().any(|edge| !edge.label.is_empty());
    if has_captions
        && (!direction.horizontal
            || caption_height > MAX_CAPTION_HEIGHT
            || edges.iter().any(|edge| {
                edge.label.is_empty()
                    || edge.label_height.ceil() != caption_height
                    || edge.label_width.ceil() + MIN_LABEL_CLEARANCE > distance - body_axis
            }))
    {
        return None;
    }

    let nodes = nodes
        .into_iter()
        .map(|node| {
            let leading = (node.id == first.source) != direction.reversed;
            let position = direction.point(if leading { 0.0 } else { distance }, 0.0);
            NodeGeometry {
                id: node.id.clone(),
                x: position.x,
                y: position.y,
                width,
                height,
            }
        })
        .collect();

    let (start, end) = if direction.reversed {
        (distance, body_axis)
    } else {
        (body_axis, distance)
    };
    let mut rail = 0.0;
    let edges = edges
        .iter()
        .map(|edge| {
            // D2 keeps authored edge order across increasing rail coordinates.
            rail += rail_gap;
            let label = has_captions.then(|| {
                let width = edge.label_width.ceil();
                let half_height = caption_height / 2.0;
                let midpoint = (start + end) / 2.0;
                let clearance = LABEL_PADDING + OUTPUT_STROKE_WIDTH / 2.0 + half_height;
                direction.point(
                    round_label_coordinate(midpoint - width / 2.0) + width / 2.0,
                    if direction.reversed {
                        rail + clearance
                    } else {
                        rail - clearance
                    },
                )
            });
            EdgeGeometry {
                id: edge.id.clone(),
                points: vec![direction.point(start, rail), direction.point(end, rail)],
                label,
            }
        })
        .collect();
    Some(Layout { nodes, edges })
}
