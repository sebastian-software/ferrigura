//! The cynefin diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::cynefin::CynefinDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the cynefin diagram renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct CynefinInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a CynefinDiagram,
    /// The effective configuration, in the cynefin diagram's scope.
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
pub fn render(input: &CynefinInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the cynefin diagram is not rendered yet".to_owned(),
    ))
}
