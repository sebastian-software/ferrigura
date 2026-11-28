// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/hierarchy/{eligibility,discovery}.go and
// internal/layoutgraph/layout.go.

use std::ops::Range;

use serde::Serialize;

use super::tree_preprocess::TreePreprocess;
use super::{LayoutGraph, Shape};

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct HierarchyPreprocess {
    pub candidates: Vec<i64>,
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub(crate) struct Scope {
    pub container: Option<i64>,
    pub nodes: Vec<i64>,
    pub edges: Vec<ScopedEdge>,
    pub abductions: Vec<Abduction>,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct ScopedEdge {
    pub id: i64,
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Abduction {
    pub id: i64,
    pub original_from: Option<i64>,
    pub original_to: Option<i64>,
    pub current_from: Option<i64>,
    pub current_to: Option<i64>,
}

impl LayoutGraph {
    /// Hierarchy eligibility and edge rebinding for the canonical measured
    /// workspace. The renderer's input has no fixed nodes or grouped vessels.
    /// Retain authored endpoints for later placement and result application.
    pub(crate) fn hierarchy_preprocess(&self) -> HierarchyPreprocess {
        self.hierarchy_for(&TreePreprocess::from_authored(self))
    }

    pub(crate) fn hierarchy_for(&self, workspace: &TreePreprocess) -> HierarchyPreprocess {
        let traversal = Traversal::new(workspace);
        let mut eligible = vec![false; self.nodes.len()];
        for &index in &workspace.nodes {
            eligible[index] = self.eligible(index, &traversal, workspace);
        }
        let mut candidates: Vec<_> = workspace
            .nodes
            .iter()
            .copied()
            .filter(|&index| eligible[index])
            .map(|index| self.nodes[index].id)
            .collect();
        candidates.sort_unstable();

        let mut scopes = Vec::new();
        let mut scope_slots = vec![None; self.nodes.len() + 1];
        let mut pending = vec![None];
        while let Some(container) = pending.pop() {
            let container_id = container.map(|index: usize| self.nodes[index].id);
            let container_slot = container.map_or(0, |index| index + 1);
            let nodes = &workspace.containers[container_slot];
            scope_slots[container_slot] = Some(scopes.len());
            scopes.push(Scope {
                container: container_id,
                nodes: nodes.iter().map(|&node| self.nodes[node].id).collect(),
                ..Scope::default()
            });
            for &child in nodes.iter().rev() {
                if !eligible[child] && !self.nodes[child].children.is_empty() {
                    pending.push(Some(child));
                }
            }
        }

        for &edge in &workspace.edges {
            let edge = &self.edges[edge];
            let projection = self.project(edge.from, edge.to, &traversal.depth);
            let Some(slot) = scope_slots[projection.parent.map_or(0, |index| index + 1)] else {
                continue;
            };
            let scope = &mut scopes[slot];
            let from = self.nodes[edge.from].id;
            let to = self.nodes[edge.to].id;
            if let Some((current_from, current_to)) = projection.endpoints {
                let current_from = self.nodes[current_from].id;
                let current_to = self.nodes[current_to].id;
                scope.edges.push(ScopedEdge {
                    id: edge.id,
                    from: current_from,
                    to: current_to,
                });
                if current_from != from || current_to != to {
                    scope.abductions.push(Abduction {
                        id: edge.id,
                        original_from: (from != current_from).then_some(from),
                        original_to: (to != current_to).then_some(to),
                        current_from: Some(current_from),
                        current_to: Some(current_to),
                    });
                }
            } else {
                scope.abductions.push(Abduction {
                    id: edge.id,
                    original_from: Some(from),
                    original_to: Some(to),
                    current_from: None,
                    current_to: None,
                });
            }
        }
        HierarchyPreprocess { candidates, scopes }
    }

    fn eligible(&self, index: usize, traversal: &Traversal, workspace: &TreePreprocess) -> bool {
        let node = &self.nodes[index];
        if !workspace.tree_roots[index].is_empty() {
            return false;
        }
        if node.children.is_empty() {
            return true;
        }
        if node.shape != Shape::Square {
            return false;
        }
        // A structural edge belonging to a descendant disqualifies the
        // frame when its opposite endpoint is the frame or a descendant.
        let descendants = traversal.intervals[index].clone();
        traversal.preorder[descendants.start + 1..descendants.end]
            .iter()
            .all(|&child| {
                workspace.tree_roots[child].is_empty()
                    && workspace.incident[child].iter().all(|&edge| {
                        let edge = &self.edges[edge];
                        if edge.from == edge.to {
                            return true;
                        }
                        let adjacent = if edge.from == child {
                            edge.to
                        } else {
                            edge.from
                        };
                        !descendants.contains(&traversal.intervals[adjacent].start)
                    })
            })
    }

    fn project(&self, from: usize, to: usize, depth: &[usize]) -> Projection {
        if from == to {
            return Projection {
                parent: self.nodes[from].parent,
                endpoints: Some((from, to)),
            };
        }
        let mut left = from;
        let mut right = to;
        while depth[left] > depth[right] {
            left = self.nodes[left].parent.expect("a deeper node has a parent");
        }
        while depth[right] > depth[left] {
            right = self.nodes[right]
                .parent
                .expect("a deeper node has a parent");
        }
        if left == right {
            // An edge connecting a frame with its own descendant does not
            // participate in the frame's placement in its enclosing scope.
            return Projection {
                parent: self.nodes[left].parent,
                endpoints: None,
            };
        }
        loop {
            let left_parent = self.nodes[left].parent;
            let right_parent = self.nodes[right].parent;
            if left_parent == right_parent {
                return Projection {
                    parent: left_parent,
                    endpoints: Some((left, right)),
                };
            }
            left = left_parent.expect("distinct parents at the same depth are present");
            right = right_parent.expect("distinct parents at the same depth are present");
        }
    }
}

struct Projection {
    parent: Option<usize>,
    endpoints: Option<(usize, usize)>,
}

struct Traversal {
    preorder: Vec<usize>,
    intervals: Vec<Range<usize>>,
    depth: Vec<usize>,
}

impl Traversal {
    fn new(workspace: &TreePreprocess) -> Self {
        let node_slots = workspace.containers.len() - 1;
        let mut traversal = Self {
            preorder: Vec::with_capacity(workspace.nodes.len()),
            intervals: vec![0..0; node_slots],
            depth: vec![0; node_slots],
        };
        for &root in &workspace.containers[0] {
            traversal.visit(workspace, root, 0);
        }
        traversal
    }

    fn visit(&mut self, workspace: &TreePreprocess, index: usize, depth: usize) {
        self.depth[index] = depth;
        let start = self.preorder.len();
        self.preorder.push(index);
        for &child in &workspace.containers[index + 1] {
            self.visit(workspace, child, depth + 1);
        }
        self.intervals[index] = start..self.preorder.len();
    }
}

#[cfg(test)]
mod tests;
