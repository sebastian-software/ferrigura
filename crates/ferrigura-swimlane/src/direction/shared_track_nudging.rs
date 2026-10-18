//! `nudgeSharedInteriorSubpaths` (`direction/sharedTrackNudging.ts`, lines 13-321): the Wybrow-style shared-track
//! nudge. Two interior horizontal or vertical segments of different edges that run on nearly the same track and
//! overlap are moved apart by a shift of 7, 14 or 21, when the moved route stays clear of the nodes, the labels and
//! the other edges. The endpoint stubs stay pinned, and a source stub may be rerouted through a detour.
//!
//! The pass runs up to 12 rounds. A round moves at most one edge, and the first candidate that is safe is taken, in
//! the order of the segment pairs, the shifts, and the direct shift before the source detour.

use indexmap::IndexMap;

use super::geometry_stopgap::{
    EPS, OrthogonalSegment, RectEntry, collect_node_rect_entries, dedupe_consecutive_points,
    non_empty, orthogonal_segments_for_points, orthogonal_segments_strictly_cross, overlap_length,
    rect_of_node_bounds, segment_hits_any_rect,
};
use super::node_by_id_map;
use crate::model::{Edge, LayoutData, Node, Point, Rect};

const EPS_LOCAL: f64 = 1e-3;
const MIN_SHARED: f64 = 8.0;
const TRACK_SHIFT: f64 = 7.0;
const MIN_TRACK_GAP: f64 = TRACK_SHIFT;
const SOURCE_DETOUR_STUB: f64 = 20.0;
const BUFFER: f64 = 2.0;
const MAX_ITERATIONS: usize = 12;
const SHIFTS: [f64; 6] = [
    -TRACK_SHIFT,
    TRACK_SHIFT,
    -2.0 * TRACK_SHIFT,
    2.0 * TRACK_SHIFT,
    -3.0 * TRACK_SHIFT,
    3.0 * TRACK_SHIFT,
];

/// A segment of an edge (`SegmentLite`): the index of the edge, the segment, and whether it is interior, which
/// keeps the first and last stubs of the route out of the shift.
#[derive(Debug, Clone, Copy)]
struct SegmentLite {
    edge: usize,
    segment: OrthogonalSegment,
    interior: bool,
}

/// The rectangles of the real nodes and of the label nodes, computed once for the pass.
struct Rects {
    real: Vec<RectEntry>,
    labels: Vec<RectEntry>,
}

/// `segmentsFor(edge, points)`: the orthogonal segments of a polyline of the edge, with their interior flag.
fn segments_for(edge: usize, points: &[Point]) -> Vec<SegmentLite> {
    orthogonal_segments_for_points(points, EPS_LOCAL)
        .into_iter()
        .map(|segment| SegmentLite {
            edge,
            interior: segment.index >= 1 && segment.index + 3 <= points.len(),
            segment,
        })
        .collect()
}

/// `allSegments()`: the segments of every routed edge that is not layout-only, of its deduplicated polyline.
fn all_segments(edges: &[Edge]) -> Vec<SegmentLite> {
    let mut result = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        if edge.is_layout_only == Some(true) {
            continue;
        }
        let Some(points) = edge.points.as_deref() else {
            continue;
        };
        if points.len() < 2 {
            continue;
        }
        result.extend(segments_for(index, &dedupe_consecutive_points(points, EPS)));
    }
    result
}

/// `hasCrowdedParallelTrack(a, b)`: two segments on the same axis, on tracks less than 7 apart, that share at least 8.
fn has_crowded_parallel_track(a: &SegmentLite, b: &SegmentLite) -> bool {
    let (sa, sb) = (a.segment, b.segment);
    if sa.horizontal && sb.horizontal {
        return overlap_length(sa.a.x, sa.b.x, sb.a.x, sb.b.x) >= MIN_SHARED
            && (sa.a.y - sb.a.y).abs() < MIN_TRACK_GAP;
    }
    if sa.vertical && sb.vertical {
        return overlap_length(sa.a.y, sa.b.y, sb.a.y, sb.b.y) >= MIN_SHARED
            && (sa.a.x - sb.a.x).abs() < MIN_TRACK_GAP;
    }
    false
}

/// `candidateIsSafe(edge, candidate)`: the candidate polyline of an edge keeps its segments orthogonal, clear of the
/// nodes and labels (with a buffer of 2), and it has no crowded parallel track and no strict crossing with another
/// edge.
fn candidate_is_safe(
    edges: &[Edge],
    edge_index: usize,
    candidate: &[Point],
    rects: &Rects,
) -> bool {
    let edge = &edges[edge_index];
    let candidate_segments = segments_for(edge_index, candidate);
    if candidate_segments.len() + 1 != candidate.len() {
        return false;
    }

    let endpoint_ids: Vec<&str> = [edge.start.as_deref(), edge.end.as_deref()]
        .into_iter()
        .filter_map(|id| non_empty(id))
        .collect();
    let own_label_ids: Vec<&str> = non_empty(edge.label_node_id.as_deref())
        .into_iter()
        .collect();
    for segment in &candidate_segments {
        let (a, b) = (segment.segment.a, segment.segment.b);
        if segment_hits_any_rect(a, b, &rects.real, &endpoint_ids, -BUFFER) {
            return false;
        }
        if segment_hits_any_rect(a, b, &rects.labels, &own_label_ids, -BUFFER) {
            return false;
        }
    }

    for (other_index, other) in edges.iter().enumerate() {
        if other_index == edge_index || other.is_layout_only == Some(true) {
            continue;
        }
        let Some(other_points) = other.points.as_deref() else {
            continue;
        };
        if other_points.len() < 2 {
            continue;
        }
        let other_segments =
            segments_for(other_index, &dedupe_consecutive_points(other_points, EPS));
        for candidate_segment in &candidate_segments {
            for other_segment in &other_segments {
                if has_crowded_parallel_track(candidate_segment, other_segment) {
                    return false;
                }
                if orthogonal_segments_strictly_cross(
                    candidate_segment.segment.a,
                    candidate_segment.segment.b,
                    other_segment.segment.a,
                    other_segment.segment.b,
                    EPS_LOCAL,
                ) {
                    return false;
                }
            }
        }
    }

    true
}

/// `shiftedCandidate(segment, shift)`: the polyline of the segment's edge with the segment moved by `shift` across its
/// axis, when the moved polyline still has one segment per step.
fn shifted_candidate(edges: &[Edge], segment: &SegmentLite, shift: f64) -> Option<Vec<Point>> {
    let points =
        dedupe_consecutive_points(edges[segment.edge].points.as_deref().unwrap_or(&[]), EPS);
    let index = segment.segment.index;
    if points.len() < 4 || index + 1 >= points.len() {
        return None;
    }
    let mut candidate = points;
    if segment.segment.horizontal {
        candidate[index].y += shift;
        candidate[index + 1].y += shift;
    } else if segment.segment.vertical {
        candidate[index].x += shift;
        candidate[index + 1].x += shift;
    } else {
        return None;
    }
    (segments_for(segment.edge, &candidate).len() + 1 == candidate.len()).then_some(candidate)
}

/// `SourceDetourContext`: the centres and the source rectangle of an edge whose source stub is detoured, and the
/// points after the detoured segment.
struct SourceDetourContext {
    source_center: Point,
    target_center: Point,
    source_rect: Rect,
    tail: Vec<Point>,
}

/// `nodeCenter(node, rect)`: the centre of a node, or the centre of its rectangle when it has no centre.
fn node_center(node: &Node, rect: Rect) -> Point {
    Point {
        x: node.x.unwrap_or((rect.left + rect.right) / 2.0),
        y: node.y.unwrap_or((rect.top + rect.bottom) / 2.0),
    }
}

/// `sourceDetourContextFor(segment)`: the context of a four-point route whose first segment is the one to detour.
fn source_detour_context(
    edges: &[Edge],
    nodes: &[Node],
    node_by_id: &IndexMap<String, usize>,
    segment: &SegmentLite,
) -> Option<SourceDetourContext> {
    let edge = &edges[segment.edge];
    let points = dedupe_consecutive_points(edge.points.as_deref().unwrap_or(&[]), EPS);
    if points.len() != 4 || segment.segment.index != 1 {
        return None;
    }

    let source_node = non_empty(edge.start.as_deref())
        .and_then(|id| node_by_id.get(id))
        .map(|&index| &nodes[index]);
    let target_node = non_empty(edge.end.as_deref())
        .and_then(|id| node_by_id.get(id))
        .map(|&index| &nodes[index]);
    let source_rect = source_node.and_then(rect_of_node_bounds);
    let target_rect = target_node.and_then(rect_of_node_bounds);
    let tail = points[segment.segment.index + 2..].to_vec();
    let (Some(source_node), Some(target_node), Some(source_rect), Some(target_rect)) =
        (source_node, target_node, source_rect, target_rect)
    else {
        return None;
    };
    if tail.is_empty() {
        return None;
    }

    Some(SourceDetourContext {
        source_center: node_center(source_node, source_rect),
        target_center: node_center(target_node, target_rect),
        source_rect,
        tail,
    })
}

/// `verticalSourceDetour`: the first segment of a vertical source stub, moved through a stub below or above the
/// source port.
fn vertical_source_detour(
    segment: &SegmentLite,
    shift: f64,
    context: &SourceDetourContext,
) -> Option<Vec<Point>> {
    let target_below = context.target_center.y >= context.source_center.y;
    let source_port_y = if target_below {
        context.source_rect.bottom
    } else {
        context.source_rect.top
    };
    let stub_y = source_port_y
        + if target_below {
            SOURCE_DETOUR_STUB
        } else {
            -SOURCE_DETOUR_STUB
        };
    let b = segment.segment.b;
    if (target_below && b.y <= stub_y + EPS_LOCAL) || (!target_below && b.y >= stub_y - EPS_LOCAL) {
        return None;
    }

    let rail_x = segment.segment.a.x + shift;
    let mut points = vec![
        Point {
            x: context.source_center.x,
            y: source_port_y,
        },
        Point {
            x: context.source_center.x,
            y: stub_y,
        },
        Point {
            x: rail_x,
            y: stub_y,
        },
        Point { x: rail_x, y: b.y },
    ];
    points.extend_from_slice(&context.tail);
    Some(dedupe_consecutive_points(&points, EPS_LOCAL))
}

/// `horizontalSourceDetour`: the first segment of a horizontal source stub, moved through a stub beside the source
/// port.
fn horizontal_source_detour(
    segment: &SegmentLite,
    shift: f64,
    context: &SourceDetourContext,
) -> Option<Vec<Point>> {
    let target_right = context.target_center.x >= context.source_center.x;
    let source_port_x = if target_right {
        context.source_rect.right
    } else {
        context.source_rect.left
    };
    let stub_x = source_port_x
        + if target_right {
            SOURCE_DETOUR_STUB
        } else {
            -SOURCE_DETOUR_STUB
        };
    let b = segment.segment.b;
    if (target_right && b.x <= stub_x + EPS_LOCAL) || (!target_right && b.x >= stub_x - EPS_LOCAL) {
        return None;
    }

    let rail_y = segment.segment.a.y + shift;
    let mut points = vec![
        Point {
            x: source_port_x,
            y: context.source_center.y,
        },
        Point {
            x: stub_x,
            y: context.source_center.y,
        },
        Point {
            x: stub_x,
            y: rail_y,
        },
        Point { x: b.x, y: rail_y },
    ];
    points.extend_from_slice(&context.tail);
    Some(dedupe_consecutive_points(&points, EPS_LOCAL))
}

/// `sourceDetourCandidate(segment, shift)`.
fn source_detour_candidate(
    edges: &[Edge],
    nodes: &[Node],
    node_by_id: &IndexMap<String, usize>,
    segment: &SegmentLite,
    shift: f64,
) -> Option<Vec<Point>> {
    let context = source_detour_context(edges, nodes, node_by_id, segment)?;
    if segment.segment.vertical {
        return vertical_source_detour(segment, shift, &context);
    }
    if segment.segment.horizontal {
        return horizontal_source_detour(segment, shift, &context);
    }
    None
}

/// One round of the pass: the first safe move, as the edge index and its new polyline, or `None` when no segment
/// pair can be moved.
fn find_nudge(
    edges: &[Edge],
    nodes: &[Node],
    node_by_id: &IndexMap<String, usize>,
    rects: &Rects,
) -> Option<(usize, Vec<Point>)> {
    let segments = all_segments(edges);
    for i in 0..segments.len() {
        for j in i + 1..segments.len() {
            let (first, second) = (segments[i], segments[j]);
            if first.edge == second.edge || !has_crowded_parallel_track(&first, &second) {
                continue;
            }

            let candidates: Vec<SegmentLite> = [first, second]
                .into_iter()
                .filter(|segment| segment.interior)
                .collect();
            for segment in candidates {
                for shift in SHIFTS {
                    if let Some(direct) = shifted_candidate(edges, &segment, shift)
                        && candidate_is_safe(edges, segment.edge, &direct, rects)
                    {
                        return Some((segment.edge, direct));
                    }
                    if let Some(detoured) =
                        source_detour_candidate(edges, nodes, node_by_id, &segment, shift)
                        && candidate_is_safe(edges, segment.edge, &detoured, rects)
                    {
                        return Some((segment.edge, detoured));
                    }
                }
            }
        }
    }
    None
}

/// `nudgeSharedInteriorSubpaths(edges, nodeByIdMap)`, with the node map built from the nodes of the layout.
pub(crate) fn nudge_shared_interior_subpaths(data: &mut LayoutData) {
    let LayoutData { nodes, edges, .. } = data;
    let nodes: &[Node] = nodes;
    let node_by_id = node_by_id_map(nodes);
    let (real, labels) = collect_node_rect_entries(node_by_id.values().map(|&index| &nodes[index]));
    let rects = Rects { real, labels };

    for _ in 0..MAX_ITERATIONS {
        match find_nudge(edges, nodes, &node_by_id, &rects) {
            Some((index, points)) => edges[index].points = Some(points),
            None => return,
        }
    }
}
