//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/BarycenterHeuristic.java
//!   with ForsterConstraintResolver.java (its `processConstraints` only, see below)
//!
//! The barycenter heuristic: a free layer is ordered by the barycenters of its nodes, the mean rank
//! of the ports they connect to in the fixed layer, plus a little random noise. A node without
//! connections gets a random place among the others. Random values are drawn in the order of the
//! Java, since the order of the nodes depends on them.
//!
//! `ForsterConstraintResolver.processConstraints` builds one group per node and returns the nodes in
//! their order when no constraint is violated. The layered model has no in-layer constraints, so the
//! constraint graph is always empty and the layer passes through unchanged. Its barycenter writes
//! (`stateOf(node).barycenter = group.getBarycenter()`) read back the node's own value, so they do
//! nothing either.
//!
//! The model-order barycenter (`ModelOrderBarycenterHeuristic`) is not ported; the sweep rejects it.
//! The barycenter associates (`BARYCENTER_ASSOCIATES`) are set only for north and south port dummies,
//! which are not ported, so `calculate_barycenter` has no associates to visit.

use std::cmp::Ordering;

use super::port_distributor::PortDistributor;
use crate::core::data::enums::NodeType;
use crate::java::random::Random;
use crate::java::sort;
use crate::layered::lgraph::PortType;
use crate::layered::p3order::{Layout, Net, start_index};

/// `BarycenterHeuristic.RANDOM_AMOUNT`, a float in Java: the noise added to each barycenter is
/// `nextFloat() * RANDOM_AMOUNT - RANDOM_AMOUNT / 2`.
const RANDOM_AMOUNT: f32 = 0.07;

/// `BarycenterHeuristic.BarycenterState`, kept per node.
#[derive(Clone, Default)]
struct BarycenterState {
    /// `summedWeight`, a double.
    summed_weight: f64,
    /// `degree`, the number of ports relevant to the barycenter.
    degree: i32,
    /// `barycenter`, `null` until known.
    barycenter: Option<f64>,
    /// `visited`, for the traversal of `calculateBarycenter`.
    visited: bool,
}

/// `BarycenterHeuristic` with the port distributor it asks for the port ranks.
pub(super) struct BarycenterHeuristic {
    /// `barycenterState`, indexed by node.
    states: Vec<BarycenterState>,
    /// `portDistributor`: it holds the port ranks and distributes the ports during the sweep.
    pub(super) ports: PortDistributor,
}

impl BarycenterHeuristic {
    pub(super) fn new(net: &Net, ports: PortDistributor) -> Self {
        BarycenterHeuristic {
            states: vec![BarycenterState::default(); net.graph().nodes.len()],
            ports,
        }
    }

    /// `setFirstLayerOrder`: the first layer of the sweep, in random order.
    pub(super) fn set_first_layer_order(
        &mut self,
        layout: &mut Layout,
        random: &mut Random,
        forward: bool,
    ) {
        let start = start_index(forward, layout.order.len());
        let mut nodes = layout.order[start].clone();
        self.order_randomly(&mut nodes, random);
        layout.order[start] = nodes;
    }

    /// `minimizeCrossings(order, freeLayerIndex, forwardSweep, isFirstSweep)`: reorders the free layer
    /// after the port ranks of the fixed layer are computed.
    pub(super) fn minimize_crossings(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        random: &mut Random,
        free: usize,
        forward: bool,
        is_first_sweep: bool,
    ) {
        if free != start_index(forward, layout.order.len()) {
            let fixed = if forward { free - 1 } else { free + 1 };
            self.ports.calculate_port_ranks(
                net,
                &layout.node_ports,
                &layout.order[fixed],
                port_type_for(forward),
            );
        }
        // `preOrdered = !isFirstSweep || isExternalPortDummy(firstNodeInLayer)`.
        let first_is_external = layout.order[free]
            .first()
            .is_some_and(|&node| net.node_type(node) == NodeType::EXTERNAL_PORT);
        let pre_ordered = !is_first_sweep || first_is_external;
        let mut nodes = layout.order[free].clone();
        self.order_by_barycenters(
            net,
            &layout.node_ports,
            random,
            &mut nodes,
            pre_ordered,
            forward,
        );
        layout.order[free] = nodes;
    }

    /// `minimizeCrossings(List<LNode>, preOrdered, randomize = true, forward)`: random barycenters.
    fn order_randomly(&mut self, nodes: &mut [usize], random: &mut Random) {
        // `randomizeBarycenters`: one `nextDouble` per node, in list order.
        for &node in nodes.iter() {
            let value = random.next_double();
            let state = &mut self.states[node];
            state.barycenter = Some(value);
            state.summed_weight = value;
            state.degree = 1;
        }
        if nodes.len() > 1 {
            self.sort(nodes);
        }
    }

    /// `minimizeCrossings(List<LNode>, preOrdered, randomize = false, forward)`: barycenters from the
    /// connected ports, the unknown ones filled in, then the nodes sorted by them.
    fn order_by_barycenters(
        &mut self,
        net: &Net,
        node_ports: &[Vec<usize>],
        random: &mut Random,
        nodes: &mut [usize],
        pre_ordered: bool,
        forward: bool,
    ) {
        self.calculate_barycenters(net, node_ports, random, nodes, forward);
        self.fill_unknown_barycenters(nodes, pre_ordered, random);
        if nodes.len() > 1 {
            self.sort(nodes);
        }
    }

    /// `calculateBarycenters`: clears the visited flags of the layer, then calculates each node.
    fn calculate_barycenters(
        &mut self,
        net: &Net,
        node_ports: &[Vec<usize>],
        random: &mut Random,
        nodes: &[usize],
        forward: bool,
    ) {
        for &node in nodes {
            self.states[node].visited = false;
        }
        for &node in nodes {
            self.calculate_barycenter(net, node_ports, random, node, forward);
        }
    }

    /// `calculateBarycenter`: the summed port ranks of the node's connected ports in the fixed layer,
    /// and for connections within the layer, the barycenter state of the other node. A node with
    /// connections gets its noise and its barycenter.
    fn calculate_barycenter(
        &mut self,
        net: &Net,
        node_ports: &[Vec<usize>],
        random: &mut Random,
        node: usize,
        forward: bool,
    ) {
        if self.states[node].visited {
            return;
        }
        self.states[node].visited = true;
        self.states[node].degree = 0;
        self.states[node].summed_weight = 0.0;
        self.states[node].barycenter = None;

        for &free_port in &node_ports[node] {
            // A forward sweep looks at the predecessors (the sources of the incoming edges), a
            // backward one at the successors.
            let edges = if forward {
                net.incoming(free_port)
            } else {
                net.outgoing(free_port)
            };
            for &edge in edges {
                let fixed_port = if forward {
                    net.source(edge)
                } else {
                    net.target(edge)
                };
                let fixed_node = net.node_of(fixed_port);
                if net.layer(fixed_node) == net.layer(node) {
                    // An in-layer edge uses the other node's barycenter; a self-loop is ignored.
                    if fixed_node != node {
                        self.calculate_barycenter(net, node_ports, random, fixed_node, forward);
                        let degree = self.states[fixed_node].degree;
                        let summed = self.states[fixed_node].summed_weight;
                        self.states[node].degree += degree;
                        self.states[node].summed_weight += summed;
                    }
                } else {
                    // `summedWeight += portRanks[fixedPort.id]`: a float, widened to a double.
                    let rank = self.ports.port_ranks[fixed_port];
                    self.states[node].summed_weight += f64::from(rank);
                    self.states[node].degree += 1;
                }
            }
        }

        if self.states[node].degree > 0 {
            // `summedWeight += random.nextFloat() * RANDOM_AMOUNT - RANDOM_AMOUNT / 2`: the noise is
            // computed in float arithmetic, then added to the double sum.
            let noise = random.next_float() as f32 * RANDOM_AMOUNT - RANDOM_AMOUNT / 2.0;
            let state = &mut self.states[node];
            state.summed_weight += f64::from(noise);
            state.barycenter = Some(state.summed_weight / f64::from(state.degree));
        }
    }

    /// `fillInUnknownBarycenters`: a node without a barycenter takes the middle of its neighbours in a
    /// pre-ordered layer, and a random value above the known ones otherwise.
    fn fill_unknown_barycenters(
        &mut self,
        nodes: &[usize],
        pre_ordered: bool,
        random: &mut Random,
    ) {
        if pre_ordered {
            let mut last = -1.0_f64;
            for (index, &node) in nodes.iter().enumerate() {
                let value = match self.states[node].barycenter {
                    Some(value) => value,
                    None => {
                        // The next known value after this node, or one past the previous value.
                        let next = nodes[index + 1..]
                            .iter()
                            .find_map(|&later| self.states[later].barycenter)
                            .unwrap_or(last + 1.0);
                        let value = (last + next) / 2.0;
                        set_barycenter(&mut self.states[node], value);
                        value
                    }
                };
                last = value;
            }
        } else {
            let mut max_barycenter = 0.0_f64;
            for &node in nodes {
                if let Some(value) = self.states[node].barycenter {
                    max_barycenter = max_barycenter.max(value);
                }
            }
            max_barycenter += 2.0;
            for &node in nodes {
                if self.states[node].barycenter.is_none() {
                    // `random.nextFloat() * maxBary - 1`: the float widens to a double first.
                    let value = random.next_float() * max_barycenter - 1.0;
                    set_barycenter(&mut self.states[node], value);
                }
            }
        }
    }

    /// `Collections.sort(layer, barycenterStateComparator)`.
    fn sort(&self, nodes: &mut [usize]) {
        let states = &self.states;
        sort::sort_by(nodes, |&first, &second| {
            compare_barycenters(states[first].barycenter, states[second].barycenter)
        });
    }
}

/// `setBarycenter` of a node with an unknown barycenter in `fillInUnknownBarycenters`: the value
/// is also its summed weight, with degree one.
fn set_barycenter(state: &mut BarycenterState, value: f64) {
    state.barycenter = Some(value);
    state.summed_weight = value;
    state.degree = 1;
}

/// `PortType` of the ports a sweep direction ranks: the outputs of the fixed layer for a forward sweep.
fn port_type_for(forward: bool) -> PortType {
    if forward {
        PortType::OUTPUT
    } else {
        PortType::INPUT
    }
}

/// `barycenterStateComparator`: the nodes with a barycenter come first, ordered by it.
fn compare_barycenters(first: Option<f64>, second: Option<f64>) -> Ordering {
    match (first, second) {
        // `Double.compareTo`, which `total_cmp` matches for the values here.
        (Some(first), Some(second)) => first.total_cmp(&second),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
