//! The quadrant chart scanner: the rules of `quadrant.jison` as Jison generates them, in the
//! generated order. The first rule that matches wins, not the longest, and the keywords are
//! case-insensitive. The rule numbers in the comments are the indices of the generated `rules`
//! array, and the lists of each state are the generated `conditions`; see
//! `docs-port/quadrant/PLAN.md`, section 3.
//!
//! The scanner reproduces Jison's `next()` and its `done` flag: once the input is empty, the next
//! call reads the rules of the current state at the end of the input, and after that every call
//! returns Jison's raw end symbol, which the grammar does not accept.

use crate::scan::{StateStack, line_len};
use crate::text::{is_js_space, is_word};

/// The start conditions. Every state but `Initial` is exclusive: only its own rules apply in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    Title,
    AccTitle,
    AccDescr,
    AccDescrMulti,
    MdString,
    Str,
    ClassName,
    PointStart,
    PointX,
    PointY,
}

/// The kinds of the grammar's terminals. A token's text is the text its rule matched, which the
/// grammar reads through its span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token {
    /// One or more line breaks.
    Newline,
    /// `;`.
    Semi,
    /// The end of the input, as the grammar's `EOF` terminal (rule 52 at the end of the text).
    Eof,
    /// Jison's raw end symbol, returned once the input is done. The grammar does not accept it.
    RawEof,
    /// A character no rule takes: Jison's lexical error. The scan stops here.
    Invalid,
    /// One white space character of ASCII or of a space the rule takes (`\s`).
    Space,
    /// `quadrantChart`, the header.
    Quadrant,
    /// `x-axis`.
    XAxis,
    /// `y-axis`.
    YAxis,
    /// `-->` and the like, with its blanks.
    AxisDelimiter,
    /// `quadrant-1`.
    Quadrant1,
    /// `quadrant-2`.
    Quadrant2,
    /// `quadrant-3`.
    Quadrant3,
    /// `quadrant-4`.
    Quadrant4,
    /// `classDef`.
    ClassDef,
    /// `title`.
    Title,
    /// The rest of the line after `title`.
    TitleValue,
    /// `accTitle:`.
    AccTitle,
    /// The rest of the line after `accTitle:`.
    AccTitleValue,
    /// `accDescr:`.
    AccDescr,
    /// The rest of the line after `accDescr:`.
    AccDescrValue,
    /// The text of an `accDescr { … }` block, up to its `}`; empty only at the end of the input.
    AccDescrMultiValue,
    /// The text in a Markdown string, between a quote and a backtick.
    MdStr,
    /// The text between two quotes; empty only at the end of the input.
    Str,
    /// The name after `:::`.
    ClassName,
    /// The `:` and `[` of a point, with the blanks around them.
    PointStart,
    /// The x coordinate of a point.
    PointX,
    /// The y coordinate of a point.
    PointY,
    /// `A`-`Z` and `a`-`z`, one or more.
    Alpha,
    /// A run of characters beyond ASCII.
    UnicodeText,
    /// `:` on its own.
    Colon,
    /// `+`.
    Plus,
    /// `,`.
    Comma,
    /// `=`.
    Equals,
    /// `*`.
    Mult,
    /// `#`.
    Brkt,
    /// `_`.
    Underscore,
    /// `.`.
    Dot,
    /// `&`.
    Amp,
    /// `-`.
    Minus,
    /// One or more decimal digits.
    Num,
    /// One of the characters of `[!"#$%&'*+,-.`?\\_/]`.
    Punctuation,
}

/// A token with the bytes where its text starts and ends in the scanned text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// The rules of each state, as the generated `conditions` list them. Each number is an index of
/// the generated `rules` array (the rule comments below give them).
fn rules_of(state: State) -> &'static [u8] {
    match state {
        State::Initial => &[
            0, 1, 2, 3, 4, 6, 8, 10, 13, 14, 15, 16, 17, 18, 19, 20, 21, 24, 27, 29, 34, 35, 36,
            37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52,
        ],
        State::Title => &[5],
        State::AccTitle => &[7],
        State::AccDescr => &[9],
        State::AccDescrMulti => &[11, 12],
        State::MdString => &[22, 23],
        State::Str => &[25, 26],
        State::ClassName => &[28],
        State::PointStart => &[30, 31],
        State::PointX => &[32],
        State::PointY => &[33],
    }
}

/// Reads the whole text. The result ends with the first `RawEof`, or with the first `Invalid`.
pub(crate) fn scan(text: &str) -> Vec<Spanned> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        states: StateStack::new(State::Initial),
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
    states: StateStack<State>,
    done: bool,
}

/// What a matching rule does: a token and the bytes it consumes, or the bytes it skips.
enum Matched {
    Token(Token, usize),
    Skip(usize),
}

/// Whether `rest` starts with `word`, ignoring ASCII case, as a case-insensitive literal does.
fn starts_with_ci(rest: &str, word: &str) -> bool {
    rest.as_bytes()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word.as_bytes()))
}

/// The literal `word` (case-insensitive) with the `\b` that Jison adds after a keyword: a word
/// character must not follow.
fn keyword_len(rest: &str, word: &str) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    match rest[word.len()..].chars().next() {
        Some(next) if is_word(next) => None,
        _ => Some(word.len()),
    }
}

/// Byte length of the run of ASCII spaces at the start of `text` (the ` *` of the rules).
fn ascii_spaces_len(text: &str) -> usize {
    text.len() - text.trim_start_matches(' ').len()
}

/// Byte length of the run of JavaScript white space at the start of `text`, line breaks included.
fn space_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// Byte length of the run of ASCII digits at the start of `text`.
fn digits_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len()
}

/// Byte length of `text` up to the first character of `stop`, or all of `text`.
fn run_until(text: &str, stop: char) -> usize {
    text.find(stop).unwrap_or(text.len())
}

/// `word` (case-insensitive), optional white space, `delimiter`, optional white space: the
/// `accTitle\s*:\s*`, `accDescr\s*:\s*` and `accDescr\s*\{\s*` rules.
fn delimited_len(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after_word = &rest[word.len()..];
    let spaces = space_run(after_word);
    let after_delimiter = after_word[spaces..].strip_prefix(delimiter)?;
    Some(word.len() + spaces + delimiter.len_utf8() + space_run(after_delimiter))
}

/// The `x-axis` family: ASCII spaces, the word (case-insensitive), and ASCII spaces.
fn spaced_word_len(rest: &str, word: &str) -> Option<usize> {
    let leading = ascii_spaces_len(rest);
    let after_leading = &rest[leading..];
    if !starts_with_ci(after_leading, word) {
        return None;
    }
    let after_word = &after_leading[word.len()..];
    Some(leading + word.len() + ascii_spaces_len(after_word))
}

/// The `-->` delimiter: ASCII spaces, two or more hyphens, `>`, and ASCII spaces.
fn delimiter_len(rest: &str) -> Option<usize> {
    let leading = ascii_spaces_len(rest);
    let after_leading = &rest[leading..];
    let hyphens = after_leading.len() - after_leading.trim_start_matches('-').len();
    if hyphens < 2 {
        return None;
    }
    let after_hyphens = after_leading[hyphens..].strip_prefix('>')?;
    Some(leading + hyphens + 1 + ascii_spaces_len(after_hyphens))
}

/// A point coordinate, `(1)|(0(.\d+)?)`. The `.` is any character but a line terminator, and it
/// is one UTF-16 unit, so a character beyond the BMP takes no digits after it.
fn coordinate_len(rest: &str) -> Option<usize> {
    if rest.starts_with('1') {
        return Some(1);
    }
    let after_zero = rest.strip_prefix('0')?;
    if let Some(any) = after_zero.chars().next() {
        let is_dot = any.len_utf16() == 1 && !matches!(any, '\n' | '\r' | '\u{2028}' | '\u{2029}');
        if is_dot {
            let digits = digits_run(&after_zero[any.len_utf8()..]);
            if digits > 0 {
                return Some(1 + any.len_utf8() + digits);
            }
        }
    }
    Some(1)
}

/// Byte length of the first character of `rest`, when it is one of `chars`.
fn one_of_len(rest: &str, chars: &[char]) -> Option<usize> {
    let first = rest.chars().next()?;
    chars.contains(&first).then_some(first.len_utf8())
}

/// Byte length of the run of characters that satisfy `keep`, or `None` when the run is empty.
fn run_len(rest: &str, keep: impl Fn(char) -> bool) -> Option<usize> {
    let len = rest.len() - rest.trim_start_matches(keep).len();
    (len > 0).then_some(len)
}

/// The characters of `[!"#$%&'*+,-.`?\\_/]`.
const PUNCTUATION: &[char] = &[
    '!', '"', '#', '$', '%', '&', '\'', '*', '+', ',', '-', '.', '`', '?', '\\', '_', '/',
];

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn state(&self) -> State {
        self.states.top()
    }

    /// The next token: Jison's `next()` called until it returns a token, or the raw end once done.
    fn next_token(&mut self) -> Spanned {
        loop {
            if self.done {
                return Spanned {
                    token: Token::RawEof,
                    start: self.pos,
                    end: self.pos,
                };
            }
            if self.rest().is_empty() {
                self.done = true;
            }
            let start = self.pos;
            match self.match_rules() {
                Some(Matched::Token(token, consumed)) => {
                    self.pos += consumed;
                    return Spanned {
                        token,
                        start,
                        end: self.pos,
                    };
                }
                Some(Matched::Skip(consumed)) => {
                    self.pos += consumed;
                }
                None => {
                    let token = if self.rest().is_empty() {
                        Token::RawEof
                    } else {
                        Token::Invalid
                    };
                    return Spanned {
                        token,
                        start,
                        end: start,
                    };
                }
            }
        }
    }

    /// The first rule of the current state that matches at the position.
    fn match_rules(&mut self) -> Option<Matched> {
        for &index in rules_of(self.state()) {
            if let Some(matched) = self.rule(index) {
                return Some(matched);
            }
        }
        None
    }

    /// Rule `index` of the generated table. The side effects of its action (the state stack)
    /// happen only when it matches.
    fn rule(&mut self, index: u8) -> Option<Matched> {
        let text = self.text;
        let rest = &text[self.pos..];
        match index {
            // `\%\%(?!\{)[^\n]*`: a comment, unless `%%` is followed by `{`.
            0 => {
                let after = rest.strip_prefix("%%")?;
                if after.starts_with('{') {
                    return None;
                }
                Some(Matched::Skip(2 + line_len(after)))
            }
            // `[^\}]\%\%[^\n]*`: one character other than `}` (a line break included), then a
            // comment. The character is one UTF-16 unit, so one beyond the BMP cannot take `%%`.
            1 => {
                let first = rest.chars().next()?;
                if first == '}' || first.len_utf16() != 1 {
                    return None;
                }
                let after = rest[first.len_utf8()..].strip_prefix("%%")?;
                Some(Matched::Skip(first.len_utf8() + 2 + line_len(after)))
            }
            // `[\n\r]+`.
            2 => run_len(rest, |c| c == '\n' || c == '\r')
                .map(|len| Matched::Token(Token::Newline, len)),
            // `\%\%[^\n]*`: a comment.
            3 => {
                let after = rest.strip_prefix("%%")?;
                Some(Matched::Skip(2 + line_len(after)))
            }
            // `title\b`: enters the title state.
            4 => {
                let len = keyword_len(rest, "title")?;
                self.states.push(State::Title);
                Some(Matched::Token(Token::Title, len))
            }
            // `[^\n]*` of the title state: the value, then leave the state.
            5 => {
                self.states.pop();
                Some(Matched::Token(Token::TitleValue, line_len(rest)))
            }
            // `accTitle\s*:\s*`: enters the accTitle state.
            6 => {
                let len = delimited_len(rest, "accTitle", ':')?;
                self.states.push(State::AccTitle);
                Some(Matched::Token(Token::AccTitle, len))
            }
            // `[^\n]*` of the accTitle state.
            7 => {
                self.states.pop();
                Some(Matched::Token(Token::AccTitleValue, line_len(rest)))
            }
            // `accDescr\s*:\s*`: enters the accDescr state.
            8 => {
                let len = delimited_len(rest, "accDescr", ':')?;
                self.states.push(State::AccDescr);
                Some(Matched::Token(Token::AccDescr, len))
            }
            // `[^\n]*` of the accDescr state.
            9 => {
                self.states.pop();
                Some(Matched::Token(Token::AccDescrValue, line_len(rest)))
            }
            // `accDescr\s*\{\s*`: enters the block; no token.
            10 => {
                let len = delimited_len(rest, "accDescr", '{')?;
                self.states.push(State::AccDescrMulti);
                Some(Matched::Skip(len))
            }
            // `\}` of the block: leaves the block; no token.
            11 => {
                rest.starts_with('}').then_some(())?;
                self.states.pop();
                Some(Matched::Skip(1))
            }
            // `[^\}]*` of the block: its text, which may be empty.
            12 => Some(Matched::Token(
                Token::AccDescrMultiValue,
                run_until(rest, '}'),
            )),
            // ` *x-axis *`.
            13 => spaced_word_len(rest, "x-axis").map(|len| Matched::Token(Token::XAxis, len)),
            // ` *y-axis *`.
            14 => spaced_word_len(rest, "y-axis").map(|len| Matched::Token(Token::YAxis, len)),
            // ` *--+> *`.
            15 => delimiter_len(rest).map(|len| Matched::Token(Token::AxisDelimiter, len)),
            // ` *quadrant-N *`.
            16..=19 => {
                let word = match index {
                    16 => "quadrant-1",
                    17 => "quadrant-2",
                    18 => "quadrant-3",
                    _ => "quadrant-4",
                };
                let len = spaced_word_len(rest, word)?;
                let token = match index {
                    16 => Token::Quadrant1,
                    17 => Token::Quadrant2,
                    18 => Token::Quadrant3,
                    _ => Token::Quadrant4,
                };
                Some(Matched::Token(token, len))
            }
            // `classDef\b`.
            20 => keyword_len(rest, "classDef").map(|len| Matched::Token(Token::ClassDef, len)),
            // `["][`]`: enters the Markdown string; no token.
            21 => {
                rest.starts_with("\"`").then_some(())?;
                self.states.push(State::MdString);
                Some(Matched::Skip(2))
            }
            // `[^`"]+` of the Markdown string.
            22 => {
                run_len(rest, |c| c != '`' && c != '"').map(|len| Matched::Token(Token::MdStr, len))
            }
            // `` [`]["] `` leaves the Markdown string; no token.
            23 => {
                rest.starts_with("`\"").then_some(())?;
                self.states.pop();
                Some(Matched::Skip(2))
            }
            // `["]`: enters the quoted string; no token.
            24 => {
                rest.starts_with('"').then_some(())?;
                self.states.push(State::Str);
                Some(Matched::Skip(1))
            }
            // `["]` of the quoted string: leaves it; no token.
            25 => {
                rest.starts_with('"').then_some(())?;
                self.states.pop();
                Some(Matched::Skip(1))
            }
            // `[^"]*` of the quoted string: its text, which may be empty.
            26 => Some(Matched::Token(Token::Str, run_until(rest, '"'))),
            // `:::`: enters the class name; no token.
            27 => {
                rest.starts_with(":::").then_some(())?;
                self.states.push(State::ClassName);
                Some(Matched::Skip(3))
            }
            // `^\w+` of the class name: leaves the state.
            28 => {
                let len = rest.len() - rest.trim_start_matches(is_word).len();
                if len == 0 {
                    return None;
                }
                self.states.pop();
                Some(Matched::Token(Token::ClassName, len))
            }
            // `\s*:\s*\[\s*`: enters the point.
            29 => {
                let before = space_run(rest);
                let after_colon = rest[before..].strip_prefix(':')?;
                let after_spaces = space_run(after_colon);
                let after_bracket = after_colon[after_spaces..].strip_prefix('[')?;
                let len = before + 1 + after_spaces + 1 + space_run(after_bracket);
                self.states.push(State::PointStart);
                Some(Matched::Token(Token::PointStart, len))
            }
            // The x coordinate, after the `[`: enters the x state.
            30 => {
                let len = coordinate_len(rest)?;
                self.states.push(State::PointX);
                Some(Matched::Token(Token::PointX, len))
            }
            // `\s*\] *`: the point ends; no token.
            31 => {
                let before = space_run(rest);
                let after = rest[before..].strip_prefix(']')?;
                let len = before + 1 + ascii_spaces_len(after);
                self.states.pop();
                Some(Matched::Skip(len))
            }
            // `\s*,\s*`: the x state ends and the y state begins; no token.
            32 => {
                let before = space_run(rest);
                let after = rest[before..].strip_prefix(',')?;
                let len = before + 1 + space_run(after);
                self.states.pop();
                self.states.push(State::PointY);
                Some(Matched::Skip(len))
            }
            // The y coordinate: leaves the y state.
            33 => {
                let len = coordinate_len(rest)?;
                self.states.pop();
                Some(Matched::Token(Token::PointY, len))
            }
            // ` *quadrantChart *`: the header.
            34 => spaced_word_len(rest, "quadrantChart")
                .map(|len| Matched::Token(Token::Quadrant, len)),
            // `[A-Za-z]+`.
            35 => run_len(rest, |c| c.is_ascii_alphabetic())
                .map(|len| Matched::Token(Token::Alpha, len)),
            // `[^\x00-\x7F]+`.
            36 => {
                run_len(rest, |c| !c.is_ascii()).map(|len| Matched::Token(Token::UnicodeText, len))
            }
            // `:`.
            37 => one_of_len(rest, &[':']).map(|len| Matched::Token(Token::Colon, len)),
            // `\+`.
            38 => one_of_len(rest, &['+']).map(|len| Matched::Token(Token::Plus, len)),
            // `,`.
            39 => one_of_len(rest, &[',']).map(|len| Matched::Token(Token::Comma, len)),
            // `=`, twice in the table.
            40 | 41 => one_of_len(rest, &['=']).map(|len| Matched::Token(Token::Equals, len)),
            // `*`.
            42 => one_of_len(rest, &['*']).map(|len| Matched::Token(Token::Mult, len)),
            // `#`.
            43 => one_of_len(rest, &['#']).map(|len| Matched::Token(Token::Brkt, len)),
            // `_`.
            44 => one_of_len(rest, &['_']).map(|len| Matched::Token(Token::Underscore, len)),
            // `.`.
            45 => one_of_len(rest, &['.']).map(|len| Matched::Token(Token::Dot, len)),
            // `&`.
            46 => one_of_len(rest, &['&']).map(|len| Matched::Token(Token::Amp, len)),
            // `-`.
            47 => one_of_len(rest, &['-']).map(|len| Matched::Token(Token::Minus, len)),
            // `[0-9]+`.
            48 => run_len(rest, |c| c.is_ascii_digit()).map(|len| Matched::Token(Token::Num, len)),
            // `\s`: one white space character.
            49 => {
                let first = rest.chars().next()?;
                is_js_space(first).then(|| Matched::Token(Token::Space, first.len_utf8()))
            }
            // `;`.
            50 => one_of_len(rest, &[';']).map(|len| Matched::Token(Token::Semi, len)),
            // `[!"#$%&'*+,-.`?\\_/]`.
            51 => one_of_len(rest, PUNCTUATION).map(|len| Matched::Token(Token::Punctuation, len)),
            // `<<EOF>>`: at the end of the input.
            52 => rest.is_empty().then_some(Matched::Token(Token::Eof, 0)),
            _ => None,
        }
    }
}
