//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/ElkLayered.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/LayeredLayoutProvider.java
//!
//! `LayeredLayoutProvider` and `ElkLayered`: the driver of the layered algorithm, for one level of
//! the graph. A level whose hierarchy handling is INCLUDE_CHILDREN is laid out as a compound graph
//! (`doCompoundLayout`); any other level as a flat graph (`doLayout`).

use crate::core::data::enums::{CrossingMinimizationStrategy, HierarchyHandling};
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
    let hierarchy = node
        .properties
        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING);
    if hierarchy == Some(HierarchyHandling::INCLUDE_CHILDREN) {
        let mut graphs = importer::import_compound(node, parent, chain)?;
        do_compound_layout(&mut graphs, progress, monitor)?;
        transfer::apply_compound_layout(&graphs, node)
    } else {
        let mut graph = importer::import_level(node, parent, chain)?;
        do_layout(&mut graph, progress, monitor)?;
        transfer::apply_layout(&graph, node)
    }
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

/// `ElkLayered.doCompoundLayout`: the graphs of a compound layout, the top graph first and the
/// nested graphs after it (see `importer::import_compound`). The cross-hierarchy edges that the
/// compound graph pre- and postprocessor split and join are refused by the importer, so the two
/// processors have nothing to do here.
fn do_compound_layout(
    graphs: &mut [LGraph],
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Layered layout", 2.0);
    let pre = sub_task(progress, monitor)?;
    progress.begin(pre, "Compound graph preprocessor", 1.0);
    progress.done(pre);
    let hierarchical = sub_task(progress, monitor)?;
    hierarchical_layout(graphs, progress, hierarchical)?;
    let post = sub_task(progress, monitor)?;
    progress.begin(post, "Compound graph postprocessor", 1.0);
    progress.done(post);
    progress.done(monitor);
    Ok(())
}

/// `ElkLayered.hierarchicalLayout`: every graph has its own processors, and the graphs run them
/// from the bottom of the hierarchy up. A processor that works on the whole hierarchy (the layer
/// sweep crossing minimizer) pauses each graph until the top graph has run it for all of them.
fn hierarchical_layout(
    graphs: &mut [LGraph],
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let order = collect_all_graphs_bottom_up(graphs);
    review_and_correct_hierarchical_processors(graphs)?;
    let mut work = 0;
    for &index in &order {
        configurator::prepare(&mut graphs[index])?;
        work += graphs[index].processors.len();
    }
    progress.begin(monitor, "Recursive hierarchical layout", work as f64);
    // The next processor of each graph, as the Java's iterators keep it.
    let mut next = vec![0; graphs.len()];
    let root = 0;
    while next[root] < graphs[root].processors.len() {
        for &index in &order {
            while next[index] < graphs[index].processors.len() {
                let step = graphs[index].processors[next[index]];
                next[index] += 1;
                if !is_hierarchy_aware(step) {
                    let sub = sub_task(progress, monitor)?;
                    run_compound_step(step, graphs, index, progress, sub)?;
                } else if index == root {
                    // The processor runs on the top graph, for the whole hierarchy; then the
                    // graphs continue from the bottom.
                    let sub = sub_task(progress, monitor)?;
                    run_hierarchical_step(step, graphs, progress, sub)?;
                    break;
                } else {
                    // Skipped here; the top graph runs it for this graph.
                    break;
                }
            }
        }
    }
    progress.done(monitor);
    Ok(())
}

/// `collectAllGraphsBottomUp`: a search from the top graph, whose list is reversed so that every
/// graph comes before the graph it is nested in. The search takes the graphs from a stack.
fn collect_all_graphs_bottom_up(graphs: &[LGraph]) -> Vec<usize> {
    let mut collected = vec![0];
    let mut continue_with = vec![0];
    while let Some(next) = continue_with.pop() {
        for &node in &graphs[next].layerless_nodes {
            if let Some(nested) = graphs[next].nodes[node].nested_graph {
                collected.push(nested);
                continue_with.push(nested);
            }
        }
    }
    collected.reverse();
    collected
}

/// `reviewAndCorrectHierarchicalProcessors`: every graph must minimize crossings with the top
/// graph's strategy, and takes the top graph's hierarchical greedy switch type.
fn review_and_correct_hierarchical_processors(graphs: &mut [LGraph]) -> Result<(), LayoutError> {
    let root_strategy = graphs[0]
        .properties
        .enum_with_default::<CrossingMinimizationStrategy>(options::CROSSING_MINIMIZATION_STRATEGY);
    let root_type = graphs[0]
        .properties
        .value_or_default(options::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE);
    for graph in graphs.iter_mut() {
        let strategy = graph
            .properties
            .enum_with_default::<CrossingMinimizationStrategy>(
                options::CROSSING_MINIMIZATION_STRATEGY,
            );
        if strategy != root_strategy {
            return Err(LayoutError::Unsupported(format!(
                "the hierarchy aware processor {} in child node {} is only allowed if the root \
                 node specifies the same hierarchical processor",
                strategy.java_name(),
                graph.root_id
            )));
        }
        graph.properties.set(
            options::CROSSING_MINIMIZATION_GREEDY_SWITCH_HIERARCHICAL_TYPE,
            root_type.clone(),
        );
    }
    Ok(())
}

/// Whether the step's processor is an `IHierarchyAwareLayoutProcessor`: the layer sweep crossing
/// minimizer, as the crossing minimization phase or as a greedy switch processor.
fn is_hierarchy_aware(step: Step) -> bool {
    use crate::core::data::enums::IntermediateProcessorStrategy as S;
    matches!(
        step,
        Step::Phase(PhaseStep::CrossingMinimization(
            CrossingMinimizationStrategy::LAYER_SWEEP
        )) | Step::Intermediate(S::ONE_SIDED_GREEDY_SWITCH | S::TWO_SIDED_GREEDY_SWITCH)
    )
}

/// A step of one graph of a compound layout. The hierarchical node resizer also sizes the node the
/// graph is nested in, which is in another graph.
fn run_compound_step(
    step: Step,
    graphs: &mut [LGraph],
    index: usize,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    use crate::core::data::enums::IntermediateProcessorStrategy as S;
    if step == Step::Intermediate(S::HIERARCHICAL_NODE_RESIZER) {
        let name = intermediate::task_name(S::HIERARCHICAL_NODE_RESIZER, &graphs[index])?;
        progress.begin(monitor, &name, 1.0);
        intermediate::resize_hierarchical_graph(graphs, index)?;
        progress.done(monitor);
        return Ok(());
    }
    run_step(step, &mut graphs[index], progress, monitor)
}

/// A hierarchy aware step, run by the top graph for all graphs.
fn run_hierarchical_step(
    step: Step,
    graphs: &mut [LGraph],
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    match step {
        Step::Phase(phase @ PhaseStep::CrossingMinimization(_)) => {
            let name = phase.task_name(&graphs[0])?;
            progress.begin(monitor, &name, 1.0);
            crate::layered::p3order::sweep_compound(graphs)?;
            progress.done(monitor);
            Ok(())
        }
        _ => Err(LayoutError::Unsupported(
            "the greedy switch in a compound layout (a hierarchical LayerSweepCrossingMinimizer)"
                .to_string(),
        )),
    }
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
            S::SELF_LOOP_PREPROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.SelfLoopPreProcessor"
            }
            S::SELF_LOOP_PORT_RESTORER => {
                "org.eclipse.elk.alg.layered.intermediate.SelfLoopPortRestorer"
            }
            S::SELF_LOOP_ROUTER => "org.eclipse.elk.alg.layered.intermediate.SelfLoopRouter",
            S::SELF_LOOP_POSTPROCESSOR => {
                "org.eclipse.elk.alg.layered.intermediate.SelfLoopPostProcessor"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::lgraph::LNode;

    /// A top graph with nodes `a` and `b` holding graphs 1 and 2, and graph 1 with a node holding
    /// graph 3.
    fn hierarchy() -> Vec<LGraph> {
        let compound = |nested| LNode {
            nested_graph: Some(nested),
            ..LNode::default()
        };
        let mut graphs = vec![LGraph::default(); 4];
        graphs[0].nodes = vec![compound(1), compound(2)];
        graphs[0].layerless_nodes = vec![0, 1];
        graphs[1].nodes = vec![LNode::default(), compound(3)];
        graphs[1].layerless_nodes = vec![0, 1];
        graphs
    }

    /// The search takes graphs from a stack: graph 2 is searched before graph 1, so graph 3 is
    /// found last and comes first in the reversed list, before graph 2.
    #[test]
    fn graphs_are_collected_bottom_up_from_a_stack() {
        assert_eq!(collect_all_graphs_bottom_up(&hierarchy()), vec![3, 2, 1, 0]);
    }
}
