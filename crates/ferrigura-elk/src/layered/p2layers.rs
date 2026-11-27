//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p2layers/NetworkSimplexLayerer.java
//!
//! The network simplex layerer, phase 2. The layerless nodes are split into connected
//! components, largest first; each component is layered by the network simplex solver,
//! and its nodes join the layers the solver gives them.

use std::collections::VecDeque;

use crate::common::networksimplex::{NEdge, NGraph, NNode, NetworkSimplex};
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::p1cycles::outgoing_edges;

/// `NetworkSimplexLayerer.ITER_LIMIT_FACTOR`.
const ITER_LIMIT_FACTOR: i32 = 4;

/// `NetworkSimplexLayerer.process`, after its task's `begin`.
pub fn network_simplex_layering(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let thoroughness = graph.properties.int_with_default(options::THOROUGHNESS) * ITER_LIMIT_FACTOR;
    let nodes = graph.layerless_nodes.clone();
    if nodes.is_empty() {
        return Ok(());
    }
    let components = connected_components(graph, &nodes);
    let multiple_components = components.len() > 1;
    let mut previous_layering: Option<Vec<i32>> = None;
    for component in &components {
        let iteration_limit = thoroughness * (component.len() as f64).sqrt() as i32;
        let mut solver_graph = initialize(graph, component);
        let sub = progress
            .sub_task(monitor, 1.0)
            .ok_or_else(|| LayoutError::Unsupported("subtask of a finished monitor".to_string()))?;
        NetworkSimplex::for_graph(&mut solver_graph)
            .with_iteration_limit(iteration_limit)
            .with_previous_layering(previous_layering.clone())
            .with_balancing(true)
            .execute_with_monitor(progress, sub);
        for &solver_node in solver_graph.nodes() {
            let node = solver_graph.node(solver_node);
            let layer = usize::try_from(node.layer).map_err(|_| {
                LayoutError::Unsupported("a negative layer from the network simplex".to_string())
            })?;
            let Some(origin) = node.origin else {
                continue;
            };
            while graph.layers.len() <= layer {
                graph.layers.push(Vec::new());
            }
            graph.nodes[origin].layer = Some(layer);
            graph.layers[layer].push(origin);
        }
        if multiple_components {
            previous_layering = Some(
                graph
                    .layers
                    .iter()
                    .map(|layer| layer.len() as i32)
                    .collect(),
            );
        }
    }
    graph.layerless_nodes.clear();
    Ok(())
}

/// `connectedComponents`: the components of the given nodes, the largest first and the others
/// in the order they are found.
fn connected_components(graph: &LGraph, nodes: &[usize]) -> Vec<Vec<usize>> {
    let mut visited = vec![false; graph.nodes.len()];
    let mut components: VecDeque<Vec<usize>> = VecDeque::new();
    for &node in nodes {
        if !visited[node] {
            let mut component = Vec::new();
            depth_first(graph, node, &mut visited, &mut component);
            if components
                .front()
                .is_none_or(|first| first.len() < component.len())
            {
                components.push_front(component);
            } else {
                components.push_back(component);
            }
        }
    }
    components.into_iter().collect()
}

/// `connectedComponentsDFS`: the nodes reached from `start`, through the connected edges of
/// each port, in the order the search meets them.
fn depth_first(graph: &LGraph, start: usize, visited: &mut [bool], component: &mut Vec<usize>) {
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if visited[node] {
            continue;
        }
        visited[node] = true;
        component.push(node);
        // Push connected edges in reverse port and incoming/outgoing order so LIFO popping visits
        // the exact same sequence as the recursive DFS.
        for &port in graph.nodes[node].ports.iter().rev() {
            let port_data = &graph.ports[port];
            for &edge in port_data
                .outgoing
                .iter()
                .rev()
                .chain(port_data.incoming.iter().rev())
            {
                let opposite = opposite_node(graph, port, edge);
                if !visited[opposite] {
                    stack.push(opposite);
                }
            }
        }
    }
}

/// `getOpposite`: the node at the other end of `edge`, seen from `port`.
fn opposite_node(graph: &LGraph, port: usize, edge: usize) -> usize {
    let edge = &graph.edges[edge];
    let opposite_port = if edge.source == port {
        edge.target
    } else {
        edge.source
    };
    graph.ports[opposite_port].node
}

/// `initialize`: a solver graph for the component, with a node for each of its nodes in order,
/// and an edge for each outgoing edge of them, weighted by the shortness priority.
fn initialize(graph: &LGraph, component: &[usize]) -> NGraph {
    let mut solver = NGraph::new();
    let mut solver_node = vec![usize::MAX; graph.nodes.len()];
    for &node in component {
        let mut solver_node_data = NNode::new();
        solver_node_data.origin = Some(node);
        solver_node[node] = solver.add_node(solver_node_data);
    }
    for &node in component {
        for edge in outgoing_edges(graph, node) {
            let edge_data = &graph.edges[edge];
            if graph.ports[edge_data.source].node == graph.ports[edge_data.target].node {
                continue;
            }
            let shortness = edge_data
                .properties
                .int_with_default(options::PRIORITY_SHORTNESS);
            let source = solver_node[graph.ports[edge_data.source].node];
            let target = solver_node[graph.ports[edge_data.target].node];
            let mut solver_edge = NEdge::new(source, target);
            solver_edge.origin = Some(edge);
            // `1 * Math.max(1, PRIORITY_SHORTNESS)`: the weight is one times the shortness.
            solver_edge.weight = f64::from(shortness.max(1));
            solver_edge.delta = 1;
            solver.add_edge(solver_edge);
        }
    }
    solver
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::PortSide;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    #[test]
    fn connected_components_handles_long_chain_iteratively() {
        let handle = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let n = 10_000;
                let mut graph = LGraph::default();
                for i in 0..n {
                    graph.nodes.push(LNode {
                        ports: vec![i],
                        ..LNode::default()
                    });
                    graph.layerless_nodes.push(i);
                    graph.ports.push(LPort::new(i, PortSide::EAST));
                }
                for i in 0..(n - 1) {
                    graph.edges.push(LEdge {
                        source: i,
                        target: i + 1,
                        ..LEdge::default()
                    });
                    graph.ports[i].outgoing.push(i);
                    graph.ports[i + 1].incoming.push(i);
                }
                let components = connected_components(&graph, &graph.layerless_nodes);
                assert_eq!(components.len(), 1);
                assert_eq!(components[0].len(), n);
            })
            .expect("spawn thread");
        handle
            .join()
            .expect("connected_components long chain without overflow");
    }
}
