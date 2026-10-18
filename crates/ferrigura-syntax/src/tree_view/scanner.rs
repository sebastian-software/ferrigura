//! The lexer of the treeView grammar, `packages/parser/src/language/treeView/treeView.langium`.
//!
//! Langium builds the lexer in this order, which the reference's token builder produces
//! (`docs-port/tree-view/NOTES.md`, section 2.2):
//!
//! 1. the keyword `treeView-beta`, with the word-end suffix `(?:(?=%%)|(?!\S))`; its `LONGER_ALT`
//!    is `BARE_NAME`, so a longer word that starts with the keyword is a name;
//! 2. the terminals in declaration order: `ACC_DESCR`, `ACC_TITLE`, `TITLE`, `CLASS_ANNOTATION`,
//!    `ICON_ANNOTATION`, `DESC_ANNOTATION`, `INDENTATION`, `QUOTED_NAME`, `WS` (never matches, since
//!    `INDENTATION` takes every blank first), `ML_COMMENT`, `NL`, `BARE_NAME`.
//!
//! Chevrotain takes the first pattern that matches at the position, not the longest, except for the
//! keyword's `LONGER_ALT`. The hidden terminals (`WS`, `ML_COMMENT`, `NL`) are dropped here, since
//! the parser never sees them. Each terminal's regex is one function below.

use crate::common::{descr_block_text, plain_text};
use crate::text::{is_js_space, js_trim};

/// The keyword of the diagram.
const KEYWORD: &str = "treeView-beta";

/// A token of the grammar, with its value converted as the reference's value converter does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `treeView-beta`.
    Keyword,
    /// `ACC_DESCR`, the value after the common converter.
    AccDescr(String),
    /// `ACC_TITLE`, the value after the common converter.
    AccTitle(String),
    /// `TITLE`, the value after the common converter; empty for `title` alone.
    Title(String),
    /// `CLASS_ANNOTATION`: the class name.
    ClassAnnotation(String),
    /// `ICON_ANNOTATION`: the text between `icon(` and `)`, empty for `icon()`.
    IconAnnotation(String),
    /// `DESC_ANNOTATION`: the trimmed text after `##`.
    DescAnnotation(String),
    /// `INDENTATION`: the number of characters of indentation.
    Indentation(usize),
    /// `QUOTED_NAME`: the name between its quotes, which may span lines.
    QuotedName(String),
    /// `BARE_NAME`: the name without its trailing blanks.
    BareName(String),
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// A piece of the text that no terminal matches, at a byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexFailure {
    pub offset: usize,
}

/// What the lexer does at one position: emit a token of this many bytes, or skip hidden text.
enum Step {
    Emit(Token, usize),
    Hidden(usize),
}

/// Tokenises the prepared text of a treeView diagram, without the hidden tokens.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Spanned>, LexFailure> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        match next_step(&text[pos..]) {
            Some(Step::Emit(token, len)) if len > 0 => {
                tokens.push(Spanned { token, start: pos });
                pos += len;
            }
            Some(Step::Hidden(len)) if len > 0 => pos += len,
            _ => return Err(LexFailure { offset: pos }),
        }
    }
    Ok(tokens)
}

/// The first terminal that matches at the start of `rest`, in the lexer's order.
fn next_step(rest: &str) -> Option<Step> {
    if let Some(len) = keyword_len(rest) {
        // The keyword's LONGER_ALT: a longer `BARE_NAME` at the same position wins.
        return Some(match bare_name_len(rest) {
            Some(longer) if longer > len => {
                Step::Emit(Token::BareName(bare_name_text(&rest[..longer])), longer)
            }
            _ => Step::Emit(Token::Keyword, len),
        });
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
    if let Some((value, len)) = class_at(rest) {
        return Some(Step::Emit(Token::ClassAnnotation(value), len));
    }
    if let Some((value, len)) = icon_at(rest) {
        return Some(Step::Emit(Token::IconAnnotation(value), len));
    }
    if let Some((value, len)) = desc_at(rest) {
        return Some(Step::Emit(Token::DescAnnotation(value), len));
    }
    let blanks = blank_len(rest);
    if blanks > 0 {
        return Some(Step::Emit(Token::Indentation(blanks), blanks));
    }
    if let Some(len) = quoted_len(rest) {
        let name = rest[1..len - 1].to_owned();
        return Some(Step::Emit(Token::QuotedName(name), len));
    }
    // ML_COMMENT: `%%` and the rest of the line, hidden.
    if rest.starts_with("%%") {
        return Some(Step::Hidden(line_len(rest)));
    }
    // NL: `\r?\n`, hidden.
    if let Some(len) = newline_len(rest) {
        return Some(Step::Hidden(len));
    }
    let len = bare_name_len(rest)?;
    Some(Step::Emit(
        Token::BareName(bare_name_text(&rest[..len])),
        len,
    ))
}

/// `treeView-beta` followed by `%%`, by a space or by the end of the text: its byte length.
fn keyword_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix(KEYWORD)?;
    let ends_word = after.is_empty()
        || after.starts_with("%%")
        || after.chars().next().is_some_and(is_js_space);
    ends_word.then_some(KEYWORD.len())
}

/// The number of leading spaces and tabs, `[ \t]*`, in bytes.
fn blank_len(rest: &str) -> usize {
    rest.bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count()
}

/// The length of the first line terminator run that `NL` takes: `\r?\n`.
fn newline_len(rest: &str) -> Option<usize> {
    if rest.starts_with("\r\n") {
        Some(2)
    } else if rest.starts_with('\n') {
        Some(1)
    } else {
        None
    }
}

/// Bytes up to, not including, the next `\n` (`[^\n]*`).
fn line_len(rest: &str) -> usize {
    rest.find('\n').unwrap_or(rest.len())
}

/// Bytes up to, not including, the next `\n` or `\r` (`[^\n\r]*`).
fn text_line_len(rest: &str) -> usize {
    rest.find(['\n', '\r']).unwrap_or(rest.len())
}

/// `"[^"]*"|'[^']*'` at the start of `rest`, in bytes including the quotes. The body may hold line
/// breaks, as the regex's negated class does.
fn quoted_len(rest: &str) -> Option<usize> {
    let quote = match rest.as_bytes().first()? {
        b'"' => '"',
        b'\'' => '\'',
        _ => return None,
    };
    let close = rest[1..].find(quote)?;
    Some(1 + close + 1)
}

/// `ACC_DESCR: /[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)|[^\n\r]*)|\s*{([^}]*)})/`: the colon form as
/// for `accTitle`, or a block in braces that may run over several lines.
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

/// The `[\t ]*:` form of `accTitle` and `accDescr`, with the rest of the line up to a `%%` on it.
/// Returns the value (trimmed, with its blank runs collapsed) and the bytes consumed.
fn colon_form(rest: &str) -> Option<(String, usize)> {
    let blanks = blank_len(rest);
    let after = rest[blanks..].strip_prefix(':')?;
    let line = &after[..text_line_len(after)];
    let consumed = blanks + 1 + line.find("%%").unwrap_or(line.len());
    Some((plain_text(&rest[blanks + 1..consumed]), consumed))
}

/// `TITLE: /[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)/`: a blank and the rest of the line
/// up to a `%%` on it, or the rest of the line, or nothing when no blank follows the keyword.
fn title_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    let after = rest[lead..].strip_prefix("title")?;
    let line = &after[..text_line_len(after)];
    let consumed = match line.as_bytes().first() {
        Some(b' ' | b'\t') => 1 + line[1..].find("%%").unwrap_or(line.len() - 1),
        _ => 0,
    };
    Some((
        plain_text(&after[..consumed]),
        lead + "title".len() + consumed,
    ))
}

/// `CLASS_ANNOTATION: /[ \t]+:::[ \t]*[A-Za-z_][\w-]*/`, the value being the class name.
fn class_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    if lead == 0 {
        return None;
    }
    let after = rest[lead..].strip_prefix(":::")?;
    let name_start = after.trim_start_matches([' ', '\t']);
    let first = name_start.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let name_len = name_start
        .bytes()
        .take_while(|&b| is_word_or_dash(b))
        .count();
    let total = lead + 3 + (after.len() - name_start.len()) + name_len;
    Some((name_start[..name_len].to_owned(), total))
}

/// `ICON_ANNOTATION: /[ \t]+icon\([\w-]*(?::[\w-]+)?\)/`, the value being the text between the
/// parentheses. A colon must be followed by a name, and the name must be closed by `)`.
fn icon_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    if lead == 0 {
        return None;
    }
    let after = rest[lead..].strip_prefix("icon(")?;
    let bytes = after.as_bytes();
    let mut len = bytes.iter().take_while(|&&b| is_word_or_dash(b)).count();
    if bytes.get(len) == Some(&b':') {
        let run = bytes[len + 1..]
            .iter()
            .take_while(|&&b| is_word_or_dash(b))
            .count();
        if run == 0 {
            return None;
        }
        len += 1 + run;
    }
    if bytes.get(len) != Some(&b')') {
        return None;
    }
    Some((after[..len].to_owned(), lead + "icon(".len() + len + 1))
}

/// `DESC_ANNOTATION: /[ \t]+##[^\n\r]*/`, the value being the trimmed text after `##`.
fn desc_at(rest: &str) -> Option<(String, usize)> {
    let lead = blank_len(rest);
    if lead == 0 {
        return None;
    }
    let after = rest[lead..].strip_prefix("##")?;
    let line = text_line_len(after);
    Some((js_trim(&after[..line]).to_owned(), lead + 2 + line))
}

/// `BARE_NAME: /(?!:::|icon\(|##)[^ \t\n\r"'](?:(?![ \t]+:::[ \t]*[A-Za-z_]|[ \t]+icon\(|[ \t]+##)[^\n\r])*/`.
/// The first character is not a blank, a line break or a quote, and the name does not start with
/// `:::`, `icon(` or `##`. It runs to the end of the line, or to the first blank run that starts a
/// class, icon or description annotation. Returns the length in bytes.
fn bare_name_len(rest: &str) -> Option<usize> {
    let first = rest.chars().next()?;
    if matches!(first, ' ' | '\t' | '\n' | '\r' | '"' | '\'')
        || rest.starts_with(":::")
        || rest.starts_with("icon(")
        || rest.starts_with("##")
    {
        return None;
    }
    let mut end = first.len_utf8();
    while let Some(c) = rest[end..].chars().next() {
        if c == '\n' || c == '\r' {
            break;
        }
        if c == ' ' || c == '\t' {
            // Every position of a blank run sees the same text after it, so the run is tested once.
            if starts_annotation_at(rest, end) {
                break;
            }
            end += blank_len(&rest[end..]);
        } else {
            end += c.len_utf8();
        }
    }
    Some(end)
}

/// Whether a blank run at `pos` starts an annotation that `BARE_NAME` must stop before: a class
/// name, `icon(` or `##`. Only a run of spaces and tabs can start one.
fn starts_annotation_at(rest: &str, pos: usize) -> bool {
    let blanks = blank_len(&rest[pos..]);
    if blanks == 0 {
        return false;
    }
    let after = &rest[pos + blanks..];
    if after.starts_with("##") || after.starts_with("icon(") {
        return true;
    }
    after.strip_prefix(":::").is_some_and(|tail| {
        tail.trim_start_matches([' ', '\t'])
            .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
    })
}

/// The value of `BARE_NAME`: its trailing spaces and tabs removed.
fn bare_name_text(token: &str) -> String {
    token.trim_end_matches([' ', '\t']).to_owned()
}

/// `\w` or `-` in JavaScript's ASCII sense.
fn is_word_or_dash(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<Token> {
        tokenize(text)
            .expect("lexes")
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn the_keyword_gives_way_to_a_longer_word() {
        assert_eq!(kinds("treeView-beta")[0], Token::Keyword);
        assert_eq!(kinds("treeView-beta\na")[0], Token::Keyword);
        assert_eq!(
            kinds("treeView-beta foo"),
            vec![Token::BareName("treeView-beta foo".into())]
        );
        assert_eq!(
            kinds("treeView-betaX"),
            vec![Token::BareName("treeView-betaX".into())]
        );
    }

    #[test]
    fn a_blank_run_is_an_indentation_unless_an_annotation_starts() {
        assert_eq!(
            kinds("a :::b"),
            vec![
                Token::BareName("a".into()),
                Token::ClassAnnotation("b".into())
            ]
        );
        assert_eq!(kinds("a  c"), vec![Token::BareName("a  c".into())]);
    }
}
