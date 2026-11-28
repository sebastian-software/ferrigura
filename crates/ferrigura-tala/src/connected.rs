// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Reduced from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See d2layouts/d2talalayout/internal/{layoutgraph,placement,routing},
// d2layouts/d2talalayout/patch.go, and lib/label/label.go.

use std::collections::{HashMap, HashSet};

use crate::{Edge, EdgeGeometry, Graph, Layout, Node, NodeGeometry, Point};

const MIN_BODY_GAP: f64 = 60.0;
const IDEAL_BODY_GAP: f64 = 2.5 * MIN_BODY_GAP;
const PLACEMENT_PADDING: f64 = 1_000.0;
const LABEL_PADDING: f64 = 5.0;
const LAYOUT_STROKE_WIDTH: f64 = 3.0;
const OUTPUT_STROKE_WIDTH: f64 = 2.0;

// The differential corpus covers ordinary measured rectangles and captions with
// room between the two bodies. Larger captions take part in upstream placement.
const MIN_LABEL_CLEARANCE: f64 = 20.0;
const MAX_LABEL_CROSS_SIZE: f64 = 48.0;
const MAX_STRAIGHT_CHAIN: usize = 5;

pub(crate) fn layout(graph: &Graph) -> Option<Layout> {
    if graph.nodes.len() > MAX_STRAIGHT_CHAIN {
        return None;
    }
    let nodes: Vec<_> = graph.nodes.iter().collect();
    layout_path(graph.direction.as_deref(), &nodes, &graph.edges)
}

// The surrounding groups have already been verified by the hierarchy adapter.
pub(crate) fn layout_pair(
    direction: Option<&str>,
    nodes: [&Node; 2],
    edge: &Edge,
) -> Option<Layout> {
    layout_path(direction, &nodes, std::slice::from_ref(edge))
}

fn layout_path(direction_hint: Option<&str>, nodes: &[&Node], edges: &[Edge]) -> Option<Layout> {
    let first = nodes.first()?;
    if !(2..=MAX_STRAIGHT_CHAIN).contains(&nodes.len())
        || edges.len() + 1 != nodes.len()
        || !(100.0..=220.0).contains(&first.width)
        || !(40.0..=92.0).contains(&first.height)
        || nodes.iter().any(|node| {
            node.direction.is_some()
                || first.width != node.width
                || first.height != node.height
                || node.label_width.ceil() > node.width
                || node.label_height.ceil() > node.height
        })
    {
        return None;
    }

    // Longer chains can fold, and without a direction hint the reference can
    // choose either axis. This branch covers short, standard measured paths.
    let implicit = matches!(direction_hint, None | Some(""));
    if (nodes.len() > 2 || implicit)
        && (first.width != 152.0
            || first.height != 41.0
            || edges
                .iter()
                .any(|edge| edge.source_arrow == edge.target_arrow)
            || nodes
                .iter()
                .any(|node| node.label_width != 0.0 || node.label_height != 0.0))
    {
        return None;
    }
    if nodes.len() > 2 && edges.iter().any(|edge| edge.source_arrow) {
        return None;
    }

    let direction = Direction::from_hint(direction_hint)?;
    let (body_axis, body_cross) = direction.axes(first.width, first.height);
    if body_axis.fract() != 0.0 {
        return None;
    }
    let count = u32::try_from(nodes.len()).ok()?;
    let ranks = rank_nodes(nodes, edges, count)?;

    // ComputeCellSize supplies the compaction lattice. The minimum legal
    // separation on that lattice is then bounded by GapNormalization.
    let smaller = body_axis.min(body_cross);
    let larger = body_axis.max(body_cross);
    let cell_size = if larger < 3.0 * smaller {
        larger.ceil()
    } else {
        (1.5 * smaller).ceil()
    };
    let distance = (((body_axis + MIN_BODY_GAP) / cell_size).ceil() * cell_size)
        .min(body_axis + IDEAL_BODY_GAP);

    // Pad precedes BalanceEdgeSegments. For the collinear routes between equal
    // bodies, the perpendicular coordinate is the floored center of their span.
    let line = (PLACEMENT_PADDING + body_cross / 2.0).floor();
    let mut cross_origin = PLACEMENT_PADDING;
    let outside_negative = direction.horizontal != direction.reversed;
    let caption_cross = |path: f64, label_cross: f64, stroke_width: f64| {
        let padding = LABEL_PADDING + stroke_width / 2.0;
        if outside_negative {
            path - padding - label_cross
        } else {
            path + padding
        }
    };
    let mut caption_sizes = Vec::with_capacity(edges.len());
    for edge in edges {
        if edge.label.is_empty() {
            caption_sizes.push(None);
            continue;
        }
        let (label_axis, label_cross) =
            direction.axes(edge.label_width.ceil(), edge.label_height.ceil());
        if label_axis + MIN_LABEL_CLEARANCE > distance - body_axis
            || label_cross > MAX_LABEL_CROSS_SIZE
        {
            return None;
        }
        // The caption can extend the origin across the edge. The layout uses
        // stroke width 3 here, then the output adapter evaluates it with width 2.
        cross_origin = cross_origin.min(
            round_label_coordinate(caption_cross(line, label_cross, LAYOUT_STROKE_WIDTH)).floor(),
        );
        caption_sizes.push(Some((label_axis, label_cross)));
    }

    let position = |node_id: &str| {
        let rank = ranks[node_id];
        let slot = if direction.reversed {
            count - 1 - rank
        } else {
            rank
        };
        f64::from(slot) * distance
    };
    let nodes = nodes
        .iter()
        .map(|node| {
            let top_left = direction.point(position(&node.id), PLACEMENT_PADDING - cross_origin);
            NodeGeometry {
                id: node.id.clone(),
                x: top_left.x,
                y: top_left.y,
                width: node.width,
                height: node.height,
            }
        })
        .collect();

    let line = line - cross_origin;
    let edges = edges
        .iter()
        .zip(caption_sizes)
        .map(|(edge, caption_size)| {
            let from = position(&edge.source);
            let to = position(&edge.target);
            let (start, end) = if from < to {
                (from + body_axis, to)
            } else {
                (from, to + body_axis)
            };
            let midpoint = (start + end) / 2.0;
            let label = caption_size.map(|(label_axis, label_cross)| {
                direction.point(
                    round_label_coordinate(midpoint - label_axis / 2.0) + label_axis / 2.0,
                    round_label_coordinate(caption_cross(line, label_cross, OUTPUT_STROKE_WIDTH))
                        + label_cross / 2.0,
                )
            });
            EdgeGeometry {
                id: edge.id.clone(),
                points: vec![direction.point(start, line), direction.point(end, line)],
                label,
            }
        })
        .collect();
    Some(Layout { nodes, edges })
}

fn rank_nodes<'a>(
    nodes: &[&'a Node],
    edges: &'a [Edge],
    count: u32,
) -> Option<HashMap<&'a str, u32>> {
    let ids: HashSet<_> = nodes.iter().map(|node| node.id.as_str()).collect();
    let mut successors = HashMap::with_capacity(edges.len());
    let mut has_predecessor = HashSet::with_capacity(edges.len());
    for edge in edges {
        let (source, target) = layout_endpoints(edge);
        if !ids.contains(source)
            || !ids.contains(target)
            || successors.insert(source, target).is_some()
            || !has_predecessor.insert(target)
        {
            return None;
        }
    }
    // There must be one start and one path reaching every node. A disconnected
    // cycle leaves the walk from the start short of the total node count.
    let mut current = nodes
        .iter()
        .find(|node| !has_predecessor.contains(node.id.as_str()))?
        .id
        .as_str();
    let mut ranks = HashMap::with_capacity(nodes.len());
    for rank in 0..count {
        ranks.insert(current, rank);
        match successors.get(current) {
            Some(&next) => current = next,
            None => break,
        }
    }
    (ranks.len() == nodes.len()).then_some(ranks)
}

fn layout_endpoints(edge: &Edge) -> (&str, &str) {
    // D2 swaps source-only arrow edges for placement and restores their
    // authored traversal and caption side when building the output patch.
    if edge.source_arrow && !edge.target_arrow {
        (&edge.target, &edge.source)
    } else {
        (&edge.source, &edge.target)
    }
}

struct Direction {
    horizontal: bool,
    reversed: bool,
}

impl Direction {
    fn from_hint(direction: Option<&str>) -> Option<Self> {
        let (horizontal, reversed) = match direction {
            Some("LR") => (true, false),
            Some("RL") => (true, true),
            Some("BT") => (false, true),
            None | Some("" | "TD" | "TB") => (false, false),
            _ => return None,
        };
        Some(Self {
            horizontal,
            reversed,
        })
    }

    fn axes(&self, x: f64, y: f64) -> (f64, f64) {
        if self.horizontal { (x, y) } else { (y, x) }
    }

    fn point(&self, along: f64, across: f64) -> Point {
        let (x, y) = self.axes(along, across);
        Point { x, y }
    }
}

fn round_label_coordinate(value: f64) -> f64 {
    // D2's chopPrecision narrows the scaled coordinate before rounding it.
    (f64::from((value * 10_000.0) as f32) / 10_000.0).round()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Case {
        name: String,
        input: Graph,
        expected: Layout,
    }

    #[test]
    fn connected_rectangles_match_pinned_go_layout() {
        let fixtures = [
            include_str!("connected/pairs.json"),
            include_str!("connected/chains.json"),
        ];
        for fixture in fixtures {
            let cases: Vec<Case> = serde_json::from_str(fixture).unwrap();
            assert!(!cases.is_empty());
            for case in cases {
                assert_eq!(layout(&case.input), Some(case.expected), "{}", case.name);
            }
        }
    }

    #[test]
    fn excludes_unported_connected_placements() {
        let mut cases: Vec<Case> =
            serde_json::from_str(include_str!("connected/pairs.json")).unwrap();
        let input = &mut cases[0].input;
        input.nodes[0].width += 2.0;
        assert!(layout(input).is_none());
        input.nodes[0].width -= 2.0;
        input.edges[0].label = "Long edge caption".into();
        input.edges[0].label_width = 150.0;
        input.edges[0].label_height = 20.0;
        assert!(layout(input).is_none());
    }

    #[test]
    fn excludes_branches_and_disconnected_cycles() {
        let mut cases: Vec<Case> =
            serde_json::from_str(include_str!("connected/chains.json")).unwrap();
        let input = &mut cases[0].input;
        input.edges[1].source = input.edges[0].source.clone();
        assert!(layout(input).is_none());
        input.edges[1].source = input.edges[2].target.clone();
        input.edges[1].target = input.edges[2].source.clone();
        assert!(layout(input).is_none());
    }
}
