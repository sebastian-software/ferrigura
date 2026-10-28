//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/GraphTransformer.java
//!
//! `GraphTransformer`: turns the graph between its input direction and the internal left-to-right
//! frame. A DOWN graph is transposed, a LEFT graph is mirrored across x, and an UP graph is
//! transposed and mirrored across y, in the order the mode asks for. Only the reading direction
//! congruency is ported, which is the default.
//!
//! The node labels are transposed and mirrored with their sizes, positions and placements, and the
//! graph's node label padding with the graph's padding. Not ported, because the graphs here do not
//! have them: node padding (nodes have none), fixed positions (no corpus graph sets one), and the port
//! indices that `reverseIndex` negates (ports carry none).

use crate::core::data::JavaEnum;
use crate::core::data::OptionValue;
use crate::core::data::enums::{
    Alignment, Direction, DirectionCongruency, EdgeLabelSideSelection, LayerConstraint,
    NodeLabelPlacement, NodeType, PortSide,
};
use crate::core::data::property::PropertyHolder;
use crate::core::math::{KVector, Spacing};
use crate::layered::LayoutError;
use crate::layered::lgraph::{InLayerConstraint, LGraph, LNode};
use crate::layered::options;

/// `GraphTransformer.Mode`: the direction preprocessor transforms towards the input direction
/// (`TO_INPUT_DIRECTION` in Java), and the postprocessor back to the internal frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    ToInternalLtr,
    ToInputDirection,
}

/// The axis a mirror runs across. A coordinate, a spacing side and a port side belong to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn coordinate(self, v: &KVector) -> f64 {
        match self {
            Axis::X => v.x,
            Axis::Y => v.y,
        }
    }

    fn coordinate_mut(self, v: &mut KVector) -> &mut f64 {
        match self {
            Axis::X => &mut v.x,
            Axis::Y => &mut v.y,
        }
    }

    /// The margin on the far side of the axis, `node.getMargin().right` or `.bottom`.
    fn far_margin(self, margin: &Spacing) -> f64 {
        match self {
            Axis::X => margin.right,
            Axis::Y => margin.bottom,
        }
    }

    /// `mirrorX(Spacing)` swaps left and right, `mirrorY(Spacing)` swaps top and bottom.
    fn mirror_spacing(self, spacing: &mut Spacing) {
        match self {
            Axis::X => std::mem::swap(&mut spacing.left, &mut spacing.right),
            Axis::Y => std::mem::swap(&mut spacing.top, &mut spacing.bottom),
        }
    }

    /// `getMirroredPortSideX` and `getMirroredPortSideY`.
    fn mirror_side(self, side: PortSide) -> PortSide {
        match (self, side) {
            (Axis::X, PortSide::EAST) => PortSide::WEST,
            (Axis::X, PortSide::WEST) => PortSide::EAST,
            (Axis::Y, PortSide::NORTH) => PortSide::SOUTH,
            (Axis::Y, PortSide::SOUTH) => PortSide::NORTH,
            _ => side,
        }
    }

    /// The alignment a mirror gives a node: left and right trade places across x, top and bottom
    /// across y. The other alignments stay.
    fn mirror_alignment(self, alignment: Alignment) -> Option<Alignment> {
        match (self, alignment) {
            (Axis::X, Alignment::LEFT) => Some(Alignment::RIGHT),
            (Axis::X, Alignment::RIGHT) => Some(Alignment::LEFT),
            (Axis::Y, Alignment::TOP) => Some(Alignment::BOTTOM),
            (Axis::Y, Alignment::BOTTOM) => Some(Alignment::TOP),
            _ => None,
        }
    }
}

/// `GraphTransformer.process` under `DirectionCongruency.READING_DIRECTION`.
pub fn transform(graph: &mut LGraph, mode: Mode) -> Result<(), LayoutError> {
    let congruency = graph
        .properties
        .enum_of::<DirectionCongruency>(options::DIRECTION_CONGRUENCY);
    if congruency != Some(DirectionCongruency::READING_DIRECTION) {
        return Err(LayoutError::Unsupported(
            "direction congruency other than reading direction".to_string(),
        ));
    }
    let direction = graph.properties.enum_of::<Direction>(options::DIRECTION);
    match direction {
        Some(Direction::LEFT) => mirror_all(graph, Axis::X),
        Some(Direction::DOWN) => transpose_all(graph),
        Some(Direction::UP) => {
            if mode == Mode::ToInternalLtr {
                transpose_all(graph);
                mirror_all(graph, Axis::Y);
            } else {
                mirror_all(graph, Axis::Y);
                transpose_all(graph);
            }
        }
        // The switch in Java has no case for RIGHT: the graph already runs left to right.
        Some(Direction::RIGHT) => {}
        other => {
            return Err(LayoutError::Unsupported(format!(
                "direction {other:?} in the graph transformation"
            )));
        }
    }
    Ok(())
}

/// `GraphTransformer.mirrorAllX` and `mirrorAllY`: the nodes, their ports, the edges' bend points,
/// junction points and labels, and the graph's padding are mirrored across the axis.
fn mirror_all(graph: &mut LGraph, axis: Axis) {
    let nodes: Vec<usize> = graph
        .layerless_nodes
        .iter()
        .copied()
        .chain(graph.layers.iter().flatten().copied())
        .collect();
    let offset = mirror_offset(graph, &nodes, axis);
    mirror_nodes(graph, &nodes, axis, offset);
    // Each label is mirrored once, however many lists hold it.
    for id in edge_label_ids(graph) {
        let label = &mut graph.labels[id];
        let label_size = axis.coordinate(&label.size);
        let position = axis.coordinate_mut(&mut label.position);
        *position = (offset - label_size) - *position;
    }
    axis.mirror_spacing(&mut graph.padding);
}

/// The line that the mirror runs across, `offset` in `GraphTransformer.mirrorX`. Before the layout
/// has a size it is the far edge of the nodes; afterwards it is the graph's extent.
fn mirror_offset(graph: &LGraph, nodes: &[usize], axis: Axis) -> f64 {
    let length = axis.coordinate(&graph.size);
    let graph_offset = axis.coordinate(&graph.offset);
    let mut offset = if length == 0.0 {
        nodes.iter().fold(0.0_f64, |offset, &node| {
            let data = &graph.nodes[node];
            offset.max(
                axis.coordinate(&data.position)
                    + axis.coordinate(&data.size)
                    + axis.far_margin(&data.margin),
            )
        })
    } else {
        length - graph_offset
    };
    offset -= graph_offset;
    offset
}

/// The ids of the labels that the edges hold, each once.
fn edge_label_ids(graph: &LGraph) -> std::collections::BTreeSet<usize> {
    graph
        .edges
        .iter()
        .flat_map(|edge| edge.labels.iter().copied())
        .collect()
}

/// `GraphTransformer.mirrorX(List<LNode>, LGraph)` and `mirrorY`, without the labels.
fn mirror_nodes(graph: &mut LGraph, nodes: &[usize], axis: Axis, offset: f64) {
    for &node in nodes {
        let node_size = axis.coordinate(&graph.nodes[node].size);
        {
            let position = axis.coordinate_mut(&mut graph.nodes[node].position);
            *position = (offset - node_size) - *position;
        }
        mirror_node_label_placement(&mut graph.nodes[node].properties, axis);
        if let Some(alignment) = graph.nodes[node]
            .properties
            .enum_of::<Alignment>(options::ALIGNMENT)
            .and_then(|alignment| axis.mirror_alignment(alignment))
        {
            graph.nodes[node].properties.set(
                options::ALIGNMENT,
                Some(OptionValue::Enum {
                    class: "Alignment",
                    name: alignment.java_name().to_string(),
                }),
            );
        }
        // `mirrorExternalPortSideX/Y` and `mirrorLayerConstraintX` / `mirrorInLayerConstraintY`.
        if graph.nodes[node].node_type == NodeType::EXTERNAL_PORT {
            mirror_external_port(&mut graph.nodes[node], axis);
        }
        for port in graph.nodes[node].ports.clone() {
            let port_size = axis.coordinate(&graph.ports[port].size);
            {
                let position = axis.coordinate_mut(&mut graph.ports[port].position);
                *position = (node_size - port_size) - *position;
            }
            {
                let anchor = axis.coordinate_mut(&mut graph.ports[port].anchor);
                *anchor = port_size - *anchor;
            }
            graph.ports[port].side = axis.mirror_side(graph.ports[port].side);
            for edge in graph.ports[port].outgoing.clone() {
                for point in &mut graph.edges[edge].bend_points {
                    let coordinate = axis.coordinate_mut(point);
                    *coordinate = offset - *coordinate;
                }
                if let Some(points) = graph.junction_points.get_mut(&edge) {
                    for point in points {
                        let coordinate = axis.coordinate_mut(point);
                        *coordinate = offset - *coordinate;
                    }
                }
            }
        }
        // The node labels: their placement, and their position within the node's far side.
        for label in graph.nodes[node].labels.clone() {
            let label_size = axis.coordinate(&graph.labels[label].size);
            mirror_node_label_placement(&mut graph.labels[label].properties, axis);
            let position = axis.coordinate_mut(&mut graph.labels[label].position);
            *position = (node_size - label_size) - *position;
        }
    }
}

/// `GraphTransformer.mirrorNodeLabelPlacementX` and `mirrorNodeLabelPlacementY`: a placement that names
/// the left side takes the right side, or the reverse, across x; the top and the bottom across y.
/// A placement that names neither side is kept.
fn mirror_node_label_placement(properties: &mut PropertyHolder, axis: Axis) {
    let (first, second) = match axis {
        Axis::X => (NodeLabelPlacement::H_LEFT, NodeLabelPlacement::H_RIGHT),
        Axis::Y => (NodeLabelPlacement::V_TOP, NodeLabelPlacement::V_BOTTOM),
    };
    let Some(OptionValue::EnumSet { class, names }) =
        properties.get(options::NODE_LABELS_PLACEMENT).cloned()
    else {
        return;
    };
    let (from, to) = if names.iter().any(|name| name == first.java_name()) {
        (first, second)
    } else if names.iter().any(|name| name == second.java_name()) {
        (second, first)
    } else {
        return;
    };
    let mut mirrored: Vec<String> = names
        .into_iter()
        .filter(|name| name != from.java_name())
        .collect();
    if !mirrored.iter().any(|name| name == to.java_name()) {
        mirrored.push(to.java_name().to_string());
    }
    properties.set(
        options::NODE_LABELS_PLACEMENT,
        Some(OptionValue::EnumSet {
            class,
            names: mirrored,
        }),
    );
}

/// `GraphTransformer.transposeAll`: the nodes, their ports, the edges' bend points, junction points
/// and labels, the graph's offset, size and padding, and the edge label side selection.
fn transpose_all(graph: &mut LGraph) {
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
        // `transposeProperties`: the minimum size trades its width and height.
        if let Some(OptionValue::Vector(minimum)) =
            node.properties.get(options::NODE_SIZE_MINIMUM).cloned()
        {
            node.properties.set(
                options::NODE_SIZE_MINIMUM,
                Some(OptionValue::Vector(KVector {
                    x: minimum.y,
                    y: minimum.x,
                })),
            );
        }
        transpose_node_label_placement(&mut node.properties);
        // `transposeExternalPortSide` and `transposeLayerConstraint`.
        if node.node_type == NodeType::EXTERNAL_PORT {
            transpose_external_port(node);
        }
        // `transposeAll(nodes)`: the node labels, with their sizes, positions and placements.
        for &id in &node.labels {
            let label = &mut graph.labels[id];
            std::mem::swap(&mut label.position.x, &mut label.position.y);
            std::mem::swap(&mut label.size.x, &mut label.size.y);
            transpose_node_label_placement(&mut label.properties);
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
    }
    // Each label is transposed once, however many lists hold it.
    for id in edge_label_ids(graph) {
        let label = &mut graph.labels[id];
        std::mem::swap(&mut label.position.x, &mut label.position.y);
        std::mem::swap(&mut label.size.x, &mut label.size.y);
    }
    for points in graph.junction_points.values_mut() {
        for point in points {
            std::mem::swap(&mut point.x, &mut point.y);
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
    // The graph's node label padding is transposed too. Unset, it is the default, which is the same
    // on every side, so there is nothing to do.
    if let Some(OptionValue::Padding(padding)) =
        graph.properties.get(options::NODE_LABELS_PADDING).cloned()
    {
        graph.properties.set(
            options::NODE_LABELS_PADDING,
            Some(OptionValue::Padding(transposed(padding))),
        );
    }
}

/// `mirrorExternalPortSideX` with `mirrorLayerConstraintX`, or `mirrorExternalPortSideY` with
/// `mirrorInLayerConstraintY`, for an external port dummy.
fn mirror_external_port(node: &mut LNode, axis: Axis) {
    if let Some(external) = node.external_port.as_mut() {
        external.side = external.side.map(|side| axis.mirror_side(side));
    }
    match axis {
        Axis::X => {
            let mirrored = match layer_constraint(node) {
                LayerConstraint::FIRST => Some(LayerConstraint::LAST),
                LayerConstraint::FIRST_SEPARATE => Some(LayerConstraint::LAST_SEPARATE),
                LayerConstraint::LAST => Some(LayerConstraint::FIRST),
                LayerConstraint::LAST_SEPARATE => Some(LayerConstraint::FIRST_SEPARATE),
                LayerConstraint::NONE => None,
            };
            if let Some(constraint) = mirrored {
                set_layer_constraint(node, constraint);
            }
        }
        Axis::Y => {
            node.in_layer_constraint = match node.in_layer_constraint {
                InLayerConstraint::TOP => InLayerConstraint::BOTTOM,
                InLayerConstraint::BOTTOM => InLayerConstraint::TOP,
                InLayerConstraint::NONE => InLayerConstraint::NONE,
            };
        }
    }
}

/// `transposeExternalPortSide` and `transposeLayerConstraint` for an external port dummy: a dummy in
/// the first or last separate layer goes to the top or bottom of its layer, and back.
fn transpose_external_port(node: &mut LNode) {
    if let Some(external) = node.external_port.as_mut() {
        external.side = external.side.map(transposed_side);
    }
    match (layer_constraint(node), node.in_layer_constraint) {
        (LayerConstraint::FIRST_SEPARATE, _) => {
            set_layer_constraint(node, LayerConstraint::NONE);
            node.in_layer_constraint = InLayerConstraint::TOP;
        }
        (LayerConstraint::LAST_SEPARATE, _) => {
            set_layer_constraint(node, LayerConstraint::NONE);
            node.in_layer_constraint = InLayerConstraint::BOTTOM;
        }
        (_, InLayerConstraint::TOP) => {
            set_layer_constraint(node, LayerConstraint::FIRST_SEPARATE);
            node.in_layer_constraint = InLayerConstraint::NONE;
        }
        (_, InLayerConstraint::BOTTOM) => {
            set_layer_constraint(node, LayerConstraint::LAST_SEPARATE);
            node.in_layer_constraint = InLayerConstraint::NONE;
        }
        _ => {}
    }
}

/// `LayeredOptions.LAYERING_LAYER_CONSTRAINT` of a node.
fn layer_constraint(node: &LNode) -> LayerConstraint {
    node.properties
        .enum_with_default::<LayerConstraint>(options::LAYER_CONSTRAINT)
}

fn set_layer_constraint(node: &mut LNode, constraint: LayerConstraint) {
    node.properties.set(
        options::LAYER_CONSTRAINT,
        Some(OptionValue::Enum {
            class: "LayerConstraint",
            name: constraint.java_name().to_string(),
        }),
    );
}

/// `GraphTransformer.transposeNodeLabelPlacement`: the horizontal position becomes the vertical one
/// and back, INSIDE and OUTSIDE stay, and H_PRIORITY is set when the placement did not have it. An
/// unset or empty placement is left alone.
pub(crate) fn transpose_node_label_placement(properties: &mut PropertyHolder) {
    let names = match properties.get(options::NODE_LABELS_PLACEMENT) {
        Some(OptionValue::EnumSet { names, .. }) if !names.is_empty() => names.clone(),
        _ => return,
    };
    let old: Vec<NodeLabelPlacement> = names
        .iter()
        .filter_map(|name| NodeLabelPlacement::from_java_text(name))
        .collect();
    let has = |placement: NodeLabelPlacement| old.contains(&placement);
    let mut new = Vec::new();
    new.push(if has(NodeLabelPlacement::INSIDE) {
        NodeLabelPlacement::INSIDE
    } else {
        NodeLabelPlacement::OUTSIDE
    });
    if !has(NodeLabelPlacement::H_PRIORITY) {
        new.push(NodeLabelPlacement::H_PRIORITY);
    }
    if has(NodeLabelPlacement::H_LEFT) {
        new.push(NodeLabelPlacement::V_TOP);
    } else if has(NodeLabelPlacement::H_CENTER) {
        new.push(NodeLabelPlacement::V_CENTER);
    } else if has(NodeLabelPlacement::H_RIGHT) {
        new.push(NodeLabelPlacement::V_BOTTOM);
    }
    if has(NodeLabelPlacement::V_TOP) {
        new.push(NodeLabelPlacement::H_LEFT);
    } else if has(NodeLabelPlacement::V_CENTER) {
        new.push(NodeLabelPlacement::H_CENTER);
    } else if has(NodeLabelPlacement::V_BOTTOM) {
        new.push(NodeLabelPlacement::H_RIGHT);
    }
    // An EnumSet iterates in the order of the constants.
    new.sort();
    properties.set(
        options::NODE_LABELS_PLACEMENT,
        Some(OptionValue::EnumSet {
            class: "NodeLabelPlacement",
            names: new
                .iter()
                .map(|placement| placement.java_name().to_string())
                .collect(),
        }),
    );
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
