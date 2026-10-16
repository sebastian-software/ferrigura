//! The class diagram parser (M6): `parser/classDiagram.jison` and the class DB `classDb.ts`, ported
//! to Rust. The plan, the rules and the quirks that are reproduced are in
//! `docs-port/class/PLAN.md`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter
//! (the shared preparation), directives, and comment lines. The class grammar takes no other
//! change: no entity encoding and no line break at the end.

mod db;
mod grammar;
mod lexer;
mod member;

use ferrigura_model::class::{ClassDiagram, ClassMember};

use crate::error::ParseError;
use crate::flowchart::clean_comments;
use crate::preprocess::prepare;
use crate::preprocess::remove_directives;

/// The `securityLevel` of the configuration, as far as the class DB looks at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityLevel {
    /// `strict`, the default.
    #[default]
    Strict,
    /// `antiscript`.
    Antiscript,
    /// `sandbox`: link targets are `_top`.
    Sandbox,
    /// `loose`: URLs are not sanitised.
    Loose,
    /// Any other value: the reference treats it as neither strict nor loose.
    Other,
}

/// The options of the parser, which change how the DB sanitises its texts. The defaults are the
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
    let cleaned = clean_comments(&remove_directives(&prepared.text));
    let mut db = db::ClassDb::new(options);
    // `Diagram.ts` sets the front matter's title before the parser runs, and only when it is set.
    if let Some(title) = prepared.title.as_deref().filter(|title| !title.is_empty()) {
        db.set_diagram_title(title);
    }
    grammar::parse(&cleaned, prepared.line_offset, db)
}

/// `new ClassMember(input, type)` of the reference, with the default options: the text is
/// sanitised, then parsed as an attribute or a method. `None` where the reference throws (a
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
