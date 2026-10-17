//! The mindmap scanner: the rules of `mindmap.jison`, in the file's order. Jison takes the first
//! rule that matches at the current position, not the longest, so the order below is the order of
//! the file. The lexer has five exclusive states, entered by the shape, description, icon and class
//! rules; a rule without a state belongs to the initial state only. See `docs-port/mindmap/PLAN.md`,
//! section 3, for the table and the quirks `M1`-`M16`.
//!
//! At the end of the text the rules of the current state are read, as Jison's `next()` reads them
//! once the input is empty: the initial state gives the end token, and an exclusive state gives
//! Jison's raw end symbol, which the grammar does not accept. A text that ends inside a shape, an
//! icon or a class is therefore rejected (M10).

use crate::scan::StateStack;
use crate::text::{is_js_space, is_line_term, is_word};

/// The start conditions of the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// The initial state, where the rules without a condition apply.
    Initial,
    /// Inside a shape: the rules `<NODE>`.
    Node,
    /// Inside a quoted description: `<NSTR>`.
    NodeStr,
    /// Inside a description quoted with a backtick: `<NSTR2>`.
    NodeStr2,
    /// Inside `::icon(`: `<ICON>`.
    Icon,
    /// After `:::`: `<CLASS>`.
    Class,
}

/// The tokens of the grammar, with the value of the rules that carry one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `SPACELINE`: blank lines, or a comment (`\s*%%.*`).
    SpaceLine,
    /// `MINDMAP`: the keyword, as a whole word.
    Mindmap,
    /// `NL`: one line break.
    Newline,
    /// `SPACELIST`: indentation, with its length in characters, which is the node's level.
    SpaceList(usize),
    /// `NODE_ID`: the id of a node, everything up to a bracket.
    NodeId(String),
    /// `NODE_DSTART`: the opening of a shape, such as `[` or `((`.
    NodeStart(String),
    /// `NODE_DESCR`: the text of a shape.
    NodeDescr(String),
    /// `NODE_DEND`: the closing of a shape, such as `]` or `))`.
    NodeEnd(String),
    /// `ICON`: the text of `::icon(…)`.
    Icon(String),
    /// `CLASS`: the rest of the line after `:::`.
    Class(String),
    /// `EOF`: the end of the text, as the grammar's terminal.
    Eof,
    /// Jison's raw end symbol: the text ended in an exclusive state. The grammar does not accept it.
    RawEof,
    /// A character no rule takes: Jison's lexical error.
    Invalid,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// What a rule does when it matches: how many bytes it takes, the token it returns (if any), and
/// the state it enters or leaves.
struct Matched {
    len: usize,
    token: Option<Token>,
    enter: Option<State>,
    leave: bool,
}

impl Matched {
    fn token(len: usize, token: Token) -> Option<Self> {
        Some(Self {
            len,
            token: Some(token),
            enter: None,
            leave: false,
        })
    }

    fn enter(len: usize, state: State) -> Option<Self> {
        Some(Self {
            len,
            token: None,
            enter: Some(state),
            leave: false,
        })
    }

    fn leave(len: usize, token: Option<Token>) -> Option<Self> {
        Some(Self {
            len,
            token,
            enter: None,
            leave: true,
        })
    }
}

/// Reads the text with Jison's rules. The result ends with the first `Eof`, `RawEof` or `Invalid`.
pub(crate) fn scan(text: &str) -> Vec<Spanned> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        states: StateStack::new(State::Initial),
    };
    let mut tokens = Vec::new();
    loop {
        match scanner.step() {
            Step::Token(spanned) => {
                let ends = matches!(spanned.token, Token::Eof | Token::RawEof | Token::Invalid);
                tokens.push(spanned);
                if ends {
                    return tokens;
                }
            }
            Step::Skip => {}
        }
    }
}

/// The result of one call to Jison's `next()`: a token, or an action that returns none.
enum Step {
    Token(Spanned),
    Skip,
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    states: StateStack<State>,
}

impl Scanner<'_> {
    fn step(&mut self) -> Step {
        let start = self.pos;
        let rest = &self.text[self.pos..];
        let matched = match self.states.top() {
            State::Initial => initial_rule(rest),
            State::Node => node_rule(rest),
            State::NodeStr2 => node_str2_rule(rest),
            State::NodeStr => node_str_rule(rest),
            State::Icon => icon_rule(rest),
            State::Class => class_rule(rest),
        };
        let Some(matched) = matched else {
            // No rule takes the text. At the end, Jison returns its raw end symbol; elsewhere it
            // throws, which the scanner reports as an invalid token.
            let token = if rest.is_empty() {
                Token::RawEof
            } else {
                Token::Invalid
            };
            return Step::Token(Spanned { token, start });
        };
        self.pos += matched.len;
        if let Some(state) = matched.enter {
            self.states.push(state);
        }
        if matched.leave {
            self.states.pop();
        }
        match matched.token {
            Some(token) => Step::Token(Spanned { token, start }),
            None => Step::Skip,
        }
    }
}

/// Byte length of the run of JavaScript white space at the start of `text`, line breaks included.
fn space_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// Byte length of the run of characters that `.` takes: all but the line terminators.
fn dot_run(text: &str) -> usize {
    text.find(is_line_term).unwrap_or(text.len())
}

/// Byte length of the run of characters before the first of `stops`, or of all of `text`.
fn run_until(text: &str, stops: &[char]) -> usize {
    text.find(|c: char| stops.contains(&c))
        .unwrap_or(text.len())
}

/// Whether `rest` starts with `word`, ignoring ASCII case.
fn starts_with_ci(rest: &str, word: &str) -> bool {
    rest.as_bytes()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word.as_bytes()))
}

/// The rules of the initial state, in the order of `mindmap.jison` (lines 21-44).
fn initial_rule(rest: &str) -> Option<Matched> {
    // `\s*\%\%.*`: a comment, which takes the blanks and line breaks before its `%%`.
    let spaces = space_run(rest);
    if let Some(after) = rest[spaces..].strip_prefix("%%") {
        return Matched::token(spaces + 2 + dot_run(after), Token::SpaceLine);
    }
    // `"mindmap"`, with the word boundary that Jison adds after a word character.
    if starts_with_ci(rest, "mindmap")
        && !rest
            .get(7..)
            .and_then(|after| after.chars().next())
            .is_some_and(is_word)
    {
        return Matched::token(7, Token::Mindmap);
    }
    if rest.starts_with(":::") {
        return Matched::enter(3, State::Class);
    }
    if starts_with_ci(rest, "::icon(") {
        return Matched::enter(7, State::Icon);
    }
    // `[\s]+[\n]`: white space that ends in a line break after its first character. The match
    // runs to the last line break of the white space.
    let blanks = &rest[..space_run(rest)];
    if let Some(last) = blanks.rfind('\n').filter(|&last| last > 0) {
        return Matched::token(last + 1, Token::SpaceLine);
    }
    if rest.starts_with('\n') {
        let breaks = rest.len() - rest.trim_start_matches('\n').len();
        return Matched::token(breaks, Token::Newline);
    }
    for literal in ["-)", "(-", "))", ")", "((", "{{", "(", "["] {
        if rest.starts_with(literal) {
            return Some(Matched {
                len: literal.len(),
                token: Some(Token::NodeStart(literal.to_owned())),
                enter: Some(State::Node),
                leave: false,
            });
        }
    }
    // `[\s]+`: indentation. Its length is the level of the node.
    let spaces = space_run(rest);
    if spaces > 0 {
        return Matched::token(spaces, Token::SpaceList(rest[..spaces].chars().count()));
    }
    // `[^\(\[\n\)\{\}]+`: the id, up to a bracket.
    let id = run_until(rest, &['(', '[', '\n', ')', '{', '}']);
    if id > 0 {
        return Matched::token(id, Token::NodeId(rest[..id].to_owned()));
    }
    if rest.is_empty() {
        return Matched::token(0, Token::Eof);
    }
    None
}

/// The rules of the `NODE` state (lines 45-60). The description rules of `NSTR`, `NSTR2` are
/// separate states; the quote rules of this state enter them.
fn node_rule(rest: &str) -> Option<Matched> {
    if rest.starts_with("\"`") {
        return Matched::enter(2, State::NodeStr2);
    }
    if rest.starts_with('"') {
        return Matched::enter(1, State::NodeStr);
    }
    for text in ["))", ")", "]", "}}", "(-", "-)", "((", "("] {
        if rest.starts_with(text) {
            return Matched::leave(text.len(), Some(Token::NodeEnd(text.to_owned())));
        }
    }
    // `[^\)\]\(\}]+`: the description.
    let description = run_until(rest, &[')', ']', '(', '}']);
    if description > 0 {
        return Matched::token(
            description,
            Token::NodeDescr(rest[..description].to_owned()),
        );
    }
    // `.+`: the rest of the line, which takes a lone `}`.
    let line = dot_run(rest);
    if line > 0 {
        return Matched::token(line, Token::NodeDescr(rest[..line].to_owned()));
    }
    None
}

/// The rules of the `NSTR2` state: a description quoted with a backtick.
fn node_str2_rule(rest: &str) -> Option<Matched> {
    let text = run_until(rest, &['`', '"']);
    if text > 0 {
        return Matched::token(text, Token::NodeDescr(rest[..text].to_owned()));
    }
    if rest.starts_with("`\"") {
        return Matched::leave(2, None);
    }
    None
}

/// The rules of the `NSTR` state: a description in double quotes.
fn node_str_rule(rest: &str) -> Option<Matched> {
    let text = run_until(rest, &['"']);
    if text > 0 {
        return Matched::token(text, Token::NodeDescr(rest[..text].to_owned()));
    }
    if rest.starts_with('"') {
        return Matched::leave(1, None);
    }
    None
}

/// The rules of the `ICON` state: the text up to the first `)`.
fn icon_rule(rest: &str) -> Option<Matched> {
    let text = run_until(rest, &[')']);
    if text > 0 {
        return Matched::token(text, Token::Icon(rest[..text].to_owned()));
    }
    if rest.starts_with(')') {
        return Matched::leave(1, None);
    }
    None
}

/// The rules of the `CLASS` state: the rest of the line, or the line break that ends an empty class.
fn class_rule(rest: &str) -> Option<Matched> {
    let line = dot_run(rest);
    if line > 0 {
        return Matched::leave(line, Some(Token::Class(rest[..line].to_owned())));
    }
    if rest.starts_with('\n') {
        return Matched::leave(1, None);
    }
    None
}
