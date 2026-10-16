//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/AbstractBarycenterPortDistributor.java
//!   with NodeRelativePortDistributor.java, LayerTotalPortDistributor.java and ISweepPortDistributor.java
//!
//! The port ranks of the barycenter heuristic and the distribution of the ports of each node while
//! the sweep runs. A port's rank is its place in the layer on the other side of its edges; the ports
//! of a node are sorted by the barycenters of the ranks they connect to.
//!
//! Java's `float` is kept as `f32` for the ranks, the barycenters of ports and the sums that form
//! them, since equal barycenters are ordered by the sort and a rounding in the sums would change
//! the order. Doubles stay doubles. A northern or southern port takes its barycenter from the place
//! of its north/south port dummy in the layer (`dealWithNorthSouthPorts`).

use std::cmp::Ordering;

use crate::core::data::enums::PortSide;
use crate::java::sort;
use crate::layered::graph_util::opposed;
use crate::layered::lgraph::PortType;
use crate::layered::p3order::{Layout, Net, start_index};
use crate::layered::ports::is_order_fixed;

/// `ISweepPortDistributor.create`: the node-relative rule, or the layer-total rule, chosen at random.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RankRule {
    NodeRelative,
    LayerTotal,
}

/// `AbstractBarycenterPortDistributor` with its rank rule.
pub(super) struct PortDistributor {
    rule: RankRule,
    /// `portRanks`, indexed by port.
    pub(super) port_ranks: Vec<f32>,
    /// `portBarycenter`, indexed by port. It keeps its value between calls, as the Java array does.
    port_barycenters: Vec<f32>,
    /// `nodePositions`, the index of each node in its layer.
    positions: Vec<usize>,
    /// `inLayerPorts`: the ports with an edge to their own layer, of the side being distributed.
    in_layer_ports: Vec<usize>,
    /// `minBarycenter` and `maxBarycenter`, for the node whose ports are being collected.
    min_barycenter: f32,
    max_barycenter: f32,
}

impl PortDistributor {
    /// The distributor for the graph, `new NodeRelativePortDistributor` or `new LayerTotalPortDistributor`.
    pub(super) fn new(net: &Net, node_relative: bool) -> Self {
        let graph = net.graph();
        let mut positions = vec![0; graph.nodes.len()];
        for layer in &graph.layers {
            for (index, &node) in layer.iter().enumerate() {
                positions[node] = index;
            }
        }
        PortDistributor {
            rule: if node_relative {
                RankRule::NodeRelative
            } else {
                RankRule::LayerTotal
            },
            port_ranks: vec![0.0; graph.ports.len()],
            port_barycenters: vec![0.0; graph.ports.len()],
            positions,
            in_layer_ports: Vec::new(),
            min_barycenter: 0.0,
            max_barycenter: 0.0,
        }
    }

    /// `calculatePortRanks(LNode[] layer, PortType)`: ranks the ports of one type in a whole layer.
    pub(super) fn calculate_port_ranks(
        &mut self,
        net: &Net,
        node_ports: &[Vec<usize>],
        layer: &[usize],
        port_type: PortType,
    ) {
        // `consumedRank` is a float in Java.
        let mut consumed_rank = 0.0_f32;
        for &node in layer {
            consumed_rank += self.node_ranks(net, &node_ports[node], consumed_rank, port_type);
        }
    }

    /// `calculatePortRanks(LNode, float, PortType)` of the chosen rule; returns the rank it consumes.
    fn node_ranks(
        &mut self,
        net: &Net,
        ports: &[usize],
        rank_sum: f32,
        port_type: PortType,
    ) -> f32 {
        match self.rule {
            RankRule::NodeRelative => {
                node_relative_ranks(&mut self.port_ranks, net, ports, rank_sum, port_type)
            }
            RankRule::LayerTotal => {
                layer_total_ranks(&mut self.port_ranks, net, ports, rank_sum, port_type)
            }
        }
    }

    /// `distributePortsWhileSweeping`: ranks the fixed layer's ports, distributes the free layer's
    /// ports, ranks the free layer's ports and distributes the fixed layer's ports on the opposite
    /// side. The Java returns `false` here and nothing reads it, so it is not kept.
    pub(super) fn distribute_ports_while_sweeping(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        current: usize,
        forward: bool,
    ) {
        self.update_node_positions(&layout.order[current]);
        let length = layout.order.len();
        let side = if forward {
            PortSide::WEST
        } else {
            PortSide::EAST
        };
        let free_layer = layout.order[current].clone();
        if current != start_index(forward, length) {
            let fixed_index = if forward { current - 1 } else { current + 1 };
            let fixed_layer = layout.order[fixed_index].clone();
            self.calculate_port_ranks(
                net,
                &layout.node_ports,
                &fixed_layer,
                port_type_for(forward),
            );
            for &node in &free_layer {
                self.distribute_node_ports(net, &mut layout.node_ports, node, side);
            }
            self.calculate_port_ranks(
                net,
                &layout.node_ports,
                &free_layer,
                port_type_for(!forward),
            );
            // A node with a nested graph keeps the order its nested graph gave its ports there.
            for &node in &fixed_layer {
                if net.graph().nodes[node].nested_graph.is_none() {
                    self.distribute_node_ports(net, &mut layout.node_ports, node, opposed(side));
                }
            }
        } else {
            for &node in &free_layer {
                self.distribute_node_ports(net, &mut layout.node_ports, node, side);
            }
        }
    }

    /// `distributePorts(LNode, PortSide)`: the ports of the side, then the south and north ports, are
    /// collected and their barycenters computed, and the node's ports are sorted. Nodes with fixed
    /// port order keep their ports.
    fn distribute_node_ports(
        &mut self,
        net: &Net,
        node_ports: &mut [Vec<usize>],
        node: usize,
        side: PortSide,
    ) {
        if is_order_fixed(net.constraints(node)) {
            return;
        }
        let on_side = net.side_ports(&node_ports[node], side);
        self.distribute_on_side(net, node_ports, node, &on_side);
        let south = net.side_ports(&node_ports[node], PortSide::SOUTH);
        self.distribute_on_side(net, node_ports, node, &south);
        let north = net.side_ports(&node_ports[node], PortSide::NORTH);
        self.distribute_on_side(net, node_ports, node, &north);
        self.sort_ports(net, &mut node_ports[node]);
    }

    /// `distributePorts(LNode, Iterable<LPort>)`.
    fn distribute_on_side(
        &mut self,
        net: &Net,
        node_ports: &[Vec<usize>],
        node: usize,
        ports: &[usize],
    ) {
        self.in_layer_ports.clear();
        self.collect_ports(net, node_ports, node, ports);
        if !self.in_layer_ports.is_empty() {
            self.in_layer_barycenters(net, node);
        }
    }

    /// `iteratePortsAndCollectInLayerPorts`: the barycenter of each port from the ranks of the ports
    /// it connects to; ports with an edge into their own layer are collected instead.
    fn collect_ports(
        &mut self,
        net: &Net,
        node_ports: &[Vec<usize>],
        node: usize,
        ports: &[usize],
    ) {
        self.min_barycenter = 0.0;
        self.max_barycenter = 0.0;
        let node_layer = net.layer(node);
        // `2 * node.getLayer().getNodes().size() + 1`, an int widened to a float.
        let absurdly_large_float = (2 * net.layer_size(node) + 1) as f32;
        'ports: for &port in ports {
            let side = net.side(port);
            let north_south_port = matches!(side, PortSide::NORTH | PortSide::SOUTH);
            // `float sum = 0`.
            let mut sum = 0.0_f32;
            if north_south_port {
                let Some(port_dummy) = net.graph().ports[port].north_south_dummy else {
                    continue;
                };
                sum += self.deal_with_north_south_ports(
                    net,
                    node_ports,
                    absurdly_large_float,
                    port,
                    port_dummy,
                );
            } else {
                // Outgoing edges go to the next layer, which is seen clockwise.
                for &edge in net.outgoing(port) {
                    let connected = net.target(edge);
                    if net.layer_of_port(connected) == node_layer {
                        self.in_layer_ports.push(port);
                        continue 'ports;
                    }
                    sum += self.port_ranks[connected];
                }
                // Incoming edges go to the previous layer, which is seen counter-clockwise.
                for &edge in net.incoming(port) {
                    let connected = net.source(edge);
                    if net.layer_of_port(connected) == node_layer {
                        self.in_layer_ports.push(port);
                        continue 'ports;
                    }
                    sum -= self.port_ranks[connected];
                }
            }
            let degree = net.degree(port);
            if degree > 0 {
                // `sum / port.getDegree()`, float by int.
                let barycenter = sum / degree as f32;
                self.port_barycenters[port] = barycenter;
                // `Math.min` and `Math.max` on floats; no NaN occurs, and the sign of zero does not
                // affect the comparisons made with these values.
                self.min_barycenter = self.min_barycenter.min(barycenter);
                self.max_barycenter = self.max_barycenter.max(barycenter);
            } else if north_south_port {
                self.port_barycenters[port] = sum;
            }
        }
    }

    /// `dealWithNorthSouthPorts`: the barycenter of a northern or southern port from the place of its
    /// dummy. An input port's edges come from the left, so it goes to the far end of its side; an
    /// output port's go to the right, and the port takes the dummy's place; a port with both goes to
    /// the middle.
    fn deal_with_north_south_ports(
        &self,
        net: &Net,
        node_ports: &[Vec<usize>],
        absurdly_large_float: f32,
        port: usize,
        port_dummy: usize,
    ) -> f32 {
        let mut input = false;
        let mut output = false;
        for &dummy_port in &node_ports[port_dummy] {
            if net.graph().ports[dummy_port].origin_port == Some(port) {
                if !net.outgoing(dummy_port).is_empty() {
                    output = true;
                } else if !net.incoming(dummy_port).is_empty() {
                    input = true;
                }
            }
        }
        // `positionOf(portDummy)`, an int.
        let position = self.positions[port_dummy] as f32;
        let north = net.side(port) == PortSide::NORTH;
        if input && input ^ output {
            if north {
                -position
            } else {
                absurdly_large_float - position
            }
        } else if output && input ^ output {
            position + 1.0
        } else if input && output {
            if north {
                0.0
            } else {
                absurdly_large_float / 2.0
            }
        } else {
            0.0
        }
    }

    /// `calculateInLayerPortsBarycenterValues`: a port with in-layer edges goes to the top or the
    /// bottom of its side, depending on where its connected nodes are.
    fn in_layer_barycenters(&mut self, net: &Net, node: usize) {
        let node_layer = net.layer(node);
        // `positionOf(node) + 1` and `layer size + 1`, ints in Java, converted to float for the comparison.
        let node_index = (self.positions[node] + 1) as f32;
        let layer_size = (net.layer_size(node) + 1) as f32;
        for port in self.in_layer_ports.clone() {
            let mut sum = 0_usize;
            let mut connections = 0_usize;
            for connected in net.connected_ports(port) {
                if net.layer_of_port(connected) == node_layer {
                    sum += self.positions[net.node_of(connected)] + 1;
                    connections += 1;
                }
            }
            // `(float) sum / inLayerConnections`.
            let barycenter = sum as f32 / connections as f32;
            match net.side(port) {
                PortSide::EAST => {
                    self.port_barycenters[port] = if barycenter < node_index {
                        // A low value puts the port above.
                        self.min_barycenter - barycenter
                    } else {
                        // A high value puts the port below.
                        self.max_barycenter + (layer_size - barycenter)
                    };
                }
                PortSide::WEST => {
                    self.port_barycenters[port] = if barycenter < node_index {
                        self.max_barycenter + barycenter
                    } else {
                        self.min_barycenter - (layer_size - barycenter)
                    };
                }
                _ => {}
            }
        }
    }

    /// `updateNodePositions`: the index of each node of the layer.
    fn update_node_positions(&mut self, layer: &[usize]) {
        for (index, &node) in layer.iter().enumerate() {
            self.positions[node] = index;
        }
    }

    /// `sortPorts`: by side, then by barycenter; ports with barycenter zero come first, as the Java
    /// comparator puts them.
    fn sort_ports(&self, net: &Net, ports: &mut [usize]) {
        let barycenters = &self.port_barycenters;
        sort::sort_by(ports, |&first, &second| {
            let (side1, side2) = (net.side(first), net.side(second));
            if side1 != side2 {
                return (side1 as usize).cmp(&(side2 as usize));
            }
            let (bary1, bary2) = (barycenters[first], barycenters[second]);
            if bary1 == 0.0 && bary2 == 0.0 {
                Ordering::Equal
            } else if bary1 == 0.0 {
                Ordering::Less
            } else if bary2 == 0.0 {
                Ordering::Greater
            } else {
                // `Float.compare`.
                bary1.total_cmp(&bary2)
            }
        });
    }
}

/// `PortType` of the ports a sweep direction ranks: the outputs of the fixed layer for a forward sweep.
fn port_type_for(forward: bool) -> PortType {
    if forward {
        PortType::OUTPUT
    } else {
        PortType::INPUT
    }
}

/// `NodeRelativePortDistributor.calculatePortRanks`: each node gets the range `[rankSum, rankSum + 1]`.
/// Returns the rank the node consumes, which is always 1.
fn node_relative_ranks(
    port_ranks: &mut [f32],
    net: &Net,
    ports: &[usize],
    rank_sum: f32,
    port_type: PortType,
) -> f32 {
    match port_type {
        PortType::INPUT => {
            let inputs: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&port| !net.incoming(port).is_empty())
                .collect();
            let north_input_count = inputs
                .iter()
                .filter(|&&port| net.side(port) == PortSide::NORTH)
                .count();
            // `1.0f / (inputCount + 1)`.
            let incr = 1.0_f32 / (inputs.len() + 1) as f32;
            // Ranks go north - west - south - east.
            let mut north_pos = rank_sum + north_input_count as f32 * incr;
            let mut rest_pos = rank_sum + 1.0 - incr;
            for port in inputs {
                if net.side(port) == PortSide::NORTH {
                    port_ranks[port] = north_pos;
                    north_pos -= incr;
                } else {
                    port_ranks[port] = rest_pos;
                    rest_pos -= incr;
                }
            }
        }
        PortType::OUTPUT => {
            let outputs: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&port| !net.outgoing(port).is_empty())
                .collect();
            let incr = 1.0_f32 / (outputs.len() + 1) as f32;
            let mut pos = rank_sum + incr;
            for port in outputs {
                port_ranks[port] = pos;
                pos += incr;
            }
        }
    }
    1.0
}

/// `LayerTotalPortDistributor.calculatePortRanks`: each node gets a range as wide as its ports.
/// Returns the rank the node consumes.
fn layer_total_ranks(
    port_ranks: &mut [f32],
    net: &Net,
    ports: &[usize],
    rank_sum: f32,
    port_type: PortType,
) -> f32 {
    match port_type {
        PortType::INPUT => {
            let inputs: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&port| !net.incoming(port).is_empty())
                .collect();
            let north_input_count = inputs
                .iter()
                .filter(|&&port| net.side(port) == PortSide::NORTH)
                .count();
            let input_count = inputs.len();
            let mut north_pos = rank_sum + north_input_count as f32;
            let mut rest_pos = rank_sum + input_count as f32;
            for port in inputs {
                if net.side(port) == PortSide::NORTH {
                    port_ranks[port] = north_pos;
                    north_pos -= 1.0;
                } else {
                    port_ranks[port] = rest_pos;
                    rest_pos -= 1.0;
                }
            }
            input_count as f32
        }
        PortType::OUTPUT => {
            let mut pos = 0_usize;
            for port in ports.iter().copied() {
                if !net.outgoing(port).is_empty() {
                    pos += 1;
                    port_ranks[port] = rank_sum + pos as f32;
                }
            }
            pos as f32
        }
    }
}
