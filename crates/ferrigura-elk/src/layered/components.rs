//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/components/ComponentsProcessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/components/SimpleRowGraphPlacer.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/components/AbstractGraphPlacer.java
//!
//! The connected components of a flat level. `split` makes one layered graph per component, in the
//! order a depth-first search finds them. Each component is laid out on its own, and `combine` places
//! the laid-out components in rows, the way `SimpleRowGraphPlacer` does, and makes them the graph.
//!
//! A component is a copy of its nodes, their ports, their edges and their labels, in an arena of its
//! own. Its indices differ from the graph's, so the copy is made with [`transplant`], which both
//! `split` and `combine` use.
//!
//! Not ported: external ports (a component's port sides), the model order placers (the component
//! ordering option must be NONE), and the components compactor (COMPACTION_CONNECTED_COMPONENTS).

use std::collections::HashMap;

use crate::core::data::enums::GraphProperties;
use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::java::sort::sort_by;
use crate::layered::lgraph::{LGraph, LabelId};
use crate::layered::options;

/// `LayeredOptions.SEPARATE_CONNECTED_COMPONENTS`'s id, read through the options module.
const SEPARATE_CONNECTED_COMPONENTS: &str = options::SEPARATE_CONNECTED_COMPONENTS;
/// `CoreOptions.ASPECT_RATIO` (`org.eclipse.elk.aspectRatio`); its layered default is 1.6.
const ASPECT_RATIO: &str = "org.eclipse.elk.aspectRatio";
/// `CoreOptions.SPACING_COMPONENT_COMPONENT`, with its default of 20.
const SPACING_COMPONENT_COMPONENT: &str = "org.eclipse.elk.spacing.componentComponent";
/// `LayeredOptions.COMPACTION_CONNECTED_COMPONENTS`, which is off by default.
const COMPACTION_CONNECTED_COMPONENTS: &str =
    "org.eclipse.elk.layered.compaction.connectedComponents";
/// `CoreOptions.PRIORITY`, which is 0 unless set.
const PRIORITY: &str = "org.eclipse.elk.priority";

/// Whether the graph is split into its connected components: `separateConnectedComponents`, which
/// is on unless it is set to false.
pub fn separates(graph: &LGraph) -> bool {
    graph.properties.bool_of(SEPARATE_CONNECTED_COMPONENTS)
}

/// `ComponentsProcessor.split`: one graph per connected component of the graph's nodes, in the
/// order the depth-first search finds them, from the first node of the graph's layerless list.
/// The graph itself is not changed.
pub fn split(graph: &LGraph) -> Result<Vec<LGraph>, LayoutError> {
    if graph
        .graph_properties
        .contains(&GraphProperties::EXTERNAL_PORTS)
    {
        return Err(LayoutError::Unsupported(
            "connected components of a graph with external ports".to_string(),
        ));
    }
    let mut visited = vec![false; graph.nodes.len()];
    let mut components = Vec::new();
    for &start in &graph.layerless_nodes {
        let mut members = Vec::new();
        collect_component(graph, start, &mut visited, &mut members);
        // A node that an earlier search reached gives no component (Java's `dfs` returns null).
        if !members.is_empty() {
            components.push(component_of(graph, &members)?);
        }
    }
    Ok(components)
}

/// The depth-first search of `ComponentsProcessor.dfs`: the node, then the nodes its ports connect
/// to, port by port, predecessors before successors.
fn collect_component(graph: &LGraph, node: usize, visited: &mut [bool], members: &mut Vec<usize>) {
    if visited[node] {
        return;
    }
    visited[node] = true;
    members.push(node);
    for &port in &graph.nodes[node].ports {
        // `getConnectedPorts`: the sources of the incoming edges, then the targets of the outgoing.
        let connected = graph.ports[port]
            .incoming
            .iter()
            .map(|&edge| graph.edges[edge].source)
            .chain(
                graph.ports[port]
                    .outgoing
                    .iter()
                    .map(|&edge| graph.edges[edge].target),
            );
        for other in connected.collect::<Vec<usize>>() {
            collect_component(graph, graph.ports[other].node, visited, members);
        }
    }
}

/// A component of `graph`: the given nodes with what they hold, and the graph's properties. The
/// component has no random generator yet, which the caller hands over.
fn component_of(graph: &LGraph, members: &[usize]) -> Result<LGraph, LayoutError> {
    let mut component = LGraph {
        properties: graph.properties.clone(),
        graph_properties: graph.graph_properties.clone(),
        root_id: graph.root_id.clone(),
        processors: graph.processors.clone(),
        padding: graph.padding,
        ..LGraph::default()
    };
    // The minimum size belongs to the whole graph: `newGraph.setProperty(NODE_SIZE_MINIMUM, null)`.
    component.properties.set(options::NODE_SIZE_MINIMUM, None);
    transplant(graph, members, &mut component)?;
    Ok(component)
}

/// Copies the given nodes of `from`, with their ports, their edges, their labels and their junction
/// points, into `into`, appended in the order given. The copies are layerless. Returns the indices
/// of the copied nodes in `into`.
fn transplant(
    from: &LGraph,
    members: &[usize],
    into: &mut LGraph,
) -> Result<Vec<usize>, LayoutError> {
    let mut port_map: HashMap<usize, usize> = HashMap::new();
    let mut edge_map: HashMap<usize, usize> = HashMap::new();
    let mut label_map: HashMap<LabelId, LabelId> = HashMap::new();
    let mut new_nodes = Vec::with_capacity(members.len());
    for &old in members {
        let source = &from.nodes[old];
        let new_node = into.nodes.len();
        let mut node = source.clone();
        node.layer = None;
        node.ports = Vec::with_capacity(source.ports.len());
        node.labels = source
            .labels
            .iter()
            .map(|&label| copy_label(from, into, label, &mut label_map))
            .collect();
        // The fields of the layout's dummies are finished once the layout is; a node that is still
        // in the graph carries none of them.
        node.end_labels = None;
        node.long_edge_source = None;
        node.long_edge_target = None;
        node.origin_edge = None;
        node.represented_labels = Vec::new();
        for &old_port in &source.ports {
            let new_port = into.ports.len();
            let mut port = from.ports[old_port].clone();
            port.node = new_node;
            port.outgoing = Vec::new();
            port.incoming = Vec::new();
            into.ports.push(port);
            port_map.insert(old_port, new_port);
            node.ports.push(new_port);
        }
        into.nodes.push(node);
        new_nodes.push(new_node);
    }
    // The edges, from the outgoing lists of the ports. An edge is copied once, from its source port.
    for &old in members {
        for &old_port in &from.nodes[old].ports {
            for &old_edge in &from.ports[old_port].outgoing {
                if edge_map.contains_key(&old_edge) {
                    continue;
                }
                let source = &from.edges[old_edge];
                let (Some(&source_port), Some(&target_port)) =
                    (port_map.get(&source.source), port_map.get(&source.target))
                else {
                    return Err(LayoutError::Unsupported(
                        "an edge that leaves its connected component".to_string(),
                    ));
                };
                let new_edge = into.edges.len();
                let mut edge = source.clone();
                edge.source = source_port;
                edge.target = target_port;
                edge.labels = source
                    .labels
                    .iter()
                    .map(|&label| copy_label(from, into, label, &mut label_map))
                    .collect();
                if let Some(points) = from.junction_points.get(&old_edge) {
                    into.junction_points.insert(new_edge, points.clone());
                }
                into.edges.push(edge);
                edge_map.insert(old_edge, new_edge);
            }
        }
    }
    // The port lists, in their order, now that every edge has its copy.
    for &old in members {
        for &old_port in &from.nodes[old].ports {
            let new_port = *port_map.get(&old_port).ok_or_else(|| {
                LayoutError::Unsupported("a port outside its component".to_string())
            })?;
            let mut outgoing = Vec::with_capacity(from.ports[old_port].outgoing.len());
            for edge in &from.ports[old_port].outgoing {
                outgoing.push(mapped(&edge_map, *edge)?);
            }
            let mut incoming = Vec::with_capacity(from.ports[old_port].incoming.len());
            for edge in &from.ports[old_port].incoming {
                incoming.push(mapped(&edge_map, *edge)?);
            }
            into.ports[new_port].outgoing = outgoing;
            into.ports[new_port].incoming = incoming;
        }
    }
    into.layerless_nodes.extend(new_nodes.iter().copied());
    Ok(new_nodes)
}

/// The copy of a label, made once however many lists hold it.
fn copy_label(
    from: &LGraph,
    into: &mut LGraph,
    label: LabelId,
    copies: &mut HashMap<LabelId, LabelId>,
) -> LabelId {
    if let Some(&copy) = copies.get(&label) {
        return copy;
    }
    let copy = into.add_label(from.labels[label].clone());
    copies.insert(label, copy);
    copy
}

fn mapped(edge_map: &HashMap<usize, usize>, edge: usize) -> Result<usize, LayoutError> {
    edge_map.get(&edge).copied().ok_or_else(|| {
        LayoutError::Unsupported("an edge that leaves its connected component".to_string())
    })
}

/// `ComponentsProcessor.combine`: the laid-out components become the graph. One component is moved
/// as it is, with its offset folded into its positions; several are sorted and placed in rows.
/// The graph keeps its own offset, which is zero here.
pub fn combine(components: Vec<LGraph>, target: &mut LGraph) -> Result<(), LayoutError> {
    clear_arenas(target);
    let mut components = components;
    match components.len() {
        0 => {
            target.size = KVector::default();
            Ok(())
        }
        1 => {
            let mut source = components.remove(0);
            // `moveGraph(target, source, 0, 0)`: the nodes, bend points and labels take the offset.
            let offset = source.offset;
            offset_component(&mut source, offset.x, offset.y);
            let members = source.layerless_nodes.clone();
            transplant(&source, &members, target)?;
            // `target.copyProperties(source)`: the target keeps the keys the source does not have.
            target.properties.copy_from(&source.properties);
            target.graph_properties = source.graph_properties.clone();
            target.padding = source.padding;
            target.size = source.size;
            Ok(())
        }
        _ => combine_rows(components, target),
    }
}

/// `SimpleRowGraphPlacer.combine` for several components.
fn combine_rows(components: Vec<LGraph>, target: &mut LGraph) -> Result<(), LayoutError> {
    // `sortComponents`: the priority of the nodes, the largest first, then the smaller area first.
    // The sort is the Java's, which is stable, so equal components keep their order.
    let mut keyed: Vec<(i64, f64, LGraph)> = components
        .into_iter()
        .map(|graph| {
            let priority: i64 = graph
                .layerless_nodes
                .iter()
                .map(|&node| i64::from(graph.nodes[node].properties.int_of(PRIORITY).unwrap_or(0)))
                .sum();
            let area = graph.size.x * graph.size.y;
            (priority, area, graph)
        })
        .collect();
    sort_by(&mut keyed, |first, second| {
        let priority = second.0 - first.0;
        if priority == 0 {
            first.1.total_cmp(&second.1)
        } else {
            priority.cmp(&0)
        }
    });
    let mut components: Vec<LGraph> = keyed.into_iter().map(|(_, _, graph)| graph).collect();

    // The target takes the properties of the first component.
    target.properties.copy_from(&components[0].properties);
    target.graph_properties = components[0].graph_properties.clone();
    if components[0]
        .properties
        .bool_of(COMPACTION_CONNECTED_COMPONENTS)
    {
        return Err(LayoutError::Unsupported(
            "the compaction of connected components".to_string(),
        ));
    }

    let mut max_row_width = 0.0_f64;
    let mut total_area = 0.0_f64;
    for graph in &components {
        max_row_width = max_row_width.max(graph.size.x);
        total_area += graph.size.x * graph.size.y;
    }
    // `(float) Math.sqrt(totalArea) * ASPECT_RATIO`: the square root is a float before the product.
    let aspect_ratio = target.properties.double_with_default(ASPECT_RATIO);
    max_row_width = max_row_width.max(f64::from(total_area.sqrt() as f32) * aspect_ratio);
    let component_spacing = target
        .properties
        .double_with_default(SPACING_COMPONENT_COMPONENT);
    place_components(&mut components, target, max_row_width, component_spacing);

    for graph in &mut components {
        let members = graph.layerless_nodes.clone();
        transplant(&*graph, &members, target)?;
    }
    Ok(())
}

/// `SimpleRowGraphPlacer.placeComponents`: the components fill rows no wider than `max_row_width`,
/// and each moves to its place, with its offset. The graph's size is the rows' size.
fn place_components(
    components: &mut [LGraph],
    target: &mut LGraph,
    max_row_width: f64,
    component_spacing: f64,
) {
    let mut xpos = 0.0_f64;
    let mut ypos = 0.0_f64;
    let mut highest_box = 0.0_f64;
    let mut broadest_row = component_spacing;
    for graph in components.iter_mut() {
        let size = graph.size;
        if xpos + size.x > max_row_width {
            xpos = 0.0;
            ypos += highest_box + component_spacing;
            highest_box = 0.0;
        }
        let offset = graph.offset;
        offset_component(graph, xpos + offset.x, ypos + offset.y);
        graph.offset = KVector::default();
        broadest_row = broadest_row.max(xpos + size.x);
        highest_box = highest_box.max(size.y);
        xpos += size.x + component_spacing;
    }
    target.size = KVector {
        x: broadest_row,
        y: ypos + highest_box,
    };
}

/// `AbstractGraphPlacer.offsetGraph`: the nodes, the bend points, the junction points and the edge
/// labels of a graph move by the given amount. Node labels are relative to their node and stay.
fn offset_component(graph: &mut LGraph, dx: f64, dy: f64) {
    for node in graph.layerless_nodes.clone() {
        graph.nodes[node].position.x += dx;
        graph.nodes[node].position.y += dy;
        for port in graph.nodes[node].ports.clone() {
            for edge in graph.ports[port].outgoing.clone() {
                for point in &mut graph.edges[edge].bend_points {
                    point.x += dx;
                    point.y += dy;
                }
                if let Some(points) = graph.junction_points.get_mut(&edge) {
                    for point in points {
                        point.x += dx;
                        point.y += dy;
                    }
                }
                for label in graph.edges[edge].labels.clone() {
                    graph.labels[label].position.x += dx;
                    graph.labels[label].position.y += dy;
                }
            }
        }
    }
}

/// The arenas and the layerless list of the target, which the components are copied into.
fn clear_arenas(graph: &mut LGraph) {
    graph.nodes.clear();
    graph.ports.clear();
    graph.edges.clear();
    graph.labels.clear();
    graph.junction_points.clear();
    graph.layerless_nodes.clear();
    graph.layers.clear();
    graph.layer_sizes.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::PortSide;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    /// Two nodes joined by an edge, and a third node alone: two components, and the first one
    /// found is the pair, since the search starts at the first layerless node.
    fn two_components() -> LGraph {
        let mut graph = LGraph::default();
        for index in 0..3 {
            graph.nodes.push(LNode {
                id: format!("n{index}"),
                size: KVector { x: 10.0, y: 10.0 },
                ..LNode::default()
            });
            graph.layerless_nodes.push(index);
        }
        for (node, side) in [(0, PortSide::EAST), (1, PortSide::WEST)] {
            let port = graph.ports.len();
            graph.ports.push(LPort::new(node, side));
            graph.nodes[node].ports.push(port);
        }
        graph.edges.push(LEdge {
            id: "e".to_string(),
            source: 0,
            target: 1,
            ..LEdge::default()
        });
        graph.ports[0].outgoing.push(0);
        graph.ports[1].incoming.push(0);
        graph
    }

    /// The search gives the connected pair first, then the single node, each in its own arena.
    #[test]
    fn components_come_in_search_order_with_their_own_arenas() {
        let graph = two_components();
        let components = split(&graph).unwrap();
        assert_eq!(components.len(), 2);
        let ids: Vec<&str> = components[0]
            .layerless_nodes
            .iter()
            .map(|&node| components[0].nodes[node].id.as_str())
            .collect();
        assert_eq!(ids, vec!["n0", "n1"]);
        assert_eq!(components[0].edges.len(), 1);
        assert_eq!(components[0].edges[0].source, 0);
        assert_eq!(components[0].edges[0].target, 1);
        assert_eq!(components[1].layerless_nodes.len(), 1);
        assert!(components[1].edges.is_empty());
    }

    /// The components are sorted by area, the smaller first, and placed in rows. The total area of
    /// 700 gives a row width of sqrt(700) * 1.6, about 42.3, so the 30-wide pair starts a new row
    /// below the single node, after the spacing of 20.
    #[test]
    fn components_are_sorted_by_area_and_placed_in_rows() {
        let mut graph = two_components();
        graph.size = KVector::default();
        let mut components = split(&graph).unwrap();
        components[0].size = KVector { x: 30.0, y: 20.0 };
        components[1].size = KVector { x: 10.0, y: 10.0 };
        combine(components, &mut graph).unwrap();
        let position = |id: &str| {
            let node = graph
                .layerless_nodes
                .iter()
                .map(|&node| &graph.nodes[node])
                .find(|node| node.id == id)
                .unwrap();
            node.position
        };
        assert_eq!(position("n2"), KVector { x: 0.0, y: 0.0 });
        assert_eq!(position("n0"), KVector { x: 0.0, y: 30.0 });
        assert_eq!(position("n1"), KVector { x: 0.0, y: 30.0 });
        assert_eq!(graph.size, KVector { x: 30.0, y: 50.0 });
    }
}
