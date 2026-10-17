//! The compound graph of layout-base 2.0.1 (`LGraphManager`, `LGraph`, `LNode`, `LEdge`) with the
//! fields of `FDLayoutNode`, `FDLayoutEdge` and `CoSENode` that the default path reads.
//!
//! Nodes, graphs and edges live in arenas and refer to each other by index. The order of the
//! arenas is the order the JavaScript objects are created, and every list keeps the insertion
//! order of the source (`push` and `splice` as the source does), because the engine's sums and
//! its discrete tests depend on that order.
//!
//! Graph 0 is the root graph. `graphs` holds the graphs in the order `LGraphManager.add` pushed
//! them, which is the order `getAllNodes` and `getAllEdges` concatenate them in.

use crate::geometry::{PointD, RectD};

/// `Integer.MAX_VALUE` and `Integer.MIN_VALUE` (`layout-base/src/util/Integer.js`), the sentinels
/// of `inclusionTreeDepth` and `estimatedSize`, and of the bounds of a graph.
pub(crate) const INTEGER_MAX: f64 = 2_147_483_647.0;
pub(crate) const INTEGER_MIN: f64 = -2_147_483_648.0;

/// `LayoutConstants.DEFAULT_GRAPH_MARGIN` (`LayoutConstants.js`): the margin of a graph whose
/// parent has no padding, which is the root (its parent is the dummy root node).
pub(crate) const DEFAULT_GRAPH_MARGIN: f64 = 15.0;

/// `LayoutConstants.EMPTY_COMPOUND_NODE_SIZE`.
pub(crate) const EMPTY_COMPOUND_NODE_SIZE: f64 = 40.0;

/// `LayoutConstants.SIMPLE_NODE_SIZE`, used by the inter-graph ideal length.
pub(crate) const SIMPLE_NODE_SIZE: f64 = 40.0;

/// One `LNode`, with its `CoSENode` and `FDLayoutNode` fields.
#[derive(Clone, Debug)]
pub(crate) struct Node {
    /// The node's id (`node.id`, set from `data('id')`); constraints name nodes by it.
    pub id: String,
    /// `rect`: upper left corner and size. For a compound it is the bounds of its children after
    /// `updateBounds`, and the estimated size after `calcEstimatedSize`.
    pub rect: RectD,
    /// The graph this node belongs to (`owner`).
    pub owner: usize,
    /// The graph of its children (`child`), if it has children.
    pub child: Option<usize>,
    /// Incident edges, in the order they were added (`edges`).
    pub edges: Vec<usize>,
    /// `paddingLeft` (and the others, all equal): `parseInt(css('padding'))`. `None` for the
    /// dummy parent of the root, which has no padding and makes the root's margin the default.
    pub padding: Option<f64>,
    /// `nodeRepulsion`: the value of `nodeRepulsion(node)`, 4500 for every node of Mermaid.
    pub node_repulsion: f64,
    /// `noOfChildren`, set by `calcNoOfChildrenForAllNodes`.
    pub no_of_children: f64,
    /// `estimatedSize`; `INTEGER_MIN` until `calcEstimatedSize` runs.
    pub estimated_size: f64,
    /// `inclusionTreeDepth`; `INTEGER_MAX` until `calcInclusionTreeDepths` runs.
    pub inclusion_tree_depth: f64,
    /// `springForceX/Y`, `repulsionForceX/Y`, `gravitationForceX/Y`, `displacementX/Y`.
    pub spring: PointD,
    pub repulsion: PointD,
    pub gravitation: PointD,
    pub displacement: PointD,
    /// The grid cells the node covers: `startX`, `finishX`, `startY`, `finishY`.
    pub start_x: i64,
    pub finish_x: i64,
    pub start_y: i64,
    pub finish_y: i64,
    /// `surrounding`: the nodes repelled from this node, from the last grid update.
    pub surrounding: Vec<usize>,
    /// `fixedNodeWeight`, set by the constraint set-up for compounds (`initConstraintVariables`).
    pub fixed_node_weight: Option<f64>,
}

impl Node {
    fn new(id: String, owner: usize, rect: RectD, padding: Option<f64>) -> Node {
        Node {
            id,
            rect,
            owner,
            child: None,
            edges: Vec::new(),
            padding,
            node_repulsion: 4500.0,
            no_of_children: 1.0,
            estimated_size: INTEGER_MIN,
            inclusion_tree_depth: INTEGER_MAX,
            spring: PointD::default(),
            repulsion: PointD::default(),
            gravitation: PointD::default(),
            displacement: PointD::default(),
            start_x: 0,
            finish_x: 0,
            start_y: 0,
            finish_y: 0,
            surrounding: Vec::new(),
            fixed_node_weight: None,
        }
    }
}

/// One `LGraph`.
#[derive(Clone, Debug)]
pub(crate) struct Graph {
    /// The node whose child this graph is; `None` for the root.
    pub parent: Option<usize>,
    /// `nodes`, in the order they were added.
    pub nodes: Vec<usize>,
    /// `edges`: the edges whose two ends are in this graph.
    pub edges: Vec<usize>,
    /// `left`, `right`, `top`, `bottom`: the bounds with the margin (`updateBounds`).
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    /// `estimatedSize`; `INTEGER_MIN` until `calcEstimatedSize` runs.
    pub estimated_size: f64,
    /// `isConnected` (set by `updateConnected`, a field that shadows the method in the source).
    pub is_connected: bool,
}

impl Graph {
    fn new(parent: Option<usize>) -> Graph {
        Graph {
            parent,
            nodes: Vec::new(),
            edges: Vec::new(),
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
            estimated_size: INTEGER_MIN,
            is_connected: false,
        }
    }
}

/// One `LEdge` (`FDLayoutEdge` fields included).
#[derive(Clone, Debug)]
pub(crate) struct Edge {
    pub source: usize,
    pub target: usize,
    /// `isInterGraph`: the ends are in different graphs; the edge is then in the manager's list.
    pub inter_graph: bool,
    /// `idealLength`, from the input and, for inter-graph edges, `calcIdealEdgeLengths`.
    pub ideal_length: f64,
    /// `edgeElasticity`, from the input.
    pub edge_elasticity: f64,
    /// `length`, `lengthX`, `lengthY` and `isOverlapingSourceAndTarget`, from `updateLength`.
    pub length: f64,
    pub length_x: f64,
    pub length_y: f64,
    pub overlapping: bool,
    /// `lca`, `sourceInLca`, `targetInLca`, from `calcLowestCommonAncestors`.
    pub lca: Option<usize>,
    pub source_in_lca: usize,
    pub target_in_lca: usize,
}

/// The graph manager and its graphs, nodes and edges.
#[derive(Clone, Debug, Default)]
pub(crate) struct Model {
    pub nodes: Vec<Node>,
    pub graphs: Vec<Graph>,
    pub edges: Vec<Edge>,
    /// `LGraphManager.edges`: the inter-graph edges, in the order they were added.
    pub inter_edges: Vec<usize>,
}

impl Model {
    /// `LGraphManager.addRoot`: the root graph, with no parent node.
    pub(crate) fn new() -> Model {
        Model {
            nodes: Vec::new(),
            graphs: vec![Graph::new(None)],
            edges: Vec::new(),
            inter_edges: Vec::new(),
        }
    }

    pub(crate) fn root(&self) -> usize {
        0
    }

    /// `LNode` created with its rectangle and added to `graph` (`LGraph.add(node)`).
    pub(crate) fn add_node(
        &mut self,
        graph: usize,
        node_id: &str,
        rect: RectD,
        padding: Option<f64>,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes
            .push(Node::new(node_id.to_string(), graph, rect, padding));
        self.graphs[graph].nodes.push(id);
        id
    }

    /// `LGraphManager.add(newGraph, parentNode)`: a new graph, the child of `parent`.
    pub(crate) fn add_child_graph(&mut self, parent: usize) -> usize {
        let id = self.graphs.len();
        self.graphs.push(Graph::new(Some(parent)));
        self.nodes[parent].child = Some(id);
        id
    }

    /// `LGraphManager.add(newEdge, source, target)`: within one graph the edge is that graph's;
    /// otherwise it is an inter-graph edge. Returns `None` when the edge is refused by `LGraph.add`
    /// (the ends are not in the graph, which the caller never asks for).
    pub(crate) fn add_edge(&mut self, source: usize, target: usize) -> usize {
        let id = self.edges.len();
        let source_graph = self.nodes[source].owner;
        let target_graph = self.nodes[target].owner;
        let inter_graph = source_graph != target_graph;
        self.edges.push(Edge {
            source,
            target,
            inter_graph,
            ideal_length: 0.0,
            edge_elasticity: 0.0,
            length: 0.0,
            length_x: 0.0,
            length_y: 0.0,
            overlapping: false,
            lca: None,
            source_in_lca: source,
            target_in_lca: target,
        });
        if inter_graph {
            self.inter_edges.push(id);
        } else {
            self.graphs[source_graph].edges.push(id);
        }
        self.nodes[source].edges.push(id);
        self.nodes[target].edges.push(id);
        id
    }

    /// `LGraphManager.getAllNodes`: the nodes of every graph, graph by graph.
    pub(crate) fn all_nodes(&self) -> Vec<usize> {
        let mut list = Vec::new();
        for graph in &self.graphs {
            list.extend_from_slice(&graph.nodes);
        }
        list
    }

    /// `LGraphManager.getAllEdges`: the edges of every graph, then the inter-graph edges.
    pub(crate) fn all_edges(&self) -> Vec<usize> {
        let mut list = Vec::new();
        for graph in &self.graphs {
            list.extend_from_slice(&graph.edges);
        }
        list.extend_from_slice(&self.inter_edges);
        list
    }

    /// `LNode.getLeft`, `getRight`, `getTop`, `getBottom`.
    pub(crate) fn left(&self, node: usize) -> f64 {
        self.nodes[node].rect.x
    }

    pub(crate) fn right(&self, node: usize) -> f64 {
        self.nodes[node].rect.right()
    }

    pub(crate) fn top(&self, node: usize) -> f64 {
        self.nodes[node].rect.y
    }

    pub(crate) fn bottom(&self, node: usize) -> f64 {
        self.nodes[node].rect.bottom()
    }

    /// `LNode.getCenterX` and `getCenterY`.
    pub(crate) fn centre_x(&self, node: usize) -> f64 {
        self.nodes[node].rect.center_x()
    }

    pub(crate) fn centre_y(&self, node: usize) -> f64 {
        self.nodes[node].rect.center_y()
    }

    /// `LNode.getEdgesBetween(other)`: the edges of `node` that end at `other`, in `node.edges`
    /// order.
    pub(crate) fn edges_between(&self, node: usize, other: usize) -> Vec<usize> {
        self.nodes[node]
            .edges
            .iter()
            .copied()
            .filter(|&e| self.edges[e].source == other || self.edges[e].target == other)
            .collect()
    }

    /// `LNode.getOtherEnd`-style accessor: the end of `edge` that is not `node`.
    pub(crate) fn other_end(&self, edge: usize, node: usize) -> usize {
        let e = &self.edges[edge];
        if e.source == node { e.target } else { e.source }
    }

    /// `LNode.withChildren`: the node and all its descendants, in a `Set` of insertion order.
    pub(crate) fn with_children(&self, node: usize) -> Vec<usize> {
        let mut set: Vec<usize> = vec![node];
        if let Some(child) = self.nodes[node].child {
            for &c in &self.graphs[child].nodes {
                for d in self.with_children(c) {
                    if !set.contains(&d) {
                        set.push(d);
                    }
                }
            }
        }
        set
    }

    /// `LNode.getNoOfChildren`: the number of leaves below the node, at least 1.
    pub(crate) fn no_of_children(&self, node: usize) -> f64 {
        let mut count = 0.0;
        match self.nodes[node].child {
            None => count = 1.0,
            Some(child) => {
                for &c in &self.graphs[child].nodes {
                    count += self.no_of_children(c);
                }
            }
        }
        if count == 0.0 {
            count = 1.0;
        }
        count
    }

    /// `LGraph.getInclusionTreeDepth` (the root is 1).
    pub(crate) fn graph_inclusion_depth(&self, graph: usize) -> f64 {
        match self.graphs[graph].parent {
            None => 1.0,
            Some(parent) => self.nodes[parent].inclusion_tree_depth,
        }
    }

    /// `LGraph.calcEstimatedSize` (`LGraph.js` 386-408): the sum of the nodes' estimated sizes over
    /// the root of the node count's square root. Compounds get their estimated size as rectangle
    /// width and height (`LNode.calcEstimatedSize`).
    pub(crate) fn calc_graph_estimated_size(&mut self, graph: usize) -> f64 {
        let nodes = self.graphs[graph].nodes.clone();
        let mut size = 0.0;
        for &n in &nodes {
            size += self.calc_node_estimated_size(n);
        }
        let estimated = if size == 0.0 {
            EMPTY_COMPOUND_NODE_SIZE
        } else {
            size / (nodes.len() as f64).sqrt()
        };
        self.graphs[graph].estimated_size = estimated;
        estimated
    }

    /// `LNode.calcEstimatedSize` (`LNode.js` 264-277).
    pub(crate) fn calc_node_estimated_size(&mut self, node: usize) -> f64 {
        match self.nodes[node].child {
            None => {
                let rect = self.nodes[node].rect;
                let estimated = (rect.width + rect.height) / 2.0;
                self.nodes[node].estimated_size = estimated;
                estimated
            }
            Some(child) => {
                let estimated = self.calc_graph_estimated_size(child);
                self.nodes[node].estimated_size = estimated;
                self.nodes[node].rect.width = estimated;
                self.nodes[node].rect.height = estimated;
                estimated
            }
        }
    }

    /// `LGraph.updateBounds(recursive)` (`LGraph.js` 248-317). The margin is the padding of the
    /// graph's parent, or `DEFAULT_GRAPH_MARGIN` for the root. Sums keep the source's operand
    /// order: `x + width + margin`.
    pub(crate) fn update_graph_bounds(&mut self, graph: usize, recursive: bool) {
        let mut left = INTEGER_MAX;
        let mut right = -INTEGER_MAX;
        let mut top = INTEGER_MAX;
        let mut bottom = -INTEGER_MAX;
        let nodes = self.graphs[graph].nodes.clone();
        for &n in &nodes {
            if recursive && self.nodes[n].child.is_some() {
                self.update_node_bounds(n);
            }
            let node_left = self.left(n);
            let node_right = self.right(n);
            let node_top = self.top(n);
            let node_bottom = self.bottom(n);
            if left > node_left {
                left = node_left;
            }
            if right < node_right {
                right = node_right;
            }
            if top > node_top {
                top = node_top;
            }
            if bottom < node_bottom {
                bottom = node_bottom;
            }
        }
        let bounding = RectD::new(left, top, right - left, bottom - top);
        if left == INTEGER_MAX {
            // No nodes: the bounds are the parent's (LGraph.js 277-281). The engine never gets a
            // graph without nodes, so the parent is the only case that matters.
            if let Some(parent) = self.graphs[graph].parent {
                let g = &mut self.graphs[graph];
                g.left = self.nodes[parent].rect.x;
                g.right = self.nodes[parent].rect.right();
                g.top = self.nodes[parent].rect.y;
                g.bottom = self.nodes[parent].rect.bottom();
            }
        }
        let margin = self.margin(graph);
        let g = &mut self.graphs[graph];
        g.left = bounding.x - margin;
        g.right = bounding.x + bounding.width + margin;
        g.top = bounding.y - margin;
        g.bottom = bounding.y + bounding.height + margin;
    }

    /// The margin of a graph: the padding of its parent when it has one, the default otherwise
    /// (`LGraph.updateBounds` reads `nodes[0].getParent().paddingLeft`).
    fn margin(&self, graph: usize) -> f64 {
        match self.graphs[graph].parent {
            Some(parent) => match self.nodes[parent].padding {
                Some(padding) => padding,
                None => DEFAULT_GRAPH_MARGIN,
            },
            None => DEFAULT_GRAPH_MARGIN,
        }
    }

    /// `LNode.updateBounds` (`LNode.js` 297-349) without labels: a compound takes the bounds of
    /// its child graph.
    pub(crate) fn update_node_bounds(&mut self, node: usize) {
        if let Some(child) = self.nodes[node]
            .child
            .filter(|&c| !self.graphs[c].nodes.is_empty())
        {
            self.update_graph_bounds(child, true);
            let g = self.graphs[child].clone();
            let rect = &mut self.nodes[node].rect;
            rect.x = g.left;
            rect.y = g.top;
            rect.width = g.right - g.left;
            rect.height = g.bottom - g.top;
        }
    }

    /// `LGraphManager.updateBounds`: `rootGraph.updateBounds(true)`.
    pub(crate) fn update_bounds(&mut self) {
        self.update_graph_bounds(0, true);
    }

    /// `LGraph.updateConnected` (`LGraph.js` 410-446): a breadth-first walk from the first node,
    /// through the edges of each node (with its children), to the edges that stay in this graph.
    pub(crate) fn update_connected(&mut self, graph: usize) {
        let nodes = self.graphs[graph].nodes.clone();
        if nodes.is_empty() {
            self.graphs[graph].is_connected = true;
            return;
        }
        // The queue takes every node pushed, as the source does; `visited` is a Set, so it counts
        // each node once.
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        let mut visited = vec![false; self.nodes.len()];
        let mut visited_count = 0usize;
        let mut visited_list: Vec<usize> = Vec::new();
        for n in self.with_children(nodes[0]) {
            queue.push_back(n);
            if !visited[n] {
                visited[n] = true;
                visited_count += 1;
                visited_list.push(n);
            }
        }
        while let Some(current) = queue.pop_front() {
            let incident = self.nodes[current].edges.clone();
            for e in incident {
                let neighbour = self.other_end_in_graph(e, current, graph);
                if let Some(neighbour) = neighbour.filter(|&v| !visited[v]) {
                    for n in self.with_children(neighbour) {
                        queue.push_back(n);
                        if !visited[n] {
                            visited[n] = true;
                            visited_count += 1;
                            visited_list.push(n);
                        }
                    }
                }
            }
        }
        self.graphs[graph].is_connected = false;
        if visited_count >= nodes.len() {
            let in_graph = visited_list
                .iter()
                .filter(|&&v| self.nodes[v].owner == graph)
                .count();
            if in_graph == nodes.len() {
                self.graphs[graph].is_connected = true;
            }
        }
    }

    /// `LEdge.getOtherEndInGraph(node, graph)` (`LEdge.js` 82-103): the end of `edge` that is not
    /// `node`, lifted to the ancestor that belongs to `graph`, or `None`.
    pub(crate) fn other_end_in_graph(
        &self,
        edge: usize,
        node: usize,
        graph: usize,
    ) -> Option<usize> {
        let mut other = self.other_end(edge, node);
        loop {
            let owner = self.nodes[other].owner;
            if owner == graph {
                return Some(other);
            }
            if owner == 0 {
                return None;
            }
            other = self.graphs[owner].parent?;
        }
    }

    /// `LGraphManager.calcLowestCommonAncestors` (`LGraphManager.js` 346-415) for every edge of
    /// `getAllEdges`.
    pub(crate) fn calc_lowest_common_ancestors(&mut self) {
        for edge in self.all_edges() {
            let source_node = self.edges[edge].source;
            let target_node = self.edges[edge].target;
            self.edges[edge].lca = None;
            self.edges[edge].source_in_lca = source_node;
            self.edges[edge].target_in_lca = target_node;
            if source_node == target_node {
                self.edges[edge].lca = Some(self.nodes[source_node].owner);
                continue;
            }
            let mut source_ancestor = self.nodes[source_node].owner;
            while self.edges[edge].lca.is_none() {
                self.edges[edge].target_in_lca = target_node;
                let mut target_ancestor = self.nodes[target_node].owner;
                while self.edges[edge].lca.is_none() {
                    if target_ancestor == source_ancestor {
                        self.edges[edge].lca = Some(target_ancestor);
                        break;
                    }
                    if target_ancestor == 0 {
                        break;
                    }
                    let parent = self.graphs[target_ancestor]
                        .parent
                        .expect("a graph other than the root has a parent");
                    self.edges[edge].target_in_lca = parent;
                    target_ancestor = self.nodes[parent].owner;
                }
                if source_ancestor == 0 {
                    break;
                }
                if self.edges[edge].lca.is_none() {
                    let parent = self.graphs[source_ancestor]
                        .parent
                        .expect("a graph other than the root has a parent");
                    self.edges[edge].source_in_lca = parent;
                    source_ancestor = self.nodes[parent].owner;
                }
            }
        }
    }

    /// `LGraphManager.calcInclusionTreeDepths` (`LGraphManager.js` 453-472).
    pub(crate) fn calc_inclusion_tree_depths(&mut self, graph: usize, depth: f64) {
        let nodes = self.graphs[graph].nodes.clone();
        for n in nodes {
            self.nodes[n].inclusion_tree_depth = depth;
            if let Some(child) = self.nodes[n].child {
                self.calc_inclusion_tree_depths(child, depth + 1.0);
            }
        }
    }
}
