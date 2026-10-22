//! The lexer of the packet grammar. The terminals are those of `common.langium` that the grammar
//! uses (`TITLE`, `ACC_TITLE`, `ACC_DESCR`, `STRING`, `NEWLINE` and the hidden ones) and the keyword
//! and punctuation of `packet.langium`. Chevrotain takes the first terminal that matches, and the
//! numbers are the unsigned `INT`: the shared lexer reads `-?[0-9]+`, which would take `0-10` as
//! `0` and `-10`, so the packet scanner has its own. Any other character is an error here, as it
//! is for the grammar, since no other terminal is used by a packet.

use crate::common::{descr_block_text, plain_text, string_text};
use crate::error::{ParseError, Source};
use crate::lexer::{colon_form, directive_len, yaml_len};
use crate::text::{is_js_space, is_line_term};

/// A token of the packet grammar. Comments, directives, front matter and blanks are hidden.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `NEWLINE`: `\r?\n`.
    Newline,
    /// The keyword `packet`, followed by a space, a line break, the end of the text or `%%`.
    Packet,
    /// `TITLE`: the value after the converter.
    Title(String),
    /// `ACC_TITLE`: the value after the converter.
    AccTitle(String),
    /// `ACC_DESCR`: the value after the converter.
    AccDescr(String),
    /// `STRING`: the quotes are removed and the escapes resolved.
    Str(String),
    /// `INT`: `0`, or a non-zero digit and the digits after it.
    Int(f64),
    /// `-`.
    Dash,
    /// `+`.
    Plus,
    /// `:`.
    Colon,
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// What one step of the scanner made of the text: a hidden piece, or a token and its length.
enum Step {
    Hidden(usize),
    Token(Token, usize),
}

/// Tokenises the prepared text of `source`, without the hidden pieces.
pub(crate) fn tokenize(source: &Source<'_>) -> Result<Vec<Spanned>, ParseError> {
    let text = source.text();
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        match step(&text[pos..]) {
            Ok(Step::Hidden(len)) => pos += len,
            Ok(Step::Token(token, len)) => {
                tokens.push(Spanned { token, start: pos });
                pos += len;
            }
            Err(message) => return Err(source.error(pos, message)),
        }
    }
    Ok(tokens)
}

/// The next piece of `rest`, which is not empty. The order of the checks is the order of the
/// terminals, with the single-character ones after the words they could start.
fn step(rest: &str) -> Result<Step, &'static str> {
    let first = rest.as_bytes()[0];
    match first {
        // WHITESPACE: `[\t ]+`, hidden.
        b' ' | b'\t' => {
            let blanks = rest.len() - rest.trim_start_matches(['\t', ' ']).len();
            return Ok(Step::Hidden(blanks));
        }
        b'\n' => return Ok(Step::Token(Token::Newline, 1)),
        b'\r' if rest.starts_with("\r\n") => return Ok(Step::Token(Token::Newline, 2)),
        b'"' | b'\'' => return string(rest),
        b'0'..=b'9' => return Ok(number(rest)),
        b'-' if rest.starts_with("---") => {
            // YAML: a `---` block that is not at the start of the text, hidden.
            return yaml_len(&rest[3..])
                .map(|len| Step::Hidden(3 + len))
                .ok_or("unexpected character");
        }
        b'-' => return Ok(Step::Token(Token::Dash, 1)),
        b'+' => return Ok(Step::Token(Token::Plus, 1)),
        b':' => return Ok(Step::Token(Token::Colon, 1)),
        b'%' if rest.starts_with("%%") => return Ok(comment(&rest[2..])),
        _ => {}
    }
    if let Some(after) = rest.strip_prefix("title") {
        let (value, consumed) = title(after);
        return Ok(Step::Token(Token::Title(value), 5 + consumed));
    }
    if let Some(after) = rest.strip_prefix("accTitle") {
        return match colon_form(after) {
            Some((value, consumed)) => Ok(Step::Token(Token::AccTitle(value), 8 + consumed)),
            None => Err("expected ':' after the accessibility keyword"),
        };
    }
    if let Some(after) = rest.strip_prefix("accDescr") {
        let (value, consumed) = acc_descr(after)?;
        return Ok(Step::Token(Token::AccDescr(value), 8 + consumed));
    }
    // The header keywords, longest first: `"packet" | "packet-beta"`.
    for keyword in ["packet-beta", "packet"] {
        if let Some(after) = rest.strip_prefix(keyword)
            && keyword_boundary(after)
        {
            return Ok(Step::Token(Token::Packet, keyword.len()));
        }
    }
    Err("unexpected character")
}

/// `(?:(?=%%)|(?!\S))` after the keyword: the end of the text, a blank, or `%%` follows.
fn keyword_boundary(after: &str) -> bool {
    after.is_empty() || after.starts_with("%%") || after.chars().next().is_some_and(is_js_space)
}

/// The length of the first line of `text`, up to a line break.
fn line_len(text: &str) -> usize {
    text.find(['\n', '\r']).unwrap_or(text.len())
}

/// `DIRECTIVE` and `SINGLE_LINE_COMMENT`, after the `%%`: a directive when one closes, else the rest
/// of the line.
fn comment(after: &str) -> Step {
    let len = after
        .starts_with('{')
        .then(|| directive_len(after))
        .flatten()
        .unwrap_or_else(|| line_len(after));
    Step::Hidden(2 + len)
}

/// `TITLE: /[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)/`, after the keyword: a blank starts
/// the value, which reads to the first `%%` of the line or to its end. Returns the value and the
/// number of bytes it takes after the keyword.
fn title(after: &str) -> (String, usize) {
    let line = &after[..line_len(after)];
    let consumed = match line.as_bytes().first() {
        Some(b'\t' | b' ') => {
            let tail = &line[1..];
            1 + tail.find("%%").unwrap_or(tail.len())
        }
        _ => 0,
    };
    (plain_text(&after[..consumed]), consumed)
}

/// `ACC_DESCR`, after the keyword: the `:` form as `ACC_TITLE` has it, or `{ … }` over several
/// lines, with its value converted as the multi-line `accDescr` is.
fn acc_descr(after: &str) -> Result<(String, usize), &'static str> {
    if let Some((value, consumed)) = colon_form(after) {
        return Ok((value, consumed));
    }
    let space = after.len() - after.trim_start_matches(is_js_space).len();
    if !after[space..].starts_with('{') {
        return Err("expected ':' or '{' after accDescr");
    }
    let body = &after[space + 1..];
    let Some(close) = body.find('}') else {
        return Err("accDescr block is not closed by '}'");
    };
    Ok((descr_block_text(&body[..close]), space + 1 + close + 1))
}

/// `INT: /0|[1-9][0-9]*(?!\.)/`: a lone `0`, or the whole run of digits that starts with a
/// non-zero digit. A `.` after a number is left for the scanner to reject.
fn number(rest: &str) -> Step {
    let len = if rest.starts_with('0') {
        1
    } else {
        rest.bytes().take_while(u8::is_ascii_digit).count()
    };
    // A run of ASCII digits always parses, to infinity when it is beyond the double range.
    let value = rest[..len].parse::<f64>().unwrap_or(f64::NAN);
    Step::Token(Token::Int(value), len)
}

/// `STRING: /"([^"\\]|\\.)*"|'([^'\\]|\\.)*'/`. The body may span line breaks; an escape cannot
/// escape a line break, since `.` does not match one.
fn string(rest: &str) -> Result<Step, &'static str> {
    let quote = rest.as_bytes()[0];
    let mut i = 1;
    while i < rest.len() {
        let c = rest[i..].chars().next().unwrap_or_default();
        if c as u32 == u32::from(quote) {
            return Ok(Step::Token(Token::Str(string_text(&rest[..=i])), i + 1));
        }
        if c == '\\' {
            match rest[i + 1..].chars().next() {
                Some(escaped) if !is_line_term(escaped) => i += 1 + escaped.len_utf8(),
                _ => break,
            }
        } else {
            i += c.len_utf8();
        }
    }
    Err("unterminated string")
}
