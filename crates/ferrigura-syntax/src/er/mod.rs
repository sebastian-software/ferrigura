//! The ER parser (M6): `erDiagram.jison` and the ER DB `erDb.ts`, ported to Rust. The plan, the
//! quirks that are reproduced on purpose, and the model are in `docs-port/er/PLAN.md`.
//!
//! [`parse`] takes the diagram text as `Diagram.fromText` gives it to the parser: front matter,
//! directives and comments are removed, the entities are encoded, and a line break is added.
//! [`parse_grammar`] takes the text as the parser sees it in the reference's specs, with nothing
//! removed.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::er::ErDiagram;

use crate::error::{ParseError, Source};
use crate::flowchart::{cleanup_comments, encode_entities, remove_directives};
use crate::preprocess::prepare;

pub use db::{ErDb, Member};

/// Parses an ER diagram with its front matter, as `Diagram.fromText` does.
pub fn parse(raw: &str) -> Result<ErDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let mut db = ErDb::new();
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    parse_text(&text, prepared.line_offset, db)
}

/// Parses ER text with the grammar and the DB alone: no front matter, no directives, no comments,
/// no entity encoding and no added line break. It is the entry point of the ported parser specs,
/// which call the reference's `parser.parse` directly.
pub fn parse_grammar(text: &str) -> Result<ErDiagram, ParseError> {
    parse_text(text, 0, ErDb::new())
}

/// Lexes and parses the text into the DB, and returns the DB's model.
fn parse_text(text: &str, line_offset: usize, db: ErDb) -> Result<ErDiagram, ParseError> {
    let source = Source::new(text, line_offset);
    let tokens = scanner::tokenize(text)
        .map_err(|error| source.error(error.offset, "unexpected character in the diagram"))?;
    let mut parser = grammar::Parser::new(source, tokens, db);
    parser.parse_document()?;
    Ok(parser.into_model())
}
