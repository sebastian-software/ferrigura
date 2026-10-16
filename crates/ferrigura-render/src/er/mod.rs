//! The ER diagram: a port of the unified renderer with the ER `getData` (`diagrams/er/erRenderer-unified.ts`, `erDb.ts`). The research is in
//! `docs-port/er/NOTES.md`, section 5.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::er::ErDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the ER renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct ErInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a ErDiagram,
    /// The effective configuration, in the ER diagram's scope.
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
pub fn render(input: &ErInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the ER diagram is not rendered yet".to_owned(),
    ))
}
