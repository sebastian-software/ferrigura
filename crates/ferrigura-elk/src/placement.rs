//! A node placement of the caller's, run after ELK's own. Not ported from ELK.
//!
//! ELK Layered decides the layers of the nodes and their order, then where each node stands along
//! its layer, then the routes. [`layout_with`] lets the caller decide the middle step anew: after
//! the network simplex placement of a graph, it hands a [`Placement`] the layers as plain data and
//! takes the places back. The placement itself is the caller's code and lives outside this crate,
//! which stays a port.

use std::cell::RefCell;

use crate::common::networksimplex::{NEdge, NGraph, NNode, NetworkSimplex};
use crate::core::data::enums::{NodeType, PortSide};
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::p4nodes::bk::access::{is_in_layer_edge, is_self_loop};
use crate::layered::spacings;
use crate::{Layout, LayoutError, json};

thread_local! {
    /// The placement of the layout that runs on this thread, if it has one.
    static PLACEMENT: RefCell<Option<Box<dyn Placement>>> = const { RefCell::new(None) };
}

/// Decides where the nodes of a layered graph stand along their layers.
pub trait Placement {
    /// Sets [`Node::place`] of the nodes, and [`Port::at`] of the ports it moves. The order of the
    /// nodes in a layer and their least distances ([`Node::gap`]) hold: a result that breaks them
    /// is dropped, and the graph keeps ELK's placement.
    fn place(&self, layers: &mut Layers);
}

/// What a node of a layer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A node of the graph.
    Node,
    /// The place where a long edge passes a layer.
    Bend,
    /// The label of an edge, which takes room in a layer of its own.
    Label,
    /// The port of the graph around this one, for an edge that leaves it.
    Border,
}

/// The side of a node a port is on, along the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    /// Towards the layer before.
    Before,
    /// Towards the layer after.
    After,
}

/// A node of a layer.
#[derive(Debug, Clone)]
pub struct Node {
    pub kind: Kind,
    /// Its size along the layer.
    pub size: f64,
    /// Where it starts along the layer.
    pub place: f64,
    /// The least distance of its place from the place of the node before it in the layer, 0 for
    /// the first.
    pub gap: f64,
    /// Its ports, as indices into [`Layers::ports`].
    pub ports: Vec<usize>,
}

/// A port: where edges meet a node.
#[derive(Debug, Clone)]
pub struct Port {
    /// Its node, as an index into [`Layers::nodes`].
    pub node: usize,
    pub side: Side,
    /// Where its edges meet the node along the layer, from the node's place.
    pub at: f64,
    /// Its edges, as indices into [`Layers::edges`].
    pub edges: Vec<usize>,
}

/// An edge between two layers.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    /// The port it leaves, as an index into [`Layers::ports`].
    pub source: usize,
    /// The port it enters.
    pub target: usize,
    /// Whether it runs against its direction in the diagram: the layout turned it around to break
    /// a cycle.
    pub reversed: bool,
}

/// A layered graph between its node placement and its edge routing, as plain data.
#[derive(Debug, Clone)]
pub struct Layers {
    /// The layers along the flow, each with its nodes in order, as indices into `nodes`.
    pub layers: Vec<Vec<usize>>,
    pub nodes: Vec<Node>,
    /// The ports that have an edge between two layers.
    pub ports: Vec<Port>,
    /// The edges between two layers. A self-loop is not among them.
    pub edges: Vec<Edge>,
    /// The effort the graph asks of the network simplex method (`elk.layered.thoroughness`).
    thoroughness: i32,
}

impl Layers {
    /// How far the source end of an edge is past its target end along the layer.
    #[must_use]
    pub fn step(&self, edge: usize) -> f64 {
        let (source, target) = (
            &self.ports[self.edges[edge].source],
            &self.ports[self.edges[edge].target],
        );
        (self.nodes[source.node].place + source.at) - (self.nodes[target.node].place + target.at)
    }

    /// The places at which the weighted lengths of the steps of the edges are least, with the
    /// order and the least distances of the nodes kept: the network simplex method on the graph
    /// that ELK's own placement builds, with the caller's weight for each edge of [`Self::edges`].
    #[must_use]
    pub fn solve(&self, weights: &[f64]) -> Vec<f64> {
        let mut network = NGraph::new();
        let mut rep = vec![usize::MAX; self.nodes.len()];
        for members in &self.layers {
            for (index, &node) in members.iter().enumerate() {
                rep[node] = network.add_node(NNode::new());
                if index > 0 {
                    let mut apart = NEdge::new(rep[members[index - 1]], rep[node]);
                    apart.weight = 0.0;
                    apart.delta = self.nodes[node].gap as i32;
                    network.add_edge(apart);
                }
            }
        }
        for (edge, weight) in self.edges.iter().zip(weights) {
            let (source, target) = (&self.ports[edge.source], &self.ports[edge.target]);
            let between = network.add_node(NNode::new());
            for (end, delta) in [
                (source.node, (target.at - source.at).max(0.0)),
                (target.node, (source.at - target.at).max(0.0)),
            ] {
                let mut pull = NEdge::new(between, rep[end]);
                pull.weight = *weight;
                pull.delta = delta as i32;
                network.add_edge(pull);
            }
        }
        network.make_connected();
        let limit = self.thoroughness.wrapping_mul(network.nodes().len() as i32);
        NetworkSimplex::for_graph(&mut network)
            .with_iteration_limit(limit)
            .with_balancing(false)
            .execute();
        rep.iter()
            .map(|&node| {
                if node == usize::MAX {
                    0.0
                } else {
                    f64::from(network.node(node).layer)
                }
            })
            .collect()
    }

    /// Whether the nodes of every layer keep their least distances.
    fn holds(&self) -> bool {
        self.nodes.iter().all(|node| node.place.is_finite())
            && self.ports.iter().all(|port| port.at.is_finite())
            && self.layers.iter().all(|members| {
                members.windows(2).all(|pair| {
                    self.nodes[pair[1]].place - self.nodes[pair[0]].place
                        >= self.nodes[pair[1]].gap - 1e-6
                })
            })
    }
}

/// Lays out an elkjs JSON graph as [`crate::layout`] does, with the caller's node placement run
/// after the network simplex placement of each graph that uses it. A graph with another placement
/// strategy keeps that strategy's result.
///
/// # Errors
///
/// Returns an error if the graph cannot be imported or laid out.
pub fn layout_with<P: Placement + 'static>(
    graph: &json::JsonNode,
    logging: bool,
    placement: P,
) -> Result<Layout, LayoutError> {
    struct Reset(Option<Box<dyn Placement>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            PLACEMENT.with(|cell| *cell.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(PLACEMENT.with(|cell| cell.borrow_mut().replace(Box::new(placement))));
    crate::layout(graph, logging)
}

/// Runs the placement of the layout on this thread on a graph whose nodes ELK has placed. A graph
/// with an edge inside a layer, or with a dummy the view has no kind for, keeps its placement.
pub(crate) fn run(graph: &mut LGraph) {
    PLACEMENT.with(|cell| {
        let placement = cell.borrow();
        let Some(placement) = placement.as_deref() else {
            return;
        };
        let Some((mut layers, nodes, ports)) = view(graph) else {
            return;
        };
        placement.place(&mut layers);
        if !layers.holds() {
            return;
        }
        for (index, &node) in nodes.iter().enumerate() {
            graph.nodes[node].position.y = layers.nodes[index].place;
        }
        for (index, &port) in ports.iter().enumerate() {
            graph.ports[port].position.y = layers.ports[index].at - graph.ports[port].anchor.y;
        }
    });
}

/// The graph as plain data, with the nodes and the ports of the graph that its nodes and ports
/// stand for.
fn view(graph: &LGraph) -> Option<(Layers, Vec<usize>, Vec<usize>)> {
    let mut node_of = Vec::new();
    let mut index_of_node = vec![usize::MAX; graph.nodes.len()];
    let mut nodes = Vec::new();
    let mut layers = Vec::new();
    for members in &graph.layers {
        let mut layer = Vec::new();
        for (index, &node) in members.iter().enumerate() {
            let this = &graph.nodes[node];
            let kind = match this.node_type {
                NodeType::NORMAL => Kind::Node,
                NodeType::LONG_EDGE => Kind::Bend,
                NodeType::LABEL => Kind::Label,
                NodeType::EXTERNAL_PORT => Kind::Border,
                NodeType::NORTH_SOUTH_PORT | NodeType::BREAKING_POINT => return None,
            };
            let gap = if index == 0 {
                0.0
            } else {
                let before = &graph.nodes[members[index - 1]];
                (before.size.y
                    + before.margin.bottom
                    + spacings::vertical(graph, before, this)
                    + this.margin.top)
                    .ceil()
            };
            index_of_node[node] = nodes.len();
            layer.push(nodes.len());
            node_of.push(node);
            nodes.push(Node {
                kind,
                size: this.size.y,
                place: this.position.y,
                gap,
                ports: Vec::new(),
            });
        }
        layers.push(layer);
    }

    let mut port_of = Vec::new();
    let mut index_of_port = vec![usize::MAX; graph.ports.len()];
    let mut ports: Vec<Port> = Vec::new();
    let mut edges = Vec::new();
    for (edge, ends) in graph.edges.iter().enumerate() {
        if is_self_loop(graph, edge) {
            continue;
        }
        let (from, to) = (graph.ports[ends.source].node, graph.ports[ends.target].node);
        if index_of_node[from] == usize::MAX || index_of_node[to] == usize::MAX {
            continue;
        }
        if is_in_layer_edge(graph, edge) {
            return None;
        }
        let mut end = |port: usize| -> Option<usize> {
            if index_of_port[port] == usize::MAX {
                let side = match graph.ports[port].side {
                    PortSide::WEST => Side::Before,
                    PortSide::EAST => Side::After,
                    _ => return None,
                };
                let node = index_of_node[graph.ports[port].node];
                index_of_port[port] = ports.len();
                nodes[node].ports.push(ports.len());
                port_of.push(port);
                ports.push(Port {
                    node,
                    side,
                    at: graph.ports[port].position.y + graph.ports[port].anchor.y,
                    edges: Vec::new(),
                });
            }
            Some(index_of_port[port])
        };
        let (source, target) = (end(ends.source)?, end(ends.target)?);
        ports[source].edges.push(edges.len());
        ports[target].edges.push(edges.len());
        edges.push(Edge {
            source,
            target,
            reversed: ends.reversed,
        });
    }
    let thoroughness = graph.properties.int_with_default(options::THOROUGHNESS);
    Some((
        Layers {
            layers,
            nodes,
            ports,
            edges,
            thoroughness,
        },
        node_of,
        port_of,
    ))
}
