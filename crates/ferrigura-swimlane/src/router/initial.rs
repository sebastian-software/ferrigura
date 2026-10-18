//! The initial routing of the edges (`routeEdgesOrthogonal`, phase 1): the routing order (332-368), and for each
//! edge the anchors (766-800), the obstacle detours of the anchors (802-935), the straight fast path (937-1061),
//! the pipes of the anchors and the grid around the obstacles (1063-1124), the search (in `search`), and the
//! segments of the route (1497-1611).
//!
//! The reference routes the edges in the order of `routingOrder`, and each edge sees the segments of the edges
//! before it. The port assignment and the search read and write the same state in the same order.

use crate::math::{js_max, js_min};
use crate::model::Point;

use super::js_sort::sort_by_js;
use super::ports::EdgePorts;
use super::search::Endpoints;
use super::{
    ANCHOR_OFFSET, EPS, Geo, HORIZONTAL_PIPE_MARGIN, Obstacle, Orientation, ROUTING_MARGIN, Role,
    RoutedSegment, Router, SegmentRef, VERTICAL_PIPE_MARGIN, js_sign,
};

/// An entry of the routing order: the edge, whether it crosses lanes, and the distance between its nodes.
#[derive(Clone, Copy)]
struct OrderEntry {
    index: usize,
    cross_lane: f64,
    dx: f64,
    dy: f64,
}

/// The exit point of an obstacle detour (`obstacleDetour`'s result): the point beyond the obstacle, and whether
/// the port is on the positive side of its node, which decides the direction of the step.
#[derive(Clone, Copy)]
struct Detour {
    x: f64,
    y: f64,
    leaves_positive_side: bool,
}

/// `obstacleDetour(port, node, opposite, obs, portIsVertical)` (`router.ts:818-844`): where an anchor that is
/// inside an obstacle is pushed to, beyond the obstacle on the side that the port leaves by.
fn obstacle_detour(
    port: Point,
    node: Geo,
    opposite: Geo,
    obs: &Obstacle,
    port_is_vertical: bool,
) -> Detour {
    if port_is_vertical {
        let leaves_positive_side = port.y > node.y;
        let go_right = opposite.x >= port.x;
        return Detour {
            x: if go_right {
                obs.max_x + HORIZONTAL_PIPE_MARGIN
            } else {
                obs.min_x - HORIZONTAL_PIPE_MARGIN
            },
            y: if leaves_positive_side {
                obs.max_y + VERTICAL_PIPE_MARGIN
            } else {
                obs.min_y - VERTICAL_PIPE_MARGIN
            },
            leaves_positive_side,
        };
    }

    let leaves_positive_side = port.x > node.x;
    let go_down = opposite.y >= port.y;
    Detour {
        x: if leaves_positive_side {
            obs.max_x + HORIZONTAL_PIPE_MARGIN
        } else {
            obs.min_x - HORIZONTAL_PIPE_MARGIN
        },
        y: if go_down {
            obs.max_y + VERTICAL_PIPE_MARGIN
        } else {
            obs.min_y - VERTICAL_PIPE_MARGIN
        },
        leaves_positive_side,
    }
}

impl Router {
    /// `routingOrder` (`router.ts:332-368`): the cross-lane edges first, then the shorter edges, then the edge
    /// index. The comparator's threshold of one unit makes it an order that V8's sort decides on as it goes.
    pub(super) fn routing_order(&self) -> Vec<usize> {
        let mut entries: Vec<OrderEntry> = (0..self.edges.len())
            .map(|index| {
                let edge = &self.edges[index];
                let (Some(start), Some(end)) = (edge.start.as_deref(), edge.end.as_deref()) else {
                    return OrderEntry {
                        index,
                        cross_lane: 0.0,
                        dx: 0.0,
                        dy: 0.0,
                    };
                };
                let src_lane = self.lane_by_node_id.get(start);
                let dst_lane = self.lane_by_node_id.get(end);
                let cross_lane = if src_lane.is_some() && dst_lane.is_some() && src_lane != dst_lane
                {
                    1.0
                } else {
                    0.0
                };
                let (dx, dy) = match (self.geo(start), self.geo(end)) {
                    (Some(src), Some(dst)) => ((dst.x - src.x).abs(), (dst.y - src.y).abs()),
                    _ => (0.0, 0.0),
                };
                OrderEntry {
                    index,
                    cross_lane,
                    dx,
                    dy,
                }
            })
            .collect();
        sort_by_js(&mut entries, |a, b| {
            if a.cross_lane != b.cross_lane {
                return b.cross_lane - a.cross_lane;
            }
            let a_distance = a.dx + a.dy;
            let b_distance = b.dx + b.dy;
            if (a_distance - b_distance).abs() > 1.0 {
                return a_distance - b_distance;
            }
            a.index as f64 - b.index as f64
        });
        entries.into_iter().map(|entry| entry.index).collect()
    }

    /// The routing of one edge (`router.ts:735-1612`), in the order of the routing. An edge with a missing
    /// endpoint, a self-loop, or a node that is not in the layout is not routed.
    pub(super) fn route_edge(&mut self, index: usize) {
        let (Some(start), Some(end)) = (
            self.edges[index].start.clone(),
            self.edges[index].end.clone(),
        ) else {
            return;
        };
        if self.edges[index]
            .points
            .as_ref()
            .is_some_and(|points| !points.is_empty())
        {
            return;
        }
        if start == end {
            return;
        }
        let (Some(src), Some(dst)) = (self.geo(&start), self.geo(&end)) else {
            return;
        };
        let ends = Endpoints {
            edge: index,
            start: &start,
            end: &end,
        };

        // 2. The ports, from the side assignment (lines 756-764).
        let ports = self.ports_for_edge(index, src, dst);

        // 3. The anchors: 20 units out of each port, along the axis of its side (lines 766-800).
        let mut src_anchor = ports.src_port;
        let mut dst_anchor = ports.dst_port;
        let src_vertical = ports.src_side.is_vertical();
        let dst_vertical = ports.dst_side.is_vertical();
        if src_vertical {
            let is_bottom = ports.src_port.y > src.y;
            src_anchor.y = if is_bottom {
                ports.src_port.y + ANCHOR_OFFSET
            } else {
                ports.src_port.y - ANCHOR_OFFSET
            };
        } else {
            let is_right = ports.src_port.x > src.x;
            src_anchor.x = if is_right {
                ports.src_port.x + ANCHOR_OFFSET
            } else {
                ports.src_port.x - ANCHOR_OFFSET
            };
        }
        if dst_vertical {
            let is_bottom = ports.dst_port.y > dst.y;
            dst_anchor.y = if is_bottom {
                ports.dst_port.y + ANCHOR_OFFSET
            } else {
                ports.dst_port.y - ANCHOR_OFFSET
            };
        } else {
            let is_right = ports.dst_port.x > dst.x;
            dst_anchor.x = if is_right {
                ports.dst_port.x + ANCHOR_OFFSET
            } else {
                ports.dst_port.x - ANCHOR_OFFSET
            };
        }

        // An anchor inside an obstacle is pushed out, with the waypoints that lead round it (lines 855-935).
        let mut src_handle: Vec<Point> = Vec::new();
        if let Some(obs) = self.obstacle_at(src_anchor, &start, &end) {
            if src_vertical {
                let detour = obstacle_detour(ports.src_port, src, dst, &obs, true);
                src_anchor.x = detour.x;
                src_anchor.y = detour.y;
                let gap_y = if detour.leaves_positive_side {
                    js_min(obs.min_y - 2.0, ports.src_port.y + ANCHOR_OFFSET)
                } else {
                    js_max(obs.max_y + 2.0, ports.src_port.y - ANCHOR_OFFSET)
                };
                src_handle = vec![
                    Point {
                        x: ports.src_port.x,
                        y: gap_y,
                    },
                    Point {
                        x: detour.x,
                        y: gap_y,
                    },
                    Point {
                        x: detour.x,
                        y: detour.y,
                    },
                ];
            } else {
                let detour = obstacle_detour(ports.src_port, src, dst, &obs, false);
                let gap_x = if detour.leaves_positive_side {
                    js_min(obs.min_x - 2.0, ports.src_port.x + ANCHOR_OFFSET)
                } else {
                    js_max(obs.max_x + 2.0, ports.src_port.x - ANCHOR_OFFSET)
                };
                src_anchor.x = detour.x;
                src_anchor.y = detour.y;
                src_handle = vec![
                    Point {
                        x: gap_x,
                        y: ports.src_port.y,
                    },
                    Point {
                        x: gap_x,
                        y: detour.y,
                    },
                    Point {
                        x: detour.x,
                        y: detour.y,
                    },
                ];
            }
        }

        let mut dst_handle: Vec<Point> = Vec::new();
        if let Some(obs) = self.obstacle_at(dst_anchor, &start, &end) {
            if dst_vertical {
                let detour = obstacle_detour(ports.dst_port, dst, src, &obs, true);
                dst_anchor.x = detour.x;
                dst_anchor.y = detour.y;
                dst_handle = vec![
                    Point {
                        x: detour.x,
                        y: detour.y,
                    },
                    Point {
                        x: ports.dst_port.x,
                        y: detour.y,
                    },
                ];
            } else {
                let detour = obstacle_detour(ports.dst_port, dst, src, &obs, false);
                dst_anchor.x = detour.x;
                dst_anchor.y = detour.y;
                dst_handle = vec![
                    Point {
                        x: detour.x,
                        y: detour.y,
                    },
                    Point {
                        x: detour.x,
                        y: ports.dst_port.y,
                    },
                ];
            }
        }

        // The straight fast path: the ports are aligned and the direct segment is free (lines 978-1061).
        if src_handle.is_empty()
            && dst_handle.is_empty()
            && self.straight_path(index, ends, &ports, src_anchor, dst_anchor)
        {
            return;
        }

        // The anchors snap to the pipes of their columns (lines 1063-1067).
        let src_pipe = self.get_or_add_pipe(Orientation::Vertical, src_anchor.x);
        src_anchor.x = self.pipes[src_pipe].coord;
        let dst_pipe = self.get_or_add_pipe(Orientation::Vertical, dst_anchor.x);
        dst_anchor.x = self.pipes[dst_pipe].coord;

        // The bounding box of the search, with the room for the obstacles on the path (lines 1070-1094).
        let obstacles = self.obstacles.clone();
        let mut bb_min_x = js_min(src_anchor.x, dst_anchor.x) - 50.0;
        let mut bb_max_x = js_max(src_anchor.x, dst_anchor.x) + 50.0;
        let mut bb_min_y = js_min(src_anchor.y, dst_anchor.y) - 50.0;
        let mut bb_max_y = js_max(src_anchor.y, dst_anchor.y) + 50.0;
        let path_min_x = js_min(src_anchor.x, dst_anchor.x);
        let path_max_x = js_max(src_anchor.x, dst_anchor.x);
        let path_min_y = js_min(src_anchor.y, dst_anchor.y);
        let path_max_y = js_max(src_anchor.y, dst_anchor.y);
        for &obs in &obstacles {
            let obs_blocks_path = obs.min_x < path_max_x
                && obs.max_x > path_min_x
                && obs.min_y < path_max_y
                && obs.max_y > path_min_y;
            if obs_blocks_path {
                bb_min_x = js_min(bb_min_x, obs.min_x - ROUTING_MARGIN);
                bb_max_x = js_max(bb_max_x, obs.max_x + ROUTING_MARGIN);
                bb_min_y = js_min(bb_min_y, obs.min_y - ROUTING_MARGIN);
                bb_max_y = js_max(bb_max_y, obs.max_y + ROUTING_MARGIN);
            }
        }

        // The pipes around the obstacles, at the safe distances, which are never on the boundary (1097-1112).
        for &obs in &obstacles {
            if obs.max_x < bb_min_x
                || obs.min_x > bb_max_x
                || obs.max_y < bb_min_y
                || obs.min_y > bb_max_y
            {
                continue;
            }
            self.get_or_add_pipe(Orientation::Horizontal, obs.min_y - HORIZONTAL_PIPE_MARGIN);
            self.get_or_add_pipe(Orientation::Horizontal, obs.max_y + HORIZONTAL_PIPE_MARGIN);
            self.get_or_add_pipe(Orientation::Vertical, obs.min_x - VERTICAL_PIPE_MARGIN);
            self.get_or_add_pipe(Orientation::Vertical, obs.max_x + VERTICAL_PIPE_MARGIN);
        }
        // The horizontal pipes of the two anchors (lines 1114-1116).
        self.get_or_add_pipe(Orientation::Horizontal, src_anchor.y);
        self.get_or_add_pipe(Orientation::Horizontal, dst_anchor.y);

        // The pipes in the box, in the order of creation (lines 1119-1124).
        let mut h_pipes: Vec<usize> = (0..self.pipes.len())
            .filter(|&p| {
                let pipe = &self.pipes[p];
                pipe.orientation == Orientation::Horizontal
                    && pipe.coord >= bb_min_y
                    && pipe.coord <= bb_max_y
            })
            .collect();
        let mut v_pipes: Vec<usize> = (0..self.pipes.len())
            .filter(|&p| {
                let pipe = &self.pipes[p];
                pipe.orientation == Orientation::Vertical
                    && pipe.coord >= bb_min_x
                    && pipe.coord <= bb_max_x
            })
            .collect();

        // The search: the L-shaped candidates, then A* (lines 1127-1324).
        let mut found = self.find_path(ends, src_anchor, dst_anchor, &mut h_pipes, &mut v_pipes);

        // The simplification of a long path (lines 1329-1493).
        if found.len() > 4
            && let Some(simplified) = self.simplify_path(ends, &found)
        {
            found = simplified;
        }

        // 5. The full path, with its handles and its ports, and the collapse of the overshoots (1497-1536).
        let mut full: Vec<Point> = Vec::with_capacity(found.len() + 4);
        full.push(ports.src_port);
        full.extend(src_handle);
        full.extend(found);
        let mut dst_handle_reversed = dst_handle;
        dst_handle_reversed.reverse();
        full.extend(dst_handle_reversed);
        full.push(ports.dst_port);
        remove_overshoot(&mut full);

        // The collinear points are dropped, except where the direction reverses (lines 1538-1576).
        let simplified = collapse_collinear(&full);
        self.add_route_segments(index, &simplified);
    }

    /// The straight fast path (`router.ts:978-1061`): when the anchors are aligned and the direct segment between
    /// the ports is free, the edge is the four points of the straight line, and it is not routed further. Returns
    /// whether the fast path took the edge.
    fn straight_path(
        &mut self,
        index: usize,
        ends: Endpoints<'_>,
        ports: &EdgePorts,
        src_anchor: Point,
        dst_anchor: Point,
    ) -> bool {
        let (start, end) = (ends.start, ends.end);
        let hp_margin = HORIZONTAL_PIPE_MARGIN;
        let anchors_same_x = (src_anchor.x - dst_anchor.x).abs() < hp_margin;
        let anchors_same_y = (src_anchor.y - dst_anchor.y).abs() < hp_margin;
        let has_port_offset = self.port_offsets.contains_key(&(index, Role::Src))
            || self.port_offsets.contains_key(&(index, Role::Dst));
        let src_face_total = self.port_group_len(start, ports.src_side, Role::Src)
            + self.port_group_len(start, ports.src_side, Role::Dst);
        let dst_face_total = self.port_group_len(end, ports.dst_side, Role::Src)
            + self.port_group_len(end, ports.dst_side, Role::Dst);
        let face_contested = src_face_total > 1 || dst_face_total > 1;
        let src_incident_total = self.incident_totals.get(start).copied().unwrap_or(0);
        let dst_incident_total = self.incident_totals.get(end).copied().unwrap_or(0);
        let contested_face_has_label = (src_face_total > 1
            && self.face_has_label_node(Some(start), ports.src_side))
            || (dst_face_total > 1 && self.face_has_label_node(Some(end), ports.dst_side));
        let src_contest_allows = src_face_total <= 1 || src_incident_total <= 2;
        let dst_contest_allows = dst_face_total <= 1 || dst_incident_total <= 2;
        let can_preserve_simple_contested_straight =
            face_contested && !contested_face_has_label && src_contest_allows && dst_contest_allows;
        if !((anchors_same_x || anchors_same_y)
            && !has_port_offset
            && (!face_contested || can_preserve_simple_contested_straight))
        {
            return false;
        }
        if self.is_segment_blocked(ports.src_port, ports.dst_port, Some(start), Some(end)) {
            return false;
        }

        let points = vec![ports.src_port, src_anchor, dst_anchor, ports.dst_port];
        self.edges[index].points = Some(points);
        self.straight_intra_lane_edges.insert(index);
        // The full port-to-port line is a routed segment for later searches, but not a segment of the edge.
        let (orientation, coord, from, to) = if anchors_same_y {
            (
                Orientation::Horizontal,
                ports.src_port.y,
                js_min(ports.src_port.x, ports.dst_port.x),
                js_max(ports.src_port.x, ports.dst_port.x),
            )
        } else {
            (
                Orientation::Vertical,
                ports.src_port.x,
                js_min(ports.src_port.y, ports.dst_port.y),
                js_max(ports.src_port.y, ports.dst_port.y),
            )
        };
        self.segments.push(RoutedSegment {
            edge: index,
            segment: 0,
            orientation,
            pipe: None,
            pipe_coord: coord,
            track: 0,
            from,
            to,
        });
        true
    }

    /// The segments of a route (`router.ts:1579-1611`): one per pair of consecutive points, on the pipe of its
    /// line. A pipe's first track holds the segments in the order of the edges.
    fn add_route_segments(&mut self, index: usize, points: &[Point]) {
        for k in 0..points.len().saturating_sub(1) {
            let p1 = points[k];
            let p2 = points[k + 1];
            let orientation = if (p1.x - p2.x).abs() < EPS {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            };
            let coord = if orientation == Orientation::Vertical {
                p1.x
            } else {
                p1.y
            };
            let (from, to) = if orientation == Orientation::Vertical {
                (js_min(p1.y, p2.y), js_max(p1.y, p2.y))
            } else {
                (js_min(p1.x, p2.x), js_max(p1.x, p2.x))
            };
            let pipe = self.get_or_add_pipe(orientation, coord);

            self.segments.push(RoutedSegment {
                edge: index,
                segment: k,
                orientation,
                pipe: Some(pipe),
                pipe_coord: self.pipes[pipe].coord,
                track: 0,
                from,
                to,
            });
            let segment_index = self.segments.len() - 1;
            self.edge_segments[index].push(segment_index);

            // The first track of the pipe holds the segments of the route, in the order they were routed.
            if self.pipes[pipe].tracks.is_empty() {
                self.pipes[pipe].tracks.push(Vec::new());
            }
            self.pipes[pipe].tracks[0].push(SegmentRef {
                edge: index,
                segment: k,
                from,
                to,
            });
        }
    }
}

/// The overshoot at the end of the path (`router.ts:1508-1536`): when the last three points are collinear and
/// the middle one goes past the last, the middle one is removed.
fn remove_overshoot(full: &mut Vec<Point>) {
    let n = full.len();
    if n < 3 {
        return;
    }
    let c = full[n - 1];
    let b = full[n - 2];
    let a = full[n - 3];

    let is_horizontal = (a.y - b.y).abs() < EPS && (b.y - c.y).abs() < EPS;
    let is_vertical = (a.x - b.x).abs() < EPS && (b.x - c.x).abs() < EPS;
    if is_horizontal {
        let sign_ab = js_sign(b.x - a.x);
        let sign_ac = js_sign(c.x - a.x);
        if sign_ab != 0.0 && sign_ab == sign_ac && (b.x - a.x).abs() > (c.x - a.x).abs() {
            full.remove(n - 2);
        }
    } else if is_vertical {
        let sign_ab = js_sign(b.y - a.y);
        let sign_ac = js_sign(c.y - a.y);
        if sign_ab != 0.0 && sign_ab == sign_ac && (b.y - a.y).abs() > (c.y - a.y).abs() {
            full.remove(n - 2);
        }
    }
}

/// The collinear points of a path are dropped, except that the second point is kept, and a point where the
/// direction reverses is kept (`router.ts:1538-1576`). The first and last points are always kept.
fn collapse_collinear(full: &[Point]) -> Vec<Point> {
    let mut simplified: Vec<Point> = vec![full[0]];
    for k in 1..full.len().saturating_sub(1) {
        if k == 1 {
            simplified.push(full[k]);
            continue;
        }
        let prev = simplified[simplified.len() - 1];
        let curr = full[k];
        let next = full[k + 1];
        if (prev.y - curr.y).abs() < EPS && (curr.y - next.y).abs() < EPS {
            let dir1 = curr.x > prev.x;
            let dir2 = next.x > curr.x;
            if dir1 != dir2 {
                simplified.push(curr);
            }
            continue;
        }
        if (prev.x - curr.x).abs() < EPS && (curr.x - next.x).abs() < EPS {
            let dir1 = curr.y > prev.y;
            let dir2 = next.y > curr.y;
            if dir1 != dir2 {
                simplified.push(curr);
            }
            continue;
        }
        simplified.push(curr);
    }
    simplified.push(full[full.len() - 1]);
    simplified
}
