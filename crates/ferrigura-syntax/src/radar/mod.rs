//! The radar parser (P3): `packages/parser/src/language/radar/` and the radar DB of
//! `packages/mermaid/src/diagrams/radar/db.ts`, ported to Rust. The plan, the lexer's order and the
//! quirks kept are in `docs-port/radar/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The parse then fills the
//! DB in the order `radar/parser.ts` gives it: the header texts (the title only when non-empty),
//! the axes, the curves, and the options. The first failing call is the error.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::radar::RadarDiagram;

use self::db::Db;
use self::grammar::Ast;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// Parses a radar diagram, with its front matter, and returns its model.
///
/// # Errors
///
/// A lexer or parser error at the first token that the grammar does not take, or the DB's error at
/// the curve that caused it (a curve that names its entries needs the axes first). Both are
/// rejections, and the message and the position say why.
pub fn parse(raw: &str) -> Result<RadarDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let ast = grammar::parse_text(source)?;

    let mut db = Db::new();
    // `Diagram.fromText` sets the front matter title before the parse; the grammar's title
    // replaces it when the grammar gives a non-empty one (`populateCommonDb`).
    if let Some(title) = &prepared.title {
        db.set_title(title);
    }
    populate(&ast, &mut db, source)?;
    Ok(db.into_model())
}

/// The DB calls of `radar/parser.ts`'s `populate`, in its order.
fn populate(ast: &Ast, db: &mut Db, source: Source<'_>) -> Result<(), ParseError> {
    if !ast.acc_descr.is_empty() {
        db.set_acc_descr(&ast.acc_descr);
    }
    if !ast.acc_title.is_empty() {
        db.set_acc_title(&ast.acc_title);
    }
    if !ast.title.is_empty() {
        db.set_title(&ast.title);
    }
    db.set_axes(&ast.axes);
    db.set_curves(&ast.curves)
        .map_err(|(offset, message)| source.error(offset, message))?;
    db.set_options(&ast.options);
    Ok(())
}
