//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/loops/SelfLoopHolder.java,
//!   SelfLoopPort.java, SelfLoopEdge.java, SelfHyperLoop.java and SelfLoopType.java
//!
//! The self loop model: the holder of a node's self loops, their ports and edges, and the
//! hyper loops that group them. Only simple self loops are ported: a hyper loop that holds two or
//! more edges, which Java routes as one loop, is reported as unsupported.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::core::data::enums::{Direction, NodeType, PortConstraints, PortSide};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::loops::labels::SelfHyperLoopLabels;
use crate::layered::options;
use crate::layered::p1cycles::outgoing_edges;
use crate::layered::ports;

/// `PortSide.values().length`: the number of entries of the per-side arrays.
pub const SIDE_COUNT: usize = 5;

/// `SelfLoopHolder`: the self loops of one node, installed by the preprocessor on the node.
#[derive(Debug, Clone, Default)]
pub struct SelfLoopHolder {
    /// The index of the node in `LGraph::nodes`.
    pub node: usize,
    /// `getSLHyperLoops()`: the hyper loops, in the order they were found.
    pub hyper_loops: Vec<SelfHyperLoop>,
    /// `getSLPortMap()`: the self loop ports, in the order the self loops reach them.
    pub sl_ports: Vec<SelfLoopPort>,
    /// The self loop edges, in the order of the node's outgoing edges.
    pub sl_edges: Vec<SelfLoopEdge>,
    /// `arePortsHidden()`: whether a self loop port is hidden from the node.
    pub ports_hidden: bool,
    /// `getRoutingSlotCount()`: the routing slots used on each side, by `PortSide` ordinal.
    pub routing_slot_count: [usize; SIDE_COUNT],
}

/// `SelfLoopPort`: a port of the node, as the self loops see it.
#[derive(Debug, Clone, Default)]
pub struct SelfLoopPort {
    /// The index of the port in `LGraph::ports`.
    pub lport: usize,
    /// `hadOnlySelfLoops()`: whether every edge at the port is a self loop.
    pub had_only_self_loops: bool,
    /// The indices of the self loop edges that enter the port, in `SelfLoopHolder::sl_edges`.
    pub incoming: Vec<usize>,
    /// The indices of the self loop edges that leave the port.
    pub outgoing: Vec<usize>,
    /// `isHidden()`: whether the port is currently removed from the node.
    pub hidden: bool,
}

/// `SelfLoopEdge`: an edge that is a self loop.
#[derive(Debug, Clone, Copy)]
pub struct SelfLoopEdge {
    /// The index of the edge in `LGraph::edges`.
    pub ledge: usize,
    /// The index of the source port in `SelfLoopHolder::sl_ports`.
    pub source: usize,
    /// The index of the target port in `SelfLoopHolder::sl_ports`.
    pub target: usize,
    /// The index of the hyper loop in `SelfLoopHolder::hyper_loops`, set once the hyper loops exist.
    pub hyper_loop: Option<usize>,
}

/// `SelfHyperLoop`: the self loops that share ports, routed as one. Only loops of one edge are
/// supported; see the module documentation.
#[derive(Debug, Clone, Default)]
pub struct SelfHyperLoop {
    /// `getSLPorts()`: the indices of its ports in `SelfLoopHolder::sl_ports`.
    pub sl_ports: Vec<usize>,
    /// `getSLEdges()`: the indices of its edges in `SelfLoopHolder::sl_edges`.
    pub sl_edges: Vec<usize>,
    /// `getSLLabels()`: the labels of its edges, when they have any.
    pub labels: Option<SelfHyperLoopLabels>,
    /// `getSelfLoopType()`, set by `computePortsPerSide`.
    pub loop_type: Option<SelfLoopType>,
    /// `getSLPortsBySide()`: its ports on each side, in port order, keyed by side.
    pub ports_by_side: BTreeMap<PortSide, Vec<usize>>,
    /// `getLeftmostPort()`: the index of the port in `SelfLoopHolder::sl_ports`.
    pub leftmost: Option<usize>,
    /// `getRightmostPort()`: the index of the port in `SelfLoopHolder::sl_ports`.
    pub rightmost: Option<usize>,
    /// `getOccupiedPortSides()`.
    pub occupied_sides: BTreeSet<PortSide>,
    /// `getRoutingSlot(side)`, by `PortSide` ordinal.
    pub routing_slot: [usize; SIDE_COUNT],
}

/// `SelfLoopType`: the sides a self loop's ports are on. The variants keep the Java names.
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfLoopType {
    ONE_SIDE,
    TWO_SIDES_CORNER,
    TWO_SIDES_OPPOSING,
    THREE_SIDES,
    FOUR_SIDES,
}

impl SelfLoopType {
    /// `SelfLoopType.fromPortSides`: the type for the given sides, `None` for a number of sides
    /// that is not one to four.
    pub fn from_port_sides(
        sides: &BTreeSet<PortSide>,
    ) -> Result<Option<SelfLoopType>, LayoutError> {
        if sides.contains(&PortSide::UNDEFINED) {
            return Err(LayoutError::Unsupported(
                "a self loop port without a side".to_string(),
            ));
        }
        Ok(match sides.len() {
            1 => Some(SelfLoopType::ONE_SIDE),
            2 => {
                let east_west = sides.contains(&PortSide::EAST) && sides.contains(&PortSide::WEST);
                let north_south =
                    sides.contains(&PortSide::NORTH) && sides.contains(&PortSide::SOUTH);
                if east_west || north_south {
                    Some(SelfLoopType::TWO_SIDES_OPPOSING)
                } else {
                    Some(SelfLoopType::TWO_SIDES_CORNER)
                }
            }
            3 => Some(SelfLoopType::THREE_SIDES),
            4 => Some(SelfLoopType::FOUR_SIDES),
            _ => None,
        })
    }
}

/// `PortSide.right()`: the next side clockwise.
pub fn right(side: PortSide) -> PortSide {
    match side {
        PortSide::NORTH => PortSide::EAST,
        PortSide::EAST => PortSide::SOUTH,
        PortSide::SOUTH => PortSide::WEST,
        PortSide::WEST => PortSide::NORTH,
        PortSide::UNDEFINED => PortSide::UNDEFINED,
    }
}

/// `PortSide.left()`: the next side counter-clockwise.
pub fn left(side: PortSide) -> PortSide {
    match side {
        PortSide::NORTH => PortSide::WEST,
        PortSide::WEST => PortSide::SOUTH,
        PortSide::SOUTH => PortSide::EAST,
        PortSide::EAST => PortSide::NORTH,
        PortSide::UNDEFINED => PortSide::UNDEFINED,
    }
}

/// `LEdge.isSelfLoop()`: an attached edge whose ends are on the same node. A detached edge is
/// never a self loop, as in Java where its source and target are null.
pub fn is_self_loop(graph: &LGraph, edge: usize) -> bool {
    graph.is_attached(edge) && {
        let data = &graph.edges[edge];
        graph.ports[data.source].node == graph.ports[data.target].node
    }
}

/// `LPort.getNetFlow()`: the incoming edges minus the outgoing ones.
pub fn port_net_flow(graph: &LGraph, lport: usize) -> i64 {
    graph.ports[lport].incoming.len() as i64 - graph.ports[lport].outgoing.len() as i64
}

/// `LPort.setSide(side)`: the side, and the anchor the side implies. Ports made by the importer
/// have no explicitly supplied anchor, so the anchor always follows the side.
pub fn set_port_side(graph: &mut LGraph, lport: usize, side: PortSide) {
    let port = &mut graph.ports[lport];
    port.side = side;
    match side {
        PortSide::NORTH => {
            port.anchor.x = port.size.x / 2.0;
            port.anchor.y = 0.0;
        }
        PortSide::EAST => {
            port.anchor.x = port.size.x;
            port.anchor.y = port.size.y / 2.0;
        }
        PortSide::SOUTH => {
            port.anchor.x = port.size.x / 2.0;
            port.anchor.y = port.size.y;
        }
        PortSide::WEST => {
            port.anchor.x = 0.0;
            port.anchor.y = port.size.y / 2.0;
        }
        PortSide::UNDEFINED => {}
    }
}

/// `SelfLoopHolder.needsSelfLoopProcessing`: a regular node with at least one self loop.
pub fn needs_self_loop_processing(graph: &LGraph, node: usize) -> bool {
    graph.nodes[node].node_type == NodeType::NORMAL
        && outgoing_edges(graph, node)
            .into_iter()
            .any(|edge| is_self_loop(graph, edge))
}

impl SelfLoopHolder {
    /// `SelfLoopHolder.install` and `initialize`: the self loop edges and ports of the node, grouped
    /// into hyper loops. Each hyper loop must hold one edge.
    pub fn initialize(graph: &LGraph, node: usize) -> Result<SelfLoopHolder, LayoutError> {
        let mut holder = SelfLoopHolder {
            node,
            ..SelfLoopHolder::default()
        };
        // Create self loop edges and ports for every self loop.
        for edge in outgoing_edges(graph, node) {
            if !is_self_loop(graph, edge) {
                continue;
            }
            let data = &graph.edges[edge];
            let source = holder.port_for(graph, data.source);
            let target = holder.port_for(graph, data.target);
            holder.sl_edges.push(SelfLoopEdge {
                ledge: edge,
                source,
                target,
                hyper_loop: None,
            });
            let index = holder.sl_edges.len() - 1;
            holder.sl_ports[source].outgoing.push(index);
            holder.sl_ports[target].incoming.push(index);
        }

        // Run a breadth-first search from every port that no earlier search reached.
        let mut visited = vec![false; holder.sl_ports.len()];
        for start in 0..holder.sl_ports.len() {
            if !visited[start] {
                let hyper_loop = holder.initialize_hyper_loop(graph, start, &mut visited);
                holder.hyper_loops.push(hyper_loop);
            }
        }

        if holder
            .hyper_loops
            .iter()
            .any(|hyper| hyper.sl_edges.len() > 1)
        {
            return Err(LayoutError::Unsupported(
                "self loops that share a port (hyper loops)".to_string(),
            ));
        }
        Ok(holder)
    }

    /// `SelfLoopHolder.selfLoopPortFor`: the index of the self loop port of `lport`, made if needed.
    fn port_for(&mut self, graph: &LGraph, lport: usize) -> usize {
        if let Some(index) = self.sl_ports.iter().position(|port| port.lport == lport) {
            return index;
        }
        let had_only_self_loops = graph.ports[lport]
            .incoming
            .iter()
            .chain(graph.ports[lport].outgoing.iter())
            .all(|&edge| is_self_loop(graph, edge));
        self.sl_ports.push(SelfLoopPort {
            lport,
            had_only_self_loops,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            hidden: false,
        });
        self.sl_ports.len() - 1
    }

    /// `SelfLoopHolder.initializeHyperLoop`: collects the self loops reachable from `start`. A port is
    /// marked visited when it leaves the queue, as in Java, so it may be queued twice.
    fn initialize_hyper_loop(
        &mut self,
        graph: &LGraph,
        start: usize,
        visited: &mut [bool],
    ) -> SelfHyperLoop {
        let index = self.hyper_loops.len();
        let mut hyper = SelfHyperLoop::default();
        let mut queue = VecDeque::from([start]);
        while let Some(current) = queue.pop_front() {
            visited[current] = true;
            for edge in self.sl_ports[current].outgoing.clone() {
                self.add_self_loop_edge(graph, &mut hyper, index, edge);
                let target = self.sl_edges[edge].target;
                if !visited[target] {
                    queue.push_back(target);
                }
            }
            for edge in self.sl_ports[current].incoming.clone() {
                self.add_self_loop_edge(graph, &mut hyper, index, edge);
                let source = self.sl_edges[edge].source;
                if !visited[source] {
                    queue.push_back(source);
                }
            }
        }
        hyper
    }

    /// `SelfHyperLoop.addSelfLoopEdge`: adds the edge, its ports, and its labels.
    fn add_self_loop_edge(
        &mut self,
        graph: &LGraph,
        hyper: &mut SelfHyperLoop,
        hyper_index: usize,
        edge: usize,
    ) {
        if hyper.sl_edges.contains(&edge) {
            return;
        }
        self.sl_edges[edge].hyper_loop = Some(hyper_index);
        hyper.sl_edges.push(edge);
        for port in [self.sl_edges[edge].source, self.sl_edges[edge].target] {
            if !hyper.sl_ports.contains(&port) {
                hyper.sl_ports.push(port);
            }
        }
        let labels = &graph.edges[self.sl_edges[edge].ledge].labels;
        if !labels.is_empty() {
            let slot = hyper
                .labels
                .get_or_insert_with(|| SelfHyperLoopLabels::new(graph));
            slot.add_labels(graph, labels);
        }
    }

    /// `SelfHyperLoop.computePortsPerSide`: the ports on each side, and the loop type they give.
    pub fn compute_ports_per_side(
        &mut self,
        graph: &LGraph,
        hyper_index: usize,
    ) -> Result<(), LayoutError> {
        let mut by_side: BTreeMap<PortSide, Vec<usize>> = BTreeMap::new();
        for &port in &self.hyper_loops[hyper_index].sl_ports {
            let side = graph.ports[self.sl_ports[port].lport].side;
            if side == PortSide::UNDEFINED {
                return Err(LayoutError::Unsupported(
                    "a self loop port without a side".to_string(),
                ));
            }
            by_side.entry(side).or_default().push(port);
        }
        let sides: BTreeSet<PortSide> = by_side.keys().copied().collect();
        let hyper = &mut self.hyper_loops[hyper_index];
        hyper.loop_type = SelfLoopType::from_port_sides(&sides)?;
        hyper.ports_by_side = by_side;
        Ok(())
    }

    /// The self loop ports of a hyper loop that are hidden, in the loop's port order.
    pub fn hidden_ports(&self, hyper_index: usize) -> Vec<usize> {
        self.hyper_loops[hyper_index]
            .sl_ports
            .iter()
            .copied()
            .filter(|&port| self.sl_ports[port].hidden)
            .collect()
    }
}

/// `SelfLoopPreProcessor.hideSelfLoops` and `hidePorts`: detaches the self loop edges, and hides the
/// ports that have only self loops from the node, unless the node's port order is fixed or the
/// graph nested in the node has external ports (`hierarchyMode`).
pub fn hide(graph: &mut LGraph, holder: &mut SelfLoopHolder) -> Result<(), LayoutError> {
    for hyper in &holder.hyper_loops {
        for &edge in &hyper.sl_edges {
            graph.detach_edge(holder.sl_edges[edge].ledge);
        }
    }

    let constraints = graph.nodes[holder.node]
        .properties
        .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS);
    if ports::is_order_fixed(constraints) || graph.nodes[holder.node].nested_external_ports {
        return Ok(());
    }
    for index in 0..holder.sl_ports.len() {
        if !holder.sl_ports[index].had_only_self_loops {
            continue;
        }
        let lport = holder.sl_ports[index].lport;
        let node_ports = &mut graph.nodes[holder.node].ports;
        if let Some(position) = node_ports.iter().position(|&port| port == lport) {
            node_ports.remove(position);
        }
        holder.sl_ports[index].hidden = true;
        holder.ports_hidden = true;
    }
    Ok(())
}

/// The direction the graph is laid out in, `LayeredOptions.DIRECTION`.
pub fn graph_direction(graph: &LGraph) -> Direction {
    graph
        .properties
        .enum_with_default::<Direction>(options::DIRECTION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::math::KVector;
    use crate::layered::lgraph::LPort;

    fn sides(list: &[PortSide]) -> BTreeSet<PortSide> {
        list.iter().copied().collect()
    }

    #[test]
    fn the_loop_type_follows_the_port_sides() {
        use PortSide::{EAST, NORTH, SOUTH, WEST};
        assert_eq!(
            SelfLoopType::from_port_sides(&sides(&[NORTH])).unwrap(),
            Some(SelfLoopType::ONE_SIDE)
        );
        assert_eq!(
            SelfLoopType::from_port_sides(&sides(&[NORTH, EAST])).unwrap(),
            Some(SelfLoopType::TWO_SIDES_CORNER)
        );
        assert_eq!(
            SelfLoopType::from_port_sides(&sides(&[NORTH, SOUTH])).unwrap(),
            Some(SelfLoopType::TWO_SIDES_OPPOSING)
        );
        assert_eq!(
            SelfLoopType::from_port_sides(&sides(&[EAST, WEST])).unwrap(),
            Some(SelfLoopType::TWO_SIDES_OPPOSING)
        );
        assert_eq!(
            SelfLoopType::from_port_sides(&sides(&[NORTH, EAST, SOUTH])).unwrap(),
            Some(SelfLoopType::THREE_SIDES)
        );
        assert_eq!(
            SelfLoopType::from_port_sides(&sides(&[NORTH, EAST, SOUTH, WEST])).unwrap(),
            Some(SelfLoopType::FOUR_SIDES)
        );
        assert!(SelfLoopType::from_port_sides(&sides(&[PortSide::UNDEFINED])).is_err());
    }

    #[test]
    fn setting_a_side_moves_the_anchor_to_the_matching_edge_of_the_port() {
        let mut graph = LGraph::default();
        graph.ports.push(LPort::new(0, PortSide::UNDEFINED));
        graph.ports[0].size = KVector { x: 10.0, y: 4.0 };

        set_port_side(&mut graph, 0, PortSide::EAST);
        assert_eq!(graph.ports[0].anchor, KVector { x: 10.0, y: 2.0 });
        set_port_side(&mut graph, 0, PortSide::NORTH);
        assert_eq!(graph.ports[0].anchor, KVector { x: 5.0, y: 0.0 });
        set_port_side(&mut graph, 0, PortSide::SOUTH);
        assert_eq!(graph.ports[0].anchor, KVector { x: 5.0, y: 4.0 });
        set_port_side(&mut graph, 0, PortSide::WEST);
        assert_eq!(graph.ports[0].anchor, KVector { x: 0.0, y: 2.0 });
    }
}
