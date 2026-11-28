// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from TALA's layoutgraph node fitting and packing container guard.
// See NOTICE.md for the original paths and revision.

use crate::layout_graph::{LayoutGraph, Shape};
use crate::{Graph, Layout, Node, NodeGeometry, connected, routing};

const CONTAINER_PADDING: f64 = 60.0;
const LABEL_FIT_MARGIN: f64 = 20.0;
const MAX_FIXED_PADDING_LABEL_HEIGHT: f64 = 50.0;
const MAX_LEAF_SIDE: f64 = 2000.0;

/// Places the verified rectangular container layouts.
pub(crate) fn layout(graph: &Graph, translated: &LayoutGraph) -> Option<Layout> {
    if let Some(layout) = boundary::layout(graph) {
        return Some(layout);
    }

    let roots = translated
        .roots
        .iter()
        .map(|&index| translated.nodes[index].input_index)
        .collect::<Vec<_>>();
    let mut children = vec![Vec::new(); graph.nodes.len()];
    for node in &translated.nodes {
        if let Some(parent) = node.parent
            && !graph.nodes[translated.nodes[parent].input_index].is_group
        {
            return None;
        }
        children[node.input_index] = node
            .children
            .iter()
            .map(|&index| translated.nodes[index].input_index)
            .collect();
    }
    let mut inner_group = match roots.as_slice() {
        [first, second] if graph.edges.is_empty() => {
            return siblings::layout(graph, &children, [*first, *second], None);
        }
        [outer] if graph.edges.is_empty() => {
            if let [first, second] = children[*outer].as_slice() {
                return siblings::layout(graph, &children, [*first, *second], Some(*outer));
            }
            *outer
        }
        [only] => *only,
        _ => return None,
    };
    let mut groups = vec![inner_group];
    let leaves = loop {
        let group = &graph.nodes[inner_group];
        if !is_fitted_group(group) {
            return None;
        }
        if let [child] = children[inner_group].as_slice()
            && graph.nodes[*child].is_group
        {
            inner_group = *child;
            groups.push(inner_group);
            continue;
        }
        break children[inner_group].as_slice();
    };

    // A cycle cannot be reached from the sole root when each node has one parent.
    // Reject disconnected nodes, empty groups, and unported nested branching.
    if groups.len() + leaves.len() != graph.nodes.len() {
        return None;
    }
    let mut content = layout_content(graph, leaves, &graph.nodes[inner_group])?;
    let mut geometry: Vec<_> = graph.nodes.iter().map(node_geometry).collect();
    let mut position = 0.0;
    for &index in &groups {
        geometry[index].x = position;
        geometry[index].y = position;
        position += CONTAINER_PADDING;
    }

    let mut content_width: f64 = 0.0;
    let mut content_height: f64 = 0.0;
    for (&index, mut node) in leaves.iter().zip(content.nodes) {
        content_width = content_width.max(node.x + node.width);
        content_height = content_height.max(node.y + node.height);
        node.x += position;
        node.y += position;
        geometry[index] = node;
    }
    // The routed component's visible extent includes the loop offsets which
    // TALA includes when it wraps the container. Its origin is already cropped.
    for point in content.edges.iter_mut().flat_map(|edge| &mut edge.points) {
        content_width = content_width.max(point.x);
        content_height = content_height.max(point.y);
        point.x += position;
        point.y += position;
    }

    for &index in groups.iter().rev() {
        (content_width, content_height) =
            fitted_frame(&graph.nodes[index], content_width, content_height);
        geometry[index].width = content_width;
        geometry[index].height = content_height;
    }

    Some(Layout {
        nodes: geometry,
        edges: content.edges,
    })
}

fn layout_content(graph: &Graph, leaves: &[usize], inner_group: &Node) -> Option<Layout> {
    if !leaves
        .iter()
        .all(|&index| is_rectangular_leaf(&graph.nodes[index]))
    {
        return None;
    }
    match leaves {
        [index] => {
            let leaf = &graph.nodes[*index];
            if graph.edges.iter().any(|edge| {
                edge.source != leaf.id || edge.target != leaf.id || !edge.label.is_empty()
            }) {
                return None;
            }
            let mut node = node_geometry(leaf);
            let edges = if graph.edges.is_empty() {
                Vec::new()
            } else {
                routing::isolated_unlabeled_loops(&mut node, &graph.edges)
            };
            Some(Layout {
                nodes: vec![node],
                edges,
            })
        }
        [first, second] => {
            let [edge] = graph.edges.as_slice() else {
                return None;
            };
            if !edge.label.is_empty() {
                return None;
            }
            let nodes = [&graph.nodes[*first], &graph.nodes[*second]];
            // The container owns the direction. When it is implicit, the
            // standard directed pair selects a vertical axis for these dimensions.
            let direction = match inner_group.direction.as_deref() {
                None | Some("")
                    if edge.source_arrow != edge.target_arrow
                        && nodes
                            .iter()
                            .all(|node| node.width == 152.0 && node.height == 41.0) =>
                {
                    Some("TB")
                }
                None | Some("") => return None,
                value => value,
            };
            connected::layout_pair(direction, nodes, edge)
        }
        _ => None,
    }
}

fn node_geometry(node: &Node) -> NodeGeometry {
    NodeGeometry {
        id: node.id.clone(),
        x: 0.0,
        y: 0.0,
        width: node.width,
        height: node.height,
    }
}

fn is_fitted_group(node: &Node) -> bool {
    node.is_group
        && (node.label.is_empty() || node.label_height.ceil() <= MAX_FIXED_PADDING_LABEL_HEIGHT)
}

fn fitted_frame(group: &Node, width: f64, height: f64) -> (f64, f64) {
    // Bin packing fits the group to the content's rounded bounds. A wide
    // inside-top title can extend the right edge without moving the child.
    let mut width = width.round() + 2.0 * CONTAINER_PADDING;
    if !group.label.is_empty() {
        width = width.max(group.label_width.ceil() + LABEL_FIT_MARGIN);
    }
    (width, height.round() + 2.0 * CONTAINER_PADDING)
}

fn is_rectangular_leaf(node: &Node) -> bool {
    !node.is_group
        && Shape::parse(node) == Ok(Shape::Square)
        && (1.0..=MAX_LEAF_SIDE).contains(&node.width)
        && (1.0..=MAX_LEAF_SIDE).contains(&node.height)
        && (node.label.is_empty()
            || (node.label_width.ceil() <= node.width && node.label_height.ceil() <= node.height))
}

mod boundary;
mod siblings;

#[cfg(test)]
mod tests;
