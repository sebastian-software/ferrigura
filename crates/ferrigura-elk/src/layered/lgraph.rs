//! Layered graph model, `LGraph` and friends in `org.eclipse.elk.alg.layered.graph`.
//!
//! Graphs are arenas of nodes; layers and lists hold indices into them, which
//! keeps the Java object graph's sharing explicit. The graphs of a compound
//! layout are a list, the top graph first; a node holds the index of its nested
//! graph, and a nested graph the indices of its node. Edges between graphs are
//! kept by the top graph (`CrossHierarchyEdge`).

use std::collections::{BTreeMap, BTreeSet};

use crate::core::data::JavaEnum;
use crate::core::data::enums::{GraphProperties, NodeType, PortConstraints, PortSide};
use crate::core::data::property::PropertyHolder;
use crate::core::math::{KVector, Spacing};
use crate::java::random::Random;
use crate::layered::labels::LabelSide;
use crate::layered::loops::SelfLoopHolder;

/// A node of the layered graph, `LNode`.
#[derive(Debug, Clone, Default)]
pub struct LNode {
    /// The id of the ELK node this came from.
    pub id: String,
    /// `LNode.getDesignation()` as the logs print it: the root id and the node id.
    pub designation: String,
    pub properties: PropertyHolder,
    /// `LNode.getSize()`.
    pub size: KVector,
    /// `LNode.getPosition()`, set by the placer.
    pub position: KVector,
    /// `LNode.getMargin()`: top, left, bottom, right.
    pub margin: Spacing,
    pub node_type: NodeType,
    /// The index the Brandes & Koepf placer gives the node (`LNode.id` in Java), set per placement.
    pub placement_id: usize,
    /// `InternalProperties.MODEL_ORDER`: the node's place in the input, when the model order is needed.
    pub model_order: Option<usize>,
    /// The layer it was assigned to, once layering has run.
    pub layer: Option<usize>,
    /// `LNode.getPorts()`: indices into `LGraph::ports`, in order.
    pub ports: Vec<usize>,
    /// `LNode.getLabels()`: the node's labels with text, as ids in `LGraph::labels`, in the order
    /// of the ELK node.
    pub labels: Vec<LabelId>,
    /// `InternalProperties.ORIGIN` of a dummy: the index of the edge it was made for.
    pub origin_edge: Option<usize>,
    /// `InternalProperties.REPRESENTED_LABELS` of a label dummy: the labels it holds, as ids in
    /// `LGraph::labels`.
    pub represented_labels: Vec<LabelId>,
    /// `InternalProperties.LONG_EDGE_SOURCE`: the port the long edge leaves at its first node.
    pub long_edge_source: Option<usize>,
    /// `InternalProperties.LONG_EDGE_TARGET`: the port the long edge enters at its last node.
    pub long_edge_target: Option<usize>,
    /// `InternalProperties.END_LABELS`: the label cells of the node's ports that have end labels, by port.
    pub end_labels: Option<Vec<(usize, crate::layered::labelcell::LabelCell)>>,
    /// `InternalProperties.LONG_EDGE_HAS_LABEL_DUMMIES`.
    pub long_edge_has_label_dummies: bool,
    /// `InternalProperties.LABEL_SIDE` of a label dummy: the side its labels go on.
    pub label_side: LabelSide,
    /// `InternalProperties.LONG_EDGE_BEFORE_LABEL_DUMMY`: a long edge dummy that precedes a label dummy.
    pub long_edge_before_label_dummy: bool,
    /// `InternalProperties.SELF_LOOP_HOLDER`: the self loops of the node, installed by the self loop
    /// preprocessor and taken out while the self loop processors work on the node.
    pub self_loops: Option<SelfLoopHolder>,
    /// `InternalProperties.ORIGINAL_PORT_CONSTRAINTS`: the port constraints the node had before the
    /// port side processor changed them. Set by the configurator.
    pub original_port_constraints: Option<PortConstraints>,
    /// `LNode.getNestedGraph()`: in a compound layout, the index of the graph inside this node in
    /// the layout's list of graphs (see `importer::import_compound`).
    pub nested_graph: Option<usize>,
    /// The parts of an external port dummy, which stands in a nested graph for a port of the node
    /// the graph is nested in.
    pub external_port: Option<ExternalPortDummy>,
    /// `InternalProperties.IN_LAYER_CONSTRAINT`.
    pub in_layer_constraint: InLayerConstraint,
    /// Whether the graph nested in this node has `GraphProperties.EXTERNAL_PORTS`, which the
    /// compound graph preprocessor records here for the processors of the node's own graph.
    pub nested_external_ports: bool,
    /// `InternalProperties.IN_LAYER_LAYOUT_UNIT`: the node whose layout unit this node belongs to.
    /// The north/south port preprocessor sets it on a side-fixed node (the node itself) and on the
    /// dummies it makes for the node's northern and southern ports.
    pub in_layer_layout_unit: Option<usize>,
    /// `InternalProperties.IN_LAYER_SUCCESSOR_CONSTRAINTS`: the nodes of the same layer this node
    /// must come before.
    pub in_layer_successor_constraints: Vec<usize>,
    /// `InternalProperties.BARYCENTER_ASSOCIATES`: the nodes whose barycenters count towards this
    /// node's, the dummies of its northern and southern ports.
    pub barycenter_associates: Option<Vec<usize>>,
}

/// The properties of an external port dummy (`NodeType.EXTERNAL_PORT`), which
/// `LGraphUtil.createExternalPortDummy` sets.
#[derive(Debug, Clone, Default)]
pub struct ExternalPortDummy {
    /// `InternalProperties.EXT_PORT_SIDE`: the side of the parent node the port is on, in the
    /// graph's current frame.
    pub side: Option<PortSide>,
    /// `InternalProperties.EXT_PORT_SIZE`: the size of the port.
    pub size: KVector,
    /// `LayeredOptions.PORT_ANCHOR` of the dummy: the anchor the dummy's port was placed at.
    pub anchor: KVector,
    /// `InternalProperties.ORIGIN`: the port the dummy stands for, an index into the ports of the
    /// graph that holds the parent node.
    pub origin: usize,
    /// `InternalProperties.EDGE_CONSTRAINT`.
    pub edge_constraint: EdgeConstraint,
    /// `InternalProperties.PORT_RATIO_OR_POSITION`, set for port constraints with a fixed order.
    pub port_ratio_or_position: Option<f64>,
}

/// `InLayerConstraint`: a node that must be at the top or the bottom of its layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InLayerConstraint {
    #[default]
    NONE,
    TOP,
    BOTTOM,
}

/// `EdgeConstraint`: a node that may only have incoming or only outgoing edges.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeConstraint {
    #[default]
    NONE,
    INCOMING_ONLY,
    OUTGOING_ONLY,
}

/// Whether a port is where an edge leaves or enters its node, `PortType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    INPUT,
    OUTPUT,
}

/// A port of a node, `LPort`: the place where edges meet the node.
#[derive(Debug, Clone)]
pub struct LPort {
    /// The index of the node in `LGraph::nodes`.
    pub node: usize,
    /// `LPort.getSide()`: the side of the node the port sits on.
    pub side: PortSide,
    /// `LPort.getPosition()`, relative to the node.
    pub position: KVector,
    /// `LPort.getSize()`.
    pub size: KVector,
    /// `LPort.getAnchor()`: the point the edges meet the port at, relative to the port.
    pub anchor: KVector,
    /// Indices into `LGraph::edges` of the edges that leave the port.
    pub outgoing: Vec<usize>,
    /// Indices into `LGraph::edges` of the edges that enter the port.
    pub incoming: Vec<usize>,
    /// `InternalProperties.MAX_EDGE_THICKNESS`: the largest thickness of the port's edges, set when
    /// the port has end labels.
    pub max_edge_thickness: Option<f64>,
    /// `LPort.id`: the index of the port in its node's list, as the self loop routing assigns it.
    pub id: usize,
    /// `InternalProperties.PORT_DUMMY`: the external port dummy that stands for this port in the
    /// graph nested in the port's node, an index into that graph's nodes.
    pub port_dummy: Option<usize>,
    /// `InternalProperties.INSIDE_CONNECTIONS`: the port has edges to the inside of its node.
    pub inside_connections: bool,
    /// `LayeredOptions.PORT_BORDER_OFFSET`: how far the port sits outside its node's border.
    pub border_offset: f64,
    /// `InternalProperties.PORT_DUMMY` as the north/south port preprocessor sets it: the dummy, in
    /// this graph, that holds the edges of this northern or southern port.
    pub north_south_dummy: Option<usize>,
    /// `InternalProperties.ORIGIN` of a north/south port dummy's port: the northern or southern
    /// port it holds the edges of.
    pub origin_port: Option<usize>,
    /// `LGraphElement.getDesignation()` of a port the importer read: the identifier of its ELK port
    /// (`ElkGraphTransformer.getOriginIdentifier`). Other ports have none and go by their index.
    pub designation: Option<String>,
}

impl LPort {
    /// A port on `side` of `node`, without edges and at the node's origin.
    pub fn new(node: usize, side: PortSide) -> Self {
        LPort {
            node,
            side,
            position: KVector::default(),
            size: KVector::default(),
            anchor: KVector::default(),
            id: 0,
            outgoing: Vec::new(),
            incoming: Vec::new(),
            max_edge_thickness: None,
            port_dummy: None,
            inside_connections: false,
            border_offset: 0.0,
            north_south_dummy: None,
            origin_port: None,
            designation: None,
        }
    }
}

/// An edge of the layered graph, `LEdge`.
#[derive(Debug, Clone, Default)]
pub struct LEdge {
    /// The id of the ELK edge this came from.
    pub id: String,
    /// The index in `LGraph::ports` of the port the edge leaves.
    pub source: usize,
    /// The index in `LGraph::ports` of the port the edge enters.
    pub target: usize,
    /// `LEdge.getLabels()`: the labels with text, in the order of the ELK edge, as ids in
    /// `LGraph::labels`. A label shared by two edges after a join is the same entry in the arena.
    pub labels: Vec<LabelId>,
    /// `InternalProperties.MODEL_ORDER`, when the input order matters.
    pub model_order: Option<usize>,
    /// The options of the ELK edge.
    pub properties: PropertyHolder,
    /// `InternalProperties.REVERSED`: the edge runs against its input direction after a reversal.
    pub reversed: bool,
    /// `LEdge.getBendPoints()`, in the internal left-to-right frame.
    pub bend_points: Vec<KVector>,
    /// `LEdge.THICKNESS`: the line thickness, 1 by default. A programmatic option, so the
    /// registry does not hold it and the importer sets this field directly.
    pub thickness: f64,
    /// `LGraphElement.getDesignation()` of an edge whose ELK edge is contained in another level
    /// than the edge's graph, a segment of a cross-hierarchy edge: the identifiers of the ELK
    /// edge's containers and its own (`ElkGraphTransformer.getOriginIdentifier`). Other edges go by
    /// their level's prefix and their id. Copies of the edge take it with their options.
    pub designation: Option<String>,
}

/// A label of an edge, `LLabel`.
#[derive(Debug, Clone, Default)]
pub struct LLabel {
    pub text: String,
    /// `LLabel.getSize()`.
    pub size: KVector,
    /// `LLabel.getPosition()`, relative to the edge.
    pub position: KVector,
    /// The options of the ELK label, such as its placement.
    pub properties: PropertyHolder,
    /// `InternalProperties.END_LABEL_EDGE`: the edge a head or tail label was moved from.
    pub end_label_edge: Option<usize>,
    /// `InternalProperties.ORIGIN` of a label: its index among the labels of its ELK edge, so
    /// that the position can be written back to the right label.
    pub origin: Option<usize>,
}

/// The root layered graph, `LGraph`.
#[derive(Debug, Clone, Default)]
pub struct LGraph {
    pub properties: PropertyHolder,
    /// `InternalProperties.GRAPH_PROPERTIES`: features the graph has.
    pub graph_properties: BTreeSet<GraphProperties>,
    pub nodes: Vec<LNode>,
    /// Indices into `nodes` of the nodes not yet in a layer, in model order.
    pub layerless_nodes: Vec<usize>,
    /// Layers, each a list of indices into `nodes`, left to right.
    pub layers: Vec<Vec<usize>>,
    /// The designation prefix of this level's root: the ids from the root of the graph down to it,
    /// as node and edge designations start with it (`root` for the top level).
    pub root_id: String,
    /// The processor steps `GraphConfigurator` assembled.
    pub processors: Vec<crate::layered::intermediate::Step>,
    /// `LGraph.getSize()`: the drawing without padding, in the internal left-to-right direction.
    pub size: KVector,
    /// `LGraph.getOffset()`: shifts the drawing, so the topmost node of the layers starts at zero.
    pub offset: KVector,
    /// `LGraph.getPadding()`: top, left, bottom, right.
    pub padding: Spacing,
    /// `Layer.getSize()` of each entry of `layers`, set by the layer size calculation.
    pub layer_sizes: Vec<KVector>,
    /// `InternalProperties.RANDOM`: the generator the crossing minimization draws from.
    pub random: Option<Random>,
    /// The ports of the nodes, indexed by `LNode::ports`.
    pub ports: Vec<LPort>,
    /// The edges, indexed by the ports' outgoing and incoming lists.
    pub edges: Vec<LEdge>,
    /// `InternalProperties.CYCLIC`: the cycle breaker reversed some edges.
    pub cyclic: bool,
    /// `LayeredOptions.JUNCTION_POINTS` of the edges, keyed by the edge's index in `edges`. An
    /// edge without an entry has none (Java's `null`). The orthogonal router sets these.
    /// They are kept here rather than on `LEdge`: the `LEdge` literals in `importer.rs` and
    /// `dummies.rs` do not use `..Default`, so a new `LEdge` field would not compile.
    pub junction_points: BTreeMap<usize, Vec<KVector>>,
    /// The labels of the graph, `LLabel` objects in an arena. The edges and label dummies hold their
    /// ids, so a label moved from one holder to another, or copied by a join where Java shares the
    /// object, stays one label: a write through one id is seen through all of them.
    pub labels: Vec<LLabel>,
    /// `LGraph.getParentNode()`: in a compound layout, the graph that holds the node this graph is
    /// nested in, and the node's index there. `None` for the top graph of a layout.
    pub parent_node: Option<(usize, usize)>,
    /// The edges of a compound layout whose ends are in different graphs. The top graph holds them,
    /// as it holds `InternalProperties.CROSS_HIERARCHY_MAP` in Java.
    pub cross_hierarchy_edges: Vec<CrossHierarchyEdge>,
    /// `InternalProperties.HIDDEN_NODES`: the `FIRST_SEPARATE` and `LAST_SEPARATE` nodes the layer
    /// constraint preprocessor took out of the graph during layering. Their edges stay in their own
    /// ports' lists and leave the lists of the ports at the other end.
    pub hidden_nodes: Vec<usize>,
}

/// An edge of a compound layout whose source and target are in different graphs, an `LEdge` that
/// `ElkGraphImporter.importHierarchicalGraph` attaches to ports of two graphs. The compound graph
/// preprocessor replaces it with dummy edges in the graphs it passes through (`segments`), and
/// the postprocessor joins their routes into it.
#[derive(Debug, Clone, Default)]
pub struct CrossHierarchyEdge {
    /// The id of the ELK edge.
    pub id: String,
    /// The options of the ELK edge.
    pub properties: PropertyHolder,
    /// `LGraphElement.getDesignation()`: the identifiers of the ELK edge's containers and its own.
    pub designation: String,
    /// `InternalProperties.MODEL_ORDER`, when the input order matters.
    pub model_order: Option<usize>,
    /// `LEdge.THICKNESS`.
    pub thickness: f64,
    /// The graph and the port index of the edge's source.
    pub source: (usize, usize),
    /// The graph and the port index of the edge's target.
    pub target: (usize, usize),
    /// `LEdge.getLabels()` of the original edge, while the edge holds them: before the preprocessor
    /// moves them to a segment, and after the postprocessor brings them back (in the coordinates
    /// of the edge's reference graph).
    pub labels: Vec<LLabel>,
    /// The labels the preprocessor moved: the graph of the segment and the label's id there.
    pub moved_labels: Vec<(usize, LabelId)>,
    /// `InternalProperties.COORDINATE_SYSTEM_ORIGIN`: the graph the route's coordinates are
    /// relative to in the ELK graph, when that is not the graph of the source.
    pub coordinate_system_origin: Option<usize>,
    /// `CROSS_HIERARCHY_MAP.get(edge)`: the dummy edges that stand for this edge, each with its
    /// graph and the type of the external port it was made for.
    pub segments: Vec<CrossHierarchySegment>,
    /// `LEdge.getBendPoints()`, set by the postprocessor.
    pub bend_points: Vec<KVector>,
    /// `LayeredOptions.JUNCTION_POINTS`, set by the postprocessor.
    pub junction_points: Option<Vec<KVector>>,
    /// `InternalProperties.TARGET_OFFSET`: moves the target point into the reference graph.
    pub target_offset: Option<KVector>,
    /// Whether the edge is attached to its ports (`getSource() != null`): until the preprocessor
    /// removes it, and again after the postprocessor.
    pub attached: bool,
}

/// `CrossHierarchyEdge` in Java: one dummy edge of a cross-hierarchy edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossHierarchySegment {
    /// The graph of the dummy edge.
    pub graph: usize,
    /// The dummy edge, an index into the graph's edges.
    pub edge: usize,
    /// The type of the external port the segment was made for.
    pub port_type: PortType,
}

/// The id of an `LLabel` in `LGraph::labels`.
pub type LabelId = usize;

impl LGraph {
    /// Adds a label to the arena and returns its id.
    pub fn add_label(&mut self, label: LLabel) -> LabelId {
        self.labels.push(label);
        self.labels.len() - 1
    }

    /// `LNode.toString()` of the node at `node`: `n_<designation>`, with the type in brackets for
    /// non-normal nodes. A node without a designation of its own, such as a dummy, takes its
    /// index in its layer, as `LNode.getDesignation` does, or -1 when it is in no layer.
    pub fn node_name(&self, node: usize) -> String {
        let data = &self.nodes[node];
        let designation = if data.designation.is_empty() {
            data.layer
                .and_then(|layer| self.layers[layer].iter().position(|&member| member == node))
                .map_or(-1, |index| index as i64)
                .to_string()
        } else {
            data.designation.clone()
        };
        if data.node_type == NodeType::NORMAL {
            format!("n_{designation}")
        } else {
            format!(
                "n({})_{designation}",
                data.node_type.java_name().to_lowercase()
            )
        }
    }

    /// `LPort.getDesignation()`: the identifier of the port's ELK port, or else its index in its
    /// node's port list. Ports have no labels in this port.
    pub fn port_designation(&self, port: usize) -> String {
        if let Some(designation) = &self.ports[port].designation {
            return designation.clone();
        }
        let owner = self.ports[port].node;
        self.nodes[owner]
            .ports
            .iter()
            .position(|&found| found == port)
            .map_or(-1, |index| index as i64)
            .to_string()
    }

    /// `LEdge.setTarget(port)`: the edge leaves its old target's incoming list and joins the new port's.
    pub fn set_edge_target(&mut self, edge: usize, port: usize) {
        let old = self.edges[edge].target;
        if let Some(index) = self.ports[old]
            .incoming
            .iter()
            .position(|&found| found == edge)
        {
            self.ports[old].incoming.remove(index);
        }
        self.edges[edge].target = port;
        self.ports[port].incoming.push(edge);
    }

    /// `LEdge.setSource(port)`: the edge leaves its old source's outgoing list and joins the new port's.
    pub fn set_edge_source(&mut self, edge: usize, port: usize) {
        let old = self.edges[edge].source;
        if let Some(index) = self.ports[old]
            .outgoing
            .iter()
            .position(|&found| found == edge)
        {
            self.ports[old].outgoing.remove(index);
        }
        self.edges[edge].source = port;
        self.ports[port].outgoing.push(edge);
    }

    /// Whether the edge is in the outgoing list of its source port, which is Java's
    /// `LEdge.getSource() != null`. A self loop that the preprocessor hid is detached until it is
    /// restored; its `source` and `target` keep their indices meanwhile.
    pub fn is_attached(&self, edge: usize) -> bool {
        self.ports[self.edges[edge].source].outgoing.contains(&edge)
    }

    /// `LEdge.setSource(null)` and `setTarget(null)`: the edge leaves the outgoing list of its source
    /// port and the incoming list of its target port. Its ends are kept, so it can be attached again.
    pub fn detach_edge(&mut self, edge: usize) {
        let (source, target) = (self.edges[edge].source, self.edges[edge].target);
        if let Some(index) = self.ports[source]
            .outgoing
            .iter()
            .position(|&found| found == edge)
        {
            self.ports[source].outgoing.remove(index);
        }
        if let Some(index) = self.ports[target]
            .incoming
            .iter()
            .position(|&found| found == edge)
        {
            self.ports[target].incoming.remove(index);
        }
    }

    /// `LEdge.setSource(source)` and `setTarget(target)` for the ends a detached edge kept: the edge
    /// is appended to the outgoing list of its source port and the incoming list of its target port.
    pub fn attach_edge(&mut self, edge: usize) {
        let (source, target) = (self.edges[edge].source, self.edges[edge].target);
        self.ports[source].outgoing.push(edge);
        self.ports[target].incoming.push(edge);
    }

    /// `LGraph.getActualSize()`: the size with the padding around it.
    pub fn actual_size(&self) -> KVector {
        KVector {
            x: self.size.x + self.padding.left + self.padding.right,
            y: self.size.y + self.padding.top + self.padding.bottom,
        }
    }
}
