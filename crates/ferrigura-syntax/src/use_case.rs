//! The parser of the use case diagram (`usecase-beta`), ported from the Chevrotain parser of the
//! reference. The plan, the lexer's rules and the quirks kept are in `docs-port/usecase/PLAN.md`.
//!
//! The text goes through the reference's steps before the scanner sees it (`Diagram.fromText`):
//! front matter and directives, comment lines, entities, and a line break. The scanner
//! ([`scanner`]) reads the tokens, the grammar (`grammar`) the syntax tree, the visitor
//! (`visitor`) the drafts and the statements, and the builder (`builder`) checks the diagram
//! and makes the model, with its graph AST (`ast`).

pub mod json;

mod ast;
mod builder;
mod draft;
mod grammar;
#[doc(hidden)]
pub mod scanner;
mod visitor;

use ferrigura_model::use_case::UseCaseDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// Parses a use case diagram and returns its model.
///
/// # Errors
///
/// Returns the first lexer, parser or model error, at its position in the original input.
pub fn parse(text: &str) -> Result<UseCaseDiagram, ParseError> {
    let prepared = prepare(text)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let source = encode_entities(&cleaned) + "\n";
    let error = |offset: usize, message: &str| {
        Source::new(&source, prepared.line_offset).error(offset, message)
    };
    let tokens = scanner::scan(&source).map_err(|offset| error(offset, "unexpected character"))?;
    let syntax =
        grammar::parse(&source, &tokens).map_err(|fail| error(fail.offset, "unexpected token"))?;
    let diagram = visitor::build(&source, &tokens, &syntax)
        .map_err(|failure| error(failure.offset, &failure.message))?;
    // `Diagram.fromText` sets the front matter title, but the parser clears the DB (`clear` resets the
    // title, `usecase.chevrotain.ts` 13), so the title never reaches the model: it stays empty.
    Ok(diagram)
}
