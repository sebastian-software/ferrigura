//! The railroad scanner: the token tables of the four Langium grammars (`railroad`, `railroad-ebnf`,
//! `railroad-abnf` and `railroad-peg`), read in Chevrotain's order. Chevrotain takes the first token
//! type whose pattern matches at the position: the keywords of the grammar, then its terminals, then
//! its hidden tokens. A keyword with a `LONGER_ALT` gives way to the alternative when the alternative
//! matches more characters. The rules and the quirks are in `docs-port/railroad/PLAN.md`, section 2.

use crate::common::{descr_block_text, plain_text};
use crate::error::{ParseError, Source};
use crate::lexer::{directive_len, yaml_len};
use crate::text::{is_cr_lf, is_js_space, is_line_term, is_word, js_trim};

/// The four syntaxes, one per header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Syntax {
    /// `railroad-beta`: the `name = expr ;` form with `sequence(...)`, `choice(...)` and so on.
    Ir,
    /// `railroad-ebnf-beta`: ISO 14977 and W3C-style rules.
    Ebnf,
    /// `railroad-abnf-beta`: RFC 5234 rules, each ended by `;`.
    Abnf,
    /// `railroad-peg-beta`: `Name <- expr ;` with the PEG operators.
    Peg,
}

impl Syntax {
    /// The header keyword of the syntax, as its grammar writes it (case-sensitive).
    pub(crate) fn header(self) -> &'static str {
        match self {
            Self::Ir => "railroad-beta",
            Self::Ebnf => "railroad-ebnf-beta",
            Self::Abnf => "railroad-abnf-beta",
            Self::Peg => "railroad-peg-beta",
        }
    }

    /// The id of the reference's detector for the syntax (`railroadDetector.ts` and the three
    /// siblings, `railroadDiagram.ts` keys). The renderer names the SVG with it.
    pub(crate) fn detector_id(self) -> &'static str {
        match self {
            Self::Ir => "railroad",
            Self::Ebnf => "railroadEbnf",
            Self::Abnf => "railroadAbnf",
            Self::Peg => "railroadPeg",
        }
    }
}

/// The word keywords of the IR grammar, which call the expression forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Word {
    Nonterminal,
    ZeroOrMore,
    OneOrMore,
    Sequence,
    Optional,
    Terminal,
    Special,
    Choice,
}

/// The punctuation marks of the four grammars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sym {
    /// `=` (IR, EBNF, ABNF).
    Eq,
    /// `::=` (EBNF).
    ColonEq,
    /// `<-` (PEG).
    Arrow,
    /// `;`.
    Semi,
    /// `|` (EBNF) or `/` (ABNF, PEG).
    Alt,
    /// `,`.
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    /// `?` as a postfix (EBNF, PEG).
    Question,
    Star,
    Plus,
    /// `-` as the exception postfix (EBNF).
    Minus,
    /// `&` (PEG).
    Amp,
    /// `!` (PEG).
    Bang,
    /// `.` (PEG).
    Dot,
}

/// A token the grammars read. Values are the grammars' own: strings are converted as the grammar's
/// value converter converts them, and the header terminals are the common converter's texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// The header keyword of the grammar.
    Header,
    /// `TITLE`.
    Title(String),
    /// `ACC_TITLE`.
    AccTitle(String),
    /// `ACC_DESCR`.
    AccDescr(String),
    /// A rule name or a nonterminal, as written.
    Id(String),
    /// A string terminal, converted.
    Str(String),
    /// EBNF `? … ?`, trimmed.
    SpecialSeq(String),
    /// ABNF `%x41-5A` and the like, as written.
    NumVal(String),
    /// ABNF `*`, `1*3`, `3`, as written.
    Repeat(String),
    /// An IR expression keyword.
    Word(Word),
    /// A punctuation mark.
    Sym(Sym),
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// What one step of the scanner read: a token or hidden text, with its length in bytes.
type Step = (Option<Token>, usize);

/// Reads the whole text. A character no rule matches is a lexer error at its position.
pub(crate) fn scan(
    syntax: Syntax,
    text: &str,
    source: Source<'_>,
) -> Result<Vec<Spanned>, ParseError> {
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let Some((token, len)) = step(syntax, &text[at..]) else {
            return Err(source.error(at, "unexpected character"));
        };
        if let Some(token) = token {
            tokens.push(Spanned { token, start: at });
        }
        at += len;
    }
    Ok(tokens)
}

/// The first rule that matches at the start of `rest`: keywords, then terminals, then hidden tokens.
fn step(syntax: Syntax, rest: &str) -> Option<Step> {
    keyword(syntax, rest)
        .or_else(|| terminal(syntax, rest))
        .or_else(|| hidden(syntax, rest))
}

/// The keywords of the grammar in their order, with the `LONGER_ALT` alternatives taken when they
/// match more text. The header carries the boundary `(?=%%)|(?!\S)`; the word keywords of IR are
/// taken only when the identifier at the position is the word itself, since the identifier is their
/// `LONGER_ALT`.
fn keyword(syntax: Syntax, rest: &str) -> Option<Step> {
    if let Some(len) = header_len(rest, syntax.header()) {
        return Some((Some(Token::Header), len));
    }
    if syntax == Syntax::Ir {
        for (text, word) in IR_WORDS {
            if ir_name_len(rest) == text.len() && rest.starts_with(text) {
                return Some((Some(Token::Word(word)), text.len()));
            }
        }
    }
    for &(literal, sym) in punctuation(syntax) {
        if !rest.starts_with(literal) {
            continue;
        }
        // The `LONGER_ALT` of the punctuation: the alternative wins when it matches more text.
        match (syntax, literal) {
            (Syntax::Ebnf, "(") => {
                if let Some(len) = iso_comment_len(rest) {
                    return Some((None, len));
                }
            }
            (Syntax::Ebnf, "?") => {
                if let Some(len) = special_sequence(rest) {
                    return Some((
                        Some(Token::SpecialSeq(js_trim(&rest[1..len - 1]).to_owned())),
                        len,
                    ));
                }
            }
            (Syntax::Ebnf, "-") => {
                if let Some(len) = yaml_block_len(rest) {
                    return Some((None, len));
                }
            }
            (Syntax::Abnf, ";") => {
                let len = 1 + line_len(&rest[1..]);
                if len > 1 {
                    return Some((None, len));
                }
            }
            _ => {}
        }
        return Some((Some(Token::Sym(sym)), literal.len()));
    }
    None
}

const IR_WORDS: [(&str, Word); 8] = [
    ("nonterminal", Word::Nonterminal),
    ("zeroOrMore", Word::ZeroOrMore),
    ("oneOrMore", Word::OneOrMore),
    ("sequence", Word::Sequence),
    ("optional", Word::Optional),
    ("terminal", Word::Terminal),
    ("special", Word::Special),
    ("choice", Word::Choice),
];

/// The punctuation keywords of each grammar, longest first where one is a prefix of another.
fn punctuation(syntax: Syntax) -> &'static [(&'static str, Sym)] {
    match syntax {
        Syntax::Ir => &[
            ("=", Sym::Eq),
            (";", Sym::Semi),
            ("(", Sym::LParen),
            (",", Sym::Comma),
            (")", Sym::RParen),
        ],
        Syntax::Ebnf => &[
            ("::=", Sym::ColonEq),
            ("=", Sym::Eq),
            (";", Sym::Semi),
            ("|", Sym::Alt),
            (",", Sym::Comma),
            ("(", Sym::LParen),
            (")", Sym::RParen),
            ("[", Sym::LBracket),
            ("]", Sym::RBracket),
            ("{", Sym::LBrace),
            ("}", Sym::RBrace),
            ("?", Sym::Question),
            ("*", Sym::Star),
            ("+", Sym::Plus),
            ("-", Sym::Minus),
        ],
        Syntax::Abnf => &[
            ("=", Sym::Eq),
            (";", Sym::Semi),
            ("/", Sym::Alt),
            ("(", Sym::LParen),
            (")", Sym::RParen),
            ("[", Sym::LBracket),
            ("]", Sym::RBracket),
        ],
        Syntax::Peg => &[
            ("<-", Sym::Arrow),
            (";", Sym::Semi),
            ("/", Sym::Alt),
            ("&", Sym::Amp),
            ("!", Sym::Bang),
            ("?", Sym::Question),
            ("*", Sym::Star),
            ("+", Sym::Plus),
            ("(", Sym::LParen),
            (")", Sym::RParen),
            (".", Sym::Dot),
        ],
    }
}

/// The terminals of the grammar in their order: the common header terminals, then the grammar's own.
fn terminal(syntax: Syntax, rest: &str) -> Option<Step> {
    if let Some(len) = title_len(rest) {
        return Some((Some(Token::Title(title_value(syntax, &rest[..len]))), len));
    }
    if let Some((len, value)) = acc_title(rest) {
        return Some((Some(Token::AccTitle(value)), len));
    }
    if let Some((len, value)) = acc_descr(rest) {
        return Some((Some(Token::AccDescr(value)), len));
    }
    match syntax {
        Syntax::Ir | Syntax::Ebnf | Syntax::Peg => {
            if let Some(len) = ir_name_len_opt(rest) {
                return Some((Some(Token::Id(rest[..len].to_owned())), len));
            }
            let len = quoted_len(rest)?;
            Some((Some(Token::Str(decode_escaped(&rest[..len]))), len))
        }
        Syntax::Abnf => {
            if let Some(len) = abnf_name_len(rest) {
                return Some((Some(Token::Id(rest[..len].to_owned())), len));
            }
            if let Some(len) = abnf_string_len(rest) {
                return Some((Some(Token::Str(rest[1..len - 1].to_owned())), len));
            }
            if let Some(len) = numval_len(rest) {
                return Some((Some(Token::NumVal(rest[..len].to_owned())), len));
            }
            let len = repeat_len(rest)?;
            Some((Some(Token::Repeat(rest[..len].to_owned())), len))
        }
    }
}

/// The hidden tokens: white space, a YAML block, a directive, a comment line, a block comment (IR and
/// EBNF) and a hash comment (PEG). The `(*` of EBNF and the `;` of ABNF are hidden through their
/// keywords, and are read in `keyword`.
fn hidden(syntax: Syntax, rest: &str) -> Option<Step> {
    let first = rest.chars().next()?;
    if matches!(first, '\t' | ' ' | '\r' | '\n') {
        let len = rest.len() - rest.trim_start_matches(['\t', ' ', '\r', '\n']).len();
        return Some((None, len));
    }
    if rest.starts_with("---")
        && let Some(len) = yaml_block_len(rest)
    {
        return Some((None, len));
    }
    if rest.starts_with("%%{")
        && let Some(len) = directive_len(&rest[2..])
    {
        return Some((None, 2 + len));
    }
    if let Some(after) = rest.strip_prefix("%%") {
        return Some((None, 2 + line_len(after)));
    }
    if matches!(syntax, Syntax::Ir | Syntax::Ebnf) && rest.starts_with("/*") {
        let close = rest[2..].find("*/")?;
        return Some((None, 2 + close + 2));
    }
    if syntax == Syntax::Peg && rest.starts_with('#') {
        return Some((None, line_len(rest)));
    }
    None
}

/// `header(?:(?=%%)|(?!\S))`: the header, followed by `%%`, a white space or the end of the text.
fn header_len(rest: &str, header: &str) -> Option<usize> {
    let after = rest.strip_prefix(header)?;
    let bounded = after.is_empty()
        || after.starts_with("%%")
        || after.chars().next().is_some_and(is_js_space);
    bounded.then_some(header.len())
}

/// The length of the identifier at the start of `rest` in IR, EBNF and PEG: `[A-Z_a-z][\w-]*`.
fn ir_name_len(rest: &str) -> usize {
    ir_name_len_opt(rest).unwrap_or(0)
}

fn ir_name_len_opt(rest: &str) -> Option<usize> {
    let first = rest.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    Some(first.len_utf8() + run_len(&rest[1..], |c| is_word(c) || c == '-'))
}

/// The length of the rule name of ABNF: `[A-Za-z][A-Za-z0-9-]*`.
fn abnf_name_len(rest: &str) -> Option<usize> {
    let first = rest.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    Some(1 + run_len(&rest[1..], |c| c.is_ascii_alphanumeric() || c == '-'))
}

/// The byte length of the longest run of characters that satisfy `keep` at the start of `text`.
fn run_len(text: &str, keep: impl Fn(char) -> bool) -> usize {
    text.char_indices()
        .find(|&(_, c)| !keep(c))
        .map_or(text.len(), |(index, _)| index)
}

/// `TITLE: title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)`: the keyword, then the rest of the line up to
/// a `%%` (or the whole line when there is none). The keyword alone is a title, so `titles` is the
/// title `title` and the identifier `s`.
fn title_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix("title")?;
    match after.as_bytes().first() {
        Some(b'\t' | b' ') => {
            let line = &after[..line_len(after)];
            let body = &line[1..];
            let consumed = 1 + body.find("%%").unwrap_or(body.len());
            Some(5 + consumed)
        }
        _ => Some(5),
    }
}

/// The value of a `TITLE` token: the converter's text (the text after the keyword, trimmed, with
/// runs of blanks collapsed), unquoted when it is quoted. The IR, EBNF and PEG grammars decode the
/// quoted text as their strings are decoded; ABNF keeps it as written.
fn title_value(syntax: Syntax, token: &str) -> String {
    let value = plain_text(&token[5..]);
    let trimmed = js_trim(&value);
    let quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''));
    if !quoted {
        return value;
    }
    match syntax {
        Syntax::Abnf => slice_quotes(trimmed).to_owned(),
        Syntax::Ir | Syntax::Ebnf | Syntax::Peg => decode_escaped(trimmed),
    }
}

/// `ACC_TITLE: accTitle[\t ]*:(?:[^\n\r]*?(?=%%)|[^\n\r]*)`: returns the length and the converted
/// text (the text after the colon, trimmed and collapsed).
fn acc_title(rest: &str) -> Option<(usize, String)> {
    let after = rest.strip_prefix("accTitle")?;
    colon_form(after).map(|(len, value)| (8 + len, value))
}

/// `ACC_DESCR: accDescr(?:[\t ]*:(…)|\s*{([^}]*)})`: the colon form as `ACC_TITLE`, or the braces
/// over any number of lines, whose text is the block converter's.
fn acc_descr(rest: &str) -> Option<(usize, String)> {
    let after = rest.strip_prefix("accDescr")?;
    if let Some((len, value)) = colon_form(after) {
        return Some((8 + len, value));
    }
    let spaces = after.len() - after.trim_start_matches(is_js_space).len();
    let braced = after[spaces..].strip_prefix('{')?;
    let close = braced.find('}')?;
    Some((
        8 + spaces + 1 + close + 1,
        descr_block_text(&braced[..close]),
    ))
}

/// `[\t ]*:` and the text after it, as the accessibility keywords read it: the length of the match
/// after the keyword, and the converted text.
fn colon_form(after: &str) -> Option<(usize, String)> {
    let blanks = after.len() - after.trim_start_matches(['\t', ' ']).len();
    let text = after[blanks..].strip_prefix(':')?;
    let line = &text[..line_len(text)];
    let consumed = line.find("%%").unwrap_or(line.len());
    Some((blanks + 1 + consumed, plain_text(&text[..consumed])))
}

/// `"([^"\\]|\\.)*"` or `'([^'\\]|\\.)*'`: the length of a quoted string, where a backslash takes the
/// character after it, which may not be a line break. A line break inside the quotes is allowed.
fn quoted_len(rest: &str) -> Option<usize> {
    let quote = rest.chars().next().filter(|&c| c == '"' || c == '\'')?;
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

/// `"[^"]*"` of ABNF: a string with no escapes, which may span line breaks.
fn abnf_string_len(rest: &str) -> Option<usize> {
    if !rest.starts_with('"') {
        return None;
    }
    rest[1..].find('"').map(|close| close + 2)
}

/// `%[xXdDbB][0-9A-Fa-f]+(?:-[0-9A-Fa-f]+|\.[0-9A-Fa-f]+)*`: an ABNF numeric value.
fn numval_len(rest: &str) -> Option<usize> {
    let mut chars = rest.chars();
    if chars.next() != Some('%') || !matches!(chars.next(), Some('x' | 'X' | 'd' | 'D' | 'b' | 'B'))
    {
        return None;
    }
    let digits = hex_run(&rest[2..]);
    if digits == 0 {
        return None;
    }
    let mut len = 2 + digits;
    loop {
        let after = &rest[len..];
        let Some(separator) = after.chars().next().filter(|&c| c == '-' || c == '.') else {
            return Some(len);
        };
        let run = hex_run(&after[1..]);
        if run == 0 {
            return Some(len);
        }
        len += separator.len_utf8() + run;
    }
}

fn hex_run(text: &str) -> usize {
    run_len(text, |c| c.is_ascii_hexdigit())
}

/// `[0-9]*\*[0-9]*` or else `[0-9]+`: an ABNF repeat, as written.
fn repeat_len(rest: &str) -> Option<usize> {
    let digits = run_len(rest, |c| c.is_ascii_digit());
    if rest[digits..].starts_with('*') {
        let after = &rest[digits + 1..];
        return Some(digits + 1 + run_len(after, |c| c.is_ascii_digit()));
    }
    (digits > 0).then_some(digits)
}

/// `\(\*[\s\S]*?\*\)`: an EBNF comment in ISO form, up to the first `*)`.
fn iso_comment_len(rest: &str) -> Option<usize> {
    let close = rest.strip_prefix("(*")?.find("*)")?;
    Some(2 + close + 2)
}

/// `\?(?=[^?;]*[^?\s;][^?;]*\?)[^?;]*\?`: an EBNF special sequence. The text between the question
/// marks has no `?` and no `;`, and holds a character that is not white space. Returns its length.
fn special_sequence(rest: &str) -> Option<usize> {
    let inner = rest.strip_prefix('?')?;
    let end = inner.find(['?', ';'])?;
    if !inner[end..].starts_with('?') {
        return None;
    }
    let between = &inner[..end];
    between
        .chars()
        .any(|c| !is_js_space(c))
        .then_some(1 + end + 1)
}

/// `---[\t ]*\r?\n(?:[\S\s]*?\r?\n)?---(?:\r?\n|(?!\S))`: a YAML block, which is hidden.
fn yaml_block_len(rest: &str) -> Option<usize> {
    rest.starts_with("---")
        .then(|| yaml_len(&rest[3..]))
        .flatten()
        .map(|len| 3 + len)
}

/// The length of the text up to, not including, the next `\n` or `\r`: the `[^\n\r]*` of the
/// reference's comment, title and accessibility patterns, which a U+2028 does not stop.
fn line_len(text: &str) -> usize {
    text.find(is_cr_lf).unwrap_or(text.len())
}

/// JavaScript's `slice(1, -1)` of a text whose first and last characters are ASCII quotes, or the
/// empty text when there is one character or none.
fn slice_quotes(quoted: &str) -> &str {
    if quoted.len() < 2 {
        return "";
    }
    &quoted[1..quoted.len() - 1]
}

/// The decoding of the IR, EBNF and PEG strings and quoted titles: the quotes are removed, `\n`,
/// `\r` and `\t` become their characters, and any other escaped character stands for itself. A
/// backslash at the very end stands for itself.
pub(crate) fn decode_escaped(quoted: &str) -> String {
    let content = slice_quotes(quoted);
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(syntax: Syntax, text: &str) -> Vec<Token> {
        scan(syntax, text, Source::new(text, 0))
            .expect("lexes")
            .into_iter()
            .map(|spanned| spanned.token)
            .collect()
    }

    #[test]
    fn the_header_needs_a_boundary() {
        assert_eq!(tokens(Syntax::Ir, "railroad-beta\n")[0], Token::Header);
        assert_eq!(tokens(Syntax::Ir, "railroad-beta%%c")[0], Token::Header);
        assert_eq!(
            tokens(Syntax::Ir, "railroad-betax")[0],
            Token::Id("railroad-betax".into())
        );
    }

    #[test]
    fn a_title_keyword_eats_the_prefix_of_a_word() {
        assert_eq!(tokens(Syntax::Ir, "titles")[0], Token::Title(String::new()));
    }

    #[test]
    fn an_abnf_semicolon_before_text_is_a_comment() {
        assert_eq!(
            tokens(Syntax::Abnf, "a = \"x\";y"),
            vec![
                Token::Id("a".into()),
                Token::Sym(Sym::Eq),
                Token::Str("x".into()),
            ]
        );
        assert_eq!(
            tokens(Syntax::Abnf, "a = \"x\";\n")[3],
            Token::Sym(Sym::Semi)
        );
    }

    #[test]
    fn an_ir_string_decodes_only_three_escapes() {
        assert_eq!(decode_escaped(r#""a\bc\nd""#), "a\u{62}c\nd");
    }
}
