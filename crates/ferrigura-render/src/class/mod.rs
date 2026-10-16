//! The class diagram: a port of the unified renderer with the class `getData` (`diagrams/class/classRenderer-v3-unified.ts`, `classDb.ts`). The research is in
//! `docs-port/class/NOTES.md`, section 5.

use ferrigura_model::class::ClassDiagram;
use ferrigura_model::config::MermaidConfig;

use crate::RenderError;
use crate::theme::Themes;

/// What the class renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct ClassInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a ClassDiagram,
    /// The effective configuration, in the class diagram's scope.
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
pub fn render(input: &ClassInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the class diagram is not rendered yet".to_owned(),
    ))
}
