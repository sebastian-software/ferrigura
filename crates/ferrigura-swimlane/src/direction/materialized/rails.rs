//! `liftObstacleHuggingSameSideRails` (`materializedGeometry.ts:511-714`): a rail that runs just inside the
//! border of a taller node in between is moved outside that node, when the move does not add crossings.

use super::common::*;
use crate::model::{LayoutData, Point};

const BUFFER: f64 = 2.0;
const CLEARANCE: f64 = 20.0;
const MAX_ITERATIONS: usize = 8;

/// `liftObstacleHuggingSameSideRails(edges, nodeByIdMap)`.
pub fn lift_obstacle_hugging_same_side_rails(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        let (real, labels) = node_rect_entries(view);
        for _ in 0..MAX_ITERATIONS {
            if !lift_once(view, points, &real, &labels) {
                return;
            }
        }
    });
}

/// One iteration: the first visible edge whose middle rail can be lifted gets the lifted path.
fn lift_once(view: &View, points: &mut Points, real: &[RectEntry], labels: &[RectEntry]) -> bool {
    let visible = view.visible();
    let current_crossings = strict_crossing_count(&visible, points, &Replacements::new());

    for &edge in &visible {
        let current = points_of(points, &Replacements::new(), edge);
        let Some(rail) = middle_rail(&current) else {
            continue;
        };
        let blockers = blocking_rects(view, edge, &rail, real);
        if blockers.is_empty() {
            continue;
        }

        let coords = if rail.horizontal {
            [
                js_min_of(blockers.iter().map(|entry| entry.rect.top)) - CLEARANCE,
                js_max_of(blockers.iter().map(|entry| entry.rect.bottom)) + CLEARANCE,
            ]
        } else {
            [
                js_min_of(blockers.iter().map(|entry| entry.rect.left)) - CLEARANCE,
                js_max_of(blockers.iter().map(|entry| entry.rect.right)) + CLEARANCE,
            ]
        };

        for coord in coords {
            let Some(candidate) = candidate_by_moving_rail(&current, &rail, coord) else {
                continue;
            };
            if !candidate_is_safe(
                view,
                points,
                real,
                labels,
                &visible,
                edge,
                &candidate,
                current_crossings,
            ) {
                continue;
            }
            points.set(edge, candidate);
            return true;
        }
    }
    false
}

/// `middleRail(points)`: the middle segment of a path of three segments that alternate between horizontal
/// and vertical.
fn middle_rail(points: &[Point]) -> Option<OrthogonalSegment> {
    let segments = segments_for(points);
    if segments.len() != 3 {
        return None;
    }
    let middle = segments[1];
    if segments[0].horizontal == middle.horizontal || segments[2].horizontal == middle.horizontal {
        return None;
    }
    Some(middle)
}

/// `blockingRectsFor(edge, rail)`: the real nodes that the rail runs along, which the edge does not end at.
fn blocking_rects<'r>(
    view: &View,
    edge: usize,
    rail: &OrthogonalSegment,
    real: &'r [RectEntry],
) -> Vec<&'r RectEntry> {
    let endpoints = endpoint_ids(&view.edges[edge]);
    real.iter()
        .filter(|entry| {
            if endpoints.contains(&entry.id.as_str()) {
                return false;
            }
            let rect = entry.rect;
            if rail.horizontal {
                let x_overlap = overlap_length(rail.a.x, rail.b.x, rect.left, rect.right);
                return x_overlap >= MIN_SHARED
                    && rail.a.y >= rect.top - BUFFER
                    && rail.a.y <= rect.bottom + BUFFER;
            }
            let y_overlap = overlap_length(rail.a.y, rail.b.y, rect.top, rect.bottom);
            y_overlap >= MIN_SHARED
                && rail.a.x >= rect.left - BUFFER
                && rail.a.x <= rect.right + BUFFER
        })
        .collect()
}

/// `candidateByMovingRail(points, rail, coord)`: the path with the rail at `coord`, simplified, when it is
/// still an orthogonal path.
fn candidate_by_moving_rail(
    points: &[Point],
    rail: &OrthogonalSegment,
    coord: f64,
) -> Option<Vec<Point>> {
    let mut candidate = points.to_vec();
    if rail.horizontal {
        candidate[rail.index].y = coord;
        candidate[rail.index + 1].y = coord;
    } else if rail.vertical {
        candidate[rail.index].x = coord;
        candidate[rail.index + 1].x = coord;
    } else {
        return None;
    }
    let flat = simplified(&candidate);
    if is_orthogonal_path(&flat) {
        Some(flat)
    } else {
        None
    }
}

/// `candidateIsSafe(edge, candidate, currentCrossings)`.
#[allow(clippy::too_many_arguments)]
fn candidate_is_safe(
    view: &View,
    points: &Points,
    real: &[RectEntry],
    labels: &[RectEntry],
    visible: &[usize],
    edge: usize,
    candidate: &[Point],
    current_crossings: f64,
) -> bool {
    let endpoints = endpoint_ids(&view.edges[edge]);
    let candidate_segments = segments_for(candidate);
    if candidate_segments.len() + 1 != candidate.len() {
        return false;
    }

    for segment in &candidate_segments {
        if segment_hits_any_rect(segment.a, segment.b, real, &endpoints, -BUFFER) {
            return false;
        }
        if segment_hits_any_rect(segment.a, segment.b, labels, &[], -BUFFER) {
            return false;
        }
    }

    for &other in visible {
        if other == edge {
            continue;
        }
        let other_segments = segments_for(&points_of(points, &Replacements::new(), other));
        for candidate_segment in &candidate_segments {
            for other_segment in &other_segments {
                if same_axis_segment_overlap_length(candidate_segment, other_segment, 0.5)
                    >= MIN_SHARED
                {
                    return false;
                }
            }
        }
    }

    let mut replacements = Replacements::new();
    replacements.insert(edge, candidate.to_vec());
    strict_crossing_count(visible, points, &replacements) <= current_crossings
}
