//! The parser of the cynefin diagram (P3): `packages/parser/src/language/cynefin/` and the cynefin
//! DB of `packages/mermaid/src/diagrams/cynefin/cynefinDb.ts`, ported to Rust. The plan, the lexer's
//! order and the quirks kept are in `docs-port/cynefin/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The front matter title is
//! set first, then the parse fills the DB as `cynefinParser.ts` does.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::cynefin::CynefinDiagram;

use self::db::Db;
use self::grammar::Ast;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// Parses a cynefin diagram, with its front matter, and returns its model.
///
/// # Errors
///
/// A lexical error or a parser error at the first token the grammar does not take, or the error of
/// the front matter. Both are rejections, and the message and the position say why.
pub fn parse(raw: &str) -> Result<CynefinDiagram, ParseError> {
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
    populate(&ast, &mut db);
    Ok(db.into_model())
}

/// The DB calls of `cynefinParser.ts`, in its order: the common header texts (each only when it is
/// not empty, as `populateCommonDb` tests), then the domains, then the transitions.
fn populate(ast: &Ast, db: &mut Db) {
    if !ast.acc_descr.is_empty() {
        db.set_acc_descr(&ast.acc_descr);
    }
    if !ast.acc_title.is_empty() {
        db.set_acc_title(&ast.acc_title);
    }
    if !ast.title.is_empty() {
        db.set_title(&ast.title);
    }
    db.set_domains(&ast.domains);
    db.set_transitions(&ast.transitions);
}
