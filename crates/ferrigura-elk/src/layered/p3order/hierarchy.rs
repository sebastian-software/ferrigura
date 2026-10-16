//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/LayerSweepCrossingMinimizer.java
//!   with GraphInfoHolder.java, LayerSweepTypeDecider.java and SweepCopy.java
//!
//! The layer sweep with the barycenter heuristic over a hierarchy of graphs: the graph of a flat
//! layout, or the graphs of a compound layout with the top graph first. Each graph gets a
//! `GraphInfoHolder`. A graph is swept on its own (bottom-up) or, when `LayerSweepTypeDecider`
//! finds that its node's ports matter more than random influences, as part of the sweep of the
//! graph around it: the sweep enters the nested graph at the node's layer, orders its first layer
//! of external port dummies by the node's ports, and orders the node's ports by the dummies of its
//! last layer.
//!
//! The minimizer's generator is the top graph's; it draws the sweep directions and is reset to the
//! seed for each graph swept bottom-up. Each graph's heuristic draws from its own graph's generator,
//! which for the top graph is the minimizer's.

use crate::core::data::OptionValue;
use crate::core::data::enums::{NodeType, OrderingStrategy, PortSide};
use crate::java::random::Random;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::ports::is_order_fixed;

use super::barycenter::BarycenterHeuristic;
use super::counting::AllCrossingsCounter;
use super::port_distributor::PortDistributor;
use super::{
    Layout, Net, check_supported, in_north_south_east_west_order, in_sweep_range, start_index,
    transfer_node_and_port_orders,
};

/// `GraphInfoHolder` for the barycenter heuristic.
struct GraphSweep<'g> {
    /// The index of the graph in the layout's list of graphs.
    graph: usize,
    net: Net<'g>,
    /// `currentNodeOrder`, with the live port lists of the graph's nodes.
    layout: Layout,
    /// `crossMinimizer` with its `portDistributor`.
    heuristic: BarycenterHeuristic,
    /// `crossingsCounter`.
    counter: AllCrossingsCounter,
    /// The generator of the graph's heuristic, when it is not the top graph's.
    random: Option<Random>,
    /// `InternalProperties.FIRST_TRY_WITH_INITIAL_ORDER` of the graph. Java's
    /// `SECOND_TRY_WITH_INITIAL_ORDER` has the same id (`firstTryWithInitialOrder`), and graph
    /// properties are keyed by id, so both are this one flag.
    first_try: bool,
    /// `CONSIDER_MODEL_ORDER_STRATEGY != NONE` of the graph.
    consider_model_order: bool,
    /// `dontSweepInto()`: the graph is swept on its own.
    bottom_up: bool,
    /// `childGraphs`: the graphs nested in the graph's nodes, in the order of the layers.
    child_graphs: Vec<usize>,
    /// The holder of the graph's parent graph and the node it is nested in.
    parent: Option<(usize, usize)>,
    /// `hasExternalPorts`.
    has_external_ports: bool,
    /// `currentlyBestNodeAndPortOrder`.
    currently_best: Option<Layout>,
    /// `bestNodeAndPortOrder`.
    best: Option<Layout>,
}

/// `LayerSweepCrossingMinimizer` for `CrossMinType.BARYCENTER`, with its holders in the order of the
/// graph ids (`graph.id`).
struct Hierarchy<'g> {
    holders: Vec<GraphSweep<'g>>,
    /// The holder of each graph of the layout, by the graph's index.
    holder_of: Vec<Option<usize>>,
    /// The minimizer's `random`: the top graph's generator.
    random: Random,
    /// `randomSeed`.
    seed: i64,
    /// `graphsWhoseNodeOrderChanged`: the holders swept since the attempts began.
    changed: Vec<usize>,
    /// A case the sweep met that the Java throws on or that is not ported, reported at the end.
    error: Option<LayoutError>,
}

/// `LayerSweepCrossingMinimizer.process` for `CrossMinType.BARYCENTER` on `graphs`, the top graph
/// first. `hierarchical` is whether the top graph's hierarchy handling is INCLUDE_CHILDREN.
pub(super) fn run(graphs: &mut [LGraph], hierarchical: bool) -> Result<(), LayoutError> {
    let top = &graphs[0];
    if top.layers.iter().all(Vec::is_empty) {
        return Ok(());
    }
    if top.layers.len() == 1 && top.layers[0].len() == 1 && !hierarchical {
        return Ok(());
    }
    let mut random = graphs[0]
        .random
        .take()
        .ok_or_else(|| LayoutError::Unsupported("a graph without a random source".to_string()))?;
    // `initialize`: the seed the attempts restart from, drawn first.
    let seed = random.next_long();
    let mut own_randoms: Vec<Option<Random>> = graphs.iter_mut().map(|g| g.random.take()).collect();
    let result = {
        let view: &[LGraph] = graphs;
        sweep_all(view, random, seed, &mut own_randoms)
    };
    let (random, orders, fixed_order_parents) = match result {
        Ok(result) => result,
        Err((error, random)) => {
            graphs[0].random = Some(random);
            for (graph, own) in graphs.iter_mut().zip(own_randoms).skip(1) {
                graph.random = own;
            }
            return Err(error);
        }
    };
    graphs[0].random = Some(random);
    for (graph, own) in graphs.iter_mut().zip(own_randoms).skip(1) {
        graph.random = own;
    }
    // `setPortOrderOnParentGraph` fixed the port order of these nodes.
    for (graph, node) in fixed_order_parents {
        graphs[graph].nodes[node].properties.set(
            options::PORT_CONSTRAINTS,
            Some(OptionValue::Enum {
                class: "PortConstraints",
                name: "FIXED_ORDER".to_string(),
            }),
        );
    }
    // `transferNodeAndPortOrdersToGraph`, in the order of the graph ids.
    for (graph, best) in orders {
        transfer_node_and_port_orders(&mut graphs[graph], best);
    }
    Ok(())
}

/// The sweep of the whole hierarchy, on the graphs as they are. Returns the top graph's generator,
/// the best order of each graph that has one (by graph index, in the order of the graph ids), and
/// the nodes whose port order `setPortOrderOnParentGraph` fixed. On an error, the generator comes
/// back with it.
#[allow(clippy::type_complexity)]
fn sweep_all(
    graphs: &[LGraph],
    random: Random,
    seed: i64,
    own_randoms: &mut [Option<Random>],
) -> Result<(Random, Vec<(usize, Layout)>, Vec<(usize, usize)>), (LayoutError, Random)> {
    let mut hierarchy = Hierarchy {
        holders: Vec::new(),
        holder_of: vec![None; graphs.len()],
        random,
        seed,
        changed: Vec::new(),
        error: None,
    };
    match hierarchy
        .initialize(graphs, own_randoms)
        .and_then(|to_sweep| {
            let fixed = hierarchy.minimize_crossings(&to_sweep);
            hierarchy.error.take().map_or(Ok(fixed), Err)
        }) {
        Ok(fixed) => {
            for holder in &mut hierarchy.holders {
                if holder.graph != 0 {
                    own_randoms[holder.graph] = holder.random.take();
                }
            }
            let orders = hierarchy
                .holders
                .iter_mut()
                .filter_map(|holder| holder.best.take().map(|best| (holder.graph, best)))
                .collect();
            Ok((hierarchy.random, orders, fixed))
        }
        Err(error) => {
            for holder in &mut hierarchy.holders {
                if holder.graph != 0 {
                    own_randoms[holder.graph] = holder.random.take();
                }
            }
            Err((error, hierarchy.random))
        }
    }
}

impl<'g> Hierarchy<'g> {
    /// `initialize`: a holder for each graph, breadth first through the nested graphs in the order
    /// of the nodes in the layers. Returns `graphsToSweepOn`: the holders swept bottom-up, the last
    /// one first.
    fn initialize(
        &mut self,
        graphs: &'g [LGraph],
        own_randoms: &mut [Option<Random>],
    ) -> Result<Vec<usize>, LayoutError> {
        let mut queue = vec![0];
        let mut next = 0;
        let mut to_sweep = Vec::new();
        while next < queue.len() {
            let index = queue[next];
            next += 1;
            let graph = &graphs[index];
            check_supported(graph)?;
            let holder_index = self.holders.len();
            self.holder_of[index] = Some(holder_index);
            let net = Net::new(graph);
            let layout = Layout::initial(graph);
            // `ISweepPortDistributor.create` draws from the graph's own generator.
            let mut own = if index == 0 {
                None
            } else {
                Some(own_randoms[index].take().ok_or_else(|| {
                    LayoutError::Unsupported("a graph without a random source".to_string())
                })?)
            };
            let node_relative = match own.as_mut() {
                Some(random) => random.next_boolean(),
                None => self.random.next_boolean(),
            };
            let ports = PortDistributor::new(&net, node_relative);
            let heuristic = BarycenterHeuristic::new(&net, &layout, ports);
            let counter = AllCrossingsCounter::new(&net, &layout);
            // `initAtNodeLevel`: the nested graphs, in the order of the layers.
            let child_graphs: Vec<usize> = graph
                .layers
                .iter()
                .flatten()
                .filter_map(|&node| graph.nodes[node].nested_graph)
                .collect();
            queue.extend(child_graphs.iter().copied());
            let parent = graph.parent_node.map(|(parent_graph, node)| {
                (self.holder_of[parent_graph].unwrap_or(usize::MAX), node)
            });
            let consider_model_order = graph
                .properties
                .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
                .is_some_and(|strategy| strategy != OrderingStrategy::NONE);
            self.holders.push(GraphSweep {
                graph: index,
                net,
                layout,
                heuristic,
                counter,
                random: own,
                first_try: false,
                consider_model_order,
                bottom_up: true,
                child_graphs,
                parent,
                has_external_ports: graph
                    .graph_properties
                    .contains(&crate::core::data::enums::GraphProperties::EXTERNAL_PORTS),
                currently_best: None,
                best: None,
            });
            let bottom_up = self.use_bottom_up(holder_index)?;
            self.holders[holder_index].bottom_up = bottom_up;
            if bottom_up {
                to_sweep.insert(0, holder_index);
            }
        }
        Ok(to_sweep)
    }

    /// `LayerSweepTypeDecider.useBottomUp`.
    fn use_bottom_up(&self, holder: usize) -> Result<bool, LayoutError> {
        let data = &self.holders[holder];
        let graph = data.net.graph();
        let boundary = graph
            .properties
            .double_with_default(options::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS);
        let Some((parent_holder, parent_node)) = data.parent else {
            return Ok(true);
        };
        let parent = &self.holders[parent_holder];
        let parent_ports = &parent.layout.node_ports[parent_node];
        let fixed_port_order = is_order_fixed(parent.net.constraints(parent_node));
        let count = |side: PortSide| {
            parent_ports
                .iter()
                .filter(|&&port| parent.net.side(port) == side)
                .count()
        };
        let fewer_than_two = count(PortSide::EAST) < 2 && count(PortSide::WEST) < 2;
        if boundary < -1.0 || fixed_port_order || fewer_than_two {
            return Ok(true);
        }
        // The barycenter heuristic is not deterministic, so the paths decide.
        let net = &data.net;
        let layout = &data.layout;
        let order = &layout.order;
        // `nodeInfo`, by layer and position: connected edges, hierarchical and random influence.
        let mut info: Vec<Vec<[i64; 3]>> = order
            .iter()
            .map(|layer| vec![[0; 3]; layer.len()])
            .collect();
        let position_of = |node: usize| -> (usize, usize) {
            let layer = net.layer(node);
            let index = order[layer]
                .iter()
                .position(|&member| member == node)
                .unwrap_or(0);
            (layer, index)
        };
        let mut paths_to_random: i64 = 0;
        let mut paths_to_hierarchical: i64 = 0;
        for (layer_index, layer) in order.iter().enumerate() {
            let mut north_south_dummies = Vec::new();
            for (node_index, &node) in layer.iter().enumerate() {
                if net.node_type(node) == NodeType::NORTH_SOUTH_PORT {
                    north_south_dummies.push(node);
                    continue;
                }
                let ports = &layout.node_ports[node];
                if net.node_type(node) == NodeType::EXTERNAL_PORT {
                    info[layer_index][node_index][1] = 1;
                    // `originPort(node).getSide()`: the port on the node of the parent graph.
                    let eastern =
                        graph.nodes[node]
                            .external_port
                            .as_ref()
                            .is_some_and(|external| {
                                parent.net.graph().ports[external.origin].side == PortSide::EAST
                            });
                    if eastern {
                        paths_to_hierarchical += info[layer_index][node_index][0];
                    }
                } else if has_no_connected_ports(net, ports, PortSide::WEST) {
                    info[layer_index][node_index][2] = 1;
                } else if has_no_connected_ports(net, ports, PortSide::EAST) {
                    paths_to_random += info[layer_index][node_index][0];
                }
                let current = info[layer_index][node_index];
                for &port in ports {
                    for &edge in net.outgoing(port) {
                        paths_to_random += current[2];
                        paths_to_hierarchical += current[1];
                        let target = net.node_of(net.target(edge));
                        let (target_layer, target_index) = position_of(target);
                        let target_info = &mut info[target_layer][target_index];
                        target_info[1] += current[1];
                        target_info[2] += current[2];
                        target_info[0] += current[0];
                        target_info[0] += 1;
                    }
                }
                // The north and south ports pass the influence to their dummies. A hierarchical port
                // on those sides would pass it to a dummy of another graph, which is reported.
                if ports.iter().any(|&port| {
                    matches!(net.side(port), PortSide::NORTH | PortSide::SOUTH)
                        && graph.ports[port].port_dummy.is_some()
                }) {
                    return Err(LayoutError::Unsupported(
                        "north or south hierarchical ports in LayerSweepTypeDecider".to_string(),
                    ));
                }
                let north_south_ports = net
                    .side_ports(ports, PortSide::NORTH)
                    .into_iter()
                    .chain(net.side_ports(ports, PortSide::SOUTH));
                for port in north_south_ports {
                    if let Some(dummy) = graph.ports[port].north_south_dummy {
                        paths_to_random += current[2];
                        paths_to_hierarchical += current[1];
                        let (dummy_layer, dummy_index) = position_of(dummy);
                        let dummy_info = &mut info[dummy_layer][dummy_index];
                        dummy_info[1] += current[1];
                        dummy_info[2] += current[2];
                        dummy_info[0] += current[0];
                        dummy_info[0] += 1;
                    }
                }
            }
            for node in north_south_dummies {
                let (node_layer, node_index) = position_of(node);
                let current = info[node_layer][node_index];
                for &port in &layout.node_ports[node] {
                    for &edge in net.outgoing(port) {
                        paths_to_random += current[2];
                        paths_to_hierarchical += current[1];
                        let target = net.node_of(net.target(edge));
                        let (target_layer, target_index) = position_of(target);
                        let target_info = &mut info[target_layer][target_index];
                        target_info[1] += current[1];
                        target_info[2] += current[2];
                        target_info[0] += current[0];
                        target_info[0] += 1;
                    }
                }
            }
        }
        let all_paths = (paths_to_random + paths_to_hierarchical) as f64;
        let normalized = if all_paths == 0.0 {
            f64::INFINITY
        } else {
            (paths_to_random - paths_to_hierarchical) as f64 / all_paths
        };
        Ok(normalized >= boundary)
    }

    /// `minimizeCrossings`: each graph swept bottom-up, the innermost first, with
    /// `compareDifferentRandomizedLayouts`; a nested graph then orders its node's ports. Returns
    /// the nodes whose port order that fixed, by graph index and node.
    fn minimize_crossings(&mut self, to_sweep: &[usize]) -> Vec<(usize, usize)> {
        let mut fixed = Vec::new();
        for &holder in to_sweep {
            if self.holders[holder].layout.order.is_empty() {
                continue;
            }
            self.compare_different_randomized_layouts(holder);
            if let Some((parent_holder, parent_node)) = self.holders[holder].parent
                && self.set_port_order_on_parent_graph(holder)
            {
                fixed.push((self.holders[parent_holder].graph, parent_node));
            }
        }
        fixed
    }

    /// `setPortOrderOnParentGraph`: a graph with external ports orders its node's eastern and
    /// western hierarchical ports by the dummies of its best sweep, and fixes the node's port order.
    fn set_port_order_on_parent_graph(&mut self, holder: usize) -> bool {
        if !self.holders[holder].has_external_ports {
            return false;
        }
        let Some(best) = self.holders[holder]
            .best
            .as_ref()
            .map(|best| best.order.clone())
        else {
            return false;
        };
        self.sort_ports_by_dummy_positions_in_last_layer(holder, &best, true);
        self.sort_ports_by_dummy_positions_in_last_layer(holder, &best, false);
        if let Some((parent_holder, parent_node)) = self.holders[holder].parent {
            self.holders[parent_holder].net.fix_port_order(parent_node);
        }
        true
    }

    /// `compareDifferentRandomizedLayouts`, for the crossing counter without model order influence.
    fn compare_different_randomized_layouts(&mut self, holder: usize) {
        self.random.set_seed(self.seed);
        self.changed.clear();
        if self.holders[holder].consider_model_order {
            // The first run begins with a forward sweep, so a correct initial order is kept.
            self.holders[holder].first_try = true;
        }
        let thoroughness = self.holders[holder]
            .net
            .graph()
            .properties
            .int_with_default(options::THOROUGHNESS);
        let mut best_crossings = i32::MAX;
        for _ in 0..thoroughness {
            let crossings = self.minimize_crossings_with_counter(holder);
            if crossings < best_crossings {
                best_crossings = crossings;
                self.save_all_node_orders_of_changed_graphs();
                if best_crossings == 0 {
                    break;
                }
            }
        }
    }

    /// `minimizeCrossingsWithCounter`.
    fn minimize_crossings_with_counter(&mut self, holder: usize) -> i32 {
        let mut forward = self.random.next_boolean();
        // If the initial order is already free of crossings and the model order decides it, keep it.
        let initial_crossings = self.count_current_number_of_crossings(holder);
        if initial_crossings == 0 && self.holders[holder].first_try {
            return 0;
        }
        if !self.holders[holder].first_try || !self.holders[holder].consider_model_order {
            self.set_first_layer_order(holder, forward);
        } else {
            forward = self.holders[holder].first_try;
        }
        self.sweep_reducing_crossings(holder, forward, true);
        // Java clears the second try, then tests the first one, which is now clear: so the flag
        // ends cleared, and the second try never starts.
        self.holders[holder].first_try = false;
        let mut crossings = self.count_current_number_of_crossings(holder);
        loop {
            self.set_currently_best_node_orders();
            if crossings == 0 {
                return 0;
            }
            forward = !forward;
            let old_crossings = crossings;
            self.sweep_reducing_crossings(holder, forward, false);
            crossings = self.count_current_number_of_crossings(holder);
            if old_crossings <= crossings {
                return old_crossings;
            }
        }
    }

    /// `countCurrentNumberOfCrossings`: the crossings of the graph and of the nested graphs it
    /// sweeps into.
    fn count_current_number_of_crossings(&mut self, holder: usize) -> i32 {
        let data = &mut self.holders[holder];
        let mut total = data.counter.count_all(&data.net, &data.layout);
        for child in self.holders[holder].child_graphs.clone() {
            if let Some(child_holder) = self.holder_of[child]
                && !self.holders[child_holder].bottom_up
            {
                total += self.count_current_number_of_crossings(child_holder);
            }
        }
        total
    }

    /// `ICrossingMinimizationHeuristic.setFirstLayerOrder` of the holder's heuristic.
    fn set_first_layer_order(&mut self, holder: usize, forward: bool) {
        let data = &mut self.holders[holder];
        let random = match data.random.as_mut() {
            Some(random) => random,
            None => &mut self.random,
        };
        if let Err(error) =
            data.heuristic
                .set_first_layer_order(&data.net, &mut data.layout, random, forward)
        {
            self.error.get_or_insert(error);
        }
    }

    /// `sweepReducingCrossings`: the port distribution of the first layer, then each following layer
    /// is reordered by its barycenters and its ports are distributed; the nested graphs swept into
    /// follow each layer.
    fn sweep_reducing_crossings(&mut self, holder: usize, forward: bool, first_sweep: bool) {
        let length = self.holders[holder].layout.order.len();
        let first = start_index(forward, length);
        {
            let data = &mut self.holders[holder];
            data.heuristic.ports.distribute_ports_while_sweeping(
                &data.net,
                &mut data.layout,
                first,
                forward,
            );
        }
        self.sweep_in_hierarchical_nodes(holder, first, forward, first_sweep);
        // `firstSweep && !FIRST_TRY && !SECOND_TRY`, passed to every layer of this sweep.
        let is_first_sweep = first_sweep && !self.holders[holder].first_try;
        let (step, mut index) = if forward {
            (1_isize, 1_isize)
        } else {
            (-1_isize, length as isize - 2)
        };
        while in_sweep_range(index, length, forward) {
            let free = index as usize;
            {
                let data = &mut self.holders[holder];
                let random = match data.random.as_mut() {
                    Some(random) => random,
                    None => &mut self.random,
                };
                if let Err(error) = data.heuristic.minimize_crossings(
                    &data.net,
                    &mut data.layout,
                    random,
                    free,
                    forward,
                    is_first_sweep,
                ) {
                    self.error.get_or_insert(error);
                }
                data.heuristic.ports.distribute_ports_while_sweeping(
                    &data.net,
                    &mut data.layout,
                    free,
                    forward,
                );
            }
            self.sweep_in_hierarchical_nodes(holder, free, forward, first_sweep);
            index += step;
        }
        // `graphsWhoseNodeOrderChanged.add(graph)`.
        if !self.changed.contains(&holder) {
            self.changed.push(holder);
        }
    }

    /// `sweepInHierarchicalNodes`: the nested graphs of the layer's nodes that are swept into.
    fn sweep_in_hierarchical_nodes(
        &mut self,
        holder: usize,
        layer: usize,
        forward: bool,
        first_sweep: bool,
    ) {
        let nodes = self.holders[holder].layout.order[layer].clone();
        for node in nodes {
            let Some(nested) = self.holders[holder].net.graph().nodes[node].nested_graph else {
                continue;
            };
            let Some(nested_holder) = self.holder_of[nested] else {
                continue;
            };
            if !self.holders[nested_holder].bottom_up {
                self.sweep_in_hierarchical_node(nested_holder, holder, node, forward, first_sweep);
            }
        }
    }

    /// `sweepInHierarchicalNode`: the nested graph's first layer follows the node's ports, or is
    /// ordered at random, then the nested graph is swept, and the node's ports follow the nested
    /// graph's last layer.
    fn sweep_in_hierarchical_node(
        &mut self,
        nested: usize,
        holder: usize,
        node: usize,
        forward: bool,
        first_sweep: bool,
    ) {
        let length = self.holders[nested].layout.order.len();
        let start = start_index(forward, length);
        let first_node = self.holders[nested].layout.order[start].first().copied();
        let first_is_external = first_node.is_some_and(|first| {
            self.holders[nested].net.node_type(first) == NodeType::EXTERNAL_PORT
        });
        if first_is_external {
            let side = if forward {
                PortSide::WEST
            } else {
                PortSide::EAST
            };
            match self.sort_port_dummies_by_port_positions(holder, node, nested, start, side) {
                Ok(sorted) => self.holders[nested].layout.order[start] = sorted,
                Err(error) => {
                    self.error.get_or_insert(error);
                }
            }
        } else {
            self.set_first_layer_order(nested, forward);
        }
        self.sweep_reducing_crossings(nested, forward, first_sweep);
        let order = self.holders[nested].layout.order.clone();
        self.sort_ports_by_dummy_positions_in_last_layer(nested, &order, forward);
    }

    /// `sortPortDummiesByPortPositions`: the dummies of the hierarchical ports on `side` of the node,
    /// in the order of its ports. The layer keeps any place the ports do not fill.
    fn sort_port_dummies_by_port_positions(
        &self,
        holder: usize,
        node: usize,
        nested: usize,
        layer: usize,
        side: PortSide,
    ) -> Result<Vec<usize>, LayoutError> {
        let parent = &self.holders[holder];
        let ports =
            in_north_south_east_west_order(&parent.net, &parent.layout.node_ports[node], side);
        let current = &self.holders[nested].layout.order[layer];
        let dummies: Vec<usize> = ports
            .into_iter()
            .filter_map(|port| {
                let data = &parent.net.graph().ports[port];
                if data.inside_connections {
                    data.port_dummy
                } else {
                    None
                }
            })
            .collect();
        if dummies.len() == current.len() {
            Ok(dummies)
        } else {
            // The Java throws when there are fewer ports than dummies, and fails on more.
            Err(LayoutError::Unsupported(format!(
                "expected {} hierarchical ports, but found {}",
                current.len(),
                dummies.len()
            )))
        }
    }

    /// `sortPortsByDummyPositionsInLastLayer`: the node's hierarchical ports on the side the sweep
    /// ends at take the order of the external port dummies in the nested graph's last layer.
    fn sort_ports_by_dummy_positions_in_last_layer(
        &mut self,
        nested: usize,
        order: &[Vec<usize>],
        on_right_most_layer: bool,
    ) {
        let Some((parent_holder, parent_node)) = self.holders[nested].parent else {
            return;
        };
        let end = if on_right_most_layer {
            order.len().saturating_sub(1)
        } else {
            0
        };
        let Some(last_layer) = order.get(end) else {
            return;
        };
        let mut j: isize = if on_right_most_layer {
            0
        } else {
            last_layer.len() as isize - 1
        };
        let step: isize = if on_right_most_layer { 1 } else { -1 };
        let nested_net = &self.holders[nested].net;
        let is_external = |index: isize| {
            usize::try_from(index)
                .ok()
                .and_then(|index| last_layer.get(index))
                .is_some_and(|&node| nested_net.node_type(node) == NodeType::EXTERNAL_PORT)
        };
        if !is_external(j) {
            return;
        }
        let side = if on_right_most_layer {
            PortSide::EAST
        } else {
            PortSide::WEST
        };
        let origins: Vec<Option<usize>> = last_layer
            .iter()
            .map(|&node| {
                nested_net.graph().nodes[node]
                    .external_port
                    .as_ref()
                    .map(|external| external.origin)
            })
            .collect();
        let parent = &mut self.holders[parent_holder];
        let ports = parent.layout.node_ports[parent_node].clone();
        let mut new_ports = ports.clone();
        for (index, &port) in ports.iter().enumerate() {
            let data = &parent.net.graph().ports[port];
            if data.side == side && data.inside_connections {
                let origin = usize::try_from(j)
                    .ok()
                    .and_then(|j| origins.get(j).copied().flatten());
                if let Some(origin) = origin {
                    new_ports[index] = origin;
                }
                j += step;
            }
        }
        parent.layout.node_ports[parent_node] = new_ports;
    }

    /// `setCurrentlyBestNodeOrders`.
    fn set_currently_best_node_orders(&mut self) {
        for &holder in &self.changed {
            let data = &mut self.holders[holder];
            data.currently_best = Some(data.layout.clone());
        }
    }

    /// `saveAllNodeOrdersOfChangedGraphs`.
    fn save_all_node_orders_of_changed_graphs(&mut self) {
        for &holder in &self.changed {
            let data = &mut self.holders[holder];
            data.best = data.currently_best.clone();
        }
    }
}

/// `hasNoWesternPorts` and `hasNoEasternPorts`: no port on the side has an edge.
fn has_no_connected_ports(net: &Net, ports: &[usize], side: PortSide) -> bool {
    ports
        .iter()
        .filter(|&&port| net.side(port) == side)
        .all(|&port| net.degree(port) == 0)
}
