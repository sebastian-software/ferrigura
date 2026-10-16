//! The elkjs graph of a flowchart: a port of `buildElkGraphFromLayoutData` and the functions it
//! calls in `packages/mermaid/src/rendering-util/layout-algorithms/elk/render.ts`, with
//! `subgraphFeedbackEdges.ts`. See `docs-port/flowchart/render-notes/elk-build.md`.
//!
//! The reference keeps its working data on the elkjs objects (`isGroup`, `labelData`,
//! `layoutReversed`); here the graph holds only what elkjs reads, and [`ElkState`] holds the rest.

use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::JsonNode;
use ferrigura_model::config::MermaidConfig;

use super::data::LayoutData;

/// The measured title of a subgraph, as the reference keeps it in `labelData`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LabelData {
    /// The title's width.
    pub width: f64,
    /// The title's height less 2, as `getMeasuredLabelData` keeps it.
    pub height: f64,
    /// The width the title was wrapped at.
    pub wrapping_width: f64,
}

/// What the post-processing needs besides the graph.
#[derive(Debug, Clone, Default)]
pub struct ElkState {
    /// The subgraph of each node (`parentLookupDb.parentById`).
    pub parent_by_id: HashMap<String, String>,
    /// The members of each subgraph in order (`parentLookupDb.childrenById`).
    pub children_by_id: HashMap<String, Vec<String>>,
    /// The title of each subgraph.
    pub label_data: HashMap<String, LabelData>,
    /// The edges handed to elkjs with source and target swapped (`layoutReversed`).
    pub reversed_edges: HashSet<String>,
}

/// The graph for elkjs and the state for the post-processing.
#[derive(Debug, Clone, Default)]
pub struct ElkInput {
    /// The graph, as the reference passes it to `elk.layout`.
    pub graph: JsonNode,
    /// The side tables.
    pub state: ElkState,
}

/// Builds the elkjs graph from the measured layout data.
#[must_use]
pub fn build(data: &LayoutData, config: &MermaidConfig) -> ElkInput {
    let _ = (data, config);
    todo!("port buildElkGraphFromLayoutData (task T4)")
}
