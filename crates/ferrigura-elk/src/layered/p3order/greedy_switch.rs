//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/greedyswitch/GreedySwitchHeuristic.java
//!   with SwitchDecider.java, CrossingMatrixFiller.java, BetweenLayerEdgeTwoNodeCrossingsCounter.java
//!   and NorthSouthEdgeNeighbouringNodeCrossingsCounter.java
//!
//! The two-sided greedy switch: two neighbouring nodes of the free layer change places when that
//! lowers the crossings of their edges to both neighbouring layers, plus their in-layer crossings.
//! The between-layer crossings of each pair come from a crossing matrix filled on demand by merging
//! the adjacency lists of the two nodes; the in-layer ones from two `CrossingsCounter`s that share
//! the heuristic's port position array.
//!
//! Only the two-sided type is here; the sweep reports the one-sided one as unsupported, so the
//! one-sided matrix filling (`countEasternEdgeCrossings`, `countWesternEdgeCrossings`) and its
//! `CrossingCountSide` are not ported. The parts that only matter in hierarchical graphs (the
//! parent's port crossings in `SwitchDecider`) are absent: only a root graph is swept.
//!
//! In-layer constraints are not in the layered model: `IN_LAYER_SUCCESSOR_CONSTRAINTS` and
//! `IN_LAYER_LAYOUT_UNIT` come from the north/south port preprocessor, the hierarchical port
//! constraint processor and the semi-interactive processor, none of which is ported. So no successor
//! constraint prevents a switch, and the layout units of any two nodes are both `null`, never
//! different. North/south port dummies and their `PORT_DUMMY` ports do not exist either (the sweep
//! rejects them), so `NorthSouthEdgeNeighbouringNodeCrossingsCounter` counts zero in both orders, as
//! the Java does for a layer without them.

use std::collections::HashMap;

use crate::core::data::enums::{NodeType, PortSide};
use crate::java::sort;
use crate::layered::LayoutError;
use crate::layered::p3order::counting::CrossingsCounter;
use crate::layered::p3order::{Layout, Net, in_north_south_east_west_order, start_index};

/// `GreedySwitchHeuristic` for `CrossMinType.TWO_SIDED_GREEDY_SWITCH`.
pub(super) struct GreedySwitchHeuristic {
    /// `portPositions`: the array the in-layer counters of every switch decider share. It keeps its
    /// values from one free layer to the next, as the Java array does.
    port_positions: Vec<i32>,
    /// `LNode.id` as `LayerSweepTypeDecider.initAtNodeLevel` sets it: each node's index in its layer
    /// when the sweep starts. The crossing matrix is indexed by it.
    node_ids: Vec<usize>,
}

impl GreedySwitchHeuristic {
    pub(super) fn new(net: &Net, layout: &Layout) -> Self {
        let mut node_ids = vec![0; net.graph().nodes.len()];
        for layer in &layout.order {
            for (index, &node) in layer.iter().enumerate() {
                node_ids[node] = index;
            }
        }
        GreedySwitchHeuristic {
            port_positions: vec![0; net.graph().ports.len()],
            node_ids,
        }
    }

    /// `minimizeCrossings(order, freeLayerIndex, forwardSweep, isFirstSweep)`: switches in the free
    /// layer until a pass over it switches nothing. Whether anything was switched.
    pub(super) fn minimize_crossings(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        free: usize,
    ) -> Result<bool, LayoutError> {
        let mut decider = self.set_up(net, layout, free)?;
        let mut improved = false;
        loop {
            let continue_switching =
                self.sweep_downward_in_layer(net, layout, &mut decider, free)?;
            improved |= continue_switching;
            if !continue_switching {
                return Ok(improved);
            }
        }
    }

    /// `setFirstLayerOrder`: one pass of switches over the layer the sweep starts from.
    pub(super) fn set_first_layer_order(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        forward: bool,
    ) -> Result<bool, LayoutError> {
        let start = start_index(forward, layout.order.len());
        let mut decider = self.set_up(net, layout, start)?;
        self.sweep_downward_in_layer(net, layout, &mut decider, start)
    }

    /// `setUp`: a new switch decider, with a new crossing matrix, for the free layer.
    fn set_up(
        &mut self,
        net: &Net,
        layout: &Layout,
        free: usize,
    ) -> Result<SwitchDecider, LayoutError> {
        SwitchDecider::new(net, layout, free, &mut self.port_positions)
    }

    /// `sweepDownwardInLayer`: each pair of neighbours from the top, switched if that improves.
    fn sweep_downward_in_layer(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        decider: &mut SwitchDecider,
        layer: usize,
    ) -> Result<bool, LayoutError> {
        let mut continue_switching = false;
        let length = layout.order[layer].len();
        for upper in 0..length.saturating_sub(1) {
            continue_switching |= self.switch_if_improves(net, layout, decider, layer, upper)?;
        }
        Ok(continue_switching)
    }

    /// `switchIfImproves` with `exchangeNodes`: the decider is told of the switch before the two
    /// nodes change places.
    fn switch_if_improves(
        &mut self,
        net: &Net,
        layout: &mut Layout,
        decider: &mut SwitchDecider,
        layer: usize,
        upper: usize,
    ) -> Result<bool, LayoutError> {
        let lower = upper + 1;
        if !decider.does_switch_reduce_crossings(
            net,
            layout,
            &mut self.port_positions,
            &self.node_ids,
            upper,
            lower,
        )? {
            return Ok(false);
        }
        let (upper_node, lower_node) = (layout.order[layer][upper], layout.order[layer][lower]);
        decider.notify_of_switch(
            net,
            layout,
            upper_node,
            lower_node,
            &mut self.port_positions,
        );
        layout.order[layer].swap(upper, lower);
        Ok(true)
    }
}

/// `SwitchDecider`, two-sided, for a graph without a parent.
struct SwitchDecider {
    /// `freeLayer`: the index of the layer whose nodes are switched. The Java holds the array,
    /// which the switches change; the layout's layer is that array.
    free: usize,
    /// `leftInLayerCounter`, initialized on the west side of the free layer.
    left_in_layer: CrossingsCounter,
    /// `rightInLayerCounter`, initialized on the east side.
    right_in_layer: CrossingsCounter,
    crossing_matrix: CrossingMatrixFiller,
}

impl SwitchDecider {
    /// `GreedySwitchHeuristic.getNewSwitchDecider`: the crossing matrix filler of the free layer, then
    /// the decider with its in-layer counters. `countCrossingsCausedByPortSwitch` needs a parent
    /// graph, so it is false here.
    fn new(
        net: &Net,
        layout: &Layout,
        free: usize,
        positions: &mut [i32],
    ) -> Result<Self, LayoutError> {
        let crossing_matrix = CrossingMatrixFiller::new(net, layout, free);
        let nodes = layout.order.get(free).ok_or_else(|| {
            LayoutError::Format("Greedy SwitchDecider: Free layer not in graph.".to_string())
        })?;
        let mut left_in_layer = CrossingsCounter::new(net);
        left_in_layer.init_port_positions_for_in_layer_crossings(
            net,
            layout,
            nodes,
            PortSide::WEST,
            positions,
        );
        let mut right_in_layer = CrossingsCounter::new(net);
        right_in_layer.init_port_positions_for_in_layer_crossings(
            net,
            layout,
            nodes,
            PortSide::EAST,
            positions,
        );
        Ok(SwitchDecider {
            free,
            left_in_layer,
            right_in_layer,
            crossing_matrix,
        })
    }

    /// `notifyOfSwitch`: the in-layer counters move the positions of the two nodes' ports.
    fn notify_of_switch(
        &self,
        net: &Net,
        layout: &Layout,
        upper: usize,
        lower: usize,
        positions: &mut [i32],
    ) {
        self.left_in_layer
            .switch_nodes(net, layout, upper, lower, PortSide::WEST, positions);
        self.right_in_layer
            .switch_nodes(net, layout, upper, lower, PortSide::EAST, positions);
    }

    /// `doesSwitchReduceCrossings`: the crossings of the pair as it is against the crossings with
    /// the two nodes switched, between the layers, in the layer on both sides, and at the north and
    /// south ports.
    fn does_switch_reduce_crossings(
        &mut self,
        net: &Net,
        layout: &Layout,
        positions: &mut [i32],
        node_ids: &[usize],
        upper_index: usize,
        lower_index: usize,
    ) -> Result<bool, LayoutError> {
        let upper = layout.order[self.free][upper_index];
        let lower = layout.order[self.free][lower_index];
        if constraints_prevent_switch(net, layout, upper, lower) {
            return Ok(false);
        }
        let left = self
            .left_in_layer
            .count_in_layer_crossings_between_nodes_in_both_orders(
                net,
                layout,
                upper,
                lower,
                PortSide::WEST,
                positions,
            )?;
        let right = self
            .right_in_layer
            .count_in_layer_crossings_between_nodes_in_both_orders(
                net,
                layout,
                upper,
                lower,
                PortSide::EAST,
                positions,
            )?;
        // `northSouthCounter.countCrossings`: zero in both orders without north/south port dummies.
        let (north_south_upper_lower, north_south_lower_upper) = (0, 0);
        let upper_lower = self
            .crossing_matrix
            .entry(net, layout, node_ids, upper, lower)?
            + left.0
            + right.0
            + north_south_upper_lower;
        let lower_upper = self
            .crossing_matrix
            .entry(net, layout, node_ids, lower, upper)?
            + left.1
            + right.1
            + north_south_lower_upper;
        Ok(upper_lower > lower_upper)
    }
}

/// `constraintsPreventSwitch`. Successor constraints and layout units do not exist in this model (see
/// the module comment), so `haveSuccessorConstraints` is false and `haveLayoutUnitConstraints` comes
/// down to its north and south edge test.
fn constraints_prevent_switch(net: &Net, layout: &Layout, upper: usize, lower: usize) -> bool {
    have_layout_unit_constraints(net, layout, upper, lower)
        || are_normal_and_north_south_port_dummy(net, upper, lower)
}

/// `haveLayoutUnitConstraints` with both layout units `null`: `areInDifferentLayoutUnits` is false,
/// so only an upper node with northern edges or a lower node with southern edges prevents the
/// switch, unless one of the two is a long edge dummy.
fn have_layout_unit_constraints(net: &Net, layout: &Layout, upper: usize, lower: usize) -> bool {
    let neither_is_long_edge_dummy =
        net.node_type(upper) != NodeType::LONG_EDGE && net.node_type(lower) != NodeType::LONG_EDGE;
    let upper_has_northern_edges = has_edges_on_side(net, layout, upper, PortSide::NORTH);
    let lower_has_southern_edges = has_edges_on_side(net, layout, lower, PortSide::SOUTH);
    neither_is_long_edge_dummy && (upper_has_northern_edges || lower_has_southern_edges)
}

/// `hasEdgesOnSide`: a port on `side` with an edge (no port has a `PORT_DUMMY` here).
fn has_edges_on_side(net: &Net, layout: &Layout, node: usize, side: PortSide) -> bool {
    net.side_ports(&layout.node_ports[node], side)
        .into_iter()
        .any(|port| net.degree(port) > 0)
}

/// `areNormalAndNorthSouthPortDummy`.
fn are_normal_and_north_south_port_dummy(net: &Net, upper: usize, lower: usize) -> bool {
    let north_south = |node| net.node_type(node) == NodeType::NORTH_SOUTH_PORT;
    let normal = |node| net.node_type(node) == NodeType::NORMAL;
    north_south(upper) && normal(lower) || north_south(lower) && normal(upper)
}

/// `CrossingMatrixFiller`, two-sided: the between-layer crossings of each pair of nodes of the free
/// layer in both orders, counted when first asked for.
struct CrossingMatrixFiller {
    /// `isCrossingMatrixFilled`, indexed by the node ids.
    filled: Vec<Vec<bool>>,
    /// `crossingMatrix`: `[upper][lower]` holds the crossings with `upper` above `lower`.
    matrix: Vec<Vec<i32>>,
    /// `inBetweenLayerCrossingCounter`.
    counter: BetweenLayerEdgeTwoNodeCrossingsCounter,
}

impl CrossingMatrixFiller {
    fn new(net: &Net, layout: &Layout, free: usize) -> Self {
        let length = layout.order.get(free).map_or(0, Vec::len);
        CrossingMatrixFiller {
            filled: vec![vec![false; length]; length],
            matrix: vec![vec![0; length]; length],
            counter: BetweenLayerEdgeTwoNodeCrossingsCounter::new(net, layout, free),
        }
    }

    /// `getCrossingMatrixEntry`: the crossings with `upper` above `lower`.
    fn entry(
        &mut self,
        net: &Net,
        layout: &Layout,
        node_ids: &[usize],
        upper: usize,
        lower: usize,
    ) -> Result<i32, LayoutError> {
        let (upper_id, lower_id) = (node_ids[upper], node_ids[lower]);
        let length = self.matrix.len();
        if upper_id >= length || lower_id >= length {
            return Err(LayoutError::Unsupported(
                "a node id outside its layer in the greedy switch".to_string(),
            ));
        }
        if !self.filled[upper_id][lower_id] {
            // `fillCrossingMatrix`, two-sided.
            self.counter
                .count_both_side_crossings(net, layout, upper, lower)?;
            self.matrix[upper_id][lower_id] = self.counter.upper_lower_crossings;
            self.matrix[lower_id][upper_id] = self.counter.lower_upper_crossings;
            self.filled[upper_id][lower_id] = true;
            self.filled[lower_id][upper_id] = true;
        }
        Ok(self.matrix[upper_id][lower_id])
    }
}

/// `BetweenLayerEdgeTwoNodeCrossingsCounter`: the crossings between the edges of two nodes of the
/// free layer and a neighbouring layer, in both orders of the two nodes.
struct BetweenLayerEdgeTwoNodeCrossingsCounter {
    upper_lower_crossings: i32,
    lower_upper_crossings: i32,
    free: usize,
    /// `portPositions`: the east ports of the western layer and the west ports of the eastern one,
    /// each layer numbered from zero, north to south. A `HashMap<LPort, Integer>` in Java.
    port_positions: HashMap<usize, i32>,
    /// `easternAdjacencies`.
    eastern: Adjacencies,
    /// `westernAdjacencies`.
    western: Adjacencies,
}

impl BetweenLayerEdgeTwoNodeCrossingsCounter {
    /// The constructor with `setPortPositionsForNeighbouringLayers`.
    fn new(net: &Net, layout: &Layout, free: usize) -> Self {
        let mut port_positions = HashMap::new();
        let mut set_positions_for_layer = |layer: usize, side: PortSide| {
            let mut port_id = 0;
            for &node in &layout.order[layer] {
                for port in in_north_south_east_west_order(net, &layout.node_ports[node], side) {
                    port_positions.insert(port, port_id);
                    port_id += 1;
                }
            }
        };
        // `freeLayerIsNotFirstLayer` and `freeLayerIsNotLastLayer`.
        if free > 0 {
            set_positions_for_layer(free - 1, PortSide::EAST);
        }
        if free + 1 < layout.order.len() {
            set_positions_for_layer(free + 1, PortSide::WEST);
        }
        BetweenLayerEdgeTwoNodeCrossingsCounter {
            upper_lower_crossings: 0,
            lower_upper_crossings: 0,
            free,
            port_positions,
            eastern: Adjacencies::default(),
            western: Adjacencies::default(),
        }
    }

    /// `countBothSideCrossings`: the western crossings, then the eastern ones.
    fn count_both_side_crossings(
        &mut self,
        net: &Net,
        layout: &Layout,
        upper: usize,
        lower: usize,
    ) -> Result<(), LayoutError> {
        self.upper_lower_crossings = 0;
        self.lower_upper_crossings = 0;
        if upper == lower {
            return Ok(());
        }
        self.add_crossings(net, layout, upper, lower, PortSide::WEST)?;
        self.add_crossings(net, layout, upper, lower, PortSide::EAST)
    }

    /// `addWesternCrossings` and `addEasternCrossings`: the adjacency lists of the two nodes on
    /// `side`, reset, and merged when neither is empty.
    fn add_crossings(
        &mut self,
        net: &Net,
        layout: &Layout,
        upper: usize,
        lower: usize,
        side: PortSide,
    ) -> Result<(), LayoutError> {
        let adjacencies = if side == PortSide::WEST {
            &mut self.western
        } else {
            &mut self.eastern
        };
        let upper_slot =
            adjacencies.adjacency_for(net, layout, self.free, upper, side, &self.port_positions)?;
        let lower_slot =
            adjacencies.adjacency_for(net, layout, self.free, lower, side, &self.port_positions)?;
        let mut upper_list = std::mem::take(&mut adjacencies.lists[upper_slot]);
        let mut lower_list = std::mem::take(&mut adjacencies.lists[lower_slot]);
        if upper_list.size() != 0 && lower_list.size() != 0 {
            let (upper_lower, lower_upper) =
                count_crossings_by_merging_adjacency_lists(&mut upper_list, &mut lower_list);
            self.upper_lower_crossings += upper_lower;
            self.lower_upper_crossings += lower_upper;
        }
        adjacencies.lists[upper_slot] = upper_list;
        adjacencies.lists[lower_slot] = lower_list;
        Ok(())
    }
}

/// `countCrossingsByMergingAdjacencyLists`: walks both lists from the top. An adjacency of one list
/// above the first of the other is crossed by all remaining adjacencies of the other list; at the
/// same position, each list's adjacencies below the other node's first one are counted.
fn count_crossings_by_merging_adjacency_lists(
    upper: &mut AdjacencyList,
    lower: &mut AdjacencyList,
) -> (i32, i32) {
    let mut upper_lower = 0;
    let mut lower_upper = 0;
    while !upper.is_empty() && !lower.is_empty() {
        // `isBelow(firstPort, secondPort)`: `firstPort > secondPort`.
        if upper.first() > lower.first() {
            upper_lower += upper.size();
            lower.remove_first();
        } else if lower.first() > upper.first() {
            lower_upper += lower.size();
            upper.remove_first();
        } else {
            upper_lower += upper.count_adjacencies_below_node_of_first_port();
            lower_upper += lower.count_adjacencies_below_node_of_first_port();
            upper.remove_first();
            lower.remove_first();
        }
    }
    (upper_lower, lower_upper)
}

/// `easternAdjacencies` or `westernAdjacencies`: a map from the nodes of the free layer to their
/// adjacency lists, empty until the first list is asked for.
#[derive(Default)]
struct Adjacencies {
    /// The slot of each node's list in `lists`.
    slots: HashMap<usize, usize>,
    lists: Vec<AdjacencyList>,
}

impl Adjacencies {
    /// `getAdjacencyFor`: the slot of the node's list, reset. The lists of all nodes of the free layer
    /// are made the first time.
    fn adjacency_for(
        &mut self,
        net: &Net,
        layout: &Layout,
        free: usize,
        node: usize,
        side: PortSide,
        positions: &HashMap<usize, i32>,
    ) -> Result<usize, LayoutError> {
        if self.slots.is_empty() {
            for &member in &layout.order[free] {
                self.slots.insert(member, self.lists.len());
                self.lists
                    .push(AdjacencyList::new(net, layout, member, side, positions)?);
            }
        }
        let slot = *self.slots.get(&node).ok_or_else(|| {
            LayoutError::Unsupported("a greedy switch on a node outside the free layer".to_string())
        })?;
        self.lists[slot].reset();
        Ok(slot)
    }
}

/// `AdjacencyList`: the positions of the ports a node's edges reach in the neighbouring layer on one
/// side, sorted, with equal neighbours merged into one adjacency. Removing does not delete entries:
/// `current_index`, `current_size` and the current cardinalities track the state, and `reset`
/// restores the full list.
#[derive(Default)]
struct AdjacencyList {
    adjacencies: Vec<Adjacency>,
    /// `size`: the number of edges in the list, which `reset` restores.
    full_size: i32,
    current_size: i32,
    current_index: usize,
}

/// `Adjacency`: a port position and how many edges reach it.
#[derive(Clone)]
struct Adjacency {
    position: i32,
    cardinality: i32,
    current_cardinality: i32,
}

impl AdjacencyList {
    /// The constructor with `getAdjacenciesSortedByPosition`.
    fn new(
        net: &Net,
        layout: &Layout,
        node: usize,
        side: PortSide,
        positions: &HashMap<usize, i32>,
    ) -> Result<Self, LayoutError> {
        let mut list = AdjacencyList::default();
        // `iterateTroughEdgesCollectingAdjacencies`.
        for port in in_north_south_east_west_order(net, &layout.node_ports[node], side) {
            // `getEdgesConnectedTo`: the incoming edges on the west, the outgoing ones on the east.
            let edges = if side == PortSide::WEST {
                net.incoming(port)
            } else {
                net.outgoing(port)
            };
            for &edge in edges {
                if !net.is_self_loop(edge) && !net.is_in_layer(edge) {
                    list.add_adjacency_of(net, edge, side, positions)?;
                    list.full_size += 1;
                    list.current_size += 1;
                }
            }
        }
        // `Collections.sort(adjacencyList)` by position.
        sort::sort_by(&mut list.adjacencies, |a, b| a.position.cmp(&b.position));
        Ok(list)
    }

    /// `addAdjacencyOf`: the position of the edge's other end, merged into the last adjacency when
    /// it has the same position.
    fn add_adjacency_of(
        &mut self,
        net: &Net,
        edge: usize,
        side: PortSide,
        positions: &HashMap<usize, i32>,
    ) -> Result<(), LayoutError> {
        // `adjacentPortOf`: the source on the west, the target on the east.
        let adjacent_port = if side == PortSide::WEST {
            net.source(edge)
        } else {
            net.target(edge)
        };
        // The Java unboxes `null` here (a `NullPointerException`) for a port the counter did not number.
        let position = *positions.get(&adjacent_port).ok_or_else(|| {
            LayoutError::Unsupported(
                "an edge in the greedy switch that does not reach the facing side of the \
                 neighbouring layer"
                    .to_string(),
            )
        })?;
        match self.adjacencies.last_mut() {
            Some(last) if last.position == position => {
                last.cardinality += 1;
                last.current_cardinality += 1;
            }
            _ => self.adjacencies.push(Adjacency {
                position,
                cardinality: 1,
                current_cardinality: 1,
            }),
        }
        Ok(())
    }

    /// `reset`: the full list again. Only the first adjacency is reset here; the others are reset when
    /// they become current.
    fn reset(&mut self) {
        self.current_index = 0;
        self.current_size = self.full_size;
        if !self.is_empty() {
            self.reset_current_adjacency();
        }
    }

    /// `Adjacency.reset` of the current adjacency.
    fn reset_current_adjacency(&mut self) {
        let current = &mut self.adjacencies[self.current_index];
        current.current_cardinality = current.cardinality;
    }

    /// `countAdjacenciesBelowNodeOfFirstPort`.
    fn count_adjacencies_below_node_of_first_port(&self) -> i32 {
        self.current_size - self.adjacencies[self.current_index].current_cardinality
    }

    /// `removeFirst`.
    fn remove_first(&mut self) {
        if self.is_empty() {
            return;
        }
        if self.adjacencies[self.current_index].current_cardinality == 1 {
            // `incrementCurrentIndex`: the next adjacency is reset for reuse.
            self.current_index += 1;
            if self.current_index < self.adjacencies.len() {
                self.reset_current_adjacency();
            }
        } else {
            self.adjacencies[self.current_index].current_cardinality -= 1;
        }
        self.current_size -= 1;
    }

    fn is_empty(&self) -> bool {
        self.current_size == 0
    }

    /// `first`: the position of the current adjacency.
    fn first(&self) -> i32 {
        self.adjacencies[self.current_index].position
    }

    /// `size`: the number of edges left in the list.
    fn size(&self) -> i32 {
        self.current_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An adjacency list of the given positions, in that order before the sort.
    fn list(positions: &[i32]) -> AdjacencyList {
        let mut list = AdjacencyList::default();
        for &position in positions {
            match list.adjacencies.last_mut() {
                Some(last) if last.position == position => {
                    last.cardinality += 1;
                    last.current_cardinality += 1;
                }
                _ => list.adjacencies.push(Adjacency {
                    position,
                    cardinality: 1,
                    current_cardinality: 1,
                }),
            }
            list.full_size += 1;
            list.current_size += 1;
        }
        sort::sort_by(&mut list.adjacencies, |a, b| a.position.cmp(&b.position));
        list
    }

    #[test]
    fn the_javadoc_example_crosses_twice_in_the_given_order() {
        // A reaches p1 twice, B reaches p0: B's edge crosses both of A's; switched, nothing crosses.
        let (mut a, mut b) = (list(&[1, 1]), list(&[0]));
        assert_eq!(
            count_crossings_by_merging_adjacency_lists(&mut a, &mut b),
            (2, 0)
        );
    }

    #[test]
    fn shared_neighbours_count_the_adjacencies_below_the_other_node() {
        // Both nodes reach p0 and p1: one crossing in either order.
        let (mut a, mut b) = (list(&[0, 1]), list(&[0, 1]));
        assert_eq!(
            count_crossings_by_merging_adjacency_lists(&mut a, &mut b),
            (1, 1)
        );
        // A reset list counts the same again.
        a.reset();
        b.reset();
        assert_eq!(
            count_crossings_by_merging_adjacency_lists(&mut a, &mut b),
            (1, 1)
        );
    }
}
