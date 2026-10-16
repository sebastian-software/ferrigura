//! The gantt diagram: a port of `diagrams/gantt/ganttRenderer.js`. The research is in
//! `docs-port/gantt/NOTES.md`, section 6.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::gantt::GanttDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the gantt renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct GanttInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a GanttDiagram,
    /// The effective configuration, in the gantt diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
    /// The current time in milliseconds since the epoch, for the today marker.
    pub now: i64,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Not rendered yet.
pub fn render(input: &GanttInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the gantt diagram is not rendered yet".to_owned(),
    ))
}
