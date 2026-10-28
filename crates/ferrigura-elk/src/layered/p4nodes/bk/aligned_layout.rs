//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/bk/BKAlignedLayout.java
//!
//! One of the four candidate placements of the Brandes & Koepf node placement, with the directions it
//! was built in. The Java arrays indexed by `LNode.id` are vectors indexed by placement id, and a
//! `LNode` stored in them is its placement id.
//!
//! The balanced layout has no directions and no block roots. Its `root` entries stay `None`, as the
//! `null` entries of Java do. `y` is `None` until a node is placed; the other arrays are plain values,
//! which Java only reads after it has set them.

use crate::layered::lgraph::LGraph;
use crate::layered::spacings;

use super::access::{java_max, java_min, lnode, port_node_id};
use super::neighborhood::NeighborhoodInformation;

/// `BKAlignedLayout.VDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VDirection {
    /// Iteration direction top-down.
    Down,
    /// Iteration direction bottom-up.
    Up,
}

/// `BKAlignedLayout.HDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HDirection {
    /// Iterating from right to left.
    Right,
    /// Iterating from left to right.
    Left,
}

/// `BKAlignedLayout`.
#[derive(Debug, Clone)]
pub struct AlignedLayout {
    /// `root`: the root of each node's block.
    pub root: Vec<Option<usize>>,
    /// `blockSize`: the size of a block, by its root.
    pub block_size: Vec<f64>,
    /// `align`: the next node of the block, the first node again after the last, a ring.
    pub align: Vec<usize>,
    /// `innerShift`: the shift that keeps a node straight inside its block.
    pub inner_shift: Vec<f64>,
    /// `sink`: the root of the class a block belongs to, by block root.
    pub sink: Vec<usize>,
    /// `shift`: the shift of a class, by its sink.
    pub shift: Vec<f64>,
    /// `y`: the y coordinate of each node, `None` until it is placed.
    pub y: Vec<Option<f64>>,
    /// `vdir`: the vertical direction, `None` for the balanced layout.
    pub vdir: Option<VDirection>,
    /// `hdir`: the horizontal direction, `None` for the balanced layout.
    pub hdir: Option<HDirection>,
    /// `su`: blocks, by root, that are part of a straightened edge.
    pub su: Vec<bool>,
    /// `od`: blocks, by root, that consist of dummy nodes only.
    pub od: Vec<bool>,
}

impl AlignedLayout {
    /// The `BKAlignedLayout` constructor: `su` is all false and `od` all true, as in Java.
    pub fn new(node_count: usize, vdir: Option<VDirection>, hdir: Option<HDirection>) -> Self {
        AlignedLayout {
            root: vec![None; node_count],
            block_size: vec![0.0; node_count],
            align: vec![0; node_count],
            inner_shift: vec![0.0; node_count],
            sink: vec![0; node_count],
            shift: vec![0.0; node_count],
            y: vec![None; node_count],
            vdir,
            hdir,
            su: vec![false; node_count],
            od: vec![true; node_count],
        }
    }

    /// `root[n.id]`. Only the balanced layout has no roots, and nothing reads them from there.
    pub fn root_of(&self, id: usize) -> usize {
        self.root[id].expect("the block root is set by the vertical alignment")
    }

    /// `y[n.id]`, read once the node is placed.
    pub fn y_of(&self, id: usize) -> f64 {
        self.y[id].expect("the node is placed before its y is read")
    }

    /// `layoutSize()`: the extent from the top of the highest node to the bottom of the lowest block.
    pub fn layout_size(&self, graph: &LGraph) -> f64 {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for layer in &graph.layers {
            for &node in layer {
                let id = graph.nodes[node].placement_id;
                let y_min = self.y_of(id);
                let y_max = y_min + self.block_size[self.root_of(id)];
                min = java_min(min, y_min);
                max = java_max(max, y_max);
            }
        }
        max - min
    }

    /// `toString()`: the horizontal and vertical direction names, or `BALANCED`.
    pub fn name(&self) -> String {
        let mut result = String::new();
        match self.hdir {
            Some(HDirection::Right) => result.push_str("RIGHT"),
            Some(HDirection::Left) => result.push_str("LEFT"),
            None => {}
        }
        match self.vdir {
            Some(VDirection::Down) => result.push_str("DOWN"),
            Some(VDirection::Up) => result.push_str("UP"),
            None => result.push_str("BALANCED"),
        }
        result
    }

    /// `calculateDelta(src, tgt)`: how far the target port lies below the source port. Both ports
    /// are indices into `LGraph::ports`.
    pub fn calculate_delta(&self, graph: &LGraph, src: usize, tgt: usize) -> f64 {
        let src_port = &graph.ports[src];
        let tgt_port = &graph.ports[tgt];
        let src_id = port_node_id(graph, src);
        let tgt_id = port_node_id(graph, tgt);
        let src_pos =
            self.y_of(src_id) + self.inner_shift[src_id] + src_port.position.y + src_port.anchor.y;
        let tgt_pos =
            self.y_of(tgt_id) + self.inner_shift[tgt_id] + tgt_port.position.y + tgt_port.anchor.y;
        tgt_pos - src_pos
    }

    /// `shiftBlock(rootNode, delta)`: moves every node of the block by `delta`.
    pub fn shift_block(&mut self, root_node: usize, delta: f64) {
        let mut current = root_node;
        loop {
            let new_pos = self.y_of(current) + delta;
            self.y[current] = Some(new_pos);
            current = self.align[current];
            if current == root_node {
                break;
            }
        }
    }

    /// `checkSpaceAbove(blockRoot, delta, ni)`: how far the block can move up by at most `delta`
    /// without overlapping the nodes above its nodes.
    pub fn check_space_above(
        &self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        block_root: usize,
        delta: f64,
    ) -> f64 {
        let mut available_space = delta;
        let root_node = block_root;
        let mut current = root_node;
        loop {
            current = self.align[current];
            let min_y_current = self.min_y(graph, ni, current);
            if let Some(neighbor) = self.upper_neighbor(graph, ni, current) {
                let max_y_neighbor = self.max_y(graph, ni, neighbor);
                let spacing = spacings::vertical(
                    graph,
                    lnode(graph, ni, current),
                    lnode(graph, ni, neighbor),
                );
                available_space =
                    java_min(available_space, min_y_current - (max_y_neighbor + spacing));
            }
            if root_node == current {
                break;
            }
        }
        available_space
    }

    /// `checkSpaceBelow(blockRoot, delta, ni)`: how far the block can move down by at most `delta`
    /// without overlapping the nodes below its nodes.
    pub fn check_space_below(
        &self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        block_root: usize,
        delta: f64,
    ) -> f64 {
        let mut available_space = delta;
        let root_node = block_root;
        let mut current = root_node;
        loop {
            current = self.align[current];
            let max_y_current = self.max_y(graph, ni, current);
            if let Some(neighbor) = self.lower_neighbor(graph, ni, current) {
                let min_y_neighbor = self.min_y(graph, ni, neighbor);
                let spacing = spacings::vertical(
                    graph,
                    lnode(graph, ni, current),
                    lnode(graph, ni, neighbor),
                );
                available_space =
                    java_min(available_space, min_y_neighbor - (max_y_current + spacing));
            }
            if root_node == current {
                break;
            }
        }
        available_space
    }

    /// `getMinY(n)`: `y[root] + innerShift - margin.top`, with no spacing.
    pub fn min_y(&self, graph: &LGraph, ni: &NeighborhoodInformation, n: usize) -> f64 {
        let root = self.root_of(n);
        self.y_of(root) + self.inner_shift[n] - lnode(graph, ni, n).margin.top
    }

    /// `getMaxY(n)`: `y[root] + innerShift + size.y + margin.bottom`, with no spacing.
    pub fn max_y(&self, graph: &LGraph, ni: &NeighborhoodInformation, n: usize) -> f64 {
        let root = self.root_of(n);
        let node = lnode(graph, ni, n);
        self.y_of(root) + self.inner_shift[n] + node.size.y + node.margin.bottom
    }

    /// `getLowerNeighbor(n, ni)`: the node after `n` in its layer, if there is one.
    fn lower_neighbor(
        &self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        n: usize,
    ) -> Option<usize> {
        let layer = &graph.layers[lnode(graph, ni, n)
            .layer
            .expect("a placed node has a layer")];
        let index = ni.node_index[n];
        if index + 1 < layer.len() {
            Some(graph.nodes[layer[index + 1]].placement_id)
        } else {
            None
        }
    }

    /// `getUpperNeighbor(n, ni)`: the node before `n` in its layer, if there is one.
    fn upper_neighbor(
        &self,
        graph: &LGraph,
        ni: &NeighborhoodInformation,
        n: usize,
    ) -> Option<usize> {
        let layer = &graph.layers[lnode(graph, ni, n)
            .layer
            .expect("a placed node has a layer")];
        let index = ni.node_index[n];
        if index > 0 {
            Some(graph.nodes[layer[index - 1]].placement_id)
        } else {
            None
        }
    }
}
