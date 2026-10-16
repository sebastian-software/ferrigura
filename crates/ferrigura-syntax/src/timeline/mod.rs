//! The timeline parser (M6): `timeline.jison` and the timeline DB `timelineDb.js`, ported to Rust.
//! The plan, the lexer rules and the quirks the port keeps are in `docs-port/timeline/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the grammar sees it: front matter
//! (its title is dropped, as the timeline DB has no setter for it), directives, comment lines,
//! entities and a line break. Detection is the caller's job: [`crate::detect`] reads the type first.

mod db;
mod scanner;

use ferrigura_model::timeline::TimelineDiagram;

use crate::error::{ParseError, Source};
use crate::flowchart::{cleanup_comments, encode_entities, remove_directives};
use crate::preprocess::prepare;
use crate::text::js_trim;

use db::Db;
use scanner::{Header, Spanned, Token, tokenize};

/// Parses a timeline, with its front matter, and returns its model.
pub fn parse(raw: &str) -> Result<TimelineDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let tokens = tokenize(&text);
    let mut db = Db::new();
    document(&tokens, source, &mut db)?;
    Ok(db.into_model())
}

/// `start: timeline_header document EOF`, with the actions of each statement.
fn document(tokens: &[Spanned], source: Source<'_>, db: &mut Db) -> Result<(), ParseError> {
    let Some(first) = tokens.first() else {
        return Err(source.error(0, "expected 'timeline' at the start of the diagram"));
    };
    let Token::Header(header) = first.token else {
        return Err(source.error(
            first.start,
            "expected 'timeline' at the start of the diagram",
        ));
    };
    match header {
        Header::Lr => db.set_direction("LR"),
        Header::Td => db.set_direction("TD"),
        Header::Plain => {}
    }

    let mut at = 1;
    loop {
        // Without an `EOF` the input ended inside a value, or never reached its end.
        let Some(spanned) = tokens.get(at) else {
            return Err(source.error(source.text().len(), "expected the end of the diagram"));
        };
        at += 1;
        match &spanned.token {
            Token::Newline => {}
            Token::Eof => return Ok(()),
            Token::Title(raw) => db.set_title(after_chars(raw, 6)),
            Token::AccTitle => match tokens.get(at) {
                Some(Spanned {
                    token: Token::AccTitleValue(value),
                    ..
                }) => {
                    at += 1;
                    db.set_acc_title(js_trim(value));
                }
                _ => return Err(source.error(spanned.start, "expected the value of accTitle")),
            },
            Token::AccDescr => match tokens.get(at) {
                Some(Spanned {
                    token: Token::AccDescrValue(value),
                    ..
                }) => {
                    at += 1;
                    db.set_acc_descr(js_trim(value));
                }
                _ => return Err(source.error(spanned.start, "expected the value of accDescr")),
            },
            Token::AccDescrBlockValue(value) => db.set_acc_descr(js_trim(value)),
            Token::Section(raw) => db.add_section(after_chars(raw, 8)),
            Token::Period(raw) => db.add_period(raw),
            Token::Event(raw) => {
                if !db.add_event(after_chars(raw, 2)) {
                    return Err(source.error(spanned.start, "an event needs a period before it"));
                }
            }
            Token::Invalid => return Err(source.error(spanned.start, "unexpected character")),
            _ => return Err(source.error(spanned.start, "unexpected keyword or value")),
        }
    }
}

/// The text after its first `count` characters. The keyword of each statement is ASCII, so this
/// cuts the keyword and the one character that follows it, as `substr` does in the reference.
fn after_chars(text: &str, count: usize) -> &str {
    text.char_indices()
        .nth(count)
        .map_or("", |(index, _)| &text[index..])
}
