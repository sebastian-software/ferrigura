//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/compound/CompoundGraphPreprocessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/compound/CompoundGraphPostprocessor.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/compound/CrossHierarchyEdgeComparator.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/LGraphUtil.java
//!   (`createExternalPortDummy`, `getExternalPortPosition`, `isDescendant`, `changeCoordSystem`)
//!
//! The edges of a compound layout whose ends are in different graphs. The preprocessor replaces
//! each with dummy edges, one in every graph the edge passes through, which end at external port
//! dummies: nodes on the border of a nested graph that stand for a port of the graph's node. The
//! postprocessor joins the routes of the dummy edges into the original edge.
//!
//! The graphs are those of `importer::import_compound`, the top graph first; the top graph holds
//! the cross-hierarchy edges. Inside self loops are refused by the importer. Port labels are not
//! ported: the ports of this model have none, so the external port dummies get none either.

use crate::core::data::OptionValue;
use crate::core::data::enums::{
    Direction, EdgeLabelPlacement, GraphProperties, LayerConstraint, NodeType, PortConstraints,
    PortSide,
};
use crate::core::math::KVector;
use crate::layered::LayoutError;
use crate::layered::graph_util::{opposed, side_from_direction};
use crate::layered::lgraph::{
    CrossHierarchyEdge, CrossHierarchySegment, EdgeConstraint, ExternalPortDummy,
    InLayerConstraint, LEdge, LGraph, LNode, LPort, PortType,
};
use crate::layered::options;
use crate::layered::p5edges::orthogonal::TOLERANCE;
use crate::layered::ports::{is_order_fixed, is_side_fixed};
use crate::layered::spacings::SPACING_EDGE_EDGE;

/// A node of one of the graphs: the graph's index and the node's index there.
type NodeRef = (usize, usize);
/// A port of one of the graphs: the graph's index and the port's index there.
type PortRef = (usize, usize);

/// `CompoundGraphPreprocessor.ExternalPort`: an external port made for one or more original edges.
#[derive(Debug, Clone)]
struct ExternalPort {
    /// The cross-hierarchy edges, as indices into the top graph's list.
    orig_edges: Vec<usize>,
    /// The dummy edge that connects the external port dummy.
    new_edge: (usize, usize),
    /// The external port dummy.
    dummy_node: NodeRef,
    /// The port on the parent node that the dummy stands for.
    dummy_port: PortRef,
    port_type: PortType,
    /// Whether the external port is handed to the graph above.
    exported: bool,
}

/// The state of `CompoundGraphPreprocessor.process`.
struct Preprocessor<'a> {
    graphs: &'a mut [LGraph],
    /// The cross-hierarchy edges, taken from the top graph while the preprocessor works.
    edges: Vec<CrossHierarchyEdge>,
    /// The external ports made so far.
    external_ports: Vec<ExternalPort>,
    /// `dummyNodeMap`: the external port dummy made for a port of a parent node.
    dummy_node_map: Vec<(PortRef, NodeRef)>,
}

/// `CompoundGraphPreprocessor.process` on the graphs of a compound layout.
pub fn preprocess(graphs: &mut [LGraph]) -> Result<(), LayoutError> {
    let edges = std::mem::take(&mut graphs[0].cross_hierarchy_edges);
    let mut pre = Preprocessor {
        graphs,
        edges,
        external_ports: Vec::new(),
        dummy_node_map: Vec::new(),
    };
    let result = pre.run();
    let Preprocessor { graphs, edges, .. } = pre;
    graphs[0].cross_hierarchy_edges = edges;
    result
}

impl Preprocessor<'_> {
    fn run(&mut self) -> Result<(), LayoutError> {
        self.transform_hierarchy_edges(0, None)?;
        self.move_labels_and_remove_original_edges();
        self.set_sides_of_ports_to_sides_of_dummy_nodes();
        for index in 0..self.graphs.len() {
            if let Some((parent, node)) = self.graphs[index].parent_node
                && self.graphs[index]
                    .graph_properties
                    .contains(&GraphProperties::EXTERNAL_PORTS)
            {
                self.graphs[parent].nodes[node].nested_external_ports = true;
            }
        }
        Ok(())
    }

    /// `transformHierarchyEdges`: the nested graphs first, then the edges of this graph's children
    /// that leave a nested graph, then those that leave this graph. Returns the external ports this
    /// graph exports to the graph above.
    fn transform_hierarchy_edges(
        &mut self,
        graph: usize,
        parent_node: Option<NodeRef>,
    ) -> Result<Vec<usize>, LayoutError> {
        let mut contained = Vec::new();
        for node in self.graphs[graph].layerless_nodes.clone() {
            let Some(nested) = self.graphs[graph].nodes[node].nested_graph else {
                continue;
            };
            contained.extend(self.transform_hierarchy_edges(nested, Some((graph, node)))?);
            // Every port of the node gets an external port dummy inside, when the nested graph has
            // external ports at all.
            if self.graphs[nested]
                .graph_properties
                .contains(&GraphProperties::EXTERNAL_PORTS)
            {
                let constraints = port_constraints(&self.graphs[graph].nodes[node]);
                let direction = direction_of(&self.graphs[nested])?;
                for port in self.graphs[graph].nodes[node].ports.clone() {
                    if self.dummy_for((graph, port)).is_some() {
                        continue;
                    }
                    let port_data = &self.graphs[graph].ports[port];
                    let holder = PortHolder {
                        border_offset: port_data.border_offset,
                    };
                    let side = port_data.side;
                    let size = port_data.size;
                    let net_flow = self.port_net_flow((graph, port));
                    let dummy = create_external_port_dummy(
                        &holder,
                        constraints,
                        side,
                        -net_flow,
                        size,
                        direction,
                        &mut self.graphs[nested],
                        port,
                    )?;
                    self.dummy_node_map.push(((graph, port), (nested, dummy)));
                    self.graphs[nested].layerless_nodes.push(dummy);
                }
            }
        }
        let mut exported = Vec::new();
        self.process_inner_hierarchical_edge_segments(
            graph,
            parent_node,
            &contained,
            &mut exported,
        )?;
        if let Some(parent_node) = parent_node {
            self.process_outer_hierarchical_edge_segments(graph, parent_node, &mut exported)?;
        }
        Ok(exported)
    }

    /// `processInnerHierarchicalEdgeSegments`: the original edges of the external ports of this
    /// graph's children connect to a child of this graph, to another child's external port, or
    /// leave this graph through an external port of its own.
    fn process_inner_hierarchical_edge_segments(
        &mut self,
        graph: usize,
        parent_node: Option<NodeRef>,
        contained: &[usize],
        exported: &mut Vec<usize>,
    ) -> Result<(), LayoutError> {
        let mut created = Vec::new();
        for &external in contained {
            let mut current: Option<usize> = None;
            let port_type = self.external_ports[external].port_type;
            for orig in self.external_ports[external].orig_edges.clone() {
                // The node at the other end of the original edge.
                let other = if port_type == PortType::OUTPUT {
                    self.edges[orig].target
                } else {
                    self.edges[orig].source
                };
                let other_node = (other.0, self.graphs[other.0].ports[other.1].node);
                if other_node.0 == graph {
                    // The edge connects to a direct child.
                    let dummy_port = self.external_ports[external].dummy_port;
                    if port_type == PortType::OUTPUT {
                        self.connect_child(graph, external, orig, dummy_port.1, other.1);
                    } else {
                        self.connect_child(graph, external, orig, other.1, dummy_port.1);
                    }
                } else if parent_node
                    .is_none_or(|parent| is_descendant(self.graphs, other_node, parent))
                {
                    // The edge connects two direct children; the output side connects them.
                    if port_type == PortType::OUTPUT {
                        self.connect_siblings(graph, external, contained, orig)?;
                    }
                } else if let Some(parent_node) = parent_node {
                    // The edge leaves this graph.
                    let opposite = self.external_ports[external].dummy_port.1;
                    let new_external = self.introduce_hierarchical_edge_segment(
                        graph,
                        parent_node,
                        orig,
                        opposite,
                        port_type,
                        current,
                    )?;
                    if Some(new_external) != current {
                        created.push(new_external);
                    }
                    if self.external_ports[new_external].exported {
                        current = Some(new_external);
                    }
                }
            }
        }
        self.add_dummies_and_export(graph, &created, exported);
        Ok(())
    }

    /// The end of `processInner...` and `processOuter...`: the dummies join the graph, and the
    /// exported external ports go to the graph above.
    fn add_dummies_and_export(
        &mut self,
        graph: usize,
        created: &[usize],
        exported: &mut Vec<usize>,
    ) {
        for &external in created {
            let (_, dummy) = self.external_ports[external].dummy_node;
            if !self.graphs[graph].layerless_nodes.contains(&dummy) {
                self.graphs[graph].layerless_nodes.push(dummy);
            }
            if self.external_ports[external].exported {
                exported.push(external);
            }
        }
    }

    /// `connectChild`: a dummy edge between two ports of this graph, for the original edge.
    fn connect_child(
        &mut self,
        graph: usize,
        external: usize,
        orig: usize,
        source_port: usize,
        target_port: usize,
    ) {
        let edge = create_dummy_edge(
            &mut self.graphs[graph],
            &self.edges[orig],
            source_port,
            target_port,
        );
        let port_type = self.external_ports[external].port_type;
        self.edges[orig].segments.push(CrossHierarchySegment {
            graph,
            edge,
            port_type,
        });
    }

    /// `connectSiblings`: a dummy edge from the output external port of one child to the input
    /// external port of another, for the original edge both carry.
    fn connect_siblings(
        &mut self,
        graph: usize,
        external_output: usize,
        contained: &[usize],
        orig: usize,
    ) -> Result<(), LayoutError> {
        let target_external = contained
            .iter()
            .copied()
            .find(|&other| {
                other != external_output && self.external_ports[other].orig_edges.contains(&orig)
            })
            .ok_or_else(|| {
                LayoutError::Unsupported(format!(
                    "the cross-hierarchy edge {} without its input external port",
                    self.edges[orig].id
                ))
            })?;
        let source_port = self.external_ports[external_output].dummy_port.1;
        let target_port = self.external_ports[target_external].dummy_port.1;
        let edge = create_dummy_edge(
            &mut self.graphs[graph],
            &self.edges[orig],
            source_port,
            target_port,
        );
        self.edges[orig].segments.push(CrossHierarchySegment {
            graph,
            edge,
            port_type: self.external_ports[external_output].port_type,
        });
        Ok(())
    }

    /// `processOuterHierarchicalEdgeSegments`: the original edges at the ports of this graph's
    /// children that leave the parent node get an external port each.
    fn process_outer_hierarchical_edge_segments(
        &mut self,
        graph: usize,
        parent_node: NodeRef,
        exported: &mut Vec<usize>,
    ) -> Result<(), LayoutError> {
        let mut created = Vec::new();
        for child in self.graphs[graph].layerless_nodes.clone() {
            for port in self.graphs[graph].nodes[child].ports.clone() {
                // The ports of this model hold one edge each, so `currentExternalOutputPort` and
                // `currentExternalInputPort` only matter for that one edge.
                for (orig, outgoing) in self.attached_edges_at((graph, port)) {
                    let other = if outgoing {
                        self.edges[orig].target
                    } else {
                        self.edges[orig].source
                    };
                    let other_node = (other.0, self.graphs[other.0].ports[other.1].node);
                    if is_descendant(self.graphs, other_node, parent_node) {
                        continue;
                    }
                    let port_type = if outgoing {
                        PortType::OUTPUT
                    } else {
                        PortType::INPUT
                    };
                    let new_external = self.introduce_hierarchical_edge_segment(
                        graph,
                        parent_node,
                        orig,
                        port,
                        port_type,
                        None,
                    )?;
                    created.push(new_external);
                }
            }
        }
        self.add_dummies_and_export(graph, &created, exported);
        Ok(())
    }

    /// The cross-hierarchy edges attached to a port: the outgoing ones, then the incoming ones,
    /// each with whether it leaves the port.
    fn attached_edges_at(&self, port: PortRef) -> Vec<(usize, bool)> {
        let attached = || {
            self.edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.attached)
        };
        let outgoing = attached()
            .filter(|(_, edge)| edge.source == port)
            .map(|(index, _)| (index, true));
        let incoming = attached()
            .filter(|(_, edge)| edge.target == port)
            .map(|(index, _)| (index, false));
        outgoing.chain(incoming).collect()
    }

    /// `introduceHierarchicalEdgeSegment`: the original edge leaves `graph` through an external port.
    /// A new external port is made unless edges are merged and `default` can take the edge.
    fn introduce_hierarchical_edge_segment(
        &mut self,
        graph: usize,
        parent_node: NodeRef,
        orig: usize,
        opposite_port: usize,
        port_type: PortType,
        default: Option<usize>,
    ) -> Result<usize, LayoutError> {
        let merge = matches!(
            self.graphs[graph]
                .properties
                .value_or_default(options::MERGE_HIERARCHY_EDGES),
            Some(OptionValue::Boolean(true))
        );
        // Whether the edge ends at the parent node itself.
        let parent_end_port = {
            let end = if port_type == PortType::INPUT {
                self.edges[orig].source
            } else {
                self.edges[orig].target
            };
            let end_node = (end.0, self.graphs[end.0].ports[end.1].node);
            (end_node == parent_node).then_some(end)
        };
        let external = match default {
            Some(default) if merge && parent_end_port.is_none() => {
                self.external_ports[default].orig_edges.push(orig);
                // `EDGE_THICKNESS` of the merged edge: the larger one.
                let (edge_graph, edge) = self.external_ports[default].new_edge;
                let thickness = self.graphs[edge_graph].edges[edge]
                    .thickness
                    .max(self.edges[orig].thickness);
                self.graphs[edge_graph].edges[edge].thickness = thickness;
                default
            }
            _ => {
                let parent_constraints =
                    port_constraints(&self.graphs[parent_node.0].nodes[parent_node.1]);
                let side = match parent_end_port {
                    Some(port) => self.graphs[port.0].ports[port.1].side,
                    None if is_side_fixed(parent_constraints) => {
                        if port_type == PortType::INPUT {
                            PortSide::WEST
                        } else {
                            PortSide::EAST
                        }
                    }
                    None => PortSide::UNDEFINED,
                };
                let dummy =
                    self.create_external_port_dummy(graph, parent_node, port_type, side, orig)?;
                let dummy_own_port = self.graphs[graph].nodes[dummy].ports[0];
                let (source, target) = if port_type == PortType::INPUT {
                    (dummy_own_port, opposite_port)
                } else {
                    (opposite_port, dummy_own_port)
                };
                let edge =
                    create_dummy_edge(&mut self.graphs[graph], &self.edges[orig], source, target);
                let origin = self.graphs[graph].nodes[dummy]
                    .external_port
                    .as_ref()
                    .map_or(0, |external| external.origin);
                self.external_ports.push(ExternalPort {
                    orig_edges: vec![orig],
                    new_edge: (graph, edge),
                    dummy_node: (graph, dummy),
                    dummy_port: (parent_node.0, origin),
                    port_type,
                    exported: parent_end_port.is_none(),
                });
                self.external_ports.len() - 1
            }
        };
        let (_, edge) = self.external_ports[external].new_edge;
        self.edges[orig].segments.push(CrossHierarchySegment {
            graph,
            edge,
            port_type,
        });
        Ok(external)
    }

    /// `CompoundGraphPreprocessor.createExternalPortDummy`: the dummy for the port the edge meets on
    /// the parent node, made once per port, or a new dummy with a new port on the parent node.
    fn create_external_port_dummy(
        &mut self,
        graph: usize,
        parent_node: NodeRef,
        port_type: PortType,
        side: PortSide,
        orig: usize,
    ) -> Result<usize, LayoutError> {
        let outside_port = if port_type == PortType::INPUT {
            self.edges[orig].source
        } else {
            self.edges[orig].target
        };
        let direction = direction_of(&self.graphs[graph])?;
        let parent_constraints = port_constraints(&self.graphs[parent_node.0].nodes[parent_node.1]);
        let outside_node = (
            outside_port.0,
            self.graphs[outside_port.0].ports[outside_port.1].node,
        );
        let dummy = if outside_node == parent_node {
            match self.dummy_for(outside_port) {
                Some((_, dummy)) => dummy,
                None => {
                    let port = &self.graphs[outside_port.0].ports[outside_port.1];
                    let holder = PortHolder {
                        border_offset: port.border_offset,
                    };
                    let size = port.size;
                    let net_flow = self.calculate_net_flow(outside_port);
                    let dummy = create_external_port_dummy(
                        &holder,
                        parent_constraints,
                        side,
                        net_flow,
                        size,
                        direction,
                        &mut self.graphs[graph],
                        outside_port.1,
                    )?;
                    self.dummy_node_map.push((outside_port, (graph, dummy)));
                    dummy
                }
            }
        } else {
            // `createExternalPortProperties`: half the edge spacing as the border offset.
            let holder = PortHolder {
                border_offset: self.graphs[graph]
                    .properties
                    .double_with_default(SPACING_EDGE_EDGE)
                    / 2.0,
            };
            let net_flow = if port_type == PortType::INPUT { -1 } else { 1 };
            // The port does not exist yet; it is made below, and the dummy's origin set then.
            let dummy = create_external_port_dummy(
                &holder,
                parent_constraints,
                side,
                net_flow,
                // A dummy port is not considered to have a size (#766).
                KVector::default(),
                direction,
                &mut self.graphs[graph],
                usize::MAX,
            )?;
            let port = self.create_port_for_dummy(graph, dummy, parent_node, port_type)?;
            if let Some(external) = self.graphs[graph].nodes[dummy].external_port.as_mut() {
                external.origin = port;
            }
            self.dummy_node_map
                .push(((parent_node.0, port), (graph, dummy)));
            dummy
        };
        let target = &mut self.graphs[graph];
        target
            .graph_properties
            .insert(GraphProperties::EXTERNAL_PORTS);
        let fixed = is_side_fixed(
            target
                .properties
                .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS),
        );
        target.properties.set(
            options::PORT_CONSTRAINTS,
            Some(OptionValue::Enum {
                class: "PortConstraints",
                name: if fixed { "FIXED_SIDE" } else { "FREE" }.to_string(),
            }),
        );
        Ok(dummy)
    }

    /// `createPortForDummy`: a port on the parent node, on the side the flow leaves or enters by,
    /// with the dummy's border offset.
    fn create_port_for_dummy(
        &mut self,
        graph: usize,
        dummy: usize,
        parent_node: NodeRef,
        port_type: PortType,
    ) -> Result<usize, LayoutError> {
        let border_offset = border_offset_of(&self.graphs[graph].nodes[dummy]);
        let parent_graph = &mut self.graphs[parent_node.0];
        let direction = direction_of(parent_graph)?;
        let side = match port_type {
            PortType::INPUT => opposed(side_from_direction(direction)),
            PortType::OUTPUT => side_from_direction(direction),
        };
        let mut port = LPort::new(parent_node.1, side);
        port.border_offset = border_offset;
        parent_graph.ports.push(port);
        let index = parent_graph.ports.len() - 1;
        parent_graph.nodes[parent_node.1].ports.push(index);
        Ok(index)
    }

    /// `dummyNodeMap.get(port)`.
    fn dummy_for(&self, port: PortRef) -> Option<NodeRef> {
        self.dummy_node_map
            .iter()
            .find(|(found, _)| *found == port)
            .map(|&(_, dummy)| dummy)
    }

    /// `LPort.getNetFlow()`: the incoming edges less the outgoing ones, the cross-hierarchy edges
    /// still attached to the port included.
    fn port_net_flow(&self, port: PortRef) -> i64 {
        let data = &self.graphs[port.0].ports[port.1];
        let mut flow = data.incoming.len() as i64 - data.outgoing.len() as i64;
        for (_, outgoing) in self.attached_edges_at(port) {
            flow += if outgoing { -1 } else { 1 };
        }
        flow
    }

    /// `CompoundGraphPreprocessor.calculateNetFlow`: the votes of a parent node's port for being an
    /// output port: an edge to the inside of the node votes for input, one to the outside for
    /// output, and the other way round for incoming edges. Self loops vote as outside edges.
    fn calculate_net_flow(&self, port: PortRef) -> i64 {
        let (graph, port_index) = port;
        let node = (graph, self.graphs[graph].ports[port_index].node);
        // The node at the other end of an edge, as its graph's parent node and the node itself.
        let inside = |other: NodeRef| self.graphs[other.0].parent_node == Some(node);
        let (mut output_vote, mut input_vote) = (0, 0);
        let data = &self.graphs[graph].ports[port_index];
        let mut outgoing: Vec<NodeRef> = data
            .outgoing
            .iter()
            .map(|&edge| {
                let target = self.graphs[graph].edges[edge].target;
                (graph, self.graphs[graph].ports[target].node)
            })
            .collect();
        let mut incoming: Vec<NodeRef> = data
            .incoming
            .iter()
            .map(|&edge| {
                let source = self.graphs[graph].edges[edge].source;
                (graph, self.graphs[graph].ports[source].node)
            })
            .collect();
        for (orig, out) in self.attached_edges_at(port) {
            let other = if out {
                self.edges[orig].target
            } else {
                self.edges[orig].source
            };
            let other_node = (other.0, self.graphs[other.0].ports[other.1].node);
            if out {
                outgoing.push(other_node);
            } else {
                incoming.push(other_node);
            }
        }
        for target in outgoing {
            if target == node {
                output_vote += 1;
            } else if inside(target) {
                input_vote += 1;
            } else {
                output_vote += 1;
            }
        }
        for source in incoming {
            if source == node {
                input_vote += 1;
            } else if inside(source) {
                output_vote += 1;
            } else {
                input_vote += 1;
            }
        }
        output_vote - input_vote
    }

    /// `moveLabelsAndRemoveOriginalEdges`: the labels of each original edge go to one of its
    /// segments, by their placement, and the original edges leave their ports.
    fn move_labels_and_remove_original_edges(&mut self) {
        for orig in 0..self.edges.len() {
            if !self.edges[orig].labels.is_empty() {
                let segments = sorted_segments(self.graphs, &self.edges[orig].segments);
                for label in std::mem::take(&mut self.edges[orig].labels) {
                    let placement = label
                        .properties
                        .enum_with_default::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT);
                    let target = match placement {
                        EdgeLabelPlacement::HEAD => segments.len().checked_sub(1),
                        EdgeLabelPlacement::CENTER => shallowest_edge_segment(&segments),
                        EdgeLabelPlacement::TAIL => Some(0),
                    };
                    let Some(segment) = target.and_then(|index| segments.get(index)) else {
                        self.edges[orig].labels.push(label);
                        continue;
                    };
                    // The segment's graph, which holds its source, gets the properties for labels.
                    let graph = &mut self.graphs[segment.graph];
                    let id = graph.add_label(label);
                    graph.edges[segment.edge].labels.push(id);
                    graph.graph_properties.insert(GraphProperties::END_LABELS);
                    graph
                        .graph_properties
                        .insert(GraphProperties::CENTER_LABELS);
                    self.edges[orig].moved_labels.push((segment.graph, id));
                }
            }
            self.edges[orig].attached = false;
        }
    }

    /// `setSidesOfPortsToSidesOfDummyNodes`: each port with a dummy inside takes the dummy's side,
    /// and its node's port sides become fixed.
    fn set_sides_of_ports_to_sides_of_dummy_nodes(&mut self) {
        for &((port_graph, port), (dummy_graph, dummy)) in &self.dummy_node_map {
            let side = self.graphs[dummy_graph].nodes[dummy]
                .external_port
                .as_ref()
                .and_then(|external| external.side)
                .unwrap_or(PortSide::UNDEFINED);
            let graph = &mut self.graphs[port_graph];
            graph.ports[port].port_dummy = Some(dummy);
            graph.ports[port].inside_connections = true;
            graph.ports[port].side = side;
            let node = graph.ports[port].node;
            graph.nodes[node].properties.set(
                options::PORT_CONSTRAINTS,
                Some(OptionValue::Enum {
                    class: "PortConstraints",
                    name: "FIXED_SIDE".to_string(),
                }),
            );
            graph
                .graph_properties
                .insert(GraphProperties::NON_FREE_PORTS);
        }
    }
}

/// The options `createExternalPortDummy` reads from its property holder: a port, or the holder of
/// `createExternalPortProperties`.
struct PortHolder {
    /// `LayeredOptions.PORT_BORDER_OFFSET`.
    border_offset: f64,
}

/// `LGraphUtil.createExternalPortDummy` without explicit anchors: a dummy node in `graph` with one
/// port, on the side the external port is on (from the flow when the port constraints leave the
/// side free). `origin` is the port on the parent node the dummy stands for. Returns the dummy's
/// index; the caller puts it in the layerless list. A fixed port order would make the dummy
/// remember the port's index or position (`PORT_RATIO_OR_POSITION`), which is not ported, so the
/// port's position is not needed.
#[allow(clippy::too_many_arguments)]
fn create_external_port_dummy(
    holder: &PortHolder,
    constraints: PortConstraints,
    port_side: PortSide,
    net_flow: i64,
    port_size: KVector,
    direction: Direction,
    graph: &mut LGraph,
    origin: usize,
) -> Result<usize, LayoutError> {
    let side = if is_side_fixed(constraints) {
        port_side
    } else if net_flow >= 0 {
        side_from_direction(direction)
    } else {
        opposed(side_from_direction(direction))
    };
    if is_order_fixed(constraints) {
        return Err(LayoutError::Unsupported(
            "external port dummies with a fixed port order (PORT_RATIO_OR_POSITION)".to_string(),
        ));
    }
    let node_index = graph.nodes.len();
    let mut dummy = LNode {
        node_type: NodeType::EXTERNAL_PORT,
        ..LNode::default()
    };
    dummy.properties.set(
        options::PORT_CONSTRAINTS,
        Some(OptionValue::Enum {
            class: "PortConstraints",
            name: "FIXED_POS".to_string(),
        }),
    );
    dummy.properties.set(
        options::PORT_BORDER_OFFSET,
        Some(OptionValue::Double(holder.border_offset)),
    );
    let mut anchor = KVector {
        x: port_size.x / 2.0,
        y: port_size.y / 2.0,
    };
    let mut edge_constraint = EdgeConstraint::NONE;
    let dummy_port_side = match side {
        PortSide::WEST => {
            set_layer_constraint(&mut dummy, LayerConstraint::FIRST_SEPARATE);
            edge_constraint = EdgeConstraint::OUTGOING_ONLY;
            dummy.size.y = port_size.y;
            if holder.border_offset < 0.0 {
                dummy.size.x = -holder.border_offset;
            }
            anchor.x = port_size.x;
            // The dummy has no width, so the anchor is relative to minus the port's width (#546).
            anchor.x -= port_size.x;
            PortSide::EAST
        }
        PortSide::EAST => {
            set_layer_constraint(&mut dummy, LayerConstraint::LAST_SEPARATE);
            edge_constraint = EdgeConstraint::INCOMING_ONLY;
            dummy.size.y = port_size.y;
            if holder.border_offset < 0.0 {
                dummy.size.x = -holder.border_offset;
            }
            anchor.x = 0.0;
            PortSide::WEST
        }
        PortSide::NORTH => {
            dummy.in_layer_constraint = InLayerConstraint::TOP;
            dummy.size.x = port_size.x;
            if holder.border_offset < 0.0 {
                dummy.size.y = -holder.border_offset;
            }
            anchor.y = port_size.y;
            anchor.y -= port_size.y;
            PortSide::SOUTH
        }
        PortSide::SOUTH => {
            dummy.in_layer_constraint = InLayerConstraint::BOTTOM;
            dummy.size.x = port_size.x;
            if holder.border_offset < 0.0 {
                dummy.size.y = -holder.border_offset;
            }
            anchor.y = 0.0;
            PortSide::NORTH
        }
        PortSide::UNDEFINED => {
            return Err(LayoutError::Unsupported(
                "an external port without a side".to_string(),
            ));
        }
    };
    dummy.external_port = Some(ExternalPortDummy {
        side: Some(side),
        size: port_size,
        anchor,
        origin,
        edge_constraint,
        port_ratio_or_position: None,
    });
    let mut port = LPort::new(node_index, dummy_port_side);
    port.position = anchor;
    graph.ports.push(port);
    dummy.ports.push(graph.ports.len() - 1);
    graph.nodes.push(dummy);
    Ok(node_index)
}

fn set_layer_constraint(node: &mut LNode, constraint: LayerConstraint) {
    use crate::core::data::JavaEnum;
    node.properties.set(
        options::LAYER_CONSTRAINT,
        Some(OptionValue::Enum {
            class: "LayerConstraint",
            name: constraint.java_name().to_string(),
        }),
    );
}

/// `createDummyEdge`: a new edge with the options of the original edge (and its model order), from
/// `source` to `target`, both ports of `graph`.
fn create_dummy_edge(
    graph: &mut LGraph,
    orig: &CrossHierarchyEdge,
    source: usize,
    target: usize,
) -> usize {
    let index = graph.edges.len();
    graph.edges.push(LEdge {
        id: orig.id.clone(),
        source,
        target,
        labels: Vec::new(),
        model_order: orig.model_order,
        properties: orig.properties.clone(),
        reversed: false,
        bend_points: Vec::new(),
        thickness: orig.thickness,
    });
    graph.ports[source].outgoing.push(index);
    graph.ports[target].incoming.push(index);
    index
}

/// `LayeredOptions.PORT_CONSTRAINTS` of a node.
fn port_constraints(node: &LNode) -> PortConstraints {
    node.properties
        .enum_with_default::<PortConstraints>(options::PORT_CONSTRAINTS)
}

/// `LayeredOptions.PORT_BORDER_OFFSET` of a node, 0 when unset (Layered's default).
pub fn border_offset_of(node: &LNode) -> f64 {
    node.properties
        .double_of(options::PORT_BORDER_OFFSET)
        .unwrap_or(0.0)
}

/// `LGraphUtil.getDirection` for a graph whose direction is set. An undefined direction would be
/// taken from the aspect ratio, which is not ported.
fn direction_of(graph: &LGraph) -> Result<Direction, LayoutError> {
    match graph
        .properties
        .enum_with_default::<Direction>(options::DIRECTION)
    {
        Direction::UNDEFINED => Err(LayoutError::Unsupported(
            "a graph without a direction (LGraphUtil.getDirection is not ported)".to_string(),
        )),
        direction => Ok(direction),
    }
}

/// `LGraphUtil.isDescendant(child, parent)`: `parent` is the node of a graph that holds `child`,
/// at any depth.
pub fn is_descendant(graphs: &[LGraph], child: NodeRef, parent: NodeRef) -> bool {
    let mut next = graphs[child.0].parent_node;
    while let Some(current) = next {
        if current == parent {
            return true;
        }
        next = graphs[current.0].parent_node;
    }
    false
}

/// `CrossHierarchyEdgeComparator.hierarchyLevel` from the top graph: how deep the graph is nested.
fn hierarchy_level(graphs: &[LGraph], graph: usize) -> i64 {
    let mut level = 0;
    let mut current = graph;
    while let Some((parent, _)) = graphs[current].parent_node {
        current = parent;
        level += 1;
    }
    level
}

/// The segments of an original edge, from its source to its target (`CrossHierarchyEdgeComparator`
/// with the top graph): the output segments from the deepest graph up, then the input segments from
/// the highest graph down. The sort is stable, as `Collections.sort` is.
fn sorted_segments(
    graphs: &[LGraph],
    segments: &[CrossHierarchySegment],
) -> Vec<CrossHierarchySegment> {
    let mut sorted = segments.to_vec();
    crate::java::sort::sort_by(&mut sorted, |a, b| {
        use std::cmp::Ordering;
        match (a.port_type, b.port_type) {
            (PortType::OUTPUT, PortType::INPUT) => Ordering::Less,
            (PortType::INPUT, PortType::OUTPUT) => Ordering::Greater,
            (PortType::OUTPUT, PortType::OUTPUT) => {
                hierarchy_level(graphs, b.graph).cmp(&hierarchy_level(graphs, a.graph))
            }
            (PortType::INPUT, PortType::INPUT) => {
                hierarchy_level(graphs, a.graph).cmp(&hierarchy_level(graphs, b.graph))
            }
        }
    });
    sorted
}

/// `getShallowestEdgeSegment`: the segment before the first input segment, or the last segment
/// when all go up.
fn shallowest_edge_segment(segments: &[CrossHierarchySegment]) -> Option<usize> {
    let mut result = None;
    for (index, segment) in segments.iter().enumerate() {
        if segment.port_type == PortType::INPUT {
            return Some(index.saturating_sub(1));
        } else if index == segments.len() - 1 {
            result = Some(index);
        }
    }
    result
}

/// `LGraphUtil.changeCoordSystem(point, oldGraph, newGraph)`: the point, relative to `old`, made
/// relative to `new`.
pub fn change_coord_system(graphs: &[LGraph], point: &mut KVector, old: usize, new: usize) {
    if old == new {
        return;
    }
    let mut graph = old;
    loop {
        point.x += graphs[graph].offset.x;
        point.y += graphs[graph].offset.y;
        let Some((parent, node)) = graphs[graph].parent_node else {
            break;
        };
        let padding = graphs[graph].padding;
        point.x += padding.left;
        point.y += padding.top;
        let position = graphs[parent].nodes[node].position;
        point.x += position.x;
        point.y += position.y;
        graph = parent;
    }
    let mut graph = new;
    loop {
        point.x -= graphs[graph].offset.x;
        point.y -= graphs[graph].offset.y;
        let Some((parent, node)) = graphs[graph].parent_node else {
            break;
        };
        let padding = graphs[graph].padding;
        point.x -= padding.left;
        point.y -= padding.top;
        let position = graphs[parent].nodes[node].position;
        point.x -= position.x;
        point.y -= position.y;
        graph = parent;
    }
}

/// `LPort.getAbsoluteAnchor()`: the node's position, plus the port's position and anchor.
fn absolute_anchor(graph: &LGraph, port: usize) -> KVector {
    let data = &graph.ports[port];
    let node = graph.nodes[data.node].position;
    KVector {
        x: node.x + data.position.x + data.anchor.x,
        y: node.y + data.position.y + data.anchor.y,
    }
}

/// `CrossHierarchyEdge.getActualSource()`: the source of the segment's dummy edge, or the port an
/// external port dummy there stands for.
fn actual_source(graphs: &[LGraph], segment: &CrossHierarchySegment) -> PortRef {
    let graph = &graphs[segment.graph];
    let port = graph.edges[segment.edge].source;
    actual_port(graphs, segment.graph, port)
}

/// `CrossHierarchyEdge.getActualTarget()`.
fn actual_target(graphs: &[LGraph], segment: &CrossHierarchySegment) -> PortRef {
    let graph = &graphs[segment.graph];
    let port = graph.edges[segment.edge].target;
    actual_port(graphs, segment.graph, port)
}

fn actual_port(graphs: &[LGraph], graph: usize, port: usize) -> PortRef {
    let node = &graphs[graph].nodes[graphs[graph].ports[port].node];
    match (&node.external_port, graphs[graph].parent_node) {
        (Some(external), Some((parent, _))) if node.node_type == NodeType::EXTERNAL_PORT => {
            (parent, external.origin)
        }
        _ => (graph, port),
    }
}

/// `CompoundGraphPostprocessor.process`: each original edge takes the routes of its segments, in
/// the coordinates of its reference graph, and its ports back; the dummy edges leave their ports.
pub fn postprocess(graphs: &mut [LGraph]) -> Result<(), LayoutError> {
    let add_unnecessary_bendpoints = graphs[0]
        .properties
        .bool_of(options::UNNECESSARY_BENDPOINTS);
    let mut edges = std::mem::take(&mut graphs[0].cross_hierarchy_edges);
    let mut dummy_edges: Vec<(usize, usize)> = Vec::new();
    for orig in &mut edges {
        let segments = sorted_segments(graphs, &orig.segments);
        let (Some(first), Some(last)) = (segments.first(), segments.last()) else {
            continue;
        };
        let source_port = actual_source(graphs, first);
        let target_port = actual_target(graphs, last);
        let reference_node = (
            source_port.0,
            graphs[source_port.0].ports[source_port.1].node,
        );
        let target_node = (
            target_port.0,
            graphs[target_port.0].ports[target_port.1].node,
        );
        let reference_graph = if is_descendant(graphs, target_node, reference_node) {
            graphs[reference_node.0].nodes[reference_node.1]
                .nested_graph
                .unwrap_or(reference_node.0)
        } else {
            reference_node.0
        };
        // `clearJunctionPoints`.
        let any_junction_points = segments.iter().any(|segment| {
            graphs[segment.graph]
                .junction_points
                .get(&segment.edge)
                .is_some_and(|points| !points.is_empty())
        });
        orig.junction_points = any_junction_points.then(Vec::new);
        orig.bend_points.clear();
        let mut last_point: Option<KVector> = None;
        for segment in &segments {
            let mut offset = KVector::default();
            change_coord_system(graphs, &mut offset, segment.graph, reference_graph);
            let graph = &graphs[segment.graph];
            let edge = &graph.edges[segment.edge];
            let bend_points: Vec<KVector> = edge
                .bend_points
                .iter()
                .map(|point| KVector {
                    x: point.x + offset.x,
                    y: point.y + offset.y,
                })
                .collect();
            let mut source_point = absolute_anchor(graph, edge.source);
            source_point.x += offset.x;
            source_point.y += offset.y;
            let mut target_point = absolute_anchor(graph, edge.target);
            target_point.x += offset.x;
            target_point.y += offset.y;
            if let Some(last) = last_point {
                let next = bend_points.first().copied().unwrap_or(target_point);
                let x_diff_enough = (last.x - next.x).abs() > TOLERANCE;
                let y_diff_enough = (last.y - next.y).abs() > TOLERANCE;
                if (!add_unnecessary_bendpoints && x_diff_enough && y_diff_enough)
                    || (add_unnecessary_bendpoints && (x_diff_enough || y_diff_enough))
                {
                    orig.bend_points.push(source_point);
                }
            }
            orig.bend_points.extend(bend_points.iter().copied());
            last_point = Some(bend_points.last().copied().unwrap_or(source_point));
            // `copyJunctionPoints`.
            if let Some(points) = graph.junction_points.get(&segment.edge)
                && let Some(target) = orig.junction_points.as_mut()
            {
                target.extend(points.iter().map(|point| KVector {
                    x: point.x + offset.x,
                    y: point.y + offset.y,
                }));
            }
            if actual_target(graphs, segment) == target_port {
                if target_port.0 != segment.graph {
                    offset = KVector::default();
                    change_coord_system(graphs, &mut offset, target_port.0, reference_graph);
                }
                orig.target_offset = Some(offset);
            }
            // `copyLabelsBack`: the labels this edge lent the segment, in the reference graph.
            let label_graph = segment.graph;
            let mut kept = Vec::new();
            for &id in &graphs[segment.graph].edges[segment.edge].labels {
                if orig.moved_labels.contains(&(label_graph, id)) {
                    let mut label = graphs[label_graph].labels[id].clone();
                    change_coord_system(graphs, &mut label.position, label_graph, reference_graph);
                    orig.labels.push(label);
                } else {
                    kept.push(id);
                }
            }
            graphs[segment.graph].edges[segment.edge].labels = kept;
            if !dummy_edges.contains(&(segment.graph, segment.edge)) {
                dummy_edges.push((segment.graph, segment.edge));
            }
        }
        orig.attached = true;
    }
    for (graph, edge) in dummy_edges {
        graphs[graph].detach_edge(edge);
    }
    graphs[0].cross_hierarchy_edges = edges;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::math::Spacing;

    /// A top graph whose node 1 holds graph 1, whose node 0 holds graph 2.
    fn three_levels() -> Vec<LGraph> {
        let mut graphs = vec![LGraph::default(), LGraph::default(), LGraph::default()];
        graphs[0].nodes = vec![LNode::default(), LNode::default()];
        graphs[0].nodes[1].nested_graph = Some(1);
        graphs[0].nodes[1].position = KVector { x: 100.0, y: 50.0 };
        graphs[0].offset = KVector { x: 1.0, y: 2.0 };
        graphs[1].parent_node = Some((0, 1));
        graphs[1].nodes = vec![LNode::default()];
        graphs[1].nodes[0].nested_graph = Some(2);
        graphs[1].nodes[0].position = KVector { x: 10.0, y: 20.0 };
        graphs[1].offset = KVector { x: 3.0, y: 4.0 };
        graphs[1].padding = Spacing {
            top: 5.0,
            left: 6.0,
            bottom: 0.0,
            right: 0.0,
        };
        graphs[2].parent_node = Some((1, 0));
        graphs[2].offset = KVector { x: 0.5, y: 0.25 };
        graphs
    }

    #[test]
    fn a_point_moves_up_through_offsets_paddings_and_node_positions() {
        let graphs = three_levels();
        let mut point = KVector::default();
        change_coord_system(&graphs, &mut point, 2, 0);
        // Graph 2's offset (it has no padding) and its node's position in graph 1, then graph 1's
        // offset, padding and node position in the top graph. The top graph's offset is added on
        // the way up and taken off on the way down.
        assert_eq!(point, KVector { x: 119.5, y: 79.25 });
        let mut back = point;
        change_coord_system(&graphs, &mut back, 0, 2);
        assert_eq!(back, KVector::default());
    }

    #[test]
    fn descendants_are_found_through_the_parent_nodes() {
        let graphs = three_levels();
        assert!(is_descendant(&graphs, (2, 0), (0, 1)));
        assert!(is_descendant(&graphs, (2, 0), (1, 0)));
        assert!(!is_descendant(&graphs, (1, 0), (1, 0)));
        assert!(!is_descendant(&graphs, (0, 0), (0, 1)));
    }

    #[test]
    fn segments_run_up_through_the_outputs_and_down_through_the_inputs() {
        let graphs = three_levels();
        let segment = |graph, port_type| CrossHierarchySegment {
            graph,
            edge: graph,
            port_type,
        };
        let segments = vec![
            segment(2, PortType::INPUT),
            segment(0, PortType::OUTPUT),
            segment(1, PortType::INPUT),
            segment(2, PortType::OUTPUT),
        ];
        let sorted: Vec<(usize, PortType)> = sorted_segments(&graphs, &segments)
            .iter()
            .map(|segment| (segment.graph, segment.port_type))
            .collect();
        assert_eq!(
            sorted,
            vec![
                (2, PortType::OUTPUT),
                (0, PortType::OUTPUT),
                (1, PortType::INPUT),
                (2, PortType::INPUT),
            ]
        );
        let sorted = sorted_segments(&graphs, &segments);
        // A centre label goes to the segment before the first input segment.
        assert_eq!(shallowest_edge_segment(&sorted), Some(1));
        assert_eq!(shallowest_edge_segment(&sorted[..2]), Some(1));
        assert_eq!(shallowest_edge_segment(&sorted[2..]), Some(0));
    }
}
