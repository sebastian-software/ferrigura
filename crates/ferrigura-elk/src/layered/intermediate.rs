//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/IntermediateProcessorStrategy.java and the processors it creates
//!
//! Intermediate processors, `IntermediateProcessorStrategy` and its processors.
//!
//! Each strategy gives the task name the processor logs, as its `begin` call
//! does in the Java source. The processors run their nodes-only behaviour; the
//! ones that need edges, ports or labels are reported as unsupported when
//! their name is needed, so they cannot silently do nothing.

use crate::core::data::JavaEnum;
use crate::core::data::enums::{
    EdgeLabelSideSelection, HierarchyHandling, IntermediateProcessorStrategy, NodeType,
    OrderingStrategy, PortConstraints,
};
use crate::core::math::KVector;
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::layered::LayoutError;
use crate::layered::dummies;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::phases::PhaseStep;
use crate::layered::ports;

/// One step of the algorithm: a phase implementation or an intermediate processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Phase(PhaseStep),
    Intermediate(IntermediateProcessorStrategy),
}

/// The task name a strategy's processor logs, for the processors of the nodes-only path.
pub fn task_name(
    strategy: IntermediateProcessorStrategy,
    graph: &LGraph,
) -> Result<String, LayoutError> {
    use IntermediateProcessorStrategy as S;
    let fixed = match strategy {
        S::DIRECTION_PREPROCESSOR => "Graph transformation (TO_INPUT_DIRECTION)",
        S::DIRECTION_POSTPROCESSOR => "Graph transformation (TO_INTERNAL_LTR)",
        S::EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER => "Edge and layer constraint edge reversal",
        S::LAYER_CONSTRAINT_PREPROCESSOR => "Layer constraint preprocessing",
        S::LAYER_CONSTRAINT_POSTPROCESSOR => "Layer constraint postprocessing",
        S::LONG_EDGE_SPLITTER => "Edge splitting",
        S::PORT_SIDE_PROCESSOR => "Port side processing",
        S::PORT_LIST_SORTER => "Port order processing",
        S::IN_LAYER_CONSTRAINT_PROCESSOR => "Layer constraint edge reversal",
        S::LABEL_AND_NODE_SIZE_PROCESSOR => "Node and Port Label Placement and Node Sizing",
        S::INNERMOST_NODE_MARGIN_CALCULATOR => "Node margin calculation",
        S::LAYER_SIZE_AND_GRAPH_HEIGHT_CALCULATOR => "Layer size calculation",
        S::LABEL_DUMMY_INSERTER => "Label dummy insertions",
        S::LONG_EDGE_JOINER => "Edge joining",
        S::END_LABEL_PREPROCESSOR => "End label pre-processing",
        S::END_LABEL_POSTPROCESSOR => "End label post-processing",
        S::END_LABEL_SORTER => "Sort end labels",
        S::REVERSED_EDGE_RESTORER => "Restoring reversed edges",
        S::SELF_LOOP_PREPROCESSOR => "Self-Loop pre-processing",
        S::SELF_LOOP_PORT_RESTORER => "Self-Loop ordering",
        S::SELF_LOOP_ROUTER => "Self-Loop routing",
        S::SELF_LOOP_POSTPROCESSOR => "Self-Loop post-processing",
        S::HIERARCHICAL_NODE_RESIZER => "Resize child graph to fit parent.",
        // `LayerSweepCrossingMinimizer` begins with `"Minimize Crossings " + crossMinType`.
        S::ONE_SIDED_GREEDY_SWITCH => "Minimize Crossings ONE_SIDED_GREEDY_SWITCH",
        S::TWO_SIDED_GREEDY_SWITCH => "Minimize Crossings TWO_SIDED_GREEDY_SWITCH",
        S::LABEL_DUMMY_SWITCHER => "Label dummy switching",
        S::LABEL_DUMMY_REMOVER => "Label dummy removal",
        S::LABEL_SIDE_SELECTOR => {
            let mode = graph
                .properties
                .enum_of::<EdgeLabelSideSelection>(options::EDGE_LABELS_SIDE_SELECTION)
                .map_or("null", JavaEnum::java_name);
            return Ok(format!("Label side selection ({mode})"));
        }
        S::SORT_BY_INPUT_ORDER_OF_MODEL => {
            let order = graph
                .properties
                .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
                .map_or("NONE", JavaEnum::java_name);
            return Ok(format!("Sort By Input Model {order}"));
        }
        other => {
            return Err(LayoutError::Unsupported(format!(
                "intermediate processor {} is not ported yet",
                other.java_name()
            )));
        }
    };
    Ok(fixed.to_string())
}

/// Runs one intermediate processor: its task, under `monitor`.
pub fn run(
    strategy: IntermediateProcessorStrategy,
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let name = task_name(strategy, graph)?;
    progress.begin(monitor, &name, 1.0);
    match strategy {
        IntermediateProcessorStrategy::DIRECTION_PREPROCESSOR => {
            crate::layered::graph_transformer::transform(
                graph,
                crate::layered::graph_transformer::Mode::ToInputDirection,
            )?;
        }
        IntermediateProcessorStrategy::DIRECTION_POSTPROCESSOR => {
            crate::layered::graph_transformer::transform(
                graph,
                crate::layered::graph_transformer::Mode::ToInternalLtr,
            )?;
        }
        IntermediateProcessorStrategy::SORT_BY_INPUT_ORDER_OF_MODEL => {
            sort_by_input_model(graph, progress, monitor)?;
        }
        IntermediateProcessorStrategy::LAYER_SIZE_AND_GRAPH_HEIGHT_CALCULATOR => {
            layer_size_and_graph_height(graph)?;
        }
        IntermediateProcessorStrategy::LABEL_DUMMY_INSERTER => {
            dummies::insert_label_dummies(graph);
        }
        IntermediateProcessorStrategy::PORT_SIDE_PROCESSOR => {
            ports::process_port_sides(graph);
        }
        IntermediateProcessorStrategy::LONG_EDGE_SPLITTER => {
            dummies::split_long_edges(graph);
        }
        IntermediateProcessorStrategy::LONG_EDGE_JOINER => {
            dummies::join_long_edges(graph);
        }
        IntermediateProcessorStrategy::EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER => {
            crate::layered::layerconstraints::reverse_edges(graph)?;
        }
        IntermediateProcessorStrategy::LAYER_CONSTRAINT_PREPROCESSOR => {
            crate::layered::layerconstraints::preprocess(graph)?;
        }
        IntermediateProcessorStrategy::LAYER_CONSTRAINT_POSTPROCESSOR => {
            crate::layered::layerconstraints::postprocess(graph)?;
        }
        // In-layer constraints come from external ports, which are not ported, so there are none to
        // place and the step changes nothing.
        IntermediateProcessorStrategy::IN_LAYER_CONSTRAINT_PROCESSOR => {}
        // Reversed edges are restored by a processor that is not ported yet; with none, it does nothing.
        IntermediateProcessorStrategy::REVERSED_EDGE_RESTORER => {
            crate::layered::p1cycles::restore_reversed_edges(graph);
        }
        IntermediateProcessorStrategy::SELF_LOOP_PREPROCESSOR => {
            crate::layered::loops::preprocess(graph)?;
        }
        IntermediateProcessorStrategy::SELF_LOOP_PORT_RESTORER => {
            crate::layered::loops::restore_ports(graph)?;
        }
        IntermediateProcessorStrategy::SELF_LOOP_ROUTER => {
            crate::layered::loops::route(graph)?;
        }
        IntermediateProcessorStrategy::SELF_LOOP_POSTPROCESSOR => {
            crate::layered::loops::postprocess(graph)?;
        }
        IntermediateProcessorStrategy::END_LABEL_PREPROCESSOR => {
            crate::layered::endlabels::preprocess(graph);
        }
        IntermediateProcessorStrategy::END_LABEL_SORTER => {
            crate::layered::endlabels::sort(graph);
        }
        IntermediateProcessorStrategy::END_LABEL_POSTPROCESSOR => {
            crate::layered::endlabels::postprocess(graph);
        }
        IntermediateProcessorStrategy::PORT_LIST_SORTER => {
            ports::sort_ports(graph)?;
        }
        IntermediateProcessorStrategy::LABEL_AND_NODE_SIZE_PROCESSOR => {
            crate::layered::nodespacing::label_and_node_size(graph)?;
        }
        IntermediateProcessorStrategy::INNERMOST_NODE_MARGIN_CALCULATOR => {
            crate::layered::nodespacing::node_margins(graph)?;
        }
        IntermediateProcessorStrategy::HIERARCHICAL_NODE_RESIZER => {
            resize_root(graph)?;
        }
        IntermediateProcessorStrategy::LABEL_DUMMY_SWITCHER => {
            crate::layered::labels::label_dummy_switching(graph)?;
        }
        IntermediateProcessorStrategy::LABEL_SIDE_SELECTOR => {
            crate::layered::labels::label_side_selection(graph)?;
        }
        IntermediateProcessorStrategy::LABEL_DUMMY_REMOVER => {
            crate::layered::labels::label_dummy_removal(graph)?;
        }
        IntermediateProcessorStrategy::ONE_SIDED_GREEDY_SWITCH
        | IntermediateProcessorStrategy::TWO_SIDED_GREEDY_SWITCH => {
            greedy_switch(graph)?;
        }
        _ => {}
    }
    progress.done(monitor);
    Ok(())
}

/// `LayerSweepCrossingMinimizer` run as a greedy switch. It returns at once for an empty graph and
/// for a single node outside a hierarchical layout, as its `process` does before it sorts anything.
/// Other graphs need the greedy switch heuristic, which is not ported.
fn greedy_switch(graph: &LGraph) -> Result<(), LayoutError> {
    let empty = graph.layers.iter().all(Vec::is_empty);
    let single = graph.layers.len() == 1 && graph.layers[0].len() == 1;
    let hierarchical = graph
        .properties
        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
        == Some(HierarchyHandling::INCLUDE_CHILDREN);
    if empty || (single && !hierarchical) {
        return Ok(());
    }
    Err(LayoutError::Unsupported(
        "greedy switch crossing minimization on several nodes".to_string(),
    ))
}

/// `HierarchicalNodeResizingProcessor.process` for the root graph: the nodes leave their
/// layers for the layerless list, and the graph's size becomes its actual size less the padding.
fn resize_root(graph: &mut LGraph) -> Result<(), LayoutError> {
    if graph
        .properties
        .enum_set_contains(options::NODE_SIZE_CONSTRAINTS, "MINIMUM_SIZE")
    {
        return Err(LayoutError::Unsupported(
            "a minimum node size on the root".to_string(),
        ));
    }
    for layer in std::mem::take(&mut graph.layers) {
        graph.layerless_nodes.extend(layer);
    }
    graph.layer_sizes.clear();
    for &node in &graph.layerless_nodes {
        graph.nodes[node].layer = None;
    }
    // `resizeGraph` without a minimum size: the adjusted size is the actual size, so the
    // offset does not change and the size is the actual size less the padding.
    let actual = graph.actual_size();
    graph.size.x = actual.x - graph.padding.left - graph.padding.right;
    graph.size.y = actual.y - graph.padding.top - graph.padding.bottom;
    Ok(())
}

/// `LayerSizeAndGraphHeightCalculator.process`: the size of each layer, the graph's height,
/// and the offset that moves the topmost node of the layers to zero.
fn layer_size_and_graph_height(graph: &mut LGraph) -> Result<(), LayoutError> {
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found_nodes = false;
    let mut sizes = Vec::with_capacity(graph.layers.len());
    for layer in &graph.layers {
        let mut layer_size = KVector::default();
        let (Some(&first), Some(&last)) = (layer.first(), layer.last()) else {
            sizes.push(layer_size);
            continue;
        };
        found_nodes = true;
        for &node in layer {
            let node = &graph.nodes[node];
            if node.node_type == NodeType::EXTERNAL_PORT {
                return Err(LayoutError::Unsupported(
                    "external ports in the layer size calculation".to_string(),
                ));
            }
            layer_size.x = layer_size
                .x
                .max(node.size.x + node.margin.left + node.margin.right);
        }
        let first = &graph.nodes[first];
        let top = first.position.y - first.margin.top;
        let last = &graph.nodes[last];
        let bottom = last.position.y + last.size.y + last.margin.bottom;
        layer_size.y = bottom - top;
        min_y = min_y.min(top);
        max_y = max_y.max(bottom);
        sizes.push(layer_size);
    }
    // Without nodes the graph's height is zero, as the Java sets the extremes.
    if !found_nodes {
        min_y = 0.0;
        max_y = 0.0;
    }
    graph.layer_sizes = sizes;
    graph.size.y = max_y - min_y;
    graph.offset.y -= min_y;
    Ok(())
}

/// `SortByInputModelProcessor.process`: the ports of each node that the input order does not fix
/// are sorted by their edges, and each layer's nodes by their model order. The logs name the port
/// lists and the layers.
fn sort_by_input_model(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let strategy = graph
        .properties
        .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
        .unwrap_or(OrderingStrategy::NONE);
    let port_model_order = graph
        .properties
        .bool_of(options::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER);
    let long_edge_strategy = graph
        .properties
        .enum_of::<crate::core::data::enums::LongEdgeOrderingStrategy>(
            options::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY,
        )
        .unwrap_or(crate::core::data::enums::LongEdgeOrderingStrategy::DUMMY_NODE_OVER);
    for layer_index in 0..graph.layers.len() {
        // The first layer is its own previous layer, as in the Java.
        let previous_layer = graph.layers[layer_index.saturating_sub(1)].clone();
        for node in graph.layers[layer_index].clone() {
            let constraints = graph.nodes[node]
                .properties
                .enum_of::<PortConstraints>(options::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::UNDEFINED);
            if constraints != PortConstraints::FIXED_ORDER
                && constraints != PortConstraints::FIXED_POS
            {
                let targets =
                    crate::layered::preserveorder::long_edge_target_node_preprocessing(graph, node);
                let mut comparator = crate::layered::preserveorder::ModelOrderPortComparator::new(
                    previous_layer.clone(),
                    strategy,
                    targets,
                    port_model_order,
                );
                let mut ports = graph.nodes[node].ports.clone();
                {
                    let view: &LGraph = &*graph;
                    crate::java::sort::sort_by(&mut ports, |&a, &b| comparator.compare(view, a, b));
                }
                graph.nodes[node].ports = ports;
                let names: Vec<String> = graph.nodes[node]
                    .ports
                    .iter()
                    .map(|&port| port_name(graph, port))
                    .collect();
                progress.log(
                    monitor,
                    format!(
                        "Node {} ports: [{}]",
                        graph.node_name(node),
                        names.join(", ")
                    ),
                );
            }
        }
        let mut comparator = crate::layered::preserveorder::ModelOrderNodeComparator::new(
            previous_layer,
            strategy,
            long_edge_strategy,
        );
        let mut members = graph.layers[layer_index].clone();
        {
            let view: &LGraph = &*graph;
            crate::java::sort::sort_by(&mut members, |&a, &b| comparator.compare(view, a, b));
        }
        graph.layers[layer_index] = members;
        let names: Vec<String> = graph.layers[layer_index]
            .iter()
            .map(|&node| graph.node_name(node))
            .collect();
        progress.log(
            monitor,
            format!(
                "Layer {}: L_{}[{}]",
                layer_index,
                layer_index,
                names.join(", ")
            ),
        );
    }
    Ok(())
}

/// `LPort.toString()`: `p_<index>[<owner>]`, with the single edge's other end, as ` << ` for an
/// incoming and ` >> ` for an outgoing edge, when the port has only that one edge.
fn port_name(graph: &LGraph, port: usize) -> String {
    let data = &graph.ports[port];
    let index_on_owner = |port: usize| {
        let owner = graph.ports[port].node;
        graph.nodes[owner]
            .ports
            .iter()
            .position(|&member| member == port)
            .map_or(-1, |index| index as i64)
    };
    let mut text = format!("p_{}[{}]", index_on_owner(port), graph.node_name(data.node));
    if data.incoming.len() == 1 && data.outgoing.is_empty() {
        let source = graph.edges[data.incoming[0]].source;
        if source != port {
            text += &format!(
                " << {}[{}]",
                index_on_owner(source),
                graph.node_name(graph.ports[source].node)
            );
        }
    }
    if data.incoming.is_empty() && data.outgoing.len() == 1 {
        let target = graph.edges[data.outgoing[0]].target;
        if target != port {
            text += &format!(
                " >> {}[{}]",
                index_on_owner(target),
                graph.node_name(graph.ports[target].node)
            );
        }
    }
    text
}
