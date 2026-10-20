//! Parser of the swimlane diagram (`swimlane-beta`), ported from the reference's swimlane diagram
//! (`packages/mermaid/src/diagrams/swimlanes/`). The reference has no grammar of its own: the text
//! goes through the flowchart's parser (`flow.jison`) and fills the flowchart's DB, and the keyword
//! is one more graph word there (`flow.jison` 116, `"swimlane-beta"`, as `graph` and `flowchart`).
//! So this parser is the flowchart's, reached with that header. The quirks of the path and the
//! parser's decisions are in `docs-port/swimlanes/PLAN.md`.
//!
//! The text is a swimlane diagram when the detector takes it (`/^\s*swimlane-beta\b/`,
//! `diagrams/swimlanes/detector.ts:8`, see [`crate::detect`]). As with the other diagram types, the
//! caller detects the type first; this parser does not check the header.

use ferrigura_model::swimlanes::SwimlanesDiagram;

use crate::error::ParseError;
use crate::flowchart::{self, FlowchartError};

/// Parses a swimlane diagram and returns its model: the flowchart's, of which the header is
/// `swimlane-beta` (`flowchart::parse`, with the default options of the flowchart DB).
///
/// # Errors
///
/// A rejection of the flowchart parser, with the message and the position it gives: a syntax error,
/// an unknown shape, or more edges than `maxEdges` (500, `flowDb.ts:330`).
pub fn parse(text: &str) -> Result<SwimlanesDiagram, ParseError> {
    flowchart::parse(text).map_err(|FlowchartError::Syntax(error)| error)
}
