// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/trees/{layout,preprocess_helpers,types}.go.

use std::collections::HashSet;

use serde::Serialize;

use super::{LayoutEdge, LayoutGraph};
use crate::Error;

const MAX_WORK: usize = 61_440_000;

#[derive(Debug, Default)]
pub(crate) struct TreeMember {
    pub sentinel_edge: Option<usize>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// Geometry remains in the authored graph. The tree stage owns the active
/// node and edge order, container order, and detached tree memberships.
#[derive(Debug)]
pub(crate) struct TreePreprocess {
    pub nodes: Vec<usize>,
    pub edges: Vec<usize>,
    pub containers: Vec<Vec<usize>>,
    pub incident: Vec<Vec<usize>>,
    pub tree_roots: Vec<Vec<usize>>,
    pub members: Vec<TreeMember>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Capture {
    nodes: Vec<i64>,
    edges: Vec<i64>,
    containers: Vec<ContainerOrder>,
    trees: Vec<DetachedTree>,
}

#[derive(Debug, Serialize)]
struct ContainerOrder {
    container: Option<i64>,
    nodes: Vec<i64>,
}

#[derive(Debug, Serialize)]
struct DetachedTree {
    sentinel: i64,
    roots: Vec<i64>,
    members: Vec<DetachedMember>,
}

#[derive(Debug, Serialize)]
struct DetachedMember {
    id: i64,
    edge: Option<i64>,
    children: Vec<i64>,
}

impl TreePreprocess {
    pub(crate) fn container_order(&self) -> Vec<usize> {
        let mut order = Vec::new();
        let mut stack: Vec<_> = self.containers[0]
            .iter()
            .map(|&node| (node + 1, false))
            .collect();
        while let Some((container, emit)) = stack.pop() {
            if emit {
                order.push(container);
            } else if !self.containers[container].is_empty() {
                stack.push((container, true));
                stack.extend(
                    self.containers[container]
                        .iter()
                        .map(|&node| (node + 1, false)),
                );
            }
        }
        order.push(0);
        order
    }

    pub(crate) fn from_authored(graph: &LayoutGraph) -> Self {
        let mut containers = Vec::with_capacity(graph.nodes.len() + 1);
        containers.push(graph.roots.clone());
        containers.extend(graph.nodes.iter().map(|node| node.children.clone()));
        let nodes = graph.nodes.len();
        let edges = graph.edges.len();
        Self {
            nodes: (0..nodes).collect(),
            edges: (0..edges).collect(),
            containers,
            incident: graph
                .nodes
                .iter()
                .map(|node| node.incident_edges.clone())
                .collect(),
            tree_roots: (0..nodes).map(|_| Vec::new()).collect(),
            members: (0..nodes).map(|_| TreeMember::default()).collect(),
        }
    }

    pub(crate) fn capture(&self, graph: &LayoutGraph) -> Capture {
        let node_id = |node: usize| graph.nodes[node].id;
        let edge_id = |edge: usize| graph.edges[edge].id;
        let containers = std::iter::once(None)
            .chain(
                self.nodes
                    .iter()
                    .copied()
                    .filter(|&node| !self.containers[node + 1].is_empty())
                    .map(Some),
            )
            .map(|container| ContainerOrder {
                container: container.map(node_id),
                nodes: self.containers[container.map_or(0, |slot| slot + 1)]
                    .iter()
                    .copied()
                    .map(node_id)
                    .collect(),
            })
            .collect();
        let trees = self
            .nodes
            .iter()
            .copied()
            .filter_map(|sentinel| {
                let roots = &self.tree_roots[sentinel];
                if roots.is_empty() {
                    return None;
                }
                let mut stack: Vec<_> = roots.iter().rev().copied().collect();
                let mut members = Vec::new();
                while let Some(node) = stack.pop() {
                    let member = &self.members[node];
                    members.push(DetachedMember {
                        id: node_id(node),
                        edge: member.sentinel_edge.map(edge_id),
                        children: member.children.iter().copied().map(node_id).collect(),
                    });
                    stack.extend(member.children.iter().rev().copied());
                }
                Some(DetachedTree {
                    sentinel: node_id(sentinel),
                    roots: roots.iter().copied().map(node_id).collect(),
                    members,
                })
            })
            .collect();
        Capture {
            nodes: self.nodes.iter().copied().map(node_id).collect(),
            edges: self.edges.iter().copied().map(edge_id).collect(),
            containers,
            trees,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum EdgeDirection {
    Outwards,
    Inwards,
    Bidirectional,
    Undirected,
}

struct Preprocessing<'a> {
    graph: &'a LayoutGraph,
    state: TreePreprocess,
    work: usize,
}

impl LayoutGraph {
    /// Applies ordinary tree extraction and returns a private result. Measured
    /// graphs have no table columns, Nears, fixed nodes or sequence vessels.
    pub(crate) fn tree_preprocess(&self) -> Result<TreePreprocess, Error> {
        let mut preprocessing = Preprocessing::new(self);
        let containers = preprocessing.state.container_order();
        for &container in &containers {
            preprocessing.extract(container)?;
        }
        for container in containers {
            preprocessing.restore_non_branching(container)?;
        }
        Ok(preprocessing.state)
    }
}

impl<'a> Preprocessing<'a> {
    fn new(graph: &'a LayoutGraph) -> Self {
        Self {
            graph,
            state: TreePreprocess::from_authored(graph),
            work: 0,
        }
    }

    fn extract(&mut self, container: usize) -> Result<(), Error> {
        let nodes = self.state.containers[container].clone();
        let mut terminal: Vec<_> = self
            .graph
            .nodes
            .iter()
            .map(|node| !node.children.is_empty())
            .collect();
        let mut roots: Vec<Vec<usize>> = (0..self.graph.nodes.len()).map(|_| Vec::new()).collect();
        let mut arrows = vec![[false; 2]; self.graph.nodes.len()];
        loop {
            let fringe = self.fringe(&nodes, container, &terminal)?;
            if fringe.is_empty() {
                break;
            }
            for &node in &fringe {
                if !terminal[node] {
                    self.connect_fringe(node, &mut terminal, &mut roots, &mut arrows)?;
                }
            }
            for node in fringe {
                if !terminal[node] {
                    let edge = self.state.members[node]
                        .sentinel_edge
                        .ok_or(Error::InvalidGraph)?;
                    self.remove(node, container, edge)?;
                }
            }
        }
        let active = self.state.nodes.clone();
        self.charge(active.len())?;
        for node in active {
            let mut attached = std::mem::take(&mut roots[node]);
            self.charge(attached.len())?;
            attached.retain(|&root| !terminal[root]);
            if attached.is_empty() {
                continue;
            }
            let (sentinel, attached) = self.merge_tree(node, container, attached)?;
            self.state.tree_roots[sentinel] = attached;
        }
        Ok(())
    }

    fn fringe(
        &mut self,
        nodes: &[usize],
        container: usize,
        terminal: &[bool],
    ) -> Result<Vec<usize>, Error> {
        self.charge(nodes.len().saturating_mul(2))?;
        let scope = container.checked_sub(1);
        Ok(nodes
            .iter()
            .copied()
            .filter(|&node| {
                let [edge] = self.state.incident[node].as_slice() else {
                    return false;
                };
                let adjacent = opposite(node, &self.graph.edges[*edge]);
                !terminal[node] && self.graph.nodes[adjacent].parent == scope
            })
            .collect())
    }

    fn connect_fringe(
        &mut self,
        node: usize,
        terminal: &mut [bool],
        roots: &mut [Vec<usize>],
        arrows: &mut [[bool; 2]],
    ) -> Result<(), Error> {
        self.charge(1)?;
        let [edge] = self.state.incident[node].as_slice() else {
            return Err(Error::InvalidGraph);
        };
        let edge = *edge;
        let reference = &self.graph.edges[edge];
        let sentinel = opposite(node, reference);
        if self.state.incident[sentinel].len() == 1 && !terminal[sentinel] {
            terminal[node] = true;
            return Ok(());
        }
        self.state.members[node].sentinel_edge = Some(edge);
        roots[sentinel].push(node);
        let has_arrow = if reference.from == sentinel {
            reference.source_arrow
        } else {
            reference.target_arrow
        };
        arrows[sentinel][usize::from(has_arrow)] = true;

        let children: Vec<_> = roots[node]
            .iter()
            .copied()
            .filter(|&root| !terminal[root])
            .collect();
        self.charge(roots[node].len())?;
        if children.is_empty() {
            return Ok(());
        }
        let mut directions = HashSet::new();
        for &child in &children {
            directions.insert(self.dominant_direction(child, true)?);
        }
        if directions.len() > 1 || arrows[node] == [true, true] {
            terminal[node] = true;
            return Ok(());
        }
        for child in children {
            self.attach(node, child)?;
        }
        let direction = edge_direction(node, reference);
        if direction != EdgeDirection::Undirected
            && !directions.contains(&EdgeDirection::Undirected)
            && !directions.contains(&direction)
        {
            terminal[sentinel] = true;
        }
        Ok(())
    }

    fn remove(&mut self, node: usize, container: usize, edge: usize) -> Result<(), Error> {
        self.charge(
            self.state.nodes.len()
                + self.state.edges.len()
                + self.state.containers[container].len(),
        )?;
        self.state.nodes.retain(|&index| index != node);
        self.state.containers[container].retain(|&index| index != node);
        self.state.edges.retain(|&index| index != edge);
        let from = self.graph.edges[edge].from;
        let to = self.graph.edges[edge].to;
        self.charge(self.state.incident[from].len() + self.state.incident[to].len())?;
        self.state.incident[from].retain(|&index| index != edge);
        if to != from {
            self.state.incident[to].retain(|&index| index != edge);
        }
        Ok(())
    }

    fn dominant_direction(
        &mut self,
        root: usize,
        include_root: bool,
    ) -> Result<EdgeDirection, Error> {
        let mut stack = if include_root {
            vec![root]
        } else {
            self.state.members[root]
                .children
                .iter()
                .rev()
                .copied()
                .collect()
        };
        let mut seen = HashSet::new();
        while let Some(node) = stack.pop() {
            self.charge(1)?;
            if !seen.insert(node) {
                return Err(Error::InvalidGraph);
            }
            let member = &self.state.members[node];
            let edge = member.sentinel_edge.ok_or(Error::InvalidGraph)?;
            let direction = edge_direction(node, &self.graph.edges[edge]);
            if direction != EdgeDirection::Undirected {
                return Ok(direction);
            }
            stack.extend(member.children.iter().rev().copied());
        }
        let edge = self.state.members[root]
            .sentinel_edge
            .ok_or(Error::InvalidGraph)?;
        Ok(edge_direction(root, &self.graph.edges[edge]))
    }

    fn attach(&mut self, parent: usize, child: usize) -> Result<(), Error> {
        self.charge(1)?;
        if let Some(old) = self.state.members[child].parent {
            self.charge(self.state.members[old].children.len())?;
            self.state.members[old]
                .children
                .retain(|&current| current != child);
        }
        self.state.members[child].parent = Some(parent);
        self.state.members[parent].children.push(child);
        Ok(())
    }

    fn merge_tree(
        &mut self,
        sentinel: usize,
        container: usize,
        roots: Vec<usize>,
    ) -> Result<(usize, Vec<usize>), Error> {
        if !self.state.incident[sentinel].is_empty()
            || roots.len() <= 1
            || !self.graph.nodes[sentinel].children.is_empty()
        {
            return Ok((sentinel, roots));
        }
        let candidates = self.candidate_roots(&roots)?;
        for candidate in candidates {
            if let Some(chain) = self.line(candidate)? {
                return self.reverse_and_join(sentinel, container, roots, &chain);
            }
        }
        Ok((sentinel, roots))
    }

    fn candidate_roots(&mut self, roots: &[usize]) -> Result<Vec<usize>, Error> {
        let mut outward = Vec::new();
        let mut inward = Vec::new();
        let mut bidirectional = Vec::new();
        let mut undirected = Vec::new();
        for &root in roots {
            match self.dominant_direction(root, false)? {
                EdgeDirection::Outwards => outward.push(root),
                EdgeDirection::Inwards => inward.push(root),
                EdgeDirection::Bidirectional => bidirectional.push(root),
                EdgeDirection::Undirected => undirected.push(root),
            }
        }
        let count = roots.len();
        let (n_out, n_in, n_bi, n_un) = (
            outward.len(),
            inward.len(),
            bidirectional.len(),
            undirected.len(),
        );
        let mut candidates = Vec::new();
        if n_out == 1 && 1 + n_in + n_un == count {
            candidates.extend(outward);
        }
        if n_in == 1 && 1 + n_out + n_un == count {
            candidates.extend(inward);
        }
        if (n_un > 0 && n_out > 1 && n_un + n_out == count)
            || (n_un > 0 && n_in > 1 && n_un + n_in == count)
        {
            candidates.extend_from_slice(&undirected);
        }
        if n_bi + n_un == count {
            candidates.extend(bidirectional);
            candidates.extend(undirected);
        }
        Ok(candidates)
    }

    fn line(&mut self, root: usize) -> Result<Option<Vec<usize>>, Error> {
        let mut chain = Vec::new();
        let mut node = root;
        loop {
            self.charge(1)?;
            chain.push(node);
            match self.state.members[node].children.as_slice() {
                [] => return Ok(Some(chain)),
                [next] => node = *next,
                _ => return Ok(None),
            }
        }
    }

    fn reverse_and_join(
        &mut self,
        sentinel: usize,
        container: usize,
        roots: Vec<usize>,
        chain: &[usize],
    ) -> Result<(usize, Vec<usize>), Error> {
        let candidate = chain[0];
        let new_sentinel = *chain.last().ok_or(Error::InvalidGraph)?;
        let outer_edge = self.state.members[candidate].sentinel_edge;
        for root in roots.into_iter().filter(|&root| root != new_sentinel) {
            self.attach(sentinel, root)?;
        }
        for pair in chain.windows(2) {
            let (current, next) = (pair[0], pair[1]);
            self.state.members[current].sentinel_edge =
                self.state.members[next].sentinel_edge.take();
            self.state.members[current].children.clear();
        }
        for pair in chain.windows(2).rev() {
            self.attach(pair[1], pair[0])?;
        }
        self.attach(candidate, sentinel)?;
        self.state.members[sentinel].sentinel_edge = outer_edge;
        let new_root = self.state.members[new_sentinel].children[0];
        self.state.members[new_root].parent = None;

        self.charge(self.state.nodes.len() + self.state.containers[container].len())?;
        self.state.nodes.retain(|&node| node != sentinel);
        self.state.nodes.push(new_sentinel);
        self.state.containers[container].retain(|&node| node != sentinel);
        self.state.containers[container].push(new_sentinel);
        Ok((new_sentinel, vec![new_root]))
    }

    fn restore_non_branching(&mut self, container: usize) -> Result<(), Error> {
        let nodes = self.state.containers[container].clone();
        for node in nodes {
            self.charge(1)?;
            let roots = std::mem::take(&mut self.state.tree_roots[node]);
            if roots.is_empty() {
                continue;
            }
            let isolated = self.graph.nodes[node].children.is_empty()
                && self.state.incident[node].iter().all(|&edge| {
                    roots
                        .iter()
                        .any(|&root| self.state.members[root].sentinel_edge == Some(edge))
                });
            if isolated && (roots.len() > 1 || self.line(roots[0])?.is_none()) {
                self.state.tree_roots[node] = roots;
                continue;
            }
            for root in roots {
                if let Some(chain) = self.line(root)? {
                    for member in chain {
                        self.restore(member, container)?;
                    }
                } else {
                    self.state.tree_roots[node].push(root);
                }
            }
        }
        Ok(())
    }

    fn restore(&mut self, node: usize, container: usize) -> Result<(), Error> {
        self.charge(1)?;
        let edge = self.state.members[node]
            .sentinel_edge
            .ok_or(Error::InvalidGraph)?;
        let reference = &self.graph.edges[edge];
        self.state.nodes.push(node);
        self.state.containers[container].push(node);
        self.state.incident[reference.from].push(edge);
        self.state.incident[reference.to].push(edge);
        self.state.edges.push(edge);
        Ok(())
    }

    fn charge(&mut self, work: usize) -> Result<(), Error> {
        self.work = self.work.saturating_add(work);
        if self.work > MAX_WORK {
            return Err(Error::WorkLimit);
        }
        Ok(())
    }
}

fn opposite(node: usize, edge: &LayoutEdge) -> usize {
    if edge.from == node {
        edge.to
    } else {
        edge.from
    }
}

fn edge_direction(node: usize, edge: &LayoutEdge) -> EdgeDirection {
    match (edge.source_arrow, edge.target_arrow) {
        (true, true) => EdgeDirection::Bidirectional,
        (false, false) => EdgeDirection::Undirected,
        _ if edge.to == node => EdgeDirection::Inwards,
        _ => EdgeDirection::Outwards,
    }
}

#[cfg(test)]
mod tests;
