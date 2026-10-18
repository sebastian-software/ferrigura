//! The port assignment (`routeEdgesOrthogonal`, step 6, lines 397-735): the side of each edge at each end, the
//! split of the siblings that leave a node by one side, the bimodal in and out of diamonds, the port groups along
//! each side, and the port offsets. The ports of an edge (`portsForEdge`, 711-733) and the labels of a face
//! (`edgeHasLabelNode`, `faceHasLabelNode`, 679-694) are here too.
//!
//! The order of the steps is the order of the reference, and so is each map's insertion order. The side load
//! of 6.2 is the input of 6.2b, so both are in one pass over the edges.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::math::{js_max, js_min};
use crate::model::Point;

use super::js_sort::sort_by_js;
use super::setup::{determine_side, orthogonal_port, port_for_side};
use super::{
    Geo, MAX_PORT_SPACING, MIN_PORT_SPACING, PortEntry, PortKey, Role, Router, Side, SideInfo,
    js_sign,
};

/// The two ports of an edge and the sides they sit on (the result of `portsForEdge`).
#[derive(Clone, Copy, Debug)]
pub(super) struct EdgePorts {
    pub(super) src_port: Point,
    pub(super) dst_port: Point,
    pub(super) src_side: Side,
    pub(super) dst_side: Side,
}

/// `applyPortOffset(basePort, side, offset)` (`router.ts:697-709`): the offset along the side.
fn apply_port_offset(base: Point, side: Side, offset: f64) -> Point {
    if side.is_vertical() {
        Point {
            x: base.x + offset,
            y: base.y,
        }
    } else {
        Point {
            x: base.x,
            y: base.y + offset,
        }
    }
}

/// `preferenceStrength(info)` (`router.ts:481-486`): the ratio of the offset along the other axis to the offset
/// along the side's axis. An offset of zero on the side's axis gives infinity.
fn preference_strength(info: &SideInfo) -> f64 {
    if info.src_side.is_vertical() {
        if info.abs_dx == 0.0 {
            f64::INFINITY
        } else {
            info.abs_dy / info.abs_dx
        }
    } else if info.abs_dy == 0.0 {
        f64::INFINITY
    } else {
        info.abs_dx / info.abs_dy
    }
}

/// `secondarySide(info)` (`router.ts:487-492`): the side that a sibling moves to, on the other axis.
fn secondary_side(info: &SideInfo) -> Side {
    if info.src_side.is_vertical() {
        if info.dx_sign >= 0.0 {
            Side::Right
        } else {
            Side::Left
        }
    } else if info.dy_sign >= 0.0 {
        Side::Bottom
    } else {
        Side::Top
    }
}

impl Router {
    /// Step 6 (`router.ts:397-677`), in order: the incident edges, the sides, the sibling split, the diamond
    /// rule, the port groups, and the offsets.
    pub(super) fn assign_ports(&mut self) {
        self.count_incident_edges();
        self.build_side_info();
        let mut side_load = self.split_sibling_sides();
        self.resolve_diamond_bimodal(&mut side_load);
        self.build_port_groups();
        self.build_port_offsets();
    }

    /// `incidentEdgeTotals` (`router.ts:406-413`): the number of edges at each node, skipping an edge with a
    /// missing endpoint or a self-loop.
    fn count_incident_edges(&mut self) {
        let mut totals: HashMap<String, usize> = HashMap::new();
        for edge in &self.edges {
            let (Some(start), Some(end)) = (&edge.start, &edge.end) else {
                continue;
            };
            if start == end {
                continue;
            }
            *totals.entry(start.clone()).or_insert(0) += 1;
            *totals.entry(end.clone()).or_insert(0) += 1;
        }
        self.incident_totals = totals;
    }

    /// Step 6.1 (`router.ts:419-462`): the side of each edge at each of its ends, computed for every edge whose
    /// endpoints are two distinct nodes of the layout and which has no points yet.
    fn build_side_info(&mut self) {
        let mut info = IndexMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let (Some(start), Some(end)) = (&edge.start, &edge.end) else {
                continue;
            };
            if start == end {
                continue;
            }
            if edge
                .points
                .as_ref()
                .is_some_and(|points| !points.is_empty())
            {
                continue;
            }
            let (Some(src), Some(dst)) = (self.geo(start), self.geo(end)) else {
                continue;
            };
            let dx = dst.x - src.x;
            let dy = dst.y - src.y;
            info.insert(
                index,
                SideInfo {
                    src_id: start.clone(),
                    dst_id: end.clone(),
                    src_side: determine_side(src, Point { x: dst.x, y: dst.y }),
                    dst_side: determine_side(dst, Point { x: src.x, y: src.y }),
                    abs_dx: dx.abs(),
                    abs_dy: dy.abs(),
                    dx_sign: js_sign(dx),
                    dy_sign: js_sign(dy),
                },
            );
        }
        self.side_info = info;
    }

    /// Step 6.2 (`router.ts:464-564`): when two or more edges leave a node by the same side, the strongest keeps
    /// the side and a weaker one moves to its secondary side, if that side is less loaded. Returns the side load
    /// that the next step reads: the count of edges committed to each node side.
    fn split_sibling_sides(&mut self) -> HashMap<(String, Side), i64> {
        let mut source_side_groups: IndexMap<(String, Side), Vec<usize>> = IndexMap::new();
        for (&index, info) in &self.side_info {
            source_side_groups
                .entry((info.src_id.clone(), info.src_side))
                .or_default()
                .push(index);
        }

        let mut side_load: HashMap<(String, Side), i64> = HashMap::new();
        for info in self.side_info.values() {
            *side_load
                .entry((info.src_id.clone(), info.src_side))
                .or_insert(0) += 1;
            *side_load
                .entry((info.dst_id.clone(), info.dst_side))
                .or_insert(0) += 1;
        }

        for mut group in source_side_groups.into_values() {
            if group.len() < 2 {
                continue;
            }
            // The strongest first; the edge index breaks a tie, and a strength within 1e-9 counts as a tie.
            sort_by_js(&mut group, |a, b| {
                let strength_a = preference_strength(&self.side_info[&a]);
                let strength_b = preference_strength(&self.side_info[&b]);
                if (strength_a - strength_b).abs() > 1e-9 {
                    strength_b - strength_a
                } else {
                    a as f64 - b as f64
                }
            });
            for &index in group.iter().skip(1) {
                let info = self.side_info[&index].clone();
                let secondary = secondary_side(&info);
                let primary_load = side_load
                    .get(&(info.src_id.clone(), info.src_side))
                    .copied()
                    .unwrap_or(0);
                let secondary_load = side_load
                    .get(&(info.src_id.clone(), secondary))
                    .copied()
                    .unwrap_or(0);
                // Only a strictly less loaded secondary side is taken, so equal loads keep the side.
                if secondary_load >= primary_load {
                    continue;
                }
                side_load.insert((info.src_id.clone(), info.src_side), primary_load - 1);
                side_load.insert((info.src_id.clone(), secondary), secondary_load + 1);
                if let Some(entry) = self.side_info.get_mut(&index) {
                    entry.src_side = secondary;
                }
            }
        }
        side_load
    }

    /// Step 6.2b (`router.ts:566-609`): on a diamond, an out-edge that shares a side with an in-edge moves to its
    /// secondary side when no in-edge uses it and it carries no load.
    fn resolve_diamond_bimodal(&mut self, side_load: &mut HashMap<(String, Side), i64>) {
        let mut in_sides_by_node: HashMap<String, HashSet<Side>> = HashMap::new();
        for info in self.side_info.values() {
            in_sides_by_node
                .entry(info.dst_id.clone())
                .or_default()
                .insert(info.dst_side);
        }

        let indices: Vec<usize> = self.side_info.keys().copied().collect();
        for index in indices {
            let info = self.side_info[&index].clone();
            if !self.geo(&info.src_id).is_some_and(|node| node.diamond) {
                continue;
            }
            let Some(in_sides) = in_sides_by_node.get(&info.src_id) else {
                continue;
            };
            if !in_sides.contains(&info.src_side) {
                continue;
            }
            let secondary = secondary_side(&info);
            let secondary_load = side_load
                .get(&(info.src_id.clone(), secondary))
                .copied()
                .unwrap_or(0);
            if in_sides.contains(&secondary) || secondary_load > 0 {
                continue;
            }
            let primary_load = side_load
                .get(&(info.src_id.clone(), info.src_side))
                .copied()
                .unwrap_or(0);
            side_load.insert(
                (info.src_id.clone(), info.src_side),
                (primary_load - 1).max(0),
            );
            side_load.insert((info.src_id.clone(), secondary), 1);
            if let Some(entry) = self.side_info.get_mut(&index) {
                entry.src_side = secondary;
            }
        }
    }

    /// Step 6.3 (`router.ts:611-630`): each edge is added to the port group of its source side and of its
    /// target side, with the coordinate of the node at the other end.
    fn build_port_groups(&mut self) {
        let mut groups: IndexMap<PortKey, Vec<PortEntry>> = IndexMap::new();
        for (&index, info) in &self.side_info {
            let (Some(src), Some(dst)) = (self.geo(&info.src_id), self.geo(&info.dst_id)) else {
                continue;
            };
            let dst_coord = if info.src_side.is_vertical() {
                dst.x
            } else {
                dst.y
            };
            groups
                .entry((info.src_id.clone(), info.src_side, Role::Src))
                .or_default()
                .push(PortEntry {
                    edge: index,
                    opposite_coord: dst_coord,
                });
            let src_coord = if info.dst_side.is_vertical() {
                src.x
            } else {
                src.y
            };
            groups
                .entry((info.dst_id.clone(), info.dst_side, Role::Dst))
                .or_default()
                .push(PortEntry {
                    edge: index,
                    opposite_coord: src_coord,
                });
        }
        self.port_groups = groups;
    }

    /// The offsets of the ports in the groups of two or more (`router.ts:632-677`): the group is sorted by the
    /// coordinate of the other node, and its ports are spread along the side. A diamond gives 0.3 of the side.
    fn build_port_offsets(&mut self) {
        for group in self.port_groups.values_mut() {
            if group.len() >= 2 {
                sort_by_js(group, |a, b| a.opposite_coord - b.opposite_coord);
            }
        }

        let mut offsets: Vec<((usize, Role), f64)> = Vec::new();
        for ((node_id, side, role), group) in &self.port_groups {
            if group.len() < 2 {
                continue;
            }
            let Some(node) = self.geo(node_id) else {
                continue;
            };
            let side_length = if matches!(side, Side::Left | Side::Right) {
                node.height.unwrap_or(10.0)
            } else {
                node.width.unwrap_or(10.0)
            };
            let effective_length = if node.diamond {
                side_length * 0.3
            } else {
                side_length
            };
            let spacing = js_min(
                MAX_PORT_SPACING,
                js_max(
                    MIN_PORT_SPACING,
                    effective_length / (group.len() as f64 + 1.0),
                ),
            );
            let total_span = spacing * (group.len() as f64 - 1.0);
            let start_offset = -total_span / 2.0;
            for (j, element) in group.iter().enumerate() {
                let offset = start_offset + j as f64 * spacing;
                offsets.push(((element.edge, *role), offset));
            }
        }
        self.port_offsets.extend(offsets);
    }

    /// The number of edges of a node on one side and role (`portGroups.get(...)?.length ?? 0`).
    pub(super) fn port_group_len(&self, node: &str, side: Side, role: Role) -> usize {
        self.port_group_edges(node, side, role).len()
    }

    /// The edges in a port group, or none.
    fn port_group_edges(&self, node: &str, side: Side, role: Role) -> &[PortEntry] {
        self.port_groups
            .get(&(node.to_owned(), side, role))
            .map_or(&[], Vec::as_slice)
    }

    /// `faceHasLabelNode(nodeId, side)` (`router.ts:682-694`): whether an edge that has a label node is at this
    /// side of the node, in either role.
    pub(super) fn face_has_label_node(&self, node: Option<&str>, side: Side) -> bool {
        let Some(node) = node.filter(|id| !id.is_empty()) else {
            return false;
        };
        [Role::Src, Role::Dst].into_iter().any(|role| {
            self.port_group_edges(node, side, role)
                .iter()
                .any(|entry| self.edges[entry.edge].label_node_id.is_some())
        })
    }

    /// `portsForEdge(edgeIndex, src, dst)` (`router.ts:711-733`): the ports of an edge on the sides of the sibling
    /// split, offset along their sides when the edge is in a port group of two or more.
    pub(super) fn ports_for_edge(&self, edge: usize, src: Geo, dst: Geo) -> EdgePorts {
        let info = self.side_info.get(&edge);
        let src_target = Point { x: dst.x, y: dst.y };
        let dst_target = Point { x: src.x, y: src.y };
        let src_side = info.map_or_else(|| determine_side(src, src_target), |info| info.src_side);
        let dst_side = info.map_or_else(|| determine_side(dst, dst_target), |info| info.dst_side);
        let mut src_port = match info {
            Some(info) => port_for_side(src, info.src_side),
            None => orthogonal_port(src, src_target, true),
        };
        let mut dst_port = match info {
            Some(info) => port_for_side(dst, info.dst_side),
            None => orthogonal_port(dst, dst_target, false),
        };
        if let Some(&offset) = self.port_offsets.get(&(edge, Role::Src)) {
            src_port = apply_port_offset(src_port, src_side, offset);
        }
        if let Some(&offset) = self.port_offsets.get(&(edge, Role::Dst)) {
            dst_port = apply_port_offset(dst_port, dst_side, offset);
        }
        EdgePorts {
            src_port,
            dst_port,
            src_side,
            dst_side,
        }
    }
}
