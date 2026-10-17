//! The C4 diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P2-PLAN.md`, the registries).

use ferrigura_model::c4::C4Diagram;
use ferrigura_model::config::MermaidConfig;

use crate::RenderError;
use crate::theme::Themes;

/// What the C4 renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct C4Input<'a> {
    /// The parsed diagram.
    pub diagram: &'a C4Diagram,
    /// The effective configuration, in the C4 diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Not rendered yet.
pub fn render(input: &C4Input<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the C4 diagram is not rendered yet".to_owned(),
    ))
}
