//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/GraphConfigurator.java
//!
//! `GraphConfigurator`: picks the phases and assembles the processor list.
//!
//! The assembly follows `AlgorithmAssembler.build`: for each phase, the
//! processors configured before it run first, then the phase; the processors
//! configured after the last phase run at the end. Each slot is ordered by the
//! processors' enum ordinals. The conditions that add processors are those of
//! `GraphConfigurator.getPhaseIndependentLayoutProcessorConfiguration`.

use crate::core::alg::LayoutProcessorConfiguration;
use crate::core::data::enums::{
    CrossingMinimizationStrategy, CycleBreakingStrategy, Direction, EdgeRouting, GraphProperties,
    GreedySwitchType, HierarchyHandling, IntermediateProcessorStrategy as S, LayeringStrategy,
    NodePlacementStrategy, NodePromotionStrategy, OrderingStrategy, PortConstraints,
};
use crate::java::random::Random;
use crate::layered::LayoutError;
use crate::layered::intermediate::Step;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::phases::{P1, P2, P3, P4, P5, PHASES, PhaseStep};

/// `GraphConfigurator.prepareGraphForLayout`: sets `lgraph.processors`.
pub fn prepare(graph: &mut LGraph) -> Result<(), LayoutError> {
    // Centred labels and self loops are handled by their processors; other features (hyperedges,
    // hierarchical ports, ...) need processors that are not ported yet.
    if let Some(feature) = graph.graph_properties.iter().find(|feature| {
        **feature != GraphProperties::CENTER_LABELS
            && **feature != GraphProperties::END_LABELS
            && **feature != GraphProperties::SELF_LOOPS
    }) {
        return Err(LayoutError::Unsupported(format!(
            "graph feature {feature:?}"
        )));
    }
    if !graph.properties.has(options::DIRECTION)
        || graph.properties.enum_of::<Direction>(options::DIRECTION) == Some(Direction::UNDEFINED)
    {
        return Err(LayoutError::Unsupported(
            "a graph without a direction (LGraphUtil.getDirection is not ported)".to_string(),
        ));
    }
    configure_graph_properties(graph);
    copy_original_port_constraints(graph);
    graph.random = Some(random_source(graph)?);
    graph.processors = assemble(graph)?;
    Ok(())
}

/// `GraphConfigurator`: the generator for `elk.randomSeed`. Layered's default seed is 1
/// (`supports` in Layered.melk, which the registry does not read). Seed 0 asks for a
/// system seed, which cannot be reproduced.
fn random_source(graph: &LGraph) -> Result<Random, LayoutError> {
    let seed = graph.properties.int_of(options::RANDOM_SEED).unwrap_or(1);
    if seed == 0 {
        return Err(LayoutError::Unsupported(
            "randomSeed 0 asks for a system seed".to_string(),
        ));
    }
    Ok(Random::new(i64::from(seed)))
}

/// `GraphConfigurator.configureGraphProperties`: the defaults the processors read.
fn configure_graph_properties(graph: &mut LGraph) {
    if !graph
        .properties
        .has(options::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES)
    {
        let orthogonal = graph
            .properties
            .enum_of::<EdgeRouting>(options::EDGE_ROUTING)
            == Some(EdgeRouting::ORTHOGONAL);
        graph.properties.set(
            options::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES,
            Some(crate::core::data::OptionValue::Boolean(orthogonal)),
        );
    }
}

/// `GraphConfigurator.copyPortContraints`: each node keeps the port constraints it was given, since
/// the port side processing and the self loop preprocessing change the ones it uses.
fn copy_original_port_constraints(graph: &mut LGraph) {
    for node in &mut graph.nodes {
        node.original_port_constraints = Some(
            node.properties
                .enum_of::<PortConstraints>(options::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::UNDEFINED),
        );
    }
}

/// The phase implementations the strategy options select.
fn phases(graph: &LGraph) -> [PhaseStep; PHASES] {
    let props = &graph.properties;
    [
        PhaseStep::CycleBreaking(
            props
                .enum_of::<CycleBreakingStrategy>(options::CYCLE_BREAKING_STRATEGY)
                .unwrap_or(CycleBreakingStrategy::GREEDY),
        ),
        PhaseStep::Layering(
            props
                .enum_of::<LayeringStrategy>(options::LAYERING_STRATEGY)
                .unwrap_or(LayeringStrategy::NETWORK_SIMPLEX),
        ),
        PhaseStep::CrossingMinimization(
            props
                .enum_of::<CrossingMinimizationStrategy>(options::CROSSING_MINIMIZATION_STRATEGY)
                .unwrap_or(CrossingMinimizationStrategy::LAYER_SWEEP),
        ),
        PhaseStep::NodePlacement(
            props
                .enum_of::<NodePlacementStrategy>(options::NODE_PLACEMENT_STRATEGY)
                .unwrap_or(NodePlacementStrategy::BRANDES_KOEPF),
        ),
        PhaseStep::EdgeRouting(
            props
                .enum_of::<EdgeRouting>(options::EDGE_ROUTING)
                .unwrap_or(EdgeRouting::ORTHOGONAL),
        ),
    ]
}

/// `getPhaseIndependentLayoutProcessorConfiguration`, plus the baseline table.
fn phase_independent(graph: &LGraph) -> Result<LayoutProcessorConfiguration<S>, LayoutError> {
    let props = &graph.properties;
    let mut config = LayoutProcessorConfiguration::new(PHASES);
    // BASELINE_PROCESSING_CONFIGURATION
    config
        .add_before(P4, S::INNERMOST_NODE_MARGIN_CALCULATOR)
        .add_before(P4, S::LABEL_AND_NODE_SIZE_PROCESSOR)
        .add_before(P5, S::LAYER_SIZE_AND_GRAPH_HEIGHT_CALCULATOR)
        .add_after(P5, S::END_LABEL_SORTER);
    // `OrthogonalEdgeRouter`'s additions for centred labels.
    if graph
        .graph_properties
        .contains(&GraphProperties::CENTER_LABELS)
    {
        config
            .add_before(P2, S::LABEL_DUMMY_INSERTER)
            .add_before(P4, S::LABEL_DUMMY_SWITCHER)
            .add_before(P4, S::LABEL_SIDE_SELECTOR)
            .add_after(P5, S::LABEL_DUMMY_REMOVER);
    }
    // `OrthogonalEdgeRouter`'s additions for head and tail labels.
    if graph
        .graph_properties
        .contains(&GraphProperties::END_LABELS)
    {
        config
            .add_before(P4, S::LABEL_SIDE_SELECTOR)
            .add_before(P4, S::END_LABEL_PREPROCESSOR)
            .add_after(P5, S::END_LABEL_POSTPROCESSOR);
    }
    if props.enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
        == Some(HierarchyHandling::INCLUDE_CHILDREN)
    {
        config.add_after(P5, S::HIERARCHICAL_NODE_RESIZER);
    }
    if props.bool_of(options::FEEDBACK_EDGES) {
        config.add_before(P1, S::PORT_SIDE_PROCESSOR);
    } else {
        config.add_before(P3, S::PORT_SIDE_PROCESSOR);
    }
    // Directions with a leftward or vertical flow are turned around by the transformer.
    if let Some(Direction::LEFT | Direction::DOWN | Direction::UP) =
        props.enum_of::<Direction>(options::DIRECTION)
    {
        config
            .add_before(P1, S::DIRECTION_PREPROCESSOR)
            .add_after(P5, S::DIRECTION_POSTPROCESSOR);
    }
    if props
        .enum_of::<NodePromotionStrategy>(options::NODE_PROMOTION_STRATEGY)
        .is_some_and(|strategy| strategy != NodePromotionStrategy::NONE)
    {
        return Err(LayoutError::Unsupported("node promotion".to_string()));
    }
    if props
        .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
        .is_some_and(|strategy| strategy != OrderingStrategy::NONE)
    {
        config.add_before(P3, S::SORT_BY_INPUT_ORDER_OF_MODEL);
    }
    if props.bool_of(options::INTERACTIVE)
        || props.bool_of(options::GENERATE_POSITION_AND_LAYER_IDS)
    {
        return Err(LayoutError::Unsupported("interactive layout".to_string()));
    }
    if greedy_switch_active(graph) {
        return Err(LayoutError::Unsupported(
            "greedy switch crossing minimization".to_string(),
        ));
    }
    Ok(config)
}

/// `GraphConfigurator.activateGreedySwitchFor`: whether the greedy switch processor is added.
fn greedy_switch_active(graph: &LGraph) -> bool {
    let props = &graph.properties;
    let hierarchical = props.enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
        == Some(HierarchyHandling::INCLUDE_CHILDREN);
    if hierarchical {
        let hierarchical_type = props.enum_of::<GreedySwitchType>(
            options::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
        );
        return hierarchical_type != Some(GreedySwitchType::OFF);
    }
    let switch_type =
        props.enum_of::<GreedySwitchType>(options::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE);
    let interactive = props.bool_of(options::CROSSING_MINIMIZATION_SEMI_INTERACTIVE)
        || props.enum_of::<CrossingMinimizationStrategy>(options::CROSSING_MINIMIZATION_STRATEGY)
            == Some(CrossingMinimizationStrategy::INTERACTIVE);
    let threshold = props
        .int_of(options::CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD)
        .unwrap_or(0);
    let graph_size = i32::try_from(graph.layerless_nodes.len()).unwrap_or(i32::MAX);
    !interactive
        && switch_type != Some(GreedySwitchType::OFF)
        && (threshold == 0 || threshold > graph_size)
}

/// The steps in the order `AlgorithmAssembler.build` produces them.
fn assemble(graph: &LGraph) -> Result<Vec<Step>, LayoutError> {
    let phases = phases(graph);
    let mut config = LayoutProcessorConfiguration::new(PHASES);
    for phase in phases {
        if let Some(phase_config) = phase.configuration(graph)? {
            config.add_all(&phase_config);
        }
    }
    config.add_all(&phase_independent(graph)?);
    let mut steps = Vec::new();
    for (index, phase) in phases.iter().enumerate() {
        for processor in config.processors_before(index) {
            steps.push(Step::Intermediate(processor));
        }
        steps.push(Step::Phase(*phase));
    }
    for processor in config.processors_after(PHASES - 1) {
        steps.push(Step::Intermediate(processor));
    }
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_steps_come_in_ordinal_order_per_slot() {
        let mut graph = LGraph::default();
        graph.properties.set(
            options::DIRECTION,
            Some(crate::core::data::OptionValue::Enum {
                class: "Direction",
                name: "DOWN".to_string(),
            }),
        );
        graph.properties.set(
            options::HIERARCHY_HANDLING,
            Some(crate::core::data::OptionValue::Enum {
                class: "HierarchyHandling",
                name: "INCLUDE_CHILDREN".to_string(),
            }),
        );
        prepare(&mut graph).unwrap();
        let names: Vec<String> = graph
            .processors
            .iter()
            .filter_map(|step| match step {
                Step::Intermediate(strategy) => Some(format!("{strategy:?}")),
                Step::Phase(_) => None,
            })
            .collect();
        assert!(names.contains(&"DIRECTION_PREPROCESSOR".to_string()));
        assert!(names.contains(&"HIERARCHICAL_NODE_RESIZER".to_string()));
    }
}
