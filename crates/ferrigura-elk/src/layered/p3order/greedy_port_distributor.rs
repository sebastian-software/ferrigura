//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/GreedyPortDistributor.java
//!
//! The port distributor of the two-sided greedy switch: on each node of the free layer whose port
//! order is not fixed, neighbouring ports on the side facing the fixed layer change places while
//! that lowers the crossings of their edges. Nodes with a nested graph do not occur in the swept
//! root graph, so the hierarchical crossing counter of the Java is never used.

use crate::core::data::enums::PortSide;
use crate::layered::LayoutError;
use crate::layered::p3order::counting::{CrossingsCounter, switch_ports};
use crate::layered::p3order::{Layout, Net};
use crate::layered::ports::is_order_fixed;

/// `GreedyPortDistributor`.
pub(super) struct GreedyPortDistributor {
    /// `crossingsCounter`.
    counter: CrossingsCounter,
    /// `portPos`, the counter's port positions. They keep their values between the calls.
    port_positions: Vec<i32>,
}

impl GreedyPortDistributor {
    /// The distributor after `initAfterTraversal`.
    pub(super) fn new(net: &Net) -> Self {
        GreedyPortDistributor {
            counter: CrossingsCounter::new(net),
            port_positions: vec![0; net.graph().ports.len()],
        }
    }

    /// `distributePortsWhileSweeping`: whether a port changed its place.
    pub(super) fn distribute_ports_while_sweeping(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        current: usize,
        forward: bool,
    ) -> Result<bool, LayoutError> {
        self.initialize(net, layout, current, forward);
        self.distribute_ports_in_layer(net, layout, current, forward)
    }

    /// `initialize`: the counter counts between the free layer and the one before it in the sweep,
    /// or, at the start of the sweep, in the free layer on the side the sweep comes from.
    fn initialize(&mut self, net: &Net, layout: &Layout, current: usize, forward: bool) {
        let order = &layout.order;
        if forward && current > 0 {
            self.counter.init_for_counting_between(
                net,
                layout,
                &order[current - 1],
                &order[current],
                &mut self.port_positions,
            );
        } else if !forward && current + 1 < order.len() {
            self.counter.init_for_counting_between(
                net,
                layout,
                &order[current],
                &order[current + 1],
                &mut self.port_positions,
            );
        } else {
            let side = if forward {
                PortSide::WEST
            } else {
                PortSide::EAST
            };
            self.counter.init_port_positions_for_in_layer_crossings(
                net,
                layout,
                &order[current],
                side,
                &mut self.port_positions,
            );
        }
    }

    /// `distributePortsInLayer`: the nodes whose port order is free, on the side facing the fixed
    /// layer.
    fn distribute_ports_in_layer(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        current: usize,
        forward: bool,
    ) -> Result<bool, LayoutError> {
        let side = if forward {
            PortSide::WEST
        } else {
            PortSide::EAST
        };
        let mut improved = false;
        for node in layout.order[current].clone() {
            if is_order_fixed(net.constraints(node)) {
                continue;
            }
            improved |= self.distribute_ports_on_node(net, layout, node, side)?;
        }
        Ok(improved)
    }

    /// `distributePortsOnNode`: passes over the ports of `side`, north to south and east to west,
    /// switching neighbours until a pass switches nothing. The Java switches in a view of the port
    /// list; here the view is the list of the ports' indices in the node's port list.
    fn distribute_ports_on_node(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        node: usize,
        side: PortSide,
    ) -> Result<bool, LayoutError> {
        let node_ports = &mut layout.node_ports[node];
        let mut view: Vec<usize> = (0..node_ports.len())
            .filter(|&index| net.side(node_ports[index]) == side)
            .collect();
        if matches!(side, PortSide::SOUTH | PortSide::WEST) {
            view.reverse();
        }
        let mut improved = false;
        loop {
            let mut continue_switching = false;
            for i in 0..view.len().saturating_sub(1) {
                let upper = node_ports[view[i]];
                let lower = node_ports[view[i + 1]];
                if self.switching_decreases_crossings(net, upper, lower)? {
                    improved = true;
                    // `switchPorts`: the counter first, then the list.
                    switch_ports(upper, lower, &mut self.port_positions);
                    node_ports.swap(view[i], view[i + 1]);
                    continue_switching = true;
                }
            }
            if !continue_switching {
                return Ok(improved);
            }
        }
    }

    /// `switchingDecreasesCrossings`, without the hierarchical counter.
    fn switching_decreases_crossings(
        &mut self,
        net: &Net,
        upper: usize,
        lower: usize,
    ) -> Result<bool, LayoutError> {
        let (upper_lower, lower_upper) =
            self.counter.count_crossings_between_ports_in_both_orders(
                net,
                upper,
                lower,
                &mut self.port_positions,
            )?;
        Ok(upper_lower > lower_upper)
    }
}
