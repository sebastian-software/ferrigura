//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/EndLabelPreprocessor.java,
//!   EndLabelSorter.java and EndLabelPostprocessor.java
//!
//! The head and tail labels of edges. The preprocessor gathers the labels at each port into a label
//! cell beside the port, removes the overlaps between the cells on a side, and grows the node's
//! margins to hold them. The sorter orders the labels of a port by their edges, and the postprocessor
//! moves the cells to the node's position and lays the labels out in them.
//!
//! Not ported: port dummies, which only hierarchical ports have, and the port label management that
//! ports with labels of their own need. The importer rejects node labels.

use std::cmp::Ordering;

use crate::common::overlaps::{OverlapRemovalDirection, RectangleStripOverlapRemover};
use crate::core::data::enums::{Direction, EdgeLabelPlacement, NodeType, PortSide};
use crate::core::math::{ElkRectangle, KVector};
use crate::java::sort;
use crate::layered::labelcell::{HorizontalLabelAlignment, LabelCell, VerticalLabelAlignment};
use crate::layered::labels::{LabelSide, label_side};
use crate::layered::lgraph::{LGraph, LLabel, LabelId};
use crate::layered::options;

/// The thickness of a port without incident edges, `NO_INCIDENT_EDGE_THICKNESS`.
const NO_INCIDENT_EDGE_THICKNESS: f64 = -1.0;

/// `EndLabelPreprocessor.process`: the head and tail labels of each node's ports, in cells beside
/// the ports, with the margins of the nodes grown to hold them.
pub fn preprocess(graph: &mut LGraph) {
    let edge_label_spacing = graph
        .properties
        .double_with_default(options::SPACING_EDGE_LABEL);
    let label_label_spacing = graph
        .properties
        .double_with_default(options::SPACING_LABEL_LABEL);
    let vertical = is_vertical(graph);
    let nodes: Vec<usize> = graph.layers.iter().flatten().copied().collect();
    for node in nodes {
        process_node(
            graph,
            node,
            edge_label_spacing,
            label_label_spacing,
            vertical,
        );
    }
}

/// `EndLabelSorter.process`: the labels at a port with two or more edges that have end labels are
/// grouped by their edge, and the groups are ordered by the ports and nodes at their other ends.
pub fn sort(graph: &mut LGraph) {
    // `EndLabelSorter.initialize`: the ids the comparison orders by, given to nodes and ports in
    // the order of the layers.
    let mut node_ids = vec![0_usize; graph.nodes.len()];
    let mut port_ids = vec![0_usize; graph.ports.len()];
    let mut next_id = 0;
    for layer in &graph.layers {
        for &node in layer {
            node_ids[node] = next_id;
            next_id += 1;
            for &port in &graph.nodes[node].ports {
                port_ids[port] = next_id;
                next_id += 1;
            }
        }
    }
    let nodes: Vec<usize> = graph.layers.iter().flatten().copied().collect();
    for node in nodes {
        if graph.nodes[node].node_type != NodeType::NORMAL || graph.nodes[node].end_labels.is_none()
        {
            continue;
        }
        let ports = graph.nodes[node].ports.clone();
        for port in ports {
            if needs_sorting(graph, port) {
                sort_port_labels(graph, port, &node_ids, &port_ids);
            }
        }
    }
}

/// `EndLabelPostprocessor.process`: the cells move to their node's position, and the labels are
/// laid out in them. The labels are shared with the edges that hold them, so one write is seen by all.
pub fn postprocess(graph: &mut LGraph) {
    let nodes: Vec<usize> = graph.layers.iter().flatten().copied().collect();
    for node in nodes {
        let data = &graph.nodes[node];
        if !matches!(data.node_type, NodeType::NORMAL | NodeType::EXTERNAL_PORT)
            || data.end_labels.is_none()
        {
            continue;
        }
        let position = data.position;
        let cells = graph.nodes[node].end_labels.take().unwrap_or_default();
        for (_, mut cell) in cells {
            cell.rect.move_by(position);
            for (label, label_position) in cell.label_layout(graph) {
                graph.labels[label].position = label_position;
            }
        }
    }
}

fn is_vertical(graph: &LGraph) -> bool {
    matches!(
        graph.properties.enum_of::<Direction>(options::DIRECTION),
        Some(Direction::UP | Direction::DOWN)
    )
}

/// The placement of an edge label, `CENTER` when none is set.
fn placement(label: &LLabel) -> EdgeLabelPlacement {
    label
        .properties
        .enum_with_default::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT)
}

/// `EndLabelPreprocessor.processNode`.
fn process_node(
    graph: &mut LGraph,
    node: usize,
    edge_label_spacing: f64,
    label_label_spacing: f64,
    vertical: bool,
) {
    let ports = graph.nodes[node].ports.clone();
    let mut cells: Vec<Option<LabelCell>> = Vec::with_capacity(ports.len());
    for &port in &ports {
        let labels = gather_labels(graph, port);
        let cell = labels.and_then(|labels| {
            create_configured_label_cell(graph, &labels, label_label_spacing, vertical)
        });
        cells.push(cell);
    }
    place_labels(
        graph,
        node,
        &ports,
        &mut cells,
        label_label_spacing,
        edge_label_spacing,
        vertical,
    );
    let port_cells: Vec<(usize, LabelCell)> = ports
        .iter()
        .zip(&cells)
        .filter_map(|(&port, cell)| cell.clone().map(|cell| (port, cell)))
        .collect();
    if !port_cells.is_empty() {
        graph.nodes[node].end_labels = Some(port_cells);
        update_node_margins(graph, node, &cells);
    }
}

/// `EndLabelPreprocessor.createConfiguredLabelCell`: a cell holding the labels, sized to them.
fn create_configured_label_cell(
    graph: &LGraph,
    labels: &[LabelId],
    label_label_spacing: f64,
    vertical: bool,
) -> Option<LabelCell> {
    if labels.is_empty() {
        return None;
    }
    let mut cell = LabelCell::new(label_label_spacing, !vertical);
    for &label in labels {
        cell.add_label(graph, label);
    }
    cell.rect.height = cell.minimum_height();
    cell.rect.width = cell.minimum_width();
    Some(cell)
}

/// `EndLabelPreprocessor.gatherLabels(LPort)`: the labels of the port's edges that end at it, the
/// tail labels of the edges that leave it and the head labels of the edges that enter it. Also
/// sets the largest edge thickness at the port. `None` when the port has no incident edge.
fn gather_labels(graph: &mut LGraph, port: usize) -> Option<Vec<LabelId>> {
    let mut labels = Vec::new();
    let max_thickness = gather_incident_labels(graph, port, &mut labels);
    if !labels.is_empty() {
        graph.ports[port].max_edge_thickness = Some(max_thickness);
    }
    (max_thickness != NO_INCIDENT_EDGE_THICKNESS).then_some(labels)
}

/// `EndLabelPreprocessor.gatherLabels(LPort, List)`: appends the labels of the port's incident
/// edges (incoming first, as `LPort.getConnectedEdges` lists them) and returns the largest
/// thickness of those edges.
fn gather_incident_labels(graph: &mut LGraph, port: usize, target: &mut Vec<LabelId>) -> f64 {
    let mut max_thickness = NO_INCIDENT_EDGE_THICKNESS;
    let incident: Vec<usize> = graph.ports[port]
        .incoming
        .iter()
        .chain(&graph.ports[port].outgoing)
        .copied()
        .collect();
    for edge in incident {
        max_thickness = max_thickness.max(graph.edges[edge].thickness);
        let wanted = if graph.edges[edge].source == port {
            EdgeLabelPlacement::TAIL
        } else {
            EdgeLabelPlacement::HEAD
        };
        let labels: Vec<LabelId> = graph.edges[edge]
            .labels
            .iter()
            .copied()
            .filter(|&label| placement(&graph.labels[label]) == wanted)
            .collect();
        for &label in &labels {
            let end_label_edge = &mut graph.labels[label].end_label_edge;
            if end_label_edge.is_none() {
                *end_label_edge = Some(edge);
            }
        }
        target.extend(labels);
    }
    max_thickness
}

/// `EndLabelPreprocessor.placeLabels(LNode, ...)`: each cell beside its port, then the overlaps
/// removed on the two sides that the cells of the node's direction share.
fn place_labels(
    graph: &LGraph,
    node: usize,
    ports: &[usize],
    cells: &mut [Option<LabelCell>],
    label_label_spacing: f64,
    edge_label_spacing: f64,
    vertical: bool,
) {
    for (index, &port) in ports.iter().enumerate() {
        if let Some(cell) = cells[index].as_mut() {
            place_port_labels(graph, port, cell, edge_label_spacing);
        }
    }
    let (first, second) = if vertical {
        (PortSide::EAST, PortSide::WEST)
    } else {
        (PortSide::NORTH, PortSide::SOUTH)
    };
    for side in [first, second] {
        remove_label_overlaps(
            graph,
            node,
            ports,
            cells,
            side,
            2.0 * label_label_spacing,
            edge_label_spacing,
        );
    }
}

/// `EndLabelPreprocessor.placeLabels(LPort, LabelCell, double)`: the cell beside its port, on the
/// side the port is on, aligned towards the port's edges.
fn place_port_labels(graph: &LGraph, port: usize, cell: &mut LabelCell, edge_label_spacing: f64) {
    let port_data = &graph.ports[port];
    let node = &graph.nodes[port_data.node];
    let node_size = node.size;
    let margin = node.margin;
    let anchor = KVector {
        x: port_data.position.x + port_data.anchor.x,
        y: port_data.position.y + port_data.anchor.y,
    };
    let thickness = port_data.max_edge_thickness.unwrap_or(0.0);
    let mut rect = cell.rect;
    let above = label_side(&graph.labels[cell.labels[0]]) == LabelSide::ABOVE;
    match port_data.side {
        PortSide::NORTH => {
            cell.set_vertical_alignment(VerticalLabelAlignment::Bottom);
            rect.y = -margin.top - edge_label_spacing - rect.height;
            if above {
                cell.set_horizontal_alignment(HorizontalLabelAlignment::Right);
                rect.x = anchor.x - thickness - edge_label_spacing - rect.width;
            } else {
                cell.set_horizontal_alignment(HorizontalLabelAlignment::Left);
                rect.x = anchor.x + thickness + edge_label_spacing;
            }
        }
        PortSide::EAST => {
            cell.set_horizontal_alignment(HorizontalLabelAlignment::Left);
            rect.x = node_size.x + margin.right + edge_label_spacing;
            if above {
                cell.set_vertical_alignment(VerticalLabelAlignment::Bottom);
                rect.y = anchor.y - thickness - edge_label_spacing - rect.height;
            } else {
                cell.set_vertical_alignment(VerticalLabelAlignment::Top);
                rect.y = anchor.y + thickness + edge_label_spacing;
            }
        }
        PortSide::SOUTH => {
            cell.set_vertical_alignment(VerticalLabelAlignment::Top);
            rect.y = node_size.y + margin.bottom + edge_label_spacing;
            if above {
                cell.set_horizontal_alignment(HorizontalLabelAlignment::Right);
                rect.x = anchor.x - thickness - edge_label_spacing - rect.width;
            } else {
                cell.set_horizontal_alignment(HorizontalLabelAlignment::Left);
                rect.x = anchor.x + thickness + edge_label_spacing;
            }
        }
        PortSide::WEST => {
            cell.set_horizontal_alignment(HorizontalLabelAlignment::Right);
            rect.x = -margin.left - edge_label_spacing - rect.width;
            if above {
                cell.set_vertical_alignment(VerticalLabelAlignment::Bottom);
                rect.y = anchor.y - thickness - edge_label_spacing - rect.height;
            } else {
                cell.set_vertical_alignment(VerticalLabelAlignment::Top);
                rect.y = anchor.y + thickness + edge_label_spacing;
            }
        }
        PortSide::UNDEFINED => {}
    }
    cell.rect = rect;
}

/// `EndLabelPreprocessor.removeLabelOverlaps`: moves the cells on one side of the node apart.
fn remove_label_overlaps(
    graph: &LGraph,
    node: usize,
    ports: &[usize],
    cells: &mut [Option<LabelCell>],
    side: PortSide,
    label_label_spacing: f64,
    edge_label_spacing: f64,
) {
    let mut remover = RectangleStripOverlapRemover::create_for_direction(overlap_direction(side))
        .with_gap(label_label_spacing, label_label_spacing)
        .with_start_coordinate(overlap_start(graph, node, side, edge_label_spacing));
    let mut indices = Vec::new();
    for (index, &port) in ports.iter().enumerate() {
        if graph.ports[port].side == side
            && let Some(cell) = &cells[index]
        {
            remover.add_rectangle(cell.rect);
            indices.push(index);
        }
    }
    for (rect, index) in remover.remove_overlaps().into_iter().zip(indices) {
        if let Some(cell) = cells[index].as_mut() {
            cell.rect = rect;
        }
    }
}

/// `EndLabelPreprocessor.calculateOverlapStartCoordinate`.
fn overlap_start(graph: &LGraph, node: usize, side: PortSide, edge_label_spacing: f64) -> f64 {
    let data = &graph.nodes[node];
    match side {
        PortSide::NORTH => -data.margin.top - edge_label_spacing,
        PortSide::SOUTH => data.size.y + data.margin.bottom + edge_label_spacing,
        PortSide::EAST => data.size.x + data.margin.right + edge_label_spacing,
        PortSide::WEST => -data.margin.left - edge_label_spacing,
        PortSide::UNDEFINED => 0.0,
    }
}

/// `EndLabelPreprocessor.portSideToOverlapRemovalDirection`.
fn overlap_direction(side: PortSide) -> OverlapRemovalDirection {
    match side {
        PortSide::NORTH => OverlapRemovalDirection::Up,
        PortSide::SOUTH => OverlapRemovalDirection::Down,
        PortSide::EAST => OverlapRemovalDirection::Right,
        PortSide::WEST | PortSide::UNDEFINED => OverlapRemovalDirection::Left,
    }
}

/// `EndLabelPreprocessor.updateNodeMargins`: the margins grow to hold the node and its cells.
fn update_node_margins(graph: &mut LGraph, node: usize, cells: &[Option<LabelCell>]) {
    let data = &graph.nodes[node];
    let size = data.size;
    let mut margin = data.margin;
    let mut bounds = ElkRectangle::new(
        -margin.left,
        -margin.top,
        margin.left + size.x + margin.right,
        margin.top + size.y + margin.bottom,
    );
    for cell in cells.iter().flatten() {
        bounds.union(&cell.rect);
    }
    margin.left = -bounds.x;
    margin.top = -bounds.y;
    margin.right = bounds.width - margin.left - size.x;
    margin.bottom = bounds.height - margin.top - size.y;
    graph.nodes[node].margin = margin;
}

/// `EndLabelSorter.needsSorting`: at least two of the port's edges have end labels here.
fn needs_sorting(graph: &LGraph, port: usize) -> bool {
    let has_label = |edge: usize, wanted: EdgeLabelPlacement| {
        graph.edges[edge]
            .labels
            .iter()
            .any(|&label| placement(&graph.labels[label]) == wanted)
    };
    let head_edges = graph.ports[port]
        .incoming
        .iter()
        .filter(|&&edge| has_label(edge, EdgeLabelPlacement::HEAD))
        .count();
    let tail_edges = graph.ports[port]
        .outgoing
        .iter()
        .filter(|&&edge| has_label(edge, EdgeLabelPlacement::TAIL))
        .count();
    head_edges + tail_edges >= 2
}

/// `EndLabelSorter.sort`: the labels of the port's cell, grouped by edge and ordered by the groups'
/// comparator, then written back to the cell.
fn sort_port_labels(graph: &mut LGraph, port: usize, node_ids: &[usize], port_ids: &[usize]) {
    let node = graph.ports[port].node;
    let Some(index) = graph.nodes[node]
        .end_labels
        .as_ref()
        .and_then(|cells| cells.iter().position(|(cell_port, _)| *cell_port == port))
    else {
        return;
    };
    let labels = match graph.nodes[node].end_labels.as_ref() {
        Some(cells) => cells[index].1.labels.clone(),
        None => return,
    };
    let mut groups = label_groups(graph, &labels);
    {
        let view: &LGraph = graph;
        sort::sort_by(&mut groups, |a, b| {
            compare_groups(view, node_ids, port_ids, a, b)
        });
    }
    let ordered: Vec<LabelId> = groups.into_iter().flat_map(|(_, group)| group).collect();
    if let Some(cells) = graph.nodes[node].end_labels.as_mut() {
        cells[index].1.labels = ordered;
    }
}

/// `EndLabelSorter.createLabelGroups`: the labels by the edge they were gathered from, in order of
/// first appearance. A label that was never gathered has no edge, and is left out.
fn label_groups(graph: &LGraph, labels: &[LabelId]) -> Vec<(usize, Vec<LabelId>)> {
    let mut groups: Vec<(usize, Vec<LabelId>)> = Vec::new();
    for &label in labels {
        let Some(edge) = graph.labels[label].end_label_edge else {
            continue;
        };
        match groups
            .iter_mut()
            .find(|(group_edge, _)| *group_edge == edge)
        {
            Some((_, group)) => group.push(label),
            None => groups.push((edge, vec![label])),
        }
    }
    groups
}

/// `EndLabelSorter.LABEL_GROUP_COMPARATOR`: by the source port, then the target node, then the
/// target port in reverse.
fn compare_groups(
    graph: &LGraph,
    node_ids: &[usize],
    port_ids: &[usize],
    first: &(usize, Vec<LabelId>),
    second: &(usize, Vec<LabelId>),
) -> Ordering {
    let (edge1, edge2) = (&graph.edges[first.0], &graph.edges[second.0]);
    port_ids[edge1.source]
        .cmp(&port_ids[edge2.source])
        .then_with(|| {
            node_ids[graph.ports[edge1.target].node].cmp(&node_ids[graph.ports[edge2.target].node])
        })
        .then_with(|| port_ids[edge2.target].cmp(&port_ids[edge1.target]))
}
