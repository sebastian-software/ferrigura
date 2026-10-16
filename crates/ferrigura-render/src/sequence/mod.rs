//! The sequence diagram: a port of `diagrams/sequence/sequenceRenderer.ts` and `svgDraw.js`. The research is in
//! `docs-port/sequence/NOTES.md`, section 5.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::sequence::SequenceDiagram;

use crate::RenderError;
use crate::theme::Themes;

/// What the sequence renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct SequenceInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a SequenceDiagram,
    /// The effective configuration, in the sequence diagram's scope.
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
pub fn render(input: &SequenceInput<'_>) -> Result<String, RenderError> {
    let _ = input;
    Err(RenderError::Unsupported(
        "the sequence diagram is not rendered yet".to_owned(),
    ))
}
