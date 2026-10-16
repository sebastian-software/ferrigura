//! The timeline diagram: a port of `diagrams/timeline/timelineRenderer.ts` and `svgDraw.js`. The research is in
//! `docs-port/timeline/NOTES.md`, section 5.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::timeline::TimelineDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the timeline renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct TimelineInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a TimelineDiagram,
    /// The effective configuration, in the timeline diagram's scope.
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
pub fn render(input: &TimelineInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the timeline diagram is not rendered yet".to_owned(),
    ))
}
