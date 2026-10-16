//! Subgraph frames: a port of the `rect` cluster in
//! `packages/mermaid/src/rendering-util/rendering-elements/clusters.js`, for the `neo` and
//! `classic` looks. See `docs-port/flowchart/render-notes/labels.md` §13 and `styles.md`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;

use super::data::Node;
use super::geom::BBox;
use crate::RenderError;

/// A painted subgraph.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// The `<g class="cluster">` element.
    pub svg: String,
    /// Its box in absolute coordinates.
    pub bbox: BBox,
}

/// Paints a laid-out subgraph: its frame and its title, as `insertCluster` does.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn paint(node: &Node, config: &MermaidConfig, theme: &Value) -> Result<Cluster, RenderError> {
    let _ = (node, config, theme);
    todo!("port the rect cluster (task T7)")
}
