//! The timeline lexer: the rules of `timeline.jison`, in the grammar's order. Jison takes the first
//! rule that matches at the current position, not the longest, and the keywords are
//! case-insensitive. The rules are numbered as in `docs-port/timeline/PLAN.md`, section 3.
//!
//! The whole prepared text is tokenised before the parser runs. The lexer's states depend on the
//! text only, never on the grammar's actions, so the token sequence is the same either way.

use crate::scan::{StateStack, line_len, space_len};
use crate::text::{is_js_space, is_word};

/// The start conditions of the timeline lexer: the initial state and the three states that the
/// accessibility rules enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    AccTitle,
    AccDescr,
    AccDescrBlock,
}

/// The header keyword: `timeline`, `timeline LR` or `timeline TD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Header {
    Plain,
    Lr,
    Td,
}

/// A token of the timeline grammar. The text of a keyword token is the whole match, and the
/// grammar's actions cut the keyword off it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    Newline,
    Header(Header),
    Title(String),
    AccTitle,
    AccTitleValue(String),
    AccDescr,
    AccDescrValue(String),
    AccDescrBlockValue(String),
    Section(String),
    Event(String),
    Period(String),
    Invalid,
    Eof,
}

/// A token with the byte offset where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub token: Token,
    pub start: usize,
}

/// What a rule matched: its length in bytes, the token it returns, and its effect on the states.
struct Match {
    len: usize,
    token: Option<Token>,
    change: Change,
}

/// The state change a rule makes after it matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Stay,
    Enter(State),
    Leave,
}

impl Match {
    fn new(len: usize, token: Option<Token>, change: Change) -> Self {
        Self { len, token, change }
    }
}

/// Tokenises the whole text, which must end with a line break, as `Diagram.fromText` makes it.
#[must_use]
pub(crate) fn tokenize(text: &str) -> Vec<Spanned> {
    let mut tokens = Vec::new();
    let mut states = StateStack::new(State::Initial);
    let mut pos = 0;
    loop {
        let rest = &text[pos..];
        if rest.is_empty() {
            // The end of the input. The initial state returns `EOF`. A value state matches the
            // empty string once, and the lexer then answers the end of the input, which the
            // grammar does not accept where it wants `EOF`.
            let token = match states.top() {
                State::Initial => Token::Eof,
                State::AccTitle => Token::AccTitleValue(String::new()),
                State::AccDescr => Token::AccDescrValue(String::new()),
                State::AccDescrBlock => Token::AccDescrBlockValue(String::new()),
            };
            tokens.push(Spanned { token, start: pos });
            break;
        }
        let matched = match_rule(states.top(), rest).unwrap_or_else(|| {
            // Unreachable: every state has a rule for each character. Kept as an error token
            // so that the lexer always advances.
            let len = rest.chars().next().map_or(1, char::len_utf8);
            Match::new(len, Some(Token::Invalid), Change::Stay)
        });
        if let Some(token) = matched.token {
            tokens.push(Spanned { token, start: pos });
        }
        match matched.change {
            Change::Stay => {}
            Change::Enter(state) => states.push(state),
            Change::Leave => states.pop(),
        }
        pos += matched.len;
    }
    tokens
}

/// The rule of `state` that matches at the start of `rest`, if any.
fn match_rule(state: State, rest: &str) -> Option<Match> {
    match state {
        State::Initial => initial_rule(rest),
        // `[^\n]*`, which may be empty, and leaves the state.
        State::AccTitle => Some(value_line(rest, Token::AccTitleValue, Change::Leave)),
        State::AccDescr => Some(value_line(rest, Token::AccDescrValue, Change::Leave)),
        State::AccDescrBlock => {
            if rest.starts_with('}') {
                Some(Match::new(1, None, Change::Leave))
            } else {
                // `[^}]*`: the block keeps its line breaks, comments and entities as text.
                let len = rest.find('}').unwrap_or(rest.len());
                Some(Match::new(
                    len,
                    Some(Token::AccDescrBlockValue(rest[..len].to_owned())),
                    Change::Stay,
                ))
            }
        }
    }
}

/// The rest of the line as a value token, `[^\n]*`.
fn value_line(rest: &str, token: fn(String) -> Token, change: Change) -> Match {
    let len = line_len(rest);
    Match::new(len, Some(token(rest[..len].to_owned())), change)
}

/// The rules of the initial state, in the order of `timeline.jison`.
fn initial_rule(rest: &str) -> Option<Match> {
    // 0: `%` that does not start `%{`, to the end of the line.
    if let Some(after) = rest.strip_prefix('%')
        && !after.starts_with('{')
    {
        return Some(Match::new(1 + line_len(after), None, Change::Stay));
    }

    // 1: one character other than `}`, then `%%`, to the end of the line. The character is part
    // of the comment, so `x%%c` is a comment and `abc%%c` is not.
    if let Some(first) = rest.chars().next()
        && first != '}'
        && let Some(after) = rest[first.len_utf8()..].strip_prefix("%%")
    {
        let len = first.len_utf8() + 2 + line_len(after);
        return Some(Match::new(len, None, Change::Stay));
    }

    // 2: one or more line breaks.
    if rest.starts_with('\n') {
        let len = rest.len() - rest.trim_start_matches('\n').len();
        return Some(Match::new(len, Some(Token::Newline), Change::Stay));
    }

    // 3: blanks, which take the line breaks after them, so no `NEWLINE` follows them.
    let spaces = space_len(rest);
    if spaces > 0 {
        return Some(Match::new(spaces, None, Change::Stay));
    }

    // 4: `#` to the end of the line.
    if rest.starts_with('#') {
        return Some(Match::new(line_len(rest), None, Change::Stay));
    }

    // 5 and 6: `timeline`, blanks or tabs, then `LR` or `TD` at a word boundary.
    if let Some(after) = ci_prefix(rest, "timeline") {
        let blanks = after.len() - after.trim_start_matches([' ', '\t']).len();
        if blanks > 0 {
            let tail = &after[blanks..];
            for (word, header) in [("lr", Header::Lr), ("td", Header::Td)] {
                if let Some(after_word) = ci_prefix(tail, word)
                    && word_ends(after_word)
                {
                    let len = rest.len() - after_word.len();
                    return Some(Match::new(len, Some(Token::Header(header)), Change::Stay));
                }
            }
        }
        // 7: `timeline` at a word boundary.
        if word_ends(after) {
            let len = rest.len() - after.len();
            return Some(Match::new(
                len,
                Some(Token::Header(Header::Plain)),
                Change::Stay,
            ));
        }
    }

    // 8: `title`, one blank character, then the rest of the line, which must not be empty. The
    // blank may be a line break.
    if let Some(after) = ci_prefix(rest, "title")
        && let Some(blank) = after.chars().next().filter(|&c| is_js_space(c))
    {
        let tail = &after[blank.len_utf8()..];
        let value_len = line_len(tail);
        if value_len > 0 {
            let len = rest.len() - tail[value_len..].len();
            let token = Token::Title(rest[..len].to_owned());
            return Some(Match::new(len, Some(token), Change::Stay));
        }
    }

    // 9: `accTitle`, optional blanks, `:`, optional blanks. The blanks may take line breaks.
    if let Some(after) = ci_prefix(rest, "acctitle")
        && let Some(tail) = colon_form(after)
    {
        let len = rest.len() - tail.len();
        return Some(Match::new(
            len,
            Some(Token::AccTitle),
            Change::Enter(State::AccTitle),
        ));
    }

    // 11: `accDescr`, optional blanks, `:`, optional blanks.
    if let Some(after) = ci_prefix(rest, "accdescr")
        && let Some(tail) = colon_form(after)
    {
        let len = rest.len() - tail.len();
        return Some(Match::new(
            len,
            Some(Token::AccDescr),
            Change::Enter(State::AccDescr),
        ));
    }

    // 13: `accDescr`, optional blanks, `{`, optional blanks: the block form, with no token.
    if let Some(after) = ci_prefix(rest, "accdescr")
        && let Some(tail) = after.trim_start_matches(is_js_space).strip_prefix('{')
    {
        let tail = tail.trim_start_matches(is_js_space);
        let len = rest.len() - tail.len();
        return Some(Match::new(len, None, Change::Enter(State::AccDescrBlock)));
    }

    // 16: `section`, one blank character, then the name, which stops at `:` or a line break.
    if let Some(after) = ci_prefix(rest, "section")
        && let Some(blank) = after.chars().next().filter(|&c| is_js_space(c))
    {
        let tail = &after[blank.len_utf8()..];
        let name_len = tail.find([':', '\n']).unwrap_or(tail.len());
        if name_len > 0 {
            let len = rest.len() - tail[name_len..].len();
            let token = Token::Section(rest[..len].to_owned());
            return Some(Match::new(len, Some(token), Change::Stay));
        }
    }

    // 17: `:`, one blank character, then the text. A `:` in the text must not be followed by a
    // blank: `: a:b` is the event `a:b`, and `: a: b` is two events, `a` and `b`.
    if let Some(after) = rest.strip_prefix(':')
        && let Some(blank) = after.chars().next().filter(|&c| is_js_space(c))
    {
        let body = &after[blank.len_utf8()..];
        let body_len = event_text_len(body);
        if body_len > 0 {
            let len = 1 + blank.len_utf8() + body_len;
            let token = Token::Event(rest[..len].to_owned());
            return Some(Match::new(len, Some(token), Change::Stay));
        }
    }

    // 18: a run of characters that are not `#`, `:` or a line break.
    let period_len = rest.find(['#', ':', '\n']).unwrap_or(rest.len());
    if period_len > 0 {
        let token = Token::Period(rest[..period_len].to_owned());
        return Some(Match::new(period_len, Some(token), Change::Stay));
    }

    // 20: any other character is invalid. The end of the input is handled by the caller.
    let first = rest.chars().next()?;
    Some(Match::new(
        first.len_utf8(),
        Some(Token::Invalid),
        Change::Stay,
    ))
}

/// `word` at the start of `text`, compared without regard to ASCII case. Returns the rest.
fn ci_prefix<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let head = text.get(..word.len())?;
    head.eq_ignore_ascii_case(word).then(|| &text[word.len()..])
}

/// Whether a keyword that ends at the start of `after` ends at a word boundary, which is what a
/// `\b` after a word character means.
fn word_ends(after: &str) -> bool {
    !after.chars().next().is_some_and(is_word)
}

/// `\s*:\s*`, the colon form of `accTitle` and `accDescr`. Returns the text after it.
fn colon_form(after: &str) -> Option<&str> {
    let tail = after.trim_start_matches(is_js_space).strip_prefix(':')?;
    Some(tail.trim_start_matches(is_js_space))
}

/// The length of the text of an event: `(?:[^:\n]|:(?!\s))+`. A colon belongs to the text only
/// when no blank follows it. Returns 0 when the text is empty.
fn event_text_len(body: &str) -> usize {
    let mut end = 0;
    let mut chars = body.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        let belongs = match c {
            '\n' => false,
            ':' => chars.peek().is_none_or(|&(_, next)| !is_js_space(next)),
            _ => true,
        };
        if !belongs {
            break;
        }
        end = index + c.len_utf8();
    }
    end
}
