//! The agentflow diagram. Not rendered yet: the renderer task of the type fills this module
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::agentflow::AgentflowDiagram;
use ferrigura_model::config::MermaidConfig;

use crate::RenderError;
use crate::theme::Themes;

/// What the agentflow diagram renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct AgentflowInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a AgentflowDiagram,
    /// The effective configuration, in the agentflow diagram's scope.
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
pub fn render(input: &AgentflowInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the agentflow diagram is not rendered yet".to_owned(),
    ))
}
