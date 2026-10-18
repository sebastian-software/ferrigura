//! `separateSharedRenderedTerminalLanes` (`materializedGeometry.ts:45-355`): where two rendered terminal rails
//! of the same node face coincide, one of them is moved to a nearby track.

use std::cmp::Ordering;

use super::common::*;
use crate::math::{js_max, js_min};
use crate::model::{LayoutData, Node, Point, Rect};

const MIN_FACE_CLEARANCE: f64 = 16.0;
const TRACK_SHIFT: f64 = 7.0;
const MAX_ITERATIONS: usize = 8;
const SHIFTS: [f64; 6] = [
    -TRACK_SHIFT,
    TRACK_SHIFT,
    -2.0 * TRACK_SHIFT,
    2.0 * TRACK_SHIFT,
    -3.0 * TRACK_SHIFT,
    3.0 * TRACK_SHIFT,
];

/// `separateSharedRenderedTerminalLanes(edges, nodeByIdMap)`.
pub fn separate_shared_rendered_terminal_lanes(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        for _ in 0..MAX_ITERATIONS {
            if !separate_once(view, points) {
                return;
            }
        }
    });
}

#[derive(Clone, Copy, PartialEq)]
enum Orientation {
    Horizontal,
    Vertical,
}

/// A terminal rail: the stub of an edge at one of its ends, where the edge meets a node face.
#[derive(Clone, Copy)]
struct TerminalLane {
    /// The index of the edge.
    edge: usize,
    /// The index of the node, which identifies the node id (`nodeId`).
    node: usize,
    at_start: bool,
    orientation: Orientation,
    coord: f64,
    min: f64,
    max: f64,
    boundary: Point,
    rail_end: Point,
    rect: Rect,
}

/// `rectIntersect(node, point)`: where the line from the centre of the node to `point` leaves the node.
fn rect_intersect(node: &Node, point: Point) -> Point {
    let x = or_zero(node.x);
    let y = or_zero(node.y);
    let dx = point.x - x;
    let dy = point.y - y;
    let mut w = or_zero(node.width) / 2.0;
    let mut h = or_zero(node.height) / 2.0;

    if dy.abs() * w > dx.abs() * h {
        if dy < 0.0 {
            h = -h;
        }
        return Point {
            x: x + if dy == 0.0 { 0.0 } else { h * dx / dy },
            y: y + h,
        };
    }

    if dx < 0.0 {
        w = -w;
    }
    Point {
        x: x + w,
        y: y + if dx == 0.0 { 0.0 } else { w * dy / dx },
    }
}

/// `terminalLaneFor(edge, atStart)`, for the points `source` of the edge.
fn terminal_lane_for(
    view: &View,
    edge: usize,
    source: &[Point],
    at_start: bool,
) -> Option<TerminalLane> {
    let points = dedupe_consecutive_points(source, EPS);
    if points.len() < 2 {
        return None;
    }

    let edge_ref = &view.edges[edge];
    let node_id = if at_start {
        edge_ref.start.as_deref()
    } else {
        edge_ref.end.as_deref()
    };
    let node_index = view.node_index(node_id)?;
    let node = &view.nodes[node_index];
    let rect = rect_of_node_bounds(node)?;

    let last = points.len() - 1;
    let endpoint = if at_start { points[0] } else { points[last] };
    let adjacent = if at_start {
        points[1]
    } else {
        points[last - 1]
    };
    let boundary = rect_intersect(node, endpoint);
    let mut rail_end = endpoint;
    if orthogonally_aligned(adjacent, boundary) {
        rail_end = adjacent;
    }

    if same_x(boundary, rail_end, EPS) {
        return Some(TerminalLane {
            edge,
            node: node_index,
            at_start,
            orientation: Orientation::Vertical,
            coord: boundary.x,
            min: js_min(boundary.y, rail_end.y),
            max: js_max(boundary.y, rail_end.y),
            boundary,
            rail_end,
            rect,
        });
    }
    if same_y(boundary, rail_end, EPS) {
        return Some(TerminalLane {
            edge,
            node: node_index,
            at_start,
            orientation: Orientation::Horizontal,
            coord: boundary.y,
            min: js_min(boundary.x, rail_end.x),
            max: js_max(boundary.x, rail_end.x),
            boundary,
            rail_end,
            rect,
        });
    }
    None
}

fn projected_overlap_length(a: &TerminalLane, b: &TerminalLane) -> f64 {
    js_max(0.0, js_min(a.max, b.max) - js_max(a.min, b.min))
}

fn same_terminal_face(a: &TerminalLane, b: &TerminalLane) -> bool {
    if a.node != b.node || a.orientation != b.orientation {
        return false;
    }

    match a.orientation {
        Orientation::Horizontal => {
            let on_horizontal_face = (a.boundary.x - a.rect.left).abs() < 1.0
                || (a.boundary.x - a.rect.right).abs() < 1.0;
            on_horizontal_face && same_x(a.boundary, b.boundary, 1.0)
        }
        Orientation::Vertical => {
            let on_vertical_face = (a.boundary.y - a.rect.top).abs() < 1.0
                || (a.boundary.y - a.rect.bottom).abs() < 1.0;
            on_vertical_face && same_y(a.boundary, b.boundary, 1.0)
        }
    }
}

fn exact_terminal_lane_conflict(a: &TerminalLane, b: &TerminalLane) -> bool {
    if a.node != b.node || a.orientation != b.orientation {
        return false;
    }

    let shared = projected_overlap_length(a, b);
    shared >= MIN_SHARED && (a.coord - b.coord).abs() < 0.5
}

fn near_terminal_lane_conflict(a: &TerminalLane, b: &TerminalLane) -> bool {
    if a.node != b.node
        || a.orientation != b.orientation
        || a.orientation != Orientation::Horizontal
        || a.at_start == b.at_start
    {
        return false;
    }

    let shared = projected_overlap_length(a, b);
    if shared < MIN_SHARED {
        return false;
    }
    let face_span = a.rect.bottom - a.rect.top;
    if shared < face_span || shared > 2.0 * face_span {
        return false;
    }

    // Wybrow-style nudging keeps connector topology fixed while preserving ordering constraints; rendered
    // terminal tracks on the same object face need the same treatment before endpoint duplication pins them.
    same_terminal_face(a, b) && (a.coord - b.coord).abs() < MIN_FACE_CLEARANCE
}

/// `boundaryStaysOnSameFace` of `shiftedCandidate`: the shifted boundary point is still on the same face.
fn boundary_stays_on_same_face(lane: &TerminalLane, shifted: Point) -> bool {
    let boundary = lane.boundary;
    let rect = lane.rect;
    if (boundary.y - rect.top).abs() < 1.0 || (boundary.y - rect.bottom).abs() < 1.0 {
        return same_y(shifted, boundary, EPS)
            && shifted.x >= rect.left + 1.0
            && shifted.x <= rect.right - 1.0;
    }

    if (boundary.x - rect.left).abs() < 1.0 || (boundary.x - rect.right).abs() < 1.0 {
        return same_x(shifted, boundary, EPS)
            && shifted.y >= rect.top + 1.0
            && shifted.y <= rect.bottom - 1.0;
    }

    false
}

/// `shiftedCandidate(lane, shift)`: the points of the edge with its terminal rail moved by `shift`, or
/// `None` when the move would leave the face or bend the next segment.
fn shifted_candidate(points: &Points, lane: &TerminalLane, shift: f64) -> Option<Vec<Point>> {
    let source = dedupe_consecutive_points(points.raw(lane.edge), EPS);
    if source.len() < 2 {
        return None;
    }

    let (shifted_boundary, shifted_rail_end) = match lane.orientation {
        Orientation::Vertical => (
            Point {
                x: lane.boundary.x + shift,
                y: lane.boundary.y,
            },
            Point {
                x: lane.rail_end.x + shift,
                y: lane.rail_end.y,
            },
        ),
        Orientation::Horizontal => (
            Point {
                x: lane.boundary.x,
                y: lane.boundary.y + shift,
            },
            Point {
                x: lane.rail_end.x,
                y: lane.rail_end.y + shift,
            },
        ),
    };

    if !boundary_stays_on_same_face(lane, shifted_boundary) {
        return None;
    }

    let len = source.len();
    if lane.at_start {
        let rail_end_is_adjacent = len > 1 && same_point(source[1], lane.rail_end, EPS);
        let rest = &source[if rail_end_is_adjacent { 2 } else { 1 }..];
        if let Some(next) = rest.first()
            && !orthogonally_aligned(*next, shifted_rail_end)
        {
            return None;
        }
        let mut out = vec![shifted_boundary, shifted_rail_end];
        out.extend_from_slice(rest);
        Some(out)
    } else {
        let rail_end_is_adjacent = len > 1 && same_point(source[len - 2], lane.rail_end, EPS);
        let before = &source[..if rail_end_is_adjacent {
            len - 2
        } else {
            len - 1
        }];
        if let Some(previous) = before.last()
            && !orthogonally_aligned(*previous, shifted_rail_end)
        {
            return None;
        }
        let mut out = before.to_vec();
        out.push(shifted_rail_end);
        out.push(shifted_boundary);
        Some(out)
    }
}

/// `laneIsStraightCollinearConnector(lane)`: the edge is a single straight segment between two nodes that
/// are aligned on the other axis.
fn lane_is_straight_collinear_connector(view: &View, points: &Points, lane: &TerminalLane) -> bool {
    let edge = &view.edges[lane.edge];
    let pts = dedupe_consecutive_points(points.raw(lane.edge), EPS);
    if pts.len() != 2 {
        return false;
    }
    let (Some(start), Some(end)) = (
        view.node(edge.start.as_deref()),
        view.node(edge.end.as_deref()),
    ) else {
        return false;
    };

    let start_x = or_zero(start.x);
    let start_y = or_zero(start.y);
    let end_x = or_zero(end.x);
    let end_y = or_zero(end.y);
    let (a, b) = (pts[0], pts[1]);

    (same_y(a, b, EPS) && (start_y - end_y).abs() < 1.0 && (start_x - end_x).abs() > 1.0)
        || (same_x(a, b, EPS) && (start_x - end_x).abs() < 1.0 && (start_y - end_y).abs() > 1.0)
}

/// The order of two lanes in the pair candidates: the lane whose edge is not a straight connector first,
/// then the lane that is not at the start.
fn candidate_order(view: &View, points: &Points, a: &TerminalLane, b: &TerminalLane) -> Ordering {
    let a_preserves_straight = lane_is_straight_collinear_connector(view, points, a);
    let b_preserves_straight = lane_is_straight_collinear_connector(view, points, b);
    if a_preserves_straight != b_preserves_straight {
        return order_by_difference(number(a_preserves_straight) - number(b_preserves_straight));
    }
    order_by_difference(number(!b.at_start) - number(!a.at_start))
}

/// The terminal lanes of the visible edges, the start lane and then the end lane of each edge.
fn lanes_of(view: &View, points: &Points) -> Vec<TerminalLane> {
    let mut lanes = Vec::new();
    for edge in view.visible() {
        lanes.extend(terminal_lane_for(view, edge, points.raw(edge), true));
        lanes.extend(terminal_lane_for(view, edge, points.raw(edge), false));
    }
    lanes
}

/// One iteration of the loop of the reference. Returns `false` when no pair was fixed, which ends the loop.
fn separate_once(view: &View, points: &mut Points) -> bool {
    let lanes = lanes_of(view, points);

    for i in 0..lanes.len() {
        for j in (i + 1)..lanes.len() {
            let first = lanes[i];
            let second = lanes[j];
            if first.edge == second.edge
                || !(exact_terminal_lane_conflict(&first, &second)
                    || near_terminal_lane_conflict(&first, &second))
            {
                continue;
            }

            let fixing_near_conflict = !exact_terminal_lane_conflict(&first, &second);
            let mut candidates = [first, second];
            candidates.sort_by(|a, b| candidate_order(view, points, a, b));
            for lane in candidates {
                for shift in SHIFTS {
                    let Some(candidate) = shifted_candidate(points, &lane, shift) else {
                        continue;
                    };
                    let Some(next_lane) =
                        terminal_lane_for(view, lane.edge, &candidate, lane.at_start)
                    else {
                        continue;
                    };
                    let conflicts = lanes.iter().any(|other| {
                        other.edge != lane.edge
                            && (exact_terminal_lane_conflict(&next_lane, other)
                                || (fixing_near_conflict
                                    && near_terminal_lane_conflict(&next_lane, other)))
                    });
                    if conflicts {
                        continue;
                    }

                    points.set(lane.edge, candidate);
                    return true;
                }
            }
        }
    }
    false
}
