//! The event modeling scanner: the tokens of `event-modeling.langium`, tried in the order Langium
//! builds them. The first rule that matches wins, not the longest one (decision 0002), so
//! `title` at the start of a line is an identifier, and `title` after a token is the title token.
//! `docs-port/eventmodeling/NOTES.md`, section 2.2, gives the order and its consequences.
//!
//! Each terminal is matched by hand, as its JavaScript regular expression reads when it is run at
//! the current position. The comments name the expression each function follows.

use crate::error::{ParseError, Source};
use crate::text::{is_cr_lf, is_js_space, is_line_term, is_word};

/// The keywords and punctuation of the grammar. Keywords are case-sensitive; a keyword made of
/// letters is a token only when no identifier character follows it (Langium's `LONGER_ALT`).
pub(crate) const KEYWORDS: &[&str] = &[
    "eventmodeling",
    "resetframe",
    "readmodel",
    "processor",
    "timeframe",
    "command",
    "entity",
    "event",
    "jsobj",
    "figma",
    "given",
    "json",
    "salt",
    "html",
    "text",
    "data",
    "note",
    "when",
    "then",
    "rmo",
    "cmd",
    "evt",
    "pcr",
    "uri",
    "gwt",
    "ui",
    "md",
    "tf",
    "rf",
    "[[",
    "]]",
    "`",
    ".",
    "->>",
];

/// The tokens the grammar reads. The keywords and the punctuation are [`Token::Keyword`], with the
/// text they were written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// A keyword or a punctuation mark, as written.
    Keyword(&'static str),
    /// `EM_ID`: a name, one part of a qualified name, or a data block's name.
    Id(String),
    /// `EM_FID`: a frame number, one to three digits.
    Fid(String),
    /// `EM_DATA_INLINE`: the inline data, braces or quotes included.
    DataInline(String),
    /// `EM_DATA_BLOCK`: a data block, from its opening brace to its closing brace.
    DataBlock(String),
    /// `EM_ACC_DESCR`: the accessibility description token, raw, with its leading blanks.
    AccDescr(String),
    /// `EM_ACC_TITLE`: the accessibility title token, raw, with its leading blanks.
    AccTitle(String),
    /// `EM_TITLE`: the title token, raw, with its leading blanks.
    Title(String),
    /// The end of the text.
    Eof,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// The result of matching one rule: a token of some length, or hidden text of some length.
enum Step {
    Emit(Token, usize),
    Hidden(usize),
}

/// Reads the whole text. The result ends with [`Token::Eof`]. A character that no rule takes is a
/// lexer error, at its position.
pub(crate) fn scan(text: &str, source: Source<'_>) -> Result<Vec<Spanned>, ParseError> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let (length, token) = match step(rest) {
            Some(Step::Emit(token, length)) => (length, Some(token)),
            Some(Step::Hidden(length)) => (length, None),
            None => return Err(source.error(pos, "unexpected character")),
        };
        if length == 0 {
            return Err(source.error(pos, "unexpected character"));
        }
        if let Some(token) = token {
            tokens.push(Spanned { token, start: pos });
        }
        pos += length;
    }
    tokens.push(Spanned {
        token: Token::Eof,
        start: text.len(),
    });
    Ok(tokens)
}

/// The rules in Langium's order: the keywords, the identifiers and the numbers, the data, the
/// header texts, the white space, and the hidden comments and front matter.
fn step(rest: &str) -> Option<Step> {
    if let Some(keyword) = keyword_at(rest) {
        return Some(Step::Emit(Token::Keyword(keyword), keyword.len()));
    }
    let id = ident_len(rest);
    if id > 0 {
        return Some(Step::Emit(Token::Id(rest[..id].to_owned()), id));
    }
    if let Some(len) = fid_len(rest) {
        return Some(Step::Emit(Token::Fid(rest[..len].to_owned()), len));
    }
    if let Some(len) = data_inline_len(rest) {
        return Some(Step::Emit(Token::DataInline(rest[..len].to_owned()), len));
    }
    if let Some(len) = line_fenced_len(rest, "{", "}") {
        return Some(Step::Emit(Token::DataBlock(rest[..len].to_owned()), len));
    }
    if let Some(len) = acc_descr_len(rest) {
        return Some(Step::Emit(Token::AccDescr(rest[..len].to_owned()), len));
    }
    if let Some(len) = acc_title_len(rest) {
        return Some(Step::Emit(Token::AccTitle(rest[..len].to_owned()), len));
    }
    if let Some(len) = title_len(rest) {
        return Some(Step::Emit(Token::Title(rest[..len].to_owned()), len));
    }
    let white = js_space_len(rest);
    if white > 0 {
        return Some(Step::Hidden(white));
    }
    if let Some(len) = line_fenced_len(rest, "---", "---") {
        return Some(Step::Hidden(len));
    }
    if let Some(len) = directive_len(rest) {
        return Some(Step::Hidden(len));
    }
    if let Some(len) = single_line_comment_len(rest) {
        return Some(Step::Hidden(len));
    }
    if let Some(len) = multi_line_comment_len(rest) {
        return Some(Step::Hidden(len));
    }
    if rest.starts_with("//") {
        return Some(Step::Hidden(line_rest_len(rest)));
    }
    None
}

/// The first keyword that `rest` starts with, when it is a token here. A word keyword is only a
/// keyword when the identifier characters that follow it are none; `eventmodeling` also needs
/// `%%`, white space or the end after it.
fn keyword_at(rest: &str) -> Option<&'static str> {
    KEYWORDS.iter().copied().find(|&keyword| {
        if !rest.starts_with(keyword) {
            return false;
        }
        if keyword.as_bytes()[0].is_ascii_alphabetic() && ident_len(rest) != keyword.len() {
            return false;
        }
        if keyword == "eventmodeling" {
            let after = &rest[keyword.len()..];
            return after.is_empty() || after.starts_with("%%") || after.starts_with(is_js_space);
        }
        true
    })
}

/// `EM_ID`, `/[_a-zA-Z][\w_]*/`: the byte length of the identifier at the start, zero if none.
fn ident_len(rest: &str) -> usize {
    let mut chars = rest.char_indices();
    match chars.next() {
        Some((_, first)) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return 0,
    }
    chars
        .find(|&(_, c)| !is_word(c))
        .map_or(rest.len(), |(index, _)| index)
}

/// `EM_FID`, `/\d{1,3}/`: one to three ASCII digits at the start.
fn fid_len(rest: &str) -> Option<usize> {
    let digits = rest.bytes().take(3).take_while(u8::is_ascii_digit).count();
    (digits > 0).then_some(digits)
}

/// `EM_DATA_INLINE`, `/\{(.*)\}|"(.*)"|'(.*)'/`: the opening character, then the text up to the
/// last matching closing character on the same line. A `{` with no `}` on its line matches
/// nothing, because the other two alternatives need a quote.
fn data_inline_len(rest: &str) -> Option<usize> {
    let close = match rest.as_bytes().first()? {
        b'{' => b'}',
        b'"' => b'"',
        b'\'' => b'\'',
        _ => return None,
    };
    let line = &rest[..line_dot_len(rest)];
    let found = line.as_bytes()[1..].iter().rposition(|&b| b == close)?;
    Some(1 + found + 1)
}

/// The body of a block whose closing mark starts a line. It follows
/// `open[\t ]*\r?\n(?:[\S\s]*?\r?\n)?close(?:\r?\n|(?!\S))`, which `EM_DATA_BLOCK` and `EM_YAML`
/// are. The lazy body ends at the first closing mark that starts a line and is followed by a
/// line break, or by the end or white space; when no such mark follows, the closing mark may come
/// right after the opening line.
fn line_fenced_len(rest: &str, open: &str, close: &str) -> Option<usize> {
    let after_open = rest.strip_prefix(open)?;
    let spaces = tab_space_len(after_open);
    let newline = newline_len(&after_open[spaces..])?;
    let body = open.len() + spaces + newline;
    let tail_text = &rest[body..];
    for (index, _) in tail_text.match_indices(close) {
        // The body ends with a line break: the byte before the closing mark is `\n`, and it is
        // part of the body, so it must come at or after the body's start.
        if index >= 1 && rest.as_bytes()[body + index - 1] == b'\n' {
            let end = body + index + close.len();
            if let Some(tail) = fence_tail_len(&rest[end..]) {
                return Some(end + tail);
            }
        }
    }
    if tail_text.starts_with(close) {
        let end = body + close.len();
        return fence_tail_len(&rest[end..]).map(|tail| end + tail);
    }
    None
}

/// `EM_DIRECTIVE`, `/[\t ]*%%{[\S\s]*?}%%(?:\r?\n|(?!\S))/`.
fn directive_len(rest: &str) -> Option<usize> {
    let spaces = tab_space_len(rest);
    let after = rest[spaces..].strip_prefix("%%{")?;
    let body = spaces + 3;
    for (index, _) in after.match_indices("}%%") {
        let end = body + index + 3;
        if let Some(tail) = fence_tail_len(&rest[end..]) {
            return Some(end + tail);
        }
    }
    None
}

/// The text after a closing mark: a line break, which is consumed, or nothing when the next
/// character is white space or the end. `None` when a non-space character follows.
fn fence_tail_len(after: &str) -> Option<usize> {
    if after.starts_with("\r\n") {
        Some(2)
    } else if after.starts_with('\n') {
        Some(1)
    } else if after.is_empty() || after.starts_with(is_js_space) {
        Some(0)
    } else {
        None
    }
}

/// `\r?\n` at the start of `text`: its byte length.
fn newline_len(text: &str) -> Option<usize> {
    if text.starts_with("\r\n") {
        Some(2)
    } else if text.starts_with('\n') {
        Some(1)
    } else {
        None
    }
}

/// `[\t ]*` at the start of `text`: its byte length.
fn tab_space_len(text: &str) -> usize {
    text.len() - text.trim_start_matches(['\t', ' ']).len()
}

/// `\s+` at the start of `text`, or `0`: its byte length.
fn js_space_len(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// `[^\n\r]*` at the start of `text`: its byte length.
fn line_rest_len(text: &str) -> usize {
    text.find(is_cr_lf).unwrap_or(text.len())
}

/// `.*` in JavaScript, which stops before any line terminator: its byte length.
fn line_dot_len(text: &str) -> usize {
    text.find(is_line_term).unwrap_or(text.len())
}

/// The value of an accessibility rule after its keyword, which follows
/// `[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)`: the text up to the first `%%` on the line, or to the end
/// of the line. Returns the byte length from the start of `after`.
fn colon_value_len(after: &str) -> Option<usize> {
    let spaces = tab_space_len(after);
    let value = after[spaces..].strip_prefix(':')?;
    let line = &value[..line_rest_len(value)];
    let length = line.find("%%").unwrap_or(line.len());
    Some(spaces + 1 + length)
}

/// `EM_ACC_DESCR`:
/// `[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)|[^\n\r]*)|\s*{([^}]*)})`. The colon form is tried
/// first; the block form takes the text up to the first `}`, line breaks included.
fn acc_descr_len(rest: &str) -> Option<usize> {
    let spaces = tab_space_len(rest);
    let after = rest[spaces..].strip_prefix("accDescr")?;
    let head = spaces + "accDescr".len();
    if let Some(length) = colon_value_len(after) {
        return Some(head + length);
    }
    let white = js_space_len(after);
    let braced = after[white..].strip_prefix('{')?;
    let close = braced.find('}')?;
    Some(head + white + 1 + close + 1)
}

/// `EM_ACC_TITLE`: `[\t ]*accTitle[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)`.
fn acc_title_len(rest: &str) -> Option<usize> {
    let spaces = tab_space_len(rest);
    let after = rest[spaces..].strip_prefix("accTitle")?;
    colon_value_len(after).map(|length| spaces + "accTitle".len() + length)
}

/// `EM_TITLE`: `[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)`. After the word, a blank starts
/// the value, which runs to the first `%%` on the line or to the end of the line; without a blank,
/// the token is the word alone.
fn title_len(rest: &str) -> Option<usize> {
    let spaces = tab_space_len(rest);
    let after = rest[spaces..].strip_prefix("title")?;
    let head = spaces + "title".len();
    match after.as_bytes().first() {
        Some(b' ' | b'\t') => {
            let value = &after[1..];
            let line = &value[..line_rest_len(value)];
            let length = line.find("%%").unwrap_or(line.len());
            Some(head + 1 + length)
        }
        _ => Some(head),
    }
}

/// `EM_SINGLE_LINE_COMMENT`: `[\t ]*%%[^\n\r]*`.
fn single_line_comment_len(rest: &str) -> Option<usize> {
    let spaces = tab_space_len(rest);
    let after = rest[spaces..].strip_prefix("%%")?;
    Some(spaces + 2 + line_rest_len(after))
}

/// `EM_ML_COMMENT`: `/\*[\s\S]*?\*\//`, up to the first `*/` after the opening.
fn multi_line_comment_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix("/*")?;
    let close = after.find("*/")?;
    Some(2 + close + 2)
}
