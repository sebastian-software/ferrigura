//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/preserveorder/ModelOrderPortComparator.java,
//!   plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/preserveorder/ModelOrderNodeComparator.java,
//!   and plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/SortByInputModelProcessor.java
//!
//! The comparators that keep the input order of the ports and nodes of a layer, as
//! `SortByInputModelProcessor` uses them. Each comparator remembers the orders it has
//! established, so that a comparison which follows from earlier ones is answered without
//! the edges. The result depends on the sequence of comparisons, so the callers sort with
//! `java::sort`, the merge sort that GWT compiles `Collections.sort` to.
//!
//! Ports carry no model order here: the importer rejects ELK ports, which are the only ports
//! `ElkGraphImporter` numbers. The branches that compare port orders therefore fall through,
//! as the Java does for a port without one.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::core::data::enums::{LongEdgeOrderingStrategy, NodeType, OrderingStrategy, PortSide};
use crate::layered::lgraph::LGraph;

/// `Integer.MAX_VALUE`: the order of a target node that has no edge yet, and of a dummy under
/// `DUMMY_NODE_OVER`.
const INTEGER_MAX: i64 = i32::MAX as i64;

/// `InternalProperties.MODEL_ORDER` of an edge as an `int`. The importer gives every edge an
/// order when the input order is used, so the zero fallback is not reached on those graphs.
fn edge_order(graph: &LGraph, edge: usize) -> i64 {
    graph.edges[edge]
        .model_order
        .map_or(0, |order| order as i64)
}

/// `InternalProperties.MODEL_ORDER` of a node, when it has one.
fn node_order(graph: &LGraph, node: usize) -> Option<i64> {
    graph.nodes[node].model_order.map(|order| order as i64)
}

/// `PortSide.ordinal()`, the order `PortSideComparator` compares.
fn side_ordinal(side: PortSide) -> usize {
    side as usize
}

/// The node an edge leaves from.
fn source_node(graph: &LGraph, edge: usize) -> usize {
    graph.ports[graph.edges[edge].source].node
}

/// The first outgoing edge of a node, in the order of its ports: `LNode.getOutgoingEdges().iterator().next()`.
fn first_outgoing_edge(graph: &LGraph, node: usize) -> Option<usize> {
    graph.nodes[node]
        .ports
        .iter()
        .find_map(|&port| graph.ports[port].outgoing.first().copied())
}

/// `SortByInputModelProcessor.getTargetNode`: the normal node that the first outgoing edge of
/// a port reaches, through the dummy nodes of a long edge. `None` when a dummy has no way on.
fn target_node(graph: &LGraph, port: usize) -> Option<usize> {
    let mut edge = graph.ports[port].outgoing[0];
    loop {
        let node = graph.ports[graph.edges[edge].target].node;
        if let Some(target) = graph.nodes[node].long_edge_target {
            return Some(graph.ports[target].node);
        }
        if graph.nodes[node].node_type == NodeType::NORMAL {
            return Some(node);
        }
        edge = first_outgoing_edge(graph, node)?;
    }
}

/// `LONG_EDGE_TARGET_NODE` of a port: the target of a port with outgoing edges. Java sets it
/// on those ports when the node's targets are preprocessed; the graph does not change while
/// the ports are sorted, so it is computed here instead.
fn long_edge_target_node(graph: &LGraph, port: usize) -> Option<usize> {
    if graph.ports[port].outgoing.is_empty() {
        return None;
    }
    target_node(graph, port)
}

/// `SortByInputModelProcessor.longEdgeTargetNodePreprocessing`: for each target node of the
/// node's ports, the smallest model order of the edges that lead to it. Reversed edges do not count.
///
/// Java keeps the result on the node as `TARGET_NODE_MODEL_ORDER`. The processor asks once
/// per node, and the sweep's copy of this count is only reached with a model order counter, so
/// the result is not kept.
pub fn long_edge_target_node_preprocessing(graph: &LGraph, node: usize) -> HashMap<usize, i64> {
    let mut target_node_model_order = HashMap::new();
    for &port in &graph.nodes[node].ports {
        if graph.ports[port].outgoing.is_empty() {
            continue;
        }
        let Some(target) = target_node(graph, port) else {
            continue;
        };
        let previous_order = target_node_model_order
            .get(&target)
            .copied()
            .unwrap_or(INTEGER_MAX);
        let edge = graph.ports[port].outgoing[0];
        if !graph.edges[edge].reversed {
            target_node_model_order.insert(target, edge_order(graph, edge).min(previous_order));
        }
    }
    target_node_model_order
}

/// `if (!map.containsKey(key)) map.put(key, new HashSet<>()) else if (map.get(key).contains(value))`:
/// whether the entry for `key` already holds `value`. An absent entry is created.
fn has_entry_with(map: &mut HashMap<usize, HashSet<usize>>, key: usize, value: usize) -> bool {
    match map.get(&key) {
        Some(set) => set.contains(&value),
        None => {
            map.insert(key, HashSet::new());
            false
        }
    }
}

/// The orders a comparator has established: for each node or port, the ones it is bigger
/// than and the ones it is smaller than. Java's `biggerThan` and `smallerThan`.
#[derive(Debug, Default)]
struct Relations {
    bigger_than: HashMap<usize, HashSet<usize>>,
    smaller_than: HashMap<usize, HashSet<usize>>,
}

impl Relations {
    /// The first checks of both comparators: an order established earlier answers the
    /// comparison, and each element gets its entries.
    fn known_order(&mut self, first: usize, second: usize) -> Option<Ordering> {
        if has_entry_with(&mut self.bigger_than, first, second) {
            return Some(Ordering::Greater);
        }
        if has_entry_with(&mut self.bigger_than, second, first) {
            return Some(Ordering::Less);
        }
        if has_entry_with(&mut self.smaller_than, first, second) {
            return Some(Ordering::Less);
        }
        // Java tests `biggerThan` here, and only when `smallerThan` had the entry already.
        let known = self.smaller_than.contains_key(&second);
        self.smaller_than.entry(second).or_default();
        if known && self.bigger_than[&second].contains(&first) {
            return Some(Ordering::Greater);
        }
        None
    }

    /// `updateBiggerAndSmallerAssociations`: records that `bigger` comes after `smaller`, and
    /// closes the relation over what both already relate to.
    fn record(&mut self, bigger: usize, smaller: usize) {
        let smaller_bigger_than = self.bigger_than.get(&smaller).cloned().unwrap_or_default();
        let bigger_smaller_than = self.smaller_than.get(&bigger).cloned().unwrap_or_default();
        self.bigger_than.entry(bigger).or_default().insert(smaller);
        self.smaller_than.entry(smaller).or_default().insert(bigger);
        for &very_small in &smaller_bigger_than {
            self.bigger_than
                .entry(bigger)
                .or_default()
                .insert(very_small);
            self.smaller_than
                .entry(very_small)
                .or_default()
                .insert(bigger);
            self.smaller_than
                .entry(very_small)
                .or_default()
                .extend(bigger_smaller_than.iter().copied());
        }
        for &very_big in &bigger_smaller_than {
            self.smaller_than
                .entry(smaller)
                .or_default()
                .insert(very_big);
            self.bigger_than
                .entry(very_big)
                .or_default()
                .insert(smaller);
            self.bigger_than
                .entry(very_big)
                .or_default()
                .extend(smaller_bigger_than.iter().copied());
        }
    }
}

/// `ModelOrderPortComparator`: orders the ports of one node by their edges' model order, or,
/// for ports whose edges lead to the same long edge target, by that target.
pub struct ModelOrderPortComparator {
    /// The target nodes of the ports, with their model order, from
    /// `long_edge_target_node_preprocessing`.
    target_node_model_order: HashMap<usize, i64>,
    /// `CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER`.
    port_model_order: bool,
    /// The nodes of the previous layer, in their order.
    previous_layer: Vec<usize>,
    strategy: OrderingStrategy,
    relations: Relations,
}

impl ModelOrderPortComparator {
    pub fn new(
        previous_layer: Vec<usize>,
        strategy: OrderingStrategy,
        target_node_model_order: HashMap<usize, i64>,
        port_model_order: bool,
    ) -> Self {
        ModelOrderPortComparator {
            target_node_model_order,
            port_model_order,
            previous_layer,
            strategy,
            relations: Relations::default(),
        }
    }

    /// `ModelOrderPortComparator.compare`, on indices into `LGraph::ports`.
    pub fn compare(&mut self, graph: &LGraph, first: usize, second: usize) -> Ordering {
        let (mut p1, mut p2) = (first, second);
        // The WEST ports sort descending, so the pair is exchanged there.
        if self.port_model_order
            && graph.ports[p1].side == PortSide::WEST
            && graph.ports[p2].side == PortSide::WEST
        {
            (p1, p2) = (p2, p1);
        }
        if let Some(order) = self.relations.known_order(p1, p2) {
            return order;
        }
        let (side1, side2) = (graph.ports[p1].side, graph.ports[p2].side);
        if side1 != side2 {
            let result = side_ordinal(side1).cmp(&side_ordinal(side2));
            if result == Ordering::Less {
                self.relations.record(p2, p1);
            } else {
                self.relations.record(p1, p2);
            }
            return result;
        }
        let in1 = &graph.ports[p1].incoming;
        let in2 = &graph.ports[p2].incoming;
        let out1 = &graph.ports[p1].outgoing;
        let out2 = &graph.ports[p2].outgoing;

        // Incoming edges: the ports order by the model order of their edges, or by the
        // position of their source nodes in the previous layer.
        if !in1.is_empty() && !in2.is_empty() {
            let p1_node = source_node(graph, in1[0]);
            let p2_node = source_node(graph, in2[0]);
            if p1_node == p2_node {
                let (p1_order, p2_order) = (edge_order(graph, in1[0]), edge_order(graph, in2[0]));
                if p1_order > p2_order {
                    self.relations.record(p1, p2);
                } else {
                    self.relations.record(p2, p1);
                }
                return p1_order.cmp(&p2_order);
            }
            for &previous in &self.previous_layer {
                if previous == p1_node {
                    self.relations.record(p1, p2);
                    return Ordering::Greater;
                } else if previous == p2_node {
                    self.relations.record(p2, p1);
                    return Ordering::Less;
                }
            }
        }

        // Outgoing edges: the ports order by the target of their edges, and then by the
        // model order of the edges, as the Java does.
        if !out1.is_empty() && !out2.is_empty() {
            let target1 = long_edge_target_node(graph, p1);
            let target2 = long_edge_target_node(graph, p2);
            if self.strategy == OrderingStrategy::PREFER_NODES
                && let (Some(t1), Some(t2)) = (target1, target2)
                && let (Some(order1), Some(order2)) = (node_order(graph, t1), node_order(graph, t2))
            {
                if order1 > order2 {
                    self.relations.record(p1, p2);
                } else {
                    self.relations.record(p2, p1);
                }
                return order1.cmp(&order2);
            }
            let mut p1_order = edge_order(graph, out1[0]);
            // Java reads the second port's order from the first port's edge. The port order
            // `p2Order` depends on is kept as it is written.
            let mut p2_order = if graph.edges[out2[0]].model_order.is_some() {
                edge_order(graph, out1[0])
            } else {
                0
            };
            if target1.is_some() && target1 == target2 {
                let reversed1 = graph.edges[out1[0]].reversed;
                let reversed2 = graph.edges[out2[0]].reversed;
                if reversed1 && !reversed2 {
                    self.relations.record(p1, p2);
                    return Ordering::Greater;
                } else if !reversed1 && reversed2 {
                    self.relations.record(p2, p1);
                    return Ordering::Less;
                }
                if p1_order > p2_order {
                    self.relations.record(p1, p2);
                } else {
                    self.relations.record(p2, p1);
                }
                return p1_order.cmp(&p2_order);
            }
            if let Some(&order) = target1.and_then(|t| self.target_node_model_order.get(&t)) {
                p1_order = order;
            }
            if let Some(&order) = target2.and_then(|t| self.target_node_model_order.get(&t)) {
                p2_order = order;
            }
            if p1_order > p2_order {
                self.relations.record(p1, p2);
            } else {
                self.relations.record(p2, p1);
            }
            return p1_order.cmp(&p2_order);
        }

        // Outgoing ports go before incoming ones.
        if !in1.is_empty() && !out2.is_empty() {
            self.relations.record(p1, p2);
            Ordering::Greater
        } else if !out1.is_empty() && !in2.is_empty() {
            self.relations.record(p2, p1);
            Ordering::Less
        } else {
            // Ports without edges on both sides carry no model order (see the module note).
            self.relations.record(p2, p1);
            Ordering::Less
        }
    }
}

/// `ModelOrderNodeComparator`: orders the nodes of one layer by their model order. Nodes without
/// one (the dummies) and the `PREFER_EDGES` strategy order them by the nodes they connect to.
pub struct ModelOrderNodeComparator {
    /// The nodes of the previous layer, in their order.
    previous_layer: Vec<usize>,
    ordering_strategy: OrderingStrategy,
    /// The order of nodes that have no connection to the previous layer.
    long_edge_node_order: LongEdgeOrderingStrategy,
    relations: Relations,
}

impl ModelOrderNodeComparator {
    pub fn new(
        previous_layer: Vec<usize>,
        ordering_strategy: OrderingStrategy,
        long_edge_node_order: LongEdgeOrderingStrategy,
    ) -> Self {
        ModelOrderNodeComparator {
            previous_layer,
            ordering_strategy,
            long_edge_node_order,
            relations: Relations::default(),
        }
    }

    /// `ModelOrderNodeComparator.compare`, on indices into `LGraph::nodes`.
    pub fn compare(&mut self, graph: &LGraph, n1: usize, n2: usize) -> Ordering {
        if let Some(order) = self.relations.known_order(n1, n2) {
            return order;
        }
        let has1 = graph.nodes[n1].model_order.is_some();
        let has2 = graph.nodes[n2].model_order.is_some();
        if self.ordering_strategy == OrderingStrategy::PREFER_EDGES || !has1 || !has2 {
            // The first port of each node that connects to another layer decides.
            if let (Some(source1), Some(source2)) = (
                previous_layer_source(graph, n1),
                previous_layer_source(graph, n2),
            ) {
                let p1_node = graph.ports[source1].node;
                let p2_node = graph.ports[source2].node;
                if p1_node == p2_node {
                    // Both connect to one node: the port order there says which comes first.
                    for &port in &graph.nodes[p1_node].ports {
                        if port == source1 {
                            self.relations.record(n2, n1);
                            return Ordering::Less;
                        } else if port == source2 {
                            self.relations.record(n1, n2);
                            return Ordering::Greater;
                        }
                    }
                    return self
                        .connected_order(graph, n1)
                        .cmp(&self.connected_order(graph, n2));
                }
                // Otherwise the nodes they connect to decide, as the previous layer is ordered.
                for &previous in &self.previous_layer {
                    if previous == p1_node {
                        self.relations.record(n2, n1);
                        return Ordering::Less;
                    } else if previous == p2_node {
                        self.relations.record(n1, n2);
                        return Ordering::Greater;
                    }
                }
            }
            // One node has no model order, so the edges decide.
            if !has1 || !has2 {
                let order1 = self.connected_order(graph, n1);
                let order2 = self.connected_order(graph, n2);
                if order1 > order2 {
                    self.relations.record(n1, n2);
                } else {
                    self.relations.record(n2, n1);
                }
                return order1.cmp(&order2);
            }
            // Both are normal nodes that do not connect to the previous layer: their model order
            // decides below.
        }
        let order1 = node_order(graph, n1).unwrap_or(0);
        let order2 = node_order(graph, n2).unwrap_or(0);
        if order1 > order2 {
            self.relations.record(n1, n2);
        } else {
            self.relations.record(n2, n1);
        }
        order1.cmp(&order2)
    }

    /// `ModelOrderNodeComparator.getModelOrderFromConnectedEdges`: the model order of the first
    /// incoming edge of the node, or the long edge order when there is none.
    fn connected_order(&self, graph: &LGraph, node: usize) -> i64 {
        let source_port = graph.nodes[node]
            .ports
            .iter()
            .copied()
            .find(|&port| !graph.ports[port].incoming.is_empty());
        if let Some(port) = source_port {
            return edge_order(graph, graph.ports[port].incoming[0]);
        }
        match self.long_edge_node_order {
            LongEdgeOrderingStrategy::DUMMY_NODE_OVER => INTEGER_MAX,
            LongEdgeOrderingStrategy::DUMMY_NODE_UNDER => -1,
            LongEdgeOrderingStrategy::EQUAL => 0,
        }
    }
}

/// The port of a node that connects to another layer, as `ModelOrderNodeComparator` finds it:
/// the last port whose first incoming edge comes from a node in a different layer.
fn previous_layer_source(graph: &LGraph, node: usize) -> Option<usize> {
    let mut source = None;
    for &port in &graph.nodes[node].ports {
        if let Some(&edge) = graph.ports[port].incoming.first() {
            let source_port = graph.edges[edge].source;
            let source_layer = graph.nodes[graph.ports[source_port].node].layer;
            if source_layer != graph.nodes[node].layer {
                source = Some(source_port);
            }
        }
    }
    source
}
