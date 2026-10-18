//! The swimlane diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::swimlanes::SwimlanesDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the swimlane diagram renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct SwimlanesInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a SwimlanesDiagram,
    /// The effective configuration, in the swimlane diagram's scope.
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
pub fn render(input: &SwimlanesInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the swimlane diagram is not rendered yet".to_owned(),
    ))
}
