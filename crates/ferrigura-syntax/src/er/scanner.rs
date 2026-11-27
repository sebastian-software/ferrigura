//! The ER lexer: the rules of `erDiagram.jison` with its start conditions, in the grammar's order.
//! The first rule that matches wins, and the keyword literals are case-insensitive with an implicit
//! word boundary. The end of the input follows Jison's `next()`: the rules run once more on the
//! empty rest, and the end marker follows. The quirks of the rules are in
//! `docs/diagrams/er.md#parsing`.

use crate::scan::{StateStack, line_len, space_len};
use crate::text::{is_js_space, is_line_term, is_word};

/// The start conditions of the ER lexer (the states of `erDiagram.jison`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    AccTitle,
    AccDescr,
    AccDescrMulti,
    Block,
    BlockBq,
    Style,
}

/// The token kinds of the ER grammar. `Literal` is a character that no rule names: it reaches the
/// parser only inside a block, where `,` and `?` are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The string `EOF` of the grammar: the end of the input in `INITIAL`.
    Eof,
    /// The end marker of Jison (`$end`), which follows `Eof`, or the end of an open state.
    End,
    ErDiagram,
    Newline,
    AccTitle,
    AccTitleValue,
    AccDescr,
    AccDescrValue,
    AccDescrMultiValue,
    DirectionTb,
    DirectionBt,
    DirectionRl,
    DirectionLr,
    /// A quoted name, quotes included.
    EntityName,
    /// A quoted text, quotes included (a role).
    Word,
    UnicodeText,
    Num,
    DecimalNum,
    EntityOne,
    BlockStart,
    BlockStop,
    Brkt,
    Comma,
    StyleSeparator,
    Colon,
    Sqs,
    Sqe,
    Style,
    ClassDef,
    Class,
    Subgraph,
    /// The keyword `end`, with the blanks after it.
    KwEnd,
    StyleText,
    Semi,
    AttributeKey,
    AttributeWord,
    /// A quoted comment, quotes included.
    Comment,
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
    OnlyOne,
    MdParent,
    NonIdentifying,
    Identifying,
    Literal(char),
}

/// One token: its kind, its text as the lexer matched it, and its byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: Kind,
    pub value: String,
    pub start: usize,
}

/// No rule matches at byte `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexError {
    pub offset: usize,
}

/// What a matching rule produced: how far it reads, and the token it gives, if any.
struct Hit {
    len: usize,
    kind: Option<Kind>,
}

fn hit(len: usize, kind: Kind) -> Option<Hit> {
    Some(Hit {
        len,
        kind: Some(kind),
    })
}

fn silent(len: usize) -> Option<Hit> {
    Some(Hit { len, kind: None })
}

/// The direction words of `.*direction\s+WORD[^\n]*`, in the grammar's order.
const DIRECTION_WORDS: [(&str, Kind); 4] = [
    ("TB", Kind::DirectionTb),
    ("BT", Kind::DirectionBt),
    ("RL", Kind::DirectionRl),
    ("LR", Kind::DirectionLr),
];

/// Precomputed line boundaries, `direction\s+WORD` occurrences, and tilde/space positions so
/// per-token rules run in O(log N) instead of rescanning the rest of the line.
struct Index {
    len: usize,
    line_breaks: Vec<usize>,
    directions: [Vec<(usize, usize)>; 4],
    spaces: Vec<usize>,
    tildes: Vec<usize>,
}

impl Index {
    fn new(text: &str) -> Self {
        let has_tilde = text.contains('~');
        let mut line_breaks = Vec::new();
        let mut directions: [Vec<(usize, usize)>; 4] = Default::default();
        let mut spaces = Vec::new();
        let mut tildes = Vec::new();
        let bytes = text.as_bytes();
        for (index, c) in text.char_indices() {
            if is_line_term(c) {
                line_breaks.push(index);
            }
            if has_tilde {
                if is_js_space(c) {
                    spaces.push(index);
                } else if c == '~' {
                    tildes.push(index);
                }
            }
            if matches!(c, 'd' | 'D')
                && bytes
                    .get(index..index + 9)
                    .is_some_and(|slice| slice.eq_ignore_ascii_case(b"direction"))
            {
                let after = index + 9;
                let ws = space_len(&text[after..]);
                if ws > 0 {
                    let word_start = after + ws;
                    for ((word, _), occ) in DIRECTION_WORDS.iter().zip(directions.iter_mut()) {
                        if starts_with_ci(&text[word_start..], word) {
                            occ.push((index, word_start + word.len()));
                        }
                    }
                }
            }
        }
        Self {
            len: text.len(),
            line_breaks,
            directions,
            spaces,
            tildes,
        }
    }

    fn next_line_term(&self, pos: usize) -> usize {
        self.line_breaks
            .get(self.line_breaks.partition_point(|&b| b < pos))
            .copied()
            .unwrap_or(self.len)
    }

    /// `.*direction\s+WORD[^\n]*` at byte offset `pos`.
    fn direction_len(&self, pos: usize, rest: &str, word_index: usize) -> Option<usize> {
        let occurrences = &self.directions[word_index];
        if occurrences.is_empty() {
            return None;
        }
        let line_end = self.next_line_term(pos);
        let upper = occurrences.partition_point(|&(start, _)| start < line_end);
        let &(start, end) = occurrences[..upper].last()?;
        if start < pos {
            return None;
        }
        let rel_end = end - pos;
        Some(rel_end + line_len(&rest[rel_end..]))
    }

    /// `([^\s]*)[~].*[~]([^\s]*)` at byte offset `pos`.
    fn generic_len(&self, pos: usize) -> Option<usize> {
        if self.tildes.len() < 2 {
            return None;
        }
        let run_end = self
            .spaces
            .get(self.spaces.partition_point(|&s| s < pos))
            .copied()
            .unwrap_or(self.len);
        let first_lo = self.tildes.partition_point(|&t| t < pos);
        let first_hi = self.tildes.partition_point(|&t| t < run_end);
        for &first in self.tildes[first_lo..first_hi].iter().rev() {
            let line_end = self.next_line_term(first + 1);
            let second_hi = self.tildes.partition_point(|&t| t < line_end);
            let Some(&second) = self.tildes[..second_hi].last() else {
                continue;
            };
            if second <= first {
                continue;
            }
            let tail_end = self
                .spaces
                .get(self.spaces.partition_point(|&s| s < second + 1))
                .copied()
                .unwrap_or(self.len);
            return Some(tail_end - pos);
        }
        None
    }
}

/// Tokenizes the ER text. The last token is always [`Kind::End`].
pub(crate) fn tokenize(text: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer {
        states: StateStack::new(State::Initial),
        done: false,
        index: Index::new(text),
    };
    let mut tokens = Vec::new();
    let mut pos = 0;
    loop {
        if lexer.done {
            tokens.push(end_token(text.len()));
            return Ok(tokens);
        }
        if pos >= text.len() {
            lexer.done = true;
        }
        let rest = &text[pos..];
        let state = lexer.states.top();
        let Some(hit) = lexer.rule(pos, rest, state) else {
            if rest.is_empty() {
                tokens.push(end_token(text.len()));
                return Ok(tokens);
            }
            return Err(LexError { offset: pos });
        };
        let start = pos;
        pos += hit.len;
        if let Some(kind) = hit.kind {
            tokens.push(Token {
                kind,
                value: text[start..pos].to_owned(),
                start,
            });
        }
    }
}

fn end_token(offset: usize) -> Token {
    Token {
        kind: Kind::End,
        value: String::new(),
        start: offset,
    }
}

struct Lexer {
    states: StateStack<State>,
    /// Set once the input is used up, as Jison's `done`: the next call returns the end marker.
    done: bool,
    index: Index,
}

/// The rules after the accessibility, direction and whitespace rules of `INITIAL`, for the
/// cardinalities written as words: literal, and whether a word boundary follows it.
const CARDINALITY_WORDS: &[(&str, bool, Kind)] = &[
    ("one or zero", true, Kind::ZeroOrOne),
    ("one or more", true, Kind::OneOrMore),
    ("one or many", true, Kind::OneOrMore),
    ("1+", false, Kind::OneOrMore),
    ("|o", false, Kind::ZeroOrOne),
    ("zero or one", true, Kind::ZeroOrOne),
    ("zero or more", true, Kind::ZeroOrMore),
    ("zero or many", true, Kind::ZeroOrMore),
    ("0+", false, Kind::ZeroOrMore),
    ("}o", false, Kind::ZeroOrMore),
    ("many(0)", false, Kind::ZeroOrMore),
    ("many(1)", false, Kind::OneOrMore),
    ("many", true, Kind::ZeroOrMore),
    ("}|", false, Kind::OneOrMore),
    ("one", true, Kind::OnlyOne),
    ("only one", true, Kind::OnlyOne),
];

/// The rules after the number rules: the symbols of the cardinalities and the identification.
const SYMBOL_WORDS: &[(&str, bool, Kind)] = &[
    ("||", false, Kind::OnlyOne),
    ("o|", false, Kind::ZeroOrOne),
    ("o{", false, Kind::ZeroOrMore),
    ("|{", false, Kind::OneOrMore),
    ("..", false, Kind::NonIdentifying),
    ("--", false, Kind::Identifying),
    ("to", true, Kind::Identifying),
    ("optionally to", true, Kind::NonIdentifying),
    (".-", false, Kind::NonIdentifying),
    ("-.", false, Kind::NonIdentifying),
];

impl Lexer {
    /// The first rule of the current state that matches at the start of `rest`.
    fn rule(&mut self, pos: usize, rest: &str, state: State) -> Option<Hit> {
        match state {
            State::Initial => self.initial(pos, rest),
            State::AccTitle => {
                self.states.pop();
                hit(line_len(rest), Kind::AccTitleValue)
            }
            State::AccDescr => {
                self.states.pop();
                hit(line_len(rest), Kind::AccDescrValue)
            }
            State::AccDescrMulti => {
                if rest.starts_with('}') {
                    self.states.pop();
                    return silent(1);
                }
                let len = rest.find('}').unwrap_or(rest.len());
                hit(len, Kind::AccDescrMultiValue)
            }
            State::Block => self.block(pos, rest),
            State::BlockBq => {
                if rest.starts_with('`') {
                    self.states.pop();
                    return silent(1);
                }
                let len = rest.find('`').unwrap_or(rest.len());
                if len == 0 {
                    return None;
                }
                hit(len, Kind::AttributeWord)
            }
            State::Style => self.style(rest),
        }
    }

    /// The rules of `INITIAL`, in the order of the grammar.
    fn initial(&mut self, pos: usize, rest: &str) -> Option<Hit> {
        if let Some(len) = delimited_len(rest, "accTitle", ':') {
            self.states.push(State::AccTitle);
            return hit(len, Kind::AccTitle);
        }
        if let Some(len) = delimited_len(rest, "accDescr", ':') {
            self.states.push(State::AccDescr);
            return hit(len, Kind::AccDescr);
        }
        if let Some(len) = delimited_len(rest, "accDescr", '{') {
            self.states.push(State::AccDescrMulti);
            return silent(len);
        }
        for (word_index, &(_, kind)) in DIRECTION_WORDS.iter().enumerate() {
            if let Some(len) = self.index.direction_len(pos, rest, word_index) {
                return hit(len, kind);
            }
        }
        let blanks = rest.len() - rest.trim_start_matches([' ', '\t', '\r']).len();
        if blanks > 0 {
            return silent(blanks);
        }
        if rest.starts_with('\n') {
            return hit(
                rest.len() - rest.trim_start_matches('\n').len(),
                Kind::Newline,
            );
        }
        if let Some(len) = quoted_name_len(rest) {
            return hit(len, Kind::EntityName);
        }
        if let Some(len) = quoted_word_len(rest) {
            return hit(len, Kind::Word);
        }
        if let Some(len) = word_len(rest, "erDiagram") {
            return hit(len, Kind::ErDiagram);
        }
        if rest.starts_with('{') {
            self.states.push(State::Block);
            return hit(1, Kind::BlockStart);
        }
        if rest.starts_with('#') {
            return hit(1, Kind::Brkt);
        }
        if rest.starts_with(',') {
            return hit(1, Kind::Comma);
        }
        if rest.starts_with(":::") {
            return hit(3, Kind::StyleSeparator);
        }
        if rest.starts_with(':') {
            return hit(1, Kind::Colon);
        }
        if rest.starts_with('[') {
            return hit(1, Kind::Sqs);
        }
        if rest.starts_with(']') {
            return hit(1, Kind::Sqe);
        }
        if let Some(len) = word_len(rest, "style") {
            self.states.push(State::Style);
            return hit(len, Kind::Style);
        }
        if let Some(len) = word_len(rest, "classDef") {
            self.states.push(State::Style);
            return hit(len, Kind::ClassDef);
        }
        if let Some(len) = word_len(rest, "class") {
            return hit(len, Kind::Class);
        }
        if let Some(len) = word_len(rest, "subgraph") {
            return hit(len, Kind::Subgraph);
        }
        if let Some(len) = word_len(rest, "end") {
            let blanks = space_len(&rest[len..]);
            return hit(len + blanks, Kind::KwEnd);
        }
        for (literal, boundary, kind) in CARDINALITY_WORDS {
            if let Some(len) = literal_len(rest, literal, *boundary) {
                return hit(len, *kind);
            }
        }
        if let Some(len) = decimal_len(rest) {
            return hit(len, Kind::DecimalNum);
        }
        if let Some(kind) = one_kind(rest) {
            return hit(1, kind);
        }
        let digits = digits_len(rest);
        if digits > 0 {
            return hit(digits, Kind::Num);
        }
        for (literal, boundary, kind) in SYMBOL_WORDS.iter().take(4) {
            if let Some(len) = literal_len(rest, literal, *boundary) {
                return hit(len, *kind);
            }
        }
        if let Some(len) = md_parent_len(rest) {
            return hit(len, Kind::MdParent);
        }
        for (literal, boundary, kind) in SYMBOL_WORDS.iter().skip(4) {
            if let Some(len) = literal_len(rest, literal, *boundary) {
                return hit(len, *kind);
            }
        }
        let name = unicode_text_len(rest);
        if name > 0 {
            return hit(name, Kind::UnicodeText);
        }
        // `<<EOF>>`, and then the `.` rule: any other character is a literal token.
        let Some(first) = rest.chars().next() else {
            return hit(0, Kind::Eof);
        };
        hit(first.len_utf8(), Kind::Literal(first))
    }

    /// The rules of the `style` state: the line break ends it, the rest of the statement follows.
    fn style(&mut self, rest: &str) -> Option<Hit> {
        if rest.starts_with('\n') {
            self.states.pop();
            return hit(
                rest.len() - rest.trim_start_matches('\n').len(),
                Kind::Newline,
            );
        }
        let blanks = rest.len() - rest.trim_start_matches(is_js_space).len();
        if blanks > 0 {
            return silent(blanks);
        }
        if rest.starts_with(':') {
            return hit(1, Kind::Colon);
        }
        if rest.starts_with(',') {
            return hit(1, Kind::Comma);
        }
        if rest.starts_with('#') {
            return hit(1, Kind::Brkt);
        }
        let text = style_text_len(rest);
        if text > 0 {
            return hit(text, Kind::StyleText);
        }
        if rest.starts_with(';') {
            return hit(1, Kind::Semi);
        }
        None
    }
}

impl Lexer {
    /// The rules of the `block` state: names, keys, comments, and the closing brace.
    fn block(&mut self, pos: usize, rest: &str) -> Option<Hit> {
        let blanks = rest.len() - rest.trim_start_matches(is_js_space).len();
        if blanks > 0 {
            return silent(blanks);
        }
        for key in ["PK", "FK", "UK"] {
            if let Some(len) = word_len(rest, key) {
                return hit(len, Kind::AttributeKey);
            }
        }
        if let Some(len) = self.index.generic_len(pos) {
            return hit(len, Kind::AttributeWord);
        }
        if let Some(len) = attribute_word_len(rest) {
            return hit(len, Kind::AttributeWord);
        }
        if rest.starts_with('`') {
            self.states.push(State::BlockBq);
            return silent(1);
        }
        if let Some(len) = quoted_word_len(rest) {
            return hit(len, Kind::Comment);
        }
        if rest.starts_with('}') {
            self.states.pop();
            return hit(1, Kind::BlockStop);
        }
        let first = rest.chars().next()?;
        hit(first.len_utf8(), Kind::Literal(first))
    }
}

/// `accTitle\s*":"\s*` and its siblings: the word, blanks, the delimiter, and blanks, in any case.
fn delimited_len(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after_word = &rest[word.len()..];
    let spaces = space_len(after_word);
    let after_delimiter = after_word[spaces..].strip_prefix(delimiter)?;
    let trailing = space_len(after_delimiter);
    Some(word.len() + spaces + delimiter.len_utf8() + trailing)
}

/// `"[^"%\r\n\v\b\\]+"`: a quoted name without the characters that would break the parser.
fn quoted_name_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix('"')?;
    let mut len = 0;
    for c in body.chars() {
        if c == '"' {
            return (len > 0).then_some(len + 2);
        }
        if matches!(c, '%' | '\r' | '\n' | '\u{b}' | '\u{8}' | '\\') {
            return None;
        }
        len += c.len_utf8();
    }
    None
}

/// `"[^"]*"`: a quoted text, which may hold anything but a quote, line breaks included.
fn quoted_word_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix('"')?;
    body.find('"').map(|end| end + 2)
}

/// A word that Jison matches with an implicit boundary: the literal, ASCII case-insensitively, and
/// no word character after it.
fn word_len(rest: &str, word: &str) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after = rest.get(word.len()..)?;
    (!after.chars().next().is_some_and(is_word)).then_some(word.len())
}

/// A literal, ASCII case-insensitively, with the boundary when the rule has one.
fn literal_len(rest: &str, literal: &str, boundary: bool) -> Option<usize> {
    if !starts_with_ci(rest, literal) {
        return None;
    }
    if !boundary {
        return Some(literal.len());
    }
    word_len(rest, literal)
}

/// Whether `text` starts with `literal`, comparing ASCII letters without case.
fn starts_with_ci(text: &str, literal: &str) -> bool {
    text.as_bytes()
        .get(..literal.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(literal.as_bytes()))
}

/// `[0-9]+\.[0-9]+`.
fn decimal_len(rest: &str) -> Option<usize> {
    let whole = digits_len(rest);
    if whole == 0 {
        return None;
    }
    let fraction = rest[whole..].strip_prefix('.')?;
    let fraction_len = digits_len(fraction);
    (fraction_len > 0).then_some(whole + 1 + fraction_len)
}

fn digits_len(rest: &str) -> usize {
    rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len()
}

/// The rules `"1"(?=\s+[A-Za-z_"'])`, `"1"(?=\s+[0-9])`, `"1"(?=(\-\-|\.\.|\.\-|\-\.))` and `"1"\b`:
/// a `1` is a cardinality before a blank and a letter, quote or underscore, before a blank and a
/// digit, or before `--`, `..`, `.-`, `-.`. Otherwise it is the entity name `1`, when no word
/// character follows it. `None` leaves the `1` to the number rule.
fn one_kind(rest: &str) -> Option<Kind> {
    let after = rest.strip_prefix('1')?;
    let spaces = space_len(after);
    let next = after[spaces..].chars().next();
    if spaces > 0
        && next.is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '"' | '\''))
    {
        return Some(Kind::OnlyOne);
    }
    if ["--", "..", ".-", "-."]
        .iter()
        .any(|dash| after.starts_with(dash))
    {
        return Some(Kind::OnlyOne);
    }
    if after.chars().next().is_some_and(is_word) {
        return None;
    }
    Some(Kind::EntityOne)
}

/// `u(?=[.\-|])`: the parent side of a parent-child relationship.
fn md_parent_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix(['u', 'U'])?;
    after.starts_with(['.', '-', '|']).then_some(1)
}

/// `([^\x00-\x7F]|\w|\-|\*|\.)+`: a run of unquoted name characters.
fn unicode_text_len(rest: &str) -> usize {
    rest.char_indices()
        .find(|&(_, c)| !(!c.is_ascii() || is_word(c) || matches!(c, '-' | '*' | '.')))
        .map_or(rest.len(), |(index, _)| index)
}

/// `([^\x00-\x7F]|\w|\-|\*)+` in the `style` state: no dot.
fn style_text_len(rest: &str) -> usize {
    rest.char_indices()
        .find(|&(_, c)| !(!c.is_ascii() || is_word(c) || matches!(c, '-' | '*')))
        .map_or(rest.len(), |(index, _)| index)
}

/// `[\*A-Za-z_À-￿][A-Za-z0-9\-\_\[\]\(\)\.,À-￿\*]*`.
fn attribute_word_len(rest: &str) -> Option<usize> {
    let mut chars = rest.char_indices();
    let (_, first) = chars.next()?;
    if !(first == '*' || first == '_' || first.is_ascii_alphabetic() || first >= '\u{c0}') {
        return None;
    }
    let mut end = first.len_utf8();
    for (index, c) in chars {
        if !(c.is_ascii_alphanumeric()
            || matches!(c, '-' | '_' | '[' | ']' | '(' | ')' | '.' | ',' | '*')
            || c >= '\u{c0}')
        {
            break;
        }
        end = index + c.len_utf8();
    }
    Some(end)
}
