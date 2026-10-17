//! The lexer of the treemap grammar, `packages/parser/src/language/treemap/treemap.langium`.
//!
//! Chevrotain takes the first terminal that matches at a position, in the order of the terminals,
//! and not the longest one. The terminals that no rule uses are not in the lexer at all, so
//! `BOOLEAN` is absent. The order is the one of the grammar: the accessibility and title
//! terminals (which begin with the blanks before them), the keyword, `classDef`, the separators,
//! the blanks, the comments and the line breaks, the identifiers, the numbers and the strings. The
//! quirks that follow from it are in `docs-port/treemap/PLAN.md`, and the terminals in
//! `docs-port/treemap/NOTES.md` section 2.2.

use crate::common::{descr_block_text, plain_text};
use crate::lexer::colon_form;
use crate::text::is_js_space;

/// A token of the treemap grammar. Comments and line breaks are hidden, so they are not tokens.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `TREEMAP_KEYWORD`: `treemap-beta` or `treemap`.
    Keyword,
    /// `TITLE`, the value after the converter.
    Title(String),
    /// `ACC_TITLE`, the value after the converter.
    AccTitle(String),
    /// `ACC_DESCR`, the value after the converter.
    AccDescr(String),
    /// `CLASS_DEF`: the class name, and the style text (empty when none is given).
    ClassDef { name: String, style: String },
    /// `STYLE_SEPARATOR`: `:::`.
    StyleSeparator,
    /// `SEPARATOR`: `:`.
    Separator,
    /// `COMMA`: `,`.
    Comma,
    /// `INDENTATION`: a run of blanks, counted in characters.
    Indentation(usize),
    /// `ID2`: a class name.
    Id(String),
    /// `NUMBER2`: the value, as `parseFloat` reads the text after its commas are removed.
    Number(f64),
    /// `STRING2`: the text between the quotes, with no escapes resolved.
    Str(String),
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// A piece of the text that no terminal matches, with the byte offset where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexFailure {
    pub offset: usize,
    pub message: String,
}

/// One step of the lexer: a token of a length, or a hidden piece of a length.
enum Step {
    Token(Token, usize),
    Hidden(usize),
}

/// Tokenises the text of a treemap, without the hidden tokens.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Spanned>, LexFailure> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        match next_step(rest) {
            Ok(Step::Token(token, len)) => {
                tokens.push(Spanned { token, start: pos });
                pos += len;
            }
            Ok(Step::Hidden(len)) => pos += len,
            Err(message) => {
                return Err(LexFailure {
                    offset: pos,
                    message,
                });
            }
        }
    }
    Ok(tokens)
}

/// The first terminal that matches at the start of `rest`, in the order of the grammar.
fn next_step(rest: &str) -> Result<Step, String> {
    if let Some((token, len)) = acc_descr_at(rest) {
        return Ok(Step::Token(token, len));
    }
    if let Some((token, len)) = acc_title_at(rest) {
        return Ok(Step::Token(token, len));
    }
    if let Some((token, len)) = title_at(rest) {
        return Ok(Step::Token(token, len));
    }
    if let Some(len) = keyword_len(rest) {
        return Ok(Step::Token(Token::Keyword, len));
    }
    if let Some((token, len)) = class_def_at(rest) {
        return Ok(Step::Token(token, len));
    }
    if rest.starts_with(":::") {
        return Ok(Step::Token(Token::StyleSeparator, 3));
    }
    if rest.starts_with(':') {
        return Ok(Step::Token(Token::Separator, 1));
    }
    if rest.starts_with(',') {
        return Ok(Step::Token(Token::Comma, 1));
    }
    let blanks = blanks_len(rest);
    if blanks > 0 {
        return Ok(Step::Token(Token::Indentation(blanks), blanks));
    }
    // ML_COMMENT: `%%[^\n]*`, and NL: `\r?\n`. The text has no carriage return, since the
    // preprocessing turns each one into a line break.
    if rest.starts_with("%%") {
        return Ok(Step::Hidden(rest.find('\n').unwrap_or(rest.len())));
    }
    if rest.starts_with('\n') {
        return Ok(Step::Hidden(1));
    }
    if let Some(len) = id_len(rest) {
        let id = rest[..len].to_owned();
        return Ok(Step::Token(Token::Id(id), len));
    }
    let number_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_digit() || matches!(b, b'_' | b'.' | b','))
        .count();
    if number_len > 0 {
        let value = parse_number(&rest[..number_len]);
        return Ok(Step::Token(Token::Number(value), number_len));
    }
    if let Some(len) = string_len(rest) {
        let text = rest[1..len - 1].to_owned();
        return Ok(Step::Token(Token::Str(text), len));
    }
    match rest.chars().next() {
        Some(c) => Err(format!("unexpected character {c:?}")),
        None => Err("unexpected end of the text".to_owned()),
    }
}

/// `[\t ]*`, as a count of bytes (both characters are one byte).
fn blanks_len(rest: &str) -> usize {
    rest.len() - rest.trim_start_matches(['\t', ' ']).len()
}

/// `ACC_DESCR: /[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)|[^\n\r]*)|\s*{([^}]*)})/`.
fn acc_descr_at(rest: &str) -> Option<(Token, usize)> {
    let blanks = blanks_len(rest);
    let body = rest[blanks..].strip_prefix("accDescr")?;
    let keyword = blanks + "accDescr".len();
    if let Some((value, consumed)) = colon_form(body) {
        return Some((Token::AccDescr(value), keyword + consumed));
    }
    let space = body.len() - body.trim_start_matches(is_js_space).len();
    let block = body[space..].strip_prefix('{')?;
    let close = block.find('}')?;
    let len = keyword + space + 1 + close + 1;
    Some((Token::AccDescr(descr_block_text(&block[..close])), len))
}

/// `ACC_TITLE: /[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)/`.
fn acc_title_at(rest: &str) -> Option<(Token, usize)> {
    let blanks = blanks_len(rest);
    let body = rest[blanks..].strip_prefix("accTitle")?;
    let (value, consumed) = colon_form(body)?;
    Some((Token::AccTitle(value), blanks + "accTitle".len() + consumed))
}

/// `TITLE: /[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)/`. The title is the blank and the
/// rest of the line, up to a `%%` on that line; without a blank after the keyword, it is the
/// keyword alone.
fn title_at(rest: &str) -> Option<(Token, usize)> {
    let blanks = blanks_len(rest);
    let body = rest[blanks..].strip_prefix("title")?;
    let line = &body[..body.find(['\n', '\r']).unwrap_or(body.len())];
    let consumed = match line.as_bytes().first() {
        Some(b'\t' | b' ') => 1 + line[1..].find("%%").unwrap_or(line.len() - 1),
        _ => 0,
    };
    let len = blanks + "title".len() + consumed;
    Some((Token::Title(plain_text(&body[..consumed])), len))
}

/// `TREEMAP_KEYWORD: 'treemap-beta' | 'treemap'`.
fn keyword_len(rest: &str) -> Option<usize> {
    if rest.starts_with("treemap-beta") {
        Some("treemap-beta".len())
    } else if rest.starts_with("treemap") {
        Some("treemap".len())
    } else {
        None
    }
}

/// `CLASS_DEF: /classDef\s+([a-zA-Z_][a-zA-Z0-9_]+)(?:\s+([^;\r\n]*))?(?:;)?/`. The `\s` of the
/// pattern is JavaScript's, and it crosses line breaks, so the style text may be on the next line.
fn class_def_at(rest: &str) -> Option<(Token, usize)> {
    let body = rest.strip_prefix("classDef")?;
    let space = body.len() - body.trim_start_matches(is_js_space).len();
    if space == 0 {
        return None;
    }
    let after_space = &body[space..];
    let name_len = after_space
        .bytes()
        .enumerate()
        .take_while(|&(index, b)| {
            b.is_ascii_alphabetic() || b == b'_' || (index > 0 && b.is_ascii_digit())
        })
        .count();
    if name_len < 2 {
        return None;
    }
    let name = after_space[..name_len].to_owned();
    let mut len = "classDef".len() + space + name_len;
    let tail = &after_space[name_len..];
    let mut style = String::new();
    let tail_space = tail.len() - tail.trim_start_matches(is_js_space).len();
    if tail_space > 0 {
        let styled = &tail[tail_space..];
        let style_len = styled.find([';', '\r', '\n']).unwrap_or(styled.len());
        style = styled[..style_len].to_owned();
        len += tail_space + style_len;
    }
    if rest[len..].starts_with(';') {
        len += 1;
    }
    Some((Token::ClassDef { name, style }, len))
}

/// `ID2: /[a-zA-Z_][a-zA-Z0-9_]*/`, as a length.
fn id_len(rest: &str) -> Option<usize> {
    let first = rest.bytes().next()?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    Some(
        rest.bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count(),
    )
}

/// `STRING2: /"[^"]*"|'[^']*'/`, as a length including the quotes. The text may cross line breaks.
fn string_len(rest: &str) -> Option<usize> {
    let quote = match rest.bytes().next()? {
        b'"' => '"',
        b'\'' => '\'',
        _ => return None,
    };
    let close = rest[1..].find(quote)?;
    Some(close + 2)
}

/// `NUMBER2`'s converter: `parseFloat(input.replace(/,/g, ''))` for a run of digits, `_`, `.` and
/// commas. `parseFloat` reads the longest prefix that is a decimal literal, so `1_000` is 1,
/// `1..2` is 1, `.5` is 0.5, and a run with no digit at all is NaN.
fn parse_number(run: &str) -> f64 {
    let text: String = run.chars().filter(|&c| c != ',').collect();
    let whole: String = text.chars().take_while(char::is_ascii_digit).collect();
    let after_whole = &text[whole.len()..];
    let fraction: String = match after_whole.strip_prefix('.') {
        Some(rest) => rest.chars().take_while(char::is_ascii_digit).collect(),
        None => String::new(),
    };
    if whole.is_empty() && fraction.is_empty() {
        return f64::NAN;
    }
    let mut literal = if whole.is_empty() {
        "0".to_owned()
    } else {
        whole
    };
    if !fraction.is_empty() {
        literal.push('.');
        literal.push_str(&fraction);
    }
    literal.parse::<f64>().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<Token> {
        tokenize(text)
            .expect("lexes")
            .into_iter()
            .map(|spanned| spanned.token)
            .collect()
    }

    #[test]
    fn reads_a_number_as_parse_float_does() {
        assert_eq!(parse_number("1,000.5"), 1000.5);
        assert_eq!(parse_number("1,2,3"), 123.0);
        assert_eq!(parse_number("1_000"), 1.0);
        assert_eq!(parse_number("1..2"), 1.0);
        assert_eq!(parse_number("1."), 1.0);
        assert_eq!(parse_number(".5"), 0.5);
        assert_eq!(parse_number("007.50"), 7.5);
        assert!(parse_number("._.").is_nan());
        assert!(parse_number(".").is_nan());
    }

    #[test]
    fn takes_the_class_def_over_an_identifier_and_keeps_the_next_line() {
        assert_eq!(
            kinds("classDef cc\n\"A\":::cc"),
            vec![Token::ClassDef {
                name: "cc".to_owned(),
                style: "\"A\":::cc".to_owned()
            }]
        );
        assert!(matches!(kinds("classDef c fill:red")[0], Token::Id(_)));
    }

    #[test]
    fn reads_the_trailing_blanks_as_an_indentation() {
        assert_eq!(
            kinds("\"A\":1 \n"),
            vec![
                Token::Str("A".to_owned()),
                Token::Separator,
                Token::Number(1.0),
                Token::Indentation(1),
            ]
        );
        // The blank after the separator is an indentation too, which the leaf rule takes.
        assert_eq!(kinds("\"A\": 1")[2], Token::Indentation(1));
    }
}
