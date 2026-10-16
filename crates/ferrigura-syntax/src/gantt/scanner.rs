//! The gantt lexer: the rules of `gantt.jison`, with its start conditions, in the grammar's order.
//! The first rule that matches wins, as in Jison, and a keyword literal matches case-insensitively
//! with the word boundary Jison adds to quoted literals. See `docs-port/gantt/PLAN.md`, section 2.
//!
//! The lexer never depends on the parser, so the whole token list is made before the grammar runs.
//! Every start condition has a rule that matches at the end of the text (the empty value rules), so
//! the lexer does not fail; a character no rule starts with is an `Invalid` token, which the grammar
//! rejects.

use crate::scan::{StateStack, line_len, space_len};
use crate::text::{is_js_space, is_word};

/// The start conditions of the gantt lexer (the `%x` states of `gantt.jison`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    AccTitle,
    AccDescr,
    AccDescrMulti,
    Href,
    CallbackName,
    CallbackArgs,
    Click,
}

/// The tokens of the gantt grammar. Tokens without a production are kept, so that the grammar
/// rejects them where Jison does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// The end of the text (`<<EOF>>`).
    Eof,
    /// A run of line breaks (`NL`).
    Newline,
    /// `%%{` outside a directive: no production.
    OpenDirective,
    /// `accTitle:` with the blanks after it.
    AccTitle,
    /// The rest of the line after `accTitle:`.
    AccTitleValue(String),
    /// `accDescr:` with the blanks after it.
    AccDescr,
    /// The rest of the line after `accDescr:`.
    AccDescrValue(String),
    /// The text of a block `accDescr { … }`, up to the closing brace; may be empty at the end.
    AccDescrMultiValue(String),
    /// The address of a `click … href "…"`, without its quotes.
    Href(String),
    /// The name of a `click … call name(…)`.
    CallbackName(String),
    /// The arguments of a `call` between the parentheses.
    CallbackArgs(String),
    /// The task id of a `click` line.
    ClickId(String),
    /// The `gantt` keyword.
    Gantt,
    /// `dateFormat` and the rest of its line (the keyword and one blank are included).
    DateFormat(String),
    /// `inclusiveEndDates`.
    InclusiveEndDates,
    /// `topAxis`.
    TopAxis,
    /// `axisFormat` and the rest of its value.
    AxisFormat(String),
    /// `tickInterval` and the rest of its value.
    TickInterval(String),
    /// `includes` and the rest of its value.
    Includes(String),
    /// `excludes` and the rest of its value.
    Excludes(String),
    /// `todayMarker` and the rest of its value.
    TodayMarker(String),
    /// `weekday` and a day name, with the name in lower case.
    Weekday(&'static str),
    /// `weekend` and `friday` or `saturday`.
    Weekend(&'static str),
    /// A `YYYY-MM-DD` at the start of a statement: no production.
    Date,
    /// `title` and the rest of its line (the keyword and one whitespace character are included).
    Title(String),
    /// `accDescription`: no production.
    AccDescription,
    /// `section` and the rest of its line (the keyword and one whitespace character are included).
    Section(String),
    /// The text of a task, up to its colon, untrimmed.
    TaskTxt(String),
    /// The data of a task, from its colon to the end of the line.
    TaskData(String),
    /// A colon that starts no data: no production.
    Colon,
    /// A character that no rule matches: no production.
    Invalid,
}

/// A token with the byte offset where it starts in the lexed text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// What a rule does with the text it matched.
enum Action {
    /// Nothing is returned (comments, blanks and the closing quotes of the states).
    Skip,
    /// The token is returned.
    Emit(Token),
    /// A state is entered, and the token (if any) is returned.
    Begin(State, Option<Token>),
    /// The state is left, and the token (if any) is returned.
    End(Option<Token>),
    /// The state is left and another one entered (`popState` then `begin`).
    Replace(State),
}

/// Lexes the whole text. The list ends with the first `Eof` token.
pub(crate) fn tokenize(text: &str) -> Vec<Spanned> {
    let mut lexer = Lexer {
        text,
        pos: 0,
        states: StateStack::new(State::Initial),
        done: false,
    };
    let mut out = Vec::new();
    loop {
        let spanned = lexer.next_token();
        let finished = spanned.token == Token::Eof;
        out.push(spanned);
        if finished {
            return out;
        }
    }
}

struct Lexer<'a> {
    text: &'a str,
    pos: usize,
    states: StateStack<State>,
    /// Set once the input is exhausted, as Jison's `done` flag: the next call returns `Eof`.
    done: bool,
}

impl Lexer<'_> {
    /// The next token that a rule returns, with the offset where its match starts.
    fn next_token(&mut self) -> Spanned {
        loop {
            let start = self.pos;
            if self.pos == self.text.len() {
                if self.done || self.states.top() == State::Initial {
                    self.done = true;
                    return Spanned {
                        token: Token::Eof,
                        start,
                    };
                }
                // The text ends inside a start condition (an unclosed `accDescr {` block, a
                // `call` without its end, a `click` or `href` without its value). The reference
                // rejects this input at the end, so the end is an `Invalid` token here.
                self.done = true;
                return Spanned {
                    token: Token::Invalid,
                    start,
                };
            }
            let rest = &self.text[self.pos..];
            let matched = match self.states.top() {
                State::Initial => initial_rule(rest),
                state => state_rule(state, rest),
            };
            let Some((len, action)) = matched else {
                // Unreachable for the rules of gantt.jison: `[^:\n]+` covers every other character.
                let width = rest.chars().next().map_or(1, char::len_utf8);
                self.pos += width;
                return Spanned {
                    token: Token::Invalid,
                    start,
                };
            };
            self.pos += len;
            let token = match action {
                Action::Skip => None,
                Action::Emit(token) => Some(token),
                Action::Begin(state, token) => {
                    self.states.push(state);
                    token
                }
                Action::End(token) => {
                    self.states.pop();
                    token
                }
                Action::Replace(state) => {
                    self.states.pop();
                    self.states.push(state);
                    None
                }
            };
            if let Some(token) = token {
                return Spanned { token, start };
            }
        }
    }
}

/// Byte length of `word` at the start of `rest`, ignoring ASCII case, and with the word boundary
/// that Jison adds after a quoted literal that ends in a word character.
fn keyword(rest: &str, word: &str) -> Option<usize> {
    let head = rest.get(..word.len())?;
    if !head.eq_ignore_ascii_case(word) {
        return None;
    }
    let next = rest[word.len()..].chars().next();
    (!next.is_some_and(is_word)).then_some(word.len())
}

/// Byte length of `word` at the start of `rest`, ignoring ASCII case, without a boundary (an
/// unquoted rule such as `weekday\s+monday`).
fn plain_word(rest: &str, word: &str) -> Option<usize> {
    let head = rest.get(..word.len())?;
    head.eq_ignore_ascii_case(word).then_some(word.len())
}

/// Byte length of the characters at the start of `rest` that are not in `excluded`, at least one.
fn run_outside(rest: &str, excluded: &[char]) -> Option<usize> {
    let len: usize = rest
        .chars()
        .take_while(|c| !excluded.contains(c))
        .map(char::len_utf8)
        .sum();
    (len > 0).then_some(len)
}

/// `keyword\s[^…]+`: the keyword, one whitespace character (a line break included), then at least
/// one character that is not in `excluded`.
fn keyword_value(rest: &str, word: &str, excluded: &[char]) -> Option<usize> {
    let head = keyword(rest, word)?;
    let after = &rest[head..];
    let space = after.chars().next().filter(|&c| is_js_space(c))?;
    let body = head + space.len_utf8();
    let value = run_outside(&rest[body..], excluded)?;
    Some(body + value)
}

/// `keyword\s+` (one or more whitespace characters): the keyword and the blanks after it.
fn keyword_then_spaces(rest: &str, word: &str) -> Option<usize> {
    let head = keyword(rest, word)?;
    let spaces = space_len(&rest[head..]);
    (spaces > 0).then_some(head + spaces)
}

/// `"accTitle"\s*":"\s*` and its relatives: the word, blanks, a colon, blanks (line breaks too).
fn keyword_colon(rest: &str, word: &str) -> Option<usize> {
    let head = plain_word(rest, word)?;
    let after_word = head + space_len(&rest[head..]);
    let after_colon = rest[after_word..].strip_prefix(':')?;
    Some(after_word + 1 + space_len(after_colon))
}

/// `\d\d\d\d-\d\d-\d\d`.
fn date_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let shape = b"dddd-dd-dd";
    if bytes.len() < shape.len() {
        return None;
    }
    let matches = shape.iter().zip(bytes).all(|(&want, &got)| match want {
        b'd' => got.is_ascii_digit(),
        other => got == other,
    });
    matches.then_some(shape.len())
}

/// The rules that apply in the initial state, in the order of `gantt.jison` (lines 18-97).
fn initial_rule(rest: &str) -> Option<(usize, Action)> {
    // `\%\%\{` (line 18).
    if rest.starts_with("%%{") {
        return Some((3, Action::Emit(Token::OpenDirective)));
    }
    // `accTitle\s*":"\s*` (line 20).
    if let Some(len) = keyword_colon(rest, "accTitle") {
        return Some((len, Action::Begin(State::AccTitle, Some(Token::AccTitle))));
    }
    // `accDescr\s*":"\s*` (line 22).
    if let Some(len) = keyword_colon(rest, "accDescr") {
        return Some((len, Action::Begin(State::AccDescr, Some(Token::AccDescr))));
    }
    // `accDescr\s*"{"\s*` (line 24): the block form, which returns no token.
    if let Some(len) = block_open(rest) {
        return Some((len, Action::Begin(State::AccDescrMulti, None)));
    }
    // `\%\%(?!\{)*[^\n]*` (line 28): a comment. The lookahead is zero-width, so it is `%%` and the
    // rest of the line.
    if rest.starts_with("%%") {
        return Some((2 + line_len(&rest[2..]), Action::Skip));
    }
    // `[^\}]\%\%*[^\n]*` (line 29): any character but `}` followed by `%` starts a comment that
    // takes the rest of the line. The character may be a line break.
    if let Some(first) = rest.chars().next() {
        let after = &rest[first.len_utf8()..];
        if first != '}' && after.starts_with('%') {
            return Some((first.len_utf8() + line_len(after), Action::Skip));
        }
    }
    // `\%\%*[^\n]*[\n]*` (line 30): a `%` starts a comment, with the line breaks after it.
    if rest.starts_with('%') {
        let line = line_len(rest);
        let breaks = rest[line..].bytes().take_while(|&b| b == b'\n').count();
        return Some((line + breaks, Action::Skip));
    }
    // `[\n]+` (line 32).
    if rest.starts_with('\n') {
        let breaks = rest.bytes().take_while(|&b| b == b'\n').count();
        return Some((breaks, Action::Emit(Token::Newline)));
    }
    // `\s+` (line 33).
    let spaces = space_len(rest);
    if spaces > 0 {
        return Some((spaces, Action::Skip));
    }
    // `"href"[\s]+["]` (line 42).
    if let Some(head) = keyword(rest, "href") {
        let spaces = space_len(&rest[head..]);
        if spaces > 0 && rest[head + spaces..].starts_with('"') {
            return Some((head + spaces + 1, Action::Begin(State::Href, None)));
        }
    }
    // `"call"[\s]+` (line 54).
    if let Some(len) = keyword_then_spaces(rest, "call") {
        return Some((len, Action::Begin(State::CallbackName, None)));
    }
    // `"click"[\s]+` (line 67).
    if let Some(len) = keyword_then_spaces(rest, "click") {
        return Some((len, Action::Begin(State::Click, None)));
    }
    // The statement keywords (lines 71-88).
    if let Some(len) = keyword(rest, "gantt") {
        return Some((len, Action::Emit(Token::Gantt)));
    }
    if let Some(len) = keyword_value(rest, "dateFormat", &['#', '\n', ';']) {
        return Some((len, Action::Emit(Token::DateFormat(text_of(rest, len)))));
    }
    if let Some(len) = keyword(rest, "inclusiveEndDates") {
        return Some((len, Action::Emit(Token::InclusiveEndDates)));
    }
    if let Some(len) = keyword(rest, "topAxis") {
        return Some((len, Action::Emit(Token::TopAxis)));
    }
    if let Some(len) = keyword_value(rest, "axisFormat", &['#', '\n', ';']) {
        return Some((len, Action::Emit(Token::AxisFormat(text_of(rest, len)))));
    }
    if let Some(len) = keyword_value(rest, "tickInterval", &['#', '\n', ';']) {
        return Some((len, Action::Emit(Token::TickInterval(text_of(rest, len)))));
    }
    if let Some(len) = keyword_value(rest, "includes", &['#', '\n', ';']) {
        return Some((len, Action::Emit(Token::Includes(text_of(rest, len)))));
    }
    if let Some(len) = keyword_value(rest, "excludes", &['#', '\n', ';']) {
        return Some((len, Action::Emit(Token::Excludes(text_of(rest, len)))));
    }
    if let Some(len) = keyword_value(rest, "todayMarker", &['\n', ';']) {
        return Some((len, Action::Emit(Token::TodayMarker(text_of(rest, len)))));
    }
    // `weekday\s+monday` … `weekday\s+sunday` (lines 80-86) and `weekend\s+friday`,
    // `weekend\s+saturday` (lines 87-88). The names are matched in the order written.
    for day in [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ] {
        if let Some(len) = day_statement(rest, "weekday", day) {
            return Some((len, Action::Emit(Token::Weekday(day))));
        }
    }
    for day in ["friday", "saturday"] {
        if let Some(len) = day_statement(rest, "weekend", day) {
            return Some((len, Action::Emit(Token::Weekend(day))));
        }
    }
    // `\d\d\d\d-\d\d-\d\d` (line 89).
    if let Some(len) = date_len(rest) {
        return Some((len, Action::Emit(Token::Date)));
    }
    // `"title"\s[^\n]+` (line 90).
    if let Some(len) = keyword_value(rest, "title", &['\n']) {
        return Some((len, Action::Emit(Token::Title(text_of(rest, len)))));
    }
    // `"accDescription"\s[^#\n;]+` (line 91).
    if let Some(len) = keyword_value(rest, "accDescription", &['#', '\n', ';']) {
        return Some((len, Action::Emit(Token::AccDescription)));
    }
    // `"section"\s[^\n]+` (line 92).
    if let Some(len) = keyword_value(rest, "section", &['\n']) {
        return Some((len, Action::Emit(Token::Section(text_of(rest, len)))));
    }
    // `[^:\n]+` (line 93): the task text.
    if let Some(len) = run_outside(rest, &[':', '\n']) {
        return Some((len, Action::Emit(Token::TaskTxt(text_of(rest, len)))));
    }
    // `":"[^#\n;]+` (line 94): the task data, with its colon.
    if rest.starts_with(':') {
        if let Some(len) = run_outside(&rest[1..], &['#', '\n', ';']) {
            return Some((
                1 + len,
                Action::Emit(Token::TaskData(text_of(rest, 1 + len))),
            ));
        }
        // `":"` (line 95).
        return Some((1, Action::Emit(Token::Colon)));
    }
    None
}

/// `accDescr\s*"{"\s*` (line 24), as a length.
fn block_open(rest: &str) -> Option<usize> {
    let head = plain_word(rest, "accDescr")?;
    let after_word = head + space_len(&rest[head..]);
    let after_brace = rest[after_word..].strip_prefix('{')?;
    Some(after_word + 1 + space_len(after_brace))
}

/// The text of the first `len` bytes of `rest`.
fn text_of(rest: &str, len: usize) -> String {
    rest[..len].to_owned()
}

/// `keyword\s+day` without a boundary (an unquoted rule), as a length.
fn day_statement(rest: &str, keyword: &str, day: &str) -> Option<usize> {
    let head = plain_word(rest, keyword)?;
    let spaces = space_len(&rest[head..]);
    let name_at = head + spaces;
    (spaces > 0 && plain_word(&rest[name_at..], day).is_some()).then_some(name_at + day.len())
}

/// The rules of the start conditions, in the order they are written in `gantt.jison`.
fn state_rule(state: State, rest: &str) -> Option<(usize, Action)> {
    match state {
        // `(?!\n|;|#)*[^\n]*` (line 21): the rest of the line.
        State::AccTitle => {
            let len = line_len(rest);
            Some((
                len,
                Action::End(Some(Token::AccTitleValue(text_of(rest, len)))),
            ))
        }
        State::AccDescr => {
            let len = line_len(rest);
            Some((
                len,
                Action::End(Some(Token::AccDescrValue(text_of(rest, len)))),
            ))
        }
        // `[\}]` (line 25) and `[^\}]*` (line 26).
        State::AccDescrMulti => {
            if rest.starts_with('}') {
                return Some((1, Action::End(None)));
            }
            let len = rest.find('}').unwrap_or(rest.len());
            Some((
                len,
                Action::Emit(Token::AccDescrMultiValue(text_of(rest, len))),
            ))
        }
        // `["]` (line 43) and `[^"]*` (line 44).
        State::Href => {
            if rest.starts_with('"') {
                return Some((1, Action::End(None)));
            }
            let len = rest.find('"').unwrap_or(rest.len());
            Some((len, Action::Emit(Token::Href(text_of(rest, len)))))
        }
        // `\([\s]*\)` (line 55), `\(` (line 56) and `[^(]*` (line 57).
        State::CallbackName => {
            if let Some(after) = rest.strip_prefix('(') {
                let spaces = space_len(after);
                if after[spaces..].starts_with(')') {
                    return Some((1 + spaces + 1, Action::End(None)));
                }
                return Some((1, Action::Replace(State::CallbackArgs)));
            }
            let len = rest.find('(').unwrap_or(rest.len());
            Some((len, Action::Emit(Token::CallbackName(text_of(rest, len)))))
        }
        // `\)` (line 58) and `[^)]*` (line 59).
        State::CallbackArgs => {
            if rest.starts_with(')') {
                return Some((1, Action::End(None)));
            }
            let len = rest.find(')').unwrap_or(rest.len());
            Some((len, Action::Emit(Token::CallbackArgs(text_of(rest, len)))))
        }
        // `[\s\n]` (line 68) and `[^\s\n]*` (line 69).
        State::Click => {
            if rest.chars().next().is_some_and(is_js_space) {
                return Some((1, Action::End(None)));
            }
            let len: usize = rest
                .chars()
                .take_while(|&c| !is_js_space(c))
                .map(char::len_utf8)
                .sum();
            Some((len, Action::Emit(Token::ClickId(text_of(rest, len)))))
        }
        State::Initial => initial_rule(rest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<Token> {
        tokenize(text).into_iter().map(|s| s.token).collect()
    }

    #[test]
    fn keywords_match_case_insensitively_with_a_boundary() {
        assert_eq!(tokens("GANTT\n")[0], Token::Gantt);
        assert_eq!(
            tokens("ganttX :1d\n")[0],
            Token::TaskTxt("ganttX ".to_owned())
        );
    }

    #[test]
    fn a_character_followed_by_a_percent_skips_the_line() {
        // Q1: the line is dropped, as rule 29 does.
        assert_eq!(tokens("A%B :a, 2014-01-01, 1d\n")[0], Token::Newline);
    }

    #[test]
    fn section_takes_the_next_line_after_a_line_break() {
        // Q2: `\s` matches the line break.
        assert_eq!(
            tokens("section\nA :a1, 2014-01-01, 1d\n")[0],
            Token::Section("section\nA :a1, 2014-01-01, 1d".to_owned())
        );
    }

    #[test]
    fn an_accessibility_colon_takes_the_next_line() {
        // Q3: the blanks after the colon include the line break.
        assert_eq!(
            tokens("accTitle:\nsection X\n"),
            vec![
                Token::AccTitle,
                Token::AccTitleValue("section X".to_owned()),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn the_block_of_acc_descr_runs_to_the_closing_brace() {
        assert_eq!(
            tokens("accDescr {\n  one\n}\n")[0],
            Token::AccDescrMultiValue("one\n".to_owned())
        );
    }

    #[test]
    fn an_unclosed_block_ends_in_an_invalid_token() {
        // The reference rejects an unclosed block (probed by the exporter run).
        let stream = tokens("accDescr {\nhello\n");
        assert_eq!(stream[0], Token::AccDescrMultiValue("hello\n".to_owned()));
        assert_eq!(stream[1], Token::Invalid);
        assert_eq!(stream[2], Token::Eof);
    }

    #[test]
    fn a_colon_starts_task_data_up_to_a_hash_or_semicolon() {
        assert_eq!(
            tokens("A :a1, 2014-01-01, 1d\n")[..2],
            [
                Token::TaskTxt("A ".to_owned()),
                Token::TaskData(":a1, 2014-01-01, 1d".to_owned())
            ]
        );
    }

    #[test]
    fn a_date_at_the_start_of_a_statement_is_its_own_token() {
        assert_eq!(tokens("2014-01-01 x\n")[0], Token::Date);
    }

    #[test]
    fn weekday_and_weekend_names_are_lower_case() {
        assert_eq!(tokens("weekday  Monday\n")[0], Token::Weekday("monday"));
        assert_eq!(tokens("weekend friday\n")[0], Token::Weekend("friday"));
    }

    #[test]
    fn a_call_without_parentheses_runs_to_the_next_parenthesis() {
        // Q4: `[^(]*` takes line breaks.
        let stream = tokens("click A call f\nB :b, 2014-01-01, 1d\n(x)\n");
        assert_eq!(stream[0], Token::ClickId("A".to_owned()));
        assert_eq!(
            stream[1],
            Token::CallbackName("f\nB :b, 2014-01-01, 1d\n".to_owned())
        );
    }
}
