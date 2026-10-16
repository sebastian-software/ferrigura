//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p1cycles/DepthFirstCycleBreaker.java
//!   and LEdge.reverse in graph/LEdge.java
//!
//! The depth-first cycle breaker, phase 1. A depth-first search from the sources, then from
//! every node the search has not reached, reverses the edges that close a cycle.

use crate::core::data::JavaEnum;
use crate::core::data::OptionValue;
use crate::core::data::enums::EdgeLabelPlacement;
use std::collections::VecDeque;

use crate::java::random::Random;
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// `DepthFirstCycleBreaker.process`: reverses the edges that close a cycle. The edges are
/// reversed after the search, in the order the search found them.
pub fn break_cycles(graph: &mut LGraph) {
    let nodes = graph.layerless_nodes.clone();
    let mut position = vec![0; graph.nodes.len()];
    for (index, &node) in nodes.iter().enumerate() {
        position[node] = index;
    }
    let sources: Vec<usize> = nodes
        .iter()
        .copied()
        .filter(|&node| incoming_edges(graph, node).is_empty())
        .collect();
    let to_reverse = {
        let mut search = Search {
            graph,
            position,
            visited: vec![false; nodes.len()],
            active: vec![false; nodes.len()],
            to_reverse: Vec::new(),
        };
        for source in sources {
            search.visit(source);
        }
        for (index, &node) in nodes.iter().enumerate() {
            if !search.visited[index] {
                search.visit(node);
            }
        }
        search.to_reverse
    };
    for edge in to_reverse {
        reverse_edge(graph, edge);
        graph.cyclic = true;
    }
}

/// `GreedyCycleBreaker.process`: the nodes are taken off one at a time. Sinks go to the right and
/// sources to the left; when neither is left, a node with the largest outflow, drawn at random,
/// goes to the left. The edges that then point left are reversed. The edge priorities weigh the flow.
pub fn break_cycles_greedy(graph: &mut LGraph, random: &mut Random) {
    let nodes = graph.layerless_nodes.clone();
    let count = nodes.len();
    let mut flow = Flow::new(graph, &nodes);
    let mut unprocessed = count as i64;
    // The next ranks for sinks (from the right, negative) and for sources (from the left).
    let mut next_right: i32 = -1;
    let mut next_left: i32 = 1;
    while unprocessed > 0 {
        while let Some(sink) = flow.sinks.pop_front() {
            flow.mark[sink] = next_right;
            next_right -= 1;
            flow.update_neighbors(graph, nodes[sink]);
            unprocessed -= 1;
        }
        while let Some(source) = flow.sources.pop_front() {
            flow.mark[source] = next_left;
            next_left += 1;
            flow.update_neighbors(graph, nodes[source]);
            unprocessed -= 1;
        }
        if unprocessed > 0 {
            let mut max_outflow = i32::MIN;
            let mut max_nodes: Vec<usize> = Vec::new();
            for index in 0..count {
                if flow.mark[index] == 0 {
                    let outflow = flow.outdeg[index].wrapping_sub(flow.indeg[index]);
                    if outflow >= max_outflow {
                        if outflow > max_outflow {
                            max_nodes.clear();
                            max_outflow = outflow;
                        }
                        max_nodes.push(index);
                    }
                }
            }
            // An unmarked node is always a candidate here, so the list is not empty.
            if max_nodes.is_empty() {
                break;
            }
            // `chooseNodeWithMaxOutflow`: `random.nextInt(nodes.size())`.
            let draw = random.next_int_bounded(i32::try_from(max_nodes.len()).unwrap_or(i32::MAX));
            let chosen = max_nodes[draw as usize];
            flow.mark[chosen] = next_left;
            next_left += 1;
            flow.update_neighbors(graph, nodes[chosen]);
            unprocessed -= 1;
        }
    }
    // The sinks have negative ranks; they move past the sources.
    let shift_base = i32::try_from(count + 1).unwrap_or(i32::MAX);
    for mark in flow.mark.iter_mut() {
        if *mark < 0 {
            *mark += shift_base;
        }
    }
    for &node in &nodes {
        for port in graph.nodes[node].ports.clone() {
            for edge in graph.ports[port].outgoing.clone() {
                let target = graph.ports[graph.edges[edge].target].node;
                if flow.mark[flow.id[node]] > flow.mark[flow.id[target]] {
                    reverse_edge(graph, edge);
                    graph.cyclic = true;
                }
            }
        }
    }
}

/// The arrays of `GreedyCycleBreaker`. Positions are a node's place in the layerless list, which
/// the Java code keeps as `node.id`.
struct Flow {
    /// The position of each graph node (indexed by node), for the layerless nodes.
    id: Vec<usize>,
    indeg: Vec<i32>,
    outdeg: Vec<i32>,
    mark: Vec<i32>,
    sources: VecDeque<usize>,
    sinks: VecDeque<usize>,
}

/// The priority of an edge in the flow, `edge.getProperty(PRIORITY_DIRECTION)`.
fn flow_priority(graph: &LGraph, edge: usize) -> i32 {
    graph.edges[edge]
        .properties
        .int_with_default(options::PRIORITY_DIRECTION)
}

impl Flow {
    /// The in and out flow of each node, and the first sinks and sources.
    fn new(graph: &LGraph, nodes: &[usize]) -> Self {
        let mut id = vec![0; graph.nodes.len()];
        for (index, &node) in nodes.iter().enumerate() {
            id[node] = index;
        }
        let mut flow = Flow {
            id,
            indeg: vec![0; nodes.len()],
            outdeg: vec![0; nodes.len()],
            mark: vec![0; nodes.len()],
            sources: VecDeque::new(),
            sinks: VecDeque::new(),
        };
        for (index, &node) in nodes.iter().enumerate() {
            for &port in &graph.nodes[node].ports {
                // Self-loops do not count.
                for &edge in &graph.ports[port].incoming {
                    if graph.ports[graph.edges[edge].source].node == node {
                        continue;
                    }
                    let priority = flow_priority(graph, edge);
                    let weight = if priority > 0 {
                        priority.wrapping_add(1)
                    } else {
                        1
                    };
                    flow.indeg[index] = flow.indeg[index].wrapping_add(weight);
                }
                for &edge in &graph.ports[port].outgoing {
                    if graph.ports[graph.edges[edge].target].node == node {
                        continue;
                    }
                    let priority = flow_priority(graph, edge);
                    let weight = if priority > 0 {
                        priority.wrapping_add(1)
                    } else {
                        1
                    };
                    flow.outdeg[index] = flow.outdeg[index].wrapping_add(weight);
                }
            }
            if flow.outdeg[index] == 0 {
                flow.sinks.push_back(index);
            } else if flow.indeg[index] == 0 {
                flow.sources.push_back(index);
            }
        }
        flow
    }

    /// `updateNeighbors`: a node is taken off, so each neighbour that is not marked yet loses
    /// the flow of the edge between them, and joins the sinks or sources when that flow ends.
    fn update_neighbors(&mut self, graph: &LGraph, node: usize) {
        for &port in &graph.nodes[node].ports {
            // `getConnectedEdges`: the incoming edges, then the outgoing ones.
            for &edge in graph.ports[port]
                .incoming
                .iter()
                .chain(&graph.ports[port].outgoing)
            {
                let edge_data = &graph.edges[edge];
                let connected = if edge_data.source == port {
                    edge_data.target
                } else {
                    edge_data.source
                };
                let endpoint = graph.ports[connected].node;
                // Self-loops are excluded.
                if endpoint == node {
                    continue;
                }
                let priority = flow_priority(graph, edge).max(0);
                let index = self.id[endpoint];
                if self.mark[index] != 0 {
                    continue;
                }
                if edge_data.target == connected {
                    self.indeg[index] = self.indeg[index].wrapping_sub(priority.wrapping_add(1));
                    if self.indeg[index] <= 0 && self.outdeg[index] > 0 {
                        self.sources.push_back(index);
                    }
                } else {
                    self.outdeg[index] = self.outdeg[index].wrapping_sub(priority.wrapping_add(1));
                    if self.outdeg[index] <= 0 && self.indeg[index] > 0 {
                        self.sinks.push_back(index);
                    }
                }
            }
        }
    }
}

/// The depth-first search. `visited` and `active` are indexed by a node's place in the
/// layerless list, as the Java `node.id` is.
struct Search<'a> {
    graph: &'a LGraph,
    position: Vec<usize>,
    visited: Vec<bool>,
    active: Vec<bool>,
    to_reverse: Vec<usize>,
}

impl Search<'_> {
    fn visit(&mut self, node: usize) {
        let id = self.position[node];
        if self.visited[id] {
            return;
        }
        self.visited[id] = true;
        self.active[id] = true;
        for edge in outgoing_edges(self.graph, node) {
            if is_self_loop(self.graph, edge) {
                continue;
            }
            let target = self.graph.ports[self.graph.edges[edge].target].node;
            if self.active[self.position[target]] {
                self.to_reverse.push(edge);
            } else {
                self.visit(target);
            }
        }
        self.active[id] = false;
    }
}

/// `LNode.getOutgoingEdges()`: the outgoing edges of the node's ports, port by port.
pub fn outgoing_edges(graph: &LGraph, node: usize) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| graph.ports[port].outgoing.iter().copied())
        .collect()
}

/// `LNode.getIncomingEdges()`: the incoming edges of the node's ports, port by port.
pub fn incoming_edges(graph: &LGraph, node: usize) -> Vec<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .flat_map(|&port| graph.ports[port].incoming.iter().copied())
        .collect()
}

/// `LEdge.isSelfLoop()`: both ends of the edge are on the same node.
fn is_self_loop(graph: &LGraph, edge: usize) -> bool {
    let edge = &graph.edges[edge];
    graph.ports[edge.source].node == graph.ports[edge.target].node
}

/// `LEdge.reverse(graph, true)` for an edge without collector ports: the ends swap, the edge
/// moves to the other ports' lists, head and tail labels swap, and the bend points reverse.
pub fn reverse_edge(graph: &mut LGraph, edge: usize) {
    let old_source = graph.edges[edge].source;
    let old_target = graph.edges[edge].target;
    remove_edge(&mut graph.ports[old_source].outgoing, edge);
    remove_edge(&mut graph.ports[old_target].incoming, edge);
    graph.ports[old_target].outgoing.push(edge);
    graph.ports[old_source].incoming.push(edge);
    let label_ids = graph.edges[edge].labels.clone();
    let edge = &mut graph.edges[edge];
    edge.source = old_target;
    edge.target = old_source;
    for id in label_ids {
        let label = &mut graph.labels[id];
        let placement = label
            .properties
            .enum_of::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT);
        let flipped = match placement {
            Some(EdgeLabelPlacement::TAIL) => Some(EdgeLabelPlacement::HEAD),
            Some(EdgeLabelPlacement::HEAD) => Some(EdgeLabelPlacement::TAIL),
            _ => None,
        };
        if let Some(flipped) = flipped {
            label.properties.set(
                options::EDGE_LABELS_PLACEMENT,
                Some(OptionValue::Enum {
                    class: "EdgeLabelPlacement",
                    name: flipped.java_name().to_string(),
                }),
            );
        }
    }
    edge.reversed = !edge.reversed;
    edge.bend_points.reverse();
}

/// `ReversedEdgeRestorer.process`: every reversed edge is reversed back, as the outgoing edges
/// of each port are met, layer by layer. Each port's list is copied when it is reached, as the
/// Java `toEdgeArray` does.
pub fn restore_reversed_edges(graph: &mut LGraph) {
    for layer in graph.layers.clone() {
        for node in layer {
            for port in graph.nodes[node].ports.clone() {
                for edge in graph.ports[port].outgoing.clone() {
                    if graph.edges[edge].reversed {
                        reverse_edge(graph, edge);
                    }
                }
            }
        }
    }
}

/// `ArrayList.remove(edge)`: the first occurrence of the edge leaves the list.
fn remove_edge(list: &mut Vec<usize>, edge: usize) {
    if let Some(index) = list.iter().position(|&found| found == edge) {
        list.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::layered::graph_util::create_port;
    use crate::layered::lgraph::{LEdge, LNode, PortType};

    /// Three nodes in model order, DOWN, and the edges given as (from, to) pairs.
    fn graph_with(count: usize, edges: &[(usize, usize)]) -> LGraph {
        let mut graph = LGraph {
            nodes: (0..count).map(|_| LNode::default()).collect(),
            layerless_nodes: (0..count).collect(),
            ..LGraph::default()
        };
        graph.properties.set(
            options::DIRECTION,
            Some(OptionValue::Enum {
                class: "Direction",
                name: "DOWN".to_string(),
            }),
        );
        for &(from, to) in edges {
            let source = create_port(&mut graph, from, PortType::OUTPUT);
            let target = create_port(&mut graph, to, PortType::INPUT);
            let index = graph.edges.len();
            graph.ports[source].outgoing.push(index);
            graph.ports[target].incoming.push(index);
            graph.edges.push(LEdge {
                source,
                target,
                ..LEdge::default()
            });
        }
        graph
    }

    #[test]
    fn the_edge_that_closes_a_cycle_is_reversed() {
        let mut graph = graph_with(2, &[(0, 1), (1, 0)]);
        break_cycles(&mut graph);
        assert!(graph.cyclic);
        assert!(!graph.edges[0].reversed);
        assert!(graph.edges[1].reversed);
        assert_eq!(graph.ports[graph.edges[1].source].node, 0);
        assert_eq!(graph.ports[graph.edges[1].target].node, 1);
    }

    #[test]
    fn an_acyclic_graph_keeps_its_edges() {
        let mut graph = graph_with(3, &[(0, 1), (1, 2), (0, 2)]);
        break_cycles(&mut graph);
        assert!(!graph.cyclic);
        assert!(graph.edges.iter().all(|edge| !edge.reversed));
    }

    #[test]
    fn greedy_breaks_a_two_cycle_with_one_reversed_edge() {
        let mut graph = graph_with(2, &[(0, 1), (1, 0)]);
        break_cycles_greedy(&mut graph, &mut Random::new(1));
        assert!(graph.cyclic);
        assert_eq!(graph.edges.iter().filter(|edge| edge.reversed).count(), 1);
    }

    #[test]
    fn greedy_keeps_the_edges_of_an_acyclic_graph() {
        let mut graph = graph_with(3, &[(0, 1), (1, 2), (0, 2)]);
        break_cycles_greedy(&mut graph, &mut Random::new(1));
        assert!(!graph.cyclic);
        assert!(graph.edges.iter().all(|edge| !edge.reversed));
    }

    #[test]
    fn a_reversed_edge_is_restored() {
        let mut graph = graph_with(2, &[(0, 1), (1, 0)]);
        break_cycles(&mut graph);
        graph.layers = vec![vec![0], vec![1]];
        graph.nodes[0].layer = Some(0);
        graph.nodes[1].layer = Some(1);
        restore_reversed_edges(&mut graph);
        assert!(!graph.edges[1].reversed);
        assert_eq!(graph.ports[graph.edges[1].source].node, 1);
        assert_eq!(graph.ports[graph.edges[1].target].node, 0);
    }
}
