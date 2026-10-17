//! The ishikawa (fishbone) parser (P2): `ishikawa.jison` and the DB `ishikawaDb.ts`, ported by hand.
//! The plan, the rule table and the quirks are in `docs-port/ishikawa/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar then calls the DB as
//! each statement is read, in document order, and the DB's values become the model.

mod db;
mod scanner;

use ferrigura_model::ishikawa::IshikawaDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::js_trim;

use db::IshikawaDb;
use scanner::{Spanned, Token, scan};

/// Parses an ishikawa diagram and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<IshikawaDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let mut db = IshikawaDb::default();
    // `Diagram.fromText` sets the front matter title before the parse; a statement replaces it.
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    parse_tokens(&scan(&text), source, &mut db)?;
    Ok(db.into_model())
}

/// `start: spaceLines? ISHIKAWA NL? document`, where `document` is one or more statements, each
/// followed by its stop. A statement that is a cause or the head calls the DB as it is read.
fn parse_tokens(
    tokens: &[Spanned],
    source: Source<'_>,
    db: &mut IshikawaDb,
) -> Result<(), ParseError> {
    let mut at = 0;
    while matches!(kind(tokens, at), Some(Token::SpaceLine | Token::Nl)) {
        at += 1;
    }
    if !matches!(kind(tokens, at), Some(Token::Ishikawa)) {
        return Err(unexpected(
            source,
            tokens,
            at,
            "expected the 'ishikawa' header",
        ));
    }
    at += 1;
    if matches!(kind(tokens, at), Some(Token::Nl)) {
        at += 1;
    }

    loop {
        at = statement(tokens, at, source, db)?;
        at = stop(tokens, at, source)?;
        if at >= tokens.len() {
            return Ok(());
        }
    }
}

/// `statement: SPACELIST TEXT | TEXT | SPACELINE | SPACELIST`. A cause is read with its
/// indentation (the length of the blanks), and the text is trimmed before the DB sees it.
fn statement(
    tokens: &[Spanned],
    at: usize,
    source: Source<'_>,
    db: &mut IshikawaDb,
) -> Result<usize, ParseError> {
    match kind(tokens, at) {
        Some(Token::SpaceList(indent)) => {
            if let Some(Token::Text(text)) = kind(tokens, at + 1) {
                db.add_node(to_level(*indent), js_trim(text));
                Ok(at + 2)
            } else {
                Ok(at + 1)
            }
        }
        Some(Token::Text(text)) => {
            db.add_node(0, js_trim(text));
            Ok(at + 1)
        }
        Some(Token::SpaceLine) => Ok(at + 1),
        _ => Err(unexpected(
            source,
            tokens,
            at,
            "expected a cause or a blank line",
        )),
    }
}

/// `stop: NL | EOF | SPACELINE | stop NL | stop EOF`: one line end, then any number of line breaks
/// and the end of the text.
fn stop(tokens: &[Spanned], at: usize, source: Source<'_>) -> Result<usize, ParseError> {
    if !matches!(
        kind(tokens, at),
        Some(Token::Nl | Token::Eof | Token::SpaceLine)
    ) {
        return Err(unexpected(
            source,
            tokens,
            at,
            "expected the end of the line",
        ));
    }
    let mut at = at + 1;
    while matches!(kind(tokens, at), Some(Token::Nl | Token::Eof)) {
        at += 1;
    }
    Ok(at)
}

/// The kind of the token at `at`, or `None` past the end.
fn kind(tokens: &[Spanned], at: usize) -> Option<&Token> {
    tokens.get(at).map(|spanned| &spanned.token)
}

/// A blank count as the DB's level. A count that does not fit an `i64` is not reachable from a
/// text the scanner reads, so it saturates.
fn to_level(indent: usize) -> i64 {
    i64::try_from(indent).unwrap_or(i64::MAX)
}

/// An error at the token at `at`, or at the end of the text past the last token.
fn unexpected(source: Source<'_>, tokens: &[Spanned], at: usize, message: &str) -> ParseError {
    let offset = tokens
        .get(at)
        .map_or(source.text().len(), |spanned| spanned.start);
    source.error(offset, message)
}
