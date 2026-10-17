//! The edges of the tidy tree: `calculateEdgePositions` (`layout.ts` 455-628) with its geometry,
//! `intersection` (389-448) and `computeCircleEdgeIntersection` (369-387).
//!
//! Every edge gets four points (a shifted point beside each end, see `shift_source` and
//! `shift_target`). The first pass clips the endpoints against the other node's centre, and its
//! midpoint is the reference's `midX` and `midY`; the second pass clips them against the shifted
//! points, and those are the points the renderer draws. The reference's quirks are kept: the first
//! pass's midpoint is not an endpoint (Q2), `mindmapCircle` is not a round shape (Q3), and an edge
//! between two nodes of one side hooks back (Q4). The guards of `intersection` run in the
//! reference's order, so a zero size or a zero distance gives the same point.

use std::collections::HashMap;

use super::{LayoutEdge, PositionedEdge, PositionedNode, Section, js_min};
use crate::unified::geom::Point;

/// The distance of a shifted point from the side of its node (`intersectionShift`, `layout.ts` 40).
const SHIFT: f64 = 30.0;

/// The shapes whose edges are clipped as circles (`layout.ts` 493-498).
const ROUND_SHAPES: [&str; 3] = ["circle", "cloud", "bang"];

/// The edges, in their order, with the positions of their nodes (`calculateEdgePositions`). An edge
/// with a missing end gets the reference's zero result: one point at the origin, and no node data.
pub(super) fn positions(edges: &[LayoutEdge], nodes: &[PositionedNode]) -> Vec<PositionedEdge> {
    // The last node with an id wins, as the reference's `Map` does (`nodeInfo`, 460-463).
    let mut info: HashMap<&str, &PositionedNode> = HashMap::new();
    for node in nodes {
        info.insert(node.id.as_str(), node);
    }
    edges
        .iter()
        .map(
            |edge| match (info.get(edge.start.as_str()), info.get(edge.end.as_str())) {
                (Some(&source), Some(&target)) => edge_points(edge, source, target),
                _ => missing_end(edge),
            },
        )
        .collect()
}

/// The reference's result for an edge whose node is not in the layout (`layout.ts` 469-487).
fn missing_end(edge: &LayoutEdge) -> PositionedEdge {
    let origin = Point::new(0.0, 0.0);
    PositionedEdge {
        id: edge.id.clone(),
        source: edge.start.clone(),
        target: edge.end.clone(),
        start: origin,
        mid: origin,
        end: origin,
        points: vec![origin],
        source_section: None,
        target_section: None,
        source_width: None,
        source_height: None,
        target_width: None,
        target_height: None,
    }
}

/// One edge between two positioned nodes (`layout.ts` 490-626).
fn edge_points(
    edge: &LayoutEdge,
    source: &PositionedNode,
    target: &PositionedNode,
) -> PositionedEdge {
    let source_centre = Point::new(source.x, source.y);
    let target_centre = Point::new(target.x, target.y);

    // The first pass (500-527). Its midpoint is `midX` and `midY`, which the renderer does not read.
    let mut start = if is_round(source) {
        circle(source, target_centre, source_centre)
    } else {
        intersection(source, source_centre, target_centre)
    };
    let mut end = if is_round(target) {
        circle(target, source_centre, target_centre)
    } else {
        intersection(target, target_centre, source_centre)
    };
    let mid = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);

    let mut points = vec![start];
    shift_source(source, target, &mut points);
    shift_target(source, target, &mut points);
    points.push(end);

    // The second pass (579-607): the endpoints are clipped against the points beside them.
    let second = points.get(1).copied().unwrap_or(target_centre);
    start = if is_round(source) {
        circle(source, second, source_centre)
    } else {
        intersection(source, second, source_centre)
    };
    if let Some(first) = points.first_mut() {
        *first = start;
    }
    let second_last = points
        .len()
        .checked_sub(2)
        .and_then(|index| points.get(index).copied())
        .unwrap_or(source_centre);
    end = if is_round(target) {
        circle(target, second_last, target_centre)
    } else {
        intersection(target, second_last, target_centre)
    };
    if let Some(last) = points.last_mut() {
        *last = end;
    }

    PositionedEdge {
        id: edge.id.clone(),
        source: edge.start.clone(),
        target: edge.end.clone(),
        start,
        mid,
        end,
        points,
        source_section: Some(source.section),
        target_section: Some(target.section),
        source_width: Some(source.width),
        source_height: Some(source.height),
        target_width: Some(target.width),
        target_height: Some(target.height),
    }
}

/// Whether the node's edges are clipped as circles (`ROUND_SHAPES`).
fn is_round(node: &PositionedNode) -> bool {
    ROUND_SHAPES.contains(&node.shape.as_str())
}

/// The point beside the source node, on the side of its section (`layout.ts` 530-552). A source on
/// the root's side gets one on the side that faces its target.
fn shift_source(source: &PositionedNode, target: &PositionedNode, points: &mut Vec<Point>) {
    match source.section {
        Section::Left => points.push(Point::new(source.x - source.width / 2.0 - SHIFT, source.y)),
        Section::Right => points.push(Point::new(source.x + source.width / 2.0 + SHIFT, source.y)),
        Section::Root => match target.section {
            Section::Right => {
                points.push(Point::new(source.x + source.width / 2.0 + SHIFT, source.y))
            }
            Section::Left => {
                points.push(Point::new(source.x - source.width / 2.0 - SHIFT, source.y))
            }
            Section::Root => {}
        },
    }
}

/// The point beside the target node, on the side of its section (`layout.ts` 553-575). A target on
/// the root's side gets one on the side that faces its source. Between two nodes of one side this
/// point lies beyond the source's point, which is the hook of Q4.
fn shift_target(source: &PositionedNode, target: &PositionedNode, points: &mut Vec<Point>) {
    match target.section {
        Section::Left => points.push(Point::new(target.x + target.width / 2.0 + SHIFT, target.y)),
        Section::Right => points.push(Point::new(target.x - target.width / 2.0 - SHIFT, target.y)),
        Section::Root => match source.section {
            Section::Right => {
                points.push(Point::new(target.x + target.width / 2.0 + SHIFT, target.y))
            }
            Section::Left => {
                points.push(Point::new(target.x - target.width / 2.0 - SHIFT, target.y))
            }
            Section::Root => {}
        },
    }
}

/// `computeCircleEdgeIntersection` (`layout.ts` 369-387): where the segment from `line_start` to
/// `line_end` leaves the circle of the node, whose radius is half its smaller side. A segment of
/// length zero gives `line_start`.
fn circle(node: &PositionedNode, line_start: Point, line_end: Point) -> Point {
    let radius = js_min(node.width, node.height) / 2.0;
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length == 0.0 {
        return line_start;
    }
    let nx = dx / length;
    let ny = dy / length;
    Point::new(node.x - nx * radius, node.y - ny * radius)
}

/// `!value` in JavaScript for a size: zero, and NaN, are false.
fn falsy(value: f64) -> bool {
    value == 0.0 || value.is_nan()
}

/// `intersection` (`layout.ts` 389-448): where the segment from `inside` to `outside` leaves the box
/// of the node, with the reference's guards. The top and bottom sides are taken when the segment
/// leaves through them, the left and right sides otherwise. A node without a width or a height gives
/// `outside`.
fn intersection(node: &PositionedNode, outside: Point, inside: Point) -> Point {
    let (x, y) = (node.x, node.y);
    if falsy(node.width) || falsy(node.height) {
        return outside;
    }
    let w = node.width / 2.0;
    let h = node.height / 2.0;
    // Q and R of the reference: the vertical and horizontal distances between the two points.
    let q_total = (outside.y - inside.y).abs();
    let r_total = (outside.x - inside.x).abs();

    if (y - outside.y).abs() * w > (x - outside.x).abs() * h {
        // The intersection is at the top or the bottom of the box.
        let q = if inside.y < outside.y {
            outside.y - h - y
        } else {
            y - h - outside.y
        };
        let r = (r_total * q) / q_total;
        let mut result = Point::new(
            if inside.x < outside.x {
                inside.x + r
            } else {
                inside.x - r_total + r
            },
            if inside.y < outside.y {
                inside.y + q_total - q
            } else {
                inside.y - q_total + q
            },
        );
        if r == 0.0 {
            result = outside;
        }
        if r_total == 0.0 {
            result.x = outside.x;
        }
        if q_total == 0.0 {
            result.y = outside.y;
        }
        result
    } else {
        // The intersection is at the left or the right side of the box.
        let r = if inside.x < outside.x {
            outside.x - w - x
        } else {
            x - w - outside.x
        };
        let q = (q_total * r) / r_total;
        let mut result = Point::new(
            if inside.x < outside.x {
                inside.x + r_total - r
            } else {
                inside.x - r_total + r
            },
            if inside.y < outside.y {
                inside.y + q
            } else {
                inside.y - q
            },
        );
        if r == 0.0 {
            result = outside;
        }
        if r_total == 0.0 {
            result.x = outside.x;
        }
        if q_total == 0.0 {
            result.y = outside.y;
        }
        result
    }
}
