//! The requirement scanner: the rules of `requirementDiagram.jison`, in the grammar's order. Jison
//! takes the first rule that matches at the current position, not the longest. The rules that
//! have no start condition run in the initial state only; `<*>` rules run in every state. A
//! keyword literal ends with an implicit word boundary. The rule table is in
//! `docs-port/requirement/PLAN.md`, section 3.
//!
//! The scanner reproduces Jison's `next()` and its `done` flag: once the input is empty, every
//! later call returns Jison's raw end symbol, which the grammar does not accept (`RawEof`).

use ferrigura_model::requirement::{Direction, RelationshipKind, RequirementKind};

use crate::error::{ParseError, Source};
use crate::scan::{StateStack, line_len};
use crate::text::{is_js_space, is_word, js_trim};

/// The start conditions of the lexer. `Initial` is the base state; the others are pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    /// `style`, `classDef` and `class` enter it; a line break leaves it.
    Style,
    /// A double-quoted string, entered by `"`.
    Str,
    /// `accTitle:`; its value is the rest of the line.
    AccTitle,
    /// `accDescr:`; its value is the rest of the line.
    AccDescr,
    /// `accDescr {`; its value runs up to the `}`.
    AccDescrMulti,
}

/// The tokens of the grammar. The grammar's names are in the variants' docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `NEWLINE`: one or more line breaks.
    Newline,
    /// `EOF`: the end of the input, read in the initial state.
    Eof,
    /// Jison's raw end symbol, returned once the input is done. The grammar does not accept it.
    RawEof,
    /// `title`: a title in the body. No production takes it, so it is an error.
    Title,
    /// `acc_title`.
    AccTitle,
    /// `acc_title_value`: the rest of the line after `accTitle:`.
    AccTitleValue(String),
    /// `acc_descr`.
    AccDescr,
    /// `acc_descr_value`: the rest of the line after `accDescr:`.
    AccDescrValue(String),
    /// `acc_descr_multiline_value`: the text of an `accDescr { … }` block.
    AccDescrMultiValue(String),
    /// `direction_tb`, `direction_bt`, `direction_rl`, `direction_lr`.
    Direction(Direction),
    /// `RD`: `requirementDiagram`.
    Rd,
    /// `STRUCT_START`: `{`.
    StructStart,
    /// `STRUCT_STOP`: `}`.
    StructStop,
    /// `STYLE_SEPARATOR`: `:::`.
    StyleSeparator,
    /// `COLONSEP`: `:` outside a style.
    ColonSep,
    /// `ID`.
    IdKw,
    /// `TEXT`.
    TextKw,
    /// `RISK`.
    RiskKw,
    /// `VERIFYMTHD`.
    VerifyMethodKw,
    /// The six requirement kinds.
    Kind(RequirementKind),
    /// `LOW_RISK`, `MED_RISK` or `HIGH_RISK`, with the DB's display string.
    Level(&'static str),
    /// `VERIFY_*`, with the DB's display string.
    Verify(&'static str),
    /// `ELEMENT`.
    ElementKw,
    /// The seven relationship keywords.
    Relation(RelationshipKind),
    /// `TYPE`.
    TypeKw,
    /// `DOCREF`.
    DocRefKw,
    /// `STYLE`.
    StyleKw,
    /// `CLASSDEF`.
    ClassDefKw,
    /// `CLASS`.
    ClassKw,
    /// `END_ARROW_L`: `<-`.
    EndArrowL,
    /// `END_ARROW_R`: `->`.
    EndArrowR,
    /// `LINE`: `-`.
    Line,
    /// `unqString`: a run of characters that starts with a word character, trimmed.
    Word(String),
    /// `qString`: the text between two double quotes.
    Quoted(String),
    /// `ALPHA` in a style: a run of word characters.
    Alpha(String),
    /// `COLON` in a style.
    StyleColon,
    /// `SEMICOLON` in a style.
    Semicolon,
    /// `PERCENT` in a style. No rule of the grammar takes it.
    Percent,
    /// `MINUS` in a style.
    StyleMinus,
    /// `BRKT` in a style: `#`.
    StyleHash,
    /// `COMMA`.
    Comma,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// What one rule did: a token of a byte length, a length consumed without a token, or no match.
enum Step {
    Token(Token, usize),
    Skip(usize),
    NoMatch,
}

/// Reads the whole text. The result ends with `Eof` or `RawEof`.
pub(crate) fn scan(source: Source<'_>) -> Result<Vec<Spanned>, ParseError> {
    let mut scanner = Scanner {
        text: source.text(),
        pos: 0,
        states: StateStack::new(State::Initial),
        done: false,
    };
    let mut tokens = Vec::new();
    loop {
        let spanned = scanner.next_token(source)?;
        let is_end = matches!(spanned.token, Token::Eof | Token::RawEof);
        tokens.push(spanned);
        if is_end {
            return Ok(tokens);
        }
    }
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    states: StateStack<State>,
    done: bool,
}

/// Whether `rest` starts with `word`, ignoring ASCII case (the `i` flag).
fn starts_ci(rest: &str, word: &str) -> bool {
    rest.as_bytes()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word.as_bytes()))
}

/// Byte length of the run of JavaScript white space at the start of `text`, line breaks included.
fn space_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// Byte length of the run of characters before the first one that `stop` takes.
fn run_until(text: &str, stop: impl Fn(char) -> bool) -> usize {
    text.find(stop).unwrap_or(text.len())
}

/// `word\s*delimiter\s*` (the `accTitle\s*":"\s*` and `accDescr\s*"{"\s*` rules), ignoring the
/// case of the word. The length of the match.
fn delimited_len(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    if !starts_ci(rest, word) {
        return None;
    }
    let after_word = &rest[word.len()..];
    let spaces = space_run(after_word);
    let after_delimiter = after_word[spaces..].strip_prefix(delimiter)?;
    Some(word.len() + spaces + delimiter.len_utf8() + space_run(after_delimiter))
}

/// `title\s[^#\n;]+`, the rule that takes a whole body line that starts with `title `.
fn title_len(rest: &str) -> Option<usize> {
    if !starts_ci(rest, "title") {
        return None;
    }
    let after = &rest[5..];
    let blank = after.chars().next().filter(|&c| is_js_space(c))?;
    let body = &after[blank.len_utf8()..];
    let run = run_until(body, |c| matches!(c, '#' | '\n' | ';'));
    (run > 0).then_some(5 + blank.len_utf8() + run)
}

/// `\s` runs of line breaks: `(\r?\n)+`. The length of the run.
fn newlines_len(rest: &str) -> Option<usize> {
    let mut len = 0;
    loop {
        if rest[len..].starts_with("\r\n") {
            len += 2;
        } else if rest[len..].starts_with('\n') {
            len += 1;
        } else {
            break;
        }
    }
    (len > 0).then_some(len)
}

/// Whether `c` is a character that `.` does not match in a JavaScript regular expression.
fn is_dot_stop(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// The rules `.*direction\s+TB[^\n]*`, then BT, RL and LR. The `.*` is greedy, so the match takes
/// the last `direction` of the line that the keyword follows; the keyword rules are tried in the
/// grammar's order. The match runs to the end of the line.
fn direction_len(rest: &str) -> Option<(Direction, usize)> {
    let line_end = run_until(rest, is_dot_stop);
    if !contains_ci(&rest[..line_end], "direction") {
        return None;
    }
    let candidates = [
        (Direction::Tb, "TB"),
        (Direction::Bt, "BT"),
        (Direction::Rl, "RL"),
        (Direction::Lr, "LR"),
    ];
    for (direction, keyword) in candidates {
        let last = (0..=line_end)
            .rev()
            .filter(|&start| rest.is_char_boundary(start))
            .find_map(|start| direction_keyword_end(rest, start, keyword));
        if let Some(keyword_end) = last {
            return Some((direction, keyword_end + line_len(&rest[keyword_end..])));
        }
    }
    None
}

/// Whether `text` contains `word`, ignoring ASCII case.
fn contains_ci(text: &str, word: &str) -> bool {
    text.char_indices()
        .any(|(start, _)| starts_ci(&text[start..], word))
}

/// `direction\s+KEYWORD` at `start`, ignoring case: the end of the keyword, or `None`.
fn direction_keyword_end(rest: &str, start: usize, keyword: &str) -> Option<usize> {
    let after = rest.get(start..)?;
    if !starts_ci(after, "direction") {
        return None;
    }
    let after_word = &after["direction".len()..];
    let spaces = space_run(after_word);
    if spaces == 0 {
        return None;
    }
    let after_spaces = &after_word[spaces..];
    starts_ci(after_spaces, keyword).then_some(start + "direction".len() + spaces + keyword.len())
}

/// The keyword `word` with an implicit word boundary after it (Jison's `\b` after a literal that
/// ends in a word character), ignoring case. The length of the keyword.
fn keyword_len(rest: &str, word: &str) -> Option<usize> {
    if !starts_ci(rest, word) {
        return None;
    }
    let next = rest[word.len()..].chars().next();
    (!next.is_some_and(is_word)).then_some(word.len())
}

/// The keywords of the initial state, with the token each one makes, in the grammar's order. Each
/// takes a word boundary after it, so at most one of them matches a position.
fn keyword(rest: &str) -> Option<(Token, usize)> {
    type Make = fn() -> Token;
    const KEYWORDS: &[(&str, Make)] = &[
        ("requirementDiagram", || Token::Rd),
        ("id", || Token::IdKw),
        ("text", || Token::TextKw),
        ("risk", || Token::RiskKw),
        ("verifyMethod", || Token::VerifyMethodKw),
        ("requirement", || Token::Kind(RequirementKind::Requirement)),
        ("functionalRequirement", || {
            Token::Kind(RequirementKind::FunctionalRequirement)
        }),
        ("interfaceRequirement", || {
            Token::Kind(RequirementKind::InterfaceRequirement)
        }),
        ("performanceRequirement", || {
            Token::Kind(RequirementKind::PerformanceRequirement)
        }),
        ("physicalRequirement", || {
            Token::Kind(RequirementKind::PhysicalRequirement)
        }),
        ("designConstraint", || {
            Token::Kind(RequirementKind::DesignConstraint)
        }),
        ("low", || Token::Level("Low")),
        ("medium", || Token::Level("Medium")),
        ("high", || Token::Level("High")),
        ("analysis", || Token::Verify("Analysis")),
        ("demonstration", || Token::Verify("Demonstration")),
        ("inspection", || Token::Verify("Inspection")),
        ("test", || Token::Verify("Test")),
        ("element", || Token::ElementKw),
        ("contains", || Token::Relation(RelationshipKind::Contains)),
        ("copies", || Token::Relation(RelationshipKind::Copies)),
        ("derives", || Token::Relation(RelationshipKind::Derives)),
        ("satisfies", || Token::Relation(RelationshipKind::Satisfies)),
        ("verifies", || Token::Relation(RelationshipKind::Verifies)),
        ("refines", || Token::Relation(RelationshipKind::Refines)),
        ("traces", || Token::Relation(RelationshipKind::Traces)),
        ("type", || Token::TypeKw),
        ("docref", || Token::DocRefKw),
        ("style", || Token::StyleKw),
        ("classDef", || Token::ClassDefKw),
        ("class", || Token::ClassKw),
    ];
    KEYWORDS
        .iter()
        .find_map(|&(word, make)| keyword_len(rest, word).map(|len| (make(), len)))
}

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    /// The next token, reading rules until one makes a token. At the end of the input the rules
    /// of the current state are tried once more, and after that every call returns `RawEof`.
    fn next_token(&mut self, source: Source<'_>) -> Result<Spanned, ParseError> {
        loop {
            if self.done {
                return Ok(Spanned {
                    token: Token::RawEof,
                    start: self.pos,
                });
            }
            if self.rest().is_empty() {
                self.done = true;
            }
            let start = self.pos;
            let step = match self.states.top() {
                State::Initial => self.initial(),
                State::Style => self.style(),
                State::Str => self.string(),
                State::AccTitle => self.value(State::AccTitle),
                State::AccDescr => self.value(State::AccDescr),
                State::AccDescrMulti => self.multiline(),
            };
            match step {
                Step::Token(token, len) => {
                    self.pos += len;
                    return Ok(Spanned { token, start });
                }
                Step::Skip(len) => self.pos += len,
                Step::NoMatch if self.rest().is_empty() => {
                    return Ok(Spanned {
                        token: Token::RawEof,
                        start,
                    });
                }
                Step::NoMatch => {
                    let message = "unrecognized text";
                    return Err(source.error(start, message));
                }
            }
        }
    }

    /// The rules of the initial state, in the grammar's order.
    fn initial(&mut self) -> Step {
        let rest = self.rest();
        let first = rest.chars().next();

        // 18. `title\s[^#\n;]+`: no production takes the token, so the parser rejects it.
        if let Some(len) = title_len(rest) {
            return Step::Token(Token::Title, len);
        }
        // 19. `accTitle\s*":"\s*`: enters the acc_title state.
        if let Some(len) = delimited_len(rest, "accTitle", ':') {
            self.states.push(State::AccTitle);
            return Step::Token(Token::AccTitle, len);
        }
        // 21. `accDescr\s*":"\s*`: enters the acc_descr state.
        if let Some(len) = delimited_len(rest, "accDescr", ':') {
            self.states.push(State::AccDescr);
            return Step::Token(Token::AccDescr, len);
        }
        // 23. `accDescr\s*"{"\s*`: enters the multi-line state, without a token.
        if let Some(len) = delimited_len(rest, "accDescr", '{') {
            self.states.push(State::AccDescrMulti);
            return Step::Skip(len);
        }
        // 26-29. The direction rules: they take the rest of the line from the token start (Q47).
        if let Some((direction, len)) = direction_len(rest) {
            return Step::Token(Token::Direction(direction), len);
        }
        // 30. `(\r?\n)+`: NEWLINE.
        if let Some(len) = newlines_len(rest) {
            return Step::Token(Token::Newline, len);
        }
        // 31. `\s+`: skipped, line breaks included.
        if first.is_some_and(is_js_space) {
            return Step::Skip(space_run(rest));
        }
        // 32. `\#[^\n]*`: a comment to the end of the line.
        if first == Some('#') {
            return Step::Skip(line_len(rest));
        }
        // 33. `\%%[^\n]*`: a comment to the end of the line. The reference's lexer reads the
        // escape `\%` and the `%` after it as one `%`, so a single percent sign starts a comment
        // (probed: `%x` and `%` alone are accepted, `docs-port/requirement/PLAN.md`, Q45).
        if first == Some('%') {
            return Step::Skip(line_len(rest));
        }
        // 34. `<<EOF>>`.
        if rest.is_empty() {
            return Step::Token(Token::Eof, 0);
        }
        // 36-75, 77, 88-89: the keywords. The keyword that enters the style state pushes it.
        if let Some((token, len)) = keyword(rest) {
            if matches!(token, Token::StyleKw | Token::ClassDefKw | Token::ClassKw) {
                self.states.push(State::Style);
            }
            return Step::Token(token, len);
        }
        // 38-41: the braces and the colons. `:::` takes three colons, as `":"{3}`.
        if rest.starts_with(":::") {
            return Step::Token(Token::StyleSeparator, 3);
        }
        match first {
            Some('{') => return Step::Token(Token::StructStart, 1),
            Some('}') => return Step::Token(Token::StructStop, 1),
            Some(':') => return Step::Token(Token::ColonSep, 1),
            _ => {}
        }
        // 91-93: the arrows and the line. `<-` and `->` come before `-`.
        if rest.starts_with("<-") {
            return Step::Token(Token::EndArrowL, 2);
        }
        if rest.starts_with("->") {
            return Step::Token(Token::EndArrowR, 2);
        }
        if first == Some('-') {
            return Step::Token(Token::Line, 1);
        }
        // 95: `"` enters the string state, without a token.
        if first == Some('"') {
            self.states.push(State::Str);
            return Step::Skip(1);
        }
        // 99: `[\w][^:,\r\n\{\<\>\-\=]*`, trimmed.
        if first.is_some_and(is_word) {
            let len = run_until(rest, |c| {
                matches!(c, ':' | ',' | '\r' | '\n' | '{' | '<' | '>' | '-' | '=')
            });
            return Step::Token(Token::Word(js_trim(&rest[..len]).to_owned()), len);
        }
        // 103: `,`.
        if first == Some(',') {
            return Step::Token(Token::Comma, 1);
        }
        Step::NoMatch
    }

    /// The rules of the style state. A line break leaves it; the other rules of the initial state
    /// do not apply, so `(`, a tab and non-ASCII text are errors here.
    fn style(&mut self) -> Step {
        let rest = self.rest();
        let Some(first) = rest.chars().next() else {
            return Step::NoMatch;
        };
        if is_word(first) {
            let len = run_until(rest, |c| !is_word(c));
            return Step::Token(Token::Alpha(rest[..len].to_owned()), len);
        }
        match first {
            ':' => Step::Token(Token::StyleColon, 1),
            ';' => Step::Token(Token::Semicolon, 1),
            '%' => Step::Token(Token::Percent, 1),
            '-' => Step::Token(Token::StyleMinus, 1),
            '#' => Step::Token(Token::StyleHash, 1),
            ' ' => Step::Skip(1),
            '"' => {
                self.states.push(State::Str);
                Step::Skip(1)
            }
            '\n' => {
                self.states.pop();
                Step::Skip(1)
            }
            ',' => Step::Token(Token::Comma, 1),
            _ => Step::NoMatch,
        }
    }

    /// The string state: `"` leaves it, and the text up to the next `"` is a quoted string.
    fn string(&mut self) -> Step {
        let rest = self.rest();
        if rest.starts_with('"') {
            self.states.pop();
            return Step::Skip(1);
        }
        if rest.is_empty() {
            return Step::NoMatch;
        }
        let len = run_until(rest, |c| c == '"');
        Step::Token(Token::Quoted(rest[..len].to_owned()), len)
    }

    /// The acc_title and acc_descr states: the rest of the line, possibly empty, and the state is
    /// left at once.
    fn value(&mut self, state: State) -> Step {
        self.states.pop();
        let len = line_len(self.rest());
        let text = self.rest()[..len].to_owned();
        let token = match state {
            State::AccTitle => Token::AccTitleValue(text),
            _ => Token::AccDescrValue(text),
        };
        Step::Token(token, len)
    }

    /// The acc_descr_multiline state: `}` leaves it, and the text up to a `}` is the value.
    fn multiline(&mut self) -> Step {
        let rest = self.rest();
        if rest.starts_with('}') {
            self.states.pop();
            return Step::Skip(1);
        }
        let len = run_until(rest, |c| c == '}');
        Step::Token(Token::AccDescrMultiValue(rest[..len].to_owned()), len)
    }
}
