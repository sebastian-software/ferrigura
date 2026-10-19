//! The lexer of the radar grammar, `packages/parser/src/language/radar/radar.langium` with the
//! shared terminals of `language/common/common.langium`.
//!
//! Chevrotain takes the first terminal that matches, not the longest, in the order the lexer
//! builds: the keywords first, longest literal first, then the terminals in grammar order, then the
//! hidden ones. A keyword gives way to its `LONGER_ALT` terminal, the ID, only when the ID matches
//! longer. The order and each rule's probe are in `docs-port/radar/PLAN.md`, section 2. The
//! terminals that have no `LONGER_ALT` take a prefix of any word they match, so `circlex` is the
//! shape `circle` and then the name `x`, and `title1` is the title line and then the number `1`.

use ferrigura_model::radar::Graticule;

use crate::common::{descr_block_text, plain_text, string_text};
use crate::lexer::{colon_form, directive_len, yaml_len};
use crate::text::{is_js_space, is_line_term, js_number};

/// The keywords that name an option or a statement, all with the ID as `LONGER_ALT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kw {
    Axis,
    Curve,
    ShowLegend,
    Ticks,
    Graticule,
    Max,
    Min,
}

/// The keywords of the grammar, as literals. None is a prefix of another, so their order does not
/// matter among them.
const KEYWORDS: &[(&str, Kw)] = &[
    ("showLegend", Kw::ShowLegend),
    ("graticule", Kw::Graticule),
    ("curve", Kw::Curve),
    ("ticks", Kw::Ticks),
    ("axis", Kw::Axis),
    ("max", Kw::Max),
    ("min", Kw::Min),
];

/// A token of the grammar. Comments, directives, front matter and blanks are hidden.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `NEWLINE: /\r?\n/`.
    Newline,
    /// The header, `'radar-beta'` or `'radar-beta:'` with its colon in the same token.
    Header { with_colon: bool },
    /// One of the keywords of [`Kw`].
    Keyword(Kw),
    /// `:`.
    Colon,
    /// `,`.
    Comma,
    /// `[`.
    LBracket,
    /// `]`.
    RBracket,
    /// `{`.
    LBrace,
    /// `}`.
    RBrace,
    /// `GRATICULE: 'circle' | 'polygon'`, the shape it names.
    Shape(Graticule),
    /// `BOOLEAN: 'true' | 'false'`, case-sensitive, with the value Langium's converter gives.
    Boolean(bool),
    /// `ACC_DESCR`, the value after the converter.
    AccDescr(String),
    /// `ACC_TITLE`, the value after the converter.
    AccTitle(String),
    /// `TITLE`, the value after the converter.
    Title(String),
    /// `NUMBER`, read as JavaScript reads `Number(text)`.
    Number(f64),
    /// `STRING`, with the quotes removed and the escapes resolved.
    Str(String),
    /// `ID: /[\w]([-\w]*\w)?/`.
    Id(String),
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// What the lexer does at one position: emit a token of this many bytes, or skip hidden text.
enum Step {
    Emit(Token, usize),
    Hidden(usize),
}

/// Tokenises the text of a radar diagram, without the hidden tokens. The error is the byte offset
/// of the first character that no terminal matches.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Spanned>, usize> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let (len, token) = match next_step(rest) {
            Some(Step::Emit(token, len)) if len > 0 => (len, Some(token)),
            Some(Step::Hidden(len)) if len > 0 => (len, None),
            _ => return Err(pos),
        };
        if let Some(token) = token {
            tokens.push(Spanned { token, start: pos });
        }
        pos += len;
    }
    Ok(tokens)
}

/// The first terminal that matches at the start of `rest`, in the lexer's order.
fn next_step(rest: &str) -> Option<Step> {
    if rest.starts_with("radar-beta:") {
        return Some(Step::Emit(
            Token::Header { with_colon: true },
            "radar-beta:".len(),
        ));
    }
    if header_boundary(rest) {
        return Some(longer_or(
            rest,
            "radar-beta".len(),
            Token::Header { with_colon: false },
        ));
    }
    for &(literal, kw) in KEYWORDS {
        if rest.starts_with(literal) {
            return Some(longer_or(rest, literal.len(), Token::Keyword(kw)));
        }
    }
    let first = rest.chars().next()?;
    let punct = match first {
        ':' => Some(Token::Colon),
        ',' => Some(Token::Comma),
        '[' => Some(Token::LBracket),
        ']' => Some(Token::RBracket),
        '{' => Some(Token::LBrace),
        '}' => Some(Token::RBrace),
        _ => None,
    };
    if let Some(token) = punct {
        return Some(Step::Emit(token, 1));
    }
    if rest.starts_with("circle") {
        return Some(Step::Emit(Token::Shape(Graticule::Circle), "circle".len()));
    }
    if rest.starts_with("polygon") {
        return Some(Step::Emit(
            Token::Shape(Graticule::Polygon),
            "polygon".len(),
        ));
    }
    if rest.starts_with("true") {
        return Some(Step::Emit(Token::Boolean(true), "true".len()));
    }
    if rest.starts_with("false") {
        return Some(Step::Emit(Token::Boolean(false), "false".len()));
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
    if let Some(len) = number_len(rest) {
        return Some(Step::Emit(Token::Number(js_number(&rest[..len])), len));
    }
    if let Some(len) = string_len(rest) {
        return Some(Step::Emit(Token::Str(string_text(&rest[..len])), len));
    }
    if let Some(len) = id_len(rest) {
        return Some(Step::Emit(Token::Id(rest[..len].to_owned()), len));
    }
    if let Some(len) = newline_len(rest) {
        return Some(Step::Emit(Token::Newline, len));
    }
    // WHITESPACE is hidden. It comes after the terminals that start with blanks, as in the lexer.
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

/// The keyword's token when the ID does not match longer, else the ID the `LONGER_ALT` matched.
fn longer_or(rest: &str, keyword_len: usize, token: Token) -> Step {
    match id_len(rest) {
        Some(longer) if longer > keyword_len => {
            Step::Emit(Token::Id(rest[..longer].to_owned()), longer)
        }
        _ => Step::Emit(token, keyword_len),
    }
}

/// `radar-beta(?:(?=%%)|(?!\S))`: the header keyword ends at a blank, the end of the text, or `%%`.
/// The mermaid token builder adds this suffix; `radar-betaX` is not the header.
fn header_boundary(rest: &str) -> bool {
    rest.strip_prefix("radar-beta").is_some_and(|after| {
        after.is_empty() || after.starts_with("%%") || after.chars().next().is_some_and(is_js_space)
    })
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

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `ID: /[\w]([-\w]*\w)?/`. The greedy run of word characters and dashes gives back its trailing
/// dashes, so the match ends in a word character. `\w` is ASCII in JavaScript.
fn id_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !bytes.first().is_some_and(|&b| is_word_byte(b)) {
        return None;
    }
    let run = bytes
        .iter()
        .take_while(|&&b| is_word_byte(b) || b == b'-')
        .count();
    (1..=run).rev().find(|&len| is_word_byte(bytes[len - 1]))
}

/// `NUMBER: FLOAT | INT`, with `FLOAT: /[0-9]+\.[0-9]+(?!\.)/` and `INT: /0|[1-9][0-9]*(?!\.)/`.
/// The lookahead of each form gives back one digit when a dot follows the match, as the regex does;
/// a dot is never a token, so the outcome for the text that follows is a lexer error either way.
fn number_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let digits = |from: usize| {
        bytes[from.min(bytes.len())..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(0);
    if whole > 0 && bytes.get(whole) == Some(&b'.') {
        let fraction = digits(whole + 1);
        let taken = if bytes.get(whole + 1 + fraction) == Some(&b'.') {
            fraction.saturating_sub(1)
        } else {
            fraction
        };
        if taken > 0 {
            return Some(whole + 1 + taken);
        }
    }
    match bytes.first()? {
        b'0' => Some(1),
        b'1'..=b'9' => {
            if bytes.get(whole) != Some(&b'.') {
                Some(whole)
            } else {
                // `[1-9][0-9]*(?!\.)` gives back one digit before the dot; one digit alone has none.
                (whole >= 2).then(|| whole - 1)
            }
        }
        _ => None,
    }
}

/// `STRING: /"([^"\\]|\\.)*"|'([^'\\]|\\.)*'/`, from its opening quote: the length up to and
/// including the closing quote. A backslash takes the next character, which may not be a line
/// break; any other character, a line break included, is copied.
fn string_len(rest: &str) -> Option<usize> {
    let quote = match rest.as_bytes().first()? {
        b'"' => '"',
        b'\'' => '\'',
        _ => return None,
    };
    let mut i = 1;
    while i < rest.len() {
        let c = rest[i..].chars().next()?;
        if c == quote {
            return Some(i + 1);
        }
        if c == '\\' {
            let next = rest[i + 1..].chars().next()?;
            if is_line_term(next) {
                return None;
            }
            i += 1 + next.len_utf8();
        } else {
            i += c.len_utf8();
        }
    }
    None
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
/// up to a `%%` on that line, or nothing when the keyword is not followed by a blank. The leading
/// blanks are part of the token, so a title line may begin with them.
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
        tokenize(text)
            .expect("the text lexes")
            .into_iter()
            .map(|spanned| spanned.token)
            .collect()
    }

    #[test]
    fn a_keyword_gives_way_to_a_longer_word() {
        assert_eq!(kinds("axis")[0], Token::Keyword(Kw::Axis));
        assert_eq!(kinds("axisX")[0], Token::Id("axisX".into()));
        assert_eq!(kinds("maximum")[0], Token::Id("maximum".into()));
        assert_eq!(kinds("max")[0], Token::Keyword(Kw::Max));
        assert_eq!(kinds("showLegend2")[0], Token::Id("showLegend2".into()));
    }

    #[test]
    fn the_terminals_without_a_longer_alternative_split_a_word() {
        assert_eq!(
            kinds("circlex"),
            vec![Token::Shape(Graticule::Circle), Token::Id("x".into())]
        );
        assert_eq!(
            kinds("trueX"),
            vec![Token::Boolean(true), Token::Id("X".into())]
        );
        assert_eq!(kinds("title1")[0], Token::Title(String::new()));
        assert_eq!(kinds("1A"), vec![Token::Number(1.0), Token::Id("A".into())]);
    }

    #[test]
    fn the_header_keyword_ends_at_a_boundary() {
        assert_eq!(kinds("radar-beta")[0], Token::Header { with_colon: false });
        assert_eq!(kinds("radar-beta:")[0], Token::Header { with_colon: true });
        assert_eq!(
            kinds("radar-beta%% x")[0],
            Token::Header { with_colon: false }
        );
        assert_eq!(kinds("radar-betaX")[0], Token::Id("radar-betaX".into()));
        assert_eq!(kinds("radar-beta{")[0], Token::Id("radar-beta".into()));
    }

    #[test]
    fn a_title_and_an_accessibility_line_take_their_value() {
        assert_eq!(
            kinds("title   My  Title\n")[0],
            Token::Title("My Title".into())
        );
        assert_eq!(
            kinds("accTitle: Hello there")[0],
            Token::AccTitle("Hello there".into())
        );
        assert_eq!(kinds("accTitlex")[0], Token::Id("accTitlex".into()));
        assert_eq!(kinds("accDescr {\n a\n}")[0], Token::AccDescr("a".into()));
    }

    #[test]
    fn numbers_follow_the_lexer_forms() {
        assert_eq!(kinds("2.5")[0], Token::Number(2.5));
        assert_eq!(kinds("01"), vec![Token::Number(0.0), Token::Number(1.0)]);
        assert_eq!(kinds("12")[0], Token::Number(12.0));
        // `2.` is no number: the dot is a lexer error after the digit.
        assert_eq!(tokenize("2."), Err(1));
    }

    #[test]
    fn a_dash_and_a_dot_are_lexer_errors() {
        assert_eq!(tokenize("-1"), Err(0));
        assert_eq!(tokenize("a.b"), Err(1));
        assert_eq!(tokenize("A-"), Err(1));
        assert_eq!(tokenize("Äx"), Err(0));
    }

    #[test]
    fn a_word_ends_before_a_dash_and_an_id_keeps_inner_dashes() {
        assert_eq!(kinds("a-1")[0], Token::Id("a-1".into()));
        assert_eq!(kinds("_x_1")[0], Token::Id("_x_1".into()));
    }

    #[test]
    fn a_yaml_block_and_a_comment_are_hidden() {
        assert!(kinds("---\nx\n---\n").is_empty());
        // A comment hides its text, not the line break that ends it.
        assert_eq!(kinds("%% note\n"), vec![Token::Newline]);
        assert_eq!(tokenize("---\nx"), Err(0));
    }
}
