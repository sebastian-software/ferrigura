//! The C4 diagram parser: `parser/c4Diagram.jison` and the DB `c4Db.ts`, ported by hand for the five
//! headers (`C4Context`, `C4Container`, `C4Component`, `C4Dynamic` and `C4Deployment`). The rule
//! table, the grammar and the DB calls are in `docs-port/c4/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The front matter title does not
//! reach a C4 diagram, because the C4 DB has no title setter.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::c4::C4Diagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// The options of the parser. The defaults are the reference's defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// The top-level `wrap` configuration, which the DB stores on each element (`autoWrap()`).
    /// It is `false` when the configuration does not set it.
    pub wrap: bool,
}

/// Parses a C4 diagram with the default options.
///
/// # Errors
///
/// A text that the reference rejects, with the position of the first token the grammar refuses.
pub fn parse(raw: &str) -> Result<C4Diagram, ParseError> {
    parse_with(raw, Options::default())
}

/// Parses a C4 diagram with the given options.
///
/// # Errors
///
/// The same as [`parse`].
pub fn parse_with(raw: &str, options: Options) -> Result<C4Diagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let tokens = scanner::scan(&text);
    let mut db = db::Db::new(options.wrap);
    grammar::parse(source, &tokens, &mut db)?;
    Ok(db.into_model())
}
