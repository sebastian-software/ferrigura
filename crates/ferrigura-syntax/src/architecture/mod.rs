//! The architecture parser (P2): `packages/parser/src/language/architecture/` and the architecture
//! DB of `packages/mermaid/src/diagrams/architecture/architectureDb.ts`, ported to Rust. The plan,
//! the lexer's order and the quirks kept are in `docs-port/architecture/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The parse then fills the
//! DB in the order `architectureParser.ts` gives it: the header texts, every group, every service,
//! every junction, every edge, every alignment. The first failing call is the error.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::architecture::ArchitectureDiagram;

use self::db::Db;
use self::grammar::Ast;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// Parses an architecture diagram, with its front matter, and returns its model.
///
/// # Errors
///
/// A lexer or parser error at the first token that the grammar does not take, or the DB's error at
/// the statement that caused it. Both are rejections, and the message and the position say why.
pub fn parse(raw: &str) -> Result<ArchitectureDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let ast = grammar::parse_text(source)?;

    let mut db = Db::new();
    // `Diagram.fromText` sets the front matter title before the parse; the grammar's title
    // replaces it when the grammar gives one (`populateCommonDb`).
    if let Some(title) = &prepared.title {
        db.set_title(title);
    }
    populate(&ast, &mut db, source)?;
    Ok(db.into_model())
}

/// The DB calls of `architectureParser.ts`'s `populateDb`, in its order.
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
    for group in &ast.groups {
        db.add_group(group)
            .map_err(|message| source.error(group.offset, message))?;
    }
    for service in &ast.services {
        db.add_service(service)
            .map_err(|message| source.error(service.offset, message))?;
    }
    for junction in &ast.junctions {
        db.add_junction(junction)
            .map_err(|message| source.error(junction.offset, message))?;
    }
    for edge in &ast.edges {
        db.add_edge(edge)
            .map_err(|message| source.error(edge.offset, message))?;
    }
    for alignment in &ast.alignments {
        db.add_layout_hint(alignment)
            .map_err(|message| source.error(alignment.offset, message))?;
    }
    Ok(())
}
