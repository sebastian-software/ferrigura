//! The lexer of the cynefin grammar, `packages/parser/src/language/cynefin/cynefin.langium`, with
//! the common terminals of `packages/parser/src/language/common/common.langium`.
//!
//! Chevrotain takes the first terminal that matches at a position, in the order of the lexer
//! definition, and not the longest one. The order is `docs-port/cynefin/NOTES.md`, section 2.2: the
//! header in its colon form and in its guarded form, `-->`, `:`, the domain names, `ACC_DESCR`,
//! `ACC_TITLE`, `TITLE`, `STRING`, `NEWLINE`, and then the hidden `WHITESPACE`, `YAML`, `DIRECTIVE`
//! and `SINGLE_LINE_COMMENT`. A position that no terminal takes is a lexical error, and the
//! reference rejects the whole text for it. Domain names have no word end, so `complexity` is the
//! domain `complex` followed by an error.

use crate::common::{descr_block_text, plain_text, string_text};
use crate::error::{ParseError, Source};
use crate::lexer::{colon_form, directive_len, yaml_len};
use crate::text::is_js_space;

/// `DOMAIN_NAME`, in the order of the grammar's alternatives.
pub(crate) const DOMAIN_NAMES: [&str; 5] =
    ["complex", "complicated", "clear", "chaotic", "confusion"];

/// A token of the grammar. The hidden terminals are not tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `cynefin-beta` or `cynefin-beta:`: the header, which the grammar reads in either form.
    Header,
    /// `DOMAIN_NAME`.
    Domain(&'static str),
    /// `-->`.
    Arrow,
    /// `:`, which introduces a transition label.
    Colon,
    /// `STRING`, with the quotes removed and the escapes resolved.
    Str(String),
    /// `ACC_DESCR`, the value after the common converter.
    AccDescr(String),
    /// `ACC_TITLE`, the value after the common converter.
    AccTitle(String),
    /// `TITLE`, the value after the common converter.
    Title(String),
    /// `NEWLINE`.
    Newline,
}

/// A token with the byte offset where it starts in the prepared text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// What the lexer does at one position: emit a token of this many bytes, or skip hidden text.
enum Step {
    Emit(Token, usize),
    Hidden(usize),
}

/// Tokenises the prepared text of `source`, without the hidden tokens.
///
/// # Errors
///
/// A lexical error at the first position that no terminal takes.
pub(crate) fn tokenize(source: Source<'_>) -> Result<Vec<Spanned>, ParseError> {
    let text = source.text();
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let len = match next_step(&text[pos..]) {
            Some(Step::Emit(token, len)) if len > 0 => {
                tokens.push(Spanned { token, start: pos });
                len
            }
            Some(Step::Hidden(len)) if len > 0 => len,
            _ => return Err(source.error(pos, "unexpected character")),
        };
        pos += len;
    }
    Ok(tokens)
}

/// The first terminal that matches at the start of `rest`, in the lexer's order.
fn next_step(rest: &str) -> Option<Step> {
    if let Some(len) = header_len(rest) {
        return Some(Step::Emit(Token::Header, len));
    }
    if rest.starts_with("-->") {
        return Some(Step::Emit(Token::Arrow, 3));
    }
    if rest.starts_with(':') {
        return Some(Step::Emit(Token::Colon, 1));
    }
    if let Some(&name) = DOMAIN_NAMES.iter().find(|name| rest.starts_with(**name)) {
        return Some(Step::Emit(Token::Domain(name), name.len()));
    }
    if let Some((value, len)) = acc_descr_at(rest) {
        return Some(Step::Emit(Token::AccDescr(value), len));
    }
    if let Some((value, len)) = acc_title_at(rest) {
        return Some(Step::Emit(Token::AccTitle(value), len));
    }
    if let Some((value, len)) = title_at(rest) {
        return Some(Step::Emit(Token::Title(value), len));
    }
    if let Some(len) = string_len(rest) {
        return Some(Step::Emit(Token::Str(string_text(&rest[..len])), len));
    }
    if let Some(len) = newline_len(rest) {
        return Some(Step::Emit(Token::Newline, len));
    }
    // WHITESPACE comes after the terminals that start with blanks, so a title keeps its blank.
    let blanks = blank_len(rest);
    if blanks > 0 {
        return Some(Step::Hidden(blanks));
    }
    if let Some(len) = yaml_hidden_len(rest) {
        return Some(Step::Hidden(len));
    }
    if rest.starts_with("%%") {
        return Some(Step::Hidden(comment_len(rest)));
    }
    None
}

/// `cynefin-beta:` (no word end), or `cynefin-beta` with `(?:(?=%%)|(?!\S))` after it: the word
/// ends at `%%`, at a blank, or at the end of the text.
fn header_len(rest: &str) -> Option<usize> {
    if rest.starts_with("cynefin-beta:") {
        return Some("cynefin-beta:".len());
    }
    let after = rest.strip_prefix("cynefin-beta")?;
    let guarded = after.is_empty() || after.starts_with("%%") || after.starts_with(is_js_space);
    guarded.then_some("cynefin-beta".len())
}

/// `[\t ]*` at the start of `rest`, in bytes.
fn blank_len(rest: &str) -> usize {
    rest.bytes()
        .take_while(|&b| b == b'\t' || b == b' ')
        .count()
}

/// `\r?\n` at the start of `rest`, in bytes.
fn newline_len(rest: &str) -> Option<usize> {
    if rest.starts_with("\r\n") {
        Some(2)
    } else if rest.starts_with('\n') {
        Some(1)
    } else {
        None
    }
}

/// Bytes up to, not including, the next line break (`[^\n\r]*`).
fn line_len(rest: &str) -> usize {
    rest.find(['\n', '\r']).unwrap_or(rest.len())
}

/// `ACC_DESCR: /[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)|[^\n\r]*)|\s*{([^}]*)})/`: the colon form
/// as for `accTitle`, or a block in braces that may run over several lines.
fn acc_descr_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    let after = rest[lead..].strip_prefix("accDescr")?;
    let keyword = lead + "accDescr".len();
    if let Some((value, consumed)) = colon_form(after) {
        return Some((value, keyword + consumed));
    }
    let space = after.len() - after.trim_start_matches(is_js_space).len();
    let block = after[space..].strip_prefix('{')?;
    let close = block.find('}')?;
    Some((
        descr_block_text(&block[..close]),
        keyword + space + 1 + close + 1,
    ))
}

/// `ACC_TITLE: /[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)/`.
fn acc_title_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    let after = rest[lead..].strip_prefix("accTitle")?;
    let (value, consumed) = colon_form(after)?;
    Some((value, lead + "accTitle".len() + consumed))
}

/// `TITLE: /[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)/`: a blank and the rest of the line
/// up to a `%%` on that line, or nothing when the keyword is not followed by a blank.
fn title_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    let after = rest[lead..].strip_prefix("title")?;
    let line = &after[..line_len(after)];
    let consumed = match line.as_bytes().first() {
        Some(b' ' | b'\t') => 1 + line[1..].find("%%").unwrap_or(line.len() - 1),
        _ => 0,
    };
    Some((
        plain_text(&after[..consumed]),
        lead + "title".len() + consumed,
    ))
}

/// `STRING: /"([^"\\]|\\.)*"|'([^'\\]|\\.)*'/`. The body takes any character, line breaks
/// included, except an escape of a line terminator (`.` does not match one). Returns the length
/// with both quotes.
fn string_len(rest: &str) -> Option<usize> {
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut chars = rest.char_indices().skip(1);
    while let Some((index, c)) = chars.next() {
        if c == quote {
            return Some(index + 1);
        }
        if c == '\\' {
            let (_, escaped) = chars.next()?;
            if matches!(escaped, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                return None;
            }
        }
    }
    None
}

/// `YAML: /---[\t ]*\r?\n(?:[\S\s]*?\r?\n)?---(?:\r?\n|(?!\S))/`, from its opening `---`.
fn yaml_hidden_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix("---")?;
    yaml_len(after).map(|len| 3 + len)
}

/// `DIRECTIVE` or `SINGLE_LINE_COMMENT`, from its `%%`: a directive when one closes with `}%%` and a
/// boundary, otherwise the rest of the line. Both are hidden.
fn comment_len(rest: &str) -> usize {
    let after = &rest[2..];
    let len = if after.starts_with('{') {
        directive_len(after).unwrap_or_else(|| line_len(after))
    } else {
        line_len(after)
    };
    2 + len
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<Token> {
        tokenize(Source::new(text, 0))
            .expect("lexes")
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn a_domain_name_is_a_prefix_and_the_rest_is_an_error() {
        assert_eq!(kinds("complex"), vec![Token::Domain("complex")]);
        assert!(tokenize(Source::new("complexity", 0)).is_err());
    }

    #[test]
    fn the_header_keeps_its_word_end_rule_except_in_the_colon_form() {
        assert_eq!(kinds("cynefin-beta:").first(), Some(&Token::Header));
        assert!(tokenize(Source::new("cynefin-beta:x", 0)).is_err());
        assert!(tokenize(Source::new("cynefin-betaX", 0)).is_err());
        assert_eq!(kinds("cynefin-beta%%x"), vec![Token::Header]);
    }

    #[test]
    fn a_title_keeps_the_blank_that_starts_it() {
        assert_eq!(kinds(" title a  b %% c"), vec![Token::Title("a b".into())]);
    }

    #[test]
    fn a_string_may_span_lines_but_not_escape_a_line_break() {
        assert_eq!(kinds("\"a\nb\""), vec![Token::Str("a\nb".into())]);
        assert!(tokenize(Source::new("\"a\\\nb\"", 0)).is_err());
    }
}
