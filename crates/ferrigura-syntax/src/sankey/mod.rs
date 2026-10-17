//! The sankey parser: `sankey.jison` and the sankey DB, `sankeyDB.ts`, ported by hand. The plan, the
//! rule table and the quirks are in `docs-port/sankey/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding and a line break. Then `prepareTextForParsing` (the
//! parser's `parse` wrapper) normalises the blanks and the line breaks. The grammar runs the DB calls
//! in document order, one record at a time.

mod db;
mod scanner;

use ferrigura_model::sankey::SankeyDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::{is_cr_lf, is_js_space, js_parse_float, js_trim};

use db::SankeyDb;
use scanner::{Spanned, Token, scan};

/// Parses a sankey diagram and returns its model.
///
/// The error position is the first token the grammar rejects. Lines are counted in the text the
/// grammar reads, after `prepareTextForParsing` has removed its blank lines, plus the lines of the
/// front matter, as the other parsers count them.
///
/// # Errors
///
/// A text the grammar rejects: a header that is not `sankey` alone on its line, no record, a record
/// that does not have three fields, an empty field, text that no field takes (non-ASCII text, a tab,
/// a double quote in an unquoted field), an unclosed quoted field, and an entity (`#name;`, whose
/// placeholder is not ASCII).
pub fn parse(raw: &str) -> Result<SankeyDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = prepare_text_for_parsing(&(encode_entities(&cleaned) + "\n"));
    let source = Source::new(&text, prepared.line_offset);

    let mut db = SankeyDb::default();
    // `Diagram.fromText` sets the front matter title before the parse.
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    let tokens = scan(source)?;
    parse_tokens(&tokens, source, &mut db)?;
    Ok(db.into_model())
}

/// `prepareTextForParsing` (`sankeyUtils.ts`): the blanks at the very start and the very end of the
/// text go, every run of line breaks becomes one line break, and the text is trimmed. Blanks at the
/// start or end of the other lines stay, and a line of blanks in the middle stays as well, which the
/// grammar then rejects.
pub(crate) fn prepare_text_for_parsing(text: &str) -> String {
    let is_blank = |c: char| is_js_space(c) && !is_cr_lf(c);
    let inner = text.trim_start_matches(is_blank).trim_end_matches(is_blank);
    let mut out = String::with_capacity(inner.len());
    let mut in_break = false;
    for c in inner.chars() {
        if is_cr_lf(c) {
            if !in_break {
                out.push('\n');
            }
            in_break = true;
        } else {
            out.push(c);
            in_break = false;
        }
    }
    js_trim(&out).to_owned()
}

/// `start: SANKEY NEWLINE csv opt_eof`, where `csv` is records joined by line breaks. Each record
/// calls the DB when it is read: its source and then its target are created, and the link is added
/// with the value that `parseFloat` gives the third field.
fn parse_tokens(
    tokens: &[Spanned],
    source: Source<'_>,
    db: &mut SankeyDb,
) -> Result<(), ParseError> {
    let mut tokens = tokens.iter();
    match tokens.next() {
        Some(Spanned {
            token: Token::Sankey,
            ..
        }) => {}
        other => return Err(unexpected(source, other, "expected 'sankey'")),
    }
    match tokens.next() {
        Some(Spanned {
            token: Token::Newline,
            ..
        }) => {}
        other => {
            return Err(unexpected(
                source,
                other,
                "expected a line break after 'sankey'",
            ));
        }
    }
    loop {
        let (source_field, target_field, value_field) = record(&mut tokens, source)?;
        let source_node = db.find_or_create_node(&field_name(source_field));
        let target_node = db.find_or_create_node(&field_name(target_field));
        db.add_link(
            source_node,
            target_node,
            js_parse_float(js_trim(value_field)),
        );
        match tokens.next() {
            Some(Spanned {
                token: Token::Newline,
                ..
            }) => {}
            Some(Spanned {
                token: Token::Eof, ..
            }) => return Ok(()),
            other => {
                return Err(unexpected(
                    source,
                    other,
                    "expected a line break or the end of the text",
                ));
            }
        }
    }
}

/// `record: field[source] COMMA field[target] COMMA field[value]`. The fields are returned as the
/// scanner gave them.
fn record<'a>(
    tokens: &mut std::slice::Iter<'a, Spanned>,
    source: Source<'_>,
) -> Result<(&'a str, &'a str, &'a str), ParseError> {
    let source_field = field(tokens, source)?;
    comma(tokens, source)?;
    let target_field = field(tokens, source)?;
    comma(tokens, source)?;
    let value_field = field(tokens, source)?;
    Ok((source_field, target_field, value_field))
}

/// `field: escaped | non_escaped`: a run of text, or a quoted text between two quotes, whose text is
/// returned as written.
fn field<'a>(
    tokens: &mut std::slice::Iter<'a, Spanned>,
    source: Source<'_>,
) -> Result<&'a str, ParseError> {
    match tokens.next() {
        Some(Spanned {
            token: Token::Text(text),
            ..
        }) => Ok(text),
        Some(Spanned {
            token: Token::Quote,
            ..
        }) => {
            let text = match tokens.next() {
                Some(Spanned {
                    token: Token::Escaped(text),
                    ..
                }) => text,
                other => {
                    return Err(unexpected(
                        source,
                        other,
                        "expected the text of a quoted field",
                    ));
                }
            };
            match tokens.next() {
                Some(Spanned {
                    token: Token::Quote,
                    ..
                }) => Ok(text),
                other => Err(unexpected(source, other, "expected the closing quote")),
            }
        }
        other => Err(unexpected(source, other, "expected a field")),
    }
}

fn comma(tokens: &mut std::slice::Iter<'_, Spanned>, source: Source<'_>) -> Result<(), ParseError> {
    match tokens.next() {
        Some(Spanned {
            token: Token::Comma,
            ..
        }) => Ok(()),
        other => Err(unexpected(source, other, "expected a comma")),
    }
}

/// A node name as the action gives it: the field trimmed, and each `""` written as `"`.
fn field_name(field: &str) -> String {
    js_trim(field).replace("\"\"", "\"")
}

/// An error at the token that the grammar did not expect, or at the end of the text.
fn unexpected(source: Source<'_>, spanned: Option<&Spanned>, message: &str) -> ParseError {
    match spanned {
        Some(spanned) => source.error(spanned.start, message),
        None => source.error(source.text().len(), message),
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_text_for_parsing;

    #[test]
    fn the_blanks_at_the_very_start_and_end_go() {
        assert_eq!(
            prepare_text_for_parsing("  \n sankey\na,b,1  \n\n"),
            "sankey\na,b,1"
        );
    }

    #[test]
    fn a_run_of_line_breaks_becomes_one() {
        assert_eq!(
            prepare_text_for_parsing("sankey\r\n\r\n\na,b,1"),
            "sankey\na,b,1"
        );
    }

    #[test]
    fn a_line_of_blanks_in_the_middle_stays() {
        assert_eq!(
            prepare_text_for_parsing("sankey\n  \na,b,1"),
            "sankey\n  \na,b,1"
        );
    }

    #[test]
    fn the_blanks_at_the_end_of_a_line_in_the_middle_stay() {
        assert_eq!(
            prepare_text_for_parsing("sankey\na,b,1 \nc,d,2"),
            "sankey\na,b,1 \nc,d,2"
        );
    }
}
