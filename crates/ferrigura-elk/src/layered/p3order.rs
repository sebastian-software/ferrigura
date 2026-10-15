//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/LayerSweepCrossingMinimizer.java
//!   with BarycenterHeuristic.java, GraphInfoHolder.java and LayerSweepTypeDecider.java
//!
//! The layer sweep crossing minimizer with the barycenter heuristic, for the root graph
//! without edges, ports, constraints or nested graphs. Such a graph has no crossings and
//! no barycenters: a sweep only orders the nodes of its layer, by random barycenters, or
//! keeps their order when the model order decides (`FIRST_TRY_WITH_INITIAL_ORDER`). The
//! barycenter from connected nodes, the crossing counter and the port distributors are
//! not ported yet; the importer rejects the graphs that need them.

use std::cmp::Ordering;

use crate::core::data::enums::{HierarchyHandling, OrderingStrategy};
use crate::java::random::Random;
use crate::java::sort;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// The thoroughness `LayeredOptions.THOROUGHNESS` has by default.
const DEFAULT_THOROUGHNESS: i32 = 7;

/// `LayerSweepCrossingMinimizer.process` for `CrossMinType.BARYCENTER`, the layer sweep of the
/// default crossing minimization. The task's `begin` and `done` are the phase's.
pub fn sweep(graph: &mut LGraph) -> Result<(), LayoutError> {
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
    if graph.properties.bool_of(options::FORCE_NODE_MODEL_ORDER) {
        return Err(LayoutError::Unsupported(
            "crossingMinimization.forceNodeModelOrder (ModelOrderBarycenterHeuristic)".to_string(),
        ));
    }
    if graph
        .properties
        .double_of(options::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE)
        .unwrap_or(0.0)
        != 0.0
        || graph
            .properties
            .double_of(options::CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE)
            .unwrap_or(0.0)
            != 0.0
    {
        return Err(LayoutError::Unsupported(
            "the crossing counter with model order influence".to_string(),
        ));
    }
    let thoroughness = graph
        .properties
        .int_of(options::THOROUGHNESS)
        .unwrap_or(DEFAULT_THOROUGHNESS);
    let mut random = graph
        .random
        .take()
        .ok_or_else(|| LayoutError::Unsupported("a graph without a random source".to_string()))?;
    // `initialize`: the seed the attempts restart from, drawn first.
    let seed = random.next_long();
    // `ISweepPortDistributor.create` draws once per graph it creates a distributor for.
    let _ = random.next_boolean();
    let consider_model_order = graph
        .properties
        .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
        .is_some_and(|strategy| strategy != OrderingStrategy::NONE);
    let mut state = Sweep {
        order: graph.layers.clone(),
        random,
        first_try: false,
        second_try: false,
        consider_model_order,
        changed: false,
        currently_best: None,
        best: None,
    };
    state.compare_randomized_layouts(seed, thoroughness);
    // `transferNodeAndPortOrdersToGraph`: the best order saved, when a sweep has saved one.
    if let Some(best) = state.best.take() {
        graph.layers = best;
    }
    graph.random = Some(state.random);
    Ok(())
}

/// The state of one sweep: the working node order and the flags the attempts share.
struct Sweep {
    /// `currentNodeOrder`: the layers as the sweep reorders them, node indices per layer.
    order: Vec<Vec<usize>>,
    random: Random,
    /// `InternalProperties.FIRST_TRY_WITH_INITIAL_ORDER`.
    first_try: bool,
    /// `InternalProperties.SECOND_TRY_WITH_INITIAL_ORDER`.
    second_try: bool,
    /// `CONSIDER_MODEL_ORDER_STRATEGY != NONE`.
    consider_model_order: bool,
    /// Whether a sweep has changed the order, as `graphsWhoseNodeOrderChanged` records for the root.
    changed: bool,
    /// `currentlyBestNodeAndPortOrder`: the order after the latest sweep of an attempt.
    currently_best: Option<Vec<Vec<usize>>>,
    /// `bestNodeNPortOrder`: the order saved when an attempt did at least as well as the earlier ones.
    best: Option<Vec<Vec<usize>>>,
}

impl Sweep {
    /// `compareDifferentRandomizedLayouts` for the crossing counter without model order influence.
    fn compare_randomized_layouts(&mut self, seed: i64, thoroughness: i32) {
        self.random.set_seed(seed);
        self.changed = false;
        if self.consider_model_order {
            self.first_try = true;
        }
        let mut best_crossings = i32::MAX;
        for _ in 0..thoroughness {
            let crossings = self.minimize_crossings_with_counter();
            if crossings < best_crossings {
                best_crossings = crossings;
                self.save_all_node_orders();
                if best_crossings == 0 {
                    break;
                }
            }
        }
    }

    /// `minimizeCrossingsWithCounter`.
    fn minimize_crossings_with_counter(&mut self) -> i32 {
        let mut forward = self.random.next_boolean();
        let initial_crossings = self.count_crossings();
        if initial_crossings == 0 && self.first_try {
            return 0;
        }
        if (!self.first_try && !self.second_try) || !self.consider_model_order {
            self.set_first_layer_order(forward);
        } else {
            forward = self.first_try;
        }
        self.sweep_reducing_crossings(forward, true);
        if self.second_try {
            self.second_try = false;
        }
        if self.first_try {
            self.first_try = false;
            self.second_try = true;
        }
        let mut crossings = self.count_crossings();
        loop {
            self.set_currently_best_node_orders();
            if crossings == 0 {
                return 0;
            }
            forward = !forward;
            let old_crossings = crossings;
            self.sweep_reducing_crossings(forward, false);
            crossings = self.count_crossings();
            if old_crossings <= crossings {
                return old_crossings;
            }
        }
    }

    /// `countCurrentNumberOfCrossings`. The graph has no edges, so no two of them cross.
    fn count_crossings(&self) -> i32 {
        0
    }

    /// `setFirstLayerOrder`: the first layer of the sweep, in random order.
    fn set_first_layer_order(&mut self, forward: bool) {
        let start = if forward {
            0
        } else {
            self.order.len().saturating_sub(1)
        };
        self.order_layer(start, false, true);
    }

    /// `sweepReducingCrossings`: sweeps over the layers after the first, which is fixed.
    fn sweep_reducing_crossings(&mut self, forward: bool, first_sweep: bool) {
        self.changed = true;
        let length = self.order.len() as isize;
        let is_first_sweep = first_sweep && !self.first_try && !self.second_try;
        let (step, mut index) = if forward { (1, 1) } else { (-1, length - 2) };
        while (forward && index < length) || (!forward && index >= 0) {
            // `BarycenterHeuristic.minimizeCrossings`: the free layer, after the first sweep it is pre-ordered.
            self.order_layer(index as usize, !is_first_sweep, false);
            index += step;
        }
    }

    /// `BarycenterHeuristic.minimizeCrossings` for one layer: random barycenters when
    /// `randomize`, otherwise the barycenters of the connected nodes, which here are unknown.
    fn order_layer(&mut self, index: usize, pre_ordered: bool, randomize: bool) {
        let members = self.order[index].clone();
        let barycenters: Vec<Option<f64>> = if randomize {
            members
                .iter()
                .map(|_| Some(self.random.next_double()))
                .collect()
        } else {
            let mut unknown = vec![None; members.len()];
            fill_unknown_barycenters(&mut unknown, pre_ordered, &mut self.random);
            unknown
        };
        let mut ranked: Vec<(usize, Option<f64>)> = members.into_iter().zip(barycenters).collect();
        if ranked.len() > 1 {
            sort::sort_by(&mut ranked, |a, b| compare_barycenters(a.1, b.1));
        }
        self.order[index] = ranked.into_iter().map(|(node, _)| node).collect();
    }

    /// `setCurrentlyBestNodeOrders`.
    fn set_currently_best_node_orders(&mut self) {
        if self.changed {
            self.currently_best = Some(self.order.clone());
        }
    }

    /// `saveAllNodeOrdersOfChangedGraphs`.
    fn save_all_node_orders(&mut self) {
        if self.changed {
            self.best = self.currently_best.clone();
        }
    }
}

/// `BarycenterHeuristic.fillInUnknownBarycenters`: the nodes without a barycenter get one, in
/// list order, from their neighbours when the order is pre-set, and at random otherwise.
fn fill_unknown_barycenters(values: &mut [Option<f64>], pre_ordered: bool, random: &mut Random) {
    if pre_ordered {
        let mut last = -1.0;
        for index in 0..values.len() {
            let value = match values[index] {
                Some(value) => value,
                None => {
                    let next = values[index + 1..]
                        .iter()
                        .find_map(|known| *known)
                        .unwrap_or(last + 1.0);
                    let value = (last + next) / 2.0;
                    values[index] = Some(value);
                    value
                }
            };
            last = value;
        }
    } else {
        let mut max_barycenter = 0.0_f64;
        for value in values.iter().flatten() {
            max_barycenter = max_barycenter.max(*value);
        }
        max_barycenter += 2.0;
        for value in values.iter_mut().filter(|value| value.is_none()) {
            *value = Some(random.next_float() * max_barycenter - 1.0);
        }
    }
}

/// `BarycenterHeuristic.barycenterStateComparator`: the known barycenters first, in order.
fn compare_barycenters(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::layered::lgraph::LNode;

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
}
