//! Edges: a port of `insertEdge`, `insertEdgeLabel` and the label placement in
//! `packages/mermaid/src/rendering-util/rendering-elements/edges.js`, with `edgeMarker.ts`,
//! `utils/lineWithOffset.ts` and the line hops of `lineJump.ts` and `elk/lineHops.ts`. See
//! `docs-port/flowchart/render-notes/edges.md`.

use ferrigura_model::config::MermaidConfig;

use super::data::{Edge, LayoutData};
use super::geom::BBox;
use super::label::LabelLayout;
use crate::RenderError;

/// A painted edge path.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgePath {
    /// The edge id.
    pub id: String,
    /// The `<path>` element.
    pub svg: String,
    /// The box of the path's geometry, as `getBBox` gives it (no stroke, no markers).
    pub bbox: BBox,
}

/// A painted edge label.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLabel {
    /// The `<g class="edgeLabel">` element, positioned.
    pub svg: String,
    /// Its box in absolute coordinates.
    pub bbox: BBox,
}

/// Sets what the reference's `applyElkEdgeRenderData` sets on every edge before layout: curve,
/// arrow types, classes, styles, thickness, pattern, arrowhead and label.
pub fn prepare(data: &mut LayoutData, config: &MermaidConfig) {
    let _ = (data, config);
    todo!("port applyElkEdgeRenderData (task T6)")
}

/// Lays out an edge's label as `insertEdgeLabel` does when it measures it; the caller stores the
/// size on the edge before layout.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn label(edge: &Edge, config: &MermaidConfig) -> Result<Option<LabelLayout>, RenderError> {
    let _ = (edge, config);
    todo!("port insertEdgeLabel (task T6)")
}

/// The size `insertEdgeLabel` records for a label: the label group's box.
#[must_use]
pub fn label_size(label: &LabelLayout) -> (f64, f64) {
    let _ = label;
    todo!("port the edge label size (task T6)")
}

/// Paints the edges of a laid-out diagram: one path per edge, then the line hops over all of
/// them, as `paintLayoutData` and `applyElkLineJumps` do.
#[must_use]
pub fn paint_paths(edges: &[Edge], svg_id: &str, config: &MermaidConfig) -> Vec<EdgePath> {
    let _ = (edges, svg_id, config);
    todo!("port insertEdge and the line hops (task T6)")
}

/// Paints an edge's label at its laid-out position.
#[must_use]
pub fn paint_label(edge: &Edge, label: &LabelLayout, config: &MermaidConfig) -> EdgeLabel {
    let _ = (edge, label, config);
    todo!("port the edge label placement (task T6)")
}

/// The marker definitions the diagram uses, as `insertMarkers` writes them for `flowchart-v2`.
#[must_use]
pub fn markers(svg_id: &str, config: &MermaidConfig) -> String {
    let _ = (svg_id, config);
    todo!("port markers.js (task T6)")
}
