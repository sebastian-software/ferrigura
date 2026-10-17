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

/// Parses a packet diagram, with the configuration that its own front matter and directives give,
/// and returns its model. A `bitsPerRow` given to `initialize` is not seen: [`parse_with`] takes
/// the effective configuration for that.
pub fn parse(raw: &str) -> Result<PacketDiagram, ParseError> {
    let configured = effective(None, Some("packet"), &config::read(raw)?);
    parse_with(raw, &configured)
}

/// Parses a packet diagram with the effective configuration `configured`, which the reference's
/// `getConfig` reads during the parse: the site's configuration, the front matter's and the
/// directives'. The rows depend on its `bitsPerRow`.
pub fn parse_with(raw: &str, configured: &Value) -> Result<PacketDiagram, ParseError> {
    let bits_per_row = BitsPerRow::of(&bits_per_row_value(configured));
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

/// `bitsPerRow` of an effective configuration, as the renderer's arithmetic reads it: the number
/// that `bitWidth * bitsPerRow` and `start % bitsPerRow` take, which is `NaN` for a string that is
/// not a number, as in the reference.
#[must_use]
pub fn bits_per_row(configured: &Value) -> f64 {
    BitsPerRow::of(&bits_per_row_value(configured)).times
}

/// The `bitsPerRow` of an effective configuration, in the packet scope, or the default.
fn bits_per_row_value(configured: &Value) -> Value {
    configured
        .get("packet")
        .and_then(|section| section.get("bitsPerRow"))
        .cloned()
        .unwrap_or(Value::Number(DEFAULT_BITS_PER_ROW))
}
