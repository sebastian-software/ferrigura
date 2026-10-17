//! The xychart scanner: the rules of `xychart.jison` as Jison generates them, in the generated order.
//! The first rule that matches wins, as in Jison, and the keywords are case-insensitive. The rule
//! numbers in the comments are the indices of the generated `rules` array, and the lists of each
//! state are the generated `conditions`; see `docs-port/xychart/PLAN.md`, section 3.
//!
//! The scanner reproduces Jison's `next()` and its `done` flag: once the input is empty, the next
//! call reads the rules of the current state at the end of the input, and after that every call
//! returns Jison's raw end symbol, which the grammar does not accept (section 3 of the plan).

use crate::scan::line_len;
use crate::text::{is_js_space, is_word};

/// The start conditions: the initial state, the exclusive states of the accessibility rules and of
/// the quoted strings, and the inclusive states of the axis and plot statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    AxisData,
    AxisBandData,
    Data,
    DataInner,
    AccTitle,
    AccDescr,
    AccDescrMulti,
    Str,
}

/// The tokens of the grammar. Each keyword that carries no value is a unit variant; a value is the
/// text the rule matched (`yytext`), which the grammar may read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `title`.
    Title,
    /// `xychart`.
    XyChart,
    /// `vertical` or `horizontal`, as typed (the grammar compares the text case-sensitively).
    ChartOrientation(String),
    /// `x-axis`.
    XAxis,
    /// `y-axis`.
    YAxis,
    /// `line`.
    Line,
    /// `bar`.
    Bar,
    /// `-->`.
    ArrowDelimiter,
    /// `[`.
    SquareBracesStart,
    /// `]`.
    SquareBracesEnd,
    /// A number with an optional sign and a fraction, as the axis and plot rules read it.
    NumberWithDecimal(String),
    /// A string between quotes, which may span lines.
    Str(String),
    /// `,`.
    Comma,
    /// `:` on its own.
    Colon,
    /// `;`.
    Semi,
    /// One or more line breaks.
    Newline,
    /// `accTitle:`, followed by its value.
    AccTitle,
    /// The rest of the line after `accTitle:`.
    AccTitleValue(String),
    /// `accDescr:`, followed by its value.
    AccDescr,
    /// The rest of the line after `accDescr:`.
    AccDescrValue(String),
    /// The text of an `accDescr { … }` block, up to its `}`.
    AccDescrMultiValue(String),
    /// A word of plain text: `ALPHA`, `NUM`, `&`, `+`, `=`, `*`, `.`, `#`, `-` or `_`.
    AlphaNum(String),
    /// The end of the input, as the grammar's `EOF` terminal.
    Eof,
    /// Jison's raw end symbol, returned once the input is done. The grammar does not accept it.
    RawEof,
    /// A character no rule takes: Jison's lexical error. The scan stops here.
    Invalid,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// The rules of each state, as the generated `conditions` list them (`xychart-generated`, the
/// `conditions` object). Each number is an index of the generated `rules` array.
fn rules_of(state: State) -> &'static [u8] {
    match state {
        State::Initial => &[
            0, 1, 4, 5, 6, 7, 9, 11, 14, 15, 16, 17, 20, 21, 25, 26, 29, 30, 31, 32, 33, 34, 35,
            36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        ],
        State::AxisData => &[
            0, 1, 2, 4, 5, 6, 7, 9, 11, 14, 15, 16, 17, 18, 19, 20, 21, 23, 25, 26, 29, 30, 31, 32,
            33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        ],
        State::Data => &[
            0, 1, 3, 4, 5, 6, 7, 9, 11, 14, 15, 16, 17, 20, 21, 22, 25, 26, 29, 30, 31, 32, 33, 34,
            35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        ],
        State::DataInner => &[
            0, 1, 4, 5, 6, 7, 9, 11, 14, 15, 16, 17, 20, 21, 23, 24, 25, 26, 29, 30, 31, 32, 33,
            34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        ],
        State::AxisBandData => &[
            0, 1, 4, 5, 6, 7, 9, 11, 14, 15, 16, 17, 20, 21, 24, 25, 26, 29, 30, 31, 32, 33, 34,
            35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        ],
        State::AccTitle => &[8],
        State::AccDescr => &[10],
        State::AccDescrMulti => &[12, 13],
        State::Str => &[27, 28],
    }
}

/// Reads the whole text. The result ends with the first `RawEof`, or with the first `Invalid`.
pub(crate) fn scan(text: &str) -> Vec<Spanned> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        states: vec![State::Initial],
        done: false,
    };
    let mut tokens = Vec::new();
    loop {
        let spanned = scanner.next_token();
        let stop = matches!(spanned.token, Token::RawEof | Token::Invalid);
        tokens.push(spanned);
        if stop {
            return tokens;
        }
    }
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    /// The condition stack. Jison's `popState` never empties it: the initial state stays.
    states: Vec<State>,
    done: bool,
}

/// What a matching rule does: a token and the bytes it consumes, or the bytes it skips.
enum Matched {
    Token(Token, usize),
    Skip(usize),
}

/// Whether `rest` starts with `word`, ignoring ASCII case.
fn starts_with_ci(rest: &str, word: &str) -> bool {
    rest.as_bytes()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word.as_bytes()))
}

/// The keyword `word` (case-insensitive) with `\b` after it: a word character must not follow.
fn keyword_len(rest: &str, word: &str) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after = &rest[word.len()..];
    match after.chars().next() {
        Some(next) if is_word(next) => None,
        _ => Some(word.len()),
    }
}

/// Byte length of the run of JavaScript white space at the start of `text`, line breaks included.
fn space_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// Byte length of the run of ASCII digits at the start of `text`.
fn digits_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len()
}

/// Byte length of the text before the first character of `stops`, or of all of `text`.
fn run_until(text: &str, stops: &[char]) -> usize {
    text.find(|c: char| stops.contains(&c))
        .unwrap_or(text.len())
}

/// `word` (case-insensitive), optional white space, `delimiter`, optional white space: the
/// `accTitle\s*:\s*` and `accDescr\s*:\s*` rules, and the `accDescr\s*\{\s*` rule.
fn delimited_len(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after_word = &rest[word.len()..];
    let spaces = space_run(after_word);
    let after_delimiter = after_word[spaces..].strip_prefix(delimiter)?;
    Some(word.len() + spaces + delimiter.len_utf8() + space_run(after_delimiter))
}

/// The number of `[+-]?(?:\d+(?:\.\d+)?|\.\d+)` at the start of `rest`, or `None`.
fn number_len(rest: &str) -> Option<usize> {
    let sign = usize::from(rest.starts_with(['+', '-']));
    let body = &rest[sign..];
    let integer = digits_run(body);
    if integer > 0 {
        let fraction = match body[integer..].strip_prefix('.') {
            Some(after_dot) => match digits_run(after_dot) {
                0 => 0,
                digits => 1 + digits,
            },
            None => 0,
        };
        return Some(sign + integer + fraction);
    }
    let fraction = body.strip_prefix('.')?;
    match digits_run(fraction) {
        0 => None,
        digits => Some(sign + 1 + digits),
    }
}

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn state(&self) -> State {
        *self.states.last().unwrap_or(&State::Initial)
    }

    fn push(&mut self, state: State) {
        self.states.push(state);
    }

    /// Jison's `popState`: the top condition goes, but the initial one stays.
    fn pop(&mut self) {
        if self.states.len() > 1 {
            self.states.pop();
        }
    }

    /// The next token: Jison's `next()` called until it returns a token, or the raw end once done.
    fn next_token(&mut self) -> Spanned {
        loop {
            if self.done {
                return Spanned {
                    token: Token::RawEof,
                    start: self.pos,
                };
            }
            if self.rest().is_empty() {
                self.done = true;
            }
            let start = self.pos;
            match self.match_rules() {
                Some(Matched::Token(token, consumed)) => {
                    self.pos += consumed;
                    return Spanned { token, start };
                }
                Some(Matched::Skip(consumed)) => {
                    self.pos += consumed;
                }
                None => {
                    return Spanned {
                        token: Token::Invalid,
                        start,
                    };
                }
            }
        }
    }

    /// The first rule of the current state that matches at the position.
    fn match_rules(&mut self) -> Option<Matched> {
        let state = self.state();
        for &index in rules_of(state) {
            if let Some(matched) = self.rule(index) {
                return Some(matched);
            }
        }
        None
    }

    /// Rule `index` of the generated table. The side effects of the action (the condition stack)
    /// happen only when the rule matches.
    fn rule(&mut self, index: u8) -> Option<Matched> {
        let text = self.text;
        let rest = &text[self.pos..];
        let first = rest.chars().next();
        match index {
            // `\%\%(?!\{)[^\n]*`: a comment, unless `%%` is followed by `{`.
            0 => {
                let after = rest.strip_prefix("%%")?;
                if after.starts_with('{') {
                    return None;
                }
                Some(Matched::Skip(2 + line_len(after)))
            }
            // `[^\}]\%\%[^\n]*`: one character other than `}`, then a comment. The character is
            // one UTF-16 unit, so a character outside the BMP cannot be followed by `%%` here.
            1 => {
                let c = first?;
                if c == '}' || c.len_utf16() != 1 {
                    return None;
                }
                let after = rest[c.len_utf8()..].strip_prefix("%%")?;
                Some(Matched::Skip(c.len_utf8() + 2 + line_len(after)))
            }
            // `(\r?\n)` in axis_data and in data: one line break, leaving the state.
            2 | 3 => {
                let len = line_break_len(rest)?;
                self.pop();
                Some(Matched::Token(Token::Newline, len))
            }
            // `[\n\r]+`: one or more line breaks.
            4 => {
                let len = rest.len() - rest.trim_start_matches(['\n', '\r']).len();
                (len > 0).then_some(Matched::Token(Token::Newline, len))
            }
            // `\%\%[^\n]*`: a comment.
            5 => {
                let after = rest.strip_prefix("%%")?;
                Some(Matched::Skip(2 + line_len(after)))
            }
            // `title\b`.
            6 => keyword_len(rest, "title").map(|len| Matched::Token(Token::Title, len)),
            // `accTitle\s*:\s*`: enters acc_title.
            7 => {
                let len = delimited_len(rest, "accTitle", ':')?;
                self.push(State::AccTitle);
                Some(Matched::Token(Token::AccTitle, len))
            }
            // `[^\n]*` in acc_title: the rest of the line, possibly empty. Leaves the state.
            8 => {
                let len = line_len(rest);
                self.pop();
                Some(Matched::Token(
                    Token::AccTitleValue(rest[..len].to_owned()),
                    len,
                ))
            }
            // `accDescr\s*:\s*`: enters acc_descr.
            9 => {
                let len = delimited_len(rest, "accDescr", ':')?;
                self.push(State::AccDescr);
                Some(Matched::Token(Token::AccDescr, len))
            }
            // `[^\n]*` in acc_descr: the rest of the line, possibly empty. Leaves the state.
            10 => {
                let len = line_len(rest);
                self.pop();
                Some(Matched::Token(
                    Token::AccDescrValue(rest[..len].to_owned()),
                    len,
                ))
            }
            // `accDescr\s*\{\s*`: enters acc_descr_multiline, without a token.
            11 => {
                let len = delimited_len(rest, "accDescr", '{')?;
                self.push(State::AccDescrMulti);
                Some(Matched::Skip(len))
            }
            // `\}` in acc_descr_multiline: leaves the state, without a token.
            12 => rest.starts_with('}').then(|| {
                self.pop();
                Matched::Skip(1)
            }),
            // `[^\}]*` in acc_descr_multiline: the text up to `}`, possibly empty.
            13 => {
                let len = run_until(rest, &['}']);
                Some(Matched::Token(
                    Token::AccDescrMultiValue(rest[..len].to_owned()),
                    len,
                ))
            }
            // `xychart\b`.
            14 => keyword_len(rest, "xychart").map(|len| Matched::Token(Token::XyChart, len)),
            // `(?:vertical|horizontal)`: no word boundary after it (quirk P2).
            15 => ["vertical", "horizontal"]
                .into_iter()
                .find(|word| starts_with_ci(rest, word))
                .map(|word| {
                    Matched::Token(
                        Token::ChartOrientation(rest[..word.len()].to_owned()),
                        word.len(),
                    )
                }),
            // `x-axis\b`: enters axis_data.
            16 => {
                let len = keyword_len(rest, "x-axis")?;
                self.push(State::AxisData);
                Some(Matched::Token(Token::XAxis, len))
            }
            // `y-axis\b`: enters axis_data.
            17 => {
                let len = keyword_len(rest, "y-axis")?;
                self.push(State::AxisData);
                Some(Matched::Token(Token::YAxis, len))
            }
            // `\[` in axis_data: enters axis_band_data.
            18 => {
                if !rest.starts_with('[') {
                    return None;
                }
                self.push(State::AxisBandData);
                Some(Matched::Token(Token::SquareBracesStart, 1))
            }
            // `-->` in axis_data.
            19 => rest
                .starts_with("-->")
                .then_some(Matched::Token(Token::ArrowDelimiter, 3)),
            // `line\b`: enters data.
            20 => {
                let len = keyword_len(rest, "line")?;
                self.push(State::Data);
                Some(Matched::Token(Token::Line, len))
            }
            // `bar\b`: enters data.
            21 => {
                let len = keyword_len(rest, "bar")?;
                self.push(State::Data);
                Some(Matched::Token(Token::Bar, len))
            }
            // `\[` in data: enters data_inner.
            22 => {
                if !rest.starts_with('[') {
                    return None;
                }
                self.push(State::DataInner);
                Some(Matched::Token(Token::SquareBracesStart, 1))
            }
            // The number rule, in axis_data and data_inner only.
            23 => {
                let len = number_len(rest)?;
                Some(Matched::Token(
                    Token::NumberWithDecimal(rest[..len].to_owned()),
                    len,
                ))
            }
            // `\]` in axis_band_data and data_inner: leaves the state.
            24 => rest.starts_with(']').then(|| {
                self.pop();
                Matched::Token(Token::SquareBracesEnd, 1)
            }),
            // The garbled Markdown rule (quirk P4): it never matches the text a reader writes, and
            // the Markdown string state it would enter has no rules. A backtick is a lexical error.
            25 => None,
            // `"`: enters string. No token.
            26 => {
                if !rest.starts_with('"') {
                    return None;
                }
                self.push(State::Str);
                Some(Matched::Skip(1))
            }
            // `"` in string: leaves the state. No token.
            27 => {
                if !rest.starts_with('"') {
                    return None;
                }
                self.pop();
                Some(Matched::Skip(1))
            }
            // `[^"]*` in string: the text up to the next quote, possibly empty.
            28 => {
                let len = run_until(rest, &['"']);
                Some(Matched::Token(Token::Str(rest[..len].to_owned()), len))
            }
            // `\[`: SQUARE_BRACES_START, without a state change.
            29 => rest
                .starts_with('[')
                .then_some(Matched::Token(Token::SquareBracesStart, 1)),
            // `\]`: SQUARE_BRACES_END, without a state change.
            30 => rest
                .starts_with(']')
                .then_some(Matched::Token(Token::SquareBracesEnd, 1)),
            // `[A-Za-z]+`.
            31 => {
                let len = rest
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .unwrap_or(rest.len());
                (len > 0).then(|| Matched::Token(Token::AlphaNum(rest[..len].to_owned()), len))
            }
            // `:`.
            32 => rest
                .starts_with(':')
                .then_some(Matched::Token(Token::Colon, 1)),
            // `\+`, `=`, `*`, `#`, `[\_]`, `.`, `&`, `-`: the text of one character.
            33 | 35 | 36 | 37 | 38 | 39 | 40 | 41 => {
                let c = first?;
                let accepted = match index {
                    33 => c == '+',
                    35 => c == '=',
                    36 => c == '*',
                    37 => c == '#',
                    38 => c == '_',
                    39 => c == '.',
                    40 => c == '&',
                    _ => c == '-',
                };
                accepted.then(|| Matched::Token(Token::AlphaNum(c.to_string()), 1))
            }
            // `,`.
            34 => rest
                .starts_with(',')
                .then_some(Matched::Token(Token::Comma, 1)),
            // `[0-9]+`.
            42 => {
                let len = digits_run(rest);
                (len > 0).then(|| Matched::Token(Token::AlphaNum(rest[..len].to_owned()), len))
            }
            // `\s+`: white space, which runs over line breaks.
            43 => {
                let len = space_run(rest);
                (len > 0).then_some(Matched::Skip(len))
            }
            // `;`.
            44 => rest
                .starts_with(';')
                .then_some(Matched::Token(Token::Semi, 1)),
            // `$`: the end of the input.
            45 => rest.is_empty().then_some(Matched::Token(Token::Eof, 0)),
            _ => None,
        }
    }
}

/// The length of `\r?\n` at the start of `text`, or `None`.
fn line_break_len(text: &str) -> Option<usize> {
    if text.starts_with("\r\n") {
        Some(2)
    } else if text.starts_with('\n') {
        Some(1)
    } else {
        None
    }
}
