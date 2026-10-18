//! The setup of the router (`routeEdgesOrthogonal`, lines 155-270), and the side and the port of a node
//! (`chooseOrthogonalSide` 82-106, `getOrthogonalPort` 268-269).
//!
//! Ported: the lanes (165-176), the obstacles (194-213), `getOrAddPipe` (216-238), `portForSide` (243-258),
//! `isSegmentBlocked` (372-395) and `isPointInObstacle` (803-816). The pipe spans and ids of the reference are
//! not kept: the reference writes them and never reads them.

use std::collections::HashSet;

use crate::math::{js_max, js_min};
use crate::model::Point;

use super::{EPS, Geo, Obstacle, Orientation, Pipe, Router, Side};

/// `chooseOrthogonalSide(node, target, fallback)` (`router.ts:82-106`): the side of `node` that faces `target`.
/// The vertical sides win unless the target is more than three times as far along x, with a bias of 3.
pub(super) fn choose_orthogonal_side(node: Geo, target: Point, fallback: Side) -> Side {
    let dx = target.x - node.x;
    let dy = target.y - node.y;
    let abs_dx = dx.abs();
    let abs_dy = dy.abs();

    if abs_dx < EPS && abs_dy < EPS {
        return fallback;
    }

    let vertical_bias = 3.0;
    if abs_dy > EPS && abs_dy * vertical_bias >= abs_dx {
        return if dy > 0.0 { Side::Bottom } else { Side::Top };
    }
    if abs_dx > EPS {
        return if dx > 0.0 { Side::Right } else { Side::Left };
    }
    fallback
}

/// `determineSide(node, target)` (`router.ts:416-417`): the side that faces `target`, with `bottom` as the
/// fallback.
pub(super) fn determine_side(node: Geo, target: Point) -> Side {
    choose_orthogonal_side(node, target, Side::Bottom)
}

/// `portForSide(node, side)` (`router.ts:243-258`): the centre of a side of the node.
pub(super) fn port_for_side(node: Geo, side: Side) -> Point {
    let w = node.width.unwrap_or(10.0);
    let h = node.height.unwrap_or(10.0);
    let cx = node.x;
    let cy = node.y;
    match side {
        Side::Top => Point {
            x: cx,
            y: cy - h / 2.0,
        },
        Side::Bottom => Point {
            x: cx,
            y: cy + h / 2.0,
        },
        Side::Left => Point {
            x: cx - w / 2.0,
            y: cy,
        },
        Side::Right => Point {
            x: cx + w / 2.0,
            y: cy,
        },
    }
}

/// `getOrthogonalPort(node, target, isSource)` (`router.ts:268-269`): the centre of the side that faces `target`,
/// with `bottom` as the fallback for the source and `top` for the target.
pub(super) fn orthogonal_port(node: Geo, target: Point, is_source: bool) -> Point {
    let fallback = if is_source { Side::Bottom } else { Side::Top };
    port_for_side(node, choose_orthogonal_side(node, target, fallback))
}

impl Router {
    /// The lane of each node (`router.ts:165-176`): every top-level group is a lane, and the lane is given to the
    /// group and to all of its descendants. A node that is its own ancestor is visited once per lane, where the
    /// reference would recurse without end.
    pub(super) fn assign_lanes(&mut self) {
        let top_level: Vec<usize> = (0..self.nodes.len())
            .filter(|&index| {
                let node = &self.nodes[index];
                node.is_group == Some(true) && node.parent_id.as_deref().is_none_or(str::is_empty)
            })
            .collect();
        for group in top_level {
            let lane = self.nodes[group].id.clone();
            let mut visited = HashSet::new();
            self.assign_lane(group, &lane, &mut visited);
        }
    }

    /// `assignLane` (`router.ts:171-174`): gives the lane to the node and, recursively, to its children, which
    /// are the nodes whose `parentId` is the id of the node.
    fn assign_lane(&mut self, index: usize, lane: &str, visited: &mut HashSet<usize>) {
        if !visited.insert(index) {
            return;
        }
        let id = self.nodes[index].id.clone();
        self.lane_by_node_id.insert(id.clone(), lane.to_owned());
        let children: Vec<usize> = (0..self.nodes.len())
            .filter(|&child| self.nodes[child].parent_id.as_deref() == Some(id.as_str()))
            .collect();
        for child in children {
            self.assign_lane(child, lane, visited);
        }
    }

    /// The obstacles (`router.ts:194-213`): the boxes of the nodes that are neither groups nor edge-label nodes,
    /// inflated by the node padding. The visual extent is the height in LR, where the reference uses it.
    pub(super) fn build_obstacles(&mut self) {
        let padding = super::NODE_PADDING;
        let is_lr = self.is_lr;
        self.obstacles = (0..self.nodes.len())
            .filter(|&index| {
                let node = &self.nodes[index];
                node.is_group != Some(true) && node.is_edge_label != Some(true)
            })
            .map(|index| {
                let node = &self.nodes[index];
                let w = node.width.unwrap_or(10.0);
                let h = node.height.unwrap_or(10.0);
                let x = node.x.unwrap_or(0.0);
                let y = node.y.unwrap_or(0.0);
                Obstacle {
                    node: index,
                    min_x: x - w / 2.0 - padding,
                    max_x: x + w / 2.0 + padding,
                    min_y: y - h / 2.0 - padding,
                    max_y: y + h / 2.0 + padding,
                    visual_x_half_extent: if is_lr {
                        h / 2.0 + padding
                    } else {
                        w / 2.0 + padding
                    },
                }
            })
            .collect();
    }

    /// `getOrAddPipe` (`router.ts:216-238`): the first pipe of the orientation within one unit of `coord`, or a
    /// new pipe at `coord`. Returns the index of the pipe in the list.
    pub(super) fn get_or_add_pipe(&mut self, orientation: Orientation, coord: f64) -> usize {
        if let Some(index) = self
            .pipes
            .iter()
            .position(|pipe| pipe.orientation == orientation && (pipe.coord - coord).abs() < 1.0)
        {
            return index;
        }
        self.pipes.push(Pipe { orientation, coord });
        self.pipes.len() - 1
    }

    /// Whether the obstacle is excluded by an endpoint id: the reference skips an obstacle whose node id equals a
    /// truthy endpoint id.
    fn is_excluded(&self, endpoint: Option<&str>, obstacle: &Obstacle) -> bool {
        match endpoint {
            Some(id) => !id.is_empty() && self.nodes[obstacle.node].id == id,
            None => false,
        }
    }

    /// `isSegmentBlocked(p1, p2, excludeStart, excludeEnd)` (`router.ts:372-395`): whether an obstacle other than
    /// the two endpoints blocks the axis-aligned segment from `p1` to `p2`.
    pub(super) fn is_segment_blocked(
        &self,
        p1: Point,
        p2: Point,
        exclude_start: Option<&str>,
        exclude_end: Option<&str>,
    ) -> bool {
        let seg_min_x = js_min(p1.x, p2.x);
        let seg_max_x = js_max(p1.x, p2.x);
        let seg_min_y = js_min(p1.y, p2.y);
        let seg_max_y = js_max(p1.y, p2.y);
        let horizontal = (p1.x - p2.x).abs() > EPS;
        self.obstacles.iter().any(|obs| {
            if self.is_excluded(exclude_start, obs) || self.is_excluded(exclude_end, obs) {
                return false;
            }
            if horizontal {
                obs.min_y < p1.y
                    && obs.max_y > p1.y
                    && obs.max_x > seg_min_x
                    && obs.min_x < seg_max_x
            } else {
                obs.min_x < p1.x
                    && obs.max_x > p1.x
                    && obs.max_y > seg_min_y
                    && obs.min_y < seg_max_y
            }
        })
    }

    /// `isPointInObstacle(pt, excludeNodeIds)` (`router.ts:803-816`): the first obstacle that strictly contains
    /// the point, other than the endpoints `start` and `end`.
    pub(super) fn obstacle_at(&self, pt: Point, start: &str, end: &str) -> Option<Obstacle> {
        self.obstacles.iter().copied().find(|obs| {
            let id = self.nodes[obs.node].id.as_str();
            if id == start || id == end {
                return false;
            }
            pt.x > obs.min_x && pt.x < obs.max_x && pt.y > obs.min_y && pt.y < obs.max_y
        })
    }
}
