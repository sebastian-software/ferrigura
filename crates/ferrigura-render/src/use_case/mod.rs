//! The use case diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::config::UsecaseDiagramConfig;
use ferrigura_model::use_case::UseCaseDiagram;

use crate::RenderError;
use crate::context::Context;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Not rendered yet.
pub fn render(
    input: &Context<'_, UseCaseDiagram, UsecaseDiagramConfig>,
) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the use case diagram is not rendered yet".to_owned(),
    ))
}
