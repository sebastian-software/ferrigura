// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/grouping/{clusters,cluster_discovery}.go.

use std::collections::HashSet;

use serde::Serialize;

use super::tree_preprocess::TreePreprocess;
use super::{LayoutEdge, LayoutGraph, Shape};
use crate::Error;

const MAX_ESTIMATED_SIZE_RATIO: f64 = 4.0;
const MAX_WORK: usize = 1_000_000_000;
const NODE_GAP: f64 = 20.0;

/// Temporary placement endpoints retain canonical authored node and edge slots.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Endpoint {
    Node(usize),
    Vessel(usize),
}

#[derive(Debug)]
pub(crate) struct LeafCluster {
    pub container: Option<usize>,
    pub nodes: Vec<usize>,
    pub edges: Vec<usize>,
    pub fixed_size: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub(crate) enum Arrangement {
    Row,
    Column,
}

#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "camelCase")]
pub(crate) struct VesselGeometry {
    pub arrangement: Arrangement,
    pub padding: f64,
    pub width: f64,
    pub height: f64,
    pub member_sizes: Vec<[f64; 2]>,
}

impl LeafCluster {
    /// Computes the vessel from measured geometry. The container's random
    /// stream is read only when both rounded average dimensions are equal.
    pub(crate) fn geometry(
        &self,
        graph: &LayoutGraph,
        square_draw: impl FnOnce() -> Option<f64>,
    ) -> Option<VesselGeometry> {
        let count = f64::from(u32::try_from(self.nodes.len()).ok()?);
        let (mut total_width, mut total_height) = (0.0, 0.0);
        let (mut maximum_width, mut maximum_height) = (0.0_f64, 0.0_f64);
        for &node in &self.nodes {
            let node = &graph.nodes[node];
            total_width += node.width;
            total_height += node.height;
            maximum_width = maximum_width.max(node.width);
            maximum_height = maximum_height.max(node.height);
        }
        let average_width = (total_width / count).round();
        let average_height = (total_height / count).round();
        let arrangement = if average_width > average_height {
            Arrangement::Column
        } else if average_width < average_height {
            Arrangement::Row
        } else if square_draw()? > 0.5 {
            Arrangement::Column
        } else {
            Arrangement::Row
        };
        let average = match arrangement {
            Arrangement::Row => average_width,
            Arrangement::Column => average_height,
        };
        let padding = NODE_GAP.max(average.ceil() * 0.1).round();
        let (width, height) = match arrangement {
            Arrangement::Row => (
                maximum_width * count + padding * (count - 1.0),
                maximum_height,
            ),
            Arrangement::Column => (
                maximum_width,
                maximum_height * count + padding * (count - 1.0),
            ),
        };
        let member_sizes = self
            .nodes
            .iter()
            .map(|&node| {
                let node = &graph.nodes[node];
                if self.fixed_size {
                    [node.width, node.height]
                } else {
                    [maximum_width, maximum_height]
                }
            })
            .collect();
        Some(VesselGeometry {
            arrangement,
            padding,
            width,
            height,
            member_sizes,
        })
    }
}

#[derive(Debug)]
pub(crate) struct LayoutClusters {
    pub clusters: Vec<LeafCluster>,
    pub endpoints: Vec<(Endpoint, Endpoint)>,
}

#[derive(Debug, Default)]
struct Signature {
    from: usize,
    to: usize,
    directed: usize,
    bidirectional: usize,
    undirected: usize,
    own_arrowheads: [bool; 2],
    adjacent_arrowheads: [bool; 2],
}

impl Signature {
    fn add(&mut self, node: usize, edge: &LayoutEdge) {
        self.from += usize::from(edge.from == node);
        self.to += usize::from(edge.to == node);
        match (edge.source_arrow, edge.target_arrow) {
            (true, true) => self.bidirectional += 1,
            (false, false) => self.undirected += 1,
            _ => self.directed += 1,
        }
        let (own, adjacent) = if edge.from == node {
            (edge.source_arrow, edge.target_arrow)
        } else {
            (edge.target_arrow, edge.source_arrow)
        };
        self.own_arrowheads[usize::from(own)] = true;
        self.adjacent_arrowheads[usize::from(adjacent)] = true;
    }

    fn homogeneous(&self) -> bool {
        usize::from(self.directed > 0)
            + usize::from(self.bidirectional > 0)
            + usize::from(self.undirected > 0)
            <= 1
    }

    fn matches(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.bidirectional == other.bidirectional
            && self.undirected == other.undirected
            && self.homogeneous()
            && other.homogeneous()
            && self.own_arrowheads == other.own_arrowheads
            && self.adjacent_arrowheads == other.adjacent_arrowheads
    }
}

struct Discovery<'a> {
    graph: &'a LayoutGraph,
    workspace: &'a TreePreprocess,
    memberships: Vec<Option<usize>>,
    signatures: Vec<Signature>,
    neighbors: Vec<HashSet<Endpoint>>,
    eligible: Vec<bool>,
    edge_order: Vec<usize>,
    result: LayoutClusters,
    work: usize,
}

impl LayoutGraph {
    /// Discovers clusters of active leaves and preserves original incidences by
    /// returning rebound endpoints beside the graph. Hierarchy membership is
    /// supplied by discovery because its nodes cannot enter a cluster.
    pub(crate) fn leaf_clusters(
        &self,
        workspace: &TreePreprocess,
        assigned_hierarchy: &[usize],
    ) -> Result<LayoutClusters, Error> {
        let mut discovery = Discovery::new(self, workspace, assigned_hierarchy);
        // Container-local discovery runs in reverse depth-first order. A
        // leaf cluster may rebind neighbors visited by a later container.
        for container in workspace.container_order() {
            discovery.discover_container(container)?;
        }
        Ok(discovery.result)
    }
}

impl<'a> Discovery<'a> {
    fn new(
        graph: &'a LayoutGraph,
        workspace: &'a TreePreprocess,
        assigned_hierarchy: &[usize],
    ) -> Self {
        let endpoints = graph
            .edges
            .iter()
            .map(|edge| (Endpoint::Node(edge.from), Endpoint::Node(edge.to)))
            .collect();
        let mut this = Self {
            graph,
            workspace,
            memberships: vec![None; graph.nodes.len()],
            signatures: (0..graph.nodes.len())
                .map(|_| Signature::default())
                .collect(),
            neighbors: (0..graph.nodes.len()).map(|_| HashSet::new()).collect(),
            eligible: vec![false; graph.nodes.len()],
            edge_order: vec![0; graph.edges.len()],
            result: LayoutClusters {
                clusters: Vec::new(),
                endpoints,
            },
            work: 0,
        };
        for (rank, &edge) in workspace.edges.iter().enumerate() {
            this.edge_order[edge] = rank;
            let edge = &graph.edges[edge];
            this.signatures[edge.from].add(edge.from, edge);
            if edge.from != edge.to {
                this.signatures[edge.to].add(edge.to, edge);
            }
        }
        for &node in &workspace.nodes {
            this.eligible[node] = graph.nodes[node].children.is_empty()
                && workspace.tree_roots[node].is_empty()
                && workspace.incident[node]
                    .iter()
                    .all(|&edge| graph.edges[edge].from != graph.edges[edge].to);
            this.refresh_neighbors(node);
        }
        for &node in assigned_hierarchy {
            this.eligible[node] = false;
        }
        this
    }

    fn discover_container(&mut self, container: usize) -> Result<(), Error> {
        let children = &self.workspace.containers[container];
        for &node in children {
            self.charge(1)?;
            if !self.can_join(node)
                || self.neighbors[node]
                    .iter()
                    .any(|neighbor| matches!(neighbor, Endpoint::Vessel(_)))
            {
                continue;
            }
            let mut members = vec![node];
            for &other in children {
                self.charge(1)?;
                if other == node || !self.can_join(other) || !self.same_shape_and_size(node, other)
                {
                    continue;
                }
                self.charge(self.neighbors[node].len())?;
                if !self.neighbors[node].is_empty()
                    && self.neighbors[node] == self.neighbors[other]
                    && self.signatures[node].matches(&self.signatures[other])
                {
                    members.push(other);
                }
            }
            if members.len() > 1 && self.legal_scope(&members)? {
                self.install(container.checked_sub(1), members)?;
            }
        }
        Ok(())
    }

    fn can_join(&self, node: usize) -> bool {
        self.eligible[node] && self.memberships[node].is_none()
    }

    fn same_shape_and_size(&self, left: usize, right: usize) -> bool {
        let left = &self.graph.nodes[left];
        let right = &self.graph.nodes[right];
        left.shape == right.shape
            && left.width * MAX_ESTIMATED_SIZE_RATIO >= right.width
            && right.width * MAX_ESTIMATED_SIZE_RATIO >= left.width
            && left.height * MAX_ESTIMATED_SIZE_RATIO >= right.height
            && right.height * MAX_ESTIMATED_SIZE_RATIO >= left.height
    }

    fn legal_scope(&mut self, members: &[usize]) -> Result<bool, Error> {
        for &member in members {
            for &edge in &self.workspace.incident[member] {
                let Endpoint::Node(adjacent) = self.adjacent(member, edge) else {
                    continue;
                };
                let mut current = Some(member);
                while let Some(node) = current {
                    self.charge(1)?;
                    if node == adjacent {
                        return Ok(false);
                    }
                    current = self.graph.nodes[node].parent;
                }
            }
        }
        Ok(true)
    }

    fn install(&mut self, container: Option<usize>, nodes: Vec<usize>) -> Result<(), Error> {
        self.charge(nodes.len().saturating_mul(nodes.len()))?;
        let vessel = self.result.clusters.len();
        let mut incident = HashSet::new();
        for &member in &nodes {
            self.memberships[member] = Some(vessel);
            incident.extend(self.workspace.incident[member].iter().copied());
        }
        let mut edges: Vec<_> = incident.into_iter().collect();
        edges.sort_unstable_by_key(|&edge| self.edge_order[edge]);
        let mut affected = HashSet::new();
        for &edge in &edges {
            self.charge(1)?;
            let (source, target) = &mut self.result.endpoints[edge];
            for endpoint in [source, target] {
                if let Endpoint::Node(node) = *endpoint {
                    affected.insert(node);
                    if self.memberships[node] == Some(vessel) {
                        *endpoint = Endpoint::Vessel(vessel);
                    }
                }
            }
        }
        for node in affected {
            if self.memberships[node].is_none() {
                self.charge(self.workspace.incident[node].len())?;
                self.refresh_neighbors(node);
            }
        }
        let fixed_size = nodes
            .iter()
            .any(|&node| self.graph.nodes[node].shape == Shape::Circle);
        self.result.clusters.push(LeafCluster {
            container,
            nodes,
            edges,
            fixed_size,
        });
        Ok(())
    }

    fn refresh_neighbors(&mut self, node: usize) {
        self.neighbors[node] = self.workspace.incident[node]
            .iter()
            .map(|&edge| self.adjacent(node, edge))
            .collect();
    }

    fn adjacent(&self, node: usize, edge: usize) -> Endpoint {
        let (from, to) = self.result.endpoints[edge];
        if from == Endpoint::Node(node) {
            to
        } else {
            from
        }
    }

    fn charge(&mut self, units: usize) -> Result<(), Error> {
        self.work = self.work.saturating_add(units);
        if self.work > MAX_WORK {
            return Err(Error::WorkLimit);
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
pub(crate) struct Capture {
    clusters: Vec<ClusterCapture>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
#[serde(rename_all = "camelCase")]
struct ClusterCapture {
    container: Option<i64>,
    nodes: Vec<i64>,
    fixed_size: bool,
    geometry: Option<VesselGeometry>,
    edges: Vec<CapturedEndpoint>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
struct CapturedEndpoint {
    id: i64,
    from: Option<i64>,
    to: Option<i64>,
}

impl LayoutClusters {
    pub(crate) fn capture(&self, graph: &LayoutGraph) -> Capture {
        let endpoint = |endpoint| match endpoint {
            Endpoint::Node(node) => Some(graph.nodes[node].id),
            Endpoint::Vessel(_) => None,
        };
        Capture {
            clusters: self
                .clusters
                .iter()
                .map(|cluster| ClusterCapture {
                    container: cluster.container.map(|node| graph.nodes[node].id),
                    nodes: cluster
                        .nodes
                        .iter()
                        .map(|&node| graph.nodes[node].id)
                        .collect(),
                    fixed_size: cluster.fixed_size,
                    geometry: cluster.geometry(graph, || None),
                    edges: cluster
                        .edges
                        .iter()
                        .map(|&edge| {
                            let (from, to) = self.endpoints[edge];
                            CapturedEndpoint {
                                id: graph.edges[edge].id,
                                from: endpoint(from),
                                to: endpoint(to),
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests;
