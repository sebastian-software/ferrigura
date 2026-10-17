//! The block diagram parser: `block.jison` and the block DB `blockDB.ts`, ported by hand. The plan, the rule table and the quirks are in `docs-port/block/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar then builds the
//! statements, and the DB populates the blocks, edges and classes from them in document order.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::block::BlockDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

pub use db::{
    edge_str_to_edge_data, edge_str_to_edge_start_data, edge_str_to_pattern, edge_str_to_thickness,
};

/// Parses a block diagram and returns its model.
///
/// # Errors
///
/// Returns the first token the grammar rejects, or the first text the scanner does not take, with
/// its position counted from the first line of the original input. A style of a block that is not
/// declared before it is also an error, as the reference's DB throws for it.
pub fn parse(raw: &str) -> Result<BlockDiagram, ParseError> {
    parse_with_warnings(raw).map(|(diagram, _)| diagram)
}

/// Parses a block diagram and returns its model with the warnings the reference logs while it
/// populates the DB (a block wider than the configured columns of its list).
///
/// # Errors
///
/// As [`parse`].
pub fn parse_with_warnings(raw: &str) -> Result<(BlockDiagram, Vec<String>), ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let items = grammar::parse(&text, source)?;
    let mut db = db::Db::new();
    db.populate_root(items, source)?;
    Ok(db.into_model())
}
