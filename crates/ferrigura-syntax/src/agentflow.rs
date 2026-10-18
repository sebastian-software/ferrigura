//! Parser of the agentflow diagram. Not ported yet: the parser task of the type replaces this file
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::agentflow::AgentflowDiagram;

use crate::error::ParseError;

/// Parses a agentflow diagram and returns its model.
///
/// # Errors
///
/// Always, until the parser is ported: the error says that the type is not ported yet.
pub fn parse(text: &str) -> Result<AgentflowDiagram, ParseError> {
    let _ = text;
    Err(ParseError {
        line: 1,
        column: 1,
        message: "the agentflow diagram parser is not ported yet".to_owned(),
    })
}
