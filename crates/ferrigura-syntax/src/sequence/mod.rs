//! The sequence diagram parser (M6): `sequenceDiagram.jison` and the sequence DB `sequenceDb.ts`,
//! ported to Rust. The plan, the rules and the quirks are in `docs-port/sequence/PLAN.md`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter
//! (the shared preparation), directives and comment lines (removed here, as `preprocessDiagram`
//! does), and the entity encoding with a line break (`Diagram.fromText`).

mod db;
mod grammar;
mod lexer;

use ferrigura_model::sequence::SequenceDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

/// Options of the parser. The defaults are the reference's defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// The value of `autoWrap()`: whether a text without a `wrap:` or `nowrap:` prefix wraps. The
    /// reference reads it from the configuration (`wrap`, then `sequence.wrap`), and the caller
    /// resolves it. It is `false` by default.
    pub wrap: bool,
}

/// Parses a sequence diagram with the default options.
pub fn parse(raw: &str) -> Result<SequenceDiagram, ParseError> {
    parse_with(raw, Options::default())
}

/// Parses a sequence diagram with the given options.
pub fn parse_with(raw: &str, options: Options) -> Result<SequenceDiagram, ParseError> {
    let prepared = prepare(raw)?;
    // `preprocessDiagram` removes the directives and the comment lines; `Diagram.fromText` encodes
    // the entities and adds the line break that the grammar expects at the end.
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let tokens = lexer::lex(&text).map_err(|error| source.error(error.offset, error.message))?;
    let parsed = grammar::parse(source, tokens)?;

    // `yy.apply` runs after the whole parse, so a DB error comes after every syntax error.
    let mut db = db::Db::new(options);
    for (item, offset) in &parsed.items {
        db.apply(item)
            .map_err(|db::DbError(message)| source.error(*offset, message))?;
    }
    // The front matter's title is set before the parse, and the body's title replaces it.
    let title = parsed.title.as_deref().or(prepared.title.as_deref());
    Ok(db.into_model(
        title,
        parsed.acc_title.as_deref(),
        parsed.acc_descr.as_deref(),
    ))
}
