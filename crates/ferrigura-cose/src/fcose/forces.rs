//! The forces of one tick: springs along the edges, repulsion on a grid of the nodes of each
//! graph, and gravity on the nodes of unconnected graphs (`FDLayout.js` 112-356 and 391-523 of
//! layout-base 2.0.1, `LEdge.js` 105-132). The per-edge elasticity and the per-node repulsion are
//! the 2.0.1 forms (`springForce = edge.edgeElasticity * ...`, the average of the two repulsions).

use crate::fcose::layout::Layout;
use crate::geometry::{PointD, calc_separation_amount, intersection_clip, sign};

impl Layout {
    /// `FDLayout.calcSpringForces` (`FDLayout.js` 112-122): every edge of `getAllEdges`, in order.
    pub(crate) fn calc_spring_forces(&mut self) {
        for edge in self.model.all_edges() {
            self.calc_spring_force(edge);
        }
    }

    /// `LEdge.updateLength` (`LEdge.js` 105-132): the vector between the clip points of the two
    /// rectangles. The target comes first in the call, as the source has it.
    fn update_edge_length(&mut self, edge: usize) {
        let source = self.model.edges[edge].source;
        let target = self.model.edges[edge].target;
        let (overlapping, clip) = intersection_clip(
            &self.model.nodes[target].rect,
            &self.model.nodes[source].rect,
        );
        let e = &mut self.model.edges[edge];
        e.overlapping = overlapping;
        if !overlapping {
            let mut length_x = clip[0] - clip[2];
            let mut length_y = clip[1] - clip[3];
            if length_x.abs() < 1.0 {
                length_x = sign(length_x);
            }
            if length_y.abs() < 1.0 {
                length_y = sign(length_y);
            }
            e.length_x = length_x;
            e.length_y = length_y;
            e.length = (length_x * length_x + length_y * length_y).sqrt();
        }
    }

    /// `FDLayout.calcSpringForce` (`FDLayout.js` 191-233) with the edge's own ideal length and
    /// elasticity. Overlapping ends exert no force, and a zero length is skipped.
    fn calc_spring_force(&mut self, edge: usize) {
        self.update_edge_length(edge);
        let e = self.model.edges[edge].clone();
        if e.overlapping {
            return;
        }
        let length = e.length;
        if length == 0.0 {
            return;
        }
        let spring_force = e.edge_elasticity * (length - e.ideal_length);
        let spring_x = spring_force * (e.length_x / length);
        let spring_y = spring_force * (e.length_y / length);
        let source = &mut self.model.nodes[e.source].spring;
        source.x += spring_x;
        source.y += spring_y;
        let target = &mut self.model.nodes[e.target].spring;
        target.x -= spring_x;
        target.y -= spring_y;
    }

    /// `FDLayout.calcRepulsionForces` (`FDLayout.js` 124-167) with the grid variant (the default,
    /// `DEFAULT_USE_SMART_REPULSION_RANGE_CALCULATION`): the grid is rebuilt on the iterations
    /// `1 mod 10`, and each node is repelled by its surrounding nodes in the order it found them.
    pub(crate) fn calc_repulsion_forces(&mut self, grid_update_allowed: bool, force_update: bool) {
        if self.total_iterations % 10.0 == 1.0 && grid_update_allowed {
            self.update_grid();
        }
        let nodes = self.model.all_nodes();
        let mut processed = vec![false; self.model.nodes.len()];
        for node in nodes {
            self.calculate_repulsion_force_of_a_node(
                node,
                &processed,
                grid_update_allowed,
                force_update,
            );
            processed[node] = true;
        }
    }

    /// `FDLayout.calcGrid` and `updateGrid` (`FDLayout.js` 407-467): cells of the repulsion range
    /// over the root's bounds, each node in every cell its rectangle covers.
    fn update_grid(&mut self) {
        let root = self.model.root();
        let range = self.repulsion_range;
        let size_x = ((self.model.graphs[root].right - self.model.graphs[root].left) / range).ceil()
            as usize;
        let size_y = ((self.model.graphs[root].bottom - self.model.graphs[root].top) / range).ceil()
            as usize;
        self.grid = vec![vec![Vec::new(); size_y]; size_x];
        let left = self.model.graphs[root].left;
        let top = self.model.graphs[root].top;
        for node in self.model.all_nodes() {
            self.add_node_to_grid(node, left, top);
        }
    }

    /// `FDLayout.addNodeToGrid` (`FDLayout.js` 430-451). The cells are `parseInt(Math.floor(...))`
    /// of the rectangle's corners relative to the root's upper left corner.
    fn add_node_to_grid(&mut self, node: usize, left: f64, top: f64) {
        let range = self.repulsion_range;
        let rect = self.model.nodes[node].rect;
        let start_x = ((rect.x - left) / range).floor() as i64;
        let finish_x = ((rect.width + rect.x - left) / range).floor() as i64;
        let start_y = ((rect.y - top) / range).floor() as i64;
        let finish_y = ((rect.height + rect.y - top) / range).floor() as i64;
        for i in start_x..=finish_x {
            for j in start_y..=finish_y {
                self.grid[i as usize][j as usize].push(node);
            }
        }
        let n = &mut self.model.nodes[node];
        n.start_x = start_x;
        n.finish_x = finish_x;
        n.start_y = start_y;
        n.finish_y = finish_y;
    }

    /// `FDLayout.calculateRepulsionForceOfANode` (`FDLayout.js` 469-523): on a grid iteration the
    /// `surrounding` of `node` is rebuilt from the cells around it (the nodes of the same graph,
    /// not yet processed, within the repulsion range on both axes, in the order of the cells); then
    /// the node is repelled by each of them.
    fn calculate_repulsion_force_of_a_node(
        &mut self,
        node: usize,
        processed: &[bool],
        grid_update_allowed: bool,
        force_update: bool,
    ) {
        if (self.total_iterations % 10.0 == 1.0 && grid_update_allowed) || force_update {
            let mut surrounding: Vec<usize> = Vec::new();
            let mut in_surrounding = vec![false; self.model.nodes.len()];
            let a = self.model.nodes[node].clone();
            let rect_a = a.rect;
            let owner_a = a.owner;
            let grid_x = self.grid.len() as i64;
            let grid_y = if grid_x > 0 {
                self.grid[0].len() as i64
            } else {
                0
            };
            for i in (a.start_x - 1)..(a.finish_x + 2) {
                for j in (a.start_y - 1)..(a.finish_y + 2) {
                    if i < 0 || j < 0 || i >= grid_x || j >= grid_y {
                        continue;
                    }
                    let cell = self.grid[i as usize][j as usize].clone();
                    for b in cell {
                        if self.model.nodes[b].owner != owner_a || node == b {
                            continue;
                        }
                        if !processed[b] && !in_surrounding[b] {
                            let rect_b = self.model.nodes[b].rect;
                            let distance_x = (rect_a.center_x() - rect_b.center_x()).abs()
                                - ((rect_a.width / 2.0) + (rect_b.width / 2.0));
                            let distance_y = (rect_a.center_y() - rect_b.center_y()).abs()
                                - ((rect_a.height / 2.0) + (rect_b.height / 2.0));
                            if distance_x <= self.repulsion_range
                                && distance_y <= self.repulsion_range
                            {
                                surrounding.push(b);
                                in_surrounding[b] = true;
                            }
                        }
                    }
                }
            }
            self.model.nodes[node].surrounding = surrounding;
        }
        let count = self.model.nodes[node].surrounding.len();
        for k in 0..count {
            let other = self.model.nodes[node].surrounding[k];
            self.calc_repulsion_force(node, other);
        }
    }

    /// `FDLayout.calcRepulsionForce` (`FDLayout.js` 235-314, layout-base 2.0.1). Overlapping
    /// rectangles are separated by `calcSeparationAmount`, with the children's constant; otherwise
    /// the force is `(repA / 2 + repB / 2) * childrenA * childrenB / d^2` on the clipped distance.
    fn calc_repulsion_force(&mut self, a: usize, b: usize) {
        let rect_a = self.model.nodes[a].rect;
        let rect_b = self.model.nodes[b].rect;
        let children_a = self.model.nodes[a].no_of_children;
        let children_b = self.model.nodes[b].no_of_children;
        if rect_a.intersects(&rect_b) {
            let overlap =
                calc_separation_amount(&rect_a, &rect_b, self.k.default_edge_length / 2.0);
            let repulsion_x = 2.0 * overlap[0];
            let repulsion_y = 2.0 * overlap[1];
            let children_constant = children_a * children_b / (children_a + children_b);
            let node_a = &mut self.model.nodes[a].repulsion;
            node_a.x -= children_constant * repulsion_x;
            node_a.y -= children_constant * repulsion_y;
            let node_b = &mut self.model.nodes[b].repulsion;
            node_b.x += children_constant * repulsion_x;
            node_b.y += children_constant * repulsion_y;
        } else {
            let (_, clip) = intersection_clip(&rect_a, &rect_b);
            let mut distance_x = clip[2] - clip[0];
            let mut distance_y = clip[3] - clip[1];
            let min_dist = self.k.min_repulsion_dist;
            if distance_x.abs() < min_dist {
                distance_x = sign(distance_x) * min_dist;
            }
            if distance_y.abs() < min_dist {
                distance_y = sign(distance_y) * min_dist;
            }
            let distance_squared = distance_x * distance_x + distance_y * distance_y;
            let distance = distance_squared.sqrt();
            let repulsion_a = self.model.nodes[a].node_repulsion;
            let repulsion_b = self.model.nodes[b].node_repulsion;
            let repulsion_force = (repulsion_a / 2.0 + repulsion_b / 2.0) * children_a * children_b
                / distance_squared;
            let repulsion_x = repulsion_force * distance_x / distance;
            let repulsion_y = repulsion_force * distance_y / distance;
            let node_a = &mut self.model.nodes[a].repulsion;
            node_a.x -= repulsion_x;
            node_a.y -= repulsion_y;
            let node_b = &mut self.model.nodes[b].repulsion;
            node_b.x += repulsion_x;
            node_b.y += repulsion_y;
        }
    }

    /// `FDLayout.calcGravitationalForces` (`FDLayout.js` 169-178): the nodes set at the start of
    /// the layout, in their order.
    pub(crate) fn calc_gravitational_forces(&mut self) {
        let nodes = self.nodes_with_gravity.clone();
        for node in nodes {
            self.calc_gravitational_force(node);
        }
    }

    /// `FDLayout.calcGravitationalForce` (`FDLayout.js` 316-356): a node outside the range of its
    /// owner's estimated size is pulled to the owner's centre; the root and a compound differ in
    /// the range and in the constant.
    fn calc_gravitational_force(&mut self, node: usize) {
        let owner = self.model.nodes[node].owner;
        let owner_left = self.model.graphs[owner].left;
        let owner_right = self.model.graphs[owner].right;
        let owner_top = self.model.graphs[owner].top;
        let owner_bottom = self.model.graphs[owner].bottom;
        let owner_centre_x = (owner_right + owner_left) / 2.0;
        let owner_centre_y = (owner_top + owner_bottom) / 2.0;
        let rect = self.model.nodes[node].rect;
        let distance_x = rect.center_x() - owner_centre_x;
        let distance_y = rect.center_y() - owner_centre_y;
        let abs_distance_x = distance_x.abs() + rect.width / 2.0;
        let abs_distance_y = distance_y.abs() + rect.height / 2.0;
        let estimated = self.model.graphs[owner].estimated_size;
        if owner == self.model.root() {
            let range = estimated * self.gravity_range_factor;
            if abs_distance_x > range || abs_distance_y > range {
                self.model.nodes[node].gravitation = PointD::new(
                    -self.gravity_constant * distance_x,
                    -self.gravity_constant * distance_y,
                );
            }
        } else {
            let range = estimated * self.compound_gravity_range_factor;
            if abs_distance_x > range || abs_distance_y > range {
                self.model.nodes[node].gravitation = PointD::new(
                    -self.gravity_constant * distance_x * self.compound_gravity_constant,
                    -self.gravity_constant * distance_y * self.compound_gravity_constant,
                );
            }
        }
    }
}
