//! The gitGraph diagram: a port of `diagrams/git/gitGraphRenderer.ts`. The research is in
//! `docs-port/gitgraph/NOTES.md`, section 5.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::git_graph::GitGraphDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the gitGraph renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct GitGraphInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a GitGraphDiagram,
    /// The effective configuration, in the gitGraph diagram's scope.
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
pub fn render(input: &GitGraphInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the gitGraph diagram is not rendered yet".to_owned(),
    ))
}
