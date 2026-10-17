//! The radial start of a forest: `Layout.getFlatForest` and `findCenterOfTree` (layout-base
//! `Layout.js` 349-459 and 587-661), `CoSELayout.positionNodesRadially`, `radialLayout` and
//! `branchRadialLayout` (cose-base `CoSELayout.js` 292-420).
//!
//! A mindmap is one tree, so this is the placement it gets. The spring embedder then starts from
//! these centres, and the placement draws no random numbers.

use std::collections::VecDeque;

use crate::constants::{COMPONENT_SEPARATION, RADIAL_SEPARATION, WORLD_CENTER_X, WORLD_CENTER_Y};
use crate::geometry::{PointD, RectD, TWO_PI, Transform, js_max};
use crate::graph::{INTEGER_MAX, INTEGER_MIN, LGraph};

impl LGraph {
    /// `Layout.getFlatForest` (`Layout.js` 349-459): the connected components, each as its nodes in
    /// breadth-first order from the first unvisited node. Empty when a component is not a tree.
    pub(crate) fn flat_forest(&self) -> Vec<Vec<usize>> {
        let n = self.nodes.len();
        let mut flat_forest: Vec<Vec<usize>> = Vec::new();
        let mut unprocessed: Vec<usize> = (0..n).collect();
        let mut is_forest = true;
        while !unprocessed.is_empty() && is_forest {
            let mut visited: Vec<usize> = Vec::new();
            let mut in_visited = vec![false; n];
            let mut parents: Vec<Option<usize>> = vec![None; n];
            let mut to_be_visited: VecDeque<usize> = VecDeque::from([unprocessed[0]]);
            while !to_be_visited.is_empty() && is_forest {
                let current = to_be_visited.pop_front().expect("queue is not empty");
                if !in_visited[current] {
                    in_visited[current] = true;
                    visited.push(current);
                }
                for &e in &self.nodes[current].edges {
                    let neighbour = self.edges[e].other_end(current);
                    if parents[current] != Some(neighbour) {
                        if !in_visited[neighbour] {
                            to_be_visited.push_back(neighbour);
                            parents[neighbour] = Some(current);
                        } else {
                            is_forest = false;
                            break;
                        }
                    }
                }
            }
            if !is_forest {
                return Vec::new();
            }
            for value in &visited {
                if let Some(index) = unprocessed.iter().position(|u| u == value) {
                    unprocessed.remove(index);
                }
            }
            flat_forest.push(visited);
        }
        flat_forest
    }

    /// `Layout.findCenterOfTree` (`Layout.js` 587-661): the centre of a tree, by peeling leaves.
    ///
    /// The loop over `list` removes the node it visits, so every other node is skipped in each
    /// round (`Layout.js` 624-631). That quirk gives a different centre for many trees: the path
    /// `a-b-c-d-e` gives `b`, and a star gives one of its leaves. It is reproduced on purpose.
    pub(crate) fn find_center_of_tree(&self, nodes: &[usize]) -> Option<usize> {
        let mut list: Vec<usize> = nodes.to_vec();
        let mut removed: Vec<usize> = Vec::new();
        let mut remaining = vec![0_i64; self.nodes.len()];
        let mut found = false;
        let mut center = None;
        if list.len() == 1 || list.len() == 2 {
            found = true;
            center = list.first().copied();
        }
        for &node in &list {
            let degree = self.neighbours(node).len() as i64;
            remaining[node] = degree;
            if degree == 1 {
                removed.push(node);
            }
        }
        while !found {
            let mut temp: Vec<usize> = Vec::new();
            let mut i = 0;
            while i < list.len() {
                let node = list[i];
                if let Some(index) = list.iter().position(|&x| x == node) {
                    list.remove(index);
                }
                for neighbour in self.neighbours(node) {
                    if !removed.contains(&neighbour) {
                        let new_degree = remaining[neighbour] - 1;
                        if new_degree == 1 {
                            temp.push(neighbour);
                        }
                        remaining[neighbour] = new_degree;
                    }
                }
                i += 1;
            }
            removed.extend(temp);
            if list.is_empty() {
                return None;
            }
            if list.len() == 1 || list.len() == 2 {
                found = true;
                center = Some(list[0]);
            }
        }
        center
    }

    /// `CoSELayout.positionNodesRadially` (`CoSELayout.js` 292-325): the trees, in rows of
    /// `ceil(sqrt(count))`, each placed radially and moved to its cell.
    pub(crate) fn position_nodes_radially(&mut self, forest: &[Vec<usize>]) -> Option<()> {
        let number_of_columns = (forest.len() as f64).sqrt().ceil() as usize;
        let mut height = 0.0_f64;
        let mut current_y = 0.0_f64;
        let mut current_x = 0.0_f64;
        let mut point = PointD::new(0.0, 0.0);
        for (i, tree) in forest.iter().enumerate() {
            if i % number_of_columns == 0 {
                current_x = 0.0;
                current_y = height;
                if i != 0 {
                    current_y += COMPONENT_SEPARATION;
                }
                height = 0.0;
            }
            let center = self.find_center_of_tree(tree)?;
            let starting_point = PointD::new(current_x, current_y);
            point = self.radial_layout(tree, center, starting_point);
            if point.y > height {
                height = point.y.floor();
            }
            current_x = (point.x + COMPONENT_SEPARATION).floor();
        }
        self.transform_to(PointD::new(
            WORLD_CENTER_X - point.x / 2.0,
            WORLD_CENTER_Y - point.y / 2.0,
        ));
        Some(())
    }

    /// `CoSELayout.radialLayout` (`CoSELayout.js` 327-344): places one tree around its centre, then
    /// moves it to `starting_point`. Returns the far corner of the tree, in the frame of the
    /// forest.
    fn radial_layout(&mut self, tree: &[usize], center: usize, starting_point: PointD) -> PointD {
        let radial_separation = js_max(max_diagonal_in_tree(self, tree), RADIAL_SEPARATION);
        self.branch_radial_layout(center, None, 0.0, 359.0, 0.0, radial_separation);
        let bounds = calculate_bounds(self, tree);
        let transform = Transform::new(starting_point, PointD::new(bounds.x, bounds.y));
        for &node in tree {
            self.transform_node(node, &transform);
        }
        let bottom_right = PointD::new(bounds.x + bounds.width, bounds.y + bounds.height);
        transform.inverse_transform_point(bottom_right)
    }

    /// `CoSELayout.branchRadialLayout` (`CoSELayout.js` 346-407): a node at `distance` on the arc
    /// `start_angle..end_angle`, its children on the arc in the order of its edges, each one ring
    /// further out.
    fn branch_radial_layout(
        &mut self,
        node: usize,
        parent: Option<usize>,
        start_angle: f64,
        end_angle: f64,
        distance: f64,
        radial_separation: f64,
    ) {
        let mut half_interval = ((end_angle - start_angle) + 1.0) / 2.0;
        if half_interval < 0.0 {
            half_interval += 180.0;
        }
        let node_angle = (half_interval + start_angle) % 360.0;
        let teta = (node_angle * TWO_PI) / 360.0;
        let x = distance * ferrigura_jsmath::cos(teta);
        let y = distance * ferrigura_jsmath::sin(teta);
        self.set_center(node, x, y);

        let mut neighbor_edges: Vec<usize> = self.nodes[node].edges.clone();
        let mut child_count = neighbor_edges.len() as f64;
        if parent.is_some() {
            child_count -= 1.0;
        }
        let mut inc_edges_count = neighbor_edges.len();
        let mut edges = self.edges_between(node, parent);
        while edges.len() > 1 {
            let temp = edges.remove(0);
            if let Some(index) = neighbor_edges.iter().position(|&e| e == temp) {
                neighbor_edges.remove(index);
            }
            inc_edges_count -= 1;
            child_count -= 1.0;
        }
        let start_index = if parent.is_some() {
            let index = neighbor_edges
                .iter()
                .position(|&e| Some(e) == edges.first().copied())
                .map_or(-1_i64, |index| index as i64);
            ((index + 1) as usize) % inc_edges_count
        } else {
            0
        };

        let step_angle = (end_angle - start_angle).abs() / child_count;
        let mut branch_count = 0.0_f64;
        let mut i = start_index;
        while branch_count != child_count {
            let current = self.edges[neighbor_edges[i]].other_end(node);
            if Some(current) != parent {
                let child_start = (start_angle + branch_count * step_angle) % 360.0;
                let child_end = (child_start + step_angle) % 360.0;
                self.branch_radial_layout(
                    current,
                    Some(node),
                    child_start,
                    child_end,
                    distance + radial_separation,
                    radial_separation,
                );
                branch_count += 1.0;
            }
            i = (i + 1) % inc_edges_count;
        }
    }

    /// `Layout.setCenter` for a node: the corner keeps the size and moves so the centre is `(x, y)`.
    fn set_center(&mut self, node: usize, x: f64, y: f64) {
        let rect = &mut self.nodes[node].rect;
        rect.x = x - rect.width / 2.0;
        rect.y = y - rect.height / 2.0;
    }
}

/// `CoSELayout.maxDiagonalInTree` (`CoSELayout.js` 409-420): the largest diagonal of the tree's
/// nodes, from `Integer.MIN_VALUE` up.
fn max_diagonal_in_tree(graph: &LGraph, tree: &[usize]) -> f64 {
    let mut max_diagonal = INTEGER_MIN;
    for &node in tree {
        let rect = graph.nodes[node].rect;
        let diagonal = (rect.width * rect.width + rect.height * rect.height).sqrt();
        if diagonal > max_diagonal {
            max_diagonal = diagonal;
        }
    }
    max_diagonal
}

/// `LGraph.calculateBounds` (`LGraph.js` 366-384): the rectangle of the given nodes.
fn calculate_bounds(graph: &LGraph, tree: &[usize]) -> RectD {
    let mut left = INTEGER_MAX;
    let mut right = -INTEGER_MAX;
    let mut top = INTEGER_MAX;
    let mut bottom = -INTEGER_MAX;
    for &node in tree {
        let rect = graph.nodes[node].rect;
        if left > rect.x {
            left = rect.x;
        }
        if right < rect.right() {
            right = rect.right();
        }
        if top > rect.y {
            top = rect.y;
        }
        if bottom < rect.bottom() {
            bottom = rect.bottom();
        }
    }
    RectD::new(left, top, right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use crate::{Edge, Node};

    use crate::graph::LGraph;

    /// A graph of nodes `0..n` (unit sizes) with the given edges between node indices.
    fn graph(n: usize, edges: &[(usize, usize)]) -> LGraph {
        let nodes: Vec<Node> = (0..n)
            .map(|i| Node {
                id: i.to_string(),
                width: 10.0,
                height: 10.0,
            })
            .collect();
        let edges: Vec<Edge> = edges
            .iter()
            .map(|&(s, t)| Edge {
                id: format!("{s}-{t}"),
                source: s.to_string(),
                target: t.to_string(),
            })
            .collect();
        LGraph::build(&nodes, &edges).expect("a valid graph")
    }

    #[test]
    fn centre_of_a_path_is_its_second_node() {
        // a-b-c-d-e: the splice in findCenterOfTree skips every other node, so the centre is b.
        let g = graph(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        assert_eq!(g.find_center_of_tree(&[0, 1, 2, 3, 4]), Some(1));
    }

    #[test]
    fn centre_of_a_star_is_its_first_leaf() {
        // The centre s has four leaves; the quirk gives leaf 1, not s.
        let g = graph(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]);
        assert_eq!(g.find_center_of_tree(&[0, 1, 2, 3, 4]), Some(1));
    }

    #[test]
    fn flat_forest_lists_each_tree_breadth_first() {
        let g = graph(5, &[(0, 2), (2, 1), (3, 4)]);
        assert_eq!(g.flat_forest(), vec![vec![0, 2, 1], vec![3, 4]]);
    }

    #[test]
    fn a_cycle_is_not_a_forest() {
        let g = graph(3, &[(0, 1), (1, 2), (2, 0)]);
        assert!(g.flat_forest().is_empty());
    }
}
