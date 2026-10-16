//! The info diagram: a port of `diagrams/info/infoRenderer.ts`. The research is in
//! `docs-port/timeline/NOTES.md`, section 8.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::info::InfoDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the info renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct InfoInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a InfoDiagram,
    /// The effective configuration, in the info diagram's scope.
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
pub fn render(input: &InfoInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the info diagram is not rendered yet".to_owned(),
    ))
}
