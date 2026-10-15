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
use crate::core::data::OptionValue;
use crate::core::data::enums::{
    Alignment, Direction, DirectionCongruency, EdgeLabelSideSelection,
    IntermediateProcessorStrategy, NodeType, OrderingStrategy, PortSide,
};
use crate::core::math::{KVector, Spacing};
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
        S::END_LABEL_SORTER => "Sort end labels",
        S::REVERSED_EDGE_RESTORER => "Restoring reversed edges",
        S::HIERARCHICAL_NODE_RESIZER => "Resize child graph to fit parent.",
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
        IntermediateProcessorStrategy::DIRECTION_PREPROCESSOR
        | IntermediateProcessorStrategy::DIRECTION_POSTPROCESSOR => {
            transform_direction(graph)?;
        }
        IntermediateProcessorStrategy::SORT_BY_INPUT_ORDER_OF_MODEL => {
            sort_by_input_model(graph, progress, monitor);
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
        IntermediateProcessorStrategy::PORT_LIST_SORTER => {
            ports::sort_ports(graph)?;
        }
        IntermediateProcessorStrategy::LABEL_AND_NODE_SIZE_PROCESSOR => {
            // Without labels or size constraints the processor leaves the nodes alone.
            if graph.nodes.iter().any(|node| {
                !node
                    .properties
                    .enum_set_is_empty(options::NODE_SIZE_CONSTRAINTS)
            }) {
                return Err(LayoutError::Unsupported(
                    "node size constraints".to_string(),
                ));
            }
        }
        IntermediateProcessorStrategy::HIERARCHICAL_NODE_RESIZER => {
            resize_root(graph)?;
        }
        _ => {}
    }
    progress.done(monitor);
    Ok(())
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

/// `GraphTransformer.process` under `DirectionCongruency.READING_DIRECTION`: a
/// transpose for `DOWN`, nothing for `RIGHT`. The mirror transforms for `LEFT` and
/// `UP` need the graph's size and offset, which are not ported yet.
fn transform_direction(graph: &mut LGraph) -> Result<(), LayoutError> {
    let congruency = graph
        .properties
        .enum_of::<DirectionCongruency>(options::DIRECTION_CONGRUENCY);
    if congruency != Some(DirectionCongruency::READING_DIRECTION) {
        return Err(LayoutError::Unsupported(
            "direction congruency other than reading direction".to_string(),
        ));
    }
    match graph.properties.enum_of::<Direction>(options::DIRECTION) {
        Some(Direction::DOWN) => {
            // `GraphTransformer.transposeAll`: the nodes, their ports, the edges' bend points
            // and labels, the graph's offset, size and padding, and the edge label side. Node
            // padding and label padding are not transposed: nodes have no padding of their own
            // here, and the label padding only matters with node labels.
            for node in &mut graph.nodes {
                std::mem::swap(&mut node.position.x, &mut node.position.y);
                std::mem::swap(&mut node.size.x, &mut node.size.y);
                if let Some(alignment) = node
                    .properties
                    .enum_of::<Alignment>(options::ALIGNMENT)
                    .and_then(transposed_alignment)
                {
                    node.properties.set(
                        options::ALIGNMENT,
                        Some(OptionValue::Enum {
                            class: "Alignment",
                            name: alignment.java_name().to_string(),
                        }),
                    );
                }
            }
            for port in &mut graph.ports {
                std::mem::swap(&mut port.position.x, &mut port.position.y);
                std::mem::swap(&mut port.anchor.x, &mut port.anchor.y);
                std::mem::swap(&mut port.size.x, &mut port.size.y);
                port.side = transposed_side(port.side);
            }
            for edge in &mut graph.edges {
                for point in &mut edge.bend_points {
                    std::mem::swap(&mut point.x, &mut point.y);
                }
                for label in &mut edge.labels {
                    std::mem::swap(&mut label.position.x, &mut label.position.y);
                    std::mem::swap(&mut label.size.x, &mut label.size.y);
                }
            }
            if let Some(selection) = graph
                .properties
                .enum_of::<EdgeLabelSideSelection>(options::EDGE_LABELS_SIDE_SELECTION)
            {
                graph.properties.set(
                    options::EDGE_LABELS_SIDE_SELECTION,
                    Some(OptionValue::Enum {
                        class: "EdgeLabelSideSelection",
                        name: transposed_selection(selection).java_name().to_string(),
                    }),
                );
            }
            std::mem::swap(&mut graph.offset.x, &mut graph.offset.y);
            std::mem::swap(&mut graph.size.x, &mut graph.size.y);
            graph.padding = transposed(graph.padding);
            Ok(())
        }
        Some(Direction::RIGHT) => Ok(()),
        other => Err(LayoutError::Unsupported(format!(
            "direction {other:?} (mirror transforms)"
        ))),
    }
}

/// `GraphTransformer.transposeProperties`: left and top alignments trade places, as do right
/// and bottom. The other alignments stay.
fn transposed_alignment(alignment: Alignment) -> Option<Alignment> {
    match alignment {
        Alignment::LEFT => Some(Alignment::TOP),
        Alignment::RIGHT => Some(Alignment::BOTTOM),
        Alignment::TOP => Some(Alignment::LEFT),
        Alignment::BOTTOM => Some(Alignment::RIGHT),
        _ => None,
    }
}

/// `GraphTransformer.transposePortSide`.
fn transposed_side(side: PortSide) -> PortSide {
    match side {
        PortSide::NORTH => PortSide::WEST,
        PortSide::WEST => PortSide::NORTH,
        PortSide::SOUTH => PortSide::EAST,
        PortSide::EAST => PortSide::SOUTH,
        PortSide::UNDEFINED => PortSide::UNDEFINED,
    }
}

/// `EdgeLabelSideSelection.transpose`.
fn transposed_selection(selection: EdgeLabelSideSelection) -> EdgeLabelSideSelection {
    match selection {
        EdgeLabelSideSelection::ALWAYS_UP => EdgeLabelSideSelection::ALWAYS_DOWN,
        EdgeLabelSideSelection::ALWAYS_DOWN => EdgeLabelSideSelection::ALWAYS_UP,
        EdgeLabelSideSelection::DIRECTION_UP => EdgeLabelSideSelection::DIRECTION_DOWN,
        EdgeLabelSideSelection::DIRECTION_DOWN => EdgeLabelSideSelection::DIRECTION_UP,
        EdgeLabelSideSelection::SMART_UP => EdgeLabelSideSelection::SMART_DOWN,
        EdgeLabelSideSelection::SMART_DOWN => EdgeLabelSideSelection::SMART_UP,
    }
}

/// `GraphTransformer.transpose(Spacing)`: the top and bottom sides move to the left and right.
fn transposed(spacing: Spacing) -> Spacing {
    Spacing {
        top: spacing.left,
        left: spacing.top,
        bottom: spacing.right,
        right: spacing.bottom,
    }
}

/// `SortByInputModelProcessor.process` for graphs without ports: each layer is sorted by
/// the nodes' model order, and the logs name the port lists and the layer.
fn sort_by_input_model(graph: &mut LGraph, progress: &mut ProgressMonitor, monitor: Monitor) {
    for layer_index in 0..graph.layers.len() {
        for &node in &graph.layers[layer_index] {
            // Nodes without ports (the importer rejects ports) keep an empty port list.
            progress.log(
                monitor,
                format!("Node {} ports: []", graph.nodes[node].text_name()),
            );
        }
        let mut sorted = graph.layers[layer_index].clone();
        let orders: Vec<usize> = graph
            .nodes
            .iter()
            .map(|node| node.model_order.unwrap_or(0))
            .collect();
        // Collections.sort: the GWT merge sort, so the comparator is observed in the same order.
        crate::java::sort::sort_by(&mut sorted, |&a, &b| orders[a].cmp(&orders[b]));
        graph.layers[layer_index] = sorted;
        let names: Vec<String> = graph.layers[layer_index]
            .iter()
            .map(|&node| graph.nodes[node].text_name())
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
}
