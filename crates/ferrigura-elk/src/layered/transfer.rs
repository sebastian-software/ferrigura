//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphLayoutTransferrer.java
//!
//! `ElkGraphLayoutTransferrer.applyLayout` for the root graph: the node positions, the
//! edges' routes and the root's size go back into the ELK graph. Labels and junction points
//! are not transferred yet.

use std::collections::HashMap;

use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::{ElkEdgeSection, ElkNode, Shape};
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// `applyLayout(lgraph)` for a root without nested graphs.
pub fn apply_layout(graph: &LGraph, root: &mut ElkNode) -> Result<(), LayoutError> {
    if root
        .properties
        .enum_set_contains(options::NODE_SIZE_OPTIONS, "COMPUTE_PADDING")
    {
        return Err(LayoutError::Unsupported(
            "elk.nodeSize.options COMPUTE_PADDING".to_string(),
        ));
    }
    // The nodes sit inside the padding, so the offset includes its top and left.
    let offset = KVector {
        x: graph.offset.x + graph.padding.left,
        y: graph.offset.y + graph.padding.top,
    };
    // The nodes of the graph are the root's children, in the same order.
    for (child, node) in root.children.iter_mut().zip(&graph.nodes) {
        child.shape.x = node.position.x + offset.x;
        child.shape.y = node.position.y + offset.y;
    }
    // `applyLayout`'s edge list: the outgoing edges of the nodes' ports, node by node.
    let mut edge_list = Vec::new();
    for &node in &graph.layerless_nodes {
        for &port in &graph.nodes[node].ports {
            edge_list.extend(graph.ports[port].outgoing.iter().copied());
        }
    }
    let mut sections = Vec::new();
    let mut label_shapes = Vec::new();
    {
        let index_of: HashMap<&str, usize> = root
            .edges
            .iter()
            .enumerate()
            .map(|(index, edge)| (edge.id.as_str(), index))
            .collect();
        for edge in edge_list {
            if let Some(&index) = index_of.get(graph.edges[edge].id.as_str()) {
                sections.push((index, edge_section(graph, edge, offset)));
                // `applyEdgeLayout`'s labels: each one's place, offset like the edge.
                for label in &graph.edges[edge].labels {
                    if let Some(origin) = label.origin {
                        label_shapes.push((
                            index,
                            origin,
                            Shape {
                                x: label.position.x + offset.x,
                                y: label.position.y + offset.y,
                                width: label.size.x,
                                height: label.size.y,
                            },
                        ));
                    }
                }
            }
        }
    }
    for (index, section) in sections {
        root.edges[index].sections = vec![section];
    }
    for (index, origin, shape) in label_shapes {
        if let Some(label) = root.edges[index].labels.get_mut(origin) {
            label.shape = shape;
        }
    }
    // `applyParentNodeLayout`: the root takes the actual size, unless its size is fixed.
    if !root.properties.bool_of(options::NODE_SIZE_FIXED_GRAPH_SIZE) {
        let actual = graph.actual_size();
        root.shape.width = actual.x;
        root.shape.height = actual.y;
    }
    Ok(())
}

/// `applyEdgeLayout`: an edge's route as one section, from the anchor of its source port to the
/// anchor of its target port, with its bend points between, all offset like the nodes.
fn edge_section(graph: &LGraph, edge: usize, offset: KVector) -> ElkEdgeSection {
    let ledge = &graph.edges[edge];
    let mut points = Vec::with_capacity(ledge.bend_points.len() + 2);
    points.push(absolute_anchor(graph, ledge.source));
    points.extend(ledge.bend_points.iter().copied());
    points.push(absolute_anchor(graph, ledge.target));
    for point in &mut points {
        point.x += offset.x;
        point.y += offset.y;
    }
    let start = points.remove(0);
    let end = points.pop().unwrap_or(start);
    ElkEdgeSection {
        start,
        end,
        bend_points: points,
    }
}

/// `LPort.getAbsoluteAnchor`: the node's position, plus the port's position and anchor.
fn absolute_anchor(graph: &LGraph, port: usize) -> KVector {
    let port = &graph.ports[port];
    let owner = graph.nodes[port.node].position;
    KVector {
        x: owner.x + port.position.x + port.anchor.x,
        y: owner.y + port.position.y + port.anchor.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ElkEdge;
    use crate::layered::lgraph::{LEdge, LNode, LPort};

    /// Two nodes `a` (at 10, 20) and `b` (at 30, 100), joined by an edge with one bend point.
    /// The source port is at (5, 40) of `a`, the target port at (5, 0) of `b`.
    #[test]
    fn a_section_runs_from_the_source_anchor_to_the_target_anchor() {
        let mut graph = LGraph {
            nodes: vec![LNode::default(), LNode::default()],
            layerless_nodes: vec![0, 1],
            ..LGraph::default()
        };
        graph.nodes[0].position = KVector { x: 10.0, y: 20.0 };
        graph.nodes[1].position = KVector { x: 30.0, y: 100.0 };
        graph.padding = crate::core::math::Spacing {
            top: 12.0,
            left: 12.0,
            bottom: 12.0,
            right: 12.0,
        };
        let mut source = LPort::new(0, crate::core::data::enums::PortSide::EAST);
        source.position = KVector { x: 5.0, y: 40.0 };
        let mut target = LPort::new(1, crate::core::data::enums::PortSide::WEST);
        target.position = KVector { x: 5.0, y: 0.0 };
        graph.ports = vec![source, target];
        graph.nodes[0].ports = vec![0];
        graph.nodes[1].ports = vec![1];
        graph.ports[0].outgoing.push(0);
        graph.ports[1].incoming.push(0);
        graph.edges.push(LEdge {
            id: "e".to_string(),
            source: 0,
            target: 1,
            bend_points: vec![KVector { x: 50.0, y: 60.0 }],
            ..LEdge::default()
        });
        let mut root = ElkNode {
            children: vec![
                ElkNode {
                    id: "a".to_string(),
                    ..ElkNode::default()
                },
                ElkNode {
                    id: "b".to_string(),
                    ..ElkNode::default()
                },
            ],
            edges: vec![ElkEdge {
                id: "e".to_string(),
                sources: vec!["a".to_string()],
                targets: vec!["b".to_string()],
                ..ElkEdge::default()
            }],
            ..ElkNode::default()
        };
        apply_layout(&graph, &mut root).unwrap();
        // The offset is the padding's top and left, 12 on each axis.
        assert_eq!(root.children[0].shape.x, 22.0);
        assert_eq!(root.children[0].shape.y, 32.0);
        let section = &root.edges[0].sections[0];
        assert_eq!(section.start, KVector { x: 27.0, y: 72.0 });
        assert_eq!(section.end, KVector { x: 47.0, y: 112.0 });
        assert_eq!(section.bend_points, vec![KVector { x: 62.0, y: 72.0 }]);
    }

    /// A label's place is offset like the edge, and it is written to its own ELK label.
    #[test]
    fn a_label_is_written_back_to_its_elk_label_offset_like_the_edge() {
        use crate::graph::ElkLabel;
        use crate::layered::lgraph::{LEdge, LLabel, LNode};
        let mut graph = LGraph {
            nodes: vec![LNode::default(), LNode::default()],
            layerless_nodes: vec![0, 1],
            ..LGraph::default()
        };
        graph.padding = crate::core::math::Spacing {
            top: 12.0,
            left: 12.0,
            bottom: 12.0,
            right: 12.0,
        };
        graph.ports = vec![
            LPort::new(0, crate::core::data::enums::PortSide::EAST),
            LPort::new(1, crate::core::data::enums::PortSide::WEST),
        ];
        graph.nodes[0].ports = vec![0];
        graph.nodes[1].ports = vec![1];
        graph.ports[0].outgoing.push(0);
        graph.ports[1].incoming.push(0);
        graph.edges.push(LEdge {
            id: "e".to_string(),
            source: 0,
            target: 1,
            labels: vec![LLabel {
                size: KVector { x: 20.0, y: 10.0 },
                position: KVector { x: 3.0, y: 4.0 },
                origin: Some(0),
                ..LLabel::default()
            }],
            ..LEdge::default()
        });
        let mut root = ElkNode {
            children: vec![
                ElkNode {
                    id: "a".to_string(),
                    ..ElkNode::default()
                },
                ElkNode {
                    id: "b".to_string(),
                    ..ElkNode::default()
                },
            ],
            edges: vec![ElkEdge {
                id: "e".to_string(),
                sources: vec!["a".to_string()],
                targets: vec!["b".to_string()],
                labels: vec![ElkLabel {
                    text: Some("x".to_string()),
                    ..ElkLabel::default()
                }],
                ..ElkEdge::default()
            }],
            ..ElkNode::default()
        };
        apply_layout(&graph, &mut root).unwrap();
        let shape = root.edges[0].labels[0].shape;
        assert_eq!(
            (shape.x, shape.y, shape.width, shape.height),
            (15.0, 16.0, 20.0, 10.0)
        );
    }
}
