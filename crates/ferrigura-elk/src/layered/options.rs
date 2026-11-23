//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/Layered.melk (generated LayeredOptions)
//!
//! Option ids that the layered algorithm reads, `LayeredOptions` in the Java sources.
//!
//! Every id here is resolved against the generated registry when it is compiled,
//! so a misspelled id fails the build instead of reading a default.

use crate::core::data::OptionId;

pub const CYCLE_BREAKING_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.cycleBreaking.strategy");
pub const LAYERING_STRATEGY: OptionId = OptionId::of("org.eclipse.elk.layered.layering.strategy");
pub const CROSSING_MINIMIZATION_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.strategy");
pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.greedySwitch.type");
pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.greedySwitch.activationThreshold");
pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.greedySwitchHierarchical.type");
pub const CROSSING_MINIMIZATION_SEMI_INTERACTIVE: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.semiInteractive");
pub const NODE_PLACEMENT_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.nodePlacement.strategy");
pub const NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES: OptionId =
    OptionId::of("org.eclipse.elk.layered.nodePlacement.favorStraightEdges");
pub const NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY: OptionId =
    OptionId::of("org.eclipse.elk.layered.nodePlacement.networkSimplex.nodeFlexibility");
pub const NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT: OptionId =
    OptionId::of("org.eclipse.elk.layered.nodePlacement.networkSimplex.nodeFlexibility.default");
pub const EDGE_ROUTING: OptionId = OptionId::of("org.eclipse.elk.edgeRouting");
pub const CONSIDER_MODEL_ORDER_NO_MODEL_ORDER: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.noModelOrder");
pub const CONSIDER_MODEL_ORDER_COMPONENTS: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.components");
pub const DIRECTION_CONGRUENCY: OptionId =
    OptionId::of("org.eclipse.elk.layered.directionCongruency");
pub const NODE_PLACEMENT_BK_FIXED_ALIGNMENT: OptionId =
    OptionId::of("org.eclipse.elk.layered.nodePlacement.bk.fixedAlignment");
pub const NODE_PLACEMENT_BK_EDGE_STRAIGHTENING: OptionId =
    OptionId::of("org.eclipse.elk.layered.nodePlacement.bk.edgeStraightening");
pub const FEEDBACK_EDGES: OptionId = OptionId::of("org.eclipse.elk.layered.feedbackEdges");
pub const CONSIDER_MODEL_ORDER_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.strategy");
pub const CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.portModelOrder");
pub const CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.longEdgeStrategy");
pub const NODE_PROMOTION_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.layering.nodePromotion.strategy");
pub const POST_COMPACTION_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.compaction.postCompaction.strategy");
pub const WRAPPING_STRATEGY: OptionId = OptionId::of("org.eclipse.elk.layered.wrapping.strategy");
pub const HIGH_DEGREE_NODES_TREATMENT: OptionId =
    OptionId::of("org.eclipse.elk.layered.highDegreeNodes.treatment");
pub const INTERACTIVE: OptionId = OptionId::of("org.eclipse.elk.interactive");
pub const GENERATE_POSITION_AND_LAYER_IDS: OptionId =
    OptionId::of("org.eclipse.elk.layered.generatePositionAndLayerIds");
pub const HIERARCHY_HANDLING: OptionId = OptionId::of("org.eclipse.elk.hierarchyHandling");
pub const DIRECTION: OptionId = OptionId::of("org.eclipse.elk.direction");
pub const RANDOM_SEED: OptionId = OptionId::of("org.eclipse.elk.randomSeed");
pub const THOROUGHNESS: OptionId = OptionId::of("org.eclipse.elk.layered.thoroughness");
pub const SEPARATE_CONNECTED_COMPONENTS: OptionId =
    OptionId::of("org.eclipse.elk.separateConnectedComponents");
pub const ALGORITHM: OptionId = OptionId::of("org.eclipse.elk.algorithm");
pub const FORCE_NODE_MODEL_ORDER: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.forceNodeModelOrder");
pub const ALIGNMENT: OptionId = OptionId::of("org.eclipse.elk.alignment");
pub const PADDING: OptionId = OptionId::of("org.eclipse.elk.padding");
pub const NODE_SIZE_CONSTRAINTS: OptionId = OptionId::of("org.eclipse.elk.nodeSize.constraints");
pub const NODE_SIZE_OPTIONS: OptionId = OptionId::of("org.eclipse.elk.nodeSize.options");
pub const NODE_SIZE_FIXED_GRAPH_SIZE: OptionId =
    OptionId::of("org.eclipse.elk.nodeSize.fixedGraphSize");
pub const NODE_LABELS_PLACEMENT: OptionId = OptionId::of("org.eclipse.elk.nodeLabels.placement");
pub const NODE_LABELS_PADDING: OptionId = OptionId::of("org.eclipse.elk.nodeLabels.padding");
pub const NODE_SIZE_MINIMUM: OptionId = OptionId::of("org.eclipse.elk.nodeSize.minimum");
pub const CONTENT_ALIGNMENT: OptionId = OptionId::of("org.eclipse.elk.contentAlignment");
pub const NO_LAYOUT: OptionId = OptionId::of("org.eclipse.elk.noLayout");
pub const PORT_CONSTRAINTS: OptionId = OptionId::of("org.eclipse.elk.portConstraints");
pub const SPACING_PORTS_SURROUNDING: OptionId =
    OptionId::of("org.eclipse.elk.spacing.portsSurrounding");
pub const SPACING_EDGE_LABEL: OptionId = OptionId::of("org.eclipse.elk.spacing.edgeLabel");
pub const SPACING_LABEL_LABEL: OptionId = OptionId::of("org.eclipse.elk.spacing.labelLabel");
pub const EDGE_LABELS_PLACEMENT: OptionId = OptionId::of("org.eclipse.elk.edgeLabels.placement");
pub const EDGE_LABELS_SIDE_SELECTION: OptionId =
    OptionId::of("org.eclipse.elk.layered.edgeLabels.sideSelection");
pub const EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY: OptionId =
    OptionId::of("org.eclipse.elk.layered.edgeLabels.centerLabelPlacementStrategy");
pub const EDGE_LABELS_INLINE: OptionId = OptionId::of("org.eclipse.elk.edgeLabels.inline");
pub const SPACING_EDGE_NODE_BETWEEN_LAYERS: OptionId =
    OptionId::of("org.eclipse.elk.layered.spacing.edgeNodeBetweenLayers");
pub const SPACING_NODE_NODE_BETWEEN_LAYERS: OptionId =
    OptionId::of("org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers");
pub const UNNECESSARY_BENDPOINTS: OptionId =
    OptionId::of("org.eclipse.elk.layered.unnecessaryBendpoints");
pub const PRIORITY_SHORTNESS: OptionId = OptionId::of("org.eclipse.elk.layered.priority.shortness");
pub const PRIORITY_STRAIGHTNESS: OptionId =
    OptionId::of("org.eclipse.elk.layered.priority.straightness");
pub const PRIORITY_DIRECTION: OptionId = OptionId::of("org.eclipse.elk.layered.priority.direction");
pub const LAYER_CONSTRAINT: OptionId =
    OptionId::of("org.eclipse.elk.layered.layering.layerConstraint");
pub const CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.crossingCounterNodeInfluence");
pub const CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE: OptionId =
    OptionId::of("org.eclipse.elk.layered.considerModelOrder.crossingCounterPortInfluence");
pub const EDGE_ROUTING_SELF_LOOP_DISTRIBUTION: OptionId =
    OptionId::of("org.eclipse.elk.layered.edgeRouting.selfLoopDistribution");
pub const EDGE_ROUTING_SELF_LOOP_ORDERING: OptionId =
    OptionId::of("org.eclipse.elk.layered.edgeRouting.selfLoopOrdering");
pub const SPACING_NODE_SELF_LOOP: OptionId = OptionId::of("org.eclipse.elk.spacing.nodeSelfLoop");
pub const INSIDE_SELF_LOOPS_ACTIVATE: OptionId =
    OptionId::of("org.eclipse.elk.insideSelfLoops.activate");
pub const INSIDE_SELF_LOOPS_YO: OptionId = OptionId::of("org.eclipse.elk.insideSelfLoops.yo");
pub const CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS: OptionId =
    OptionId::of("org.eclipse.elk.layered.crossingMinimization.hierarchicalSweepiness");
pub const MERGE_HIERARCHY_EDGES: OptionId =
    OptionId::of("org.eclipse.elk.layered.mergeHierarchyEdges");
pub const PORT_LABELS_PLACEMENT: OptionId = OptionId::of("org.eclipse.elk.portLabels.placement");
pub const PORT_BORDER_OFFSET: OptionId = OptionId::of("org.eclipse.elk.port.borderOffset");
pub const PORT_SIDE: OptionId = OptionId::of("org.eclipse.elk.port.side");
pub const PORT_ANCHOR: OptionId = OptionId::of("org.eclipse.elk.port.anchor");
pub const ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES: OptionId =
    OptionId::of("org.eclipse.elk.layered.allowNonFlowPortsToSwitchSides");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::option_by_id;

    #[test]
    fn every_id_is_registered() {
        for id in [
            CYCLE_BREAKING_STRATEGY,
            LAYERING_STRATEGY,
            CROSSING_MINIMIZATION_STRATEGY,
            CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
            CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD,
            CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
            CROSSING_MINIMIZATION_SEMI_INTERACTIVE,
            NODE_PLACEMENT_STRATEGY,
            NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES,
            EDGE_ROUTING,
            DIRECTION_CONGRUENCY,
            CONSIDER_MODEL_ORDER_NO_MODEL_ORDER,
            CONSIDER_MODEL_ORDER_COMPONENTS,
            NODE_PLACEMENT_BK_FIXED_ALIGNMENT,
            NODE_PLACEMENT_BK_EDGE_STRAIGHTENING,
            FEEDBACK_EDGES,
            CONSIDER_MODEL_ORDER_STRATEGY,
            CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER,
            CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY,
            NODE_PROMOTION_STRATEGY,
            POST_COMPACTION_STRATEGY,
            WRAPPING_STRATEGY,
            HIGH_DEGREE_NODES_TREATMENT,
            INTERACTIVE,
            GENERATE_POSITION_AND_LAYER_IDS,
            HIERARCHY_HANDLING,
            DIRECTION,
            RANDOM_SEED,
            THOROUGHNESS,
            SEPARATE_CONNECTED_COMPONENTS,
            ALGORITHM,
            FORCE_NODE_MODEL_ORDER,
            ALIGNMENT,
            PADDING,
            NODE_SIZE_CONSTRAINTS,
            NODE_SIZE_OPTIONS,
            NODE_SIZE_FIXED_GRAPH_SIZE,
            CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE,
            CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE,
            NO_LAYOUT,
            EDGE_LABELS_PLACEMENT,
            PORT_CONSTRAINTS,
            SPACING_EDGE_LABEL,
            SPACING_LABEL_LABEL,
            EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
            EDGE_LABELS_INLINE,
            SPACING_EDGE_NODE_BETWEEN_LAYERS,
            SPACING_NODE_NODE_BETWEEN_LAYERS,
            PRIORITY_SHORTNESS,
            PRIORITY_STRAIGHTNESS,
            PRIORITY_DIRECTION,
            INSIDE_SELF_LOOPS_ACTIVATE,
            INSIDE_SELF_LOOPS_YO,
            CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
            MERGE_HIERARCHY_EDGES,
            PORT_LABELS_PLACEMENT,
            PORT_BORDER_OFFSET,
            PORT_SIDE,
            PORT_ANCHOR,
            ALLOW_NON_FLOW_PORTS_TO_SWITCH_SIDES,
            NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY,
            NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT,
            SPACING_PORTS_SURROUNDING,
        ] {
            assert_eq!(
                option_by_id(id.name()),
                Some(id),
                "{id} does not round-trip"
            );
        }
    }
}
