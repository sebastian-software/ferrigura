//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/GraphTransformer.java
//!
//! `GraphTransformer`: turns the graph between its input direction and the internal left-to-right
//! frame. A DOWN graph is transposed, a LEFT graph is mirrored across x, and an UP graph is
//! transposed and mirrored across y, in the order the mode asks for. Only the reading direction
//! congruency is ported, which is the default.
//!
//! Not ported, because the graphs here do not have them: node labels and their padding (the
//! importer rejects node labels), node padding (nodes have none), fixed positions (no corpus graph
//! sets one), and the port indices that `reverseIndex` negates (ports carry none).

use crate::core::data::JavaEnum;
use crate::core::data::OptionValue;
use crate::core::data::enums::{
    Alignment, Direction, DirectionCongruency, EdgeLabelSideSelection, PortSide,
};
use crate::core::math::{KVector, Spacing};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
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
    match graph.properties.enum_of::<Direction>(options::DIRECTION) {
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
    mirror_nodes(graph, &nodes, axis);
    axis.mirror_spacing(&mut graph.padding);
}

/// `GraphTransformer.mirrorX(List<LNode>, LGraph)` and `mirrorY`.
fn mirror_nodes(graph: &mut LGraph, nodes: &[usize], axis: Axis) {
    // Before the layout has a size, the mirror line is the far edge of the nodes.
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

    for &node in nodes {
        let node_size = axis.coordinate(&graph.nodes[node].size);
        {
            let position = axis.coordinate_mut(&mut graph.nodes[node].position);
            *position = (offset - node_size) - *position;
        }
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
                for label in &mut graph.edges[edge].labels {
                    let label_size = axis.coordinate(&label.size);
                    let position = axis.coordinate_mut(&mut label.position);
                    *position = (offset - label_size) - *position;
                }
            }
        }
    }
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
