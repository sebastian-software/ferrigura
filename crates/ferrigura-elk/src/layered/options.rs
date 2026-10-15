//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/Layered.melk (generated LayeredOptions)
//!
//! Option ids that the layered algorithm reads, `LayeredOptions` in the Java sources.
//!
//! Every id here is checked against the generated registry by a test, so a
//! misspelt id fails the build instead of reading a default.

pub const CYCLE_BREAKING_STRATEGY: &str = "org.eclipse.elk.layered.cycleBreaking.strategy";
pub const LAYERING_STRATEGY: &str = "org.eclipse.elk.layered.layering.strategy";
pub const CROSSING_MINIMIZATION_STRATEGY: &str =
    "org.eclipse.elk.layered.crossingMinimization.strategy";
pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE: &str =
    "org.eclipse.elk.layered.crossingMinimization.greedySwitch.type";
pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_ACTIVATION_THRESHOLD: &str =
    "org.eclipse.elk.layered.crossingMinimization.greedySwitch.activationThreshold";
pub const CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE: &str =
    "org.eclipse.elk.layered.crossingMinimization.greedySwitchHierarchical.type";
pub const CROSSING_MINIMIZATION_SEMI_INTERACTIVE: &str =
    "org.eclipse.elk.layered.crossingMinimization.semiInteractive";
pub const NODE_PLACEMENT_STRATEGY: &str = "org.eclipse.elk.layered.nodePlacement.strategy";
pub const NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES: &str =
    "org.eclipse.elk.layered.nodePlacement.favorStraightEdges";
pub const EDGE_ROUTING: &str = "org.eclipse.elk.edgeRouting";
pub const CONSIDER_MODEL_ORDER_NO_MODEL_ORDER: &str =
    "org.eclipse.elk.layered.considerModelOrder.noModelOrder";
pub const CONSIDER_MODEL_ORDER_COMPONENTS: &str =
    "org.eclipse.elk.layered.considerModelOrder.components";
pub const DIRECTION_CONGRUENCY: &str = "org.eclipse.elk.layered.directionCongruency";
pub const NODE_PLACEMENT_BK_FIXED_ALIGNMENT: &str =
    "org.eclipse.elk.layered.nodePlacement.bk.fixedAlignment";
pub const NODE_PLACEMENT_BK_EDGE_STRAIGHTENING: &str =
    "org.eclipse.elk.layered.nodePlacement.bk.edgeStraightening";
pub const FEEDBACK_EDGES: &str = "org.eclipse.elk.layered.feedbackEdges";
pub const CONSIDER_MODEL_ORDER_STRATEGY: &str =
    "org.eclipse.elk.layered.considerModelOrder.strategy";
pub const NODE_PROMOTION_STRATEGY: &str = "org.eclipse.elk.layered.layering.nodePromotion.strategy";
pub const POST_COMPACTION_STRATEGY: &str =
    "org.eclipse.elk.layered.compaction.postCompaction.strategy";
pub const WRAPPING_STRATEGY: &str = "org.eclipse.elk.layered.wrapping.strategy";
pub const HIGH_DEGREE_NODES_TREATMENT: &str = "org.eclipse.elk.layered.highDegreeNodes.treatment";
pub const INTERACTIVE: &str = "org.eclipse.elk.interactive";
pub const GENERATE_POSITION_AND_LAYER_IDS: &str =
    "org.eclipse.elk.layered.generatePositionAndLayerIds";
pub const HIERARCHY_HANDLING: &str = "org.eclipse.elk.hierarchyHandling";
pub const DIRECTION: &str = "org.eclipse.elk.direction";
pub const RANDOM_SEED: &str = "org.eclipse.elk.randomSeed";
pub const THOROUGHNESS: &str = "org.eclipse.elk.layered.thoroughness";
pub const SEPARATE_CONNECTED_COMPONENTS: &str = "org.eclipse.elk.separateConnectedComponents";
pub const ALGORITHM: &str = "org.eclipse.elk.algorithm";
pub const FORCE_NODE_MODEL_ORDER: &str =
    "org.eclipse.elk.layered.crossingMinimization.forceNodeModelOrder";
pub const ALIGNMENT: &str = "org.eclipse.elk.alignment";
pub const PADDING: &str = "org.eclipse.elk.padding";
pub const NODE_SIZE_CONSTRAINTS: &str = "org.eclipse.elk.nodeSize.constraints";
pub const NODE_SIZE_OPTIONS: &str = "org.eclipse.elk.nodeSize.options";
pub const NODE_SIZE_FIXED_GRAPH_SIZE: &str = "org.eclipse.elk.nodeSize.fixedGraphSize";
pub const NO_LAYOUT: &str = "org.eclipse.elk.noLayout";
pub const PORT_CONSTRAINTS: &str = "org.eclipse.elk.portConstraints";
pub const SPACING_EDGE_LABEL: &str = "org.eclipse.elk.spacing.edgeLabel";
pub const SPACING_LABEL_LABEL: &str = "org.eclipse.elk.spacing.labelLabel";
pub const EDGE_LABELS_PLACEMENT: &str = "org.eclipse.elk.edgeLabels.placement";
pub const EDGE_LABELS_SIDE_SELECTION: &str = "org.eclipse.elk.layered.edgeLabels.sideSelection";
pub const EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY: &str =
    "org.eclipse.elk.layered.edgeLabels.centerLabelPlacementStrategy";
pub const EDGE_LABELS_INLINE: &str = "org.eclipse.elk.edgeLabels.inline";
pub const SPACING_EDGE_NODE_BETWEEN_LAYERS: &str =
    "org.eclipse.elk.layered.spacing.edgeNodeBetweenLayers";
pub const SPACING_NODE_NODE_BETWEEN_LAYERS: &str =
    "org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers";
pub const UNNECESSARY_BENDPOINTS: &str = "org.eclipse.elk.layered.unnecessaryBendpoints";
pub const PRIORITY_SHORTNESS: &str = "org.eclipse.elk.layered.priority.shortness";
pub const LAYER_CONSTRAINT: &str = "org.eclipse.elk.layered.layering.layerConstraint";
pub const CONSIDER_MODEL_ORDER_CROSSING_COUNTER_NODE_INFLUENCE: &str =
    "org.eclipse.elk.layered.considerModelOrder.crossingCounterNodeInfluence";
pub const CONSIDER_MODEL_ORDER_CROSSING_COUNTER_PORT_INFLUENCE: &str =
    "org.eclipse.elk.layered.considerModelOrder.crossingCounterPortInfluence";

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
        ] {
            assert!(option_by_id(id).is_some(), "{id} is not registered");
        }
    }
}
