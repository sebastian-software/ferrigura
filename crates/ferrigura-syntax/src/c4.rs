//! Parser of the C4 diagram. Not ported yet: the parser task of the type replaces this file
//! (`docs-port/P2-PLAN.md`, the registries).

use ferrigura_model::c4::C4Diagram;

use crate::error::ParseError;

/// Parses a C4 diagram and returns its model.
///
/// # Errors
///
/// Always, until the parser is ported: the error says that the type is not ported yet.
pub fn parse(text: &str) -> Result<C4Diagram, ParseError> {
    let _ = text;
    Err(ParseError {
        line: 1,
        column: 1,
        message: "the C4 parser is not ported yet".to_owned(),
    })
}
