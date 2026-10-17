//! The forces of one iteration on the tree path: springs along the edges, repulsion on a grid,
//! gravity on disconnected graphs, and the move (`FDLayout.js` 113-179 and 192-523,
//! `CoSENode.js` 14-63 for the move).

use crate::constants::{
    GRAVITY_RANGE_FACTOR, GRAVITY_STRENGTH, GRID_CALCULATION_CHECK_PERIOD, IDEAL_EDGE_LENGTH,
    MIN_REPULSION_DIST, REPULSION_STRENGTH, SPRING_STRENGTH,
};
use crate::engine::Engine;
use crate::geometry::{PointD, calc_separation_amount, intersection_clip, sign};

impl Engine {
    /// `FDLayout.calcSpringForces` (`FDLayout.js` 113-123): every edge, in order.
    pub(crate) fn calc_spring_forces(&mut self) {
        for e in 0..self.graph.edges.len() {
            let ideal_length = self.graph.edges[e].ideal_length;
            self.calc_spring_force(e, ideal_length);
        }
    }

    /// `LEdge.updateLength` (`LEdge.js` 105-132): the vector between the two rectangles, clipped to
    /// their boundaries. The target is the first rectangle, as the source code passes it.
    fn update_length(&mut self, e: usize) {
        let source = self.graph.edges[e].source;
        let target = self.graph.edges[e].target;
        let (overlapping, clip) = intersection_clip(
            &self.graph.nodes[target].rect,
            &self.graph.nodes[source].rect,
        );
        let edge = &mut self.graph.edges[e];
        edge.overlapping = overlapping;
        if !overlapping {
            let mut length_x = clip[0] - clip[2];
            let mut length_y = clip[1] - clip[3];
            if length_x.abs() < 1.0 {
                length_x = sign(length_x);
            }
            if length_y.abs() < 1.0 {
                length_y = sign(length_y);
            }
            edge.length_x = length_x;
            edge.length_y = length_y;
            edge.length = (length_x * length_x + length_y * length_y).sqrt();
        }
    }

    /// `FDLayout.calcSpringForce` (`FDLayout.js` 192-234): Hooke's law between the clipped ends,
    /// with the ideal length. Overlapping rectangles exert no spring.
    fn calc_spring_force(&mut self, e: usize, ideal_length: f64) {
        self.update_length(e);
        let edge = self.graph.edges[e].clone();
        if edge.overlapping {
            return;
        }
        let length = edge.length;
        if length == 0.0 {
            return;
        }
        let spring_force = SPRING_STRENGTH * (length - ideal_length);
        let spring_x = spring_force * (edge.length_x / length);
        let spring_y = spring_force * (edge.length_y / length);
        let source = &mut self.graph.nodes[edge.source].spring;
        source.x += spring_x;
        source.y += spring_y;
        let target = &mut self.graph.nodes[edge.target].spring;
        target.x -= spring_x;
        target.y -= spring_y;
    }

    /// `FDLayout.calcRepulsionForces` (`FDLayout.js` 125-168), the grid variant: the grid is rebuilt
    /// on iterations `1 mod 10`, and each node is repelled by the nodes it was last found near.
    pub(crate) fn calc_repulsion_forces(
        &mut self,
        grid_update_allowed: bool,
        force_surrounding_update: bool,
    ) {
        if self.total_iterations % GRID_CALCULATION_CHECK_PERIOD == 1 && grid_update_allowed {
            self.update_grid();
        }
        let n = self.graph.nodes.len();
        let mut processed = vec![false; n];
        for a in 0..n {
            self.calculate_repulsion_force_of_a_node(
                a,
                &processed,
                grid_update_allowed,
                force_surrounding_update,
            );
            processed[a] = true;
        }
    }

    /// `FDLayout.calcGrid` and `updateGrid` (`FDLayout.js` 381-467): cells of the repulsion range
    /// over the root bounds, each node in every cell its rectangle covers.
    fn update_grid(&mut self) {
        let range = self.repulsion_range;
        let size_x = ((self.graph.right - self.graph.left) / range).ceil() as usize;
        let size_y = ((self.graph.bottom - self.graph.top) / range).ceil() as usize;
        self.grid = vec![vec![Vec::new(); size_y]; size_x];
        let (left, top) = (self.graph.left, self.graph.top);
        for v in 0..self.graph.nodes.len() {
            self.add_node_to_grid(v, left, top);
        }
    }

    /// `FDLayout.addNodeToGrid` (`FDLayout.js` 432-455). The last finish uses `height + y`, as the
    /// source does; the cells are the same as for `y + height` apart from rounding of that sum.
    fn add_node_to_grid(&mut self, v: usize, left: f64, top: f64) {
        let range = self.repulsion_range;
        let rect = self.graph.nodes[v].rect;
        let start_x = ((rect.x - left) / range).floor() as i64;
        let finish_x = ((rect.width + rect.x - left) / range).floor() as i64;
        let start_y = ((rect.y - top) / range).floor() as i64;
        let finish_y = ((rect.height + rect.y - top) / range).floor() as i64;
        for i in start_x..=finish_x {
            for j in start_y..=finish_y {
                self.grid[i as usize][j as usize].push(v);
            }
        }
        let node = &mut self.graph.nodes[v];
        node.start_x = start_x;
        node.finish_x = finish_x;
        node.start_y = start_y;
        node.finish_y = finish_y;
    }

    /// `FDLayout.calculateRepulsionForceOfANode` (`FDLayout.js` 469-523): on a grid iteration, the
    /// nodes in the cells around `a` that are not processed yet and are within the repulsion range
    /// become its `surrounding`; then `a` is repelled by each of them.
    fn calculate_repulsion_force_of_a_node(
        &mut self,
        a: usize,
        processed: &[bool],
        grid_update_allowed: bool,
        force_surrounding_update: bool,
    ) {
        if (self.total_iterations % GRID_CALCULATION_CHECK_PERIOD == 1 && grid_update_allowed)
            || force_surrounding_update
        {
            let n = self.graph.nodes.len();
            let rect_a = self.graph.nodes[a].rect;
            let (start_x, finish_x) = (self.graph.nodes[a].start_x, self.graph.nodes[a].finish_x);
            let (start_y, finish_y) = (self.graph.nodes[a].start_y, self.graph.nodes[a].finish_y);
            let mut surrounding: Vec<usize> = Vec::new();
            let mut in_surrounding = vec![false; n];
            for i in (start_x - 1)..(finish_x + 2) {
                for j in (start_y - 1)..(finish_y + 2) {
                    if i < 0
                        || j < 0
                        || i as usize >= self.grid.len()
                        || j as usize >= self.grid[0].len()
                    {
                        continue;
                    }
                    for &b in &self.grid[i as usize][j as usize] {
                        if a == b {
                            continue;
                        }
                        if !processed[b] && !in_surrounding[b] {
                            let rect_b = self.graph.nodes[b].rect;
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
            self.graph.nodes[a].surrounding = surrounding;
        }
        let count = self.graph.nodes[a].surrounding.len();
        for k in 0..count {
            let b = self.graph.nodes[a].surrounding[k];
            self.calc_repulsion_force(a, b);
        }
    }

    /// `FDLayout.calcRepulsionForce` (`FDLayout.js` 236-314): overlapping rectangles are pushed
    /// apart by their separation; otherwise the inverse square of the clipped distance.
    fn calc_repulsion_force(&mut self, a: usize, b: usize) {
        let rect_a = self.graph.nodes[a].rect;
        let rect_b = self.graph.nodes[b].rect;
        let children_a = self.graph.nodes[a].no_of_children;
        let children_b = self.graph.nodes[b].no_of_children;
        if rect_a.intersects(&rect_b) {
            let overlap = calc_separation_amount(&rect_a, &rect_b, IDEAL_EDGE_LENGTH / 2.0);
            let repulsion_x = 2.0 * overlap[0];
            let repulsion_y = 2.0 * overlap[1];
            let children_constant = (children_a * children_b) / (children_a + children_b);
            let node_a = &mut self.graph.nodes[a].repulsion;
            node_a.x -= children_constant * repulsion_x;
            node_a.y -= children_constant * repulsion_y;
            let node_b = &mut self.graph.nodes[b].repulsion;
            node_b.x += children_constant * repulsion_x;
            node_b.y += children_constant * repulsion_y;
        } else {
            let (_, clip) = intersection_clip(&rect_a, &rect_b);
            let mut distance_x = clip[2] - clip[0];
            let mut distance_y = clip[3] - clip[1];
            if distance_x.abs() < MIN_REPULSION_DIST {
                distance_x = sign(distance_x) * MIN_REPULSION_DIST;
            }
            if distance_y.abs() < MIN_REPULSION_DIST {
                distance_y = sign(distance_y) * MIN_REPULSION_DIST;
            }
            let distance_squared = distance_x * distance_x + distance_y * distance_y;
            let distance = distance_squared.sqrt();
            let repulsion_force = REPULSION_STRENGTH * children_a * children_b / distance_squared;
            let repulsion_x = repulsion_force * distance_x / distance;
            let repulsion_y = repulsion_force * distance_y / distance;
            let node_a = &mut self.graph.nodes[a].repulsion;
            node_a.x -= repulsion_x;
            node_a.y -= repulsion_y;
            let node_b = &mut self.graph.nodes[b].repulsion;
            node_b.x += repulsion_x;
            node_b.y += repulsion_y;
        }
    }

    /// `FDLayout.calcGravitationalForces` (`FDLayout.js` 170-179): the nodes of a disconnected graph.
    pub(crate) fn calc_gravitational_forces(&mut self) {
        let nodes = self.nodes_with_gravity.clone();
        for node in nodes {
            self.calc_gravitational_force(node);
        }
    }

    /// `FDLayout.calcGravitationalForce` (`FDLayout.js` 325-356) for a node of the root graph: it is
    /// pulled to the centre of the graph when it is outside the gravity range.
    fn calc_gravitational_force(&mut self, node: usize) {
        let owner_centre_x = (self.graph.right + self.graph.left) / 2.0;
        let owner_centre_y = (self.graph.top + self.graph.bottom) / 2.0;
        let rect = self.graph.nodes[node].rect;
        let distance_x = rect.center_x() - owner_centre_x;
        let distance_y = rect.center_y() - owner_centre_y;
        let abs_distance_x = distance_x.abs() + rect.width / 2.0;
        let abs_distance_y = distance_y.abs() + rect.height / 2.0;
        let estimated_size = self.graph.estimated_size * GRAVITY_RANGE_FACTOR;
        if abs_distance_x > estimated_size || abs_distance_y > estimated_size {
            let target = &mut self.graph.nodes[node].gravitation;
            *target = PointD::new(
                -GRAVITY_STRENGTH * distance_x,
                -GRAVITY_STRENGTH * distance_y,
            );
        }
    }

    /// `FDLayout.moveNodes` (`FDLayout.js` 271-279) with `CoSENode.move`: each node moves by its
    /// forces, scaled by the cooling factor and its child count, capped per axis.
    pub(crate) fn move_nodes(&mut self) {
        for node in 0..self.graph.nodes.len() {
            self.move_node(node);
        }
    }

    fn move_node(&mut self, node: usize) {
        let cooling = self.cooling_factor;
        let max_displacement = self.max_node_displacement;
        let target = &mut self.graph.nodes[node];
        let children = target.no_of_children;
        let mut displacement_x =
            cooling * (target.spring.x + target.repulsion.x + target.gravitation.x) / children;
        let mut displacement_y =
            cooling * (target.spring.y + target.repulsion.y + target.gravitation.y) / children;
        if displacement_x.abs() > cooling * max_displacement {
            displacement_x = cooling * max_displacement * sign(displacement_x);
        }
        if displacement_y.abs() > cooling * max_displacement {
            displacement_y = cooling * max_displacement * sign(displacement_y);
        }
        target.rect.x += displacement_x;
        target.rect.y += displacement_y;
        target.spring = PointD::default();
        target.repulsion = PointD::default();
        target.gravitation = PointD::default();
        self.total_displacement += displacement_x.abs() + displacement_y.abs();
    }
}
