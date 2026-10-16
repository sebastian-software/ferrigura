//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/bk/BKAligner.java
//!
//! The block building of the Brandes & Koepf placement, `BKAligner.verticalAlignment`, and the shift of
//! the nodes inside each block, `BKAligner.insideBlockShift`.

use indexmap::IndexSet;

use crate::core::data::enums::NodeType;
use crate::error::LayoutError;
use crate::layered::lgraph::LGraph;

use super::access::{java_max, lnode, name_of};
use super::aligned_layout::{AlignedLayout, HDirection, VDirection};
use super::neighborhood::{Neighbor, NeighborhoodInformation};
use super::{get_blocks, get_edge};

/// `BKAligner.verticalAlignment(bal, markedEdges)`: every node starts as its own block, and a node
/// joins the block of one of its neighbours in the previous layer, unless the edge to it is marked.
pub fn vertical_alignment(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
    marked_edges: &IndexSet<usize>,
) {
    for layer in &graph.layers {
        for &node in layer {
            let id = graph.nodes[node].placement_id;
            bal.root[id] = Some(id);
            bal.align[id] = id;
            bal.inner_shift[id] = 0.0;
        }
    }

    // If the horizontal direction is LEFT, the layers are traversed from right to left.
    let mut layers: Vec<&Vec<usize>> = graph.layers.iter().collect();
    if bal.hdir == Some(HDirection::Left) {
        layers.reverse();
    }

    for layer in layers {
        // `r` is the position in layer order where the last block was found, -1 for none.
        let mut r: i64 = -1;
        let mut nodes: Vec<usize> = layer.clone();
        if bal.vdir == Some(VDirection::Up) {
            // Upwards, the nodes of a layer are traversed in reverse, starting from INT_MAX.
            r = i64::from(i32::MAX);
            nodes.reverse();
        }

        for node in nodes {
            let v = graph.nodes[node].placement_id;
            let neighbors: &[Neighbor] = if bal.hdir == Some(HDirection::Left) {
                &ni.right_neighbors[v]
            } else {
                &ni.left_neighbors[v]
            };
            if neighbors.is_empty() {
                continue;
            }

            // Only the (two) middle neighbours of a node with many neighbours are considered.
            let d = neighbors.len() as f64;
            let low = ((d + 1.0) / 2.0).floor() as i64 - 1;
            let high = ((d + 1.0) / 2.0).ceil() as i64 - 1;
            let is_long = graph.nodes[node].node_type == NodeType::LONG_EDGE;

            if bal.vdir == Some(VDirection::Up) {
                for m in (low..=high).rev() {
                    let pair = neighbors[m as usize];
                    if bal.align[v] == v
                        && !marked_edges.contains(&pair.edge)
                        && r > ni.node_index[pair.node] as i64
                    {
                        join(bal, v, pair.node, is_long);
                        r = ni.node_index[pair.node] as i64;
                    }
                }
            } else {
                for m in low..=high {
                    let pair = neighbors[m as usize];
                    if bal.align[v] == v
                        && !marked_edges.contains(&pair.edge)
                        && r < ni.node_index[pair.node] as i64
                    {
                        join(bal, v, pair.node, is_long);
                        r = ni.node_index[pair.node] as i64;
                    }
                }
            }
        }
    }
}

/// The body of the joining branch of `verticalAlignment`: `v` takes the block of `u`, and the ring
/// of the block is closed through `v`.
fn join(bal: &mut AlignedLayout, v: usize, u: usize, is_long: bool) {
    bal.align[u] = v;
    bal.root[v] = bal.root[u];
    let root = bal.root_of(v);
    bal.align[v] = root;
    bal.od[root] = bal.od[root] && is_long;
}

/// `BKAligner.insideBlockShift(bal)`: moves the nodes of each block so that the ports of the edges
/// inside the block are level, and sets the size of each block.
pub fn inside_block_shift(
    graph: &LGraph,
    ni: &NeighborhoodInformation,
    bal: &mut AlignedLayout,
) -> Result<(), LayoutError> {
    // The keys of `getBlocks`, the block roots in layer order.
    let roots: Vec<usize> = get_blocks(graph, bal)
        .keys()
        .map(|root| root.expect("the block root is set by the vertical alignment"))
        .collect();
    for root in roots {
        // The top left corner of the root is at (0, 0). The space needed above it and below it
        // gives the block size, and each node's inner shift is measured from that corner.
        let root_node = lnode(graph, ni, root);
        let mut space_above = root_node.margin.top;
        let mut space_below = root_node.size.y + root_node.margin.bottom;
        bal.inner_shift[root] = 0.0;

        let mut current = root;
        loop {
            let next = bal.align[current];
            if next == root {
                break;
            }
            let edge = get_edge(graph, ni, current, next).ok_or_else(|| {
                LayoutError::Format(format!(
                    "node placement found no edge between {} and {}",
                    name_of(graph, ni, current),
                    name_of(graph, ni, next)
                ))
            })?;
            let source = &graph.ports[graph.edges[edge].source];
            let target = &graph.ports[graph.edges[edge].target];
            let port_pos_diff = if bal.hdir == Some(HDirection::Left) {
                target.position.y + target.anchor.y - source.position.y - source.anchor.y
            } else {
                source.position.y + source.anchor.y - target.position.y - target.anchor.y
            };

            let next_inner_shift = bal.inner_shift[current] + port_pos_diff;
            bal.inner_shift[next] = next_inner_shift;

            let next_node = lnode(graph, ni, next);
            space_above = java_max(space_above, next_node.margin.top - next_inner_shift);
            space_below = java_max(
                space_below,
                next_inner_shift + next_node.size.y + next_node.margin.bottom,
            );
            current = next;
        }

        // The inner shifts were relative to the top left corner: add the space above it.
        let mut current = root;
        loop {
            bal.inner_shift[current] += space_above;
            current = bal.align[current];
            if current == root {
                break;
            }
        }
        bal.block_size[root] = space_above + space_below;
    }
    Ok(())
}
