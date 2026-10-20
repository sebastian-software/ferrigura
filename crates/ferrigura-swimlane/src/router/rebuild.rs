//! The geometry rebuild and the write-back of the router (`routeEdgesOrthogonal`, phase 3 at lines 2006-2205, and
//! the end of the function at lines 2210-2282).
//!
//! The segments of each pipe are grouped into clusters of overlapping intervals. In each cluster, the tracks are
//! spread around the pipe's coordinate: the tracks that the destinations pull to the left go left, those pulled to
//! the right go right, and the neutral track is at the coordinate. Then the points of each edge are rebuilt from
//! its segments, and written to the layout, with the endpoints clamped to the boundary of their nodes.

use std::collections::HashMap;

use indexmap::IndexSet;

use crate::math::{js_max, js_min};
use crate::model::{LayoutData, Point};

use super::{EPS, Geo, Orientation, Router, TRACK_SPACING};
use crate::js::js_sort_by;

/// A segment of a pipe, with the track that holds it (`SegmentInfo`, lines 2013-2019).
#[derive(Clone, Copy, Debug)]
struct SegmentInfo {
    edge: usize,
    segment: usize,
    track: usize,
    from: f64,
    to: f64,
}

/// A segment as the rebuild draws it (`RoutedLine`): its orientation, the coordinate of its track, and its span.
#[derive(Clone, Copy, Debug)]
struct RoutedLine {
    orientation: Orientation,
    coord: f64,
    from: f64,
    to: f64,
}

/// `pointOnLine(line, along)` (`router.ts:114-116`).
fn point_on_line(line: RoutedLine, along: f64) -> Point {
    if line.orientation == Orientation::Vertical {
        Point {
            x: line.coord,
            y: along,
        }
    } else {
        Point {
            x: along,
            y: line.coord,
        }
    }
}

/// `sharedLineEndpointCoord(line, nextLine)` (`router.ts:108-112`): the end of the line that the next line
/// shares, or the start of the line.
fn shared_line_endpoint_coord(line: RoutedLine, next: RoutedLine) -> f64 {
    if (line.to - next.from).abs() < EPS || (line.to - next.to).abs() < EPS {
        line.to
    } else {
        line.from
    }
}

/// The points of an edge from its routed lines (`router.ts:2141-2204`): the lines are joined at their junctions,
/// and the path ends at the destination port. Repeated points are dropped.
fn rebuild_route(src_port: Point, dst_port: Point, lines: &[RoutedLine]) -> Vec<Point> {
    let mut new_points = vec![src_port];
    for k in 0..lines.len() {
        let line = lines[k];
        let prev_pt = new_points[new_points.len() - 1];
        let vertical = line.orientation == Orientation::Vertical;
        let prev_along = if vertical { prev_pt.y } else { prev_pt.x };
        let prev_track_coord = if vertical { prev_pt.x } else { prev_pt.y };
        let has_next_line = k + 1 < lines.len();

        if (prev_track_coord - line.coord).abs() > EPS {
            new_points.push(point_on_line(line, prev_along));
        }

        if has_next_line && lines[k + 1].orientation == line.orientation {
            let next = lines[k + 1];
            if (line.coord - next.coord).abs() > EPS {
                let junction = if vertical {
                    (prev_along + next.from) / 2.0
                } else {
                    shared_line_endpoint_coord(line, next)
                };
                new_points.push(point_on_line(line, junction));
                new_points.push(point_on_line(next, junction));
            } else if k == 0 || k + 2 == lines.len() {
                new_points.push(point_on_line(line, shared_line_endpoint_coord(line, next)));
            }
        } else if has_next_line {
            new_points.push(point_on_line(line, lines[k + 1].coord));
        } else {
            let end_along = if (line.from - prev_along).abs() < (line.to - prev_along).abs() {
                line.to
            } else {
                line.from
            };
            new_points.push(point_on_line(line, end_along));
        }
    }

    // The path ends at the destination port, so that the renderer's intersection keeps the last anchor.
    let last = new_points[new_points.len() - 1];
    if (last.x - dst_port.x).abs() > EPS || (last.y - dst_port.y).abs() > EPS {
        new_points.push(dst_port);
    }

    let mut filtered = vec![new_points[0]];
    for &p in &new_points[1..] {
        let prev = filtered[filtered.len() - 1];
        if (p.x - prev.x).abs() > EPS || (p.y - prev.y).abs() > EPS {
            filtered.push(p);
        }
    }
    filtered
}

/// `nodeBoundaryClamp(p, node)` (`router.ts:2229-2263`): a point inside or on the box of the node moves to the
/// nearest side of the box. A node without a size, or with a size of zero or less, leaves the point as it is.
fn node_boundary_clamp(p: Point, node: Geo) -> Point {
    let cx = node.x;
    let cy = node.y;
    let w = node.width.unwrap_or(0.0);
    let h = node.height.unwrap_or(0.0);
    if w <= 0.0 || h <= 0.0 {
        return p;
    }
    let left = cx - w / 2.0;
    let right = cx + w / 2.0;
    let top = cy - h / 2.0;
    let bottom = cy + h / 2.0;
    if p.x < left || p.x > right || p.y < top || p.y > bottom {
        return p;
    }
    let d_left = p.x - left;
    let d_right = right - p.x;
    let d_top = p.y - top;
    let d_bottom = bottom - p.y;
    let min_d = js_min(js_min(js_min(d_left, d_right), d_top), d_bottom);
    if min_d == d_left {
        Point { x: left, y: p.y }
    } else if min_d == d_right {
        Point { x: right, y: p.y }
    } else if min_d == d_top {
        Point { x: p.x, y: top }
    } else {
        Point { x: p.x, y: bottom }
    }
}

impl Router {
    /// The geometry of phase 3: the track coordinates, then the points of each edge.
    pub(super) fn rebuild_geometry(&mut self) {
        let segment_coords = self.assign_track_coordinates();
        self.rebuild_points(&segment_coords);
    }

    /// The coordinates of the segments on their tracks (`router.ts:2010-2122`), by the cluster of each pipe.
    /// Returns the coordinate of each segment, by its edge and its position in the edge's route.
    fn assign_track_coordinates(&mut self) -> HashMap<(usize, usize), f64> {
        let mut segment_coords: HashMap<(usize, usize), f64> = HashMap::new();
        for pipe_index in 0..self.pipes.len() {
            let pipe_coord = self.pipes[pipe_index].coord;
            let mut segments: Vec<SegmentInfo> = Vec::new();
            for (track_index, track) in self.pipes[pipe_index].tracks.iter().enumerate() {
                for s in track {
                    segments.push(SegmentInfo {
                        edge: s.edge,
                        segment: s.segment,
                        track: track_index,
                        from: s.from,
                        to: s.to,
                    });
                }
            }
            js_sort_by(&mut segments, |a, b| a.from - b.from);

            // The clusters: a segment that starts before the end of the cluster joins it.
            let mut clusters: Vec<Vec<SegmentInfo>> = Vec::new();
            if let Some(&first) = segments.first() {
                let mut current = vec![first];
                let mut cluster_end = first.to;
                for &s in &segments[1..] {
                    if s.from < cluster_end {
                        current.push(s);
                        cluster_end = js_max(cluster_end, s.to);
                    } else {
                        clusters.push(current);
                        current = vec![s];
                        cluster_end = s.to;
                    }
                }
                clusters.push(current);
            }

            for cluster in &clusters {
                self.assign_cluster_coordinates(cluster, pipe_coord, &mut segment_coords);
            }
        }
        segment_coords
    }

    /// The coordinates of the tracks of one cluster (`router.ts:2056-2121`).
    fn assign_cluster_coordinates(
        &mut self,
        cluster: &[SegmentInfo],
        pipe_coord: f64,
        segment_coords: &mut HashMap<(usize, usize), f64>,
    ) {
        let mut used_tracks: IndexSet<usize> = IndexSet::new();
        for s in cluster {
            used_tracks.insert(s.track);
        }
        let mut track_scores: HashMap<usize, f64> = HashMap::new();
        for s in cluster {
            let info = self.dest_info(s.edge);
            let score = track_scores.get(&s.track).copied().unwrap_or(0.0) + info.delta;
            track_scores.insert(s.track, score);
        }
        let score = |track: usize| track_scores.get(&track).copied().unwrap_or(0.0);

        let mut left_tracks: Vec<usize> = used_tracks
            .iter()
            .copied()
            .filter(|&t| score(t) < -1.0)
            .collect();
        let mut right_tracks: Vec<usize> = used_tracks
            .iter()
            .copied()
            .filter(|&t| score(t) > 1.0)
            .collect();
        let mut neutral_tracks: Vec<usize> = used_tracks
            .iter()
            .copied()
            .filter(|&t| score(t).abs() <= 1.0)
            .collect();

        js_sort_by(&mut left_tracks, |a, b| score(b) - score(a));
        js_sort_by(&mut right_tracks, |a, b| score(a) - score(b));

        let mut left_count = 0;
        for &track in &left_tracks {
            left_count += 1;
            let coord = pipe_coord - left_count as f64 * TRACK_SPACING;
            self.assign_track_coord(cluster, track, coord, pipe_coord, segment_coords);
        }

        // Without a neutral track, the track nearest the centre becomes the neutral one.
        if neutral_tracks.is_empty() && !used_tracks.is_empty() {
            let mut ordered: Vec<usize> = used_tracks.iter().copied().collect();
            js_sort_by(&mut ordered, |a, b| score(a).abs() - score(b).abs());
            let best_track = ordered[0];
            if let Some(index) = left_tracks.iter().position(|&t| t == best_track) {
                left_tracks.remove(index);
            }
            if let Some(index) = right_tracks.iter().position(|&t| t == best_track) {
                right_tracks.remove(index);
            }
            neutral_tracks.push(best_track);
        }

        // The neutral tracks take the centre, then alternate to either side, each further one half a spacing out.
        for (neutral_assigned, &track) in neutral_tracks.iter().enumerate() {
            if neutral_assigned == 0 {
                self.assign_track_coord(cluster, track, pipe_coord, pipe_coord, segment_coords);
            } else {
                let dir = if neutral_assigned % 2 == 1 { 1.0 } else { -1.0 };
                let magnitude = (neutral_assigned as f64 / 2.0).ceil();
                let coord = pipe_coord + dir * magnitude * TRACK_SPACING * 0.5;
                self.assign_track_coord(cluster, track, coord, pipe_coord, segment_coords);
            }
        }

        let mut right_count = 0;
        for &track in &right_tracks {
            right_count += 1;
            let coord = pipe_coord + right_count as f64 * TRACK_SPACING;
            self.assign_track_coord(cluster, track, coord, pipe_coord, segment_coords);
        }
    }

    /// `assignCoord(trackIndex, coord)` (`router.ts:2072-2080`): the segments of the cluster on the track take the
    /// coordinate, except the straight fast-path edges, which keep the pipe's.
    fn assign_track_coord(
        &self,
        cluster: &[SegmentInfo],
        track: usize,
        coord: f64,
        pipe_coord: f64,
        segment_coords: &mut HashMap<(usize, usize), f64>,
    ) {
        for s in cluster.iter().filter(|s| s.track == track) {
            let effective = if self.straight_intra_lane_edges.contains(&s.edge) {
                pipe_coord
            } else {
                coord
            };
            segment_coords.insert((s.edge, s.segment), effective);
        }
    }

    /// The points of each routed edge (`router.ts:2128-2205`).
    fn rebuild_points(&mut self, segment_coords: &HashMap<(usize, usize), f64>) {
        for index in 0..self.edges.len() {
            let indices = self.edge_segments[index].clone();
            if indices.is_empty() {
                continue;
            }
            let (Some(start), Some(end)) = (
                self.edges[index].start.clone(),
                self.edges[index].end.clone(),
            ) else {
                continue;
            };
            let (Some(src), Some(dst)) = (self.geo(&start), self.geo(&end)) else {
                continue;
            };
            let ports = self.ports_for_edge(index, src, dst);
            let lines: Vec<RoutedLine> = indices
                .iter()
                .map(|&segment| {
                    let s = self.segments[segment];
                    RoutedLine {
                        orientation: s.orientation,
                        coord: segment_coords
                            .get(&(s.edge, s.segment))
                            .copied()
                            .unwrap_or(s.pipe_coord),
                        from: s.from,
                        to: s.to,
                    }
                })
                .collect();
            self.edges[index].points = Some(rebuild_route(ports.src_port, ports.dst_port, &lines));
        }
    }

    /// The end of `routeEdgesOrthogonal` (`router.ts:2210-2282`): the points of the edges are written to the
    /// layout, the layout-only edges are removed, and each endpoint of an edge is clamped to the boundary of its
    /// node.
    pub(super) fn write_back(&self, data: &mut LayoutData) {
        for edge in &self.edges {
            if let Some(points) = &edge.points {
                data.edges[edge.origin].points = Some(points.clone());
            }
        }

        data.edges.retain(|edge| edge.is_layout_only != Some(true));

        for edge in &mut data.edges {
            let src = edge
                .start
                .as_deref()
                .filter(|id| !id.is_empty())
                .and_then(|id| self.geo(id));
            let dst = edge
                .end
                .as_deref()
                .filter(|id| !id.is_empty())
                .and_then(|id| self.geo(id));
            let Some(points) = edge.points.as_mut() else {
                continue;
            };
            if points.len() < 2 {
                continue;
            }
            if let Some(src) = src {
                points[0] = node_boundary_clamp(points[0], src);
            }
            if let Some(dst) = dst {
                let last = points.len() - 1;
                points[last] = node_boundary_clamp(points[last], dst);
            }
        }
    }
}
