//! The treemap parser (`packages/parser/src/language/treemap/treemap.langium`) and its DB
//! (`packages/mermaid/src/diagrams/treemap/`), ported by hand. The plan, the lexer's rules and the
//! quirks kept are in `docs-port/treemap/PLAN.md`; the research is in `docs-port/treemap/NOTES.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The DB then takes the
//! grammar's rows in the order of `populate`.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::treemap::TreemapDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// Parses a treemap diagram and returns its model.
pub fn parse(raw: &str) -> Result<TreemapDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let tokens = match scanner::tokenize(&text) {
        Ok(tokens) => tokens,
        Err(failure) => {
            return Err(source.error(failure.offset, format!("Lexer error: {}", failure.message)));
        }
    };
    let ast = grammar::parse(&tokens, source)?;
    Ok(db::build(&ast, prepared.title.as_deref()))
}
