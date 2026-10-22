//! The sankey scanner: the rules of `sankey.jison`, in the grammar's order. Jison takes the first rule
//! that matches at the position, not the longest, and the rules follow the grammar's. The header
//! keyword is read in the initial state; the rest of the text is read in `csv`, and a quoted field is
//! read in `escaped_text`. See `docs-port/sankey/PLAN.md`, section 3.
//!
//! A rule that can match the empty text (the text field and the quoted text) matches nothing at a
//! character that no field takes, such as a tab, a non-ASCII character or the end of a quoted field.
//! Jison then returns an empty token again and again at the same position, and the grammar never
//! accepts that: every such text is a parse error, so the scanner reports it at once.

use crate::error::{ParseError, Source};

/// The header keyword, as the initial state reads it.
const HEADER: &str = "sankey";

/// The header keyword of the beta, which upstream Mermaid still takes (`<INITIAL>"sankey-beta"`).
const BETA_HEADER: &str = "sankey-beta";

/// The tokens of the grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `sankey`, the header keyword, in any case.
    Sankey,
    /// A line break.
    Newline,
    /// The comma between two fields.
    Comma,
    /// A double quote, which opens or closes a quoted field.
    Quote,
    /// A field outside quotes: one or more text characters.
    Text(String),
    /// The characters between the quotes as they are written, so `""` stays two quotes. Never empty.
    Escaped(String),
    /// The end of the text.
    Eof,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// Reads the whole text, which is the text after `prepareTextForParsing`. The result ends with
/// [`Token::Eof`], or with the error of a character no rule takes.
pub(crate) fn scan(source: Source<'_>) -> Result<Vec<Spanned>, ParseError> {
    let text = source.text();
    // The initial state takes `sankey-beta` or `sankey` in any case, in that order, with no word
    // boundary after it.
    let starts_with = |keyword: &str| {
        text.as_bytes()
            .get(..keyword.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(keyword.as_bytes()))
    };
    let header_len = if starts_with(BETA_HEADER) {
        BETA_HEADER.len()
    } else if starts_with(HEADER) {
        HEADER.len()
    } else {
        return Err(source.error(0, "expected the keyword 'sankey'"));
    };
    let mut tokens = vec![Spanned {
        token: Token::Sankey,
        start: 0,
    }];
    let mut pos = header_len;
    let mut quoted = false;
    loop {
        let rest = &text[pos..];
        if quoted {
            // `escaped_text`, rule 7: a quote that does not begin a `""` pair closes the field.
            if rest.starts_with('"') && !rest[1..].starts_with('"') {
                tokens.push(Spanned {
                    token: Token::Quote,
                    start: pos,
                });
                quoted = false;
                pos += 1;
                continue;
            }
            // `escaped_text`, rule 8: text characters, commas, line breaks and `""` pairs.
            let len = escaped_len(rest);
            if len == 0 {
                return Err(source.error(pos, "a character that a quoted field does not take"));
            }
            tokens.push(Spanned {
                token: Token::Escaped(rest[..len].to_owned()),
                start: pos,
            });
            pos += len;
            continue;
        }
        let Some(c) = rest.chars().next() else {
            tokens.push(Spanned {
                token: Token::Eof,
                start: pos,
            });
            return Ok(tokens);
        };
        let (token, len) = match c {
            '\n' => (Token::Newline, 1),
            ',' => (Token::Comma, 1),
            '"' => {
                quoted = true;
                (Token::Quote, 1)
            }
            _ => {
                // `csv`, the text rule: a run of text characters, which may be empty.
                let len = text_len(rest);
                if len == 0 {
                    return Err(source.error(pos, "a character that no field takes"));
                }
                (Token::Text(rest[..len].to_owned()), len)
            }
        };
        tokens.push(Spanned { token, start: pos });
        pos += len;
    }
}

/// Whether `byte` is in `TEXTDATA`: printable ASCII, without the double quote and the comma
/// (`sankey.jison` 19-25).
fn is_text_data(byte: u8) -> bool {
    (0x20..=0x7E).contains(&byte) && byte != b'"' && byte != b','
}

/// The length of the text rule's match: the text characters at the start of `rest`.
fn text_len(rest: &str) -> usize {
    rest.bytes().take_while(|&byte| is_text_data(byte)).count()
}

/// The length of the quoted text's match: text characters, commas, line breaks and `""` pairs, as
/// many as come in a row. A lone quote is not in it.
fn escaped_len(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if is_text_data(byte) || matches!(byte, b',' | b'\n' | b'\r') {
            index += 1;
        } else if byte == b'"' && bytes.get(index + 1) == Some(&b'"') {
            index += 2;
        } else {
            break;
        }
    }
    index
}
