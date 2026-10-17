//! The parser of the packet diagram, `packet.langium` and the packet DB of
//! `packages/mermaid/src/diagrams/packet/`, parsed by hand as pie is. The decisions and the quirks
//! the port keeps are in `docs-port/packet/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities and a line break. The DB reads `bitsPerRow`
//! from the effective configuration during the parse (`parser.ts:16`), so the front matter and the
//! directives change the rows.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::config::effective;
use ferrigura_model::packet::PacketDiagram;
use ferrigura_model::value::Value;

use self::db::BitsPerRow;
use crate::config;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// `bitsPerRow` of the packet defaults (`defaultConfig.ts`, `schemas/config.schema.yaml`).
const DEFAULT_BITS_PER_ROW: f64 = 32.0;

/// Parses a packet diagram, with its front matter and directives, and returns its model.
pub fn parse(raw: &str) -> Result<PacketDiagram, ParseError> {
    let bits_per_row = bits_per_row_of(raw)?;
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let ast = grammar::parse_text(source)?;

    let mut diagram = PacketDiagram::new();
    // `populateCommonDb` sets the grammar's title only when it is non-empty; the front matter's
    // title was set before the parse, so it stays otherwise.
    diagram.title = if ast.title.is_empty() {
        prepared
            .title
            .as_deref()
            .map(sanitize_text)
            .unwrap_or_default()
    } else {
        sanitize_text(&ast.title)
    };
    // `setAccTitle` removes the leading blanks after sanitising; `setAccDescription` removes the
    // blanks after each line break.
    diagram.acc_title = strip_leading_space(&sanitize_text(&ast.acc_title)).to_owned();
    diagram.acc_descr = newline_then_space_to_newline(&sanitize_text(&ast.acc_descr));
    diagram.rows = db::populate(&ast.blocks, bits_per_row, source)?;
    Ok(diagram)
}

/// `bitsPerRow` of the effective configuration of the text, in the packet scope.
fn bits_per_row_of(raw: &str) -> Result<BitsPerRow, ParseError> {
    let settings = config::read(raw)?;
    let resolved = effective(None, Some("packet"), &settings);
    let value = resolved
        .get("packet")
        .and_then(|section| section.get("bitsPerRow"))
        .cloned()
        .unwrap_or(Value::Number(DEFAULT_BITS_PER_ROW));
    Ok(BitsPerRow::of(&value))
}
