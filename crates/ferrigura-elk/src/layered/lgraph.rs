//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/LGraph.java
//!
//! The layered graph, `LGraph` and friends in `org.eclipse.elk.alg.layered.graph`.
//!
//! Stage C step 2 covers the root graph and its layerless nodes. Layers, ports,
//! edges and nested graphs are added with the processors that use them.

use std::collections::BTreeSet;

use crate::core::data::enums::GraphProperties;
use crate::core::data::property::PropertyHolder;
use crate::core::math::KVector;
use crate::layered::intermediate::Step;

/// A node of the layered graph.
#[derive(Debug, Clone, Default)]
pub struct LNode {
    /// The id of the ELK node this came from.
    pub id: String,
    pub properties: PropertyHolder,
    /// The node's size, `LNode.getSize()`.
    pub size: KVector,
    /// The layer it was assigned to, once layering has run.
    pub layer: Option<usize>,
}

/// The root layered graph.
#[derive(Debug, Clone, Default)]
pub struct LGraph {
    pub properties: PropertyHolder,
    /// `InternalProperties.GRAPH_PROPERTIES`: features the graph has.
    pub graph_properties: BTreeSet<GraphProperties>,
    pub nodes: Vec<LNode>,
    /// Indices into `nodes` of the nodes not yet in a layer, in model order.
    pub layerless_nodes: Vec<usize>,
    /// Layers, each a list of indices into `nodes`.
    pub layers: Vec<Vec<usize>>,
    /// `InternalProperties.PROCESSORS`: the steps `GraphConfigurator` assembled.
    pub processors: Vec<Step>,
}
