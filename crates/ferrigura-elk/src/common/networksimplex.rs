//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/networksimplex/NetworkSimplex.java
//!   with NGraph.java, NNode.java and NEdge.java
//!
//! The network simplex of ELK. It finds a layering that minimizes the sum of weight times
//! length over all edges, as described by Gansner et al. in "A technique for drawing directed
//! graphs" (1993). The layered algorithm uses it for layering, node placement and compaction.
//!
//! Java objects are indices here. [`NGraph`] owns the nodes and edges, and [`NNode`] and
//! [`NEdge`] refer to each other by index. The Java `NGraph.nodes` list is [`NGraph::nodes`].
//! Removing a node from it and adding the node back moves the node to the end, as in Java.
//! The Java `LinkedHashSet` of tree edges is an `IndexSet`. It iterates in insertion order,
//! and it removes with `shift_remove`, because `IndexSet::remove` changes the order.
//!
//! Java semantics kept: `int` arithmetic wraps, `double` operations keep their operand order,
//! and the `else if` in `minimalSpan` is reproduced as written. Where Java throws for misuse,
//! this port panics: the `IllegalArgumentException` and `IllegalStateException` cases, and the
//! `NullPointerException` of a disconnected graph that has edges (call
//! [`NGraph::make_connected`] first). `NGraph.writeDebugGraph` is not ported, since it writes an
//! ELK graph file for debugging.

use std::collections::VecDeque;

use indexmap::IndexSet;

use crate::core::util::progress::{Monitor, ProgressMonitor};

/// `REMOVE_SUBTREES_THRESH`: graphs with fewer nodes keep their leaves.
const REMOVE_SUBTREES_THRESH: usize = 40;

/// `FUZZY_ST_ZERO`: a cut value below this counts as negative, which tolerates rounding.
const FUZZY_ST_ZERO: f64 = -1e-10;

/// A node of an [`NGraph`]. Its edges are indices into the graph's edges.
#[derive(Debug, Clone, Default)]
pub struct NNode {
    /// Java `id`: a public id that the algorithm does not use.
    pub id: i32,
    /// Java `origin`: the caller's handle for the object this node derives from. `None` is Java `null`.
    pub origin: Option<usize>,
    /// Java `type`: a label for debug output with no semantic meaning.
    pub node_type: String,
    /// Java `layer`: the rank of the node. [`NetworkSimplex::execute`] normalizes it so that
    /// the lowest layer is 0.
    pub layer: i32,
    /// Java `incomingEdges`, in insertion order.
    incoming: Vec<usize>,
    /// Java `outgoingEdges`, in insertion order.
    outgoing: Vec<usize>,
    /// Java `internalId`: the position of the node in the node list of the current run.
    internal_id: usize,
    /// Java `treeNode`: whether the node is part of the tight tree.
    tree_node: bool,
    /// Java `unknownCutvalues`: the tree edges whose cut value is not determined yet.
    unknown_cutvalues: Vec<usize>,
}

impl NNode {
    /// Java `NNode.of()`: a node with id 0, no origin, an empty type and layer 0.
    pub fn new() -> Self {
        Self::default()
    }
}

/// An edge of an [`NGraph`] from a source to a target node, with a weight and a minimal length.
#[derive(Debug, Clone)]
pub struct NEdge {
    /// Java `id`: a public id that the algorithm does not use.
    pub id: i32,
    /// Java `origin`. Java's `NEdge.of(origin)` drops its argument, so the port has no
    /// constructor for it. Set this field instead.
    pub origin: Option<usize>,
    /// Java `weight`: the cost of one unit of length.
    pub weight: f64,
    /// Java `delta`: the minimal length of the edge, 1 unless set.
    pub delta: i32,
    source: usize,
    target: usize,
    /// Java `internalId`: the position of the edge in the edge list of the current run.
    internal_id: usize,
    /// Java `treeEdge`: whether the edge is part of the spanning tree.
    tree_edge: bool,
}

impl NEdge {
    /// Java `NEdge.of().source(source).target(target)`: weight 0 and minimal length 1.
    pub fn new(source: usize, target: usize) -> Self {
        NEdge {
            id: 0,
            origin: None,
            weight: 0.0,
            delta: 1,
            source,
            target,
            internal_id: 0,
            tree_edge: false,
        }
    }

    /// Java `getSource()`.
    pub fn source(&self) -> usize {
        self.source
    }

    /// Java `getTarget()`.
    pub fn target(&self) -> usize {
        self.target
    }

    /// Java `getOther(some)`: the endpoint that is not `some`. Panics if `some` is no endpoint.
    pub fn other(&self, some: usize) -> usize {
        if some == self.source {
            self.target
        } else if some == self.target {
            self.source
        } else {
            panic!("Node {some} not part of edge {self:?}")
        }
    }
}

/// A graph for the network simplex. It owns the nodes and edges, which are addressed by index.
#[derive(Debug, Clone, Default)]
pub struct NGraph {
    /// Java `nodes`: the node indices in list order.
    node_order: Vec<usize>,
    /// The node arena. A node stays here after `removeSubtrees` takes it off `node_order`.
    nodes: Vec<NNode>,
    /// The edge arena.
    edges: Vec<NEdge>,
}

impl NGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Java `NNode.create(graph)`: appends `node` to the node list and returns its index.
    pub fn add_node(&mut self, node: NNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        self.node_order.push(index);
        index
    }

    /// Java `NEdgeBuilder.create()`: appends `edge` to the outgoing edges of its source and to
    /// the incoming edges of its target, and returns its index. Panics on a self-loop, as Java does.
    pub fn add_edge(&mut self, edge: NEdge) -> usize {
        assert!(
            edge.source != edge.target,
            "Network simplex does not support self-loops: {edge:?}"
        );
        let index = self.edges.len();
        self.nodes[edge.source].outgoing.push(index);
        self.nodes[edge.target].incoming.push(index);
        self.edges.push(edge);
        index
    }

    /// Java `nodes`: the node indices in list order.
    pub fn nodes(&self) -> &[usize] {
        &self.node_order
    }

    /// The node with index `node`.
    pub fn node(&self, node: usize) -> &NNode {
        &self.nodes[node]
    }

    /// The node with index `node`, mutably.
    pub fn node_mut(&mut self, node: usize) -> &mut NNode {
        &mut self.nodes[node]
    }

    /// The edge with index `edge`.
    pub fn edge(&self, edge: usize) -> &NEdge {
        &self.edges[edge]
    }

    /// The edge with index `edge`, mutably. Its endpoints stay as they are.
    pub fn edge_mut(&mut self, edge: usize) -> &mut NEdge {
        &mut self.edges[edge]
    }

    /// Java `getIncomingEdges()`.
    pub fn incoming_edges(&self, node: usize) -> &[usize] {
        &self.nodes[node].incoming
    }

    /// Java `getOutgoingEdges()`.
    pub fn outgoing_edges(&self, node: usize) -> &[usize] {
        &self.nodes[node].outgoing
    }

    /// Java `getConnectedEdges()`: the incoming edges, then the outgoing edges.
    pub fn connected_edges(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        let n = &self.nodes[node];
        n.incoming.iter().chain(n.outgoing.iter()).copied()
    }

    /// Java `NEdge.reverse()`: swaps source and target, and moves the edge between the lists of
    /// the two nodes.
    pub fn reverse_edge(&mut self, edge: usize) {
        let old_source = self.edges[edge].source;
        let old_target = self.edges[edge].target;
        self.edges[edge].source = old_target;
        self.edges[edge].target = old_source;
        // The new target (the old source) takes the edge as incoming, and the new source
        // (the old target) takes it as outgoing.
        remove_value(&mut self.nodes[old_source].outgoing, edge);
        self.nodes[old_source].incoming.push(edge);
        remove_value(&mut self.nodes[old_target].incoming, edge);
        self.nodes[old_target].outgoing.push(edge);
    }

    /// Java `makeConnected()`. If the graph has several connected components, it connects one
    /// representative of each to a new artificial root with zero-weight, zero-delta edges, and
    /// returns that root. Otherwise it returns `None`.
    pub fn make_connected(&mut self) -> Option<usize> {
        for (id, &node) in self.node_order.iter().enumerate() {
            self.nodes[node].internal_id = id;
        }
        let cc_rep = self.find_con_comp_representatives();
        if cc_rep.len() > 1 {
            Some(self.create_artificial_root_and_connect(&cc_rep))
        } else {
            None
        }
    }

    /// Java `isAcyclic()`: a topological numbering, then a check of every edge. Returns `false`
    /// if the graph has a cycle.
    pub fn is_acyclic(&mut self) -> bool {
        for (id, &node) in self.node_order.iter().enumerate() {
            self.nodes[node].internal_id = id;
        }
        let count = self.node_order.len();

        // initialize the number of incident edges for each node
        let mut incident = vec![0i32; count];
        let mut layer = vec![0i32; count];
        for &node in &self.node_order {
            incident[self.nodes[node].internal_id] += self.nodes[node].incoming.len() as i32;
        }

        let mut roots = VecDeque::new();
        for &node in &self.node_order {
            if self.nodes[node].incoming.is_empty() {
                roots.push_back(node);
            }
        }
        if roots.is_empty() && !self.node_order.is_empty() {
            return false;
        }
        while let Some(node) = roots.pop_front() {
            let node_id = self.nodes[node].internal_id;
            for &edge in &self.nodes[node].outgoing {
                let target = self.edges[edge].target;
                let target_id = self.nodes[target].internal_id;
                layer[target_id] = layer[target_id].max(layer[node_id].wrapping_add(1));
                incident[target_id] -= 1;
                if incident[target_id] == 0 {
                    roots.push_back(target);
                }
            }
        }

        // check for backward edges
        for &node in &self.node_order {
            for &edge in &self.nodes[node].outgoing {
                let target_id = self.nodes[self.edges[edge].target].internal_id;
                let source_id = self.nodes[self.edges[edge].source].internal_id;
                if layer[target_id] <= layer[source_id] {
                    return false;
                }
            }
        }
        true
    }

    /// Java `createArtificialRootAndConnect`.
    fn create_artificial_root_and_connect(&mut self, nodes_to_connect: &[usize]) -> usize {
        let root = self.add_node(NNode::new());
        for &src in nodes_to_connect {
            let mut edge = NEdge::new(root, src);
            edge.delta = 0;
            edge.weight = 0.0;
            self.add_edge(edge);
        }
        root
    }

    /// Java `findConCompRepresentatives`.
    fn find_con_comp_representatives(&self) -> Vec<usize> {
        let mut cc_rep = Vec::new();
        let mut mark = vec![false; self.node_order.len()];
        for &node in &self.node_order {
            if !mark[self.nodes[node].internal_id] {
                cc_rep.push(node);
                self.dfs(node, &mut mark);
            }
        }
        cc_rep
    }

    /// Java `dfs(node, mark)`.
    fn dfs(&self, node: usize, mark: &mut [bool]) {
        let id = self.nodes[node].internal_id;
        if mark[id] {
            return;
        }
        mark[id] = true;
        for edge in self.connected_edges(node) {
            let other = self.edges[edge].other(node);
            self.dfs(other, mark);
        }
    }

    /// The number of edges that [`NGraph::connected_at`] addresses for `node`.
    fn connected_len(&self, node: usize) -> usize {
        let n = &self.nodes[node];
        n.incoming.len() + n.outgoing.len()
    }

    /// The `k`th edge of `node` in the order of [`NGraph::connected_edges`]. Loops that
    /// recurse with `&mut self` read the edges by index, since the lists do not change while
    /// such a loop runs.
    fn connected_at(&self, node: usize, k: usize) -> usize {
        let n = &self.nodes[node];
        if k < n.incoming.len() {
            n.incoming[k]
        } else {
            n.outgoing[k - n.incoming.len()]
        }
    }
}

/// Java `List.remove(Object)`: removes the first occurrence and keeps the order of the rest.
fn remove_value(list: &mut Vec<usize>, value: usize) {
    if let Some(position) = list.iter().position(|&item| item == value) {
        list.remove(position);
    }
}

/// Java `NetworkSimplex`: finds the layering of a graph that minimizes the sum of weight times
/// length over all edges, with every length at least the edge's minimal length.
///
/// The graph must not have cycles, and it must be connected when it has edges (see
/// [`NGraph::make_connected`]). The layers are written to [`NNode::layer`].
pub struct NetworkSimplex<'a> {
    /// Java `graph`.
    graph: &'a mut NGraph,
    /// Java `previousLayeringNodeCounts`: nodes that a previous layering put in each layer.
    /// `None` is Java `null`.
    previous_layering_node_counts: Option<Vec<i32>>,
    /// Java `balance`.
    balance: bool,
    /// Java `iterationLimit`.
    iteration_limit: i32,
    /// Java `edges`: the edges of this run, in the order of the node list's outgoing edges.
    edges: Vec<usize>,
    /// Java `treeEdges`: the spanning tree edges in insertion order.
    tree_edges: IndexSet<usize>,
    /// Java `sources`: the nodes without incoming edges.
    sources: Vec<usize>,
    /// Java `edgeVisited`, indexed by the edge's internal id.
    edge_visited: Vec<bool>,
    /// Java `postOrder`.
    post_order: i32,
    /// Java `poID`, indexed by the node's internal id.
    po_id: Vec<i32>,
    /// Java `lowestPoID`, indexed by the node's internal id.
    lowest_po_id: Vec<i32>,
    /// Java `cutvalue`, indexed by the edge's internal id.
    cutvalue: Vec<f64>,
    /// Java `subtreeNodesStack`: the removed leaves, each with the edge that joined it to the graph.
    subtree_nodes_stack: Vec<(usize, usize)>,
}

impl<'a> NetworkSimplex<'a> {
    /// Java `forGraph(graph)`.
    pub fn for_graph(graph: &'a mut NGraph) -> Self {
        NetworkSimplex {
            graph,
            previous_layering_node_counts: None,
            balance: false,
            iteration_limit: i32::MAX,
            edges: Vec::new(),
            tree_edges: IndexSet::new(),
            sources: Vec::new(),
            edge_visited: Vec::new(),
            post_order: 1,
            po_id: Vec::new(),
            lowest_po_id: Vec::new(),
            cutvalue: Vec::new(),
            subtree_nodes_stack: Vec::new(),
        }
    }

    /// Java `withBalancing(doBalance)`: whether to balance the widths of the layers.
    pub fn with_balancing(mut self, do_balance: bool) -> Self {
        self.balance = do_balance;
        self
    }

    /// Java `withPreviousLayering(considerPreviousLayering)`. `None` is Java `null`.
    pub fn with_previous_layering(mut self, considered: Option<Vec<i32>>) -> Self {
        self.previous_layering_node_counts = considered;
        self
    }

    /// Java `withIterationLimit(limit)`: the maximum number of exchanges.
    pub fn with_iteration_limit(mut self, limit: i32) -> Self {
        self.iteration_limit = limit;
        self
    }

    /// Java `execute()`: determines the optimal layering, with a monitor nobody reads.
    pub fn execute(&mut self) {
        let (mut progress, monitor) = ProgressMonitor::new(false);
        self.execute_with_monitor(&mut progress, monitor);
    }

    /// Java `execute(IElkProgressMonitor)`: determines the optimal layering. `monitor` is the
    /// task of `progress` that this run reports to.
    pub fn execute_with_monitor(&mut self, progress: &mut ProgressMonitor, monitor: Monitor) {
        progress.begin(monitor, "Network simplex", 1.0);

        if self.graph.node_order.is_empty() {
            progress.done(monitor);
            return;
        }

        // reset any old layering
        for &node in &self.graph.node_order {
            self.graph.nodes[node].layer = 0;
        }

        // remove leafs
        let remove_subtrees = self.graph.node_order.len() >= REMOVE_SUBTREES_THRESH;
        if remove_subtrees {
            self.remove_subtrees();
        }

        // init all the data structures we use
        self.initialize();
        // determine an initial feasible layering
        self.feasible_tree();
        // improve the initial layering until it is optimal
        let mut e = self.leave_edge();
        let mut iter: i32 = 0;
        while let Some(leave) = e {
            if iter >= self.iteration_limit {
                break;
            }
            let enter = self
                .enter_edge(leave)
                .expect("network simplex: no entering edge for a negative cut value");
            self.exchange(leave, enter);
            e = self.leave_edge();
            iter = iter.wrapping_add(1);
        }

        // re-attach leafs
        if remove_subtrees {
            self.reattach_subtrees();
        }

        // normalize and, if desired, balance
        if self.balance {
            let mut filling = self.normalize();
            self.balance_filling(&mut filling);
        } else {
            self.normalize();
        }

        // release the created resources
        self.dispose();
        progress.done(monitor);
    }

    /// Java `removeSubtrees()`: removes leafs until no leafs are left.
    fn remove_subtrees(&mut self) {
        self.subtree_nodes_stack = Vec::new();

        // find initial leafs
        let mut leafs = VecDeque::new();
        for &node in &self.graph.node_order {
            if self.graph.connected_len(node) == 1 {
                leafs.push_back(node);
            }
        }

        // remove them from the graph like there's no tomorrow
        while let Some(node) = leafs.pop_front() {
            // was the edge already removed?
            if self.graph.connected_len(node) == 0 {
                continue;
            }
            let edge = self.graph.connected_at(node, 0);
            let is_out_edge = !self.graph.nodes[node].outgoing.is_empty();

            let other = self.graph.edges[edge].other(node);
            if is_out_edge {
                remove_value(&mut self.graph.nodes[other].incoming, edge);
            } else {
                remove_value(&mut self.graph.nodes[other].outgoing, edge);
            }

            if self.graph.connected_len(other) == 1 {
                leafs.push_back(other);
            }

            self.subtree_nodes_stack.push((node, edge));
            // remove the node from the graph's nodes
            remove_value(&mut self.graph.node_order, node);
        }
    }

    /// Java `reattachSubtrees()`: re-attaches the removed leafs in the reverse order of their removal.
    fn reattach_subtrees(&mut self) {
        while let Some((node, edge)) = self.subtree_nodes_stack.pop() {
            let placed = self.graph.edges[edge].other(node);

            if self.graph.edges[edge].target == node {
                self.graph.nodes[placed].outgoing.push(edge);
                self.graph.nodes[node].layer = self.graph.nodes[placed]
                    .layer
                    .wrapping_add(self.graph.edges[edge].delta);
            } else {
                self.graph.nodes[placed].incoming.push(edge);
                self.graph.nodes[node].layer = self.graph.nodes[placed]
                    .layer
                    .wrapping_sub(self.graph.edges[edge].delta);
            }

            self.graph.node_order.push(node);
        }
    }

    /// Java `initialize()`: sets up the attributes of this run.
    fn initialize(&mut self) {
        // initialize node attributes
        let num_nodes = self.graph.node_order.len();
        for &node in &self.graph.node_order {
            self.graph.nodes[node].tree_node = false;
        }
        self.po_id = vec![0; num_nodes];
        self.lowest_po_id = vec![0; num_nodes];
        self.sources = Vec::new();

        // determine edges and re-index nodes
        let mut the_edges = Vec::new();
        for (index, &node) in self.graph.node_order.iter().enumerate() {
            self.graph.nodes[node].internal_id = index;
            // add node to sources
            if self.graph.nodes[node].incoming.is_empty() {
                self.sources.push(node);
            }
            the_edges.extend_from_slice(&self.graph.nodes[node].outgoing);
        }
        // re-index edges
        for (counter, &edge) in the_edges.iter().enumerate() {
            self.graph.edges[edge].internal_id = counter;
            self.graph.edges[edge].tree_edge = false;
        }
        // initialize edge attributes. Java reuses longer arrays, but dispose() releases them
        // after every run, so fresh arrays give the same results.
        let num_edges = the_edges.len();
        self.cutvalue = vec![0.0; num_edges];
        self.edge_visited = vec![false; num_edges];
        self.edges = the_edges;
        self.tree_edges = IndexSet::with_capacity(num_edges);
        self.post_order = 1;
    }

    /// Java `dispose()`: releases the data of this run. Java nulls the fields, and this port
    /// empties them.
    fn dispose(&mut self) {
        self.cutvalue = Vec::new();
        self.edges = Vec::new();
        self.tree_edges = IndexSet::new();
        self.edge_visited = Vec::new();
        self.lowest_po_id = Vec::new();
        self.po_id = Vec::new();
        self.sources = Vec::new();
        self.subtree_nodes_stack = Vec::new();
    }

    /// Java `feasibleTree()`: an initial feasible spanning tree whose edges are all tight, with
    /// the cut values of its edges.
    fn feasible_tree(&mut self) {
        // determine initial layering
        let sources = self.sources.clone();
        self.layering_topological_numbering(sources);

        if !self.edges.is_empty() {
            self.edge_visited.fill(false);
            let root = self.graph.node_order[0];
            while self.tight_tree_dfs(root) < self.graph.node_order.len() {
                // some nodes are still not part of the tree
                let e = self
                    .minimal_slack()
                    .expect("network simplex: no edge leaves the tree; the graph is not connected");
                let target = self.graph.edges[e].target;
                let mut slack = self.slack(e);
                if self.graph.nodes[target].tree_node {
                    slack = slack.wrapping_neg();
                }

                // update tree
                for &node in &self.graph.node_order {
                    if self.graph.nodes[node].tree_node {
                        let layer = &mut self.graph.nodes[node].layer;
                        *layer = layer.wrapping_add(slack);
                    }
                }
                self.edge_visited.fill(false);
            }
            // update tree-related attributes
            self.edge_visited.fill(false);
            self.postorder_traversal(root);
            self.cutvalues();
        }
    }

    /// Java `layeringTopologicalNumbering(initialRootNodes)`: a layering in which every node is
    /// as high as its incoming edges allow.
    fn layering_topological_numbering(&mut self, initial_root_nodes: Vec<usize>) {
        // initialize the number of incident edges for each node
        let mut incident = vec![0i32; self.graph.node_order.len()];
        for &node in &self.graph.node_order {
            let id = self.graph.nodes[node].internal_id;
            incident[id] += self.graph.nodes[node].incoming.len() as i32;
        }

        let mut roots: VecDeque<usize> = initial_root_nodes.into();
        while let Some(node) = roots.pop_front() {
            let outgoing = self.graph.nodes[node].outgoing.clone();
            for edge in outgoing {
                let target = self.graph.edges[edge].target;
                let candidate = self.graph.nodes[node]
                    .layer
                    .wrapping_add(self.graph.edges[edge].delta);
                let target_node = &mut self.graph.nodes[target];
                target_node.layer = target_node.layer.max(candidate);
                let id = target_node.internal_id;
                incident[id] -= 1;
                if incident[id] == 0 {
                    roots.push_back(target);
                }
            }
        }
    }

    /// Java `minimalSpan(node)`: the length of the shortest incoming edge (first) and outgoing
    /// edge (second), or -1 where there is none.
    fn minimal_span(&self, node: usize) -> (i32, i32) {
        let mut min_span_out = i32::MAX;
        let mut min_span_in = i32::MAX;

        for edge in self.graph.connected_edges(node) {
            let current_span = self.span(edge);
            // As in Java, an incoming edge that does not shorten minSpanIn is still tested
            // against minSpanOut.
            if self.graph.edges[edge].target == node && current_span < min_span_in {
                min_span_in = current_span;
            } else if current_span < min_span_out {
                min_span_out = current_span;
            }
        }

        if min_span_in == i32::MAX {
            min_span_in = -1;
        }
        if min_span_out == i32::MAX {
            min_span_out = -1;
        }
        (min_span_in, min_span_out)
    }

    /// Java `tightTreeDFS(node)`: the number of nodes reached from `node` over tight edges.
    fn tight_tree_dfs(&mut self, node: usize) -> usize {
        let mut node_count = 1;
        self.graph.nodes[node].tree_node = true;
        for k in 0..self.graph.connected_len(node) {
            let edge = self.graph.connected_at(node, k);
            let id = self.graph.edges[edge].internal_id;
            if !self.edge_visited[id] {
                self.edge_visited[id] = true;
                let opposite = self.graph.edges[edge].other(node);
                if self.graph.edges[edge].tree_edge {
                    // edge is a tree edge already: follow this path
                    node_count += self.tight_tree_dfs(opposite);
                } else if !self.graph.nodes[opposite].tree_node
                    && self.graph.edges[edge].delta == self.span(edge)
                {
                    // edge is a tight non-tree edge
                    self.graph.edges[edge].tree_edge = true;
                    self.tree_edges.insert(edge);
                    node_count += self.tight_tree_dfs(opposite);
                }
            }
        }
        node_count
    }

    /// Java `minimalSlack()`: the first edge with the least slack that has exactly one endpoint
    /// in the tree, or `None`.
    fn minimal_slack(&self) -> Option<usize> {
        let mut min_slack = i32::MAX;
        let mut min_slack_edge = None;
        for &edge in &self.edges {
            let source = self.graph.edges[edge].source;
            let target = self.graph.edges[edge].target;
            if self.graph.nodes[source].tree_node ^ self.graph.nodes[target].tree_node {
                // edge is non-tree edge and incident on the tree
                let cur_slack = self.slack(edge);
                if cur_slack < min_slack {
                    min_slack = cur_slack;
                    min_slack_edge = Some(edge);
                }
            }
        }
        min_slack_edge
    }

    /// Java `postorderTraversal(node)`: numbers the tree in postorder and returns the lowest
    /// number reachable below `node`.
    fn postorder_traversal(&mut self, node: usize) -> i32 {
        let mut lowest = i32::MAX;
        for k in 0..self.graph.connected_len(node) {
            let edge = self.graph.connected_at(node, k);
            let id = self.graph.edges[edge].internal_id;
            if self.graph.edges[edge].tree_edge && !self.edge_visited[id] {
                self.edge_visited[id] = true;
                let other = self.graph.edges[edge].other(node);
                lowest = lowest.min(self.postorder_traversal(other));
            }
        }
        let node_id = self.graph.nodes[node].internal_id;
        self.po_id[node_id] = self.post_order;
        self.lowest_po_id[node_id] = lowest.min(self.post_order);
        self.post_order = self.post_order.wrapping_add(1);
        self.lowest_po_id[node_id]
    }

    /// Java `isInHead(node, edge)`: whether `node` is in the head component of `edge`, the
    /// component of the target. Otherwise it is in the tail component.
    fn is_in_head(&self, node: usize, edge: usize) -> bool {
        let source = self.graph.nodes[self.graph.edges[edge].source].internal_id;
        let target = self.graph.nodes[self.graph.edges[edge].target].internal_id;
        let n = self.graph.nodes[node].internal_id;

        if self.lowest_po_id[source] <= self.po_id[n]
            && self.po_id[n] <= self.po_id[source]
            && self.lowest_po_id[target] <= self.po_id[n]
            && self.po_id[n] <= self.po_id[target]
        {
            // node is in a descending path in the DFS-Tree
            if self.po_id[source] < self.po_id[target] {
                // root is in the head component
                return false;
            }
            return true;
        }
        if self.po_id[source] < self.po_id[target] {
            // root is in the head component
            return true;
        }
        false
    }

    /// Java `cutvalues()`: the cut value of every tree edge, starting from the leafs of the tree.
    fn cutvalues(&mut self) {
        // determine incident tree edges for each node
        let mut leafs = Vec::new();
        for &node in &self.graph.node_order {
            let mut tree_edge_count = 0;
            self.graph.nodes[node].unknown_cutvalues.clear();
            for k in 0..self.graph.connected_len(node) {
                let edge = self.graph.connected_at(node, k);
                if self.graph.edges[edge].tree_edge {
                    self.graph.nodes[node].unknown_cutvalues.push(edge);
                    tree_edge_count += 1;
                }
            }
            if tree_edge_count == 1 {
                leafs.push(node);
            }
        }

        // determine cut values
        for &first in &leafs {
            let mut node = first;
            while self.graph.nodes[node].unknown_cutvalues.len() == 1 {
                // one tree edge with undetermined cut value is incident
                let to_determine = self.graph.nodes[node].unknown_cutvalues[0];
                let determine_id = self.graph.edges[to_determine].internal_id;
                self.cutvalue[determine_id] = self.graph.edges[to_determine].weight;
                let source = self.graph.edges[to_determine].source;
                let target = self.graph.edges[to_determine].target;
                for edge in self.graph.connected_edges(node) {
                    if edge == to_determine {
                        continue;
                    }
                    let edge_source = self.graph.edges[edge].source;
                    let edge_target = self.graph.edges[edge].target;
                    let edge_id = self.graph.edges[edge].internal_id;
                    let weight = self.graph.edges[edge].weight;
                    if self.graph.edges[edge].tree_edge {
                        // edge is tree edge
                        let rhs = self.cutvalue[edge_id] - weight;
                        if source == edge_source || target == edge_target {
                            // edge has not the same direction as toDetermine
                            self.cutvalue[determine_id] -= rhs;
                        } else {
                            self.cutvalue[determine_id] += rhs;
                        }
                    } else {
                        // edge is non-tree edge
                        if node == source {
                            if edge_source == node {
                                self.cutvalue[determine_id] += weight;
                            } else {
                                self.cutvalue[determine_id] -= weight;
                            }
                        } else if edge_source == node {
                            self.cutvalue[determine_id] -= weight;
                        } else {
                            self.cutvalue[determine_id] += weight;
                        }
                    }
                }

                // remove edge from 'unknownCutvalues'
                remove_value(
                    &mut self.graph.nodes[source].unknown_cutvalues,
                    to_determine,
                );
                remove_value(
                    &mut self.graph.nodes[target].unknown_cutvalues,
                    to_determine,
                );

                // proceed with next node
                node = if source == node { target } else { source };
            }
        }
    }

    /// Java `leaveEdge()`: a tree edge with a negative cut value, or `None` if the layering is
    /// optimal. The first such edge in insertion order is returned.
    fn leave_edge(&self) -> Option<usize> {
        for &edge in self.tree_edges.iter() {
            let id = self.graph.edges[edge].internal_id;
            if self.graph.edges[edge].tree_edge && self.cutvalue[id] < FUZZY_ST_ZERO {
                return Some(edge);
            }
        }
        None
    }

    /// Java `enterEdge(leave)`: the non-tree edge from the head to the tail component of `leave`
    /// with the least slack, or `None`.
    fn enter_edge(&self, leave: usize) -> Option<usize> {
        assert!(
            self.graph.edges[leave].tree_edge,
            "The input edge is not a tree edge."
        );

        let mut replace = None;
        let mut rep_slack = i32::MAX;
        for &edge in &self.edges {
            let source = self.graph.edges[edge].source;
            let target = self.graph.edges[edge].target;
            if self.is_in_head(source, leave) && !self.is_in_head(target, leave) {
                // edge is to consider
                let slack = self.slack(edge);
                if slack < rep_slack {
                    rep_slack = slack;
                    replace = Some(edge);
                }
            }
        }
        replace
    }

    /// Java `exchange(leave, enter)`: replaces the tree edge `leave` by `enter`, shifts the
    /// nodes on the tail side, and recomputes the postorder and the cut values.
    fn exchange(&mut self, leave: usize, enter: usize) {
        assert!(
            self.graph.edges[leave].tree_edge,
            "Given leave edge is no tree edge."
        );
        assert!(
            !self.graph.edges[enter].tree_edge,
            "Given enter edge is a tree edge already."
        );

        // update tree
        self.graph.edges[leave].tree_edge = false;
        self.tree_edges.shift_remove(&leave);
        self.graph.edges[enter].tree_edge = true;
        self.tree_edges.insert(enter);
        let enter_target = self.graph.edges[enter].target;
        let mut delta = self.slack(enter);
        if !self.is_in_head(enter_target, leave) {
            delta = delta.wrapping_neg();
        }
        for &node in &self.graph.node_order {
            if !self.is_in_head(node, leave) {
                let layer = &mut self.graph.nodes[node].layer;
                *layer = layer.wrapping_add(delta);
            }
        }

        // update tree-based values
        self.post_order = 1;
        self.edge_visited.fill(false);
        let root = self.graph.node_order[0];
        self.postorder_traversal(root);
        self.cutvalues();
    }

    /// Java `normalize()`: shifts the layers so that the lowest is 0. Returns how many nodes are
    /// in each layer, including the nodes of previously layered components.
    fn normalize(&mut self) -> Vec<i32> {
        // determine lowest assigned layer and layer count
        let mut highest = i32::MIN;
        let mut lowest = i32::MAX;
        for &node in &self.graph.node_order {
            let layer = self.graph.nodes[node].layer;
            lowest = lowest.min(layer);
            highest = highest.max(layer);
        }

        // normalize and determine layer filling
        let size = highest.wrapping_sub(lowest).wrapping_add(1) as usize;
        let mut filling = vec![0i32; size];
        for &node in &self.graph.node_order {
            let n = &mut self.graph.nodes[node];
            n.layer = n.layer.wrapping_sub(lowest);
            filling[n.layer as usize] += 1;
        }

        // also consider nodes of already layered connected components
        if let Some(counts) = &self.previous_layering_node_counts {
            for (layer_id, &count) in counts.iter().enumerate() {
                filling[layer_id] = filling[layer_id].wrapping_add(count);
                if filling.len() == layer_id + 1 {
                    break;
                }
            }
        }
        filling
    }

    /// Java `balance(filling)`: moves a node with as many incoming as outgoing edges to the least
    /// filled layer its edges allow.
    fn balance_filling(&mut self, filling: &mut [i32]) {
        // determine possible layers
        for &node in &self.graph.node_order {
            if self.graph.nodes[node].incoming.len() == self.graph.nodes[node].outgoing.len() {
                // node might get shifted
                let layer = self.graph.nodes[node].layer;
                let mut new_layer = layer;
                let (min_span_in, min_span_out) = self.minimal_span(node);
                let start = layer.wrapping_sub(min_span_in).wrapping_add(1);
                let end = layer.wrapping_add(min_span_out);
                for i in start..end {
                    if filling[i as usize] < filling[new_layer as usize] {
                        new_layer = i;
                    }
                }
                // assign new layer
                if filling[new_layer as usize] < filling[layer as usize] {
                    filling[layer as usize] -= 1;
                    filling[new_layer as usize] += 1;
                    self.graph.nodes[node].layer = new_layer;
                }
            }
        }
    }

    /// Java `tree edge length minus minimal length` as an `int`: `target.layer - source.layer - delta`.
    fn slack(&self, edge: usize) -> i32 {
        self.span(edge).wrapping_sub(self.graph.edges[edge].delta)
    }

    /// `target.layer - source.layer` of `edge`.
    fn span(&self, edge: usize) -> i32 {
        let source = self.graph.edges[edge].source;
        let target = self.graph.edges[edge].target;
        self.graph.nodes[target]
            .layer
            .wrapping_sub(self.graph.nodes[source].layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adds an edge with the given minimal length and weight, and returns its index.
    fn add_edge(
        graph: &mut NGraph,
        source: usize,
        target: usize,
        delta: i32,
        weight: f64,
    ) -> usize {
        let mut edge = NEdge::new(source, target);
        edge.delta = delta;
        edge.weight = weight;
        graph.add_edge(edge)
    }

    fn layers(graph: &NGraph, nodes: &[usize]) -> Vec<i32> {
        nodes.iter().map(|&node| graph.node(node).layer).collect()
    }

    /// Asserts that every edge is at least as long as its minimal length.
    fn assert_feasible(graph: &NGraph) {
        for edge in &graph.edges {
            let span = graph.node(edge.target).layer - graph.node(edge.source).layer;
            assert!(
                span >= edge.delta,
                "edge {edge:?} is shorter than its minimal length"
            );
        }
    }

    /// The sum of weight times length over all edges.
    fn total_weighted_length(graph: &NGraph) -> f64 {
        graph
            .edges
            .iter()
            .map(|edge| {
                edge.weight
                    * f64::from(graph.node(edge.target).layer - graph.node(edge.source).layer)
            })
            .sum()
    }

    #[test]
    fn chain_gets_strictly_increasing_layers() {
        // a -> b -> c with minimal length 1 and weight 1
        let mut graph = NGraph::new();
        let a = graph.add_node(NNode::new());
        let b = graph.add_node(NNode::new());
        let c = graph.add_node(NNode::new());
        add_edge(&mut graph, a, b, 1, 1.0);
        add_edge(&mut graph, b, c, 1, 1.0);

        NetworkSimplex::for_graph(&mut graph).execute();

        assert_eq!(layers(&graph, &[a, b, c]), vec![0, 1, 2]);
    }

    #[test]
    fn chain_respects_minimal_lengths() {
        // a -> b with delta 2 and b -> c with delta 3 have the only optimal layering 0, 2, 5
        let mut graph = NGraph::new();
        let a = graph.add_node(NNode::new());
        let b = graph.add_node(NNode::new());
        let c = graph.add_node(NNode::new());
        add_edge(&mut graph, a, b, 2, 1.0);
        add_edge(&mut graph, b, c, 3, 1.0);

        NetworkSimplex::for_graph(&mut graph).execute();

        assert_eq!(layers(&graph, &[a, b, c]), vec![0, 2, 5]);
        assert_feasible(&graph);
    }

    #[test]
    fn diamond_with_unit_weights_is_layered_in_two_ranks() {
        // a -> b, a -> c, b -> d, c -> d. Both paths give d - a of at least 2, and the total
        // weight is 2 * (d - a), so the unique optimum is a=0, b=1, c=1, d=2.
        let mut graph = NGraph::new();
        let a = graph.add_node(NNode::new());
        let b = graph.add_node(NNode::new());
        let c = graph.add_node(NNode::new());
        let d = graph.add_node(NNode::new());
        add_edge(&mut graph, a, b, 1, 1.0);
        add_edge(&mut graph, a, c, 1, 1.0);
        add_edge(&mut graph, b, d, 1, 1.0);
        add_edge(&mut graph, c, d, 1, 1.0);

        NetworkSimplex::for_graph(&mut graph).execute();

        assert_eq!(layers(&graph, &[a, b, c, d]), vec![0, 1, 1, 2]);
    }

    #[test]
    fn diamond_with_a_longer_side_keeps_the_minimal_total() {
        // Same diamond, but c -> d has delta 2. Then d - a >= 3, so the total weight is at least 6.
        // The optimum has a=0, c=1, d=3, and b is free within [1, 2].
        let mut graph = NGraph::new();
        let a = graph.add_node(NNode::new());
        let b = graph.add_node(NNode::new());
        let c = graph.add_node(NNode::new());
        let d = graph.add_node(NNode::new());
        add_edge(&mut graph, a, b, 1, 1.0);
        add_edge(&mut graph, a, c, 1, 1.0);
        add_edge(&mut graph, b, d, 1, 1.0);
        add_edge(&mut graph, c, d, 2, 1.0);

        NetworkSimplex::for_graph(&mut graph).execute();

        assert_feasible(&graph);
        assert!((total_weighted_length(&graph) - 6.0).abs() < 1e-9);
        let b_layer = graph.node(b).layer;
        assert_eq!(layers(&graph, &[a, c, d]), vec![0, 1, 3]);
        assert!(b_layer == 1 || b_layer == 2, "b is in layer {b_layer}");
    }

    #[test]
    fn weights_decide_where_a_free_node_sits() {
        // s -> m1 -> m2 -> t and s -> x -> t, all with minimal length 1. Only the weight of
        // x -> t changes between the runs.
        let build = |weight: f64| {
            let mut graph = NGraph::new();
            let s = graph.add_node(NNode::new());
            let m1 = graph.add_node(NNode::new());
            let m2 = graph.add_node(NNode::new());
            let t = graph.add_node(NNode::new());
            let x = graph.add_node(NNode::new());
            add_edge(&mut graph, s, m1, 1, 1.0);
            add_edge(&mut graph, m1, m2, 1, 1.0);
            add_edge(&mut graph, m2, t, 1, 1.0);
            add_edge(&mut graph, s, x, 1, 1.0);
            add_edge(&mut graph, x, t, 1, weight);
            (graph, [s, m1, m2, t, x])
        };

        // With weight 3 the total is 4t - 2x, which is minimal at t=3 and x=2, and both are
        // forced. The initial layering has x=1, so one exchange moves it.
        let (mut graph, nodes) = build(3.0);
        NetworkSimplex::for_graph(&mut graph).execute();
        assert_eq!(layers(&graph, &nodes), vec![0, 1, 2, 3, 2]);
        assert!((total_weighted_length(&graph) - 8.0).abs() < 1e-9);

        // The iteration limit of 0 keeps the initial layering, with x=1.
        let (mut graph, nodes) = build(3.0);
        NetworkSimplex::for_graph(&mut graph)
            .with_iteration_limit(0)
            .execute();
        assert_eq!(layers(&graph, &nodes), vec![0, 1, 2, 3, 1]);

        // With weight 1 the cut value of s -> x is 0, so no exchange happens and x stays at 1.
        // Both x=1 and x=2 are optimal here, with total weight 6.
        let (mut graph, nodes) = build(1.0);
        NetworkSimplex::for_graph(&mut graph).execute();
        assert_eq!(layers(&graph, &nodes), vec![0, 1, 2, 3, 1]);
        assert!((total_weighted_length(&graph) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn long_chain_is_layered_through_subtree_removal() {
        // 50 nodes reach the threshold, so the leafs are removed and re-attached. The only
        // optimal layering of the chain is the identity.
        let mut graph = NGraph::new();
        let nodes: Vec<usize> = (0..50).map(|_| graph.add_node(NNode::new())).collect();
        for pair in nodes.windows(2) {
            add_edge(&mut graph, pair[0], pair[1], 1, 1.0);
        }

        NetworkSimplex::for_graph(&mut graph).execute();

        let expected: Vec<i32> = (0..50).collect();
        assert_eq!(layers(&graph, &nodes), expected);
    }

    #[test]
    fn balancing_keeps_the_layering_feasible_and_optimal() {
        // s -> x -> t and s -> u -> v -> t. The total weight is 2 * (t - s) >= 6. Balancing moves
        // x only within the range its neighbors allow, so the total stays 6.
        let mut graph = NGraph::new();
        let s = graph.add_node(NNode::new());
        let x = graph.add_node(NNode::new());
        let t = graph.add_node(NNode::new());
        let u = graph.add_node(NNode::new());
        let v = graph.add_node(NNode::new());
        add_edge(&mut graph, s, x, 1, 1.0);
        add_edge(&mut graph, x, t, 1, 1.0);
        add_edge(&mut graph, s, u, 1, 1.0);
        add_edge(&mut graph, u, v, 1, 1.0);
        add_edge(&mut graph, v, t, 1, 1.0);

        NetworkSimplex::for_graph(&mut graph)
            .with_balancing(true)
            .execute();

        assert_feasible(&graph);
        assert!((total_weighted_length(&graph) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn make_connected_adds_a_root_for_each_component() {
        // Two components: a -> b and c -> d.
        let mut graph = NGraph::new();
        let a = graph.add_node(NNode::new());
        let b = graph.add_node(NNode::new());
        let c = graph.add_node(NNode::new());
        let d = graph.add_node(NNode::new());
        add_edge(&mut graph, a, b, 1, 1.0);
        add_edge(&mut graph, c, d, 1, 1.0);

        let root = graph.make_connected().expect("two components need a root");

        assert_eq!(root, 4);
        let targets: Vec<usize> = graph
            .outgoing_edges(root)
            .iter()
            .map(|&edge| graph.edge(edge).target())
            .collect();
        assert_eq!(targets, vec![a, c]);
        assert_eq!(graph.edge(graph.outgoing_edges(root)[0]).delta, 0);
        assert_eq!(graph.make_connected(), None);
    }

    #[test]
    fn is_acyclic_detects_cycles() {
        let mut cyclic = NGraph::new();
        let a = cyclic.add_node(NNode::new());
        let b = cyclic.add_node(NNode::new());
        add_edge(&mut cyclic, a, b, 1, 1.0);
        add_edge(&mut cyclic, b, a, 1, 1.0);
        assert!(!cyclic.is_acyclic());

        let mut chain = NGraph::new();
        let a = chain.add_node(NNode::new());
        let b = chain.add_node(NNode::new());
        let c = chain.add_node(NNode::new());
        add_edge(&mut chain, a, b, 1, 1.0);
        add_edge(&mut chain, b, c, 1, 1.0);
        assert!(chain.is_acyclic());
    }

    #[test]
    fn reverse_edge_moves_the_edge_between_the_lists() {
        let mut graph = NGraph::new();
        let a = graph.add_node(NNode::new());
        let b = graph.add_node(NNode::new());
        let edge = add_edge(&mut graph, a, b, 1, 1.0);

        graph.reverse_edge(edge);

        assert_eq!(graph.edge(edge).source(), b);
        assert_eq!(graph.edge(edge).target(), a);
        assert!(graph.outgoing_edges(a).is_empty());
        assert_eq!(graph.incoming_edges(a), &[edge]);
        assert_eq!(graph.outgoing_edges(b), &[edge]);
        assert!(graph.incoming_edges(b).is_empty());
    }
}
