//! The lexer of the gitGraph grammar. Chevrotain takes the first terminal that matches, in the
//! order of the grammar, and a keyword is overtaken by REFERENCE only when REFERENCE is longer
//! (`longer_alt`). The order and the probes behind each rule are in `docs-port/gitgraph/PLAN.md`
//! section 2 and `docs-port/gitgraph/NOTES.md` section 2.3.

use crate::common::{descr_block_text, plain_text, string_text};
use crate::lexer::{colon_form, directive_len, yaml_len};
use crate::text::{is_js_space, is_line_term};

/// The keywords and punctuation of the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kw {
    /// `gitGraph` as the header: the suffix rule applies.
    GitGraph,
    /// `gitGraph:`, one token in the grammar.
    GitGraphColon,
    Commit,
    Branch,
    Merge,
    Checkout,
    Switch,
    CherryPick,
    /// `id:`
    Id,
    /// `msg:`
    Msg,
    /// `tag:`
    Tag,
    /// `type:`
    Type,
    /// `order:`
    Order,
    /// `parent:`
    Parent,
    Lr,
    Tb,
    Bt,
    Normal,
    Reverse,
    Highlight,
    Colon,
}

/// A token of the gitGraph grammar. Comments, directives and front matter are hidden.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `NEWLINE`: `\r?\n`.
    Newline,
    Keyword(Kw),
    /// `REFERENCE`: a name, `\w([-\./\w]*[-\w])?`.
    Reference(String),
    /// `INT`: the number, as `0|[1-9][0-9]*(?!\.)` gives it.
    Int(f64),
    /// `STRING`, with the quotes removed and the escapes resolved.
    Str(String),
    /// `TITLE`, the value after the converter.
    Title(String),
    /// `ACC_TITLE`, the value after the converter.
    AccTitle(String),
    /// `ACC_DESCR`, the value after the converter.
    AccDescr(String),
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// A piece of the text no terminal matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexFailure {
    pub offset: usize,
    pub message: &'static str,
}

/// The literal keywords, longest pattern first where one is a prefix of another.
const KEYWORDS: &[(&str, Kw)] = &[
    ("gitGraph:", Kw::GitGraphColon),
    ("gitGraph", Kw::GitGraph),
    ("commit", Kw::Commit),
    ("branch", Kw::Branch),
    ("merge", Kw::Merge),
    ("checkout", Kw::Checkout),
    ("switch", Kw::Switch),
    ("cherry-pick", Kw::CherryPick),
    ("id:", Kw::Id),
    ("msg:", Kw::Msg),
    ("tag:", Kw::Tag),
    ("type:", Kw::Type),
    ("order:", Kw::Order),
    ("parent:", Kw::Parent),
    ("LR", Kw::Lr),
    ("TB", Kw::Tb),
    ("BT", Kw::Bt),
    ("NORMAL", Kw::Normal),
    ("REVERSE", Kw::Reverse),
    ("HIGHLIGHT", Kw::Highlight),
    (":", Kw::Colon),
];

/// Tokenises the text of a gitGraph, without the hidden tokens.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Spanned>, LexFailure> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let first = rest.as_bytes()[0];

        // WHITESPACE: `[\t ]+`, hidden.
        if first == b' ' || first == b'\t' {
            pos += rest.len() - rest.trim_start_matches(['\t', ' ']).len();
            continue;
        }
        // NEWLINE: `\r?\n`.
        if first == b'\n' {
            tokens.push(spanned(Token::Newline, pos));
            pos += 1;
            continue;
        }
        if rest.starts_with("\r\n") {
            tokens.push(spanned(Token::Newline, pos));
            pos += 2;
            continue;
        }
        // YAML, DIRECTIVE and SINGLE_LINE_COMMENT: hidden.
        if let Some(len) = hidden_len(rest) {
            pos += len;
            continue;
        }
        if let Some((token, len)) = keyword_or_accessibility(rest) {
            tokens.push(spanned(token, pos));
            pos += len;
            continue;
        }
        if let Some(len) = first.is_ascii_digit().then(|| int_len(rest)).flatten() {
            let value = rest[..len].parse::<f64>().unwrap_or(0.0);
            tokens.push(spanned(Token::Int(value), pos));
            pos += len;
            continue;
        }
        if first == b'"' || first == b'\'' {
            let Some(len) = string_len(rest) else {
                return Err(LexFailure {
                    offset: pos,
                    message: "unterminated string",
                });
            };
            tokens.push(spanned(Token::Str(string_text(&rest[..len])), pos));
            pos += len;
            continue;
        }
        if let Some(len) = reference_len(rest) {
            tokens.push(spanned(Token::Reference(rest[..len].to_owned()), pos));
            pos += len;
            continue;
        }
        return Err(LexFailure {
            offset: pos,
            message: "unexpected character",
        });
    }
    Ok(tokens)
}

fn spanned(token: Token, start: usize) -> Spanned {
    Spanned { token, start }
}

/// The length of a hidden token at the start of `rest`: front matter inside the text, a directive
/// with the line break after its closing fence, or a comment up to the end of the line.
fn hidden_len(rest: &str) -> Option<usize> {
    if let Some(after) = rest.strip_prefix("---") {
        return yaml_len(after).map(|len| 3 + len);
    }
    let after = rest.strip_prefix("%%")?;
    let len = if after.starts_with('{') {
        directive_len(after).unwrap_or_else(|| line_len(after))
    } else {
        line_len(after)
    };
    Some(2 + len)
}

/// Bytes up to, not including, the next line break.
fn line_len(rest: &str) -> usize {
    rest.find(['\n', '\r']).unwrap_or(rest.len())
}

/// A keyword, or the header-line accessibility terminals and `title`. The keyword wins unless
/// REFERENCE matches longer, in which case the word is a name.
fn keyword_or_accessibility(rest: &str) -> Option<(Token, usize)> {
    if let Some((kw, len)) = keyword_at(rest) {
        let reference = reference_len(rest).unwrap_or(0);
        if reference <= len {
            return Some((Token::Keyword(kw), len));
        }
        return None;
    }
    if let Some(found) = accessibility_at(rest) {
        return Some(found);
    }
    if let Some(after) = rest.strip_prefix("title") {
        return Some(title_at(after));
    }
    None
}

/// The keyword that `rest` starts with. `gitGraph` is a keyword only before `%%` or a blank, the
/// `(?:(?=%%)|(?!\S))` suffix of the reference's token builder.
fn keyword_at(rest: &str) -> Option<(Kw, usize)> {
    KEYWORDS.iter().find_map(|&(literal, kw)| {
        let after = rest.strip_prefix(literal)?;
        if kw == Kw::GitGraph
            && !(after.starts_with("%%") || after.chars().next().is_none_or(is_js_space))
        {
            return None;
        }
        Some((kw, literal.len()))
    })
}

/// `ACC_DESCR` and `ACC_TITLE`: the keyword, then the `:` form, and for `accDescr` also a block
/// in braces, which may run over several lines.
fn accessibility_at(rest: &str) -> Option<(Token, usize)> {
    if let Some(after) = rest.strip_prefix("accTitle") {
        let (value, consumed) = colon_form(after)?;
        return Some((Token::AccTitle(value), "accTitle".len() + consumed));
    }
    let after = rest.strip_prefix("accDescr")?;
    if let Some((value, consumed)) = colon_form(after) {
        return Some((Token::AccDescr(value), "accDescr".len() + consumed));
    }
    let space = after.len() - after.trim_start_matches(is_js_space).len();
    let block = after[space..].strip_prefix('{')?;
    let close = block.find('}')?;
    let len = "accDescr".len() + space + 1 + close + 1;
    Some((Token::AccDescr(descr_block_text(&block[..close])), len))
}

/// `TITLE`, after the keyword: a blank and the rest of the line, up to a `%%` on that line.
fn title_at(after: &str) -> (Token, usize) {
    let line = &after[..line_len(after)];
    let consumed = match line.as_bytes().first() {
        Some(b'\t' | b' ') => {
            let body = &line[1..];
            1 + body.find("%%").unwrap_or(body.len())
        }
        _ => 0,
    };
    (
        Token::Title(plain_text(&after[..consumed])),
        "title".len() + consumed,
    )
}

/// `INT: /0|[1-9][0-9]*(?!\.)/`. A run of digits that is followed by `.` gives back its last digit
/// to the lookahead, and a single digit followed by `.` is no number at all.
fn int_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    match bytes.first()? {
        b'0' => Some(1),
        b'1'..=b'9' => {
            let run = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if bytes.get(run) == Some(&b'.') {
                (run >= 2).then_some(run - 1)
            } else {
                Some(run)
            }
        }
        _ => None,
    }
}

/// `STRING: /"([^"\\]|\\.)*"|'([^'\\]|\\.)*'/`. A backslash takes the next character, which may
/// not be a line break, and any other character, a line break included, is copied.
fn string_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let quote = *bytes.first()?;
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b if b == quote => return Some(i + 1),
            b'\\' => {
                let next = rest[i + 1..].chars().next()?;
                if is_line_term(next) {
                    return None;
                }
                i += 1 + next.len_utf8();
            }
            _ => i += 1,
        }
    }
    None
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `REFERENCE: /\w([-\./\w]*[-\w])?/`: the longest prefix of the run of name characters that
/// ends in a word character or a dash. JavaScript's `\w` is ASCII only.
fn reference_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !bytes.first().is_some_and(|&b| is_word_byte(b)) {
        return None;
    }
    let run = bytes
        .iter()
        .take_while(|&&b| is_word_byte(b) || matches!(b, b'-' | b'.' | b'/'))
        .count();
    let len = (1..run)
        .rev()
        .find(|&i| is_word_byte(bytes[i]) || bytes[i] == b'-')
        .map_or(1, |i| i + 1);
    Some(len)
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
    fn a_word_that_is_longer_than_a_keyword_is_a_name() {
        assert_eq!(kinds("commitX"), vec![Token::Reference("commitX".into())]);
        assert_eq!(kinds("commit"), vec![Token::Keyword(Kw::Commit)]);
        assert_eq!(
            kinds("cherry-picks"),
            vec![Token::Reference("cherry-picks".into())]
        );
    }

    #[test]
    fn a_reference_ends_in_a_word_character_or_a_dash() {
        // `a.` is a lexer error: the name stops at `a` and the dot matches nothing.
        assert!(tokenize("a.").is_err());
        assert_eq!(kinds("a-")[0], Token::Reference("a-".into()));
        assert_eq!(
            kinds("a./b.c_d-e")[0],
            Token::Reference("a./b.c_d-e".into())
        );
        assert_eq!(kinds("1.0.1")[0], Token::Reference("1.0.1".into()));
    }

    #[test]
    fn int_wins_over_reference_except_before_a_dot() {
        assert_eq!(kinds("12")[0], Token::Int(12.0));
        assert_eq!(kinds("1.0")[0], Token::Reference("1.0".into()));
        // `10.x`: INT takes `1`, then `0`, and the dot is a lexer error.
        assert_eq!(kinds("10 ")[0], Token::Int(10.0));
        assert!(tokenize("10.x").is_err());
        assert_eq!(
            kinds("0a"),
            vec![Token::Int(0.0), Token::Reference("a".into())]
        );
    }

    #[test]
    fn title_takes_the_rest_of_its_line() {
        assert_eq!(kinds("title a  b %% c"), vec![Token::Title("a b".into())]);
        assert_eq!(kinds("title"), vec![Token::Title(String::new())]);
        assert_eq!(
            kinds("titleX"),
            vec![Token::Title(String::new()), Token::Reference("X".into())]
        );
    }

    #[test]
    fn gitgraph_is_a_keyword_only_before_a_blank_or_a_comment() {
        assert_eq!(kinds("gitGraph")[0], Token::Keyword(Kw::GitGraph));
        assert_eq!(kinds("gitGraph:")[0], Token::Keyword(Kw::GitGraphColon));
        assert_eq!(kinds("gitGraphX")[0], Token::Reference("gitGraphX".into()));
        assert_eq!(kinds("gitGraph%%c")[0], Token::Keyword(Kw::GitGraph));
    }

    #[test]
    fn hidden_tokens_and_newlines() {
        assert_eq!(
            kinds("commit %% note\r\n\n  branch a"),
            vec![
                Token::Keyword(Kw::Commit),
                Token::Newline,
                Token::Newline,
                Token::Keyword(Kw::Branch),
                Token::Reference("a".into()),
            ]
        );
    }

    #[test]
    fn strings_keep_line_breaks_and_resolve_escapes() {
        assert_eq!(kinds(r#""a\"b""#), vec![Token::Str("a\"b".into())]);
        assert_eq!(kinds("'x\ny'"), vec![Token::Str("x\ny".into())]);
        assert!(tokenize("\"open").is_err());
    }

    #[test]
    fn semicolons_are_lexer_errors() {
        assert_eq!(tokenize("commit;").unwrap_err().offset, 6);
    }

    #[test]
    fn accessibility_forms() {
        assert_eq!(kinds("accTitle: x y"), vec![Token::AccTitle("x y".into())]);
        assert_eq!(
            kinds("accDescr {\n first\n\n second   line\n}"),
            vec![Token::AccDescr("first\nsecond line".into())]
        );
        assert_eq!(kinds("accTitleX")[0], Token::Reference("accTitleX".into()));
    }
}
