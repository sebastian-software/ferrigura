//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/ElkLayered.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/LayeredLayoutProvider.java
//!
//! `LayeredLayoutProvider` and `ElkLayered`: the driver of the layered algorithm, for one level of
//! the graph. A level whose hierarchy handling is INCLUDE_CHILDREN is laid out as a compound graph
//! (`doCompoundLayout`); any other level as a flat graph (`doLayout`).

use crate::core::data::enums::HierarchyHandling;
use crate::core::data::property::PropertyHolder;
use crate::core::data::{JavaEnum, OptionValue};
use crate::core::math::KVector;
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::components;
use crate::layered::configurator;
use crate::layered::importer;
use crate::layered::intermediate::{self, Step};
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::phases::PhaseStep;
use crate::layered::transfer;

/// `LayeredLayoutProvider.layout(elkgraph, monitor)` for the level of `node`: lays out its children
/// and writes the positions and the node's size back into it. `parent` holds the options of the
/// node's parent (`None` for the root of the graph), and `chain` is the node's designation prefix.
pub fn layout(
    node: &mut ElkNode,
    parent: Option<&PropertyHolder>,
    chain: &str,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let mut graph = importer::import_level(node, parent, chain)?;
    let hierarchy = node
        .properties
        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING);
    if hierarchy == Some(HierarchyHandling::INCLUDE_CHILDREN) {
        do_compound_layout(&mut graph, progress, monitor)?;
    } else {
        do_layout(&mut graph, progress, monitor)?;
    }
    transfer::apply_layout(&graph, node)
}

/// `ElkLayered.doLayout`: the connected components of the graph are laid out one after another,
/// and then combined into the graph, which is resized to its minimum.
fn do_layout(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Layered layout", 1.0);
    configurator::prepare(graph)?;
    if !components::separates(graph) {
        layout_component(graph, progress, monitor)?;
    } else {
        let mut parts = components::split(graph)?;
        // The components share the graph's random generator, which each one takes in turn.
        let mut random = graph.random.take();
        if parts.len() == 1 {
            // `layout(components.get(0), theMonitor)`: the level's own task.
            let part = &mut parts[0];
            part.random = random.take();
            layout_component(part, progress, monitor)?;
            random = part.random.take();
        } else {
            for part in &mut parts {
                // `layout(comp, theMonitor.subTask(compWork))`: a component is a task of its own.
                let sub = sub_task(progress, monitor)?;
                progress.begin(sub, "Component Layout", 1.0);
                part.random = random.take();
                layout_component(part, progress, sub)?;
                random = part.random.take();
                progress.done(sub);
            }
        }
        graph.random = random;
        components::combine(parts, graph)?;
    }
    resize_flat_graph(graph);
    progress.done(monitor);
    Ok(())
}

/// `ElkLayered.layout` for one graph: the module log, then the processors, then the nodes leave the
/// layers for the layerless list.
fn layout_component(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let steps: Vec<Step> = graph.processors.clone();
    if progress.is_logging_enabled() {
        progress.log(
            monitor,
            format!("ELK Layered uses the following {} modules:", steps.len()),
        );
        for (slot, step) in steps.iter().enumerate() {
            progress.log(
                monitor,
                format!("   Slot {slot:02}: {}", module_name(step)?),
            );
        }
    }
    for step in steps {
        let sub = sub_task(progress, monitor)?;
        run_step(step, graph, progress, sub)?;
    }
    move_nodes_to_layerless(graph);
    Ok(())
}

/// `ElkLayered.doCompoundLayout`.
fn do_compound_layout(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Layered layout", 2.0);
    let pre = sub_task(progress, monitor)?;
    progress.begin(pre, "Compound graph preprocessor", 1.0);
    progress.done(pre);
    let hierarchical = sub_task(progress, monitor)?;
    hierarchical_layout(graph, progress, hierarchical)?;
    let post = sub_task(progress, monitor)?;
    progress.begin(post, "Compound graph postprocessor", 1.0);
    progress.done(post);
    progress.done(monitor);
    Ok(())
}

/// `ElkLayered.hierarchicalLayout` for a graph without nested graphs.
fn hierarchical_layout(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    configurator::prepare(graph)?;
    let steps: Vec<Step> = graph.processors.clone();
    progress.begin(monitor, "Recursive hierarchical layout", steps.len() as f64);
    for step in steps {
        let sub = sub_task(progress, monitor)?;
        run_step(step, graph, progress, sub)?;
    }
    progress.done(monitor);
    Ok(())
}

fn run_step(
    step: Step,
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    match step {
        Step::Intermediate(strategy) => intermediate::run(strategy, graph, progress, monitor),
        Step::Phase(phase) => phase.run(graph, progress, monitor),
    }
}

/// The Java class of a step's processor, as the module log prints it. Only the modules that the
/// configurations ported so far can have are named; any other is unsupported, so the log is never
/// wrong.
fn module_name(step: &Step) -> Result<&'static str, LayoutError> {
    use crate::core::data::enums::IntermediateProcessorStrategy as S;
    use crate::core::data::enums::{
        CrossingMinimizationStrategy, CycleBreakingStrategy, EdgeRouting, LayeringStrategy,
        NodePlacementStrategy,
    };
    let name = match step {
        Step::Intermediate(strategy) => match strategy {
            S::DIRECTION_PREPROCESSOR | S::DIRECTION_POSTPROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.GraphTransformer"
            }
            S::EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER => {
                "org.eclipse.elk.alg.layered.intermediate.EdgeAndLayerConstraintEdgeReverser"
            }
            S::LAYER_CONSTRAINT_PREPROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.LayerConstraintPreprocessor"
            }
            S::LAYER_CONSTRAINT_POSTPROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.LayerConstraintPostprocessor"
            }
            S::LONG_EDGE_SPLITTER => "org.eclipse.elk.alg.layered.intermediate.LongEdgeSplitter",
            S::LONG_EDGE_JOINER => "org.eclipse.elk.alg.layered.intermediate.LongEdgeJoiner",
            S::PORT_SIDE_PROCESSOR => "org.eclipse.elk.alg.layered.intermediate.PortSideProcessor",
            S::PORT_LIST_SORTER => "org.eclipse.elk.alg.layered.intermediate.PortListSorter",
            S::IN_LAYER_CONSTRAINT_PROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.InLayerConstraintProcessor"
            }
            S::LABEL_AND_NODE_SIZE_PROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.LabelAndNodeSizeProcessor"
            }
            S::INNERMOST_NODE_MARGIN_CALCULATOR => {
                "org.eclipse.elk.alg.layered.intermediate.InnermostNodeMarginCalculator"
            }
            S::LAYER_SIZE_AND_GRAPH_HEIGHT_CALCULATOR => {
                "org.eclipse.elk.alg.layered.intermediate.LayerSizeAndGraphHeightCalculator"
            }
            S::END_LABEL_SORTER => "org.eclipse.elk.alg.layered.intermediate.EndLabelSorter",
            S::LABEL_DUMMY_INSERTER => {
                "org.eclipse.elk.alg.layered.intermediate.LabelDummyInserter"
            }
            S::LABEL_DUMMY_SWITCHER => {
                "org.eclipse.elk.alg.layered.intermediate.LabelDummySwitcher"
            }
            S::LABEL_SIDE_SELECTOR => "org.eclipse.elk.alg.layered.intermediate.LabelSideSelector",
            S::LABEL_DUMMY_REMOVER => "org.eclipse.elk.alg.layered.intermediate.LabelDummyRemover",
            S::REVERSED_EDGE_RESTORER => {
                "org.eclipse.elk.alg.layered.intermediate.ReversedEdgeRestorer"
            }
            S::SORT_BY_INPUT_ORDER_OF_MODEL => {
                "org.eclipse.elk.alg.layered.intermediate.SortByInputModelProcessor"
            }
            S::HIERARCHICAL_NODE_RESIZER => {
                "org.eclipse.elk.alg.layered.intermediate.HierarchicalNodeResizingProcessor"
            }
            S::ONE_SIDED_GREEDY_SWITCH | S::TWO_SIDED_GREEDY_SWITCH => {
                "org.eclipse.elk.alg.layered.p3order.LayerSweepCrossingMinimizer"
            }
            other => {
                return Err(LayoutError::Unsupported(format!(
                    "the module name of {}",
                    other.java_name()
                )));
            }
        },
        Step::Phase(phase) => match phase {
            PhaseStep::CycleBreaking(CycleBreakingStrategy::DEPTH_FIRST) => {
                "org.eclipse.elk.alg.layered.p1cycles.DepthFirstCycleBreaker"
            }
            PhaseStep::CycleBreaking(CycleBreakingStrategy::GREEDY) => {
                "org.eclipse.elk.alg.layered.p1cycles.GreedyCycleBreaker"
            }
            PhaseStep::Layering(LayeringStrategy::NETWORK_SIMPLEX) => {
                "org.eclipse.elk.alg.layered.p2layers.NetworkSimplexLayerer"
            }
            PhaseStep::CrossingMinimization(CrossingMinimizationStrategy::LAYER_SWEEP) => {
                "org.eclipse.elk.alg.layered.p3order.LayerSweepCrossingMinimizer"
            }
            PhaseStep::NodePlacement(NodePlacementStrategy::BRANDES_KOEPF) => {
                "org.eclipse.elk.alg.layered.p4nodes.bk.BKNodePlacer"
            }
            PhaseStep::EdgeRouting(EdgeRouting::ORTHOGONAL | EdgeRouting::UNDEFINED) => {
                "org.eclipse.elk.alg.layered.p5edges.OrthogonalEdgeRouter"
            }
            _ => {
                return Err(LayoutError::Unsupported(
                    "the module name of this phase implementation".to_string(),
                ));
            }
        },
    };
    Ok(name)
}

/// `ElkLayered.layout`'s end and `resizeGraph`'s first step: the layers are emptied into the
/// layerless nodes, as the processors of a flat graph leave them in the layers.
fn move_nodes_to_layerless(graph: &mut LGraph) {
    for layer in std::mem::take(&mut graph.layers) {
        graph.layerless_nodes.extend(layer);
    }
    graph.layer_sizes.clear();
    for &node in &graph.layerless_nodes {
        graph.nodes[node].layer = None;
    }
}

/// `ElkLayered.resizeGraph`: the graph is at least its minimum size under MINIMUM_SIZE, and the
/// content moves with the extra room that the content alignment asks for. A graph with a fixed
/// size keeps its size.
fn resize_flat_graph(graph: &mut LGraph) {
    let calculated = graph.actual_size();
    let mut adjusted = calculated;
    if graph
        .properties
        .enum_set_contains(options::NODE_SIZE_CONSTRAINTS, "MINIMUM_SIZE")
    {
        let minimum = transfer::effective_minimum_size(&graph.properties);
        adjusted.x = calculated.x.max(minimum.x);
        adjusted.y = calculated.y.max(minimum.y);
    }
    if !graph
        .properties
        .bool_of(options::NODE_SIZE_FIXED_GRAPH_SIZE)
    {
        resize_graph_to(graph, calculated, adjusted);
    }
}

/// `ElkLayered.resizeGraphNoReallyIMeanIt`: the offset follows the content alignment, and the size
/// becomes the new size less the padding. External ports are not part of a flat graph's layout.
fn resize_graph_to(graph: &mut LGraph, old: KVector, new: KVector) {
    let alignment = match graph.properties.get(options::CONTENT_ALIGNMENT) {
        Some(OptionValue::EnumSet { names, .. }) => names.clone(),
        _ => Vec::new(),
    };
    let aligned = |name: &str| alignment.iter().any(|found| found == name);
    if new.x > old.x {
        if aligned("H_CENTER") {
            graph.offset.x += (new.x - old.x) / 2.0;
        } else if aligned("H_RIGHT") {
            graph.offset.x += new.x - old.x;
        }
    }
    if new.y > old.y {
        if aligned("V_CENTER") {
            graph.offset.y += (new.y - old.y) / 2.0;
        } else if aligned("V_BOTTOM") {
            graph.offset.y += new.y - old.y;
        }
    }
    let padding = graph.padding;
    graph.size.x = new.x - padding.left - padding.right;
    graph.size.y = new.y - padding.top - padding.bottom;
}

fn sub_task(progress: &mut ProgressMonitor, monitor: Monitor) -> Result<Monitor, LayoutError> {
    progress
        .sub_task(monitor, 1.0)
        .ok_or_else(|| LayoutError::Unsupported("subtask of a finished monitor".to_string()))
}
