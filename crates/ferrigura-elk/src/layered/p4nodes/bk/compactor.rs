//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/bk/BKCompactor.java
//!
//! The horizontal compaction of the Brandes & Koepf placement, `BKCompactor.horizontalCompaction`: the
//! y coordinates of the blocks, and the shift of the classes towards each other.
//!
//! Java keeps the class graph in a `HashMap` keyed by sink and walks it with a queue. The walk visits the
//! classes in an order that does not change the shifts: each class is final when its predecessors are
//! done, and the shifts are minima or maxima over them. So the port keeps the classes in an `IndexMap`.

use std::collections::VecDeque;

use indexmap::IndexMap;

use crate::error::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::spacings;

use super::access::{java_max, java_min, lnode};
use super::aligned_layout::{AlignedLayout, HDirection, VDirection};
use super::neighborhood::NeighborhoodInformation;
use super::threshold::{ThresholdKind, ThresholdStrategy};

/// `BKCompactor.horizontalCompaction(bal)`: places the blocks, then shifts the classes, then gives
/// each node its block's y.
pub fn horizontal_compaction(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
    straightening: ThresholdKind,
) -> Result<(), LayoutError> {
    // Initialise the sinks and shifts, with the direction in mind.
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            bal.sink[id] = id;
            bal.shift[id] = if bal.vdir == Some(VDirection::Up) {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
    }

    let spacing_node_node = graph
        .properties
        .double_with_default(spacings::SPACING_NODE_NODE);
    let mut classes = ClassGraph::default();

    let mut layers: Vec<&Vec<usize>> = graph.layers.iter().collect();
    if bal.hdir == Some(HDirection::Left) {
        layers.reverse();
    }

    let mut threshold = ThresholdStrategy::new(straightening, ni.node_count);
    for y in bal.y.iter_mut() {
        *y = None;
    }

    for layer in &layers {
        let mut nodes: Vec<usize> = (*layer).clone();
        if bal.vdir == Some(VDirection::Up) {
            nodes.reverse();
        }
        // The initial placement of every block.
        for node in nodes {
            let v = graph.nodes[node].placement_id;
            if bal.root_of(v) == v {
                place_block(
                    graph,
                    ni,
                    bal,
                    &mut threshold,
                    &mut classes,
                    v,
                    spacing_node_node,
                );
            }
        }
    }

    // Shift the classes towards each other where there is space between them.
    place_classes(bal, &mut classes)?;

    // Apply the coordinates. A block root may be shifted with its class.
    for layer in &layers {
        for &node in layer.iter() {
            let v = graph.nodes[node].placement_id;
            let root = bal.root_of(v);
            bal.y[v] = bal.y[root];
            if v == root {
                let sink_shift = bal.shift[bal.sink[v]];
                let shifts = (bal.vdir == Some(VDirection::Up) && sink_shift > f64::NEG_INFINITY)
                    || (bal.vdir == Some(VDirection::Down) && sink_shift < f64::INFINITY);
                if shifts {
                    bal.y[v] = bal.y[v].map(|y| y + sink_shift);
                }
            }
        }
    }

    // All blocks are placed: shift the postponed ones.
    threshold.post_process(graph, ni, bal);
    Ok(())
}

/// `placeBlock(root, bal)`: places the block with root `root` below or above the blocks it touches,
/// after those blocks are placed themselves.
fn place_block(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
    threshold: &mut ThresholdStrategy,
    classes: &mut ClassGraph,
    root: usize,
    spacing_node_node: f64,
) {
    if bal.y[root].is_some() {
        return;
    }

    // The initial position is 0, and the first neighbour sets it outright. Without that, a minimum
    // with 0 would be wrong: the inside shift can make the positions negative for a bottom-up layout.
    let mut is_initial_assignment = true;
    bal.y[root] = Some(0.0);

    let mut current_node = root;
    let mut thresh = if bal.vdir == Some(VDirection::Down) {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    loop {
        let current_index = ni.node_index[current_node];
        let current = lnode(graph, ni, current_node);
        let layer_nodes = &graph.layers[current.layer.expect("a placed node has a layer")];
        let current_layer_size = layer_nodes.len();

        // The top (or bottom) node of its layer is placed safely: it is the first one there.
        let has_neighbor = (bal.vdir == Some(VDirection::Down) && current_index > 0)
            || (bal.vdir == Some(VDirection::Up) && current_index + 1 < current_layer_size);

        if has_neighbor {
            // The node above (or below) the current one, and the root of its block.
            let neighbor_arena = if bal.vdir == Some(VDirection::Up) {
                layer_nodes[current_index + 1]
            } else {
                layer_nodes[current_index - 1]
            };
            let neighbor_node = &graph.nodes[neighbor_arena];
            let neighbor = neighbor_node.placement_id;
            let neighbor_root = bal.root_of(neighbor);

            // Make sure the neighbour's block is placed.
            place_block(
                graph,
                ni,
                bal,
                threshold,
                classes,
                neighbor_root,
                spacing_node_node,
            );

            // The threshold has to be calculated after the neighbour is placed, or the order of the
            // postponed blocks would be wrong.
            thresh = threshold.calculate_threshold(graph, ni, bal, thresh, root, current_node);

            // The root and the neighbour's root form a class, unless the root already has a sink.
            if bal.sink[root] == root {
                bal.sink[root] = bal.sink[neighbor_root];
            }

            if bal.sink[root] == bal.sink[neighbor_root] {
                // Both blocks are in the same class: their distance is the spacing between the nodes.
                let spacing = spacings::vertical(graph, current, neighbor_node);

                if bal.vdir == Some(VDirection::Up) {
                    let current_block_position = bal.y_of(root);
                    let new_position = bal.y_of(neighbor_root) + bal.inner_shift[neighbor]
                        - neighbor_node.margin.top
                        - spacing
                        - current.margin.bottom
                        - current.size.y
                        - bal.inner_shift[current_node];

                    if is_initial_assignment {
                        is_initial_assignment = false;
                        bal.y[root] = Some(java_min(new_position, thresh));
                    } else {
                        bal.y[root] = Some(java_min(
                            current_block_position,
                            java_min(new_position, thresh),
                        ));
                    }
                } else {
                    // DOWN
                    let current_block_position = bal.y_of(root);
                    let new_position = bal.y_of(neighbor_root)
                        + bal.inner_shift[neighbor]
                        + neighbor_node.size.y
                        + neighbor_node.margin.bottom
                        + spacing
                        + current.margin.top
                        - bal.inner_shift[current_node];

                    if is_initial_assignment {
                        is_initial_assignment = false;
                        bal.y[root] = Some(java_max(new_position, thresh));
                    } else {
                        bal.y[root] = Some(java_max(
                            current_block_position,
                            java_max(new_position, thresh),
                        ));
                    }
                }
            } else {
                // Different classes: the required distance between the two sinks is recorded in the
                // class graph, and the classes are shifted in `place_classes`.
                let sink_node = bal.sink[root];
                let neighbor_sink = bal.sink[neighbor_root];
                classes.get_or_create(sink_node);
                classes.get_or_create(neighbor_sink);

                if bal.vdir == Some(VDirection::Up) {
                    // root --> currentNode, neighborRoot --> neighbor
                    let required_space = bal.y_of(root)
                        + bal.inner_shift[current_node]
                        + current.size.y
                        + current.margin.bottom
                        + spacing_node_node
                        - (bal.y_of(neighbor_root) + bal.inner_shift[neighbor]
                            - neighbor_node.margin.top);
                    classes.add_edge(sink_node, neighbor_sink, required_space);
                } else {
                    // DOWN: neighborRoot --> neighbor, root --> currentNode
                    let required_space = bal.y_of(root) + bal.inner_shift[current_node]
                        - current.margin.top
                        - bal.y_of(neighbor_root)
                        - bal.inner_shift[neighbor]
                        - neighbor_node.size.y
                        - neighbor_node.margin.bottom
                        - spacing_node_node;
                    classes.add_edge(sink_node, neighbor_sink, required_space);
                }
            }
        } else {
            thresh = threshold.calculate_threshold(graph, ni, bal, thresh, root, current_node);
        }

        // The next node of the block, until the ring is closed.
        current_node = bal.align[current_node];
        if current_node == root {
            break;
        }
    }

    threshold.finish_block(root);
}

/// `ClassNode`s of the class graph, by sink, in the order they were first seen.
#[derive(Debug, Default)]
struct ClassGraph {
    nodes: IndexMap<usize, ClassNode>,
}

/// A `ClassNode`: its shift, its outgoing `ClassEdge`s and its number of incoming ones.
#[derive(Debug, Default)]
struct ClassNode {
    class_shift: Option<f64>,
    outgoing: Vec<ClassEdge>,
    indegree: usize,
}

/// A `ClassEdge`: the separation required to the target class.
#[derive(Debug, Clone, Copy)]
struct ClassEdge {
    target: usize,
    separation: f64,
}

impl ClassGraph {
    /// `getOrCreateClassNode(sink)`.
    fn get_or_create(&mut self, sink: usize) {
        self.nodes.entry(sink).or_default();
    }

    /// `ClassNode.addEdge(target, separation)` on the source class.
    fn add_edge(&mut self, source: usize, target: usize, separation: f64) {
        self.nodes.entry(target).or_default().indegree += 1;
        self.nodes
            .entry(source)
            .or_default()
            .outgoing
            .push(ClassEdge { target, separation });
    }
}

/// `placeClasses(bal)`: the longest-path layering of the class graph gives each class its shift.
fn place_classes(bal: &mut AlignedLayout, classes: &mut ClassGraph) -> Result<(), LayoutError> {
    // The classes without incoming edges start the walk.
    let mut sinks: VecDeque<usize> = classes
        .nodes
        .iter()
        .filter(|(_, class)| class.indegree == 0)
        .map(|(&sink, _)| sink)
        .collect();

    while let Some(sink) = sinks.pop_front() {
        let class = classes
            .nodes
            .get_mut(&sink)
            .expect("a queued class is in the class graph");
        let class_shift = *class.class_shift.get_or_insert(0.0);
        let outgoing = class.outgoing.clone();

        for edge in outgoing {
            // The first class to reach a target sets its shift; later ones minimise or maximise it.
            let target = classes
                .nodes
                .get_mut(&edge.target)
                .expect("an edge target is in the class graph");
            let reached = class_shift + edge.separation;
            target.class_shift = Some(match target.class_shift {
                None => reached,
                Some(current) if bal.vdir == Some(VDirection::Down) => java_min(current, reached),
                Some(current) => java_max(current, reached),
            });
            target.indegree -= 1;
            if target.indegree == 0 {
                sinks.push_back(edge.target);
            }
        }
    }

    // Each class's shift applies to its sink. A class that the walk never reached is on a cycle,
    // which elkjs turns into a NullPointerException here.
    for (&sink, class) in &classes.nodes {
        bal.shift[sink] = class.class_shift.ok_or_else(|| {
            LayoutError::Format(
                "the class graph of node placement has a cycle, which elkjs fails on".to_string(),
            )
        })?;
    }
    Ok(())
}
