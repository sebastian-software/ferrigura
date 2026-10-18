//! The lexer of the wardley grammar, `packages/parser/src/language/wardley/wardley.langium` with
//! `common/common.langium`.
//!
//! Langium builds the lexer in the order that `docs-port/wardley/NOTES.md`, section 2.2, lists
//! (probed on the reference). Chevrotain takes the first terminal that matches at the position,
//! not the longest. The one exception is the `-` punctuation, whose `LONGER_ALT` terminals are
//! `->`, `-->`, `-.->` and a `---` block: the longer one wins when it matches. A keyword is a
//! prefix match with no word end, so `notes` is `note` and then `s`, and `titlebar` is a name only
//! because the name's lookahead needs a blank after `title`.

use ferrigura_model::wardley::Strategy;

use crate::common::{descr_block_text, plain_text, string_text};
use crate::lexer::{colon_form, directive_len, yaml_len};
use crate::text::{is_js_space, is_line_term, js_number, js_trim};

/// The keywords, the terminals `KW_*`, in the lexer's order. `annotations` comes before
/// `annotation`, which is its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kw {
    Wardley,
    Size,
    Evolution,
    Anchor,
    Component,
    Label,
    Inertia,
    Evolve,
    Pipeline,
    Note,
    Annotations,
    Annotation,
    Accelerator,
    Deaccelerator,
}

const KEYWORDS: [(&str, Kw); 14] = [
    ("wardley-beta", Kw::Wardley),
    ("size", Kw::Size),
    ("evolution", Kw::Evolution),
    ("anchor", Kw::Anchor),
    ("component", Kw::Component),
    ("label", Kw::Label),
    ("inertia", Kw::Inertia),
    ("evolve", Kw::Evolve),
    ("pipeline", Kw::Pipeline),
    ("note", Kw::Note),
    ("annotations", Kw::Annotations),
    ("annotation", Kw::Annotation),
    ("accelerator", Kw::Accelerator),
    ("deaccelerator", Kw::Deaccelerator),
];

/// `STRATEGY: 'build' | 'buy' | 'outsource' | 'market'`, in that order.
const STRATEGIES: [(&str, Strategy); 4] = [
    ("build", Strategy::Build),
    ("buy", Strategy::Buy),
    ("outsource", Strategy::Outsource),
    ("market", Strategy::Market),
];

/// `LINK_PORT: '+<>' | '+>' | '+<'`, in that order.
const LINK_PORTS: [&str; 3] = ["+<>", "+>", "+<"];

/// A token of the grammar. The hidden tokens (blanks, comments, directives, `---` blocks) are
/// not in the list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `NEWLINE: /\r?\n/`.
    Newline,
    /// One of the punctuation terminals `[ ] ( ) { } , @ /` and `-`.
    Punct(char),
    /// `WARDLEY_NUMBER: /[0-9]+\.[0-9]+/`, read as `Number()`.
    Decimal(f64),
    /// `INT: /0|[1-9][0-9]*(?!\.)/`, read as `parseInt`.
    Integer(f64),
    /// `ARROW: '->'`.
    Arrow,
    /// `LINK_PORT`, as its text.
    LinkPort(&'static str),
    /// `LINK_ARROW`, as its text (`-->`, `-.->`, `>`, or a `+'label'` form).
    LinkArrow(String),
    /// `LINK_LABEL: /;[^\n\r]+/`, after the converter: the `;` removed and the rest trimmed.
    LinkLabel(String),
    /// `STRATEGY`.
    Strategy(Strategy),
    /// A keyword.
    Kw(Kw),
    /// `NAME_WITH_SPACES`, as written.
    Name(String),
    /// `ID: /[\w]([-\w]*\w)?/`, as written.
    Id(String),
    /// `STRING`, after the converter: the quotes removed and the escapes resolved.
    Str(String),
    /// `ACC_DESCR`, after the common converter.
    AccDescr(String),
    /// `ACC_TITLE`, after the common converter.
    AccTitle(String),
    /// `TITLE`, after the common converter.
    Title(String),
}

/// A token with the byte offset where it starts in the text that was lexed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// A piece of the text that no terminal matches, at this byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexFailure {
    pub offset: usize,
}

/// What the lexer does at one position: emit a token of this many bytes, or skip hidden text.
enum Step {
    Emit(Token, usize),
    Hidden(usize),
}

/// Tokenises the prepared text of a wardley map, without the hidden tokens.
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
    let first = rest.chars().next()?;
    if matches!(first, '[' | ']' | '(' | ')' | '{' | '}' | ',' | '@' | '/') {
        return Some(Step::Emit(Token::Punct(first), 1));
    }
    if first == '-' {
        return Some(dash_step(rest));
    }
    if let Some(len) = decimal_len(rest) {
        return Some(Step::Emit(Token::Decimal(js_number(&rest[..len])), len));
    }
    if let Some(port) = LINK_PORTS.iter().find(|port| rest.starts_with(**port)) {
        return Some(Step::Emit(Token::LinkPort(port), port.len()));
    }
    if let Some(len) = link_arrow_len(rest) {
        return Some(Step::Emit(Token::LinkArrow(rest[..len].to_owned()), len));
    }
    if let Some(len) = link_label_len(rest) {
        let value = js_trim(&rest[1..len]).to_owned();
        return Some(Step::Emit(Token::LinkLabel(value), len));
    }
    if let Some((text, strategy)) = STRATEGIES.iter().find(|(text, _)| rest.starts_with(text)) {
        return Some(Step::Emit(Token::Strategy(*strategy), text.len()));
    }
    for (literal, kw) in KEYWORDS {
        if rest.starts_with(literal) {
            return Some(Step::Emit(Token::Kw(kw), literal.len()));
        }
    }
    if let Some(len) = name_len(rest) {
        return Some(Step::Emit(Token::Name(rest[..len].to_owned()), len));
    }
    if first == ' ' || first == '\t' {
        return Some(Step::Hidden(blank_len(rest)));
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
    if let Some(len) = int_len(rest) {
        return Some(Step::Emit(Token::Integer(js_number(&rest[..len])), len));
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
    // `DIRECTIVE` and `SINGLE_LINE_COMMENT`, after the blanks that `WS` took. The `---` block
    // (`YAML`) is reached only through the `-` punctuation.
    if rest.starts_with("%%") {
        return Some(Step::Hidden(comment_len(rest)));
    }
    None
}

/// The `-` punctuation and its `LONGER_ALT` terminals: `->`, `-->`, `-.->` and a `---` block. The
/// alternatives start with different characters, so at most one of them matches.
fn dash_step(rest: &str) -> Step {
    if rest.starts_with("->") {
        return Step::Emit(Token::Arrow, 2);
    }
    if rest.starts_with("-->") {
        return Step::Emit(Token::LinkArrow("-->".to_owned()), 3);
    }
    if rest.starts_with("-.->") {
        return Step::Emit(Token::LinkArrow("-.->".to_owned()), 4);
    }
    if let Some(len) = rest.strip_prefix("---").and_then(yaml_len) {
        return Step::Hidden(3 + len);
    }
    Step::Emit(Token::Punct('-'), 1)
}

/// `WARDLEY_NUMBER: /[0-9]+\.[0-9]+/`: digits, a dot and digits, in bytes.
fn decimal_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let whole = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if whole == 0 || bytes.get(whole) != Some(&b'.') {
        return None;
    }
    let fraction = bytes[whole + 1..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    (fraction > 0).then_some(whole + 1 + fraction)
}

/// `INT: /0|[1-9][0-9]*(?!\.)/`. The `0` alternative has no lookahead. The digits of the other
/// alternative give back digits until the character after them is not a dot, so `12.` is `1`.
fn int_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    match bytes.first()? {
        b'0' => Some(1),
        b'1'..=b'9' => {
            let run = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            (1..=run).rev().find(|&len| bytes.get(len) != Some(&b'.'))
        }
        _ => None,
    }
}

/// `LINK_ARROW`'s `>` and `+'label'<>`, `+'label'<` and `+'label'>` (the label has no quote). The
/// alternatives of the regex are tried in that order.
fn link_arrow_len(rest: &str) -> Option<usize> {
    if rest.starts_with('>') {
        return Some(1);
    }
    let after = rest.strip_prefix("+'")?;
    let close = after.find('\'')?;
    let tail = &after[close + 1..];
    ["<>", "<", ">"]
        .into_iter()
        .find(|end| tail.starts_with(end))
        .map(|end| 2 + close + 1 + end.len())
}

/// `LINK_LABEL: /;[^\n\r]+/`: a `;` and at least one character up to the line break.
fn link_label_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix(';')?;
    let run = after.find(['\n', '\r']).unwrap_or(after.len());
    (run > 0).then_some(1 + run)
}

/// `NAME_WITH_SPACES: /(?!title\s|accTitle|accDescr)[A-Za-z](?:[A-Za-z0-9_()&]|-(?!>))*(?:[ \t]+[A-Za-z(](?:[A-Za-z0-9_()&]|-(?!>))*)*/`.
/// The match is the greedy one: nothing follows the pattern, so no alternative is tried.
fn name_len(rest: &str) -> Option<usize> {
    if rest.starts_with("accTitle") || rest.starts_with("accDescr") {
        return None;
    }
    if let Some(after) = rest.strip_prefix("title")
        && after.chars().next().is_some_and(is_js_space)
    {
        return None;
    }
    let first = rest.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let mut end = word_run(rest, 1);
    loop {
        let blanks = rest.as_bytes()[end..]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        if blanks == 0 {
            break;
        }
        match rest[end + blanks..].chars().next() {
            Some(c) if c.is_ascii_alphabetic() || c == '(' => {
                end = word_run(rest, end + blanks + 1);
            }
            _ => break,
        }
    }
    Some(end)
}

/// `(?:[A-Za-z0-9_()&]|-(?!>))*` from byte `from`: the end of the run of name characters.
fn word_run(rest: &str, from: usize) -> usize {
    let bytes = rest.as_bytes();
    let mut end = from;
    while let Some(&b) = bytes.get(end) {
        let keeps = b.is_ascii_alphanumeric()
            || matches!(b, b'_' | b'(' | b')' | b'&')
            || (b == b'-' && bytes.get(end + 1) != Some(&b'>'));
        if !keeps {
            break;
        }
        end += 1;
    }
    end
}

/// `STRING: /"([^"\\]|\\.)*"|'([^'\\]|\\.)*'/`, from its opening quote: the length up to and
/// including the closing quote. A backslash takes the next character, which may not be a line
/// break. Any other character, a line break included, is copied.
fn string_len(rest: &str) -> Option<usize> {
    let quote = match rest.as_bytes().first()? {
        b'"' => '"',
        b'\'' => '\'',
        _ => return None,
    };
    let mut chars = rest.char_indices().skip(1);
    while let Some((index, c)) = chars.next() {
        if c == quote {
            return Some(index + 1);
        }
        if c == '\\' {
            let (_, escaped) = chars.next()?;
            if is_line_term(escaped) {
                return None;
            }
        }
    }
    None
}

/// Whether `b` is an ASCII word character, which is `\w` in JavaScript.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `ID: /[\w]([-\w]*\w)?/`. The greedy run of word characters and dashes gives back its trailing
/// dashes, so the match ends in a word character.
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

/// `NEWLINE: /\r?\n/`.
fn newline_len(rest: &str) -> Option<usize> {
    if rest.starts_with("\r\n") {
        Some(2)
    } else if rest.starts_with('\n') {
        Some(1)
    } else {
        None
    }
}

/// `WS: /[ \t]+/`, in bytes.
fn blank_len(rest: &str) -> usize {
    rest.bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count()
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

/// `DIRECTIVE` or `SINGLE_LINE_COMMENT`, from its `%%`: a directive when one closes with `}%%` and
/// a boundary, otherwise the rest of the line.
fn comment_len(rest: &str) -> usize {
    let after = &rest[2..];
    if after.starts_with('{')
        && let Some(len) = directive_len(after)
    {
        return 2 + len;
    }
    2 + line_len(after)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<Token> {
        tokenize(text)
            .expect("the text lexes")
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn the_dash_takes_its_longer_alternatives_only_when_they_match() {
        assert_eq!(kinds("->"), vec![Token::Arrow]);
        assert_eq!(kinds("-->"), vec![Token::LinkArrow("-->".into())]);
        assert_eq!(kinds("-.->"), vec![Token::LinkArrow("-.->".into())]);
        assert_eq!(kinds("-1"), vec![Token::Punct('-'), Token::Integer(1.0)]);
    }

    #[test]
    fn a_keyword_is_a_prefix_and_a_name_needs_a_blank_after_title() {
        assert_eq!(
            kinds("notes"),
            vec![Token::Kw(Kw::Note), Token::Name("s".into())]
        );
        assert_eq!(kinds("titlebar"), vec![Token::Name("titlebar".into())]);
        assert_eq!(kinds("Size"), vec![Token::Name("Size".into())]);
    }

    #[test]
    fn a_name_keeps_its_inner_blanks_and_its_hyphens_but_not_an_arrow() {
        assert_eq!(
            kinds("Data  Store"),
            vec![Token::Name("Data  Store".into())]
        );
        assert_eq!(kinds("foo-"), vec![Token::Name("foo-".into())]);
        assert_eq!(
            kinds("a->b"),
            vec![
                Token::Name("a".into()),
                Token::Arrow,
                Token::Name("b".into())
            ]
        );
    }

    #[test]
    fn an_integer_gives_back_a_digit_before_a_dot() {
        assert_eq!(int_len("12."), Some(1));
        assert_eq!(int_len("12"), Some(2));
        assert_eq!(int_len("1."), None);
        assert_eq!(kinds("0.5"), vec![Token::Decimal(0.5)]);
    }

    #[test]
    fn a_title_stops_at_a_comment_on_its_line() {
        assert_eq!(kinds("title 50%% done"), vec![Token::Title("50".into())]);
        assert_eq!(
            kinds("title\n"),
            vec![Token::Title(String::new()), Token::Newline]
        );
    }

    #[test]
    fn a_link_label_needs_one_character() {
        assert_eq!(tokenize("A +> B ;\n").map_err(|f| f.offset), Err(7));
        assert_eq!(
            kinds("; label; more  "),
            vec![Token::LinkLabel("label; more".into())]
        );
    }
}
