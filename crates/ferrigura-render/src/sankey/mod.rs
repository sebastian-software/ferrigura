//! The sankey diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P2-PLAN.md`, the registries).

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::sankey::SankeyDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the sankey renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct SankeyInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a SankeyDiagram,
    /// The effective configuration, in the sankey diagram's scope.
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
pub fn render(input: &SankeyInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the sankey diagram is not rendered yet".to_owned(),
    ))
}
