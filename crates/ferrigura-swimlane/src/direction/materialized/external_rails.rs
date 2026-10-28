//! `reassignCrossingExternalRailChannels` (`materializedGeometry.ts:1210-1585`): the external rails of the
//! crossing edges (the rails outside both endpoints, on one side) are moved to other channels, as a group, when
//! the new channels remove crossings. A group is a connected set of rails that share a track on one side.

use indexmap::IndexMap;

use super::common::*;
use crate::math::{js_max, js_min};
use crate::model::{LayoutData, Point, Rect};

const BUFFER: f64 = 2.0;
const RAIL_CHANNEL_GAP: f64 = 12.0;
const MAX_ITERATIONS: usize = 4;
const MAX_EXHAUSTIVE_COMPONENT: usize = 6;

/// `reassignCrossingExternalRailChannels(edges, nodeByIdMap)`.
pub fn reassign_crossing_external_rail_channels(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        let (real, labels) = node_rect_entries(view);
        for _ in 0..MAX_ITERATIONS {
            let Some(replacements) = best_reassignment(view, points, &real, &labels) else {
                return;
            };
            for (edge, path) in replacements {
                points.set(edge, path);
            }
        }
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

/// An external rail: a segment of an edge that runs outside the two endpoints, on the `side`.
#[derive(Clone)]
struct ExternalRail {
    edge: usize,
    points: Vec<Point>,
    segment_index: usize,
    axis: Axis,
    side: RectSide,
    coord: f64,
    min: f64,
    max: f64,
}

/// `endpointRectsFor(edge)`: the rectangles of the two endpoints, when both exist.
fn endpoint_rects_for(view: &View, edge: usize) -> Option<(Rect, Rect)> {
    let edge_ref = &view.edges[edge];
    let src = view
        .node(edge_ref.start.as_deref())
        .and_then(rect_of_node_bounds);
    let dst = view
        .node(edge_ref.end.as_deref())
        .and_then(rect_of_node_bounds);
    match (src, dst) {
        (Some(src), Some(dst)) => Some((src, dst)),
        _ => None,
    }
}

/// `externalRailForSegment(edge, points, segment)`.
fn external_rail_for_segment(
    view: &View,
    edge: usize,
    points: &[Point],
    segment: &OrthogonalSegment,
) -> Option<ExternalRail> {
    if segment.index == 0 || segment.index + 1 >= points.len().saturating_sub(1) {
        return None;
    }

    let (src, dst) = endpoint_rects_for(view, edge)?;

    if segment.vertical {
        let coord = segment.a.x;
        let left_bound = js_min(src.left, dst.left);
        let right_bound = js_max(src.right, dst.right);
        let side = if coord < left_bound - EPS {
            RectSide::Left
        } else if coord > right_bound + EPS {
            RectSide::Right
        } else {
            return None;
        };
        return Some(ExternalRail {
            edge,
            points: points.to_vec(),
            segment_index: segment.index,
            axis: Axis::Vertical,
            side,
            coord,
            min: js_min(segment.a.y, segment.b.y),
            max: js_max(segment.a.y, segment.b.y),
        });
    }

    if segment.horizontal {
        let coord = segment.a.y;
        let top_bound = js_min(src.top, dst.top);
        let bottom_bound = js_max(src.bottom, dst.bottom);
        let side = if coord < top_bound - EPS {
            RectSide::Top
        } else if coord > bottom_bound + EPS {
            RectSide::Bottom
        } else {
            return None;
        };
        return Some(ExternalRail {
            edge,
            points: points.to_vec(),
            segment_index: segment.index,
            axis: Axis::Horizontal,
            side,
            coord,
            min: js_min(segment.a.x, segment.b.x),
            max: js_max(segment.a.x, segment.b.x),
        });
    }

    None
}

/// `collectExternalRails()`: the external rails of the visible edges, in order.
fn collect_external_rails(view: &View, points: &Points, visible: &[usize]) -> Vec<ExternalRail> {
    let mut rails = Vec::new();
    for &edge in visible {
        let path = points_of(points, &Replacements::new(), edge);
        for segment in segments_for(&path) {
            if let Some(rail) = external_rail_for_segment(view, edge, &path, &segment) {
                rails.push(rail);
            }
        }
    }
    rails
}

/// `railsInteract(a, b)`: two rails of different edges on the same side of the same axis, that share a span.
fn rails_interact(a: &ExternalRail, b: &ExternalRail) -> bool {
    a.edge != b.edge
        && a.axis == b.axis
        && a.side == b.side
        && overlap_length(a.min, a.max, b.min, b.max) >= MIN_SHARED
}

/// `connectedComponents(rails)`: the groups of two or more rails that interact, each in the order the
/// reference's search visits them (the last found is searched first).
fn connected_components(rails: &[ExternalRail]) -> Vec<Vec<ExternalRail>> {
    let mut result = Vec::new();
    let mut seen = vec![false; rails.len()];
    for start in 0..rails.len() {
        if seen[start] {
            continue;
        }
        let mut queue = vec![start];
        let mut component = Vec::new();
        seen[start] = true;
        while let Some(current) = queue.pop() {
            component.push(current);
            for next in 0..rails.len() {
                if !seen[next] && rails_interact(&rails[current], &rails[next]) {
                    seen[next] = true;
                    queue.push(next);
                }
            }
        }
        if component.len() > 1 {
            result.push(
                component
                    .into_iter()
                    .map(|index| rails[index].clone())
                    .collect(),
            );
        }
    }
    result
}

/// `uniqueCoordsFor(component)`: the distinct coordinates of the rails, padded with new channels beyond the
/// outermost one on their side, until there is one per rail.
fn unique_coords_for(component: &[ExternalRail]) -> Vec<f64> {
    let mut coords: Vec<f64> = Vec::new();
    for rail in component {
        if !coords.iter().any(|coord| (coord - rail.coord).abs() < EPS) {
            coords.push(rail.coord);
        }
    }

    while coords.len() < component.len() {
        let min = js_min_of(coords.iter().copied());
        let max = js_max_of(coords.iter().copied());
        let side = component[0].side;
        let remaining = (component.len() - coords.len()) as f64;
        coords.push(if side == RectSide::Left || side == RectSide::Top {
            min - RAIL_CHANNEL_GAP * remaining
        } else {
            max + RAIL_CHANNEL_GAP * remaining
        });
    }
    coords
}

/// `coordinateAssignmentsFor(component)`: the ways to give the rails the channels, each one that changes a
/// coordinate. Small groups get every permutation, larger ones every swap of two rails.
fn coordinate_assignments_for(component: &[ExternalRail]) -> Vec<Vec<f64>> {
    let current: Vec<f64> = component.iter().map(|rail| rail.coord).collect();
    let coords = unique_coords_for(component);
    let mut assignments = Vec::new();

    if component.len() <= MAX_EXHAUSTIVE_COMPONENT {
        let mut used = vec![false; coords.len()];
        let mut next = Vec::new();
        visit_permutations(
            &coords,
            &current,
            &mut used,
            &mut next,
            component.len(),
            &mut assignments,
        );
        return assignments;
    }

    for i in 0..current.len() {
        for j in (i + 1)..current.len() {
            let mut assignment = current.clone();
            assignment.swap(i, j);
            assignments.push(assignment);
        }
    }
    assignments
}

/// The permutations of `coords` in index order, each recorded when it changes a coordinate of `current`.
fn visit_permutations(
    coords: &[f64],
    current: &[f64],
    used: &mut [bool],
    next: &mut Vec<f64>,
    len: usize,
    out: &mut Vec<Vec<f64>>,
) {
    if next.len() == len {
        if next
            .iter()
            .enumerate()
            .any(|(index, coord)| (coord - current[index]).abs() >= EPS)
        {
            out.push(next.clone());
        }
        return;
    }
    for (index, &coord) in coords.iter().enumerate() {
        if used[index] {
            continue;
        }
        used[index] = true;
        next.push(coord);
        visit_permutations(coords, current, used, next, len, out);
        next.pop();
        used[index] = false;
    }
}

/// `replacementsForAssignment(component, assignment)`: the rails moved to their channels, each edge's path
/// simplified, or `None` when a moved path is no longer orthogonal.
fn replacements_for_assignment(
    component: &[ExternalRail],
    assignment: &[f64],
) -> Option<Replacements> {
    let mut draft: IndexMap<usize, Vec<Point>> = IndexMap::new();
    for (index, rail) in component.iter().enumerate() {
        let coord = assignment[index];
        let mut path = draft
            .get(&rail.edge)
            .cloned()
            .unwrap_or_else(|| rail.points.clone());
        match rail.axis {
            Axis::Vertical => {
                path[rail.segment_index].x = coord;
                path[rail.segment_index + 1].x = coord;
            }
            Axis::Horizontal => {
                path[rail.segment_index].y = coord;
                path[rail.segment_index + 1].y = coord;
            }
        }
        draft.insert(rail.edge, path);
    }

    let mut replacements = Replacements::new();
    for (edge, path) in draft {
        let flat = simplified(&path);
        if !is_orthogonal_path(&flat) {
            return None;
        }
        replacements.insert(edge, flat);
    }
    Some(replacements)
}

/// `candidateIsSafe(replacements)`: the moved paths keep clear of the nodes and the labels, and they share no
/// track with a visible edge that they touch.
fn candidate_is_safe(
    view: &View,
    points: &Points,
    real: &[RectEntry],
    labels: &[RectEntry],
    visible: &[usize],
    replacements: &Replacements,
) -> bool {
    for (&edge, path) in replacements {
        let endpoints = endpoint_ids(&view.edges[edge]);
        for segment in segments_for(path) {
            if segment_hits_any_rect(segment.a, segment.b, real, &endpoints, -BUFFER) {
                return false;
            }
            if segment_hits_any_rect(segment.a, segment.b, labels, &[], -BUFFER) {
                return false;
            }
        }
    }

    for i in 0..visible.len() {
        let first = visible[i];
        let first_changed = replacements.contains_key(&first);
        let first_segments = segments_for(&points_of(points, replacements, first));
        for &second in &visible[(i + 1)..] {
            if !first_changed && !replacements.contains_key(&second) {
                continue;
            }
            let second_segments = segments_for(&points_of(points, replacements, second));
            for first_segment in &first_segments {
                for second_segment in &second_segments {
                    if same_axis_segment_overlap_length(first_segment, second_segment, 0.5)
                        >= MIN_SHARED
                    {
                        return false;
                    }
                }
            }
        }
    }

    true
}

/// One iteration: the best reassignment of one group of rails, or `None` when none improves the layout.
fn best_reassignment(
    view: &View,
    points: &Points,
    real: &[RectEntry],
    labels: &[RectEntry],
) -> Option<Replacements> {
    let visible = view.visible();
    let current_crossings = strict_crossing_count(&visible, points, &Replacements::new());
    if current_crossings == 0.0 {
        return None;
    }

    let mut best: Option<Replacements> = None;
    let mut best_crossings = current_crossings;
    let mut best_bends = total_bends(&visible, points, &Replacements::new());
    let mut best_displacement = f64::INFINITY;

    let rails = collect_external_rails(view, points, &visible);
    for component in connected_components(&rails) {
        for assignment in coordinate_assignments_for(&component) {
            let Some(replacements) = replacements_for_assignment(&component, &assignment) else {
                continue;
            };
            if !candidate_is_safe(view, points, real, labels, &visible, &replacements) {
                continue;
            }

            let candidate_crossings = strict_crossing_count(&visible, points, &replacements);
            if candidate_crossings >= current_crossings {
                continue;
            }
            let candidate_bends = total_bends(&visible, points, &replacements);
            let candidate_displacement = component
                .iter()
                .enumerate()
                .fold(0.0, |sum, (index, rail)| {
                    sum + (assignment[index] - rail.coord).abs()
                });

            if candidate_crossings > best_crossings
                || (candidate_crossings == best_crossings
                    && (candidate_bends > best_bends
                        || (candidate_bends == best_bends
                            && candidate_displacement >= best_displacement)))
            {
                continue;
            }

            best = Some(replacements);
            best_crossings = candidate_crossings;
            best_bends = candidate_bends;
            best_displacement = candidate_displacement;
        }
    }

    best
}
