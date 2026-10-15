//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/counting/AllCrossingsCounter.java
//!   with CrossingsCounter.java and BinaryIndexedTree.java
//!
//! Counts all crossings of a layer order. Between-layer crossings are counted as in-layer crossings
//! of the folded layers, with a binary indexed tree over the port positions. Where a port has
//! several edges, the hyperedge counter takes over (see `hyperedge`). The north/south counting of the
//! Java is not ported: north and south port dummies are rejected before the sweep starts.
//!
//! Not ported: `CrossingsCounter`'s in-layer-between-nodes and port-swap queries (`switchNodes`,
//! `countCrossingsBetweenPortsInBothOrders`, `countInLayerCrossingsBetweenNodesInBothOrders`), used
//! by the greedy switch heuristic only, and the node cardinalities that only they read.

mod hyperedge;

use crate::core::data::enums::PortSide;
use crate::layered::p3order::{Layout, Net};

/// `BinaryIndexedTree`: a sorted multiset of indices below `max_num`, with ranks. Adding, removing
/// the entries of one index and ranking are logarithmic.
struct BinaryIndexedTree {
    binary_sums: Vec<i32>,
    nums_per_index: Vec<i32>,
    size: i32,
}

impl BinaryIndexedTree {
    fn new(max_num: usize) -> Self {
        BinaryIndexedTree {
            binary_sums: vec![0; max_num + 1],
            nums_per_index: vec![0; max_num],
            size: 0,
        }
    }

    /// Increments the entries at `index`.
    fn add(&mut self, index: usize) {
        self.size += 1;
        self.nums_per_index[index] += 1;
        let mut i = index + 1;
        while i < self.binary_sums.len() {
            self.binary_sums[i] += 1;
            i += i & i.wrapping_neg();
        }
    }

    /// The number of entries before `index`.
    fn rank(&self, index: usize) -> i32 {
        let mut i = index;
        let mut sum = 0;
        while i > 0 {
            sum += self.binary_sums[i];
            i -= i & i.wrapping_neg();
        }
        sum
    }

    fn size(&self) -> i32 {
        self.size
    }

    /// Removes all entries at `index`.
    fn remove_all(&mut self, index: usize) {
        let entries = self.nums_per_index[index];
        if entries == 0 {
            return;
        }
        self.nums_per_index[index] = 0;
        self.size -= entries;
        let mut i = index + 1;
        while i < self.binary_sums.len() {
            self.binary_sums[i] -= entries;
            i += i & i.wrapping_neg();
        }
    }
}

/// `AllCrossingsCounter` with the `CrossingsCounter` and the hyperedge counter it uses.
pub(super) struct AllCrossingsCounter {
    /// `hasHyperEdgesEastOfIndex`: a layer whose gap to the next one has a port with several edges.
    has_hyperedges_east_of_index: Vec<bool>,
    /// `portPositions`, indexed by port. It is shared by the between-layer and the hyperedge counting,
    /// and keeps its values between the calls, as the Java array does.
    port_positions: Vec<usize>,
    /// The tree of the current count.
    index_tree: BinaryIndexedTree,
    /// `ends`: the end positions to add to the tree once a port is counted.
    ends: Vec<usize>,
}

impl AllCrossingsCounter {
    /// `initAtPortLevel`: the hyperedge flags of the initial order.
    pub(super) fn new(net: &Net, layout: &Layout) -> Self {
        let mut has_hyperedges_east_of_index = vec![false; layout.order.len()];
        for (layer, nodes) in layout.order.iter().enumerate() {
            for &node in nodes {
                for &port in &layout.node_ports[node] {
                    if net.degree(port) > 1 {
                        if net.side(port) == PortSide::EAST {
                            has_hyperedges_east_of_index[layer] = true;
                        } else if net.side(port) == PortSide::WEST && layer > 0 {
                            has_hyperedges_east_of_index[layer - 1] = true;
                        }
                    }
                }
            }
        }
        AllCrossingsCounter {
            has_hyperedges_east_of_index,
            port_positions: vec![0; net.graph().ports.len()],
            index_tree: BinaryIndexedTree::new(0),
            ends: Vec::new(),
        }
    }

    /// `countAllCrossings`: the in-layer crossings at the outer sides, and the crossings between
    /// each pair of layers.
    pub(super) fn count_all(&mut self, net: &Net, layout: &Layout) -> i32 {
        let order = &layout.order;
        let Some(last) = order.len().checked_sub(1) else {
            return 0;
        };
        let mut crossings = self.in_layer_on_side(net, layout, &order[0], PortSide::WEST);
        crossings += self.in_layer_on_side(net, layout, &order[last], PortSide::EAST);
        for index in 0..order.len() {
            crossings += self.count_crossings_at(net, layout, index);
        }
        crossings
    }

    /// `countCrossingsAt`. The north/south counting is absent: such dummies are rejected earlier.
    fn count_crossings_at(&mut self, net: &Net, layout: &Layout, index: usize) -> i32 {
        let order = &layout.order;
        if index + 1 >= order.len() {
            return 0;
        }
        let left = &order[index];
        let right = &order[index + 1];
        if self.has_hyperedges_east_of_index[index] {
            let mut total =
                hyperedge::count_crossings(net, layout, &mut self.port_positions, left, right);
            total += self.in_layer_on_side(net, layout, left, PortSide::EAST);
            total += self.in_layer_on_side(net, layout, right, PortSide::WEST);
            total
        } else {
            self.between_layers(net, layout, left, right)
        }
    }

    /// `countCrossingsBetweenLayers`: the ports on the east of the left layer and the west of the right
    /// one, numbered counter-clockwise.
    fn between_layers(
        &mut self,
        net: &Net,
        layout: &Layout,
        left: &[usize],
        right: &[usize],
    ) -> i32 {
        let mut ports = Vec::new();
        self.init_positions(net, layout, left, PortSide::EAST, true, &mut ports);
        self.init_positions(net, layout, right, PortSide::WEST, false, &mut ports);
        self.index_tree = BinaryIndexedTree::new(ports.len());
        self.count_on_ports(net, &ports)
    }

    /// `countInLayerCrossingsOnSide`: the in-layer crossings of the ports on one side of a layer.
    fn in_layer_on_side(
        &mut self,
        net: &Net,
        layout: &Layout,
        nodes: &[usize],
        side: PortSide,
    ) -> i32 {
        let mut ports = Vec::new();
        self.init_positions(net, layout, nodes, side, true, &mut ports);
        self.index_tree = BinaryIndexedTree::new(ports.len());
        self.count_in_layer_on_ports(net, &ports)
    }

    /// `countCrossingsOnPorts`: the crossings of the edges of each port with the edges that end
    /// before them, port by port.
    fn count_on_ports(&mut self, net: &Net, ports: &[usize]) -> i32 {
        let mut crossings = 0;
        for &port in ports {
            let position = self.position(port);
            self.index_tree.remove_all(position);
            for edge in net.connected_edges(port) {
                let end = self.position(net.other_end(edge, port));
                if end > position {
                    crossings += self.index_tree.rank(end);
                    self.ends.push(end);
                }
            }
            while let Some(end) = self.ends.pop() {
                self.index_tree.add(end);
            }
        }
        crossings
    }

    /// `countInLayerCrossingsOnPorts`: as `count_on_ports`, for the in-layer edges; each edge that
    /// leaves the layer counts the entries already in the tree.
    fn count_in_layer_on_ports(&mut self, net: &Net, ports: &[usize]) -> i32 {
        let mut crossings = 0;
        for &port in ports {
            let position = self.position(port);
            self.index_tree.remove_all(position);
            let mut between_layer_edges = 0;
            for edge in net.connected_edges(port) {
                if net.is_in_layer(edge) {
                    let end = self.position(net.other_end(edge, port));
                    if end > position {
                        crossings += self.index_tree.rank(end);
                        self.ends.push(end);
                    }
                } else {
                    between_layer_edges += 1;
                }
            }
            crossings += self.index_tree.size() * between_layer_edges;
            while let Some(end) = self.ends.pop() {
                self.index_tree.add(end);
            }
        }
        crossings
    }

    /// `initPositions`: numbers the ports of the nodes on `side`, node by node, from the top (or the
    /// bottom), and appends them to `ports`.
    fn init_positions(
        &mut self,
        net: &Net,
        layout: &Layout,
        nodes: &[usize],
        side: PortSide,
        top_down: bool,
        ports: &mut Vec<usize>,
    ) {
        let mut num_ports = ports.len();
        let count = nodes.len();
        for step in 0..count {
            let node = if top_down {
                nodes[step]
            } else {
                nodes[count - 1 - step]
            };
            let mut node_ports = net.side_ports(&layout.node_ports[node], side);
            // `getPorts(node, side, topDown)`: east ports are read in list order when top-down and
            // reversed otherwise; the other sides the other way round. That is a reversal exactly when
            // the side being east differs from the reading being top-down.
            if (side == PortSide::EAST) != top_down {
                node_ports.reverse();
            }
            for &port in &node_ports {
                self.port_positions[port] = num_ports;
                num_ports += 1;
            }
            ports.extend(node_ports);
        }
    }

    fn position(&self, port: usize) -> usize {
        self.port_positions[port]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::lgraph::{LEdge, LGraph, LNode, LPort};

    /// A graph of `nodes` nodes with no ports, layered as given.
    fn graph(nodes: usize, layers: Vec<Vec<usize>>) -> LGraph {
        LGraph {
            nodes: vec![LNode::default(); nodes],
            layers,
            ..LGraph::default()
        }
    }

    /// Adds a port on `side` to `node`.
    fn add_port(graph: &mut LGraph, node: usize, side: PortSide) -> usize {
        graph.ports.push(LPort::new(node, side));
        let port = graph.ports.len() - 1;
        graph.nodes[node].ports.push(port);
        port
    }

    /// Adds an edge between two ports.
    fn connect(graph: &mut LGraph, source: usize, target: usize) {
        let edge = graph.edges.len();
        graph.edges.push(LEdge {
            source,
            target,
            ..LEdge::default()
        });
        graph.ports[source].outgoing.push(edge);
        graph.ports[target].incoming.push(edge);
    }

    /// The crossings of `layers` in `graph`, with the port lists as they are.
    fn crossings(graph: &LGraph, layers: Vec<Vec<usize>>) -> i32 {
        let layout = Layout {
            order: layers,
            node_ports: graph.nodes.iter().map(|node| node.ports.clone()).collect(),
        };
        let net = Net::new(graph);
        AllCrossingsCounter::new(&net, &layout).count_all(&net, &layout)
    }

    #[test]
    fn two_edges_that_swap_their_ends_cross_once() {
        // Node 0 -> node 3 and node 1 -> node 2, with the layers [0, 1] and [2, 3]: the edges cross.
        let mut graph = graph(4, vec![vec![0, 1], vec![2, 3]]);
        let p0 = add_port(&mut graph, 0, PortSide::EAST);
        let p1 = add_port(&mut graph, 1, PortSide::EAST);
        let q2 = add_port(&mut graph, 2, PortSide::WEST);
        let q3 = add_port(&mut graph, 3, PortSide::WEST);
        connect(&mut graph, p0, q3);
        connect(&mut graph, p1, q2);
        assert_eq!(crossings(&graph, vec![vec![0, 1], vec![2, 3]]), 1);
        // Swapping the right layer removes the crossing.
        assert_eq!(crossings(&graph, vec![vec![0, 1], vec![3, 2]]), 0);
    }

    #[test]
    fn a_hyperedge_crosses_the_edge_that_runs_against_it() {
        // Node 0 has one port with two edges, to node 2 and to node 3; node 1 has one edge to node 2.
        // The edge from node 1 (lower) to node 2 (upper) crosses the one from node 0 to node 3.
        let mut graph = graph(4, vec![vec![0, 1], vec![2, 3]]);
        let a = add_port(&mut graph, 0, PortSide::EAST);
        let x = add_port(&mut graph, 1, PortSide::EAST);
        let w_a = add_port(&mut graph, 2, PortSide::WEST);
        let w_x = add_port(&mut graph, 2, PortSide::WEST);
        let w_y = add_port(&mut graph, 3, PortSide::WEST);
        connect(&mut graph, a, w_a);
        connect(&mut graph, a, w_y);
        connect(&mut graph, x, w_x);
        assert_eq!(crossings(&graph, vec![vec![0, 1], vec![2, 3]]), 1);
    }

    #[test]
    fn the_tree_ranks_the_entries_before_an_index() {
        let mut tree = BinaryIndexedTree::new(6);
        tree.add(1);
        tree.add(3);
        tree.add(3);
        assert_eq!(tree.rank(0), 0);
        assert_eq!(tree.rank(2), 1);
        assert_eq!(tree.rank(4), 3);
        assert_eq!(tree.size(), 3);
        tree.remove_all(3);
        assert_eq!(tree.rank(4), 1);
        assert_eq!(tree.size(), 1);
    }
}
