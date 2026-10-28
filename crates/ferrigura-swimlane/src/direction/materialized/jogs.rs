//! `shortcutRedundantOrthogonalJogs` (`materializedGeometry.ts:1587-1833`): a route that goes out and back by a
//! jog of two bends is replaced by a shorter route with fewer bends, when the shorter route keeps clear.

use std::collections::HashSet;

use ferrigura_jsmath::hypot;

use super::common::*;
use crate::model::{LayoutData, Point, Rect};

const BUFFER: f64 = 2.0;
const MAX_ITERATIONS: usize = 8;

/// `shortcutRedundantOrthogonalJogs(edges, nodeByIdMap)`.
pub fn shortcut_redundant_orthogonal_jogs(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        let (real, labels) = node_rect_entries(view);
        for _ in 0..MAX_ITERATIONS {
            let Some((edge, path)) = best_shortcut(view, points, &real, &labels) else {
                return;
            };
            points.set(edge, path);
        }
    });
}

/// `pathLength(points)` of this pass: the sum of the lengths of the orthogonal segments.
fn path_length(points: &[Point]) -> f64 {
    segments_for(points).iter().fold(0.0, |sum, segment| {
        let dx = segment.a.x - segment.b.x;
        let dy = segment.a.y - segment.b.y;
        sum + hypot(dx, dy)
    })
}

/// `shortcutCandidatesAt(points, index)`: the shorter routes that replace the jog that starts at `index`.
fn shortcut_candidates_at(points: &[Point], index: usize) -> Vec<Vec<Point>> {
    if index + 3 >= points.len() {
        return Vec::new();
    }

    let p0 = points[index];
    let p1 = points[index + 1];
    let p2 = points[index + 2];
    let p3 = points[index + 3];
    let is_hvh = is_horizontal_segment(p0, p1, EPS)
        && is_vertical_segment(p1, p2, EPS)
        && is_horizontal_segment(p2, p3, EPS);
    let is_vhv = is_vertical_segment(p0, p1, EPS)
        && is_horizontal_segment(p1, p2, EPS)
        && is_vertical_segment(p2, p3, EPS);
    if !is_hvh && !is_vhv {
        return Vec::new();
    }
    let outer_segments_oppose = if is_hvh {
        js_sign_value(p1.x - p0.x) != js_sign_value(p3.x - p2.x)
    } else {
        js_sign_value(p1.y - p0.y) != js_sign_value(p3.y - p2.y)
    };
    if !outer_segments_oppose {
        return Vec::new();
    }

    let corners: Vec<Point> = if same_x(p0, p3, EPS) || same_y(p0, p3, EPS) {
        Vec::new()
    } else {
        vec![Point { x: p0.x, y: p3.y }, Point { x: p3.x, y: p0.y }]
    };
    let raw_candidates: Vec<Vec<Point>> = if corners.is_empty() {
        vec![joined(points, index, None)]
    } else {
        corners
            .iter()
            .map(|&corner| joined(points, index, Some(corner)))
            .collect()
    };

    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for raw in raw_candidates {
        let candidate = simplified(&raw);
        if !is_orthogonal_path(&candidate) {
            continue;
        }
        if !candidate.iter().any(|&point| same_point(point, p3, EPS)) {
            continue;
        }
        let key = path_key(&candidate);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        result.push(candidate);
    }
    result
}

/// `[...points.slice(0, index + 1), corner?, ...points.slice(index + 3)]`.
fn joined(points: &[Point], index: usize, corner: Option<Point>) -> Vec<Point> {
    let mut path = points[..=index].to_vec();
    path.extend(corner);
    path.extend_from_slice(&points[index + 3..]);
    path
}

/// `segmentRunsAlongRectBorder(segment, rect)`: the segment lies along a side of the rectangle, over at least
/// `MIN_SHARED`.
fn segment_runs_along_rect_border(segment: &OrthogonalSegment, rect: Rect) -> bool {
    if segment.horizontal {
        let y = segment.a.y;
        let on_border = (y - rect.top).abs() < 1.0 || (y - rect.bottom).abs() < 1.0;
        return on_border
            && overlap_length(segment.a.x, segment.b.x, rect.left, rect.right) >= MIN_SHARED;
    }

    if segment.vertical {
        let x = segment.a.x;
        let on_border = (x - rect.left).abs() < 1.0 || (x - rect.right).abs() < 1.0;
        return on_border
            && overlap_length(segment.a.y, segment.b.y, rect.top, rect.bottom) >= MIN_SHARED;
    }

    false
}

/// `endpointRectsFor(edge)`: the rectangles of the endpoints that exist.
fn endpoint_rects_for(view: &View, edge: usize) -> Vec<Rect> {
    endpoint_ids(&view.edges[edge])
        .into_iter()
        .filter_map(|id| view.node(Some(id)).and_then(rect_of_node_bounds))
        .collect()
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
    let endpoint_rects = endpoint_rects_for(view, edge);

    for segment in segments_for(candidate) {
        if segment_hits_any_rect(segment.a, segment.b, real, &endpoints, -BUFFER) {
            return false;
        }
        if segment_hits_any_rect(segment.a, segment.b, labels, &[], -BUFFER) {
            return false;
        }
        if endpoint_rects
            .iter()
            .any(|&rect| segment_runs_along_rect_border(&segment, rect))
        {
            return false;
        }
    }

    for &other in visible {
        if other == edge {
            continue;
        }
        let other_segments = segments_for(&points_of(points, &Replacements::new(), other));
        for candidate_segment in segments_for(candidate) {
            for other_segment in &other_segments {
                if same_axis_segment_overlap_length(&candidate_segment, other_segment, 0.5)
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

/// One iteration: the best shortcut of a jog, as the edge and its new path, or `None` when none improves.
fn best_shortcut(
    view: &View,
    points: &Points,
    real: &[RectEntry],
    labels: &[RectEntry],
) -> Option<(usize, Vec<Point>)> {
    let visible = view.visible();
    let current_crossings = strict_crossing_count(&visible, points, &Replacements::new());

    let mut best: Option<(usize, Vec<Point>)> = None;
    let mut best_crossings = current_crossings;
    let mut best_bends = f64::INFINITY;
    let mut best_length = f64::INFINITY;

    for &edge in &visible {
        let current_points = points_of(points, &Replacements::new(), edge);
        let current_bends = count_orthogonal_bends(&current_points, EPS) as f64;
        let current_length = path_length(&current_points);
        for index in 0..current_points.len().saturating_sub(3) {
            for candidate in shortcut_candidates_at(&current_points, index) {
                let candidate_bends = count_orthogonal_bends(&candidate, EPS) as f64;
                let candidate_length = path_length(&candidate);
                let improves_shape = candidate_bends < current_bends
                    || (candidate_bends == current_bends
                        && candidate_length < current_length - EPS);
                if !improves_shape
                    || !candidate_is_safe(
                        view,
                        points,
                        real,
                        labels,
                        &visible,
                        edge,
                        &candidate,
                        current_crossings,
                    )
                {
                    continue;
                }

                let mut replacements = Replacements::new();
                replacements.insert(edge, candidate.clone());
                let candidate_crossings = strict_crossing_count(&visible, points, &replacements);
                if candidate_crossings > best_crossings
                    || (candidate_crossings == best_crossings
                        && (candidate_bends > best_bends
                            || (candidate_bends == best_bends && candidate_length >= best_length)))
                {
                    continue;
                }

                best = Some((edge, candidate));
                best_crossings = candidate_crossings;
                best_bends = candidate_bends;
                best_length = candidate_length;
            }
        }
    }

    best
}
