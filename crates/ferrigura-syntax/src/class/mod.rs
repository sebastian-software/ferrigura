//! The class diagram parser: `parser/classDiagram.jison` and the class DB `classDb.ts`, ported to
//! Rust. The rules and the quirks that are kept are in
//! `docs/diagrams/class.md#parsing`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter
//! (the shared preparation), directives, and comment lines. The class grammar takes no other
//! change: no entity encoding and no line break at the end.

mod db;
mod grammar;
mod lexer;
mod member;

/// `parseGenericTypes` of `diagrams/common/common.ts`, which the ER and class renderers share.
pub use member::parse_generic_types;

pub use crate::url::SecurityLevel;

use ferrigura_model::class::{ClassDiagram, ClassMember};

use crate::error::ParseError;
use crate::flowchart::clean_comments;
use crate::preprocess::prepare;
use crate::preprocess::remove_directives;

/// The options of the parser, which change how the DB sanitizes its texts. The defaults are the
/// reference's defaults: `htmlLabels` is on and `securityLevel` is `strict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// `htmlLabels`, on by default.
    pub html_labels: bool,
    /// `securityLevel`, `strict` by default.
    pub security_level: SecurityLevel,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            html_labels: true,
            security_level: SecurityLevel::Strict,
        }
    }
}

/// Parses a class diagram with the default options.
pub fn parse(raw: &str) -> Result<ClassDiagram, ParseError> {
    parse_with(raw, Options::default())
}

/// Parses a class diagram with the given options. The text is the diagram text, as
/// `Diagram.fromText` gives it to the parser.
pub fn parse_with(raw: &str, options: Options) -> Result<ClassDiagram, ParseError> {
    let prepared = prepare(raw)?;
    parse_prepared(&prepared, options)
}

/// Parses an already prepared source with the given options.
pub fn parse_prepared(
    prepared: &crate::preprocess::Prepared,
    options: Options,
) -> Result<ClassDiagram, ParseError> {
    parse_prepared_with_db(prepared, db::ClassDb::new(options))
}

/// Parses an already prepared source for rendering: encodes Mermaid `#...;` entities on click
/// links before `formatUrl` runs (matching Mermaid's `Diagram.fromText` `encodeEntities` order
/// without affecting the differential parser model of [`parse_prepared`]).
pub fn parse_prepared_for_render(
    prepared: &crate::preprocess::Prepared,
    options: Options,
) -> Result<ClassDiagram, ParseError> {
    parse_prepared_with_db(prepared, db::ClassDb::new_for_render(options))
}

fn parse_prepared_with_db(
    prepared: &crate::preprocess::Prepared,
    mut db: db::ClassDb,
) -> Result<ClassDiagram, ParseError> {
    let cleaned = clean_comments(&remove_directives(&prepared.text));
    // `Diagram.ts` sets the front matter's title before the parser runs, and only when it is set.
    if let Some(title) = prepared.title.as_deref().filter(|title| !title.is_empty()) {
        db.set_diagram_title(title);
    }
    grammar::parse(&cleaned, prepared.line_offset, db)
}

/// `new ClassMember(input, type)` of the reference, with the default options: the text is
/// sanitized, then parsed as an attribute or a method. `None` where the reference throws (a
/// method whose text has a `)` but no `(` before it).
pub fn class_member(input: &str, is_method: bool) -> Option<ClassMember> {
    db::member_model(Options::default(), input, is_method)
}

/// Parses the grammar and the DB alone, with the default options: the text is not preprocessed
/// (no front matter, directive or comment removal). The ported parser specs call the reference's
/// parser this way.
pub fn parse_grammar(text: &str) -> Result<ClassDiagram, ParseError> {
    grammar::parse(text, 0, db::ClassDb::new(Options::default()))
}
