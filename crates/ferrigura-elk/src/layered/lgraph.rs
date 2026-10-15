//! Layered graph model, `LGraph` and friends in `org.eclipse.elk.alg.layered.graph`.
//!
//! Graphs are arenas of nodes; layers and lists hold indices into them, which
//! keeps the Java object graph's sharing explicit. Edges, ports and nested graphs
//! come with the processors that use them; the importer rejects them for now.

use std::collections::BTreeSet;

use crate::core::data::enums::{GraphProperties, NodeType};
use crate::core::data::property::PropertyHolder;
use crate::core::math::{KVector, Spacing};

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
    /// The layer it was assigned to, once layering has run.
    pub layer: Option<usize>,
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
}
