//! The detour simplification of `postProcessSwimlaneLayout`: `direction/detourSimplification.ts` of Mermaid
//! 12.2.0 (`simplifyDetouredEdges`, lines 20-309).
//!
//! An edge with four or more bends is replaced by a candidate path between the sides of its two ends (the
//! orthogonal port path, and for two ends on the same side the track outside both of them), when the candidate
//! avoids the other real nodes, does not claim a face that another edge claims, and conflicts with no more
//! other edges than the current path does. The edges are rewritten in order, and each rewrite is seen by the
//! edges after it: the conflict counts and the face claims read the current points.
//!
//! The rectangles of the nodes do not change in this pass, so they are read once. The edges are read and
//! written in index order, as the TypeScript loop does.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::direction::geometry::{
    EPS, NodeBoundsInfo, RectEntry, RectSide, build_orthogonal_port_path,
    build_same_side_track_path, collect_real_node_bounds, count_orthogonal_bends,
    orthogonal_segments_cross, orthogonal_segments_for_points, port_for_rect_side,
    same_axis_segment_overlap_length, segment_hits_any_rect, truthy_text,
};
use crate::math::{js_max, js_min};
use crate::model::{Edge, LayoutData, Point};

/// `ANCHOR` (`detourSimplification.ts:30`): how far a port's first segment runs before it turns.
const ANCHOR: f64 = 20.0;
/// `MIN_SHARED` (`detourSimplification.ts:18`): the length that two segments on one line must share to conflict.
const MIN_SHARED: f64 = 8.0;
/// `BEND_THRESHOLD` (`detourSimplification.ts:129`): an edge with fewer bends is left as it is.
const BEND_THRESHOLD: usize = 4;
/// The sides of a rectangle, in the order the TypeScript tries them.
const SIDES: [RectSide; 4] = [
    RectSide::Top,
    RectSide::Bottom,
    RectSide::Left,
    RectSide::Right,
];

/// `outsideTracks` (`detourSimplification.ts:32-37`): the track outside all the real nodes, on each side.
#[derive(Debug, Clone, Copy)]
struct OutsideTracks {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
}

impl OutsideTracks {
    fn of(&self, side: RectSide) -> f64 {
        match side {
            RectSide::Top => self.top,
            RectSide::Bottom => self.bottom,
            RectSide::Left => self.left,
            RectSide::Right => self.right,
        }
    }
}

/// A claim on a face of a node by an edge (`FaceClaim`, `detourSimplification.ts:174-177`).
#[derive(Debug, Clone, PartialEq)]
struct FaceClaim {
    side: RectSide,
    edge_id: String,
}

/// The face claims of each node, by node id.
type FaceClaims = HashMap<String, Vec<FaceClaim>>;

/// `nearestSideOfRect(pt, info)` (`detourSimplification.ts:152-172`): the side of the rectangle that the point is
/// closest to. A tie keeps the earlier side in the order top, bottom, left, right.
fn nearest_side_of_rect(pt: Point, info: &NodeBoundsInfo) -> RectSide {
    let d_top = (pt.y - info.rect.top).abs();
    let d_bottom = (pt.y - info.rect.bottom).abs();
    let d_left = (pt.x - info.rect.left).abs();
    let d_right = (pt.x - info.rect.right).abs();
    let mut best = RectSide::Top;
    let mut best_dist = d_top;
    if d_bottom < best_dist {
        best = RectSide::Bottom;
        best_dist = d_bottom;
    }
    if d_left < best_dist {
        best = RectSide::Left;
        best_dist = d_left;
    }
    if d_right < best_dist {
        best = RectSide::Right;
    }
    best
}

/// `addFaceClaim(nodeId, side, edgeId)` (`detourSimplification.ts:179-183`).
fn add_face_claim(claims: &mut FaceClaims, node_id: &str, side: RectSide, edge_id: &str) {
    claims
        .entry(node_id.to_owned())
        .or_default()
        .push(FaceClaim {
            side,
            edge_id: edge_id.to_owned(),
        });
}

/// `faceIsClaimed(nodeId, side, ignoreEdgeId)` (`detourSimplification.ts:209-213`): another edge claims the face.
fn face_is_claimed(
    claims: &FaceClaims,
    node_id: &str,
    side: RectSide,
    ignore_edge_id: &str,
) -> bool {
    claims.get(node_id).is_some_and(|list| {
        list.iter()
            .any(|claim| claim.edge_id != ignore_edge_id && claim.side == side)
    })
}

/// `pathHitsNode(pts, excludeIds)` (`detourSimplification.ts:64-73`): a segment of the path meets a real node
/// other than the two ends, with the rectangles shrunk by one.
fn path_hits_node(pts: &[Point], exclude_ids: &[&str], real_node_rects: &[RectEntry]) -> bool {
    pts.windows(2)
        .any(|pair| segment_hits_any_rect(pair[0], pair[1], real_node_rects, exclude_ids, 1.0))
}

/// `pathConflictCount(path, currentEdge, includeIncidentEdges)` (`detourSimplification.ts:75-127`): the number of
/// crossings and shared runs of at least `MIN_SHARED` between the path and the other edges. `current` is the index
/// of the edge that the path would replace, and that edge is left out. Unless `include_incident`, an edge that
/// shares an end with the current edge is left out too.
fn path_conflict_count(
    path: &[Point],
    edges: &[Edge],
    current: usize,
    include_incident: bool,
) -> usize {
    let path_segments = orthogonal_segments_for_points(path, EPS);
    let current_edge = &edges[current];
    let current_start = truthy_text(current_edge.start.as_deref());
    let current_end = truthy_text(current_edge.end.as_deref());

    let mut conflicts = 0;
    for (index, other) in edges.iter().enumerate() {
        if index == current || other.is_layout_only == Some(true) {
            continue;
        }
        if !include_incident && let (Some(start), Some(end)) = (current_start, current_end) {
            let shares_end = |id: Option<&str>| id == Some(start) || id == Some(end);
            if shares_end(other.start.as_deref()) || shares_end(other.end.as_deref()) {
                continue;
            }
        }
        let Some(other_points) = other.points.as_deref() else {
            continue;
        };
        if other_points.len() < 2 {
            continue;
        }
        let other_segments = orthogonal_segments_for_points(other_points, EPS);
        for path_segment in &path_segments {
            for other_segment in &other_segments {
                if orthogonal_segments_cross(
                    path_segment.a,
                    path_segment.b,
                    other_segment.a,
                    other_segment.b,
                    EPS,
                    EPS,
                ) {
                    conflicts += 1;
                    continue;
                }
                if same_axis_segment_overlap_length(path_segment, other_segment, EPS) >= MIN_SHARED
                {
                    conflicts += 1;
                }
            }
        }
    }
    conflicts
}

/// `buildOrthogonalPathCandidates(src, srcSide, dst, dstSide)` (`detourSimplification.ts:39-62`): the orthogonal
/// port path when there is one, and for two ends on the same side the track outside all the nodes.
fn build_orthogonal_path_candidates(
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
    outside_tracks: OutsideTracks,
) -> Vec<Vec<Point>> {
    let mut paths = Vec::new();
    if let Some(base) = build_orthogonal_port_path(src, src_side, dst, dst_side, ANCHOR, EPS) {
        paths.push(base);
    }
    if src_side == dst_side {
        paths.push(build_same_side_track_path(
            src,
            src_side,
            dst,
            outside_tracks.of(src_side),
        ));
    }
    paths
}

/// `simplifyDetouredEdges(edges, nodes)` (`detourSimplification.ts:20-309`): each edge with at least
/// `BEND_THRESHOLD` bends takes the candidate path that has the fewest bends among those that conflict no more
/// than the current path, or, when the current path crosses other edges, the candidate with the fewest crossings.
pub fn simplify_detoured_edges(layout: &mut LayoutData) {
    let (node_info_by_id, real_node_rects) = collect_real_node_bounds(&layout.nodes);

    let outside_tracks = OutsideTracks {
        top: real_node_rects
            .iter()
            .fold(f64::INFINITY, |acc, node| js_min(acc, node.rect.top))
            - ANCHOR,
        bottom: real_node_rects
            .iter()
            .fold(f64::NEG_INFINITY, |acc, node| js_max(acc, node.rect.bottom))
            + ANCHOR,
        left: real_node_rects
            .iter()
            .fold(f64::INFINITY, |acc, node| js_min(acc, node.rect.left))
            - ANCHOR,
        right: real_node_rects
            .iter()
            .fold(f64::NEG_INFINITY, |acc, node| js_max(acc, node.rect.right))
            + ANCHOR,
    };

    let mut face_claims = initial_face_claims(&layout.edges, &node_info_by_id);

    for index in 0..layout.edges.len() {
        let edges = &layout.edges;
        let edge = &edges[index];
        if edge.is_layout_only == Some(true) {
            continue;
        }
        let pts: Vec<Point> = match edge.points.as_deref() {
            Some(points) if points.len() >= 2 => points.to_vec(),
            _ => continue,
        };
        let current_bends = count_orthogonal_bends(&pts, EPS);
        if current_bends < BEND_THRESHOLD {
            continue;
        }
        let (Some(src_id), Some(dst_id)) = (
            truthy_text(edge.start.as_deref()),
            truthy_text(edge.end.as_deref()),
        ) else {
            continue;
        };
        let (Some(src_info), Some(dst_info)) =
            (node_info_by_id.get(src_id), node_info_by_id.get(dst_id))
        else {
            continue;
        };
        let edge_id = edge.id.as_str();

        let current_crossing_conflicts = path_conflict_count(&pts, edges, index, true);
        let current_non_incident_conflicts = path_conflict_count(&pts, edges, index, false);

        let mut best_path: Option<Vec<Point>> = None;
        let mut best_crossing_conflicts = current_crossing_conflicts;
        let mut best_bends = current_bends;

        for src_side in SIDES {
            if face_is_claimed(&face_claims, src_id, src_side, edge_id) {
                continue;
            }
            let src_port = port_for_rect_side(src_info, src_side);
            for dst_side in SIDES {
                if face_is_claimed(&face_claims, dst_id, dst_side, edge_id) {
                    continue;
                }
                let dst_port = port_for_rect_side(dst_info, dst_side);
                for path in build_orthogonal_path_candidates(
                    src_port,
                    src_side,
                    dst_port,
                    dst_side,
                    outside_tracks,
                ) {
                    if path_hits_node(&path, &[src_id, dst_id], &real_node_rects) {
                        continue;
                    }

                    let path_bends = count_orthogonal_bends(&path, EPS);
                    if current_crossing_conflicts > 0 {
                        let path_crossing_conflicts =
                            path_conflict_count(&path, edges, index, true);
                        if path_crossing_conflicts > best_crossing_conflicts
                            || (path_crossing_conflicts == best_crossing_conflicts
                                && path_bends >= best_bends)
                        {
                            continue;
                        }
                        best_crossing_conflicts = path_crossing_conflicts;
                        best_bends = path_bends;
                        best_path = Some(path);
                        continue;
                    }

                    if path_conflict_count(&path, edges, index, false)
                        > current_non_incident_conflicts
                    {
                        continue;
                    }
                    if path_bends < best_bends {
                        best_bends = path_bends;
                        best_path = Some(path);
                    }
                }
            }
        }

        let Some(best_path) = best_path else {
            continue;
        };
        let edge_id = edge_id.to_owned();
        let src_id = src_id.to_owned();
        let dst_id = dst_id.to_owned();
        let (src_info, dst_info) = (src_info.clone(), dst_info.clone());

        // The face claims of this edge are replaced by the faces of the new path, so that the edges after it see
        // the new attach sides.
        for node_id in [&src_id, &dst_id] {
            if let Some(claims) = face_claims.get_mut(node_id.as_str()) {
                claims.retain(|claim| claim.edge_id != edge_id);
            }
        }
        let first = best_path[0];
        let last = best_path[best_path.len() - 1];
        add_face_claim(
            &mut face_claims,
            &src_id,
            nearest_side_of_rect(first, &src_info),
            &edge_id,
        );
        add_face_claim(
            &mut face_claims,
            &dst_id,
            nearest_side_of_rect(last, &dst_info),
            &edge_id,
        );
        layout.edges[index].points = Some(best_path);
    }
}

/// The face claims of the edges as they are before the loop (`detourSimplification.ts:184-207`): the first point
/// of each edge claims a face of its start, the last point a face of its end.
fn initial_face_claims(
    edges: &[Edge],
    node_info_by_id: &IndexMap<String, NodeBoundsInfo>,
) -> FaceClaims {
    let mut claims = FaceClaims::new();
    for edge in edges {
        if edge.is_layout_only == Some(true) {
            continue;
        }
        let pts = edge.points.as_deref().unwrap_or(&[]);
        let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
            continue;
        };
        let edge_id = edge.id.as_str();
        if let Some(start_id) = truthy_text(edge.start.as_deref())
            && let Some(info) = node_info_by_id.get(start_id)
        {
            add_face_claim(
                &mut claims,
                start_id,
                nearest_side_of_rect(first, info),
                edge_id,
            );
        }
        if let Some(end_id) = truthy_text(edge.end.as_deref())
            && let Some(info) = node_info_by_id.get(end_id)
        {
            add_face_claim(
                &mut claims,
                end_id,
                nearest_side_of_rect(last, info),
                edge_id,
            );
        }
    }
    claims
}
