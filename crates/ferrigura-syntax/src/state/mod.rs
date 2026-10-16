//! The state diagram parser (M6): `parser/stateDiagram.jison` and the state DB `stateDb.ts`, ported
//! to Rust. The plan, the scanner's rules, the grammar and the DB are in `docs-port/state/PLAN.md`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter
//! (the shared preparation), directives, comment lines, entity encoding and one line break at the
//! end (`Diagram.fromText`). [`parse_grammar`] runs the grammar and the DB alone, as the ported
//! specs call the reference's parser directly.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::state::{StateDiagram, Statement};

use crate::error::{ParseError, Source};
use crate::flowchart::{cleanup_comments, encode_entities, remove_directives};
use crate::preprocess::prepare;
use crate::sanitize::sanitize_text;

/// Parses a state diagram with its front matter, directives and comments, as the reference's
/// `Diagram.fromText` gives the text to the parser, and returns its model.
pub fn parse(raw: &str) -> Result<StateDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    // `setDiagramTitle` sanitises the front matter's title.
    let title = prepared
        .title
        .as_deref()
        .map(sanitize_text)
        .unwrap_or_default();
    build(&text, prepared.line_offset, title)
}

/// Parses text with the grammar and the DB alone: no front matter, no directive or comment
/// removal, no entity encoding, no line break. It is the entry point of the ported specs, which
/// call the reference's parser directly. [`parse`] is the entry point for diagram text.
pub fn parse_grammar(text: &str) -> Result<StateDiagram, ParseError> {
    build(text, 0, String::new())
}

/// The grammar and the DB over prepared text.
fn build(text: &str, line_offset: usize, title: String) -> Result<StateDiagram, ParseError> {
    let source = Source::new(text, line_offset);
    let tokens = scanner::tokenize(text).map_err(|error| grammar::scan_error(&source, error))?;
    let parsed = grammar::Parser::new(source, tokens).start()?;

    let mut document = parsed.document;
    db::translate_root(&mut document);
    let db = db::extract(&mut document);
    if let Some(message) = db::group_label_error(&document) {
        return Err(source.error(0, message));
    }

    // `getDirection` is the first root direction; the layout's `getDir` is the last one.
    let directions: Vec<&str> = document
        .iter()
        .filter_map(|statement| match statement {
            Statement::Dir(dir) => Some(dir.value.as_str()),
            _ => None,
        })
        .collect();
    let mut diagram = StateDiagram::new();
    diagram.title = title;
    diagram.acc_title = parsed.acc_title;
    diagram.acc_descr = parsed.acc_descr;
    if let Some(first) = directions.first() {
        diagram.direction = (*first).to_owned();
    }
    if let Some(last) = directions.last() {
        diagram.layout_direction = (*last).to_owned();
    }
    diagram.document = document;
    diagram.states = db.states;
    diagram.relations = db.relations;
    diagram.classes = db.classes;
    diagram.links = db.links;
    Ok(diagram)
}
