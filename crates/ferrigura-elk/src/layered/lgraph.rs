//! Layered graph model, `LGraph` and friends in `org.eclipse.elk.alg.layered.graph`.
//!
//! Graphs are arenas of nodes; layers and lists hold indices into them, which
//! keeps the Java object graph's sharing explicit. Edges, ports and nested graphs
//! come with the processors that use them; the importer rejects them for now.

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
    /// The id of the root graph, used in node designations.
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
