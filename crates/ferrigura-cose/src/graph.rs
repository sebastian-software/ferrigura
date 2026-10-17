//! The layout graph: `LNode`, `LEdge` and the root `LGraph` of layout-base, for the flat graphs
//! Mermaid gives the engine (no compounds). Nodes and edges are indices into arenas, kept in
//! insertion order, which is the order cytoscape gives them and the order the engine reads.

use std::collections::{HashMap, VecDeque};

use crate::constants::{EMPTY_COMPOUND_NODE_SIZE, GRAPH_MARGIN, IDEAL_EDGE_LENGTH, WORLD_BOUNDARY};
use crate::error::Error;
use crate::geometry::{PointD, RectD, Transform};

/// `Integer.MAX_VALUE`, the sentinel of the bounds (`Integer.js`).
pub(crate) const INTEGER_MAX: f64 = 2_147_483_647.0;
/// `Integer.MIN_VALUE`, the sentinel of the largest diagonal (`Integer.js`).
pub(crate) const INTEGER_MIN: f64 = -2_147_483_648.0;

/// `LNode` with the `CoSENode` and `FDLayoutNode` fields the tree path reads.
#[derive(Clone, Debug)]
pub(crate) struct LNode {
    pub rect: RectD,
    /// Incident edges in the order they were added (`LNode.edges`). The radial placement reads
    /// this order, so it is part of the result.
    pub edges: Vec<usize>,
    /// `noOfChildren`. `calcNoOfChildrenForAllNodes` gives 1 to every node of a flat graph.
    pub no_of_children: f64,
    pub spring: PointD,
    pub repulsion: PointD,
    pub gravitation: PointD,
    /// The grid cells the node covers, set by `addNodeToGrid` (`FDLayoutNode.setGridCoordinates`).
    pub start_x: i64,
    pub finish_x: i64,
    pub start_y: i64,
    pub finish_y: i64,
    /// The nodes the repulsion is computed against, from the last grid update
    /// (`FDLayout.calculateRepulsionForceOfANode`).
    pub surrounding: Vec<usize>,
}

impl LNode {
    fn new(rect: RectD) -> LNode {
        LNode {
            rect,
            edges: Vec::new(),
            no_of_children: 1.0,
            spring: PointD::default(),
            repulsion: PointD::default(),
            gravitation: PointD::default(),
            start_x: 0,
            finish_x: 0,
            start_y: 0,
            finish_y: 0,
            surrounding: Vec::new(),
        }
    }
}

/// `LEdge` with its `FDLayoutEdge` fields.
#[derive(Clone, Debug)]
pub(crate) struct LEdge {
    pub source: usize,
    pub target: usize,
    pub ideal_length: f64,
    /// Set by `updateLength` when the rectangles do not overlap; read only then.
    pub length: f64,
    pub length_x: f64,
    pub length_y: f64,
    pub overlapping: bool,
}

impl LEdge {
    fn new(source: usize, target: usize) -> LEdge {
        LEdge {
            source,
            target,
            ideal_length: IDEAL_EDGE_LENGTH,
            length: 0.0,
            length_x: 0.0,
            length_y: 0.0,
            overlapping: false,
        }
    }

    /// `getOtherEnd`: the end of the edge that is not `node`.
    pub(crate) fn other_end(&self, node: usize) -> usize {
        if self.source == node {
            self.target
        } else if self.target == node {
            self.source
        } else {
            panic!("node {node} is not incident with this edge");
        }
    }
}

/// The root `LGraph` of a flat graph, with the bounds that `updateBounds` and `updateLeftTop` set.
#[derive(Clone, Debug, Default)]
pub(crate) struct LGraph {
    pub nodes: Vec<LNode>,
    pub edges: Vec<LEdge>,
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub estimated_size: f64,
}

impl LGraph {
    /// The graph of the adapter (`CoSELayout.processChildrenList` and the edge loop of `run`,
    /// `index.js` 124-156 and 279-368). Every node's rectangle is its centre minus half its size.
    /// Mermaid sets each node's position to zero, so the centre is zero too; the radial placement
    /// overwrites every centre, so the position does not matter. A self loop and a parallel edge
    /// are dropped, as the adapter drops them.
    pub(crate) fn build(nodes: &[crate::Node], edges: &[crate::Edge]) -> Result<LGraph, Error> {
        let mut index: HashMap<&str, usize> = HashMap::with_capacity(nodes.len());
        let mut graph = LGraph::default();
        for (i, node) in nodes.iter().enumerate() {
            if !(node.width.is_finite() && node.height.is_finite()) {
                return Err(Error::NonFiniteSize(node.id.clone()));
            }
            if index.insert(node.id.as_str(), i).is_some() {
                return Err(Error::DuplicateNode(node.id.clone()));
            }
            let rect = RectD::new(
                0.0 - node.width / 2.0,
                0.0 - node.height / 2.0,
                node.width,
                node.height,
            );
            graph.nodes.push(LNode::new(rect));
        }
        for edge in edges {
            let source = *index
                .get(edge.source.as_str())
                .ok_or_else(|| Error::UnknownNode(edge.source.clone()))?;
            let target = *index
                .get(edge.target.as_str())
                .ok_or_else(|| Error::UnknownNode(edge.target.clone()))?;
            if source == target || !graph.edges_between(source, Some(target)).is_empty() {
                continue;
            }
            let id = graph.edges.len();
            graph.edges.push(LEdge::new(source, target));
            graph.nodes[source].edges.push(id);
            graph.nodes[target].edges.push(id);
        }
        Ok(graph)
    }

    /// `getEdgesBetween` (`LNode.js` 103-117): the edges of `node` that end at `other`, in the
    /// order of `node.edges`. `None` matches nothing, as the null parent of a root does.
    pub(crate) fn edges_between(&self, node: usize, other: Option<usize>) -> Vec<usize> {
        self.nodes[node]
            .edges
            .iter()
            .copied()
            .filter(|&e| {
                let edge = &self.edges[e];
                Some(edge.source) == other || Some(edge.target) == other
            })
            .collect()
    }

    /// `getNeighborsList` (`LNode.js` 184-206): the neighbours in the order of `node.edges`, each
    /// once (a Set).
    pub(crate) fn neighbours(&self, node: usize) -> Vec<usize> {
        let mut neighbours: Vec<usize> = Vec::new();
        for &e in &self.nodes[node].edges {
            let other = self.edges[e].other_end(node);
            if !neighbours.contains(&other) {
                neighbours.push(other);
            }
        }
        neighbours
    }

    /// The number of nodes with no edge. Their group is the tiling path, which is not ported.
    pub(crate) fn isolated_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.edges.is_empty())
            .count()
    }

    /// `LGraph.updateConnected` (`LGraph.js` 448-508) for the flat root: whether a walk from the
    /// first node reaches every node.
    pub(crate) fn is_connected(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }
        let mut visited = vec![false; self.nodes.len()];
        let mut count = 1;
        visited[0] = true;
        let mut queue: VecDeque<usize> = VecDeque::from([0]);
        while let Some(current) = queue.pop_front() {
            for &e in &self.nodes[current].edges {
                let neighbour = self.edges[e].other_end(current);
                if !visited[neighbour] {
                    visited[neighbour] = true;
                    count += 1;
                    queue.push_back(neighbour);
                }
            }
        }
        count == self.nodes.len()
    }

    /// `LGraph.updateBounds(true)` (`LGraph.js` 200-246), for a graph with no compound: the
    /// rectangle of the nodes, grown by the margin. The sums keep the order of the source, so
    /// `right` is `(x + width) + margin` with the width taken from the left edge.
    pub(crate) fn update_bounds(&mut self) {
        let mut left = INTEGER_MAX;
        let mut right = -INTEGER_MAX;
        let mut top = INTEGER_MAX;
        let mut bottom = -INTEGER_MAX;
        for node in &self.nodes {
            if left > node.rect.x {
                left = node.rect.x;
            }
            if right < node.rect.right() {
                right = node.rect.right();
            }
            if top > node.rect.y {
                top = node.rect.y;
            }
            if bottom < node.rect.bottom() {
                bottom = node.rect.bottom();
            }
        }
        let bounding = RectD::new(left, top, right - left, bottom - top);
        self.left = bounding.x - GRAPH_MARGIN;
        self.right = bounding.x + bounding.width + GRAPH_MARGIN;
        self.top = bounding.y - GRAPH_MARGIN;
        self.bottom = bounding.y + bounding.height + GRAPH_MARGIN;
    }

    /// `LGraph.updateLeftTop` (`LGraph.js` 150-177): the upper left corner of the nodes, less the
    /// margin. `None` for a graph with no nodes.
    pub(crate) fn update_left_top(&mut self) -> Option<PointD> {
        let mut top = INTEGER_MAX;
        let mut left = INTEGER_MAX;
        for node in &self.nodes {
            if top > node.rect.y {
                top = node.rect.y;
            }
            if left > node.rect.x {
                left = node.rect.x;
            }
        }
        if top == INTEGER_MAX {
            return None;
        }
        self.left = left - GRAPH_MARGIN;
        self.top = top - GRAPH_MARGIN;
        Some(PointD::new(self.left, self.top))
    }

    /// `LGraph.calcEstimatedSize` (`LGraph.js` 386-408) and `LNode.calcEstimatedSize`: the mean of
    /// the nodes' `(width + height) / 2`, over the square root of the count.
    pub(crate) fn calc_estimated_size(&mut self) {
        let mut size = 0.0;
        for node in &self.nodes {
            size += (node.rect.width + node.rect.height) / 2.0;
        }
        self.estimated_size = if size == 0.0 {
            EMPTY_COMPOUND_NODE_SIZE
        } else {
            size / (self.nodes.len() as f64).sqrt()
        };
    }

    /// `LNode.transform` (`LNode.js` 345-373): the corner is clamped to the world boundary, then
    /// mapped by the transform.
    pub(crate) fn transform_node(&mut self, node: usize, trans: &Transform) {
        let left = self.nodes[node]
            .rect
            .x
            .clamp(-WORLD_BOUNDARY, WORLD_BOUNDARY);
        let top = self.nodes[node]
            .rect
            .y
            .clamp(-WORLD_BOUNDARY, WORLD_BOUNDARY);
        let mapped = trans.inverse_transform_point(PointD::new(left, top));
        self.nodes[node].rect.x = mapped.x;
        self.nodes[node].rect.y = mapped.y;
    }

    /// `Layout.transform(newLeftTop)` (`Layout.js` 240-260): moves every node so that the upper
    /// left corner of the graph, less the margin, lands on `new_left_top`.
    pub(crate) fn transform_to(&mut self, new_left_top: PointD) {
        if let Some(left_top) = self.update_left_top() {
            let trans = Transform::new(new_left_top, left_top);
            for node in 0..self.nodes.len() {
                self.transform_node(node, &trans);
            }
        }
    }

    /// The centre of a node, as `getCenterX` and `getCenterY` give it.
    pub(crate) fn centre(&self, node: usize) -> PointD {
        PointD::new(
            self.nodes[node].rect.center_x(),
            self.nodes[node].rect.center_y(),
        )
    }
}
