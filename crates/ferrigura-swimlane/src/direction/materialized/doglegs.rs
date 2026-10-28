//! `collapseRedundantRectangularDoglegs` (`materializedGeometry.ts:357-509`): a rectangular detour of an edge
//! that the earlier passes made unnecessary is replaced by its direct route, when the route stays clear.

use super::common::*;
use crate::model::{LayoutData, Point};

const BUFFER: f64 = 2.0;
const MAX_ITERATIONS: usize = 8;

/// `collapseRedundantRectangularDoglegs(edges, nodeByIdMap)`.
pub fn collapse_redundant_rectangular_doglegs(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        let (real, labels) = node_rect_entries(view);
        for _ in 0..MAX_ITERATIONS {
            if !collapse_once(view, points, &real, &labels) {
                return;
            }
        }
    });
}

/// One iteration: the first edge (in order) with a dogleg that can be removed gets the shorter path.
fn collapse_once(
    view: &View,
    points: &mut Points,
    real: &[RectEntry],
    labels: &[RectEntry],
) -> bool {
    for edge in view.visible() {
        let current = dedupe_consecutive_points(points.raw(edge), EPS);
        if current.len() < 5 {
            continue;
        }
        for i in 0..=(current.len() - 5) {
            let Some(candidate) = without_dogleg(&current, i) else {
                continue;
            };
            if !candidate_is_safe(view, points, real, labels, edge, &candidate) {
                continue;
            }
            points.set(edge, candidate);
            return true;
        }
    }
    false
}

/// `withoutDogleg(points, i)`: the path with the dogleg that starts at `i` removed, or `None` when there is
/// no dogleg there.
fn without_dogleg(points: &[Point], i: usize) -> Option<Vec<Point>> {
    if i + 4 >= points.len() {
        return None;
    }
    let p0 = points[i];
    let p1 = points[i + 1];
    let p2 = points[i + 2];
    let p3 = points[i + 3];
    let p4 = points[i + 4];

    let terminal_vertical_dogleg = is_horizontal_segment(p0, p1, EPS)
        && is_vertical_segment(p1, p2, EPS)
        && is_horizontal_segment(p2, p3, EPS)
        && is_vertical_segment(p3, p4, EPS)
        && same_x(p0, p3, EPS)
        && same_x(p0, p4, EPS)
        && same_x(p1, p2, EPS)
        && (p1.x - p0.x) * (p3.x - p2.x) < 0.0;

    let terminal_horizontal_dogleg = is_vertical_segment(p0, p1, EPS)
        && is_horizontal_segment(p1, p2, EPS)
        && is_vertical_segment(p2, p3, EPS)
        && is_horizontal_segment(p3, p4, EPS)
        && same_y(p0, p3, EPS)
        && same_y(p0, p4, EPS)
        && same_y(p1, p2, EPS)
        && (p1.y - p0.y) * (p3.y - p2.y) < 0.0;

    if terminal_vertical_dogleg || terminal_horizontal_dogleg {
        let mut joined = points[..=i].to_vec();
        joined.push(p4);
        joined.extend_from_slice(&points[i + 5..]);
        return Some(dedupe_consecutive_points(&joined, EPS));
    }

    if i + 5 >= points.len() {
        return None;
    }
    let p5 = points[i + 5];

    let vertical_dogleg = is_vertical_segment(p0, p1, EPS)
        && is_horizontal_segment(p1, p2, EPS)
        && is_vertical_segment(p2, p3, EPS)
        && is_horizontal_segment(p3, p4, EPS)
        && is_vertical_segment(p4, p5, EPS)
        && same_x(p0, p4, EPS)
        && same_x(p0, p5, EPS)
        && same_x(p2, p3, EPS)
        && (p2.x - p1.x) * (p4.x - p3.x) < 0.0;

    let horizontal_dogleg = is_horizontal_segment(p0, p1, EPS)
        && is_vertical_segment(p1, p2, EPS)
        && is_horizontal_segment(p2, p3, EPS)
        && is_vertical_segment(p3, p4, EPS)
        && is_horizontal_segment(p4, p5, EPS)
        && same_y(p0, p4, EPS)
        && same_y(p0, p5, EPS)
        && same_y(p2, p3, EPS)
        && (p2.y - p1.y) * (p4.y - p3.y) < 0.0;

    if !vertical_dogleg && !horizontal_dogleg {
        return None;
    }

    let mut joined = points[..=i].to_vec();
    joined.push(p5);
    joined.extend_from_slice(&points[i + 6..]);
    Some(dedupe_consecutive_points(&joined, EPS))
}

/// `candidateIsSafe(edge, candidate)`: the candidate keeps clear of the nodes and the labels, and it shares
/// no track and crosses no other visible edge's track.
fn candidate_is_safe(
    view: &View,
    points: &Points,
    real: &[RectEntry],
    labels: &[RectEntry],
    edge: usize,
    candidate: &[Point],
) -> bool {
    let candidate_segments = segments_for(candidate);
    if candidate_segments.len() + 1 != candidate.len() {
        return false;
    }

    let endpoints = endpoint_ids(&view.edges[edge]);
    for segment in &candidate_segments {
        if segment_hits_any_rect(segment.a, segment.b, real, &endpoints, -BUFFER) {
            return false;
        }
        if segment_hits_any_rect(segment.a, segment.b, labels, &[], -BUFFER) {
            return false;
        }
    }

    for other in view.visible() {
        if other == edge {
            continue;
        }
        let other_points = points.raw(other);
        if other_points.len() < 2 {
            continue;
        }
        let other_segments = segments_for(&dedupe_consecutive_points(other_points, EPS));
        for candidate_segment in &candidate_segments {
            for other_segment in &other_segments {
                if same_axis_segment_overlap_length(candidate_segment, other_segment, 0.5)
                    >= MIN_SHARED
                {
                    return false;
                }
                if orthogonal_segments_strictly_cross(
                    candidate_segment.a,
                    candidate_segment.b,
                    other_segment.a,
                    other_segment.b,
                    EPS,
                ) {
                    return false;
                }
            }
        }
    }

    true
}
