//! Parser of the quadrant chart diagram. Not ported yet: the parser task of the type replaces this file
//! (`docs-port/P2-PLAN.md`, the registries).

use ferrigura_model::quadrant_chart::QuadrantChartDiagram;

use crate::error::ParseError;

/// Parses a quadrant chart diagram and returns its model.
///
/// # Errors
///
/// Always, until the parser is ported: the error says that the type is not ported yet.
pub fn parse(text: &str) -> Result<QuadrantChartDiagram, ParseError> {
    let _ = text;
    Err(ParseError {
        line: 1,
        column: 1,
        message: "the quadrantChart parser is not ported yet".to_owned(),
    })
}
