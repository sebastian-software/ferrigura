//! The lexer of the architecture grammar, `packages/parser/src/language/architecture/`.
//!
//! Langium builds the lexer in this order, which the reference's `DefaultTokenBuilder` produces
//! and which `docs-port/architecture/PLAN.md`, section 2, lists in full:
//!
//! 1. the keywords, longest first, `architecture-beta` and `architecture` with the word-end
//!    suffix of the mermaid token builder;
//! 2. the terminals of the grammar, in the order the reachable rules give them: `ARROW_DIRECTION`
//!    (`L|R|T|B`), `ARROW_GROUP`, `ARROW_INTO`, `ACC_DESCR`, `ACC_TITLE`, `TITLE`, `STRING`, `ID`,
//!    `NEWLINE`, then the hidden and skipped `WHITESPACE`, `YAML`, `DIRECTIVE` and
//!    `SINGLE_LINE_COMMENT`, then `ARCH_ICON` and `ARCH_TITLE`.
//!
//! Chevrotain takes the first match, not the longest, except that a keyword gives way to its
//! `LONGER_ALT` when that match is longer: the ID terminal for a word, the YAML block for `--` and
//! `-`. A terminal has no alternative of its own, so `Lx` is the direction `L` and then `x`, and
//! `title1` is the `title` line and then `1`.

use ferrigura_model::architecture::Direction;

use crate::common::{descr_block_text, plain_text, string_text};
use crate::lexer::{colon_form, directive_len, yaml_len};
use crate::text::{is_js_space, is_line_term};

/// The keywords of the grammar. `architecture` and `architecture-beta` are the same token here,
/// since the parser reads both as the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kw {
    Architecture,
    Junction,
    Service,
    Column,
    Group,
    Align,
    Row,
    In,
    /// `--`
    DashDash,
    /// `:`
    Colon,
    /// `-`
    Dash,
}

/// A token of the grammar. Comments, directives, front matter and whitespace are hidden.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `NEWLINE: /\r?\n/`.
    Newline,
    Keyword(Kw),
    /// `ID: /[\w]([-\w]*\w)?/`, the name of a service, junction, group, or edge end.
    Id(String),
    /// `ARROW_DIRECTION: L|R|T|B`.
    Direction(Direction),
    /// `ARROW_GROUP: {group}`.
    GroupMark,
    /// `ARROW_INTO: <|>`. The grammar keeps only that the token was there.
    Into,
    /// `ACC_DESCR`, the value after the converter.
    AccDescr(String),
    /// `ACC_TITLE`, the value after the converter.
    AccTitle(String),
    /// `TITLE`, the value after the converter.
    Title(String),
    /// `STRING`, with the quotes removed and the escapes resolved: a service's icon text.
    Str(String),
    /// `ARCH_ICON: \([\w-:]+\)`, the icon name without its parentheses.
    Icon(String),
    /// `ARCH_TITLE`, the label after the converter: without its brackets, trimmed, and without
    /// one pair of matching quotes.
    Label(String),
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// A piece of the text that no terminal matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexFailure {
    pub offset: usize,
}

/// How a keyword's `LONGER_ALT` terminal is chosen when it matches longer than the keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LongerAlt {
    /// The ID terminal, for a keyword that is a word.
    Id,
    /// The YAML block, for `--` and `-`.
    Yaml,
    /// No alternative: `:` and `architecture` have no token that can be longer.
    None,
}

/// A keyword literal in the order of the lexer. `boundary` is the word-end suffix of
/// `(?:(?=%%)|(?!\S))` that the mermaid token builder adds to the two header keywords.
struct KeywordRule {
    literal: &'static str,
    kw: Kw,
    boundary: bool,
    longer: LongerAlt,
}

/// The keywords, longest literal first, as `buildKeywordTokens` sorts them.
const KEYWORDS: &[KeywordRule] = &[
    KeywordRule {
        literal: "architecture-beta",
        kw: Kw::Architecture,
        boundary: true,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "architecture",
        kw: Kw::Architecture,
        boundary: true,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "junction",
        kw: Kw::Junction,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "service",
        kw: Kw::Service,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "column",
        kw: Kw::Column,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "group",
        kw: Kw::Group,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "align",
        kw: Kw::Align,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "row",
        kw: Kw::Row,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: "--",
        kw: Kw::DashDash,
        boundary: false,
        longer: LongerAlt::Yaml,
    },
    KeywordRule {
        literal: "in",
        kw: Kw::In,
        boundary: false,
        longer: LongerAlt::Id,
    },
    KeywordRule {
        literal: ":",
        kw: Kw::Colon,
        boundary: false,
        longer: LongerAlt::None,
    },
    KeywordRule {
        literal: "-",
        kw: Kw::Dash,
        boundary: false,
        longer: LongerAlt::Yaml,
    },
];

/// What the lexer does at one position: emit a token of this many bytes, or skip hidden text.
enum Step {
    Emit(Token, usize),
    Hidden(usize),
}

/// Tokenises the text of an architecture diagram, without the hidden tokens.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Spanned>, LexFailure> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let (len, token) = match next_step(rest) {
            Some(Step::Emit(token, len)) if len > 0 => (len, Some(token)),
            Some(Step::Hidden(len)) if len > 0 => (len, None),
            _ => return Err(LexFailure { offset: pos }),
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
    for rule in KEYWORDS {
        if let Some(len) = keyword_len(rest, rule) {
            return Some(keyword_step(rest, rule, len));
        }
    }
    let first = rest.chars().next()?;
    if let Some(direction) = direction_of(first) {
        return Some(Step::Emit(Token::Direction(direction), 1));
    }
    if rest.starts_with("{group}") {
        return Some(Step::Emit(Token::GroupMark, "{group}".len()));
    }
    if first == '<' || first == '>' {
        return Some(Step::Emit(Token::Into, 1));
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
    if let Some(len) = id_len(rest) {
        return Some(Step::Emit(Token::Id(rest[..len].to_owned()), len));
    }
    if let Some(len) = newline_len(rest) {
        return Some(Step::Emit(Token::Newline, len));
    }
    // WHITESPACE is skipped; it comes after the terminals that start with blanks.
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
    if let Some(len) = icon_len(rest) {
        return Some(Step::Emit(Token::Icon(icon_text(&rest[..len])), len));
    }
    if let Some(len) = arch_title_len(rest) {
        return Some(Step::Emit(Token::Label(label_text(&rest[..len])), len));
    }
    None
}

/// `L`, `R`, `T` or `B`, the characters of `ARROW_DIRECTION`.
fn direction_of(c: char) -> Option<Direction> {
    match c {
        'L' => Some(Direction::L),
        'R' => Some(Direction::R),
        'T' => Some(Direction::T),
        'B' => Some(Direction::B),
        _ => None,
    }
}

/// The byte length of a keyword literal at the start of `rest`, if it matches. A boundary keyword
/// must be followed by `%%`, by a blank, or by the end of the text.
fn keyword_len(rest: &str, rule: &KeywordRule) -> Option<usize> {
    let after = rest.strip_prefix(rule.literal)?;
    if rule.boundary {
        let ends_word = after.is_empty()
            || after.starts_with("%%")
            || after.chars().next().is_some_and(is_js_space);
        if !ends_word {
            return None;
        }
    }
    Some(rule.literal.len())
}

/// The token of a keyword that matched with `len` bytes, or the `LONGER_ALT` terminal that
/// matched longer.
fn keyword_step(rest: &str, rule: &KeywordRule, len: usize) -> Step {
    match rule.longer {
        LongerAlt::Id => match id_len(rest) {
            Some(longer) if longer > len => {
                Step::Emit(Token::Id(rest[..longer].to_owned()), longer)
            }
            _ => Step::Emit(Token::Keyword(rule.kw), len),
        },
        LongerAlt::Yaml => match yaml_hidden_len(rest) {
            Some(longer) if longer > len => Step::Hidden(longer),
            _ => Step::Emit(Token::Keyword(rule.kw), len),
        },
        LongerAlt::None => Step::Emit(Token::Keyword(rule.kw), len),
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
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

/// `ARCH_ICON: /\([\w-:]+\)/`: a name of word characters, dashes and colons, in parentheses.
fn icon_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix('(')?;
    let run = body
        .bytes()
        .take_while(|&b| is_word_byte(b) || b == b'-' || b == b':')
        .count();
    (run > 0 && body.as_bytes().get(run) == Some(&b')')).then_some(run + 2)
}

/// The value of `ARCH_ICON`: its parentheses removed, then trimmed.
fn icon_text(token: &str) -> String {
    token.trim_matches(['(', ')']).trim().to_owned()
}

/// `ARCH_TITLE: /\[(?:"([^"\\]|\\.)*"|'([^'\\]|\\.)*'|[^\[\]\r\n]+)\]/`. A quoted form is taken
/// when its closing quote is followed by `]`; otherwise the run of characters up to `[`, `]` or a
/// line break is taken when `]` follows it.
fn arch_title_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix('[')?;
    if body.starts_with(['"', '\''])
        && let Some(len) = string_len(body)
        && body[len..].starts_with(']')
    {
        return Some(1 + len + 1);
    }
    let run = body.find(['[', ']', '\r', '\n']).unwrap_or(body.len());
    (run > 0 && body[run..].starts_with(']')).then_some(1 + run + 1)
}

/// The value of `ARCH_TITLE`: the brackets removed, the text trimmed, and, when it is wrapped in
/// one pair of matching quotes, the quotes removed with `\"` and `\'` unescaped; then trimmed.
fn label_text(token: &str) -> String {
    let inner = &token[1..token.len() - 1];
    let mut result = plain_trim(inner).to_owned();
    let quoted = (result.starts_with('"') && result.ends_with('"'))
        || (result.starts_with('\'') && result.ends_with('\''));
    if quoted {
        // A single quote character is both the start and the end: `slice(1, -1)` gives "".
        result = if result.len() >= 2 {
            result[1..result.len() - 1].to_owned()
        } else {
            String::new()
        };
        result = result.replace("\\\"", "\"").replace("\\'", "'");
    }
    plain_trim(&result).to_owned()
}

/// `String.prototype.trim` on the text.
fn plain_trim(text: &str) -> &str {
    crate::text::js_trim(text)
}

/// `ACC_DESCR: /[\t ]*accDescr(?:[\t ]*:([^\n\r]*?(?=%%)|[^\n\r]*)|\s*{([^}]*)})/`: the colon
/// form as for `accTitle`, or a block in braces that may run over several lines.
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
            .expect("lexes")
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn a_keyword_gives_way_to_a_longer_word() {
        assert_eq!(kinds("group")[0], Token::Keyword(Kw::Group));
        assert_eq!(kinds("groupie")[0], Token::Id("groupie".into()));
        assert_eq!(kinds("in")[0], Token::Keyword(Kw::In));
        assert_eq!(kinds("inner")[0], Token::Id("inner".into()));
        assert_eq!(kinds("rowspan")[0], Token::Id("rowspan".into()));
    }

    #[test]
    fn a_direction_letter_is_a_token_before_an_id() {
        assert_eq!(
            kinds("Lx"),
            vec![Token::Direction(Direction::L), Token::Id("x".into())]
        );
        assert_eq!(kinds("bob")[0], Token::Id("bob".into()));
    }

    #[test]
    fn title_takes_its_line_and_a_word_after_it_is_a_separate_token() {
        assert_eq!(kinds("title a  b %% c"), vec![Token::Title("a b".into())]);
        assert_eq!(
            kinds("title1"),
            vec![Token::Title(String::new()), Token::Id("1".into())]
        );
        assert_eq!(kinds("accTitlex")[0], Token::Id("accTitlex".into()));
    }

    #[test]
    fn architecture_is_a_keyword_only_before_a_blank_or_a_comment() {
        assert_eq!(
            kinds("architecture-beta")[0],
            Token::Keyword(Kw::Architecture)
        );
        assert_eq!(kinds("architecture x")[0], Token::Keyword(Kw::Architecture));
        assert_eq!(
            kinds("architecture(x)")[0],
            Token::Id("architecture".into())
        );
        assert_eq!(
            kinds("architecture-betax")[0],
            Token::Id("architecture-betax".into())
        );
    }

    #[test]
    fn yaml_wins_over_the_dash_keywords_when_it_matches() {
        assert_eq!(
            kinds("---\nx\n---\n"),
            Vec::<Token>::new(),
            "a front matter block in the body is hidden"
        );
        assert_eq!(kinds("--"), vec![Token::Keyword(Kw::DashDash)]);
        assert_eq!(
            kinds("---"),
            vec![Token::Keyword(Kw::DashDash), Token::Keyword(Kw::Dash)]
        );
    }

    #[test]
    fn labels_and_icons_are_read_as_their_values() {
        assert_eq!(kinds("(logos:aws)")[0], Token::Icon("logos:aws".into()));
        assert_eq!(kinds("[ A ]")[0], Token::Label("A".into()));
        assert_eq!(kinds("[\"A x\"]")[0], Token::Label("A x".into()));
        assert_eq!(kinds("[\"x\\\"y\"]")[0], Token::Label("x\"y".into()));
        assert_eq!(kinds("[\" \"]")[0], Token::Label(String::new()));
        assert!(tokenize("[]").is_err());
        assert!(tokenize("(a b)").is_err());
    }

    #[test]
    fn a_label_stops_at_a_bracket_or_a_line_break() {
        assert!(tokenize("[A\n]").is_err());
        assert!(tokenize("[A[B]]").is_err());
        // The quoted form wins over the run: the run would stop at the first `]` inside the quotes.
        assert_eq!(kinds("[\"A]\"]")[0], Token::Label("A]".into()));
    }

    #[test]
    fn a_string_keeps_its_line_breaks() {
        assert_eq!(kinds("\"a\nb\"")[0], Token::Str("a\nb".into()));
        assert!(tokenize("\"open").is_err());
    }
}
