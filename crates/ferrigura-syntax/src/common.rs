//! The shared rules of `packages/parser/src/language/common/common.langium`: the value conversions
//! of `title`, `accTitle` and `accDescr`, and the header they make up.

use crate::cursor::Cursor;
use crate::error::ParseError;
use crate::lexer::Token;
use crate::text::{
    collapse_blanks, collapse_line_breaks, js_trim, strip_line_ends, strip_line_starts,
};

/// The text of a single-line header value: trimmed, with runs of blanks collapsed.
pub(crate) fn plain_text(text: &str) -> String {
    collapse_blanks(js_trim(text))
}

/// The text of a multi-line `accDescr { … }` block, with the indentation of each line removed,
/// blank lines squeezed and runs of blanks collapsed.
pub(crate) fn descr_block_text(content: &str) -> String {
    let stripped = strip_line_ends(&strip_line_starts(content));
    collapse_line_breaks(&collapse_blanks(&stripped))
}

/// The value of a `STRING` terminal: the quotes are removed and escapes are resolved as Langium's
/// default value converter does.
pub(crate) fn string_text(input: &str) -> String {
    // The lexer matched two ASCII quotes around the body, so the slice is on character boundaries.
    let body = &input[1..input.len() - 1];
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some(escaped) = chars.next() else { break };
        out.push(match escaped {
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'v' => '\u{b}',
            '0' => '\0',
            other => other,
        });
    }
    out
}

/// The header of a diagram: the last `title`, `accTitle` and `accDescr` given, as the grammar
/// assigns them. An empty value means none was given.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Header {
    pub title: String,
    pub acc_title: String,
    pub acc_descr: String,
}

impl Header {
    /// Reads one `TitleAndAccessibilities` line if the next token starts one, `((accDescr |
    /// accTitle | title) EOL)`, and reports whether it did.
    pub(crate) fn take_line(&mut self, cursor: &mut Cursor) -> Result<bool, ParseError> {
        let Some(token) = cursor.peek().cloned() else {
            return Ok(false);
        };
        match token {
            Token::Title(text) => self.title = text,
            Token::AccTitle(text) => self.acc_title = text,
            Token::AccDescr(text) => self.acc_descr = text,
            _ => return Ok(false),
        }
        cursor.advance();
        cursor.expect_eol()?;
        Ok(true)
    }
}
