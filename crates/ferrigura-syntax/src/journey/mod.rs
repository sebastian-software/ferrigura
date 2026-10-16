//! The user journey parser (M6): `journey.jison` and the journey DB `journeyDb.js`, ported by hand.
//! The plan, the rule table and the quirks are in `docs-port/journey/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar then runs its DB calls
//! in document order, and the DB's values become the model.

mod db;
mod scanner;

use ferrigura_model::journey::JourneyDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::js_trim;

use db::JourneyDb;
use scanner::{Spanned, Token, scan};

/// Parses a user journey and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<JourneyDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let mut db = JourneyDb::default();
    // `Diagram.fromText` sets the front matter title before the parse; a grammar title replaces it.
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    parse_tokens(&scan(&text), source, &mut db)?;
    Ok(db.into_model())
}

/// `start: journey document EOF`, where each statement calls the DB when it is read.
fn parse_tokens(
    tokens: &[Spanned],
    source: Source<'_>,
    db: &mut JourneyDb,
) -> Result<(), ParseError> {
    let mut tokens = tokens.iter();
    match tokens.next() {
        Some(Spanned {
            token: Token::Journey,
            ..
        }) => {}
        Some(spanned) => return Err(unexpected(source, spanned, "expected 'journey'")),
        None => return Err(source.error(0, "expected 'journey'")),
    }

    while let Some(spanned) = tokens.next() {
        match &spanned.token {
            Token::Eof => return Ok(()),
            Token::RawEof => return Err(unexpected(source, spanned, "unexpected end of the text")),
            Token::Newline => {}
            Token::Title(text) => db.set_diagram_title(text),
            Token::AccTitle => {
                let value = value_after(source, &mut tokens, spanned, "accTitle")?;
                db.set_acc_title(js_trim(&value));
            }
            Token::AccDescr => {
                let value = value_after(source, &mut tokens, spanned, "accDescr")?;
                db.set_acc_description(js_trim(&value));
            }
            Token::AccDescrMultiValue(text) => db.set_acc_description(js_trim(text)),
            Token::Section(name) => db.add_section(name),
            Token::TaskName(name) => match tokens.next() {
                Some(Spanned {
                    token: Token::TaskData(data),
                    ..
                }) => db.add_task(name, data),
                Some(other) => return Err(unexpected(source, other, "expected the task data")),
                None => return Err(source.error(source.text().len(), "expected the task data")),
            },
            _ => return Err(unexpected(source, spanned, "unexpected token")),
        }
    }
    Ok(())
}

/// The value token that follows an accessibility keyword, which the scanner always emits.
fn value_after(
    source: Source<'_>,
    tokens: &mut std::slice::Iter<'_, Spanned>,
    keyword: &Spanned,
    name: &str,
) -> Result<String, ParseError> {
    match tokens.next() {
        Some(Spanned {
            token: Token::AccTitleValue(value) | Token::AccDescrValue(value),
            ..
        }) => Ok(value.clone()),
        Some(other) => Err(unexpected(
            source,
            other,
            &format!("expected the text of {name}"),
        )),
        None => Err(unexpected(
            source,
            keyword,
            &format!("expected the text of {name}"),
        )),
    }
}

/// An error at a token, with a message that names what the grammar expected.
fn unexpected(source: Source<'_>, spanned: &Spanned, message: &str) -> ParseError {
    let detail = match spanned.token {
        Token::Colon => "unexpected ':'",
        Token::Invalid => "unexpected character",
        Token::Journey => "'journey' may only start the diagram",
        Token::TaskData(_) => "task data without a task name",
        _ => message,
    };
    source.error(spanned.start, detail)
}
