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

use crate::{Edge, EdgeGeometry, NodeGeometry, Point};

const SHARED_PORT_LOOP_GAP: f64 = 30.0;
pub(super) const SIDE_EDGE_SPACING: f64 = 40.0;
const PLACEMENT_PADDING: f64 = 1000.0;
const LABEL_PADDING: f64 = 5.0;
const LAYOUT_STROKE_WIDTH: f64 = 3.0;
const OUTPUT_STROKE_WIDTH: f64 = 2.0;

/// Lays out one unpinned rectangle and its only edge, a self-loop.
///
/// The first loop takes the first free corner pair (left, then top). The
/// nearest of the rectangle's three snap points on each side is its quarter
/// port. The result uses the authored traversal and the D2 connection's
/// final label center after normalization.
pub(crate) fn isolated_rectangle_loop(node: &mut NodeGeometry, edge: &Edge) -> EdgeGeometry {
    prepare(node);
    let points = corner_route(node, Corner::TopLeft, SHARED_PORT_LOOP_GAP);

    // The reference adapter takes integer measured text dimensions.
    let label_size = (!edge.label.is_empty()).then(|| Point {
        x: edge.label_width.ceil(),
        y: edge.label_height.ceil(),
    });
    let mut origin = route_origin(
        Point {
            x: node.x,
            y: node.y,
        },
        &points,
    );
    if let Some(size) = label_size {
        let top_left = label_top_left(&points, size, LAYOUT_STROKE_WIDTH, true);
        origin.x = origin.x.min(top_left.x.floor());
        origin.y = origin.y.min(top_left.y.floor());
    }
    node.x -= origin.x;
    node.y -= origin.y;
    finish_route(edge, points, origin, label_size)
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
) -> Vec<EdgeGeometry> {
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
    let mut origin = Point {
        x: node.x,
        y: node.y,
    };
    let mut routes = Vec::with_capacity(edges.len());
    for edge in edges {
        let slot = order[arrow_class(edge)];
        let points = corner_route(node, corners[slot], gaps[slot]);
        origin = route_origin(origin, &points);
        gaps[slot] += SHARED_PORT_LOOP_GAP;
        routes.push(points);
    }

    node.x -= origin.x;
    node.y -= origin.y;
    edges
        .iter()
        .zip(routes)
        .map(|(edge, points)| finish_route(edge, points, origin, None))
        .collect()
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

fn route_origin(mut origin: Point, points: &[Point]) -> Point {
    for point in points {
        origin.x = origin.x.min(point.x.floor());
        origin.y = origin.y.min(point.y.floor());
    }
    origin
}

fn finish_route(
    edge: &Edge,
    mut points: [Point; 5],
    origin: Point,
    label_size: Option<Point>,
) -> EdgeGeometry {
    for point in &mut points {
        point.x -= origin.x;
        point.y -= origin.y;
    }

    // The adapter normalizes source-only edges before layout; it restores
    // the author's order and mirrors OutsideTopCenter to OutsideBottomCenter.
    let reversed = edge.source_arrow && !edge.target_arrow;
    if reversed {
        points.reverse();
    }
    let label = label_size.map(|size| {
        let top_left = label_top_left(&points, size, OUTPUT_STROKE_WIDTH, !reversed);
        Point {
            x: top_left.x + size.x / 2.0,
            y: top_left.y + size.y / 2.0,
        }
    });

    EdgeGeometry {
        id: edge.id.clone(),
        points: points.to_vec(),
        label,
    }
}

fn label_top_left(points: &[Point; 5], size: Point, stroke_width: f64, top: bool) -> Point {
    let lengths: [f64; 4] = std::array::from_fn(|i| {
        let dx = points[i + 1].x - points[i].x;
        let dy = points[i + 1].y - points[i].y;
        (dx * dx + dy * dy).sqrt()
    });
    let mut remaining = 0.5 * lengths.iter().sum::<f64>();
    for (i, length) in lengths.into_iter().enumerate() {
        if remaining <= length {
            let from = points[i];
            let to = points[i + 1];
            let fraction = remaining / length;
            let base_x = from.x * (1.0 - fraction) + to.x * fraction;
            let base_y = from.y * (1.0 - fraction) + to.y * fraction;
            let mut normal_x = (from.y - to.y) / length;
            let mut normal_y = (to.x - from.x) / length;
            if top {
                normal_x *= -1.0;
                normal_y *= -1.0;
            }
            let offset_x = stroke_width / 2.0 + LABEL_PADDING + size.x / 2.0;
            let offset_y = stroke_width / 2.0 + LABEL_PADDING + size.y / 2.0;
            return Point {
                x: round_label_coordinate(base_x + normal_x * offset_x - size.x / 2.0),
                y: round_label_coordinate(base_y + normal_y * offset_y - size.y / 2.0),
            };
        }
        remaining -= length;
    }
    unreachable!("a finite positive loop has a midpoint")
}

fn round_label_coordinate(value: f64) -> f64 {
    let rounded = (f64::from((value * 10_000.0) as f32) / 10_000.0).round();
    if rounded == 0.0 { 0.0 } else { rounded }
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
            let edge = isolated_rectangle_loop(&mut case.node, &case.edge);
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
            let edges = isolated_unlabeled_loops(&mut case.node, &case.edges);
            assert_eq!(case.node, case.expected_node, "{}", case.name);
            assert_eq!(edges, case.expected_edges, "{}", case.name);
        }
    }
}
