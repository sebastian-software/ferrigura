//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/bk/ThresholdStrategy.java
//!
//! The threshold strategies of the Brandes & Koepf compaction. [`ThresholdKind::Null`] is
//! `NullThresholdStrategy`, the classic compaction, which has no effect. [`ThresholdKind::Simple`] is
//! `SimpleThresholdStrategy`, which is the default (`edgeStraightening` is `IMPROVE_STRAIGHTNESS`): it
//! bounds the position of the root and the last node of a block so that an edge can become straight,
//! and after all blocks are placed it moves the blocks that can still straighten an edge.
//!
//! Java keeps `bal`, `ni` and the finished blocks in fields that `init` resets. Here they are passed in,
//! and [`ThresholdStrategy::new`] is `init`.

use std::collections::VecDeque;

use crate::core::data::enums::EdgeStraighteningStrategy;
use crate::layered::lgraph::LGraph;
use crate::layered::options;

use super::access::{
    incoming_edges, is_in_layer_edge, lnode, name_of, outgoing_edges, port_node_id,
};
use super::aligned_layout::{AlignedLayout, HDirection, VDirection};
use super::neighborhood::NeighborhoodInformation;

/// `ThresholdStrategy.THRESHOLD`: `Double.MAX_VALUE`, the largest distance an edge is straightened by.
const THRESHOLD: f64 = f64::MAX;

/// Which threshold strategy the compactor uses, chosen by `nodePlacement.bk.edgeStraightening`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdKind {
    /// `NullThresholdStrategy`, for `NONE`.
    Null,
    /// `SimpleThresholdStrategy`, for `IMPROVE_STRAIGHTNESS`.
    Simple,
}

impl ThresholdKind {
    /// The strategy `BKCompactor`'s constructor picks for the graph.
    pub fn for_graph(graph: &LGraph) -> Self {
        match graph
            .properties
            .enum_of::<EdgeStraighteningStrategy>(options::NODE_PLACEMENT_BK_EDGE_STRAIGHTENING)
        {
            Some(EdgeStraighteningStrategy::IMPROVE_STRAIGHTNESS) => ThresholdKind::Simple,
            _ => ThresholdKind::Null,
        }
    }
}

/// `Postprocessable`: a block whose node `free` could be moved to straighten an edge.
#[derive(Debug, Clone, Copy)]
struct Postprocessable {
    /// The placement id of the node whose block could move.
    free: usize,
    /// Whether `free` is the root node of its block.
    is_root: bool,
    /// Whether `free` has edges that could be straightened.
    has_edges: bool,
    /// The edge that was picked to be straightened.
    edge: Option<usize>,
}

impl Postprocessable {
    fn new(free: usize, is_root: bool) -> Self {
        Postprocessable {
            free,
            is_root,
            has_edges: false,
            edge: None,
        }
    }
}

/// `ThresholdStrategy`, with its state: the finished blocks, and the postponed blocks in a queue and
/// a stack.
#[derive(Debug)]
pub struct ThresholdStrategy {
    kind: ThresholdKind,
    /// `blockFinished`: by root, the blocks that are completely placed.
    block_finished: Vec<bool>,
    /// `postProcessablesQueue`: postponed blocks, in the order they were postponed.
    queue: VecDeque<Postprocessable>,
    /// `postProcessablesStack`: postponed blocks that did not move, popped in reverse.
    stack: Vec<Postprocessable>,
}

impl ThresholdStrategy {
    /// `init`: a strategy with no finished blocks and nothing postponed.
    pub fn new(kind: ThresholdKind, node_count: usize) -> Self {
        ThresholdStrategy {
            kind,
            block_finished: vec![false; node_count],
            queue: VecDeque::new(),
            stack: Vec::new(),
        }
    }

    /// `finishBlock(n)`: marks the block with root `root` as completely placed.
    pub fn finish_block(&mut self, root: usize) {
        self.block_finished[root] = true;
    }

    /// `calculateThreshold(oldThresh, blockRoot, currentNode)`: the bound that lets an additional
    /// edge be straight, or the old value when the current node is neither the root nor the last one.
    pub fn calculate_threshold(
        &mut self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        bal: &mut AlignedLayout,
        old_thresh: f64,
        block_root: usize,
        current_node: usize,
    ) -> f64 {
        match self.kind {
            ThresholdKind::Null => {
                // A threshold of +infinity (UP) or -infinity (DOWN) has no effect on min or max.
                if bal.vdir == Some(VDirection::Up) {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                }
            }
            ThresholdKind::Simple => {
                // Only the root and the last node of a block. A block of one node is both.
                let is_root = block_root == current_node;
                let is_last = bal.align[current_node] == block_root;
                if !(is_root || is_last) {
                    return old_thresh;
                }
                // Java has the same two branches for RIGHT and LEFT here, so they are one.
                let mut t = old_thresh;
                if is_root {
                    t = self.get_bound(graph, ni, bal, block_root, true);
                }
                if t.is_infinite() && is_last {
                    t = self.get_bound(graph, ni, bal, current_node, false);
                }
                t
            }
        }
    }

    /// `postProcess()`: moves the postponed blocks so that their edges become straight, first in
    /// the order they were postponed, then the rest in reverse.
    pub fn post_process(
        &mut self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        bal: &mut AlignedLayout,
    ) {
        if self.kind == ThresholdKind::Null {
            return;
        }
        while let Some(pp) = self.queue.pop_front() {
            let pick = self.pick_edge(graph, ni, bal, pp);
            let Some(edge) = pick.edge else {
                continue;
            };
            // Ignore in-layer edges, unless the block consists of dummy nodes only.
            let only_dummies = bal.od[bal.root_of(pick.free)];
            if !only_dummies && is_in_layer_edge(graph, edge) {
                continue;
            }
            // Try to straighten the edge now. If that is not possible, try again later.
            let moved = self.process(graph, ni, bal, &pick);
            if !moved {
                self.stack.push(pick);
            }
        }
        while let Some(pp) = self.stack.pop() {
            self.process(graph, ni, bal, &pp);
        }
    }

    /// `pickEdge(pp)`: the first edge of `free` whose other block is finished and that the block may
    /// still move for. `has_edges` tells whether any edge was a candidate at all.
    fn pick_edge(
        &self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        bal: &AlignedLayout,
        mut pp: Postprocessable,
    ) -> Postprocessable {
        let free = lnode(graph, ni, pp.free);
        let edges = if pp.is_root {
            if bal.hdir == Some(HDirection::Right) {
                incoming_edges(graph, free)
            } else {
                outgoing_edges(graph, free)
            }
        } else if bal.hdir == Some(HDirection::Left) {
            incoming_edges(graph, free)
        } else {
            outgoing_edges(graph, free)
        };

        let mut has_edges = false;
        for edge in edges {
            // In-layer edges are ignored, unless the block is solely made of dummy nodes. With
            // self-loops and feedback edges a block can have no other connections, and then it may
            // still be moved to shorten those edges.
            let only_dummies = bal.od[bal.root_of(pp.free)];
            if !only_dummies && is_in_layer_edge(graph, edge) {
                continue;
            }

            // The block is part of a straightened edge already, so it cannot be moved again. Java
            // tests this flag twice in one condition; one test gives the same result.
            if bal.su[bal.root_of(pp.free)] {
                continue;
            }

            has_edges = true;

            // A node whose other block has no position yet cannot be used.
            let other = get_other(graph, ni, edge, pp.free);
            if self.block_finished[bal.root_of(other)] {
                pp.has_edges = true;
                pp.edge = Some(edge);
                return pp;
            }
        }

        pp.has_edges = has_edges;
        pp.edge = None;
        pp
    }

    /// `getBound(blockNode, isRoot)`: the threshold that the edge picked for the block gives, or an
    /// infinity when there is no such edge. A block that has edges but no usable one is postponed.
    fn get_bound(
        &mut self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        bal: &mut AlignedLayout,
        block_node: usize,
        is_root: bool,
    ) -> f64 {
        let invalid = if bal.vdir == Some(VDirection::Up) {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };

        let pick = self.pick_edge(graph, ni, bal, Postprocessable::new(block_node, is_root));
        match (pick.edge, pick.has_edges) {
            (None, true) => {
                self.queue.push_back(pick);
                invalid
            }
            (None, false) => invalid,
            (Some(edge), _) => {
                let left = graph.edges[edge].source;
                let right = graph.edges[edge].target;

                let threshold = if is_root {
                    // The root (first) node of the block.
                    let (root_port, other_port) = if bal.hdir == Some(HDirection::Right) {
                        (right, left)
                    } else {
                        (left, right)
                    };
                    let other_node = port_node_id(graph, other_port);
                    let other_root = bal.root_of(other_node);
                    bal.y_of(other_root)
                        + bal.inner_shift[other_node]
                        + graph.ports[other_port].position.y
                        + graph.ports[other_port].anchor.y
                        // the root node
                        - bal.inner_shift[port_node_id(graph, root_port)]
                        - graph.ports[root_port].position.y
                        - graph.ports[root_port].anchor.y
                } else {
                    // The last node of the block.
                    let (root_port, other_port) = if bal.hdir == Some(HDirection::Left) {
                        (right, left)
                    } else {
                        (left, right)
                    };
                    let other_node = port_node_id(graph, other_port);
                    bal.y_of(bal.root_of(other_node))
                        + bal.inner_shift[other_node]
                        + graph.ports[other_port].position.y
                        + graph.ports[other_port].anchor.y
                        // the root node
                        - bal.inner_shift[port_node_id(graph, root_port)]
                        - graph.ports[root_port].position.y
                        - graph.ports[root_port].anchor.y
                };

                // Neither block may move again to straighten another edge.
                let left_root = bal.root_of(port_node_id(graph, left));
                let right_root = bal.root_of(port_node_id(graph, right));
                bal.su[left_root] = true;
                bal.su[right_root] = true;
                threshold
            }
        }
    }

    /// `process(pp)`: moves the block of the other end of the picked edge until the edge is straight,
    /// as far as the neighbouring blocks allow. Returns whether the block moved at all.
    fn process(
        &self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        bal: &mut AlignedLayout,
        pp: &Postprocessable,
    ) -> bool {
        let edge = pp
            .edge
            .expect("a postponed block has an edge to straighten");
        let source = graph.edges[edge].source;
        let target = graph.edges[edge].target;
        let free_arena = ni.node_of_id[pp.free];
        let (fix, block) = if graph.ports[source].node == free_arena {
            (target, source)
        } else {
            (source, target)
        };

        // The target of the block must be the root of a different block.
        let delta = bal.calculate_delta(graph, fix, block);
        let block_node = port_node_id(graph, block);

        if delta > 0.0 && delta < THRESHOLD {
            // The target lies below the source: shift the block upwards.
            let available_space = bal.check_space_above(graph, ni, block_node, delta);
            bal.shift_block(block_node, -available_space);
            available_space > 0.0
        } else if delta < 0.0 && -delta < THRESHOLD {
            // The block was shifted too far up for the edge to be straight: shift it down again.
            let available_space = bal.check_space_below(graph, ni, block_node, -delta);
            bal.shift_block(block_node, available_space);
            available_space > 0.0
        } else {
            false
        }
    }
}

/// `getOther(edge, n)`: the node at the other end of `edge` from `n`.
fn get_other(graph: &LGraph, ni: &NeighborhoodInformation, edge: usize, n: usize) -> usize {
    let n_arena = ni.node_of_id[n];
    let source = graph.ports[graph.edges[edge].source].node;
    let target = graph.ports[graph.edges[edge].target].node;
    if source == n_arena {
        graph.nodes[target].placement_id
    } else if target == n_arena {
        graph.nodes[source].placement_id
    } else {
        // Java throws IllegalArgumentException here. Only edges of `n` are passed in, so this
        // cannot be reached.
        panic!(
            "node {} is neither source nor target of edge {}",
            name_of(graph, ni, n),
            edge
        );
    }
}
