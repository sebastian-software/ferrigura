// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from TALA's container edge abduction, compaction, and node fitting.
// See NOTICE.md for the original revision.

use super::{CONTAINER_PADDING, LABEL_FIT_MARGIN, MAX_FIXED_PADDING_LABEL_HEIGHT, node_geometry};
use crate::{EdgeGeometry, Graph, Layout, Point};

const RECTANGLE_WIDTH: f64 = 152.0;
const RECTANGLE_HEIGHT: f64 = 41.0;

/// Places one singly directed connection between a container and an exterior rectangle.
/// The interior endpoint can be the container's child or the container boundary.
pub(super) fn layout(graph: &Graph) -> Option<Layout> {
    let [edge] = graph.edges.as_slice() else {
        return None;
    };
    if graph.nodes.len() != 3 || edge.source_arrow == edge.target_arrow || !edge.label.is_empty() {
        return None;
    }

    let frame = graph
        .nodes
        .iter()
        .find(|node| node.is_group && node.parent_id.is_none())?;
    let child = graph
        .nodes
        .iter()
        .find(|node| node.parent_id.as_deref() == Some(frame.id.as_str()))?;
    let peer = graph
        .nodes
        .iter()
        .find(|node| !node.is_group && node.parent_id.is_none())?;
    if child.is_group
        || !matches!(frame.direction.as_deref(), None | Some(""))
        || frame.width != 0.0
        || frame.height != 0.0
        || !frame.label.is_empty()
            && (frame.label_width.ceil() > RECTANGLE_WIDTH
                || frame.label_height.ceil() > MAX_FIXED_PADDING_LABEL_HEIGHT)
        || [child, peer].into_iter().any(|node| {
            !matches!(node.direction.as_deref(), None | Some(""))
                || !matches!(node.shape.as_str(), "rect" | "rectangle" | "squareRect")
                || node.width != RECTANGLE_WIDTH
                || node.height != RECTANGLE_HEIGHT
                || node.label_width.ceil() > node.width
                || node.label_height.ceil() > node.height
        })
    {
        return None;
    }

    let (inside_is_source, inside_endpoint) = if edge.target == peer.id {
        (true, &edge.source)
    } else if edge.source == peer.id {
        (false, &edge.target)
    } else {
        return None;
    };
    if *inside_endpoint != frame.id && *inside_endpoint != child.id {
        return None;
    }

    let (horizontal, reversed) = match graph.direction.as_deref()? {
        "TB" | "TD" => (false, false),
        "BT" => (false, true),
        "LR" => (true, false),
        "RL" => (true, true),
        _ => return None,
    };
    let frame_is_directed_source = inside_is_source != edge.source_arrow;
    let frame_leads = frame_is_directed_source != reversed;
    let point = |along, across| {
        if horizontal {
            Point {
                x: along,
                y: across,
            }
        } else {
            Point {
                x: across,
                y: along,
            }
        }
    };

    // Edge abduction connects the exterior peer to the container during
    // placement. ComputeCellSize sees the short side of the peer and the
    // outer frame, so the orthogonal search has a 62-unit lattice.
    let lattice = (1.5 * RECTANGLE_HEIGHT).ceil();
    let (body_axis, body_cross) = if horizontal {
        (RECTANGLE_WIDTH, RECTANGLE_HEIGHT)
    } else {
        (RECTANGLE_HEIGHT, RECTANGLE_WIDTH)
    };
    let margin = 2.0 * CONTAINER_PADDING;
    let frame_axis = body_axis + margin;
    // The first fit can reserve ten more units for the measured title;
    // GapNormalization wraps the frame back to its content bounds.
    let initial_frame_axis = frame_axis
        + if frame.label.is_empty() {
            0.0
        } else {
            LABEL_FIT_MARGIN / 2.0
        };
    let compacted_distance =
        |leading_body: f64| ((leading_body + CONTAINER_PADDING) / lattice).ceil() * lattice;
    let (frame_axis_origin, peer_axis_origin) = if frame_leads {
        (0.0, compacted_distance(initial_frame_axis))
    } else {
        (compacted_distance(body_axis), 0.0)
    };

    let (inside_axis_origin, inside_axis_size) = if *inside_endpoint == frame.id {
        (frame_axis_origin, frame_axis)
    } else {
        (frame_axis_origin + CONTAINER_PADDING, body_axis)
    };
    let line = CONTAINER_PADDING + (body_cross / 2.0).floor();
    let (inside_port, peer_port) = if frame_leads {
        (inside_axis_origin + inside_axis_size, peer_axis_origin)
    } else {
        (inside_axis_origin, peer_axis_origin + body_axis)
    };
    let mut points = vec![point(inside_port, line), point(peer_port, line)];
    if !inside_is_source {
        points.reverse();
    }

    let nodes = graph
        .nodes
        .iter()
        .map(|node| {
            let mut geometry = node_geometry(node);
            let position = if node.id == frame.id {
                geometry.width = RECTANGLE_WIDTH + margin;
                geometry.height = RECTANGLE_HEIGHT + margin;
                point(frame_axis_origin, 0.0)
            } else if node.id == child.id {
                point(frame_axis_origin + CONTAINER_PADDING, CONTAINER_PADDING)
            } else {
                point(peer_axis_origin, CONTAINER_PADDING)
            };
            geometry.x = position.x;
            geometry.y = position.y;
            geometry
        })
        .collect();
    Some(Layout {
        nodes,
        edges: vec![EdgeGeometry {
            id: edge.id.clone(),
            points,
            label: None,
        }],
    })
}
