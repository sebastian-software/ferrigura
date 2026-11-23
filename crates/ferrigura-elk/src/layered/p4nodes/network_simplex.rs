//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p4nodes/NetworkSimplexPlacer.java
//!
//! Network simplex node placement, `NetworkSimplexPlacer`. The positions are the layers of an
//! auxiliary graph that the network simplex solver ([`crate::common::networksimplex`]) lays out, as
//! Gansner et al. describe it. This file holds the whole placer: the auxiliary graph, the straight
//! edge pre- and post-processing, the flexible nodes, and the application of the positions.
//!
//! A layered node, edge or port is its index in `LGraph::nodes`, `LGraph::edges` or `LGraph::ports`,
//! and a network simplex node or edge is its index in the [`NGraph`]. Java's `lNode.id` and `lEdge.id`
//! only index arrays, so the arena indices stand for them. Where Java iterates an array by id, which
//! is the order of the layers, this port iterates the nodes in layer order.
//!
//! Java's `HashMap` of ports and the `HashSet` of flexible edges are never iterated in order: the
//! port map is a vector indexed by port, and the flexible edges are a vector in insertion order, all
//! of which get the same new weight. Java `assert` statements are not ported, since elkjs is compiled
//! by GWT, which drops them.
//!
//! `getLayoutProcessorConfiguration` is in `phases::PhaseStep::configuration`, the same table as for
//! the Brandes & Koepf placer.

use std::collections::VecDeque;

use crate::common::networksimplex::{NEdge, NGraph, NNode, NetworkSimplex};
use crate::core::data::OptionValue;
use crate::core::data::enums::{
    NodeFlexibility, NodePlacementStrategy, NodeType, PortConstraints, PortSide,
};
use crate::core::math::Spacing;
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::java::math::java_round;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::p4nodes::bk::access::{
    connected_edges, is_in_layer_edge, is_self_loop, java_max, outgoing_edges,
};
use crate::layered::spacings::{self, SPACING_PORT_PORT};

/// `NetworkSimplexPlacer.EDGE_WEIGHT_BASE`.
const EDGE_WEIGHT_BASE: f64 = 4.0;
/// `NetworkSimplexPlacer.SMALL_EDGE_WEIGHT`: smaller than the default, since horizontal edges matter more.
const SMALL_EDGE_WEIGHT: f64 = 0.1;
/// `NetworkSimplexPlacer.LONG_EDGE_VS_PATH_FACTOR`.
const LONG_EDGE_VS_PATH_FACTOR: f64 = 2.0;
/// `NetworkSimplexPlacer.NODE_SIZE_WEIGHT_STATIC`: a node that must not change in size.
const NODE_SIZE_WEIGHT_STATIC: f64 = 10000.0;
/// `NetworkSimplexPlacer.NODE_SIZE_WEIGHT_FLEXIBLE`: a node that may change in size.
const NODE_SIZE_WEIGHT_FLEXIBLE: f64 = 1.0;
/// `NetworkSimplexPlacer.EPSILON`: for double equality tests.
const EPSILON: f64 = 0.00001;

/// `NetworkSimplexPlacer.VISITED`: the node has not been visited yet, and is not a junction.
const VISITED: i32 = -1;
/// `NetworkSimplexPlacer.OTHER`: the node is not a junction.
const OTHER: i32 = 0;
/// `NetworkSimplexPlacer.JUNCTION`.
const JUNCTION: i32 = 2;

/// What a network simplex node stands for: `NNode.origin` in Java, an `LNode`, an `LPort` or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    None,
    Node(usize),
    Port(usize),
}

/// `NetworkSimplexPlacer.NodeRep`: the network simplex nodes of a layered node.
#[derive(Debug, Clone, Copy)]
struct NodeRep {
    /// The layered node, an index into `LGraph::nodes`.
    origin: usize,
    /// True if the node is flexible, so that its head and tail are two different network simplex nodes.
    is_flexible: bool,
    /// The 'head' of the node, the border with the lower y coordinate.
    head: usize,
    /// The 'tail' of the node, the border with the larger y coordinate.
    tail: usize,
}

/// `NetworkSimplexPlacer.EdgeRep`: the two network simplex edges that stand for a layered edge.
#[derive(Debug, Clone, Copy)]
struct EdgeRep {
    /// The edge from the dummy to the source port.
    left: usize,
    /// The edge from the dummy to the target port.
    right: usize,
}

/// A path of layered edges, `NetworkSimplexPlacer.Path`. Its edges are indices into `LGraph::edges`.
type Path = Vec<usize>;

/// The auxiliary graph and the bookkeeping of one run of `NetworkSimplexPlacer.process`.
struct Placer<'a> {
    /// `NetworkSimplexPlacer.lGraph`.
    graph: &'a mut LGraph,
    /// `NetworkSimplexPlacer.nGraph`.
    ng: NGraph,
    /// `NetworkSimplexPlacer.portMap`: the network simplex node of each east or west port, by port.
    port_map: Vec<Option<usize>>,
    /// The origin of each network simplex node, by its index. Nodes made after the graph was made
    /// connected may lack an entry, which Java gives no origin either.
    origins: Vec<Origin>,
    /// The layered nodes in layer order: the order in which Java assigns `LNode.id`.
    order: Vec<usize>,
    /// The layer of each layered node, by node; `usize::MAX` for a node in no layer.
    node_layer: Vec<usize>,
    /// `LNode.getIndex()` of each layered node: its position in its layer.
    index_in_layer: Vec<usize>,
    /// `NetworkSimplexPlacer.nodeReps`, by layered node.
    node_reps: Vec<Option<NodeRep>>,
    /// `NetworkSimplexPlacer.edgeReps`, by layered edge.
    edge_reps: Vec<Option<EdgeRep>>,
    /// `NetworkSimplexPlacer.nodeState`, by layered node.
    node_state: Vec<i32>,
    /// `NetworkSimplexPlacer.twoPaths`.
    two_paths: Vec<Path>,
    /// `NetworkSimplexPlacer.crossing`, by layered edge.
    crossing: Vec<bool>,
    /// `NetworkSimplexPlacer.flexibleWhereSpacePermitsEdges`, in insertion order.
    flexible_where_space: Vec<usize>,
}

/// `NetworkSimplexPlacer.process`: sets the y coordinate of every node.
pub fn place(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Network simplex node placement", 1.0);

    let favor_straight_edges = graph
        .properties
        .bool_of(options::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES);
    let thoroughness = graph.properties.int_with_default(options::THOROUGHNESS);
    let mut placer = Placer::new(graph);

    // #1 build the auxiliary graph
    placer.prepare();
    placer.build_initial_auxiliary_graph()?;
    placer.insert_north_south_auxiliary_edges()?;
    placer.insert_in_layer_edge_auxiliary_edges()?;

    if favor_straight_edges {
        let pm = sub_task(progress, monitor)?;
        progress.begin(pm, "Straight Edges Pre-Processing", 1.0);
        placer.prefer_straight_edges();
        progress.done(pm);
    }

    // make sure the ngraph is connected: hierarchical nodes with unconnected ports can leave
    // several components
    placer.ng.make_connected();

    // #2 execute the network simplex
    // a significantly larger iteration limit than the layerer's, since the auxiliary graph is bigger
    let iter_limit = thoroughness.wrapping_mul(placer.ng.nodes().len() as i32);

    let sub = sub_task(progress, monitor)?;
    NetworkSimplex::for_graph(&mut placer.ng)
        .with_iteration_limit(iter_limit)
        .with_balancing(false)
        .execute_with_monitor(progress, sub);

    // nodes that may resize where space permits: their size edges were collected before
    if !placer.flexible_where_space.is_empty() {
        let pm = sub_task(progress, monitor)?;
        progress.begin(pm, "Flexible Where Space Processing", 1.0);

        placer.insert_flexible_where_space_auxiliary_edges();
        // now the nodes may resize -> alter the weights
        for edge in placer.flexible_where_space.clone() {
            placer.ng.edge_mut(edge).weight = NODE_SIZE_WEIGHT_FLEXIBLE;
        }

        // run network simplex a second time
        let sub = sub_task(progress, pm)?;
        NetworkSimplex::for_graph(&mut placer.ng)
            .with_iteration_limit(iter_limit)
            .with_balancing(false)
            .execute_with_monitor(progress, sub);

        progress.done(pm);
    }

    // post process 'two paths' that have been identified during the call of preferStraightEdges
    if favor_straight_edges {
        let pm = sub_task(progress, monitor)?;
        progress.begin(pm, "Straight Edges Post-Processing", 1.0);
        placer.post_process_two_paths();
        progress.done(pm);
    }

    // #3 apply positions
    placer.apply_positions()?;

    progress.done(monitor);
    Ok(())
}

/// `progressMonitor.subTask(1)`, which fails once `monitor` is done.
fn sub_task(progress: &mut ProgressMonitor, monitor: Monitor) -> Result<Monitor, LayoutError> {
    progress
        .sub_task(monitor, 1.0)
        .ok_or_else(|| LayoutError::Unsupported("subtask of a finished monitor".to_string()))
}

impl<'a> Placer<'a> {
    fn new(graph: &'a mut LGraph) -> Self {
        let node_count = graph.nodes.len();
        let edge_count = graph.edges.len();
        let port_count = graph.ports.len();

        let mut order = Vec::new();
        let mut node_layer = vec![usize::MAX; node_count];
        let mut index_in_layer = vec![0; node_count];
        for (layer, members) in graph.layers.iter().enumerate() {
            for (index, &node) in members.iter().enumerate() {
                node_layer[node] = layer;
                index_in_layer[node] = index;
                order.push(node);
            }
        }

        Placer {
            graph,
            ng: NGraph::new(),
            port_map: vec![None; port_count],
            origins: Vec::new(),
            order,
            node_layer,
            index_in_layer,
            node_reps: vec![None; node_count],
            edge_reps: vec![None; edge_count],
            node_state: Vec::new(),
            two_paths: Vec::new(),
            crossing: Vec::new(),
            flexible_where_space: Vec::new(),
        }
    }

    // ------------------------------------------------------------------------------------------------
    //                                         Preparation
    // ------------------------------------------------------------------------------------------------

    /// `prepare`: makes the port anchor and the port position integral.
    fn prepare(&mut self) {
        // "integerify" port anchor and port positions
        //   note that margin.top and margin.bottom are not required to be integral
        //   since they do not influence the offset calculation for the edges
        for index in 0..self.order.len() {
            let node = self.order[index];
            // if a node is flexible, an edge attaches to the port itself within the auxiliary
            // graph, thus the anchor must be integer; otherwise the port position can be altered
            // such that it accounts for the anchor's position as well
            let anchor_must_be_integer = is_flexible_node(self.graph, node);
            let ports = self.graph.nodes[node].ports.clone();
            for port in ports {
                if anchor_must_be_integer {
                    // anchor
                    let y = self.graph.ports[port].anchor.y;
                    if y != y.floor() {
                        let offset = y - java_round(y) as f64;
                        self.graph.ports[port].anchor.y -= offset;
                    }
                }

                // port + anchor
                let y = self.graph.ports[port].position.y + self.graph.ports[port].anchor.y;
                if y != y.floor() {
                    let offset = y - java_round(y) as f64;
                    self.graph.ports[port].position.y -= offset;
                }
            }
        }
        self.flexible_where_space.clear();
    }

    // ------------------------------------------------------------------------------------------------
    //                                      Auxiliary Graph
    // ------------------------------------------------------------------------------------------------

    /// `buildInitialAuxiliaryGraph`: the nodes of each layer, then the edges.
    fn build_initial_auxiliary_graph(&mut self) -> Result<(), LayoutError> {
        for layer in 0..self.graph.layers.len() {
            self.transform_layer(layer);
        }
        self.transform_edges()
    }

    /// `transformLayer`: transforms the nodes of a layer and adds the edges that separate them.
    fn transform_layer(&mut self, layer: usize) {
        let members = self.graph.layers[layer].clone();
        let mut last_rep: Option<NodeRep> = None;
        for node in members {
            let node_rep = if is_flexible_node(self.graph, node) {
                self.transform_fixed_order_node(node)
            } else {
                self.transform_fixed_pos_node(node)
            };
            self.node_reps[node] = Some(node_rep);

            // if there is a previous node in the layer, we have to create a separation edge
            if let Some(last) = last_rep {
                let last_node = &self.graph.nodes[last.origin];
                let mut spacing = last_node.margin.bottom
                    + spacings::vertical(self.graph, last_node, &self.graph.nodes[node])
                    + self.graph.nodes[node].margin.top;

                if !last.is_flexible {
                    // for non-flexible nodes their height must be included
                    // in the minimal length of the separation edge
                    spacing += self.graph.nodes[last.origin].size.y;
                }

                self.add_edge(last.tail, node_rep.head, 0.0, spacing.ceil() as i32, None);
            }

            // remember current elements for next iteration
            last_rep = Some(node_rep);
        }
    }

    /// `transformFixedPosNode`: one network simplex node for the whole layered node.
    fn transform_fixed_pos_node(&mut self, node: usize) -> NodeRep {
        let single = self.new_node(Origin::Node(node), "non-flexible");

        // register the ports with the node
        for port in self.graph.nodes[node].ports.clone() {
            if is_east_west(self.graph.ports[port].side) {
                self.port_map[port] = Some(single);
            }
        }

        NodeRep {
            origin: node,
            is_flexible: false,
            head: single,
            tail: single,
        }
    }

    /// `transformFixedOrderNode`: the top and bottom border of a node, and its ports between them.
    fn transform_fixed_order_node(&mut self, node: usize) -> NodeRep {
        // corner creation
        let top_left = self.new_node(Origin::Node(node), "flexible-head");
        let bottom_left = self.new_node(Origin::Node(node), "flexible-tail");
        let corners = NodeRep {
            origin: node,
            is_flexible: true,
            head: top_left,
            tail: bottom_left,
        };

        // weight & minimum length
        // the LabelAndNodeSizeProcessor ensures that any node size constraint is satisfied,
        // as such we can safely use the current height as minimum height
        let min_height = self.graph.nodes[node].size.y;

        let nf = node_flexibility(self.graph, node);
        let mut size_weight = NODE_SIZE_WEIGHT_STATIC;
        if is_flexible_size(nf) {
            // we are allowed to enlarge to node
            // nevertheless, a little weight is good, otherwise the node can become arbitrarily tall
            size_weight = NODE_SIZE_WEIGHT_FLEXIBLE;
        }

        let node_size_edge = self.add_edge(
            top_left,
            bottom_left,
            size_weight,
            min_height.ceil() as i32,
            None,
        );

        if nf == NodeFlexibility::NODE_SIZE_WHERE_SPACE_PERMITS {
            self.flexible_where_space.push(node_size_edge);
        }

        // port transformation
        // convert the ports to NNodes, note that the list of westward ports
        // must be reversed since their original order is from bottom to top
        let mut west = port_side_view(self.graph, node, PortSide::WEST);
        west.reverse();
        self.transform_ports(&west, corners);
        let east = port_side_view(self.graph, node, PortSide::EAST);
        self.transform_ports(&east, corners);

        // return the corners for further processing
        corners
    }

    /// `transformPorts`: a network simplex node for each port, spaced apart.
    fn transform_ports(&mut self, ports: &[usize], corners: NodeRep) {
        if ports.is_empty() {
            // nothing to do ...
            // the top and bottom border of the node are already safely spaced apart
            return;
        }

        let port_spacing = self.graph.properties.double_with_default(SPACING_PORT_PORT);
        let port_surrounding = match self
            .graph
            .properties
            .value_or_default(options::SPACING_PORTS_SURROUNDING)
        {
            Some(OptionValue::Margin(margin)) => *margin,
            // No additional port spacing set
            _ => Spacing::default(),
        };

        let mut last_n_node = corners.head;
        let mut last_port: Option<usize> = None;
        for &port in ports {
            // spacing between the current pair of ports (or to the top border of the node)
            let spacing = match last_port {
                None => port_surrounding.top,
                Some(last) => port_spacing + self.graph.ports[last].size.y,
            };

            // create NNode for the port
            let n_node = self.new_node(Origin::Port(port), "port");
            self.port_map[port] = Some(n_node);

            // connect with previous NNode
            self.add_edge(last_n_node, n_node, 0.0, spacing.ceil() as i32, None);

            last_port = Some(port);
            last_n_node = n_node;
        }

        // and connect to the bottom border
        let last = last_port.expect("the port list is not empty");
        let delta = (port_surrounding.bottom + self.graph.ports[last].size.y).ceil() as i32;
        self.add_edge(last_n_node, corners.tail, 0.0, delta, None);
    }

    /// `transformEdges`: each edge that is neither a self loop nor an in-layer edge.
    fn transform_edges(&mut self) -> Result<(), LayoutError> {
        for node in self.order.clone() {
            for edge in outgoing_edges(self.graph, &self.graph.nodes[node]) {
                if is_handled_edge(self.graph, edge) {
                    self.transform_edge(edge)?;
                }
            }
        }
        Ok(())
    }

    /// `transformEdge`: a dummy node with an edge to each of the edge's ports.
    fn transform_edge(&mut self, edge: usize) -> Result<(), LayoutError> {
        // a dummy node
        let dummy = self.new_node(Origin::None, "edge");

        // calculate port offsets
        let src_port = self.graph.edges[edge].source;
        let tgt_port = self.graph.edges[edge].target;
        let src_rep = self.rep(self.graph.ports[src_port].node);
        let tgt_rep = self.rep(self.graph.ports[tgt_port].node);

        let mut src_offset = self.graph.ports[src_port].anchor.y;
        let mut tgt_offset = self.graph.ports[tgt_port].anchor.y;
        // for non-flexible nodes, ports are relative to node positions
        if !src_rep.is_flexible {
            src_offset += self.graph.ports[src_port].position.y;
        }
        if !tgt_rep.is_flexible {
            tgt_offset += self.graph.ports[tgt_port].position.y;
        }

        let tgt_delta = java_max(0.0, src_offset - tgt_offset) as i32;
        let src_delta = java_max(0.0, tgt_offset - src_offset) as i32;

        let weight = edge_weight(self.graph, edge);

        // an edge to the source
        let left_target = self.port_rep(src_port)?;
        let left = self.add_edge(dummy, left_target, weight, src_delta, Some(edge));

        // an edge to the target
        let right_target = self.port_rep(tgt_port)?;
        let right = self.add_edge(dummy, right_target, weight, tgt_delta, Some(edge));

        // remember
        self.edge_reps[edge] = Some(EdgeRep { left, right });
        Ok(())
    }

    /// `insertInLayerEdgeAuxiliaryEdges`: keeps the edges to inverted ports short.
    fn insert_in_layer_edge_auxiliary_edges(&mut self) -> Result<(), LayoutError> {
        for node in self.order.clone() {
            if self.graph.nodes[node].node_type != NodeType::NORMAL {
                continue;
            }
            for in_layer_edge in connected_edges(self.graph, &self.graph.nodes[node]) {
                if !is_in_layer_edge(self.graph, in_layer_edge) {
                    continue;
                }

                let src_port = self.graph.edges[in_layer_edge].source;
                let tgt_port = self.graph.edges[in_layer_edge].target;
                let src_is_dummy =
                    self.graph.nodes[self.graph.ports[src_port].node].node_type != NodeType::NORMAL;

                let the_port = if src_is_dummy { tgt_port } else { src_port };
                let other_port = if the_port == src_port {
                    tgt_port
                } else {
                    src_port
                };
                let dummy_node = self.graph.ports[other_port].node;

                let port_rep = self.port_rep(the_port)?;
                // head/tail doesn't matter since it's a dummy node
                let dummy_rep = self.rep(dummy_node).head;

                let (src, tgt) =
                    if self.index_of(self.graph.ports[the_port].node) < self.index_of(dummy_node) {
                        // port --> dummy
                        (port_rep, dummy_rep)
                    } else {
                        // dummy --> port
                        (dummy_rep, port_rep)
                    };

                self.add_edge(src, tgt, EDGE_WEIGHT_BASE, 0, None);
            }
        }
        Ok(())
    }

    /// `insertNorthSouthAuxiliaryEdges`: keeps the north and south port edges short.
    fn insert_north_south_auxiliary_edges(&mut self) -> Result<(), LayoutError> {
        for node in self.order.clone() {
            for port in port_side_view(self.graph, node, PortSide::SOUTH) {
                // if no edge was attached to the port, no dummy was created ...
                if let Some(other) = self.graph.ports[port].north_south_dummy {
                    let source = self.rep(node).tail;
                    let target = self.rep(other).head;
                    // delta doesn't matter, separation is already taken care off
                    self.add_edge(source, target, SMALL_EDGE_WEIGHT, 0, None);
                }
            }

            for port in port_side_view(self.graph, node, PortSide::NORTH) {
                // if no edge was attached to the port, no dummy was created ...
                if let Some(other) = self.graph.ports[port].north_south_dummy {
                    let source = self.rep(other).tail;
                    let target = self.rep(node).head;
                    self.add_edge(source, target, SMALL_EDGE_WEIGHT, 0, None);
                }
            }
        }
        Ok(())
    }

    /// `insertFlexibleWhereSpaceAuxiliaryEdges`: for the nodes that may resize where space permits.
    fn insert_flexible_where_space_auxiliary_edges(&mut self) {
        let layers: Vec<i32> = self
            .ng
            .nodes()
            .iter()
            .map(|&node| self.ng.node(node).layer)
            .collect();
        let min_layer = *layers.iter().min().expect("the graph has nodes");
        let max_layer = *layers.iter().max().expect("the graph has nodes");
        let used_layers = max_layer.wrapping_sub(min_layer);

        let global_source = self.new_node(Origin::None, "");
        let global_sink = self.new_node(Origin::None, "");

        // make sure the distance between source and sink is preserved
        self.add_edge(
            global_source,
            global_sink,
            NODE_SIZE_WEIGHT_STATIC * 2.0,
            used_layers,
            None,
        );

        // fix the position of most non-flexible nodes and make sure the flexible nodes
        // can only increase in size
        for node in self.order.clone() {
            let rep = self.rep(node);
            if self.graph.nodes[node].node_type != NodeType::NORMAL {
                continue;
            }
            if self.graph.nodes[node].ports.len() <= 1 {
                // allow leaves to move
                continue;
            }
            let tail_layer = self.ng.node(rep.tail).layer;
            self.add_edge(
                global_source,
                rep.tail,
                0.0,
                tail_layer.wrapping_sub(min_layer),
                None,
            );

            let head_layer = self.ng.node(rep.head).layer;
            self.add_edge(
                rep.head,
                global_sink,
                0.0,
                used_layers.wrapping_sub(head_layer),
                None,
            );
        }
    }

    // ------------------------------------------------------------------------------------------------
    //                                       Apply Layout
    // ------------------------------------------------------------------------------------------------

    /// `applyPositions`: the y coordinates, sizes, port positions and label positions.
    fn apply_positions(&mut self) -> Result<(), LayoutError> {
        for node in self.order.clone() {
            // find the node's corners
            let node_rep = self.rep(node);
            let min_y = f64::from(self.ng.node(node_rep.head).layer);
            let max_y = f64::from(self.ng.node(node_rep.tail).layer);

            // set new position and size
            self.graph.nodes[node].position.y = min_y;

            let size_delta = (max_y - min_y) - self.graph.nodes[node].size.y;

            let flexible_node = is_flexible_node(self.graph, node);
            let nf = node_flexibility(self.graph, node);

            // modify the size?
            if flexible_node && is_flexible_size_where_space_permits(nf) {
                self.graph.nodes[node].size.y += size_delta;
            }

            // reposition ports if allowed
            if flexible_node && is_flexible_ports(nf) {
                for port in self.graph.nodes[node].ports.clone() {
                    if is_east_west(self.graph.ports[port].side) {
                        let n_node = self.port_rep(port)?;
                        self.graph.ports[port].position.y =
                            f64::from(self.ng.node(n_node).layer) - min_y;
                    }
                }
                // when the node got resized, the positions of labels and south ports have to be adjusted
                for label in self.graph.nodes[node].labels.clone() {
                    self.adjust_label_position(node, label, size_delta);
                }
                if is_flexible_size_where_space_permits(nf) {
                    for port in port_side_view(self.graph, node, PortSide::SOUTH) {
                        self.graph.ports[port].position.y += size_delta;
                    }
                }
            }
        }
        Ok(())
    }

    /// `adjustLabelPosition`: moves a label of a resized node along with its placement.
    fn adjust_label_position(&mut self, node: usize, label: usize, size_delta: f64) {
        let placement = &self.graph.nodes[node].properties;
        let v_bottom = placement.enum_set_contains(options::NODE_LABELS_PLACEMENT, "V_BOTTOM");
        let v_center = placement.enum_set_contains(options::NODE_LABELS_PLACEMENT, "V_CENTER");
        if v_bottom {
            self.graph.labels[label].position.y += size_delta;
        } else if v_center {
            self.graph.labels[label].position.y += size_delta / 2.0;
        }
        // V_TOP placement does not require adjustment
    }

    // ------------------------------------------------------------------------------------------------
    //                                       Straight Edges
    // ------------------------------------------------------------------------------------------------

    /// `preferStraightEdges`: potentially alters the weights of paths in a similar fashion to long
    /// edges, and collects the 'two paths' for later post processing.
    fn prefer_straight_edges(&mut self) {
        // the nodes were counted and indexed during #prepare
        self.node_state = vec![0; self.graph.nodes.len()];
        self.two_paths = Vec::new();

        // record node states
        for node in self.order.clone() {
            self.node_state[node] = get_node_state(self.graph, node);
        }

        self.mark_edge_crossings();
        let identified_paths = self.identify_paths();

        // essentially 'long paths' are treated like 'long edges':
        // the weights of the connecting edges are altered to straighten the path as much as possible
        for path in identified_paths {
            if path.len() <= 1 {
                continue;
            }

            // remember 'two paths' for processing after network simplex has been executed
            if path.len() == 2 {
                let path = order_two_path(self.graph, path);
                if !is_two_path_center_node_flexible(self.graph, &path) {
                    self.two_paths.push(path);
                }
                continue;
            }

            // ignore paths that contain long edge dummies, and paths that contain flexible nodes
            // that allow resizing
            if contains_long_edge_dummy(self.graph, &path)
                || contains_flexible_node(self.graph, &path, is_flexible_size_where_space_permits)
            {
                continue;
            }

            let path_len = path.len();
            for (index, &cur) in path.iter().enumerate() {
                let cur_rep = self.edge_rep(cur);

                // first or last segment
                let mut weight = if index == 0 || index == path_len - 1 {
                    edge_weight_of_types(NodeType::NORMAL, NodeType::LONG_EDGE)
                } else {
                    edge_weight_of_types(NodeType::LONG_EDGE, NodeType::LONG_EDGE)
                };

                // at this point one can decide whether long edges are more important than "paths"
                weight *= LONG_EDGE_VS_PATH_FACTOR;

                let old_left_weight = self.ng.edge(cur_rep.left).weight;
                self.ng.edge_mut(cur_rep.left).weight = java_max(
                    old_left_weight,
                    old_left_weight + (weight - old_left_weight),
                );
                let old_right_weight = self.ng.edge(cur_rep.right).weight;
                self.ng.edge_mut(cur_rep.right).weight = java_max(
                    old_right_weight,
                    old_right_weight + (weight - old_right_weight),
                );
            }
        }
    }

    /// `postProcessTwoPaths`: iteratively improves the 'two paths' until no bend can be removed.
    fn post_process_two_paths(&mut self) {
        let mut queue: VecDeque<Path> = self.two_paths.iter().cloned().collect();

        let mut stack: Vec<Path> = Vec::new();
        while let Some(path) = queue.pop_front() {
            let try_again = self.improve_two_path(&path, true);
            if try_again {
                stack.push(path);
            }
        }

        while let Some(path) = stack.pop() {
            self.improve_two_path(&path, false);
        }
    }

    /// `improveTwoPath`: moves the center node of a two path, if that straightens one of its edges.
    ///
    /// A two path `(u, center, v)` is ordered left to right, and its center node is not flexible.
    /// Returns `true` if nothing was changed and the path should be checked again later.
    fn improve_two_path(&mut self, path: &Path, probe: bool) -> bool {
        let left_edge = self.edge_rep(path[0]);
        let right_edge = self.edge_rep(path[1]);

        // is the edge already straight?
        if self.is_straight(left_edge) && self.is_straight(right_edge) {
            return false;
        }

        // get center node
        // the target of the left edge's right edge is a node or a port
        let center_target = self.ng.edge(left_edge.right).target();
        let center_node = match self.origin_of(center_target) {
            Origin::Node(node) => node,
            _ => return false,
        };
        let n_node = self.rep(center_node);

        // identify on which side there is more space
        let node_index = self.index_of(center_node);
        let layer_members = &self.graph.layers[self.node_layer[center_node]];
        let mut above_dist = f64::INFINITY;
        if node_index > 0 {
            let above = layer_members[node_index - 1];
            let above_rep = self.rep(above);
            let spacing = spacings::vertical(
                self.graph,
                &self.graph.nodes[above],
                &self.graph.nodes[center_node],
            )
            .ceil();
            above_dist = (f64::from(self.ng.node(n_node.head).layer)
                - self.graph.nodes[center_node].margin.top)
                - ((f64::from(self.ng.node(above_rep.head).layer)
                    + self.graph.nodes[above].size.y)
                    + self.graph.nodes[above].margin.bottom)
                - spacing;
        }
        let mut below_dist = f64::INFINITY;
        if node_index < layer_members.len() - 1 {
            let below = layer_members[node_index + 1];
            let below_rep = self.rep(below);
            let spacing = spacings::vertical(
                self.graph,
                &self.graph.nodes[below],
                &self.graph.nodes[center_node],
            )
            .ceil();
            below_dist = (f64::from(self.ng.node(below_rep.head).layer)
                - self.graph.nodes[below].margin.top)
                - ((f64::from(self.ng.node(n_node.head).layer)
                    + self.graph.nodes[center_node].size.y)
                    + self.graph.nodes[center_node].margin.bottom)
                - spacing;
        }

        // same space on both sides, check again later
        if probe && fuzzy_equals(above_dist, below_dist, EPSILON) {
            return true;
        }

        // from the way the auxiliary graph is built, one can differentiate four cases (A-D):
        //          o-v        u-o
        //         /              \            u-o   o-v         o-c-o
        //      o-c                c-o            \ /           /     \
        //     /                      \            c           u       v
        //    u                        v
        // Note that case C and D only occur if node c is blocked either above or below, respectively.
        // Otherwise the edge would be straight.
        //
        // In every case there are two non-straight edges. When routing edges orthogonally later on,
        // one saves a bend point if only one of the four edges is non-straight. To straighten one of the
        // two candidate edges, node c has to be moved up or down. It depends on the adjacent nodes in c's layer
        // if this is possible.

        // the following variables represent the length of each of the four edges,
        // i.e. if possible node c has to be moved by this distance (either up or down)
        // O--a--o--b--O--c--o--d--O
        let a = length(&self.ng, left_edge.left);
        let b = length(&self.ng, left_edge.right).wrapping_neg();
        let c = length(&self.ng, right_edge.left).wrapping_neg();
        let d = length(&self.ng, right_edge.right);

        let left_not_straight = not_straight_by(&self.ng, left_edge);
        let right_not_straight = not_straight_by(&self.ng, right_edge);
        let case_d = left_not_straight > 0 && right_not_straight < 0;
        let case_c = left_not_straight < 0 && right_not_straight > 0;
        let left_end = self.ng.node(self.ng.edge(left_edge.left).target()).layer;
        let right_end = self.ng.node(self.ng.edge(right_edge.right).target()).layer;
        let left_delta = self.ng.edge(left_edge.right).delta;
        let right_delta = self.ng.edge(right_edge.left).delta;
        let case_b = left_end.wrapping_add(left_delta) < right_end.wrapping_add(right_delta);
        let case_a = left_end.wrapping_add(left_delta) > right_end.wrapping_add(right_delta);

        let mut move_by: i32 = 0;
        if !case_d && !case_c {
            if case_a {
                if above_dist + f64::from(c) > 0.0 {
                    move_by = c;
                } else if below_dist - f64::from(a) > 0.0 {
                    move_by = a;
                }
            } else if case_b {
                if above_dist + f64::from(b) > 0.0 {
                    move_by = b;
                } else if below_dist - f64::from(d) > 0.0 {
                    move_by = d;
                }
            }
        }

        // move the center node
        let head = self.ng.node_mut(n_node.head);
        head.layer = head.layer.wrapping_add(move_by);
        if n_node.is_flexible {
            let tail = self.ng.node_mut(n_node.tail);
            tail.layer = tail.layer.wrapping_add(move_by);
        }

        false
    }

    // ------------------------------------------------------------------------------------------------
    //                                     Path identification
    // ------------------------------------------------------------------------------------------------

    /// `identifyPaths`: the paths that start at a junction.
    fn identify_paths(&mut self) -> Vec<Path> {
        let mut paths = Vec::new();
        for junction in self.order.clone() {
            if self.node_state[junction] != JUNCTION {
                continue;
            }
            for edge in connected_edges(self.graph, &self.graph.nodes[junction]) {
                if !is_handled_edge(self.graph, edge) {
                    continue;
                }
                let path = self.follow(edge, junction);
                if path.len() > 1 {
                    paths.push(path);
                }
            }
        }
        paths
    }

    /// `follow`: the path that continues from `edge`, which leaves `current`, until a junction, a
    /// visited node or a crossing is met.
    fn follow(&mut self, edge: usize, current: usize) -> Path {
        let mut path = Vec::new();
        let mut edge = edge;
        let mut current = current;
        loop {
            let other = other_node(self.graph, edge, current);
            path.push(edge);

            // stop criteria
            if self.node_state[other] == VISITED
                || self.node_state[other] == JUNCTION
                || self.crossing[edge]
            {
                return path;
            }

            // recurse
            self.node_state[other] = VISITED;
            let next = connected_edges(self.graph, &self.graph.nodes[other])
                .into_iter()
                .find(|&incident| is_handled_edge(self.graph, incident) && incident != edge);
            match next {
                Some(incident) => {
                    edge = incident;
                    current = other;
                }
                None => return path,
            }
        }
    }

    // ------------------------------------------------------------------------------------------------
    //                                      Mark Crossings
    // ------------------------------------------------------------------------------------------------

    /// `markEdgeCrossings`: marks the edges that are involved in edge crossings, between each pair of
    /// consecutive layers.
    fn mark_edge_crossings(&mut self) {
        self.crossing = vec![false; self.graph.edges.len()];
        for right in 1..self.graph.layers.len() {
            self.mark_crossing_edges(right - 1, right);
        }
    }

    /// `markCrossingEdges`: marks the crossings of edges between the two layers.
    fn mark_crossing_edges(&mut self, left: usize, right: usize) {
        // add all edges in the order they occur in the left layer
        let mut open_edges: Vec<usize> = Vec::new();
        for &node in &self.graph.layers[left] {
            for port in port_side_view(self.graph, node, PortSide::EAST) {
                for &edge in &self.graph.ports[port].outgoing {
                    if is_in_layer_edge(self.graph, edge)
                        || is_self_loop(self.graph, edge)
                        || self.node_layer[target_node(self.graph, edge)] != right
                    {
                        continue;
                    }
                    open_edges.push(edge);
                }
            }
        }

        // close the edges one after another, recording edge crossings
        for &node in self.graph.layers[right].iter().rev() {
            // don't reverse the ports, bottom up is correct
            for port in port_side_view(self.graph, node, PortSide::WEST) {
                for &edge in &self.graph.ports[port].incoming {
                    if is_in_layer_edge(self.graph, edge)
                        || is_self_loop(self.graph, edge)
                        || self.node_layer[source_node(self.graph, edge)] != left
                    {
                        continue;
                    }
                    if !open_edges.is_empty() {
                        // a Java ListIterator at the end of the list: `cursor` is its position
                        let mut cursor = open_edges.len();
                        cursor -= 1; // previous()
                        let mut last = open_edges[cursor];
                        while last != edge && cursor > 0 {
                            // mark both edges as being part of an edge crossing
                            self.crossing[last] = true;
                            self.crossing[edge] = true;
                            cursor -= 1; // previous()
                            last = open_edges[cursor];
                        }
                        if cursor > 0 {
                            // ListIterator.remove() of the element last returned by previous()
                            open_edges.remove(cursor);
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------------------------------------
    //                                       Convenience
    // ------------------------------------------------------------------------------------------------

    /// `EdgeRep.isStraight`.
    fn is_straight(&self, rep: EdgeRep) -> bool {
        not_straight_by(&self.ng, rep) == 0
    }

    /// `NetworkSimplexPlacer.addEdge`: a network simplex edge of the given weight and minimal length.
    fn add_edge(
        &mut self,
        source: usize,
        target: usize,
        weight: f64,
        delta: i32,
        origin: Option<usize>,
    ) -> usize {
        let mut edge = NEdge::new(source, target);
        edge.weight = weight;
        edge.delta = delta;
        edge.origin = origin;
        self.ng.add_edge(edge)
    }

    /// `NNode.of().origin(origin).type(node_type).create(nGraph)`.
    fn new_node(&mut self, origin: Origin, node_type: &str) -> usize {
        let mut node = NNode::new();
        node.origin = match origin {
            Origin::None => None,
            Origin::Node(index) | Origin::Port(index) => Some(index),
        };
        node.node_type = node_type.to_string();
        let index = self.ng.add_node(node);
        if self.origins.len() <= index {
            self.origins.resize(index + 1, Origin::None);
        }
        self.origins[index] = origin;
        index
    }

    /// The origin of a network simplex node; `Origin::None` for a node without one.
    fn origin_of(&self, node: usize) -> Origin {
        self.origins.get(node).copied().unwrap_or(Origin::None)
    }

    /// `nodeReps[node.id]`: the representation of a layered node, which exists once its layer is transformed.
    fn rep(&self, node: usize) -> NodeRep {
        self.node_reps[node].expect("every node of a layer has a node representation")
    }

    /// `edgeReps[edge.id]`: the representation of a handled edge.
    fn edge_rep(&self, edge: usize) -> EdgeRep {
        self.edge_reps[edge].expect("every handled edge has an edge representation")
    }

    /// `portMap.get(port)`. A port without one is a Java null, which fails the network simplex edge.
    fn port_rep(&self, port: usize) -> Result<usize, LayoutError> {
        self.port_map[port].ok_or_else(|| {
            LayoutError::Unsupported(
                "network simplex node placement found a port without a node".to_string(),
            )
        })
    }

    /// `LNode.getIndex()`: the position of the node in its layer.
    fn index_of(&self, node: usize) -> usize {
        self.index_in_layer[node]
    }
}

// ------------------------------------------------------------------------------------------------
//                                       Flexible Ports
// ------------------------------------------------------------------------------------------------

/// `NetworkSimplexPlacer.isFlexibleNode`: a node is flexible if its port constraints are not
/// `FIXED_POS`, and if its height is large enough for the ports on both sides to move.
fn is_flexible_node(graph: &LGraph, node: usize) -> bool {
    let lnode = &graph.nodes[node];

    // dummies are not flexible!
    if lnode.node_type != NodeType::NORMAL {
        return false;
    }

    // at least two ports are required ...
    if lnode.ports.len() <= 1 {
        return false;
    }

    // if ports may not be moved there's no use in enlarging the node
    let pc = lnode
        .properties
        .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS);
    if pc == PortConstraints::FIXED_POS {
        return false;
    }

    let nf = node_flexibility(graph, node);
    if nf == NodeFlexibility::NONE {
        return false;
    }

    // if we cannot resize the node, and the given height is not enough to properly accommodate
    // all ports, reuse the existing port positions
    if !is_flexible_size_where_space_permits(nf) {
        // we are not allowed to increase the node's size
        let port_spacing = graph.properties.double_with_default(SPACING_PORT_PORT);
        let additional_port_spacing = match lnode
            .properties
            .value_or_default(options::SPACING_PORTS_SURROUNDING)
        {
            Some(OptionValue::Margin(margin)) => *margin,
            _ => Spacing {
                top: port_spacing,
                left: port_spacing,
                bottom: port_spacing,
                right: port_spacing,
            },
        };

        // check west side
        let west_ports = port_side_view(graph, node, PortSide::WEST).len() as i32;
        let required_west_height = additional_port_spacing.top
            + additional_port_spacing.bottom
            + f64::from(west_ports - 1) * port_spacing;
        if required_west_height > lnode.size.y {
            return false;
        }

        // check east side
        let east_ports = port_side_view(graph, node, PortSide::EAST).len() as i32;
        let required_east_height = additional_port_spacing.top
            + additional_port_spacing.bottom
            + f64::from(east_ports - 1) * port_spacing;
        if required_east_height > lnode.size.y {
            return false;
        }
    }
    true
}

/// `NodeFlexibility.getNodeFlexibility`: the node's own setting, else the graph's default.
fn node_flexibility(graph: &LGraph, node: usize) -> NodeFlexibility {
    let lnode = &graph.nodes[node];
    if lnode
        .properties
        .has(options::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY)
    {
        lnode
            .properties
            .enum_with_default(options::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY)
    } else {
        graph
            .properties
            .enum_with_default(options::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT)
    }
}

/// The condition under which `ElkGraphLayoutTransferrer.applyNodeLayout` takes the node's size from
/// the layout: the graph places its nodes with network simplex, and the node may resize where space
/// permits.
pub fn takes_size_from_layout(graph: &LGraph, node: usize) -> bool {
    graph
        .properties
        .enum_with_default::<NodePlacementStrategy>(options::NODE_PLACEMENT_STRATEGY)
        == NodePlacementStrategy::NETWORK_SIMPLEX
        && is_flexible_size_where_space_permits(node_flexibility(graph, node))
}

/// `NodeFlexibility.isFlexibleSize`.
fn is_flexible_size(nf: NodeFlexibility) -> bool {
    nf == NodeFlexibility::NODE_SIZE
}

/// `NodeFlexibility.isFlexibleSizeWhereSpacePermits`.
fn is_flexible_size_where_space_permits(nf: NodeFlexibility) -> bool {
    nf == NodeFlexibility::NODE_SIZE_WHERE_SPACE_PERMITS || nf == NodeFlexibility::NODE_SIZE
}

/// `NodeFlexibility.isFlexiblePorts`.
fn is_flexible_ports(nf: NodeFlexibility) -> bool {
    nf == NodeFlexibility::PORT_POSITION
        || nf == NodeFlexibility::NODE_SIZE_WHERE_SPACE_PERMITS
        || nf == NodeFlexibility::NODE_SIZE
}

/// `PortSide.SIDES_EAST_WEST.contains`.
fn is_east_west(side: PortSide) -> bool {
    side == PortSide::EAST || side == PortSide::WEST
}

/// `LNode.getPortSideView(side)`, with the ports as `LNode.findPortIndices` finds them: the side's
/// run of ports in the node's list, where a later run of the same side replaces an earlier one.
fn port_side_view(graph: &LGraph, node: usize, side: PortSide) -> Vec<usize> {
    let ports = &graph.nodes[node].ports;
    let mut range: Option<(usize, usize)> = None;
    let mut current_side = PortSide::NORTH;
    let mut first_index_for_current_side = 0;
    for (index, &port) in ports.iter().enumerate() {
        let port_side = graph.ports[port].side;
        if port_side != current_side {
            if first_index_for_current_side != index && current_side == side {
                range = Some((first_index_for_current_side, index));
            }
            current_side = port_side;
            first_index_for_current_side = index;
        }
    }
    if current_side == side {
        range = Some((first_index_for_current_side, ports.len()));
    }
    match range {
        Some((start, end)) => ports[start..end].to_vec(),
        None => Vec::new(),
    }
}

// ------------------------------------------------------------------------------------------------
//                                      Edge Straightening
// ------------------------------------------------------------------------------------------------

/// `NetworkSimplexPlacer.orderTwoPath`: the two edges ordered so that the shared node is the target
/// of the first and the source of the second.
fn order_two_path(graph: &LGraph, path: Path) -> Path {
    let first = path[0];
    let second = path[1];
    if target_node(graph, first) != source_node(graph, second) {
        vec![second, first]
    } else {
        path
    }
}

/// `NetworkSimplexPlacer.isTwoPathCenterNodeFlexible`.
fn is_two_path_center_node_flexible(graph: &LGraph, path: &Path) -> bool {
    is_flexible_node(graph, target_node(graph, path[0]))
}

/// `Path.containsLongEdgeDummy`.
fn contains_long_edge_dummy(graph: &LGraph, path: &Path) -> bool {
    if path.is_empty() {
        return false;
    }
    if node_type(graph, source_node(graph, path[0])) == NodeType::LONG_EDGE {
        return true;
    }
    path.iter()
        .any(|&edge| node_type(graph, target_node(graph, edge)) == NodeType::LONG_EDGE)
}

/// `Path.containsFlexibleNode`: whether a node of the path satisfies the predicate.
fn contains_flexible_node(
    graph: &LGraph,
    path: &Path,
    predicate: fn(NodeFlexibility) -> bool,
) -> bool {
    if path.is_empty() {
        return false;
    }
    let nf = node_flexibility(graph, source_node(graph, path[0]));
    if predicate(nf) {
        return true;
    }
    path.iter()
        .any(|&edge| predicate(node_flexibility(graph, target_node(graph, edge))))
}

/// `NetworkSimplexPlacer.getEdgeWeight(LEdge)`: the weight of an edge, by the types of its nodes and
/// its straightness priority.
fn edge_weight(graph: &LGraph, edge: usize) -> f64 {
    let priority = i32::max(
        1,
        graph.edges[edge]
            .properties
            .int_with_default(options::PRIORITY_STRAIGHTNESS),
    );
    let edge_type_weight = edge_weight_of_types(
        node_type(graph, source_node(graph, edge)),
        node_type(graph, target_node(graph, edge)),
    );
    f64::from(priority) * edge_type_weight
}

/// `NetworkSimplexPlacer.getEdgeWeight(NodeType, NodeType)`.
fn edge_weight_of_types(node_type1: NodeType, node_type2: NodeType) -> f64 {
    if node_type1 == NodeType::NORMAL && node_type2 == NodeType::NORMAL {
        1.0 * EDGE_WEIGHT_BASE
    } else if node_type1 == NodeType::NORMAL || node_type2 == NodeType::NORMAL {
        2.0 * EDGE_WEIGHT_BASE
    } else {
        8.0 * EDGE_WEIGHT_BASE // SUPPRESS CHECKSTYLE MagicNumber
    }
}

/// `NetworkSimplexPlacer.isHandledEdge`: neither a self loop nor an in-layer edge.
fn is_handled_edge(graph: &LGraph, edge: usize) -> bool {
    !is_self_loop(graph, edge) && !is_in_layer_edge(graph, edge)
}

/// `NetworkSimplexPlacer.getNodeState`: whether the node is a junction.
fn get_node_state(graph: &LGraph, node: usize) -> i32 {
    let mut inco: i32 = 0;
    let mut ouco: i32 = 0;
    for &port in &graph.nodes[node].ports {
        inco += graph.ports[port]
            .incoming
            .iter()
            .filter(|&&edge| !is_self_loop(graph, edge))
            .count() as i32;
        ouco += graph.ports[port]
            .outgoing
            .iter()
            .filter(|&&edge| !is_self_loop(graph, edge))
            .count() as i32;
        if inco > 1 || ouco > 1 {
            return JUNCTION;
        }
    }
    if inco + ouco == 1 {
        return JUNCTION;
    }
    OTHER
}

/// `NetworkSimplexPlacer.EdgeRep.notStraightBy`: the y difference between the edge's ends, which is
/// zero for a straight edge.
fn not_straight_by(ng: &NGraph, rep: EdgeRep) -> i32 {
    let left = ng.edge(rep.left);
    let right = ng.edge(rep.right);
    ng.node(left.target())
        .layer
        .wrapping_sub(left.delta)
        .wrapping_sub(ng.node(right.target()).layer.wrapping_sub(right.delta))
}

/// `NetworkSimplexPlacer.length`: the length of a network simplex edge beyond its minimal length.
fn length(ng: &NGraph, edge: usize) -> i32 {
    let edge = ng.edge(edge);
    (ng.node(edge.source())
        .layer
        .wrapping_sub(ng.node(edge.target()).layer))
    .wrapping_abs()
    .wrapping_sub(edge.delta)
}

/// `DoubleMath.fuzzyEquals`.
fn fuzzy_equals(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance || a == b || (a.is_nan() && b.is_nan())
}

/// The layered node that is the source of an edge.
fn source_node(graph: &LGraph, edge: usize) -> usize {
    graph.ports[graph.edges[edge].source].node
}

/// The layered node that is the target of an edge.
fn target_node(graph: &LGraph, edge: usize) -> usize {
    graph.ports[graph.edges[edge].target].node
}

/// `LEdge.getOther(LNode)`: the node at the other end of the edge.
fn other_node(graph: &LGraph, edge: usize, node: usize) -> usize {
    if node == source_node(graph, edge) {
        target_node(graph, edge)
    } else {
        source_node(graph, edge)
    }
}

/// The type of a layered node.
fn node_type(graph: &LGraph, node: usize) -> NodeType {
    graph.nodes[node].node_type
}
