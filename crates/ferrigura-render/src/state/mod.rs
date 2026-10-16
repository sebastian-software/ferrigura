//! The state diagram: a port of the unified renderer with the state `getData` (`diagrams/state/stateRenderer-v3-unified.ts`, `dataFetcher.ts`). The research is in
//! `docs-port/state/NOTES.md`, section 5.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::state::StateDiagram;

pub mod data;

use crate::RenderError;
use crate::theme::Themes;

/// What the state renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct StateInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a StateDiagram,
    /// The effective configuration, in the state diagram's scope.
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
pub fn render(input: &StateInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the state diagram is not rendered yet".to_owned(),
    ))
}
