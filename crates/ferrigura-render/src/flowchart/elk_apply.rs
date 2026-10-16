//! What the reference does with the elkjs result: a port of `applyElkLayoutResult`,
//! `applyElkEdgeLayout` and their helpers in
//! `packages/mermaid/src/rendering-util/layout-algorithms/elk/render.ts`, with `elk/geometry.ts`
//! and `find-common-ancestor.ts`. See `docs-port/flowchart/render-notes/elk-apply.md`.
//!
//! Afterwards every node has its centre and size, every group its frame, and every edge its
//! points (clipped at the outlines, in absolute coordinates), its curve and its label centre. The
//! nodes are in the reference's paint order (`orderNodesForElkPaint`).

use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;
use ferrigura_model::config::MermaidConfig;

use super::data::LayoutData;
use super::elk_build::ElkState;
use super::shapes::Intersect;

/// Applies the elkjs result to the layout data.
///
/// `intersects` holds the outline of every leaf node by id. Groups have none during layout, as in
/// the reference, whose cluster intersect is set only when the cluster is painted.
pub fn apply(
    data: &mut LayoutData,
    output: &JsonNode,
    state: &ElkState,
    intersects: &HashMap<String, Intersect>,
    config: &MermaidConfig,
) {
    let _ = (data, output, state, intersects, config);
    todo!("port applyElkLayoutResult (task T5)")
}
