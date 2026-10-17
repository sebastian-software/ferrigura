//! Parser of the event modeling diagram. Not ported yet: the parser task of the type replaces this file
//! (`docs-port/P2-PLAN.md`, the registries).

use ferrigura_model::event_modeling::EventModelingDiagram;

use crate::error::ParseError;

/// Parses a event modeling diagram and returns its model.
///
/// # Errors
///
/// Always, until the parser is ported: the error says that the type is not ported yet.
pub fn parse(text: &str) -> Result<EventModelingDiagram, ParseError> {
    let _ = text;
    Err(ParseError {
        line: 1,
        column: 1,
        message: "the eventmodeling parser is not ported yet".to_owned(),
    })
}
