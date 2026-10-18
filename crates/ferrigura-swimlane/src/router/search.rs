//! The search of the route between the anchors of an edge (`routeEdgesOrthogonal`, lines 1152-1324): the crossing
//! penalty (278-327), the L-shaped candidates (1157-1189) and the A* search over the crossings of the pipes
//! (1191-1320), and the simplification of a long path (1329-1493).

use std::collections::{HashMap, HashSet};

use ferrigura_jsmath::hypot;

use crate::js::to_fixed;
use crate::math::{js_max, js_min};
use crate::model::Point;

use super::js_sort::sort_by_js;
use super::{
    CROSSING_PENALTY, EPS, HORIZONTAL_PIPE_MARGIN, Obstacle, Orientation, Router,
    VERTICAL_PIPE_MARGIN,
};

/// The edge that a search routes, and the ids of its two nodes (`e.start`, `e.end`).
#[derive(Clone, Copy)]
pub(super) struct Endpoints<'a> {
    pub(super) edge: usize,
    pub(super) start: &'a str,
    pub(super) end: &'a str,
}

/// The direction in which a search step reached a vertex (`arrivalDir`): `h` and `v` in the reference, and `n`
/// for the start, which has no arrival.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arrival {
    Horizontal,
    Vertical,
    Start,
}

/// An entry of the open list (`openList`): the key of the vertex, its estimate, and the point.
#[derive(Clone, Copy)]
struct OpenEntry {
    key: usize,
    f: f64,
    point: Point,
}

/// The vertices of the search. The reference keys them by the text `${x.toFixed(1)},${y.toFixed(1)}`, so two
/// points with the same text are one vertex. Each text is given an index here, which keeps that identity.
#[derive(Default)]
struct Keys {
    ids: HashMap<String, usize>,
}

impl Keys {
    /// `getKey(x, y)`: the index of the vertex with the text of the point.
    fn id(&mut self, point: Point) -> usize {
        let text = format!("{},{}", to_fixed(point.x, 1), to_fixed(point.y, 1));
        let next = self.ids.len();
        *self.ids.entry(text).or_insert(next)
    }
}

impl Router {
    /// `checkSegmentBlocked(p1, p2)` (`router.ts:1153-1155`): the blocking test with the endpoints of the edge.
    fn blocked(&self, p1: Point, p2: Point, ends: Endpoints<'_>) -> bool {
        self.is_segment_blocked(p1, p2, Some(ends.start), Some(ends.end))
    }

    /// `crossingPenalty(edgeIdx, from, to)` (`router.ts:278-327`): the penalty of a horizontal or vertical step,
    /// for each routed segment of another edge that the step crosses.
    pub(super) fn crossing_penalty(&self, edge: usize, from: Point, to: Point) -> f64 {
        if self.segments.is_empty() {
            return 0.0;
        }
        let is_horizontal = (from.y - to.y).abs() < EPS;
        let is_vertical = (from.x - to.x).abs() < EPS;
        if !is_horizontal && !is_vertical {
            return 0.0;
        }

        let mut penalties = 0.0;
        if is_horizontal {
            let y = from.y;
            let min_x = js_min(from.x, to.x) - EPS;
            let max_x = js_max(from.x, to.x) + EPS;
            if max_x <= min_x {
                return 0.0;
            }
            for seg in &self.segments {
                if seg.edge == edge || seg.orientation != Orientation::Vertical {
                    continue;
                }
                if seg.pipe_coord < min_x || seg.pipe_coord > max_x {
                    continue;
                }
                if seg.from - EPS <= y && seg.to + EPS >= y {
                    penalties += CROSSING_PENALTY;
                }
            }
        } else if is_vertical {
            let x = from.x;
            let min_y = js_min(from.y, to.y) - EPS;
            let max_y = js_max(from.y, to.y) + EPS;
            if max_y <= min_y {
                return 0.0;
            }
            for seg in &self.segments {
                if seg.edge == edge || seg.orientation != Orientation::Horizontal {
                    continue;
                }
                if seg.pipe_coord < min_y || seg.pipe_coord > max_y {
                    continue;
                }
                if seg.from - EPS <= x && seg.to + EPS >= x {
                    penalties += CROSSING_PENALTY;
                }
            }
        }
        penalties
    }

    /// The path between the anchors `src` and `dst` (lines 1152-1324): the L-shaped candidates, the search, and a
    /// three-point fallback that goes vertically then horizontally when the search finds nothing.
    /// `h_pipes` and `v_pipes` are the pipes in the box, and the search sorts them in place, as the reference does.
    pub(super) fn find_path(
        &self,
        ends: Endpoints<'_>,
        src: Point,
        dst: Point,
        h_pipes: &mut [usize],
        v_pipes: &mut [usize],
    ) -> Vec<Point> {
        let mut found: Vec<Point> = Vec::new();

        // Option 1: horizontal first, then vertical (lines 1158-1162).
        let corner_hv = Point { x: dst.x, y: src.y };
        let path_hv_blocked =
            self.blocked(src, corner_hv, ends) || self.blocked(corner_hv, dst, ends);
        // Option 2: vertical first, then horizontal (lines 1164-1168).
        let corner_vh = Point { x: src.x, y: dst.y };
        let path_vh_blocked =
            self.blocked(src, corner_vh, ends) || self.blocked(corner_vh, dst, ends);

        if !path_hv_blocked {
            // The same y or the same x is a straight line, and the corner would be a duplicate point.
            found = if (src.y - dst.y).abs() < EPS || (src.x - dst.x).abs() < EPS {
                vec![src, dst]
            } else {
                vec![src, corner_hv, dst]
            };
        } else if !path_vh_blocked {
            found = if (src.x - dst.x).abs() < EPS {
                vec![src, dst]
            } else {
                vec![src, corner_vh, dst]
            };
        }

        if found.is_empty() {
            found = self.astar(ends, src, dst, h_pipes, v_pipes);
        }
        if found.is_empty() {
            found = vec![src, Point { x: src.x, y: dst.y }, dst];
        }
        found
    }

    /// The A* search over the intersections of the pipes (lines 1127-1320). The cost of a step is its length,
    /// the crossing penalty, the penalties for a move away from the destination, and 50 for a bend. Returns the
    /// path from the source to the destination, or an empty path when the open list runs out.
    fn astar(
        &self,
        ends: Endpoints<'_>,
        src: Point,
        dst: Point,
        h_pipes: &mut [usize],
        v_pipes: &mut [usize],
    ) -> Vec<Point> {
        let mut keys = Keys::default();
        let start_key = keys.id(src);
        let end_key = keys.id(dst);

        let mut g_score: HashMap<usize, f64> = HashMap::new();
        let mut came_from: HashMap<usize, Point> = HashMap::new();
        let mut arrival: HashMap<usize, Arrival> = HashMap::new();
        let mut open_set: HashSet<usize> = HashSet::new();
        let mut open_list: Vec<OpenEntry> = Vec::new();

        g_score.insert(start_key, 0.0);
        arrival.insert(start_key, Arrival::Start);
        open_list.push(OpenEntry {
            key: start_key,
            f: hypot(dst.x - src.x, dst.y - src.y),
            point: src,
        });
        open_set.insert(start_key);

        let mut found: Vec<Point> = Vec::new();
        while !open_list.is_empty() {
            sort_by_js(&mut open_list, |a, b| a.f - b.f);
            let current = open_list.remove(0);
            open_set.remove(&current.key);

            if current.key == end_key {
                // Reconstruct the path: each vertex's predecessor, back from the destination.
                found = vec![dst];
                let mut curr_key = end_key;
                while let Some(&prev) = came_from.get(&curr_key) {
                    found.insert(0, prev);
                    curr_key = keys.id(prev);
                }
                break;
            }

            let cx = current.point.x;
            let cy = current.point.y;

            // The pipes next to the vertex, along its horizontal and vertical pipes (lines 1217-1221).
            sort_by_js(v_pipes, |a, b| self.pipes[a].coord - self.pipes[b].coord);
            let v_idx = v_pipes
                .iter()
                .position(|&p| (self.pipes[p].coord - cx).abs() < 1.0)
                .map_or(-1, |index| index as isize);
            sort_by_js(h_pipes, |a, b| self.pipes[a].coord - self.pipes[b].coord);
            let h_idx = h_pipes
                .iter()
                .position(|&p| (self.pipes[p].coord - cy).abs() < 1.0)
                .map_or(-1, |index| index as isize);

            let mut neighbors: Vec<Point> = Vec::new();
            if v_idx > 0 {
                neighbors.push(Point {
                    x: self.pipes[v_pipes[(v_idx - 1) as usize]].coord,
                    y: cy,
                });
            }
            if v_idx >= 0 && v_idx < v_pipes.len() as isize - 1 {
                neighbors.push(Point {
                    x: self.pipes[v_pipes[(v_idx + 1) as usize]].coord,
                    y: cy,
                });
            }
            if h_idx > 0 {
                neighbors.push(Point {
                    x: cx,
                    y: self.pipes[h_pipes[(h_idx - 1) as usize]].coord,
                });
            }
            if h_idx >= 0 && h_idx < h_pipes.len() as isize - 1 {
                neighbors.push(Point {
                    x: cx,
                    y: self.pipes[h_pipes[(h_idx + 1) as usize]].coord,
                });
            }

            for neighbor in neighbors {
                let min_x = js_min(cx, neighbor.x);
                let max_x = js_max(cx, neighbor.x);
                let min_y = js_min(cy, neighbor.y);
                let max_y = js_max(cy, neighbor.y);

                // The step is blocked by an obstacle that is not an endpoint (lines 1248-1260).
                let is_blocked = self.obstacles.iter().any(|obs: &Obstacle| {
                    let id = self.nodes[obs.node].id.as_str();
                    if id == ends.start || id == ends.end {
                        return false;
                    }
                    if min_x != max_x {
                        obs.min_y < cy && obs.max_y > cy && obs.max_x > min_x && obs.min_x < max_x
                    } else {
                        obs.min_x < cx && obs.max_x > cx && obs.max_y > min_y && obs.min_y < max_y
                    }
                });
                if is_blocked {
                    continue;
                }

                let n_key = keys.id(neighbor);
                let dist = (neighbor.x - cx).abs() + (neighbor.y - cy).abs();
                let penalty = self.crossing_penalty(ends.edge, current.point, neighbor);

                // The directional penalties: a step away from the destination costs much (lines 1272-1288).
                let dest_dx = dst.x - src.x;
                let dest_dy = dst.y - src.y;
                let move_dx = neighbor.x - cx;
                let move_dy = neighbor.y - cy;
                let mut dir_penalty = 0.0;
                if (dest_dy > 10.0 && move_dy < -5.0) || (dest_dy < -10.0 && move_dy > 5.0) {
                    dir_penalty = move_dy.abs() * 100.0;
                }
                if (dest_dx > 10.0 && move_dx < -5.0) || (dest_dx < -10.0 && move_dx > 5.0) {
                    dir_penalty += move_dx.abs() * 50.0;
                }

                // The bend penalty: a change of direction costs 50 (lines 1292-1298).
                let current_dir = arrival.get(&current.key).copied().unwrap_or(Arrival::Start);
                let move_dir = if move_dx.abs() > EPS {
                    Arrival::Horizontal
                } else {
                    Arrival::Vertical
                };
                let bend_penalty = if current_dir != Arrival::Start && current_dir != move_dir {
                    50.0
                } else {
                    0.0
                };

                let step_cost = dist + penalty + dir_penalty + bend_penalty;
                let tentative_g =
                    g_score.get(&current.key).copied().unwrap_or(f64::INFINITY) + step_cost;
                let h = (dst.x - neighbor.x).abs() + (dst.y - neighbor.y).abs();

                if tentative_g < g_score.get(&n_key).copied().unwrap_or(f64::INFINITY) {
                    came_from.insert(n_key, current.point);
                    g_score.insert(n_key, tentative_g);
                    arrival.insert(n_key, move_dir);
                    if open_set.contains(&n_key) {
                        if let Some(entry) = open_list.iter_mut().find(|entry| entry.key == n_key) {
                            entry.f = tentative_g + h;
                        }
                    } else {
                        open_list.push(OpenEntry {
                            key: n_key,
                            f: tentative_g + h,
                            point: neighbor,
                        });
                        open_set.insert(n_key);
                    }
                }
            }
        }
        found
    }

    /// The simplification of a long path (lines 1329-1493): when the path turns round an obstacle, the path is
    /// replaced by the three-bend detour at the extreme x of the path, if that detour is free.
    pub(super) fn simplify_path(&self, ends: Endpoints<'_>, found: &[Point]) -> Option<Vec<Point>> {
        let start = found[0];
        let end = found[found.len() - 1];

        let mut min_x = js_min(start.x, end.x);
        let mut max_x = js_max(start.x, end.x);
        let mut min_y = js_min(start.y, end.y);
        let mut max_y = js_max(start.y, end.y);
        for pt in found {
            min_x = js_min(min_x, pt.x);
            max_x = js_max(max_x, pt.x);
            min_y = js_min(min_y, pt.y);
            max_y = js_max(max_y, pt.y);
        }

        // Whether the path went right or left of its endpoints.
        let went_right = max_x > js_max(start.x, end.x);
        let went_left = min_x < js_min(start.x, end.x);

        // In LR the detour uses the visual extent of the obstacles, which is their height (lines 1351-1407).
        if self.is_lr {
            let margin = VERTICAL_PIPE_MARGIN;
            if went_right {
                let path_x = js_max(start.x, end.x);
                let path_min_y = js_min(start.y, end.y);
                let path_max_y = js_max(start.y, end.y);
                let detour_obstacles: Vec<Obstacle> = self
                    .obstacles
                    .iter()
                    .copied()
                    .filter(|obs| {
                        obs.min_x < path_x
                            && obs.max_x > path_x
                            && obs.min_y < path_max_y
                            && obs.max_y > path_min_y
                    })
                    .collect();
                if !detour_obstacles.is_empty() {
                    let mut visual_max_x = js_max(start.x, end.x);
                    for obs in &detour_obstacles {
                        let obs_center_x = (obs.min_x + obs.max_x) / 2.0;
                        if obs.visual_x_half_extent.is_nan() {
                            continue;
                        }
                        let visual_right = obs_center_x + obs.visual_x_half_extent + margin;
                        visual_max_x = js_max(visual_max_x, visual_right);
                    }
                    if !visual_max_x.is_nan() {
                        max_x = visual_max_x;
                    }
                }
            }
            if went_left {
                let detour_obstacles: Vec<Obstacle> = self
                    .obstacles
                    .iter()
                    .copied()
                    .filter(|obs| {
                        obs.min_x < js_min(start.x, end.x) + margin
                            && obs.min_y < js_max(start.y, end.y)
                            && obs.max_y > js_min(start.y, end.y)
                    })
                    .collect();
                if !detour_obstacles.is_empty() {
                    let mut visual_min_x = js_min(start.x, end.x);
                    for obs in &detour_obstacles {
                        let obs_center_x = (obs.min_x + obs.max_x) / 2.0;
                        let visual_left = obs_center_x - obs.visual_x_half_extent - margin;
                        visual_min_x = js_min(visual_min_x, visual_left);
                    }
                    min_x = visual_min_x;
                }
            }
        }

        if went_right && !went_left {
            self.try_simplify_with_detour_x(ends, start, end, max_x)
        } else if went_left && !went_right {
            self.try_simplify_with_detour_x(ends, start, end, min_x)
        } else {
            None
        }
    }

    /// `findBestReturnY` (`router.ts:1411-1459`): the height of the horizontal return of a detour, just past the
    /// obstacles that the direct path crosses, or the end's height when no obstacle is in the way.
    fn find_best_return_y(&self, start: Point, end: Point, detour_x: f64) -> f64 {
        let going_down = end.y > start.y;
        let relevant: Vec<Obstacle> = self
            .obstacles
            .iter()
            .copied()
            .filter(|obs| {
                let obs_in_x_range =
                    js_min(start.x, end.x) < obs.max_x && js_max(start.x, end.x) > obs.min_x;
                let obs_in_y_range =
                    js_min(start.y, end.y) < obs.max_y && js_max(start.y, end.y) > obs.min_y;
                obs_in_x_range && obs_in_y_range
            })
            .collect();

        // In LR, only the obstacles that the chosen detour column crosses count, when there are any.
        let at_detour: Vec<Obstacle> = if self.is_lr && !relevant.is_empty() {
            relevant
                .iter()
                .copied()
                .filter(|obs| obs.min_x < detour_x && obs.max_x > detour_x)
                .collect()
        } else {
            Vec::new()
        };
        let filtered = if at_detour.is_empty() {
            relevant
        } else {
            at_detour
        };
        if filtered.is_empty() {
            return end.y;
        }

        let margin = HORIZONTAL_PIPE_MARGIN;
        if going_down {
            // The bottom of the lowest obstacle, and only when it is above the end.
            let lowest_obs_bottom = filtered
                .iter()
                .fold(f64::NEG_INFINITY, |acc, obs| js_max(acc, obs.max_y));
            let best_y = lowest_obs_bottom + margin;
            if best_y < end.y - EPS {
                return best_y;
            }
        } else {
            // The top of the highest obstacle, and only when it is below the end.
            let highest_obs_top = filtered
                .iter()
                .fold(f64::INFINITY, |acc, obs| js_min(acc, obs.min_y));
            let best_y = highest_obs_top - margin;
            if best_y > end.y + EPS {
                return best_y;
            }
        }
        end.y
    }

    /// `trySimplifyWithDetourX` (`router.ts:1464-1481`): the path start, the corners at `detour_x` and the best
    /// return height, and the end. Returns the path when no segment of it is blocked, and `None` otherwise.
    fn try_simplify_with_detour_x(
        &self,
        ends: Endpoints<'_>,
        start: Point,
        end: Point,
        detour_x: f64,
    ) -> Option<Vec<Point>> {
        let best_y = self.find_best_return_y(start, end, detour_x);
        let corner1 = Point {
            x: detour_x,
            y: start.y,
        };
        let corner2 = Point {
            x: detour_x,
            y: best_y,
        };
        let corner3 = Point {
            x: end.x,
            y: best_y,
        };
        let seg1_blocked = self.blocked(start, corner1, ends);
        let seg2_blocked = self.blocked(corner1, corner2, ends);
        let seg3_blocked = self.blocked(corner2, corner3, ends);
        let seg4_blocked = best_y != end.y && self.blocked(corner3, end, ends);

        if seg1_blocked || seg2_blocked || seg3_blocked || seg4_blocked {
            return None;
        }
        if (best_y - end.y).abs() < EPS {
            Some(vec![start, corner1, corner2, end])
        } else {
            Some(vec![start, corner1, corner2, corner3, end])
        }
    }
}
