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
    EdgeLabelSideSelection, GraphProperties, IntermediateProcessorStrategy, NodeType,
    OrderingStrategy, PortConstraints, PortSide,
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
        S::HIERARCHICAL_PORT_CONSTRAINT_PROCESSOR => "Hierarchical port constraint processing",
        S::HIERARCHICAL_PORT_DUMMY_SIZE_PROCESSOR => "Hierarchical port dummy size processing",
        S::HIERARCHICAL_PORT_POSITION_PROCESSOR => "Hierarchical port position processing",
        S::HIERARCHICAL_PORT_ORTHOGONAL_EDGE_ROUTER => {
            "Orthogonally routing hierarchical port edges"
        }
        S::INVERTED_PORT_PROCESSOR => "Inverted port preprocessing",
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
        // In-layer constraints come from northern and southern external ports, which are not
        // ported; without them there are none to place and the step changes nothing.
        IntermediateProcessorStrategy::IN_LAYER_CONSTRAINT_PROCESSOR => {
            if graph.layers.iter().flatten().any(|&node| {
                graph.nodes[node].in_layer_constraint
                    != crate::layered::lgraph::InLayerConstraint::NONE
            }) {
                return Err(LayoutError::Unsupported(
                    "in-layer constraints (InLayerConstraintProcessor)".to_string(),
                ));
            }
        }
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
            resize_hierarchical_graph(std::slice::from_mut(graph), 0)?;
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
        IntermediateProcessorStrategy::HIERARCHICAL_PORT_CONSTRAINT_PROCESSOR => {
            crate::layered::hierarchical_ports::process_port_constraints(graph)?;
        }
        IntermediateProcessorStrategy::HIERARCHICAL_PORT_DUMMY_SIZE_PROCESSOR => {
            crate::layered::hierarchical_ports::process_dummy_sizes(graph);
        }
        IntermediateProcessorStrategy::HIERARCHICAL_PORT_POSITION_PROCESSOR => {
            crate::layered::hierarchical_ports::process_positions(graph)?;
        }
        IntermediateProcessorStrategy::HIERARCHICAL_PORT_ORTHOGONAL_EDGE_ROUTER => {
            crate::layered::hierarchical_ports::route_hierarchical_port_edges(graph)?;
        }
        IntermediateProcessorStrategy::INVERTED_PORT_PROCESSOR => {
            crate::layered::hierarchical_ports::process_inverted_ports(graph)?;
        }
        IntermediateProcessorStrategy::ONE_SIDED_GREEDY_SWITCH => {
            crate::layered::p3order::greedy_switch(graph, true)?;
        }
        IntermediateProcessorStrategy::TWO_SIDED_GREEDY_SWITCH => {
            crate::layered::p3order::greedy_switch(graph, false)?;
        }
        _ => {}
    }
    progress.done(monitor);
    Ok(())
}

/// `HierarchicalNodeResizingProcessor.process` for the graph at `index` of a compound layout's
/// graphs: the nodes leave their layers for the layerless list, the graph takes its size, and the
/// node the graph is nested in, in another graph, takes the graph's size.
pub fn resize_hierarchical_graph(graphs: &mut [LGraph], index: usize) -> Result<(), LayoutError> {
    let graph = &mut graphs[index];
    for layer in std::mem::take(&mut graph.layers) {
        graph.layerless_nodes.extend(layer);
    }
    graph.layer_sizes.clear();
    for &node in &graph.layerless_nodes {
        graph.nodes[node].layer = None;
    }
    resize_graph(graph)?;
    if let Some((parent_graph, parent_node)) = graph.parent_node {
        graph_layout_to_node(graphs, index, parent_graph, parent_node)?;
    }
    Ok(())
}

/// `HierarchicalNodeResizingProcessor.resizeGraph`: the graph is at least its minimum size under
/// MINIMUM_SIZE (with the defaults of DEFAULT_MINIMUM_SIZE), and the content moves with the extra
/// room that the content alignment asks for.
fn resize_graph(graph: &mut LGraph) -> Result<(), LayoutError> {
    let calculated = graph.actual_size();
    let mut adjusted = calculated;
    if graph
        .properties
        .enum_set_contains(options::NODE_SIZE_CONSTRAINTS, "MINIMUM_SIZE")
    {
        let minimum = crate::layered::transfer::effective_minimum_size(&graph.properties);
        adjusted.x = calculated.x.max(minimum.x);
        adjusted.y = calculated.y.max(minimum.y);
    }
    resize_graph_no_really_i_mean_it(graph, calculated, adjusted)
}

/// `HierarchicalNodeResizingProcessor.resizeGraphNoReallyIMeanIt`: the offset follows the content
/// alignment, and the size becomes the new size less the padding. Eastern and southern external
/// ports would move with a larger size; they are not ported.
fn resize_graph_no_really_i_mean_it(
    graph: &mut LGraph,
    old: KVector,
    new: KVector,
) -> Result<(), LayoutError> {
    let alignment = match graph.properties.get(options::CONTENT_ALIGNMENT) {
        Some(crate::core::data::OptionValue::EnumSet { names, .. }) => names.clone(),
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
    // The eastern and southern external ports move with a larger graph.
    if graph
        .graph_properties
        .contains(&GraphProperties::EXTERNAL_PORTS)
        && (new.x > old.x || new.y > old.y)
    {
        for node in graph.layerless_nodes.clone() {
            if graph.nodes[node].node_type != NodeType::EXTERNAL_PORT {
                continue;
            }
            match crate::layered::hierarchical_ports::ext_port_side(graph, node) {
                PortSide::EAST => graph.nodes[node].position.x += new.x - old.x,
                PortSide::SOUTH => graph.nodes[node].position.y += new.y - old.y,
                _ => {}
            }
        }
    }
    let padding = graph.padding;
    graph.size.x = new.x - padding.left - padding.right;
    graph.size.y = new.y - padding.top - padding.bottom;
    Ok(())
}

/// `HierarchicalNodeResizingProcessor.graphLayoutToNode`: the ports the external port dummies stand
/// for take their positions and sides from the dummies, and the node the graph at `index` is
/// nested in takes the graph's actual size. With external ports, the node's ports are fixed where
/// they are; otherwise they move with the node's size. Its labels move with it either way.
fn graph_layout_to_node(
    graphs: &mut [LGraph],
    index: usize,
    parent_graph: usize,
    parent_node: usize,
) -> Result<(), LayoutError> {
    for node in graphs[index].layerless_nodes.clone() {
        let Some(origin) = graphs[index].nodes[node]
            .external_port
            .as_ref()
            .map(|external| external.origin)
        else {
            continue;
        };
        let size = graphs[parent_graph].ports[origin].size;
        let position = external_port_position(&mut graphs[index], node, size.x, size.y);
        let side = crate::layered::hierarchical_ports::ext_port_side(&graphs[index], node);
        let port = &mut graphs[parent_graph].ports[origin];
        port.position = position;
        port.side = side;
    }
    let actual = graphs[index].actual_size();
    if graphs[index]
        .graph_properties
        .contains(&GraphProperties::EXTERNAL_PORTS)
    {
        // The ports have their positions now.
        let parent = &mut graphs[parent_graph];
        parent.nodes[parent_node].properties.set(
            options::PORT_CONSTRAINTS,
            Some(crate::core::data::OptionValue::Enum {
                class: "PortConstraints",
                name: "FIXED_POS".to_string(),
            }),
        );
        parent
            .graph_properties
            .insert(GraphProperties::NON_FREE_PORTS);
        resize_node(parent, parent_node, actual, false, true);
    } else {
        resize_node(&mut graphs[parent_graph], parent_node, actual, true, true);
    }
    Ok(())
}

/// `LGraphUtil.getExternalPortPosition`: where the port an external port dummy stands for goes on
/// the graph's node, from the dummy's position, the graph's size, padding and offset, and the
/// port's border offset. The dummy moves to the matching place outside the content area.
pub fn external_port_position(
    graph: &mut LGraph,
    dummy: usize,
    port_width: f64,
    port_height: f64,
) -> KVector {
    let node = &graph.nodes[dummy];
    let mut position = KVector {
        x: node.position.x + node.size.x / 2.0,
        y: node.position.y + node.size.y / 2.0,
    };
    let port_offset = crate::layered::compound::border_offset_of(node);
    let size = graph.size;
    let padding = graph.padding;
    let offset = graph.offset;
    match crate::layered::hierarchical_ports::ext_port_side(graph, dummy) {
        PortSide::NORTH => {
            position.x += padding.left + offset.x - port_width / 2.0;
            position.y = -port_height - port_offset;
            graph.nodes[dummy].position.y = -(padding.top + port_offset + offset.y);
        }
        PortSide::EAST => {
            position.x = size.x + padding.left + padding.right + port_offset;
            position.y += padding.top + offset.y - port_height / 2.0;
            graph.nodes[dummy].position.x = size.x + padding.right + port_offset - offset.x;
        }
        PortSide::SOUTH => {
            position.x += padding.left + offset.x - port_width / 2.0;
            position.y = size.y + padding.top + padding.bottom + port_offset;
            graph.nodes[dummy].position.y = size.y + padding.bottom + port_offset - offset.y;
        }
        PortSide::WEST => {
            position.x = -port_width - port_offset;
            position.y += padding.top + offset.y - port_height / 2.0;
            graph.nodes[dummy].position.x = -(padding.left + port_offset + offset.x);
        }
        PortSide::UNDEFINED => {}
    }
    position
}

/// `LGraphUtil.resizeNode(node, newSize, movePorts, moveLabels)`: the node takes the new size, its
/// ports keep their places on its sides, its labels on the right and at the bottom move with the
/// sides, and its size is fixed from now on.
///
/// The Java computes the ratios and differences as floats; elkjs keeps them as doubles, since GWT
/// emulates `float` as `double`.
pub fn resize_node(
    graph: &mut LGraph,
    node: usize,
    new_size: KVector,
    move_ports: bool,
    move_labels: bool,
) {
    let old = graph.nodes[node].size;
    let width_ratio = new_size.x / old.x;
    let height_ratio = new_size.y / old.y;
    let width_diff = new_size.x - old.x;
    let height_diff = new_size.y - old.y;
    if move_ports {
        let fixed_ports = graph.nodes[node]
            .properties
            .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS)
            == PortConstraints::FIXED_POS;
        for &port in &graph.nodes[node].ports {
            let port = &mut graph.ports[port];
            match port.side {
                PortSide::NORTH => {
                    if !fixed_ports {
                        port.position.x *= width_ratio;
                    }
                }
                PortSide::EAST => {
                    port.position.x += width_diff;
                    if !fixed_ports {
                        port.position.y *= height_ratio;
                    }
                }
                PortSide::SOUTH => {
                    if !fixed_ports {
                        port.position.x *= width_ratio;
                    }
                    port.position.y += height_diff;
                }
                PortSide::WEST => {
                    if !fixed_ports {
                        port.position.y *= height_ratio;
                    }
                }
                PortSide::UNDEFINED => {}
            }
        }
    }
    if move_labels {
        for &label in &graph.nodes[node].labels {
            let label = &mut graph.labels[label];
            let mid_x = label.position.x + label.size.x / 2.0;
            let mid_y = label.position.y + label.size.y / 2.0;
            let width_percent = mid_x / old.x;
            let height_percent = mid_y / old.y;
            if width_percent + height_percent >= 1.0 {
                if width_percent - height_percent > 0.0 && mid_y >= 0.0 {
                    // The label is on the right.
                    label.position.x += width_diff;
                    label.position.y += height_diff * height_percent;
                } else if width_percent - height_percent < 0.0 && mid_x >= 0.0 {
                    // The label is at the bottom.
                    label.position.x += width_diff * width_percent;
                    label.position.y += height_diff;
                }
            }
        }
    }
    graph.nodes[node].size = new_size;
    // `SizeConstraint.fixed()`: the size is assumed to stay as determined here.
    graph.nodes[node].properties.set(
        options::NODE_SIZE_CONSTRAINTS,
        Some(crate::core::data::OptionValue::EnumSet {
            class: "SizeConstraint",
            names: Vec::new(),
        }),
    );
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
            layer_size.x = layer_size
                .x
                .max(node.size.x + node.margin.left + node.margin.right);
        }
        let first = &graph.nodes[first];
        let mut top = first.position.y - first.margin.top;
        // An external port at either end keeps the port spacing to the graph's border.
        if first.node_type == NodeType::EXTERNAL_PORT {
            top -= ports_surrounding(graph)?.top;
        }
        let last = &graph.nodes[last];
        let mut bottom = last.position.y + last.size.y + last.margin.bottom;
        if last.node_type == NodeType::EXTERNAL_PORT {
            bottom += ports_surrounding(graph)?.bottom;
        }
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

/// `LayeredOptions.SPACING_PORTS_SURROUNDING` of the graph.
fn ports_surrounding(graph: &LGraph) -> Result<crate::core::math::Spacing, LayoutError> {
    match graph
        .properties
        .value_or_default("org.eclipse.elk.spacing.portsSurrounding")
    {
        Some(crate::core::data::OptionValue::Margin(spacing))
        | Some(crate::core::data::OptionValue::Padding(spacing)) => Ok(spacing),
        _ => Err(LayoutError::Format(
            "org.eclipse.elk.spacing.portsSurrounding is not a margin".to_string(),
        )),
    }
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
        .enum_with_default::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY);
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
                .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layered::lgraph::{LLabel, LNode};

    /// A label whose middle is right of the diagonal moves with the right side, and down by its
    /// share of the height; the node's size becomes fixed.
    #[test]
    fn resizing_a_node_moves_a_label_on_the_right() {
        let mut graph = LGraph::default();
        let label = graph.add_label(LLabel {
            size: KVector { x: 10.0, y: 10.0 },
            position: KVector { x: 80.0, y: 20.0 },
            ..LLabel::default()
        });
        graph.nodes.push(LNode {
            size: KVector { x: 100.0, y: 50.0 },
            labels: vec![label],
            ..LNode::default()
        });
        resize_node(&mut graph, 0, KVector { x: 150.0, y: 100.0 }, true, true);
        // The middle is at (85, 25): 0.85 of the width and 0.5 of the height.
        assert_eq!(graph.labels[label].position, KVector { x: 130.0, y: 45.0 });
        assert_eq!(graph.nodes[0].size, KVector { x: 150.0, y: 100.0 });
        assert!(
            graph.nodes[0]
                .properties
                .enum_set_is_empty(options::NODE_SIZE_CONSTRAINTS)
        );
        assert!(
            graph.nodes[0]
                .properties
                .has(options::NODE_SIZE_CONSTRAINTS)
        );
    }
}
