// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// d2layouts/d2talalayout/internal/loops/loops.go (corner routing for rectangular loops),
// internal/nodeshape/snappoints.go (squareSnapPoints), internal/layoutgraph/node.go
// (ports), internal/placement/stages.go (Prescale, Pad, Normalize), and patch.go
// (buildEdgePatch). Label and route arithmetic come from D2's lib/label/label.go
// (GetPointOnRoute, chopPrecision) and lib/geo/route.go (GetPointAtDistance).

use crate::normalize::normalize_routed;
use crate::route_output::{Anchor, RoutedEdge, RoutedLabel};
use crate::{Edge, EdgeGeometry, Error, NodeGeometry, Point};

const SHARED_PORT_LOOP_GAP: f64 = 30.0;
pub(super) const SIDE_EDGE_SPACING: f64 = 40.0;
const PLACEMENT_PADDING: f64 = 1000.0;

/// Lays out one unpinned rectangle and its only edge, a self-loop.
///
/// The first loop takes the first free corner pair (left, then top). The
/// nearest of the rectangle's three snap points on each side is its quarter
/// port. The result uses the authored traversal and the D2 connection's
/// final label center after normalization.
pub(crate) fn isolated_rectangle_loop(
    node: &mut NodeGeometry,
    edge: &Edge,
) -> Result<EdgeGeometry, Error> {
    prepare(node);
    let points = corner_route(node, Corner::TopLeft, SHARED_PORT_LOOP_GAP);

    // The reference adapter takes integer measured text dimensions.
    let label = (!edge.label.is_empty()).then(|| RoutedLabel {
        anchor: Anchor::OutsideTopCenter,
        percentage: 0.5,
        width: edge.label_width.ceil(),
        height: edge.label_height.ceil(),
    });
    let mut route = routed_loop(edge, points, label);
    normalize_routed(std::slice::from_mut(node), std::slice::from_mut(&mut route))?;
    route.into_geometry()
}

/// Lays out the unpinned rectangle when all its edges are unlabeled self-loops.
///
/// The Go adapter normalizes source-only arrows before layout, so there are
/// three arrowhead classes: one arrow, both arrows, and no arrows. With equal
/// label areas, the upstream stable ordering preserves edge order inside
/// each class. Classes take corners in that order and grow their routes
/// independently.
pub(crate) fn isolated_unlabeled_loops(
    node: &mut NodeGeometry,
    edges: &[Edge],
) -> Result<Vec<EdgeGeometry>, Error> {
    prepare(node);
    let mut active = [false; 3];
    for edge in edges {
        active[arrow_class(edge)] = true;
    }
    let mut order = [0; 3];
    let mut next_corner = 0;
    for (index, occupied) in active.into_iter().enumerate() {
        if occupied {
            order[index] = next_corner;
            next_corner += 1;
        }
    }

    let corners = [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft];
    let mut gaps = [SHARED_PORT_LOOP_GAP; 3];
    let mut routes = Vec::with_capacity(edges.len());
    for edge in edges {
        let slot = order[arrow_class(edge)];
        let points = corner_route(node, corners[slot], gaps[slot]);
        gaps[slot] += SHARED_PORT_LOOP_GAP;
        routes.push(routed_loop(edge, points, None));
    }

    normalize_routed(std::slice::from_mut(node), &mut routes)?;
    routes.into_iter().map(RoutedEdge::into_geometry).collect()
}

fn arrow_class(edge: &Edge) -> usize {
    match (edge.source_arrow, edge.target_arrow) {
        (true, false) | (false, true) => 0,
        (true, true) => 1,
        (false, false) => 2,
    }
}

fn prepare(node: &mut NodeGeometry) {
    // Prescale counts zero adjacent nodes when all edges are loops, so the
    // minimum side is one SideEdgeSpacing. placeNodes starts this node at 0,0
    // and Pad runs before routing. The padded origin affects label rounding.
    node.width = node.width.max(SIDE_EDGE_SPACING);
    node.height = node.height.max(SIDE_EDGE_SPACING);
    node.x = PLACEMENT_PADDING;
    node.y = PLACEMENT_PADDING;
}

#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
}

fn corner_route(node: &NodeGeometry, corner: Corner, gap: f64) -> [Point; 5] {
    let quarter_x = node.x + (node.width * 0.25).round();
    let quarter_y = node.y + (node.height * 0.25).round();
    let (from, to, from_bend, to_bend, from_vertical) = match corner {
        Corner::TopLeft => (
            Point {
                x: node.x,
                y: quarter_y,
            },
            Point {
                x: quarter_x,
                y: node.y,
            },
            Point {
                x: node.x - gap,
                y: quarter_y,
            },
            Point {
                x: quarter_x,
                y: node.y - gap,
            },
            false,
        ),
        Corner::TopRight => {
            let side_x = node.x + node.width.round();
            let port_x = node.x + (node.width * 0.75).round();
            (
                Point {
                    x: port_x,
                    y: node.y,
                },
                Point {
                    x: side_x,
                    y: quarter_y,
                },
                Point {
                    x: port_x,
                    y: node.y - gap,
                },
                Point {
                    x: side_x + gap,
                    y: quarter_y,
                },
                true,
            )
        }
        Corner::BottomLeft => {
            let side_y = node.y + node.height.round();
            let port_y = node.y + (node.height * 0.75).round();
            (
                Point {
                    x: quarter_x,
                    y: side_y,
                },
                Point {
                    x: node.x,
                    y: port_y,
                },
                Point {
                    x: quarter_x,
                    y: side_y + gap,
                },
                Point {
                    x: node.x - gap,
                    y: port_y,
                },
                true,
            )
        }
    };
    let intersection = if from_vertical {
        Point {
            x: to_bend.x,
            y: from_bend.y,
        }
    } else {
        Point {
            x: from_bend.x,
            y: to_bend.y,
        }
    };
    [from, from_bend, intersection, to_bend, to]
}

fn routed_loop(edge: &Edge, points: [Point; 5], label: Option<RoutedLabel>) -> RoutedEdge {
    RoutedEdge {
        id: edge.id.clone(),
        points: points.to_vec(),
        label,
        reversed_for_layout: edge.source_arrow && !edge.target_arrow,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Case {
        name: String,
        node: NodeGeometry,
        edge: Edge,
        expected_node: NodeGeometry,
        expected_edge: EdgeGeometry,
    }

    #[test]
    fn isolated_rectangles_match_pinned_go_layout() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("routing/rectangle-loops.json")).unwrap();
        assert_eq!(cases.len(), 9);
        for mut case in cases {
            let edge = isolated_rectangle_loop(&mut case.node, &case.edge).unwrap();
            assert_eq!(case.node, case.expected_node, "{}", case.name);
            assert_eq!(edge, case.expected_edge, "{}", case.name);
        }
    }

    #[derive(Deserialize)]
    struct LoopGroup {
        name: String,
        node: NodeGeometry,
        edges: Vec<Edge>,
        expected_node: NodeGeometry,
        expected_edges: Vec<EdgeGeometry>,
    }

    #[test]
    fn unlabeled_loop_groups_match_pinned_go_layout() {
        let cases: Vec<LoopGroup> =
            serde_json::from_str(include_str!("routing/rectangle-loop-groups.json")).unwrap();
        assert_eq!(cases.len(), 5);
        for mut case in cases {
            let edges = isolated_unlabeled_loops(&mut case.node, &case.edges).unwrap();
            assert_eq!(case.node, case.expected_node, "{}", case.name);
            assert_eq!(edges, case.expected_edges, "{}", case.name);
        }
    }
}
