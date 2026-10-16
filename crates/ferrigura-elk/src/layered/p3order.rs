//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/LayerSweepCrossingMinimizer.java
//!   with GraphInfoHolder.java, SweepCopy.java, ISweepPortDistributor.java, LayerSweepTypeDecider.java
//!   and CrossMinUtil.java
//!
//! The layer sweep crossing minimizer with the barycenter heuristic, the default, for the graph of
//! a flat layout or the graphs of a compound layout (`hierarchy`). Each attempt starts from a
//! random order of one layer, sweeps back and forth over the layers and keeps the order with the
//! fewest crossings. The best attempt's node order and port order are written back to the graph.
//!
//! The same minimizer runs as the two-sided greedy switch processor before node placement
//! (`greedy_switch`): the deterministic greedy switch heuristic with the greedy port distributor,
//! swept in alternating directions until a sweep improves nothing. The one-sided greedy switch is
//! reported as `Unsupported`.
//!
//! Not ported, and reported as `Unsupported`: the model-order barycenter (`forceNodeModelOrder`),
//! the crossing counter with node model-order influence, north and south port dummies (and the
//! barycenter associates they set), in-layer constraints, and self-loops. Without in-layer
//! constraints, `ForsterConstraintResolver.processConstraints` passes each layer through unchanged.

mod barycenter;
mod counting;
mod greedy_port_distributor;
mod greedy_switch;
mod hierarchy;
mod port_distributor;

use crate::core::data::OptionValue;
use crate::core::data::enums::{HierarchyHandling, NodeType, PortConstraints, PortSide};
use crate::java::random::Random;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::ports::is_order_fixed;

use greedy_port_distributor::GreedyPortDistributor;
use greedy_switch::GreedySwitchHeuristic;

/// `LayerSweepCrossingMinimizer.process` for `CrossMinType.BARYCENTER`, the layer sweep of the
/// default crossing minimization. The task's `begin` and `done` are the phase's.
pub fn sweep(graph: &mut LGraph) -> Result<(), LayoutError> {
    let hierarchical = graph
        .properties
        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
        == Some(HierarchyHandling::INCLUDE_CHILDREN);
    hierarchy::run(std::slice::from_mut(graph), hierarchical)
}

/// `LayerSweepCrossingMinimizer.process` for `CrossMinType.BARYCENTER` in a compound layout: the
/// top graph runs the minimizer for the graphs of the whole hierarchy (see
/// `driver::hierarchical_layout`). `graphs[0]` is the top graph.
pub fn sweep_compound(graphs: &mut [LGraph]) -> Result<(), LayoutError> {
    hierarchy::run(graphs, true)
}

/// `LayerSweepCrossingMinimizer.process` for the greedy switch types, the processor that runs before
/// node placement: `ONE_SIDED_GREEDY_SWITCH` when `one_sided`, `TWO_SIDED_GREEDY_SWITCH` otherwise.
/// The task's `begin` and `done` are the processor's.
///
/// The greedy switch heuristic is deterministic and the two-sided one always improves, so
/// `chooseMinimizingMethod` picks `minimizeCrossingsNoCounter` and the best sweep is the last one.
/// The one-sided type takes `minimizeCrossingsWithCounter` with a barycenter port distributor and the
/// first and second try flags the barycenter phase leaves on the graph; it is not ported.
pub fn greedy_switch(graph: &mut LGraph, one_sided: bool) -> Result<(), LayoutError> {
    // The short-circuits of `process`, before anything is drawn from the random generator.
    if graph.layers.iter().all(Vec::is_empty) {
        return Ok(());
    }
    let hierarchical = graph
        .properties
        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
        == Some(HierarchyHandling::INCLUDE_CHILDREN);
    if graph.layers.len() == 1 && graph.layers[0].len() == 1 && !hierarchical {
        return Ok(());
    }
    if one_sided {
        return Err(LayoutError::Unsupported(
            "the one-sided greedy switch (GreedySwitchType.ONE_SIDED)".to_string(),
        ));
    }
    check_structure(graph)?;
    let mut random = graph
        .random
        .take()
        .ok_or_else(|| LayoutError::Unsupported("a graph without a random source".to_string()))?;
    // `initialize`: the seed the randomized attempts would restart from, drawn though the greedy
    // switch never uses it. `ISweepPortDistributor.create` draws nothing for the two-sided type.
    random.next_long();
    let best = {
        let net = Net::new(graph);
        let mut layout = Layout::initial(graph);
        minimize_crossings_no_counter(&net, &mut layout, &mut random).map(|()| layout)
    };
    graph.random = Some(random);
    // `transferNodeAndPortOrdersToGraph`: the deterministic heuristic's best sweep is
    // `currentlyBestNodeAndPortOrder`, the order after the last sweep.
    transfer_node_and_port_orders(graph, best?);
    Ok(())
}

/// `minimizeCrossingsNoCounter`: sweeps in alternating directions, each starting with one pass of
/// switches over its first layer, until a sweep changes nothing. The root graph is swept, so
/// `setCurrentlyBestNodeOrders` saves the layout as it ends.
fn minimize_crossings_no_counter(
    net: &Net,
    layout: &mut Layout,
    random: &mut Random,
) -> Result<(), LayoutError> {
    let mut heuristic = GreedySwitchHeuristic::new(net, layout);
    let mut ports = GreedyPortDistributor::new(net);
    let mut forward = random.next_boolean();
    let mut improved = true;
    while improved {
        improved = heuristic.set_first_layer_order(net, layout, forward)?;
        improved |=
            greedy_sweep_reducing_crossings(net, layout, &mut heuristic, &mut ports, forward)?;
        forward = !forward;
    }
    Ok(())
}

/// `sweepReducingCrossings` with the greedy switch: the port distribution of the first layer, then
/// each following layer is switched until it improves no more and its ports are distributed. The
/// root graph's layers hold no nested graphs, so `sweepInHierarchicalNodes` changes nothing.
fn greedy_sweep_reducing_crossings(
    net: &Net,
    layout: &mut Layout,
    heuristic: &mut GreedySwitchHeuristic,
    ports: &mut GreedyPortDistributor,
    forward: bool,
) -> Result<bool, LayoutError> {
    let length = layout.order.len();
    let mut improved = ports.distribute_ports_while_sweeping(
        net,
        layout,
        start_index(forward, length),
        forward,
    )?;
    let (step, mut index) = if forward {
        (1_isize, 1_isize)
    } else {
        (-1_isize, length as isize - 2)
    };
    while in_sweep_range(index, length, forward) {
        let free = index as usize;
        improved |= heuristic.minimize_crossings(net, layout, free)?;
        improved |= ports.distribute_ports_while_sweeping(net, layout, free, forward)?;
        index += step;
    }
    Ok(improved)
}

/// The cases this port does not cover, reported before the sweep changes anything.
pub(super) fn check_supported(graph: &LGraph) -> Result<(), LayoutError> {
    if graph.properties.bool_of(options::FORCE_NODE_MODEL_ORDER) {
        return Err(LayoutError::Unsupported(
            "crossingMinimization.forceNodeModelOrder (ModelOrderBarycenterHeuristic)".to_string(),
        ));
    }
    // `LayerSweepCrossingMinimizer.compareDifferentRandomizedLayouts` tests the node influence twice,
    // as the Java does; with a non-zero node influence the model-order counter runs, which is not ported.
    if graph
        .properties
        .double_with_default(options::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE)
        != 0.0
    {
        return Err(LayoutError::Unsupported(
            "the crossing counter with model order influence".to_string(),
        ));
    }
    check_structure(graph)
}

/// The parts of a graph no layer sweep of this port handles, the barycenter's or the greedy switch's.
fn check_structure(graph: &LGraph) -> Result<(), LayoutError> {
    // A self loop is detached while the self loop processors hold it, so only attached edges count.
    for (index, edge) in graph.edges.iter().enumerate() {
        if graph.is_attached(index)
            && graph.ports[edge.source].node == graph.ports[edge.target].node
        {
            return Err(LayoutError::Unsupported(
                "self-loops in crossing minimization".to_string(),
            ));
        }
    }
    for node in graph
        .layers
        .iter()
        .flatten()
        .map(|&node| &graph.nodes[node])
    {
        if node.node_type == NodeType::NORTH_SOUTH_PORT {
            return Err(LayoutError::Unsupported(
                "north-south port dummies in crossing minimization".to_string(),
            ));
        }
        // `ForsterConstraintResolver` keeps nodes with in-layer constraints at the top or the
        // bottom of their layers; the layered model has none of its own, but external port dummies
        // on the north or south side have them.
        if node.in_layer_constraint != crate::layered::lgraph::InLayerConstraint::NONE {
            return Err(LayoutError::Unsupported(
                "in-layer constraints in crossing minimization (ForsterConstraintResolver)"
                    .to_string(),
            ));
        }
    }
    for &node in graph.layers.iter().flatten() {
        for &port in &graph.nodes[node].ports {
            let side = graph.ports[port].side;
            let has_edges =
                !graph.ports[port].incoming.is_empty() || !graph.ports[port].outgoing.is_empty();
            if matches!(side, PortSide::NORTH | PortSide::SOUTH) && has_edges {
                return Err(LayoutError::Unsupported(
                    "edges on north or south ports (north-south port dummies)".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// The graph the sweep reads and does not change: ports, edges, node types, and the layer of each
/// node. `layer_of` is `LNode.getLayer()` and `layer_sizes` is `Layer.getNodes().size()`.
pub(super) struct Net<'a> {
    graph: &'a LGraph,
    layer_of: Vec<usize>,
    layer_sizes: Vec<usize>,
    /// The nodes whose port order the sweep fixed (`setPortOrderOnParentGraph`), which read as
    /// `FIXED_ORDER` from then on.
    fixed_order: Vec<bool>,
}

impl<'a> Net<'a> {
    fn new(graph: &'a LGraph) -> Self {
        let mut layer_of = vec![usize::MAX; graph.nodes.len()];
        for (index, layer) in graph.layers.iter().enumerate() {
            for &node in layer {
                layer_of[node] = index;
            }
        }
        Net {
            graph,
            layer_of,
            layer_sizes: graph.layers.iter().map(Vec::len).collect(),
            fixed_order: vec![false; graph.nodes.len()],
        }
    }

    /// `node.setProperty(PORT_CONSTRAINTS, FIXED_ORDER)` while the sweep reads the graph.
    pub(super) fn fix_port_order(&mut self, node: usize) {
        self.fixed_order[node] = true;
    }

    /// The graph the sweep reads.
    pub(super) fn graph(&self) -> &LGraph {
        self.graph
    }

    pub(super) fn side(&self, port: usize) -> PortSide {
        self.graph.ports[port].side
    }

    pub(super) fn node_of(&self, port: usize) -> usize {
        self.graph.ports[port].node
    }

    pub(super) fn layer(&self, node: usize) -> usize {
        self.layer_of[node]
    }

    pub(super) fn layer_of_port(&self, port: usize) -> usize {
        self.layer(self.node_of(port))
    }

    pub(super) fn layer_size(&self, node: usize) -> usize {
        self.layer_sizes[self.layer(node)]
    }

    pub(super) fn node_type(&self, node: usize) -> NodeType {
        self.graph.nodes[node].node_type
    }

    pub(super) fn constraints(&self, node: usize) -> PortConstraints {
        if self.fixed_order[node] {
            return PortConstraints::FIXED_ORDER;
        }
        port_constraints(self.graph, node)
    }

    /// `LPort.getIncomingEdges()`.
    pub(super) fn incoming(&self, port: usize) -> &[usize] {
        &self.graph.ports[port].incoming
    }

    /// `LPort.getOutgoingEdges()`.
    pub(super) fn outgoing(&self, port: usize) -> &[usize] {
        &self.graph.ports[port].outgoing
    }

    /// `LPort.getDegree()`: the incoming and the outgoing edges together.
    pub(super) fn degree(&self, port: usize) -> usize {
        self.graph.ports[port].incoming.len() + self.graph.ports[port].outgoing.len()
    }

    /// `LEdge.getSource()`, as a port index.
    pub(super) fn source(&self, edge: usize) -> usize {
        self.graph.edges[edge].source
    }

    /// `LEdge.getTarget()`, as a port index.
    pub(super) fn target(&self, edge: usize) -> usize {
        self.graph.edges[edge].target
    }

    /// `LPort.getConnectedEdges()`: the incoming edges, then the outgoing ones.
    pub(super) fn connected_edges(&self, port: usize) -> impl Iterator<Item = usize> + '_ {
        let port = &self.graph.ports[port];
        port.incoming.iter().chain(port.outgoing.iter()).copied()
    }

    /// `LPort.getConnectedPorts()`: the predecessor ports, then the successor ports.
    pub(super) fn connected_ports(&self, port: usize) -> impl Iterator<Item = usize> + '_ {
        let port = &self.graph.ports[port];
        port.incoming
            .iter()
            .map(move |&edge| self.source(edge))
            .chain(port.outgoing.iter().map(move |&edge| self.target(edge)))
    }

    /// `LEdge.getSource()` or `getTarget()`, whichever is not `port`.
    pub(super) fn other_end(&self, edge: usize, port: usize) -> usize {
        if port == self.source(edge) {
            self.target(edge)
        } else {
            self.source(edge)
        }
    }

    /// `LEdge.isInLayerEdge()` without the self-loop case, which `check_supported` excludes.
    pub(super) fn is_in_layer(&self, edge: usize) -> bool {
        self.layer_of_port(self.source(edge)) == self.layer_of_port(self.target(edge))
    }

    /// `LNode.getPortSideView(side)` on the given port list: the ports on `side`, in list order.
    pub(super) fn side_ports(&self, ports: &[usize], side: PortSide) -> Vec<usize> {
        ports
            .iter()
            .copied()
            .filter(|&port| self.side(port) == side)
            .collect()
    }

    /// `LEdge.isSelfLoop()`: both ends on one node.
    pub(super) fn is_self_loop(&self, edge: usize) -> bool {
        self.node_of(self.source(edge)) == self.node_of(self.target(edge))
    }
}

/// `CrossMinUtil.inNorthSouthEastWestOrder`: the ports of a port list on `side`, north to south and
/// east to west. North and east ports keep the list order; south and west ports, which the list
/// holds clockwise, are reversed.
pub(super) fn in_north_south_east_west_order(
    net: &Net,
    ports: &[usize],
    side: PortSide,
) -> Vec<usize> {
    match side {
        PortSide::EAST | PortSide::NORTH => net.side_ports(ports, side),
        PortSide::SOUTH | PortSide::WEST => {
            let mut ports = net.side_ports(ports, side);
            ports.reverse();
            ports
        }
        _ => Vec::new(),
    }
}

/// `LNode.getProperty(PORT_CONSTRAINTS)`, `UNDEFINED` when unset.
fn port_constraints(graph: &LGraph, node: usize) -> PortConstraints {
    graph.nodes[node]
        .properties
        .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS)
}

/// The order the sweep works on: the nodes of each layer (`currentNodeOrder`) and the port list of
/// each node, indexed by node, which the port distributors reorder. A copy of this is what
/// `SweepCopy` saves.
#[derive(Clone)]
pub(super) struct Layout {
    pub(super) order: Vec<Vec<usize>>,
    pub(super) node_ports: Vec<Vec<usize>>,
}

impl Layout {
    fn initial(graph: &LGraph) -> Self {
        Layout {
            order: graph.layers.clone(),
            node_ports: graph.nodes.iter().map(|node| node.ports.clone()).collect(),
        }
    }
}

/// `startIndex` and `firstIndex`: the layer a sweep starts from, the first layer for a forward sweep
/// and the last one otherwise.
pub(super) fn start_index(forward: bool, length: usize) -> usize {
    if forward { 0 } else { length.saturating_sub(1) }
}

/// `isNotEnd`: whether the free layer index is still inside the layers in the sweep direction.
pub(super) fn in_sweep_range(index: isize, length: usize, forward: bool) -> bool {
    if forward {
        index < length as isize
    } else {
        index >= 0
    }
}

/// `SweepCopy.transferNodeAndPortOrdersToGraph(lGraph, true)`: the layers and the port lists take the
/// saved order, and nodes whose port order was free become `FIXED_ORDER`. The north and south dummy
/// side corrections do not apply, as those dummies are unsupported.
pub(super) fn transfer_node_and_port_orders(graph: &mut LGraph, best: Layout) {
    for layer in &best.order {
        for &node in layer {
            graph.nodes[node].ports = best.node_ports[node].clone();
            if !is_order_fixed(port_constraints(graph, node)) {
                graph.nodes[node].properties.set(
                    options::PORT_CONSTRAINTS,
                    Some(OptionValue::Enum {
                        class: "PortConstraints",
                        name: "FIXED_ORDER".to_string(),
                    }),
                );
            }
        }
    }
    graph.layers = best.order;
}

#[cfg(test)]
mod tests {
    use super::counting::AllCrossingsCounter;
    use super::*;
    use crate::core::data::OptionValue;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    /// A graph with one layer of `count` nodes, in model order `0..count`.
    fn one_layer(count: usize, consider: Option<&str>) -> LGraph {
        let mut graph = LGraph {
            nodes: (0..count).map(|_| LNode::default()).collect(),
            layers: vec![(0..count).collect()],
            random: Some(Random::new(1)),
            ..LGraph::default()
        };
        if let Some(name) = consider {
            graph.properties.set(
                options::CONSIDER_MODEL_ORDER_STRATEGY,
                Some(OptionValue::Enum {
                    class: "OrderingStrategy",
                    name: name.to_string(),
                }),
            );
        }
        graph
    }

    /// Adds a port on `side` to `node` and returns its index.
    fn add_port(graph: &mut LGraph, node: usize, side: PortSide) -> usize {
        graph.ports.push(LPort::new(node, side));
        let port = graph.ports.len() - 1;
        graph.nodes[node].ports.push(port);
        port
    }

    /// Adds an edge from `source` to `target` and joins the two ports.
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

    /// The diamond n0 -> n1, n0 -> n2, n1 -> n3, n2 -> n3 with the layers [n0], [n1, n2], [n3]. Each
    /// edge has its own port. The port lists are in ELK's order: east before west, and the west
    /// ports of n3 as the edges enter them.
    fn diamond() -> LGraph {
        let mut graph = LGraph {
            nodes: vec![LNode::default(); 4],
            layers: vec![vec![0], vec![1, 2], vec![3]],
            random: Some(Random::new(1)),
            ..LGraph::default()
        };
        let p01 = add_port(&mut graph, 0, PortSide::EAST);
        let p02 = add_port(&mut graph, 0, PortSide::EAST);
        let o1 = add_port(&mut graph, 1, PortSide::EAST);
        let w1 = add_port(&mut graph, 1, PortSide::WEST);
        let o2 = add_port(&mut graph, 2, PortSide::EAST);
        let w2 = add_port(&mut graph, 2, PortSide::WEST);
        let w13 = add_port(&mut graph, 3, PortSide::WEST);
        let w23 = add_port(&mut graph, 3, PortSide::WEST);
        connect(&mut graph, p01, w1);
        connect(&mut graph, p02, w2);
        connect(&mut graph, o1, w13);
        connect(&mut graph, o2, w23);
        graph
    }

    #[test]
    fn model_order_is_kept_when_it_decides() {
        let mut graph = one_layer(4, Some("NODES_AND_EDGES"));
        sweep(&mut graph).unwrap();
        assert_eq!(graph.layers, vec![vec![0, 1, 2, 3]]);
    }

    #[test]
    fn random_order_is_a_permutation() {
        let mut graph = one_layer(5, None);
        sweep(&mut graph).unwrap();
        let mut order = graph.layers[0].clone();
        order.sort_unstable();
        assert_eq!(order, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_graph_with_one_node_is_left_alone() {
        let mut graph = one_layer(1, None);
        sweep(&mut graph).unwrap();
        assert_eq!(graph.layers, vec![vec![0]]);
    }

    #[test]
    fn the_diamond_is_untangled_as_elkjs_orders_it() {
        // elkjs puts n2 before n1 in the middle layer (its trace's blocks list n0, n2, n1, n3).
        // The order with n2 first has no crossing; the other has one.
        let mut graph = diamond();
        sweep(&mut graph).unwrap();
        assert_eq!(graph.layers, vec![vec![0], vec![2, 1], vec![3]]);
        let net = Net::new(&graph);
        let layout = Layout::initial(&graph);
        assert_eq!(
            AllCrossingsCounter::new(&net, &layout).count_all(&net, &layout),
            0
        );
    }

    #[test]
    fn the_diamond_reaches_zero_crossings_for_all_seeds() {
        // Not one order: the sweep reorders the west ports of n3, which can make [1, 2] crossing-free
        // as well as [2, 1]. The invariant is the crossing count, which must reach zero for every
        // seed, whichever port rule the seed draws.
        for seed in 0..40 {
            let mut graph = diamond();
            graph.random = Some(Random::new(seed));
            sweep(&mut graph).unwrap();
            let net = Net::new(&graph);
            let layout = Layout::initial(&graph);
            let crossings = AllCrossingsCounter::new(&net, &layout).count_all(&net, &layout);
            assert_eq!(crossings, 0, "seed={seed}");
        }
    }

    #[test]
    fn the_transfer_fixes_the_port_order() {
        let mut graph = diamond();
        sweep(&mut graph).unwrap();
        for node in graph.layers.iter().flatten() {
            assert_eq!(
                port_constraints(&graph, *node),
                PortConstraints::FIXED_ORDER
            );
            let mut ports = graph.nodes[*node].ports.clone();
            ports.sort_unstable();
            let mut expected: Vec<usize> = (0..graph.ports.len())
                .filter(|&port| graph.ports[port].node == *node)
                .collect();
            expected.sort_unstable();
            assert_eq!(ports, expected);
        }
    }

    /// Two layers [n0, n1] and [n2, n3] with n0 -> n3 and n1 -> n2: the two edges cross.
    fn crossed_pair() -> LGraph {
        let mut graph = LGraph {
            nodes: vec![LNode::default(); 4],
            layers: vec![vec![0, 1], vec![2, 3]],
            random: Some(Random::new(1)),
            ..LGraph::default()
        };
        let p0 = add_port(&mut graph, 0, PortSide::EAST);
        let p1 = add_port(&mut graph, 1, PortSide::EAST);
        let w2 = add_port(&mut graph, 2, PortSide::WEST);
        let w3 = add_port(&mut graph, 3, PortSide::WEST);
        connect(&mut graph, p0, w3);
        connect(&mut graph, p1, w2);
        graph
    }

    #[test]
    fn the_two_sided_greedy_switch_removes_a_crossing() {
        let mut graph = crossed_pair();
        greedy_switch(&mut graph, false).unwrap();
        // One of the two layers is switched, whichever the random direction starts with.
        let switched = graph.layers == vec![vec![1, 0], vec![2, 3]]
            || graph.layers == vec![vec![0, 1], vec![3, 2]];
        assert!(switched, "{:?}", graph.layers);
        for node in 0..4 {
            assert_eq!(port_constraints(&graph, node), PortConstraints::FIXED_ORDER);
        }
    }

    #[test]
    fn the_greedy_switch_draws_the_seed_and_the_direction() {
        // Layers of one node each switch nothing, but `initialize` draws the seed and
        // `minimizeCrossingsNoCounter` the first direction, as in the Java.
        let mut graph = crossed_pair();
        graph.layers = vec![vec![0], vec![2]];
        greedy_switch(&mut graph, false).unwrap();
        let mut expected = Random::new(1);
        expected.next_long();
        expected.next_boolean();
        let mut random = graph.random.take().unwrap();
        assert_eq!(random.next_long(), expected.next_long());
    }

    #[test]
    fn the_one_sided_greedy_switch_is_unsupported() {
        let mut graph = crossed_pair();
        assert!(matches!(
            greedy_switch(&mut graph, true),
            Err(LayoutError::Unsupported(_))
        ));
    }

    #[test]
    fn the_model_order_barycenter_is_unsupported() {
        let mut graph = diamond();
        graph.properties.set(
            options::FORCE_NODE_MODEL_ORDER,
            Some(OptionValue::Boolean(true)),
        );
        assert!(matches!(
            sweep(&mut graph),
            Err(LayoutError::Unsupported(_))
        ));
    }
}
