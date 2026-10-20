//! The endpoint passes of `postProcessSwimlaneLayout`: `direction/endpointClip.ts` of Mermaid 12.2.0.
//!
//! [`clip_edge_endpoints_to_node_boundaries`] (`endpointClip.ts:85-103`) cuts each polyline where it enters the
//! rectangle of its ends, and clears the corner next to a straight end. [`prepare_edge_endpoints_for_renderer`]
//! (`endpointClip.ts:447-469`) snaps the ends onto the node borders, collapses the stubs that run along a
//! border, and duplicates each end, as the renderer expects.
//!
//! The TypeScript passes a `nodeByIdMap` and reads the rectangle of each end when it runs. Neither pass changes a
//! node, so the port reads the rectangles once, in [`node_rects`].
//!
//! Where the TypeScript returns its input object unchanged and the caller tests the identity (`!==`), the port
//! returns `None` or `Option` for the same case. A new object with the same values is a new value in the port,
//! and the output is the same.

use indexmap::IndexMap;

use crate::direction::geometry::{
    EPS, RectSide, dedupe_consecutive_points, orthogonalize_polyline, point_inside_rect,
    rect_of_node_bounds, same_point, same_x, same_y, simplify_polyline, truthy_text,
};
use crate::math::{js_max, js_min};
use crate::model::{Edge, LayoutData, Node, Point, Rect};

/// `INSIDE_EPS` (`endpointClip.ts:14`): the buffer of the inside test of a point.
const INSIDE_EPS: f64 = 0.5;
/// `CORNER_CLEARANCE` (`endpointClip.ts:15`): the distance an endpoint keeps from the corner of its node.
const CORNER_CLEARANCE: f64 = 4.0;

/// The rectangle of each node by id, as `nodeByIdMap` gives it: the last node with an id is the one that counts,
/// and a node without a positive size has no rectangle.
fn node_rects(nodes: &[Node]) -> IndexMap<String, Option<Rect>> {
    let mut rects = IndexMap::new();
    for node in nodes {
        rects.insert(node.id.clone(), rect_of_node_bounds(node));
    }
    rects
}

/// The points of an edge that the passes work on, and the rectangles of its ends (`endpointContextFor`,
/// `endpointClip.ts:28-41`).
struct EndpointContext {
    points: Vec<Point>,
    src_rect: Option<Rect>,
    dst_rect: Option<Rect>,
}

/// `endpointContextFor(edge, nodeByIdMap, minPoints)`: the context of an edge that is not layout-only and has at
/// least `min_points` points, or `None`.
fn endpoint_context(
    edge: &Edge,
    rects: &IndexMap<String, Option<Rect>>,
    min_points: usize,
) -> Option<EndpointContext> {
    if edge.is_layout_only == Some(true) {
        return None;
    }
    let points = edge
        .points
        .as_ref()
        .filter(|points| points.len() >= min_points)?;
    let rect_of = |id: Option<&str>| {
        truthy_text(id)
            .and_then(|id| rects.get(id))
            .copied()
            .flatten()
    };
    Some(EndpointContext {
        points: points.clone(),
        src_rect: rect_of(edge.start.as_deref()),
        dst_rect: rect_of(edge.end.as_deref()),
    })
}

/// `segmentEnterPoint(outside, inside, r)` (`endpointClip.ts:45-58`): where the segment from `outside` to
/// `inside` enters the rectangle.
fn segment_enter_point(outside: Point, inside: Point, r: Rect) -> Point {
    if same_y(outside, inside, EPS) {
        let x = if outside.x < r.left { r.left } else { r.right };
        return Point { x, y: outside.y };
    }
    if same_x(outside, inside, EPS) {
        let y = if outside.y < r.top { r.top } else { r.bottom };
        return Point { x: outside.x, y };
    }
    Point {
        x: js_min(r.right, js_max(r.left, outside.x)),
        y: js_min(r.bottom, js_max(r.top, outside.y)),
    }
}

/// `clipEndpoint(points, rect, atStart)` (`endpointClip.ts:60-83`): the points from the first one outside the
/// rectangle, with the point where the segment enters the rectangle. The points are returned unchanged when
/// the walk leaves the list.
fn clip_endpoint(points: &[Point], rect: Rect, at_start: bool) -> Vec<Point> {
    let len = points.len() as i64;
    let step: i64 = if at_start { 1 } else { -1 };
    let mut outside = if at_start { 0 } else { len - 1 };
    while (0..len).contains(&outside)
        && point_inside_rect(points[outside as usize], rect, INSIDE_EPS)
    {
        outside += step;
    }
    if !(0..len).contains(&outside) {
        return points.to_vec();
    }

    let inside = outside - step;
    if !(0..len).contains(&inside) {
        return points.to_vec();
    }

    let outside = outside as usize;
    let entry = segment_enter_point(points[outside], points[inside as usize], rect);
    if at_start {
        let mut clipped = vec![entry];
        clipped.extend_from_slice(&points[outside..]);
        clipped
    } else {
        let mut clipped = points[..=outside].to_vec();
        clipped.push(entry);
        clipped
    }
}

/// `clipEdgeEndpointsToNodeBoundaries(edges, nodeByIdMap)` (`endpointClip.ts:85-103`): each edge with at least
/// two points is cut at the boundary of its ends, orthogonalized and simplified, and its straight end corner is
/// cleared.
pub fn clip_edge_endpoints_to_node_boundaries(layout: &mut LayoutData) {
    let rects = node_rects(&layout.nodes);
    for edge in &mut layout.edges {
        let Some(context) = endpoint_context(edge, &rects, 2) else {
            continue;
        };

        let mut next = context.points;
        if let Some(rect) = context.src_rect {
            next = clip_endpoint(&next, rect, true);
        }
        if let Some(rect) = context.dst_rect {
            next = clip_endpoint(&next, rect, false);
        }
        next = simplify_polyline(&orthogonalize_polyline(&next));
        let next =
            clear_straight_endpoint_corner_connections(&next, context.src_rect, context.dst_rect)
                .unwrap_or(next);
        edge.points = Some(simplify_polyline(&orthogonalize_polyline(&next)));
    }
}

/// `snapEndpointToBoundary(inner, endpoint, r, useApproachSide)` (`endpointClip.ts:105-142`): the endpoint moved
/// onto the nearest side of the rectangle, on the line from `inner`. `None` when the endpoint stays where it is.
fn snap_endpoint_to_boundary(
    inner: Point,
    endpoint: Point,
    r: Rect,
    use_approach_side: bool,
) -> Option<Point> {
    if same_y(inner, endpoint, EPS) {
        if endpoint.y < r.top - EPS || endpoint.y > r.bottom + EPS {
            return None;
        }
        if use_approach_side {
            if inner.x < r.left - EPS {
                return Some(Point {
                    x: r.left,
                    y: inner.y,
                });
            }
            if inner.x > r.right + EPS {
                return Some(Point {
                    x: r.right,
                    y: inner.y,
                });
            }
        }
        let to_left = (endpoint.x - r.left).abs() <= (endpoint.x - r.right).abs();
        return Some(Point {
            x: if to_left { r.left } else { r.right },
            y: inner.y,
        });
    }
    if same_x(inner, endpoint, EPS) {
        if endpoint.x < r.left - EPS || endpoint.x > r.right + EPS {
            return None;
        }
        if use_approach_side {
            if inner.y < r.top - EPS {
                return Some(Point {
                    x: inner.x,
                    y: r.top,
                });
            }
            if inner.y > r.bottom + EPS {
                return Some(Point {
                    x: inner.x,
                    y: r.bottom,
                });
            }
        }
        let to_top = (endpoint.y - r.top).abs() <= (endpoint.y - r.bottom).abs();
        return Some(Point {
            x: inner.x,
            y: if to_top { r.top } else { r.bottom },
        });
    }
    None
}

/// `firstDistinctAdjacent(points, endpointIndex, step)` (`endpointClip.ts:144-157`): the first point after the
/// endpoint that is not the same point. When all the points in that direction are the same, the next point. `None`
/// when there is no next point.
fn first_distinct_adjacent(points: &[Point], endpoint_index: i64, step: i64) -> Option<Point> {
    let len = points.len() as i64;
    let endpoint = usize::try_from(endpoint_index)
        .ok()
        .and_then(|index| points.get(index))
        .copied()?;
    let mut index = endpoint_index + step;
    while (0..len).contains(&index) {
        let candidate = points[index as usize];
        if !same_point(candidate, endpoint, EPS) {
            return Some(candidate);
        }
        index += step;
    }
    let fallback = endpoint_index + step;
    if (0..len).contains(&fallback) {
        Some(points[fallback as usize])
    } else {
        None
    }
}

/// `cornerClearanceRange(min, max)` (`endpointClip.ts:159-163`): the span of an endpoint along a side, kept
/// `CORNER_CLEARANCE` from each corner. A side too short for that gives its middle.
fn corner_clearance_range(min: f64, max: f64) -> (f64, f64) {
    let lo = min + CORNER_CLEARANCE;
    let hi = max - CORNER_CLEARANCE;
    if lo <= hi {
        (lo, hi)
    } else {
        ((min + max) / 2.0, (min + max) / 2.0)
    }
}

/// `clampToCornerClearance(value, min, max)` (`endpointClip.ts:165-168`).
fn clamp_to_corner_clearance(value: f64, min: f64, max: f64) -> f64 {
    let (lo, hi) = corner_clearance_range(min, max);
    js_min(hi, js_max(lo, value))
}

/// `intersectRanges(ranges)` (`endpointClip.ts:170-179`): the span that all ranges share, or `None`.
fn intersect_ranges(ranges: &[(f64, f64)]) -> Option<(f64, f64)> {
    let lo = ranges
        .iter()
        .fold(f64::NEG_INFINITY, |acc, range| js_max(acc, range.0));
    let hi = ranges
        .iter()
        .fold(f64::INFINITY, |acc, range| js_min(acc, range.1));
    if lo > hi { None } else { Some((lo, hi)) }
}

/// `clearanceRangeForSide(r, side)` (`endpointClip.ts:181-185`).
fn clearance_range_for_side(r: Rect, side: RectSide) -> (f64, f64) {
    if is_horizontal_side(side) {
        corner_clearance_range(r.top, r.bottom)
    } else {
        corner_clearance_range(r.left, r.right)
    }
}

/// `terminalSideForSegment(endpoint, adjacent, r)` (`endpointClip.ts:187-211`): the side of the rectangle that
/// the endpoint sits on, when the segment to `adjacent` runs along that side's normal.
fn terminal_side_for_segment(endpoint: Point, adjacent: Point, r: Rect) -> Option<RectSide> {
    let y_within = (r.top - EPS..=r.bottom + EPS).contains(&endpoint.y);
    let x_within = (r.left - EPS..=r.right + EPS).contains(&endpoint.x);
    if same_y(endpoint, adjacent, EPS) && y_within {
        if (endpoint.x - r.left).abs() < EPS {
            return Some(RectSide::Left);
        }
        if (endpoint.x - r.right).abs() < EPS {
            return Some(RectSide::Right);
        }
    }
    if same_x(endpoint, adjacent, EPS) && x_within {
        if (endpoint.y - r.top).abs() < EPS {
            return Some(RectSide::Top);
        }
        if (endpoint.y - r.bottom).abs() < EPS {
            return Some(RectSide::Bottom);
        }
    }
    None
}

/// `isHorizontalSide(side)`: the left and right sides, whose normal is horizontal.
fn is_horizontal_side(side: RectSide) -> bool {
    matches!(side, RectSide::Left | RectSide::Right)
}

/// `straightClearanceRange(start, end, srcRect, dstRect, horizontal)` (`endpointClip.ts:217-236`): the span an
/// axis-aligned segment can move along, as the ends allow it, or `None`.
fn straight_clearance_range(
    start: Point,
    end: Point,
    src_rect: Option<Rect>,
    dst_rect: Option<Rect>,
    horizontal: bool,
) -> Option<(f64, f64)> {
    let src_side = src_rect.and_then(|r| terminal_side_for_segment(start, end, r));
    let dst_side = dst_rect.and_then(|r| terminal_side_for_segment(end, start, r));

    let mut ranges = Vec::new();
    if let (Some(r), Some(side)) = (src_rect, src_side)
        && is_horizontal_side(side) == horizontal
    {
        ranges.push(clearance_range_for_side(r, side));
    }
    if let (Some(r), Some(side)) = (dst_rect, dst_side)
        && is_horizontal_side(side) == horizontal
    {
        ranges.push(clearance_range_for_side(r, side));
    }

    if ranges.is_empty() {
        None
    } else {
        intersect_ranges(&ranges)
    }
}

/// `clearStraightEndpointCornerAxis(start, end, srcRect, dstRect, horizontal)` (`endpointClip.ts:238-265`): moves
/// a straight segment onto the clearance span, when it moves by at least `EPS`.
fn clear_straight_endpoint_corner_axis(
    start: Point,
    end: Point,
    src_rect: Option<Rect>,
    dst_rect: Option<Rect>,
    horizontal: bool,
) -> Option<Vec<Point>> {
    let (lo, hi) = straight_clearance_range(start, end, src_rect, dst_rect, horizontal)?;
    let current = if horizontal { start.y } else { start.x };
    let next = js_min(hi, js_max(lo, current));
    if (next - current).abs() < EPS {
        return None;
    }
    Some(if horizontal {
        vec![
            Point {
                x: start.x,
                y: next,
            },
            Point { x: end.x, y: next },
        ]
    } else {
        vec![
            Point {
                x: next,
                y: start.y,
            },
            Point { x: next, y: end.y },
        ]
    })
}

/// `clearStraightEndpointCornerConnections(points, srcRect, dstRect)` (`endpointClip.ts:267-286`). `None` when
/// the points are returned unchanged, which the TypeScript tests by identity.
fn clear_straight_endpoint_corner_connections(
    points: &[Point],
    src_rect: Option<Rect>,
    dst_rect: Option<Rect>,
) -> Option<Vec<Point>> {
    if points.len() != 2 {
        return None;
    }
    let (start, end) = (points[0], points[1]);
    if same_y(start, end, EPS) {
        return clear_straight_endpoint_corner_axis(start, end, src_rect, dst_rect, true);
    }
    if same_x(start, end, EPS) {
        return clear_straight_endpoint_corner_axis(start, end, src_rect, dst_rect, false);
    }
    None
}

/// `cornerClearedEndpoint(endpoint, r, side)` (`endpointClip.ts:288-292`): the endpoint kept inside the corner
/// clearance of its side.
fn corner_cleared_endpoint(endpoint: Point, r: Rect, side: RectSide) -> Point {
    if is_horizontal_side(side) {
        Point {
            x: endpoint.x,
            y: clamp_to_corner_clearance(endpoint.y, r.top, r.bottom),
        }
    } else {
        Point {
            x: clamp_to_corner_clearance(endpoint.x, r.left, r.right),
            y: endpoint.y,
        }
    }
}

/// `moveCollinearEndpointRun(points, endpointIndex, step, endpoint, adjusted, horizontalTerminal)`
/// (`endpointClip.ts:294-318`): the points that run on the endpoint's line move with it. The copy is a new list.
fn move_collinear_endpoint_run(
    points: &[Point],
    endpoint_index: i64,
    step: i64,
    endpoint: Point,
    adjusted: Point,
    horizontal_terminal: bool,
) -> Vec<Point> {
    let mut next = points.to_vec();
    let len = points.len() as i64;
    let mut index = endpoint_index;
    while (0..len).contains(&index) {
        let point = points[index as usize];
        if horizontal_terminal {
            if !same_y(point, endpoint, EPS) {
                break;
            }
            next[index as usize].y = adjusted.y;
        } else {
            if !same_x(point, endpoint, EPS) {
                break;
            }
            next[index as usize].x = adjusted.x;
        }
        index += step;
    }
    next
}

/// `clearEndpointCornerConnection(points, r, atStart)` (`endpointClip.ts:320-352`): when the first (or last)
/// segment leaves the rectangle's side at a corner, the endpoint moves inside the corner clearance and the run of
/// points on its line moves with it.
fn clear_endpoint_corner_connection(points: &[Point], r: Rect, at_start: bool) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let endpoint_index: i64 = if at_start { 0 } else { points.len() as i64 - 1 };
    let step: i64 = if at_start { 1 } else { -1 };
    let endpoint = points[endpoint_index as usize];
    let Some(adjacent) = first_distinct_adjacent(points, endpoint_index, step) else {
        return points.to_vec();
    };

    let Some(side) = terminal_side_for_segment(endpoint, adjacent, r) else {
        return points.to_vec();
    };

    let horizontal_terminal = is_horizontal_side(side);
    let adjusted = corner_cleared_endpoint(endpoint, r, side);
    if same_point(endpoint, adjusted, EPS) {
        return points.to_vec();
    }

    move_collinear_endpoint_run(
        points,
        endpoint_index,
        step,
        endpoint,
        adjusted,
        horizontal_terminal,
    )
}

/// `borderSideForSegment(a, b, r)` (`endpointClip.ts:354-370`): the side of the rectangle that the whole segment
/// lies on.
fn border_side_for_segment(a: Point, b: Point, r: Rect) -> Option<RectSide> {
    let x_within = js_min(a.x, b.x) >= r.left - EPS && js_max(a.x, b.x) <= r.right + EPS;
    let y_within = js_min(a.y, b.y) >= r.top - EPS && js_max(a.y, b.y) <= r.bottom + EPS;
    if (a.y - r.top).abs() < EPS && (b.y - r.top).abs() < EPS && x_within {
        return Some(RectSide::Top);
    }
    if (a.y - r.bottom).abs() < EPS && (b.y - r.bottom).abs() < EPS && x_within {
        return Some(RectSide::Bottom);
    }
    if (a.x - r.left).abs() < EPS && (b.x - r.left).abs() < EPS && y_within {
        return Some(RectSide::Left);
    }
    if (a.x - r.right).abs() < EPS && (b.x - r.right).abs() < EPS && y_within {
        return Some(RectSide::Right);
    }
    None
}

/// `leavesOutward(side, from, to, r)` (`endpointClip.ts:372-383`): the segment from `from` to `to` runs away from
/// the side, out of the rectangle.
fn leaves_outward(side: RectSide, from: Point, to: Point, r: Rect) -> bool {
    match side {
        RectSide::Top => same_x(from, to, EPS) && to.y < r.top - EPS,
        RectSide::Bottom => same_x(from, to, EPS) && to.y > r.bottom + EPS,
        RectSide::Left => same_y(from, to, EPS) && to.x < r.left - EPS,
        RectSide::Right => same_y(from, to, EPS) && to.x > r.right + EPS,
    }
}

/// `collapseOwnBorderStub(points, r, atStart)` (`endpointClip.ts:385-403`): drops the end point when it sits on
/// the border and the next segment leaves the node outward.
fn collapse_own_border_stub(points: &[Point], r: Rect, at_start: bool) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    if at_start {
        if let Some(side) = border_side_for_segment(points[0], points[1], r)
            && leaves_outward(side, points[1], points[2], r)
        {
            return points[1..].to_vec();
        }
        return points.to_vec();
    }

    let last = points.len() - 1;
    if let Some(side) = border_side_for_segment(points[last - 1], points[last], r)
        && leaves_outward(side, points[last - 1], points[last - 2], r)
    {
        return points[..last].to_vec();
    }
    points.to_vec()
}

/// `snapAndCollapseEndpoints(points, srcRect, dstRect)` (`endpointClip.ts:405-445`): snaps both ends onto their
/// node borders, collapses the border stubs, and clears the corners.
fn snap_and_collapse_endpoints(
    points: &[Point],
    src_rect: Option<Rect>,
    dst_rect: Option<Rect>,
) -> Vec<Point> {
    let mut next = points.to_vec();
    if let Some(r) = src_rect {
        if let Some(adjacent) = first_distinct_adjacent(&next, 0, 1)
            && let Some(snapped) = snap_endpoint_to_boundary(adjacent, next[0], r, false)
        {
            next[0] = snapped;
        }
        next = collapse_own_border_stub(&next, r, true);
    }
    if let Some(r) = dst_rect {
        let last = next.len() as i64 - 1;
        if let Some(adjacent) = first_distinct_adjacent(&next, last, -1)
            && let Some(snapped) = snap_endpoint_to_boundary(adjacent, next[last as usize], r, true)
        {
            next[last as usize] = snapped;
        }
        next = collapse_own_border_stub(&next, r, false);
    }

    // `straightCleared !== next || next.length === 2`: a new list is a change, and so is a two-point list.
    let straight_cleared = clear_straight_endpoint_corner_connections(&next, src_rect, dst_rect);
    if straight_cleared.is_some() || next.len() == 2 {
        return straight_cleared.unwrap_or(next);
    }

    if let Some(r) = src_rect {
        next = clear_endpoint_corner_connection(&next, r, true);
    }
    if let Some(r) = dst_rect {
        next = clear_endpoint_corner_connection(&next, r, false);
    }
    next
}

/// `prepareEdgeEndpointsForRenderer(edges, nodeByIdMap)` (`endpointClip.ts:447-469`): the ends are snapped and
/// collapsed, and a polyline of three or more points gets each end duplicated, so that the renderer draws the
/// stub of each end.
pub fn prepare_edge_endpoints_for_renderer(layout: &mut LayoutData) {
    let rects = node_rects(&layout.nodes);
    for edge in &mut layout.edges {
        let Some(context) = endpoint_context(edge, &rects, 2) else {
            continue;
        };

        let input = dedupe_consecutive_points(&context.points, EPS);
        let new_points = snap_and_collapse_endpoints(&input, context.src_rect, context.dst_rect);
        if new_points.len() < 3 {
            edge.points = Some(new_points);
            continue;
        }
        let last = new_points.len() - 1;
        let mut duplicated = Vec::with_capacity(new_points.len() + 2);
        duplicated.push(new_points[0]);
        duplicated.push(new_points[0]);
        duplicated.extend_from_slice(&new_points[1..last]);
        duplicated.push(new_points[last]);
        duplicated.push(new_points[last]);
        edge.points = Some(duplicated);
    }
}
