//! The shared lexer base, built on `logos`.
//!
//! The terminals follow `packages/parser/src/language/common/common.langium` and the pie and info
//! grammars. Terminals whose Langium regex reads to the end of the line (`title`, `accTitle`,
//! `accDescr`) are matched by logos on their keyword and then read by a callback, because logos
//! has no lazy quantifiers or lookahead. Keywords are not tokens of their own: a word is a
//! [`Token::Word`] and the parser decides whether it is a keyword.

use logos::{FilterResult, Lexer, Logos};

use crate::common::{descr_block_text, plain_text, string_text};
use crate::error::{ParseError, Source};
use crate::text::is_js_space;

/// Why a piece of input could not be tokenised.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum LexError {
    /// A character no terminal starts with.
    #[default]
    Unexpected,
    /// A number with a leading zero that is not `0` itself, or one that does not parse.
    MalformedNumber,
    /// `accTitle` or `accDescr` followed by something other than `:`.
    ExpectedColon,
    /// `accDescr` followed by neither `:` nor `{`.
    ExpectedDescrForm,
    /// `accDescr {` without a closing `}`.
    UnclosedBlock,
}

impl LexError {
    fn message(self) -> &'static str {
        match self {
            Self::Unexpected => "unexpected character",
            Self::MalformedNumber => "malformed number: a leading zero must stand alone",
            Self::ExpectedColon => "expected ':' after the accessibility keyword",
            Self::ExpectedDescrForm => "expected ':' or '{' after accDescr",
            Self::UnclosedBlock => "accDescr block is not closed by '}'",
        }
    }
}

/// One token of the input, with the byte offset where it starts.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// The terminals of the shared base and of the pie and info grammars.
///
/// Comments, directives and YAML blocks are matched and then skipped, so they never reach the
/// parser; their variants exist only so that the callbacks have a token to attach to.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[\t ]+")]
#[logos(error = LexError)]
pub(crate) enum Token {
    /// `NEWLINE: /\r?\n/`.
    #[regex(r"\r?\n")]
    Newline,

    /// Any word; keywords such as `pie` and `showInfo` are recognised by the parser.
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", word)]
    Word(String),

    /// `TITLE`: `title` and the rest of its line, as text.
    #[token("title", title)]
    Title(String),

    /// `ACC_TITLE`: `accTitle:` and the rest of its line, as text.
    #[token("accTitle", acc_title)]
    AccTitle(String),

    /// `ACC_DESCR`: `accDescr:` and the rest of its line, or `accDescr { … }` over several lines.
    #[token("accDescr", acc_descr)]
    AccDescr(String),

    /// `STRING`: a double or single quoted string, with escapes; the quotes are removed.
    #[regex(r#""([^"\\]|\\.)*"|'([^'\\]|\\.)*'"#, string)]
    Str(String),

    /// `NUMBER_PIE` and the shared `NUMBER`: an integer or a decimal, optionally negative.
    #[regex(r"-?[0-9]+(\.[0-9]+)?", number)]
    Number(f64),

    /// The `:` keyword between a slice label and its value.
    #[token(":")]
    Colon,

    /// `SINGLE_LINE_COMMENT` and `DIRECTIVE`: hidden.
    #[token("%%", comment)]
    #[allow(dead_code)]
    Comment,

    /// `YAML`: a front matter block that is not at the start of the text; hidden.
    #[token("---", yaml)]
    #[allow(dead_code)]
    Yaml,
}

fn line_len(rest: &str) -> usize {
    rest.find(['\n', '\r']).unwrap_or(rest.len())
}

fn word(lex: &mut Lexer<Token>) -> String {
    lex.slice().to_owned()
}

/// `TITLE: /[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)/`, matched after the keyword.
fn title(lex: &mut Lexer<Token>) -> String {
    let rest = lex.remainder();
    let line = &rest[..line_len(rest)];
    let consumed = match line.as_bytes().first() {
        Some(b'\t' | b' ') => {
            let after = &line[1..];
            1 + after.find("%%").unwrap_or(after.len())
        }
        _ => 0,
    };
    let value = plain_text(&rest[..consumed]);
    lex.bump(consumed);
    value
}

/// The `:` form of `accTitle` and `accDescr`: `[\t ]*:` and the rest of the line up to `%%`.
fn colon_form(rest: &str) -> Option<(String, usize)> {
    let blanks = rest.len() - rest.trim_start_matches(['\t', ' ']).len();
    if !rest[blanks..].starts_with(':') {
        return None;
    }
    let after = &rest[blanks + 1..];
    let line = &after[..line_len(after)];
    let consumed = blanks + 1 + line.find("%%").unwrap_or(line.len());
    Some((plain_text(&rest[blanks + 1..consumed]), consumed))
}

/// `ACC_TITLE: /[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)/`, matched after the keyword.
fn acc_title(lex: &mut Lexer<Token>) -> FilterResult<String, LexError> {
    match colon_form(lex.remainder()) {
        Some((value, consumed)) => {
            lex.bump(consumed);
            FilterResult::Emit(value)
        }
        None => FilterResult::Error(LexError::ExpectedColon),
    }
}

/// `ACC_DESCR`, matched after the keyword: the `:` form as for `accTitle`, or `{ … }` over lines.
fn acc_descr(lex: &mut Lexer<Token>) -> FilterResult<String, LexError> {
    let rest = lex.remainder();
    if let Some((value, consumed)) = colon_form(rest) {
        lex.bump(consumed);
        return FilterResult::Emit(value);
    }
    let space = rest.len() - rest.trim_start_matches(is_js_space).len();
    if !rest[space..].starts_with('{') {
        return FilterResult::Error(LexError::ExpectedDescrForm);
    }
    let body = &rest[space + 1..];
    let Some(close) = body.find('}') else {
        return FilterResult::Error(LexError::UnclosedBlock);
    };
    lex.bump(space + 1 + close + 1);
    FilterResult::Emit(descr_block_text(&body[..close]))
}

fn string(lex: &mut Lexer<Token>) -> String {
    string_text(lex.slice())
}

fn number(lex: &mut Lexer<Token>) -> FilterResult<f64, LexError> {
    let text = lex.slice();
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let has_fraction = unsigned.contains('.');
    let integer = unsigned.split('.').next().unwrap_or(unsigned);
    // `INT` is `0` or a non-zero digit followed by digits; `007` is `0` and then `07`.
    if !has_fraction && integer.len() > 1 && integer.starts_with('0') {
        return FilterResult::Error(LexError::MalformedNumber);
    }
    match text.parse::<f64>() {
        Ok(value) => FilterResult::Emit(value),
        Err(_) => FilterResult::Error(LexError::MalformedNumber),
    }
}

/// `(?:\r?\n|(?!\S))` after a closing fence: the number of bytes a closing fence takes after it.
fn fence_boundary(tail: &str) -> Option<usize> {
    if tail.starts_with("\r\n") {
        return Some(2);
    }
    if tail.starts_with('\n') {
        return Some(1);
    }
    match tail.chars().next() {
        None => Some(0),
        Some(c) if is_js_space(c) => Some(0),
        Some(_) => None,
    }
}

/// `%%{[\S\s]*?}%%(?:\r?\n|(?!\S))` where `rest` starts after `%%`: the length of the directive,
/// or `None` if it has no closing fence that is followed by a boundary.
fn directive_len(rest: &str) -> Option<usize> {
    let mut from = 1;
    while let Some(close) = rest[from..].find("}%%").map(|k| from + k) {
        let end = close + 3;
        if let Some(boundary) = fence_boundary(&rest[end..]) {
            return Some(end + boundary);
        }
        from = close + 1;
    }
    None
}

/// A comment or directive, after the `%%` that logos matched. Directives win over comments.
fn comment(lex: &mut Lexer<Token>) -> FilterResult<(), LexError> {
    let rest = lex.remainder();
    let consumed = rest
        .starts_with('{')
        .then(|| directive_len(rest))
        .flatten()
        .unwrap_or_else(|| line_len(rest));
    lex.bump(consumed);
    FilterResult::Skip
}

/// The length of a `---` block as `YAML` matches it, after the opening `---`.
fn yaml_len(rest: &str) -> Option<usize> {
    let blanks = rest.len() - rest.trim_start_matches(['\t', ' ']).len();
    let body_start = if rest[blanks..].starts_with("\r\n") {
        blanks + 2
    } else if rest[blanks..].starts_with('\n') {
        blanks + 1
    } else {
        return None;
    };

    // `(?:[\S\s]*?\r?\n)?---`: the closing fence follows a line break in the body, the lazy body
    // takes the first one that works, and the empty body comes last.
    let mut from = body_start;
    while let Some(newline) = rest[from..].find('\n').map(|k| from + k) {
        let fence = newline + 1;
        if rest[fence..].starts_with("---")
            && let Some(boundary) = fence_boundary(&rest[fence + 3..])
        {
            return Some(fence + 3 + boundary);
        }
        from = newline + 1;
    }
    if rest[body_start..].starts_with("---")
        && let Some(boundary) = fence_boundary(&rest[body_start + 3..])
    {
        return Some(body_start + 3 + boundary);
    }
    None
}

fn yaml(lex: &mut Lexer<Token>) -> FilterResult<(), LexError> {
    match yaml_len(lex.remainder()) {
        Some(consumed) => {
            lex.bump(consumed);
            FilterResult::Skip
        }
        None => FilterResult::Error(LexError::Unexpected),
    }
}

/// Tokenises the prepared text of `source`, without hidden tokens.
pub(crate) fn lex(source: &Source) -> Result<Vec<Spanned>, ParseError> {
    let mut tokens = Vec::new();
    for (result, span) in Token::lexer(source.text()).spanned() {
        match result {
            Ok(token) => tokens.push(Spanned {
                token,
                start: span.start,
            }),
            Err(error) => return Err(source.error(span.start, error.message())),
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<Token> {
        let source = Source::new(text, 0);
        lex(&source)
            .expect("lexes")
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn title_reads_to_the_end_of_the_line_or_a_comment() {
        assert_eq!(kinds("title a  b %% c"), vec![Token::Title("a b".into())]);
    }

    #[test]
    fn hidden_comments_and_directives_are_skipped() {
        // The directive's closing fence takes the line break after it, the comment's stays.
        assert_eq!(
            kinds("%%{init: {}}%%\n%% note\n\"a\": 1"),
            vec![
                Token::Newline,
                Token::Str("a".into()),
                Token::Colon,
                Token::Number(1.0),
            ]
        );
    }

    #[test]
    fn numbers_follow_the_grammar() {
        assert_eq!(kinds("-0")[0], Token::Number(-0.0));
        assert!(lex(&Source::new("007", 0)).is_err());
        assert!(lex(&Source::new("1.", 0)).is_err());
    }
}
