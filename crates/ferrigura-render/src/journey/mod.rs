//! The journey diagram: a port of `diagrams/user-journey/journeyRenderer.ts` and `svgDraw.js`. The research is in
//! `docs-port/journey/NOTES.md`, section 7.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::journey::JourneyDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the journey renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct JourneyInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a JourneyDiagram,
    /// The effective configuration, in the journey diagram's scope.
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
pub fn render(input: &JourneyInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the journey diagram is not rendered yet".to_owned(),
    ))
}
