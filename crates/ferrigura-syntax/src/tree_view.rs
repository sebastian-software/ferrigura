//! Parser of the tree view diagram. Not ported yet: the parser task of the type replaces this file
//! (`docs-port/P3-PLAN.md`, the registries).

use ferrigura_model::tree_view::TreeViewDiagram;

use crate::error::ParseError;

/// Parses a tree view diagram and returns its model.
///
/// # Errors
///
/// Always, until the parser is ported: the error says that the type is not ported yet.
pub fn parse(text: &str) -> Result<TreeViewDiagram, ParseError> {
    let _ = text;
    Err(ParseError {
        line: 1,
        column: 1,
        message: "the tree view diagram parser is not ported yet".to_owned(),
    })
}
