//! The cynefin diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::config::CynefinDiagramConfig;
use ferrigura_model::cynefin::CynefinDiagram;

use crate::RenderError;
use crate::context::Context;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Not rendered yet.
pub fn render(
    input: &Context<'_, CynefinDiagram, CynefinDiagramConfig>,
) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the cynefin diagram is not rendered yet".to_owned(),
    ))
}
