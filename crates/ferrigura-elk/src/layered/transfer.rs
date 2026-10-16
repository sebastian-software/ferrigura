//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphLayoutTransferrer.java
//!
//! `ElkGraphLayoutTransferrer.applyLayout` for the root graph: the node positions, the sizes of
//! nodes with size constraints, the inside node labels, the edges' routes and the root's size go
//! back into the ELK graph. Edge labels and junction points are not transferred yet.

use std::collections::HashMap;

use crate::core::data::OptionValue;
use crate::core::data::property::PropertyHolder;
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
    // The nodes of the graph are the root's children. They are matched by id, since the components
    // of a level may have been put back in another order.
    let node_of_id: HashMap<&str, usize> = graph
        .layerless_nodes
        .iter()
        .map(|&index| (graph.nodes[index].id.as_str(), index))
        .collect();
    for child in root.children.iter_mut() {
        let Some(&index) = node_of_id.get(child.id.as_str()) else {
            continue;
        };
        let node = &graph.nodes[index];
        child.shape.x = node.position.x + offset.x;
        child.shape.y = node.position.y + offset.y;
        // `applyNodeLayout`: a node with size constraints takes the size the layout gave it.
        if !child
            .properties
            .enum_set_is_empty(options::NODE_SIZE_CONSTRAINTS)
        {
            child.shape.width = node.size.x;
            child.shape.height = node.size.y;
        }
        // The inside labels, where the node or the label has a placement. They are relative to the
        // node, so the offset does not apply.
        let node_has_placement = !node
            .properties
            .enum_set_is_empty(options::NODE_LABELS_PLACEMENT);
        for &label_id in &node.labels {
            let llabel = &graph.labels[label_id];
            let label_has_placement = !llabel
                .properties
                .enum_set_is_empty(options::NODE_LABELS_PLACEMENT);
            if !(node_has_placement || label_has_placement) {
                continue;
            }
            if let Some(origin) = llabel.origin
                && let Some(elk_label) = child.labels.get_mut(origin)
            {
                elk_label.shape = Shape {
                    x: llabel.position.x,
                    y: llabel.position.y,
                    width: llabel.size.x,
                    height: llabel.size.y,
                };
            }
        }
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
                for &id in &graph.edges[edge].labels {
                    let label = &graph.labels[id];
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
    // `applyParentNodeLayout`: the node takes the actual size of its graph, unless its size is fixed.
    if !root.properties.bool_of(options::NODE_SIZE_FIXED_GRAPH_SIZE) {
        let actual = graph.actual_size();
        resize_node(root, actual.x, actual.y);
    }
    // Then the node's size is fixed (`SizeConstraint.fixed()`), whatever it was. The level above
    // reads this: a node laid out on its own is sized as it is, not by its labels again.
    root.properties.set(
        options::NODE_SIZE_CONSTRAINTS,
        Some(OptionValue::EnumSet {
            class: "SizeConstraint",
            names: Vec::new(),
        }),
    );
    Ok(())
}

/// `ElkUtil.resizeNode(node, width, height, movePorts, moveLabels)` without ports: the node becomes
/// at least `width` by `height` and at least its minimum size, and its labels move with the new
/// size as they keep their place in the node.
fn resize_node(node: &mut ElkNode, width: f64, height: f64) {
    let old = KVector {
        x: node.shape.width,
        y: node.shape.height,
    };
    let minimum = effective_minimum_size(&node.properties);
    let new = KVector {
        x: minimum.x.max(width),
        y: minimum.y.max(height),
    };
    let width_diff = new.x - old.x;
    let height_diff = new.y - old.y;
    node.shape.width = new.x;
    node.shape.height = new.y;
    for label in &mut node.labels {
        let mid_x = label.shape.x + label.shape.width / 2.0;
        let mid_y = label.shape.y + label.shape.height / 2.0;
        let width_percent = mid_x / old.x;
        let height_percent = mid_y / old.y;
        if width_percent + height_percent >= 1.0 {
            if width_percent - height_percent > 0.0 && mid_y >= 0.0 {
                // The label is on the right.
                label.shape.x += width_diff;
                label.shape.y += height_diff * height_percent;
            } else if width_percent - height_percent < 0.0 && mid_x >= 0.0 {
                // The label is on the bottom.
                label.shape.x += width_diff * width_percent;
                label.shape.y += height_diff;
            }
        }
    }
}

/// `ElkUtil.effectiveMinSizeConstraintFor`: the node's minimum size under MINIMUM_SIZE, with the
/// defaults of DEFAULT_MINIMUM_SIZE for a width or height that is not set. Zero otherwise.
pub fn effective_minimum_size(properties: &PropertyHolder) -> KVector {
    if !properties.enum_set_contains(options::NODE_SIZE_CONSTRAINTS, "MINIMUM_SIZE") {
        return KVector::default();
    }
    let mut minimum = match properties.value_or_default(options::NODE_SIZE_MINIMUM) {
        Some(OptionValue::Vector(vector)) => vector,
        _ => KVector::default(),
    };
    // Unset, `nodeSize.options` is `EnumSet.of(DEFAULT_MINIMUM_SIZE)`, which the registry's default
    // does not express.
    let default_minimum = match properties.get(options::NODE_SIZE_OPTIONS) {
        Some(OptionValue::EnumSet { names, .. }) => {
            names.iter().any(|name| name == "DEFAULT_MINIMUM_SIZE")
        }
        _ => true,
    };
    if default_minimum {
        // `ElkUtil.DEFAULT_MIN_WIDTH` and `DEFAULT_MIN_HEIGHT`.
        if minimum.x <= 0.0 {
            minimum.x = 20.0;
        }
        if minimum.y <= 0.0 {
            minimum.y = 20.0;
        }
    }
    minimum
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
            nodes: vec![
                LNode {
                    id: "a".to_string(),
                    ..LNode::default()
                },
                LNode {
                    id: "b".to_string(),
                    ..LNode::default()
                },
            ],
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
            nodes: vec![
                LNode {
                    id: "a".to_string(),
                    ..LNode::default()
                },
                LNode {
                    id: "b".to_string(),
                    ..LNode::default()
                },
            ],
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
        let label = graph.add_label(LLabel {
            size: KVector { x: 20.0, y: 10.0 },
            position: KVector { x: 3.0, y: 4.0 },
            origin: Some(0),
            ..LLabel::default()
        });
        graph.edges.push(LEdge {
            id: "e".to_string(),
            source: 0,
            target: 1,
            labels: vec![label],
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
