//! The journey scanner: the rules of `journey.jison`, in the grammar's order. The first rule that
//! matches wins, as in Jison, and the keywords are case-insensitive. See
//! `docs-port/journey/PLAN.md`, sections 2 and 3, for the rule table and the quirks `J1`-`J4`.
//!
//! The scanner reproduces Jison's `next()`, including its `done` flag: once the input is empty,
//! the next call reads the rules of the current state at the end of the input, and after that
//! every call returns Jison's raw end symbol, which is not the grammar's `EOF` terminal. A text
//! that ends inside an accessibility value is therefore rejected (`J4`).

use crate::scan::line_len;
use crate::text::{is_js_space, is_word};

/// The start conditions of the lexer: the initial state, and the exclusive states that the
/// accessibility rules enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    AccTitle,
    AccDescr,
    AccDescrMulti,
}

/// The tokens of the grammar. Keywords that carry no value are unit variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `journey`.
    Journey,
    /// `title`, with the value after its first six characters.
    Title(String),
    /// `accTitle:`, which is followed by its value.
    AccTitle,
    /// The rest of the line after `accTitle:`.
    AccTitleValue(String),
    /// `accDescr:`, which is followed by its value.
    AccDescr,
    /// The rest of the line after `accDescr:`.
    AccDescrValue(String),
    /// The text of an `accDescr { … }` block, up to its `}`.
    AccDescrMultiValue(String),
    /// `section`, with the name after its first eight characters.
    Section(String),
    /// A task name, with its blanks at the end.
    TaskName(String),
    /// A colon and the task data that follows it, colon included.
    TaskData(String),
    /// A colon on its own: an error in the grammar.
    Colon,
    /// One or more line breaks.
    Newline,
    /// A character no rule takes: an error in the grammar.
    Invalid,
    /// The end of the input, as the `EOF` terminal of the grammar.
    Eof,
    /// Jison's raw end symbol, returned once the input is done. The grammar does not accept it,
    /// so a text that ends inside an accessibility value is rejected (`J4`).
    RawEof,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// Reads the whole text. The result ends with the first end token, `Eof` or `RawEof`.
pub(crate) fn scan(text: &str) -> Vec<Spanned> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        state: State::Initial,
        done: false,
    };
    let mut tokens = Vec::new();
    loop {
        let spanned = scanner.next_token();
        let is_end = matches!(spanned.token, Token::Eof | Token::RawEof);
        tokens.push(spanned);
        if is_end {
            return tokens;
        }
    }
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    state: State,
    done: bool,
}

/// Whether `rest` starts with `word`, ignoring ASCII case.
fn starts_with_ci(rest: &str, word: &str) -> bool {
    rest.as_bytes()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word.as_bytes()))
}

/// Byte length of the run of JavaScript white space at the start of `text`, line breaks included.
fn space_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// Byte length of the text before the first of `stops`, or of all of `text` when none is there.
fn run_until(text: &str, stops: &[char]) -> usize {
    text.find(|c: char| stops.contains(&c))
        .unwrap_or(text.len())
}

/// Byte length of the first character of `text`, zero for the empty text.
fn first_len(text: &str) -> usize {
    text.chars().next().map_or(0, char::len_utf8)
}

/// The keyword `word` (case-insensitive), one white space character, then a run of characters that
/// none of `stops` is: the `title` and `section` rules. Returns the value, which is the text after
/// the keyword and its white space character, and the length of the whole match.
fn keyword_with_value(rest: &str, word: &str, stops: &[char]) -> Option<(String, usize)> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after = &rest[word.len()..];
    let blank = first_len(after);
    if blank == 0 || !after.starts_with(is_js_space) {
        return None;
    }
    let body = run_until(&after[blank..], stops);
    if body == 0 {
        return None;
    }
    let consumed = word.len() + blank + body;
    Some((rest[word.len() + blank..consumed].to_owned(), consumed))
}

/// `word` (case-insensitive), optional white space, `delimiter`, and optional white space: the
/// `accTitle\s*":"\s*`, `accDescr\s*":"\s*` and `accDescr\s*"{"\s*` rules. The length of the match.
fn delimited_len(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    if !starts_with_ci(rest, word) {
        return None;
    }
    let after_word = &rest[word.len()..];
    let spaces = space_run(after_word);
    let after_delimiter = after_word[spaces..].strip_prefix(delimiter)?;
    Some(word.len() + spaces + delimiter.len_utf8() + space_run(after_delimiter))
}

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
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
            let outcome = match self.state {
                State::Initial => self.initial_rule(),
                State::AccTitle => Some(self.value_rule(Token::AccTitleValue)),
                State::AccDescr => Some(self.value_rule(Token::AccDescrValue)),
                State::AccDescrMulti => self.multi_rule(),
            };
            match outcome {
                Some((token, consumed)) => {
                    self.pos = start + consumed;
                    return Spanned { token, start };
                }
                None if self.pos == start && !self.rest().is_empty() => {
                    // No rule takes this character: `.`, the rule that catches everything else.
                    let consumed = first_len(self.rest());
                    self.pos += consumed;
                    return Spanned {
                        token: Token::Invalid,
                        start,
                    };
                }
                // A skipping rule moved `pos`; the next rule starts from there.
                None => {}
            }
        }
    }

    /// The rules of the initial state, in the grammar's order. A rule that only skips moves `pos`
    /// itself and returns `None`; a rule that makes a token returns it with its length.
    fn initial_rule(&mut self) -> Option<(Token, usize)> {
        let rest = self.rest();
        let c = rest.chars().next();

        // 1. `\%%(?!\{)[^\n]*`: a comment line.
        if rest.starts_with("%%") && !rest[2..].starts_with('{') {
            self.pos += 2 + line_len(&rest[2..]);
            return None;
        }
        // 2. `[^\}]\%\%[^\n]*`: one character other than `}`, then a comment (J2).
        if let Some(first) = c {
            let after = &rest[first.len_utf8()..];
            if first != '}' && after.starts_with("%%") {
                self.pos += first.len_utf8() + 2 + line_len(&after[2..]);
                return None;
            }
        }
        // 3. `[\n]+`: NEWLINE.
        if c == Some('\n') {
            let blanks = rest.len() - rest.trim_start_matches('\n').len();
            return Some((Token::Newline, blanks));
        }
        // 4. `\s+`: white space, which runs over line breaks when it starts with a blank.
        if c.is_some_and(is_js_space) {
            self.pos += space_run(rest);
            return None;
        }
        // 5. `\#[^\n]*`: a comment to the end of the line.
        if c == Some('#') {
            self.pos += line_len(rest);
            return None;
        }
        // 6. `"journey"`, with the implicit word boundary of a keyword literal (J1).
        if starts_with_ci(rest, "journey") && !rest[7..].starts_with(is_word) {
            return Some((Token::Journey, 7));
        }
        // 7. `"title"\s[^#\n;]+` (J5).
        if let Some((value, consumed)) = keyword_with_value(rest, "title", &['#', '\n', ';']) {
            return Some((Token::Title(value), consumed));
        }
        // 8. `accTitle\s*":"\s*`: enters the acc_title state (J3).
        if let Some(consumed) = delimited_len(rest, "acctitle", ':') {
            self.state = State::AccTitle;
            return Some((Token::AccTitle, consumed));
        }
        // 10. `accDescr\s*":"\s*`: enters the acc_descr state.
        if let Some(consumed) = delimited_len(rest, "accdescr", ':') {
            self.state = State::AccDescr;
            return Some((Token::AccDescr, consumed));
        }
        // 12. `accDescr\s*"{"\s*`: enters the multi-line state, without a token.
        if let Some(consumed) = delimited_len(rest, "accdescr", '{') {
            self.state = State::AccDescrMulti;
            self.pos += consumed;
            return None;
        }
        // 15. `"section"\s[^#:\n;]+` (J6).
        if let Some((name, consumed)) = keyword_with_value(rest, "section", &['#', ':', '\n', ';'])
        {
            return Some((Token::Section(name), consumed));
        }
        // 16. `[^#:\n;]+`: a task name, blanks at its end included (J7).
        let name = run_until(rest, &['#', ':', '\n', ';']);
        if name > 0 {
            return Some((Token::TaskName(rest[..name].to_owned()), name));
        }
        // 17. `":"[^#\n;]+`: a colon with its task data.
        if c == Some(':') {
            let data = 1 + run_until(&rest[1..], &['#', '\n', ';']);
            if data > 1 {
                return Some((Token::TaskData(rest[..data].to_owned()), data));
            }
            // 18. `":"`: a colon on its own.
            return Some((Token::Colon, 1));
        }
        // 19. `<<EOF>>`: the end of the input.
        if rest.is_empty() {
            return Some((Token::Eof, 0));
        }
        // 20. `.`: any other character. The caller reports it as INVALID.
        None
    }

    /// The `acc_title` and `acc_descr` states: `[^\n]*`, the rest of the line, possibly empty. The
    /// state is left at once.
    fn value_rule(&mut self, make: fn(String) -> Token) -> (Token, usize) {
        let consumed = line_len(self.rest());
        let value = self.rest()[..consumed].to_owned();
        self.state = State::Initial;
        (make(value), consumed)
    }

    /// The `acc_descr_multiline` state: `}` leaves it, and the text up to a `}` is one value.
    fn multi_rule(&mut self) -> Option<(Token, usize)> {
        let rest = self.rest();
        if rest.starts_with('}') {
            self.state = State::Initial;
            self.pos += 1;
            return None;
        }
        let consumed = run_until(rest, &['}']);
        Some((
            Token::AccDescrMultiValue(rest[..consumed].to_owned()),
            consumed,
        ))
    }
}
