//! `swapDestinationTerminalTailsToReduceCrossings` (`materializedGeometry.ts:986-1204`): two edges that end at
//! the same node exchange their last segments, when the exchange removes crossings and keeps clear of the nodes
//! and of the other tracks.

use indexmap::IndexMap;

use super::common::*;
use crate::model::{LayoutData, Point};

const BUFFER: f64 = 2.0;
const MAX_ITERATIONS: usize = 4;

/// `swapDestinationTerminalTailsToReduceCrossings(edges, nodeByIdMap)`.
pub fn swap_destination_terminal_tails_to_reduce_crossings(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        let (real, _labels) = node_rect_entries(view);
        for _ in 0..MAX_ITERATIONS {
            let Some(replacements) = best_swap(view, points, &real) else {
                return;
            };
            for (edge, path) in replacements {
                points.set(edge, path);
            }
        }
    });
}

/// The last segment of an edge: the point before the terminal and the terminal.
#[derive(Clone, Copy)]
struct TerminalTail {
    tail_start: Point,
    terminal: Point,
}

/// `terminalTailFor(edge)`.
fn terminal_tail_for(points: &Points, edge: usize) -> Option<TerminalTail> {
    let path = points_of(points, &Replacements::new(), edge);
    if path.len() < 4 {
        return None;
    }
    let tail_start = path[path.len() - 2];
    let terminal = path[path.len() - 1];
    if !is_horizontal_segment(tail_start, terminal, EPS)
        && !is_vertical_segment(tail_start, terminal, EPS)
    {
        return None;
    }
    Some(TerminalTail {
        tail_start,
        terminal,
    })
}

/// `candidateWithDestinationTail(edge, tail)`: the edge, with the tail of another edge at its end.
fn candidate_with_destination_tail(
    points: &Points,
    edge: usize,
    tail: TerminalTail,
) -> Option<Vec<Point>> {
    let path = points_of(points, &Replacements::new(), edge);
    if path.len() < 3 {
        return None;
    }
    let start = path[0];
    let first_turn = path[1];

    let connector = if is_horizontal_segment(start, first_turn, EPS) {
        Point {
            x: first_turn.x,
            y: tail.tail_start.y,
        }
    } else if is_vertical_segment(start, first_turn, EPS) {
        Point {
            x: tail.tail_start.x,
            y: first_turn.y,
        }
    } else {
        return None;
    };

    let candidate = simplified(&[start, first_turn, connector, tail.tail_start, tail.terminal]);
    if is_orthogonal_path(&candidate) {
        Some(candidate)
    } else {
        None
    }
}

/// `pathHasNodeHit(edge, path)`.
fn path_has_node_hit(view: &View, real: &[RectEntry], edge: usize, path: &[Point]) -> bool {
    let endpoints = endpoint_ids(&view.edges[edge]);
    segments_for(path)
        .iter()
        .any(|segment| segment_hits_any_rect(segment.a, segment.b, real, &endpoints, -BUFFER))
}

/// `pathHasSharedTrack(edge, path, replacements)`: a visible edge other than `edge` runs along the path.
fn path_has_shared_track(
    view: &View,
    points: &Points,
    edge: usize,
    path: &[Point],
    replacements: &Replacements,
) -> bool {
    for other in view.visible() {
        if other == edge {
            continue;
        }
        for candidate_segment in segments_for(path) {
            for other_segment in segments_for(&points_of(points, replacements, other)) {
                if same_axis_segment_overlap_length(&candidate_segment, &other_segment, 0.5)
                    >= MIN_SHARED
                {
                    return true;
                }
            }
        }
    }
    false
}

/// `candidateIsSafe(edge, path, replacements)`.
fn candidate_is_safe(
    view: &View,
    points: &Points,
    real: &[RectEntry],
    edge: usize,
    path: &[Point],
    replacements: &Replacements,
) -> bool {
    !path_has_node_hit(view, real, edge, path)
        && !path_has_shared_track(view, points, edge, path, replacements)
}

/// `edgesByDestination()`: the visible edges that are long enough, grouped by the node they end at, in the
/// order of the first edge that ends at each node.
fn edges_by_destination(view: &View, points: &Points) -> Vec<Vec<usize>> {
    let mut result: IndexMap<String, Vec<usize>> = IndexMap::new();
    for edge in view.visible() {
        let Some(destination) = truthy_text(view.edges[edge].end.as_deref()) else {
            continue;
        };
        if !view.has_node(destination) {
            continue;
        }
        if points_of(points, &Replacements::new(), edge).len() < 4 {
            continue;
        }
        result.entry(destination.to_owned()).or_default().push(edge);
    }
    result.into_values().collect()
}

/// `crossingCount(replacements)`, the crossings of the visible edges.
fn crossing_count(visible: &[usize], points: &Points, replacements: &Replacements) -> f64 {
    strict_crossing_count(visible, points, replacements)
}

/// One iteration: the best pair of tail exchanges, or `None` when no exchange improves the layout.
fn best_swap(view: &View, points: &Points, real: &[RectEntry]) -> Option<Replacements> {
    let visible = view.visible();
    let current_crossings = crossing_count(&visible, points, &Replacements::new());
    if current_crossings == 0.0 {
        return None;
    }
    let current_bends = total_bends(&visible, points, &Replacements::new());

    let mut best: Option<Replacements> = None;
    let mut best_crossings = current_crossings;
    let mut best_bends = current_bends;

    for destination_edges in edges_by_destination(view, points) {
        for i in 0..destination_edges.len() {
            for j in (i + 1)..destination_edges.len() {
                let first = destination_edges[i];
                let second = destination_edges[j];
                let (Some(first_tail), Some(second_tail)) = (
                    terminal_tail_for(points, first),
                    terminal_tail_for(points, second),
                ) else {
                    continue;
                };

                let (Some(first_candidate), Some(second_candidate)) = (
                    candidate_with_destination_tail(points, first, second_tail),
                    candidate_with_destination_tail(points, second, first_tail),
                ) else {
                    continue;
                };

                let mut replacements = Replacements::new();
                replacements.insert(first, first_candidate.clone());
                replacements.insert(second, second_candidate.clone());
                if !candidate_is_safe(view, points, real, first, &first_candidate, &replacements)
                    || !candidate_is_safe(
                        view,
                        points,
                        real,
                        second,
                        &second_candidate,
                        &replacements,
                    )
                {
                    continue;
                }

                let candidate_crossings = crossing_count(&visible, points, &replacements);
                let candidate_bends = total_bends(&visible, points, &replacements);
                if candidate_crossings >= current_crossings {
                    continue;
                }
                if candidate_crossings > best_crossings
                    || (candidate_crossings == best_crossings && candidate_bends >= best_bends)
                {
                    continue;
                }
                best = Some(replacements);
                best_crossings = candidate_crossings;
                best_bends = candidate_bends;
            }
        }
    }

    best
}
