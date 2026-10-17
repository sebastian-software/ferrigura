//! The kanban scanner: the rules of `kanban.jison`, in the grammar's order. The first rule that
//! matches wins, as in Jison, and a keyword literal ends with a word boundary. The grammar's start
//! conditions are the states below. See `docs-port/kanban/PLAN.md`, section 2.
//!
//! The scanner reproduces Jison's `next()`, including its `done` flag. Once the input is empty, the
//! rules of the current state are tried on it: the initial state's end rule gives the grammar's
//! `EOF`, and every other call returns Jison's raw end symbol ([`Token::End`]), which the grammar
//! accepts only after `EOF`. A text that ends inside a node, a shape data block, an icon or a class
//! is therefore rejected.

use crate::scan::{StateStack, space_len};
use crate::text::{is_js_space, is_line_term, is_word};

/// The start conditions of the kanban lexer: the initial state and the seven exclusive states of
/// `kanban.jison` (`%x`). Rules without a state prefix apply in the initial state only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    /// `NODE`: inside the brackets of a node.
    Node,
    /// `NSTR`: a quoted description, `"…"`.
    NStr,
    /// `NSTR2`: a Markdown description, `"`…`"`.
    NStr2,
    /// `ICON`: the text of `::icon(…)`.
    Icon,
    /// `CLASS`: the text after `:::`.
    Class,
    /// `shapeData`: the text of `@{…}`.
    ShapeData,
    /// `shapeDataStr`: a quoted string inside `@{…}`.
    ShapeDataStr,
    /// `shapeDataEndBracket`: declared by the grammar, and entered by no rule.
    #[allow(dead_code)]
    ShapeDataEndBracket,
}

/// The tokens of the grammar. Brackets carry no value: the bracket only sets the node's type, which
/// the DB uses for its padding alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `kanban`.
    Kanban,
    /// A single line break.
    Nl,
    /// Blanks ending in a line break, or a comment: a statement and a stop.
    SpaceLine,
    /// Blanks, with the number of characters they hold: the level of the node that follows.
    SpaceList(u32),
    /// The id of a node, with its blanks and comments.
    NodeId(String),
    /// An opening bracket of a node.
    NodeDStart,
    /// The description of a node.
    NodeDescr(String),
    /// A closing bracket of a node.
    NodeDEnd,
    /// A piece of shape data: `@{` gives an empty piece, a quote its own text.
    ShapeData(String),
    /// The text of `::icon(…)`.
    Icon(String),
    /// The text of `:::…`.
    Class(String),
    /// The grammar's `EOF`, the end of the input.
    Eof,
    /// Jison's raw end symbol, returned once the input is done. Only the end of the token list.
    End,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// What a rule matched: how far it reads, and the token it gives, if any.
struct Hit {
    len: usize,
    token: Option<Token>,
}

fn emit(len: usize, token: Token) -> Option<Hit> {
    Some(Hit {
        len,
        token: Some(token),
    })
}

fn silent(len: usize) -> Option<Hit> {
    Some(Hit { len, token: None })
}

/// Reads the whole text. The list ends with the first [`Token::End`]. A character that no rule takes
/// is an error, and its byte offset is returned.
pub(crate) fn scan(text: &str) -> Result<Vec<Spanned>, usize> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        states: StateStack::new(State::Initial),
        done: false,
    };
    let mut tokens = Vec::new();
    loop {
        let spanned = scanner.next_token()?;
        let is_end = spanned.token == Token::End;
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

/// Whether `rest` starts with `word`, ignoring ASCII case.
fn starts_with_ci(rest: &str, word: &str) -> bool {
    rest.as_bytes()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word.as_bytes()))
}

/// Byte length of the keyword `word` (case-insensitive) when no word character follows it, as
/// Jison's implicit `\b` after a literal that ends in a word character.
fn keyword_len(rest: &str, word: &str) -> Option<usize> {
    let after = rest.get(word.len()..)?;
    (starts_with_ci(rest, word) && !after.chars().next().is_some_and(is_word)).then_some(word.len())
}

/// Byte length of the text before the first character that is not `.` matches: the run of
/// characters up to a line terminator, or all of `text`.
fn dot_len(text: &str) -> usize {
    text.find(is_line_term).unwrap_or(text.len())
}

/// `\n\s*` replaced by `<br/>`, as the shape data string rule does (`kanban.jison:37-38`).
fn line_breaks_to_br(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            out.push_str("<br/>");
            while chars.peek().is_some_and(|&next| is_js_space(next)) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    /// The next token: Jison's `next()` called until it returns a token, or the raw end.
    fn next_token(&mut self) -> Result<Spanned, usize> {
        loop {
            if self.done {
                return Ok(Spanned {
                    token: Token::End,
                    start: self.pos,
                });
            }
            if self.rest().is_empty() {
                self.done = true;
            }
            let start = self.pos;
            match self.rule() {
                Some(Hit { len, token }) => {
                    self.pos += len;
                    if let Some(token) = token {
                        return Ok(Spanned { token, start });
                    }
                }
                None if self.rest().is_empty() => {
                    return Ok(Spanned {
                        token: Token::End,
                        start,
                    });
                }
                None => return Err(start),
            }
        }
    }

    /// The rules of the current start condition, in the grammar's order.
    fn rule(&mut self) -> Option<Hit> {
        match self.states.top() {
            State::Initial => self.initial(),
            State::Node => self.node(),
            State::NStr => self.nstr(),
            State::NStr2 => self.nstr2(),
            State::Icon => self.icon(),
            State::Class => self.class(),
            State::ShapeData => self.shape_data(),
            State::ShapeDataStr => self.shape_data_str(),
            State::ShapeDataEndBracket => None,
        }
    }

    /// The rules without a state prefix, in the order of `kanban.jison:24-71`.
    fn initial(&mut self) -> Option<Hit> {
        let rest = self.rest();
        let blanks = space_len(rest);

        // `\@\{`: shape data begins, with an empty piece.
        if rest.starts_with("@{") {
            self.states.push(State::ShapeData);
            return emit(2, Token::ShapeData(String::new()));
        }
        // `\s*\%\%.*`: a comment, with the blanks before it. The blanks may run over line breaks.
        if rest[blanks..].starts_with("%%") {
            let len = blanks + 2 + dot_len(&rest[blanks + 2..]);
            return emit(len, Token::SpaceLine);
        }
        // `"kanban"`, with the word boundary of a keyword literal.
        if let Some(len) = keyword_len(rest, "kanban") {
            return emit(len, Token::Kanban);
        }
        // `":::"`: the class text follows.
        if rest.starts_with(":::") {
            self.states.push(State::Class);
            return silent(3);
        }
        // `"::icon("`, case-insensitive: the icon text follows.
        if starts_with_ci(rest, "::icon(") {
            self.states.push(State::Icon);
            return silent(7);
        }
        // `[\s]+[\n]`: blanks up to the last line break of the run, when that break is not the
        // first character.
        if let Some(last_break) = rest[..blanks].rfind('\n')
            && last_break >= 1
        {
            return emit(last_break + 1, Token::SpaceLine);
        }
        // `[\n]+`.
        if rest.starts_with('\n') {
            let len = rest.len() - rest.trim_start_matches('\n').len();
            return emit(len, Token::Nl);
        }
        // The opening brackets. Each one begins the node's bracket state.
        for bracket in ["-)", "(-", "))", ")", "((", "{{", "(", "["] {
            if rest.starts_with(bracket) {
                self.states.push(State::Node);
                return emit(bracket.len(), Token::NodeDStart);
            }
        }
        // `[\s]+`: blanks, which hold no line break here.
        if blanks > 0 {
            let level = u32::try_from(rest[..blanks].chars().count()).unwrap_or(u32::MAX);
            return emit(blanks, Token::SpaceList(level));
        }
        // `[^\(\[\n\)\{\}@]+`: the id, blanks and comments included.
        let len = rest
            .find(['(', '[', '\n', ')', '{', '}', '@'])
            .unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::NodeId(rest[..len].to_owned()));
        }
        // `<<EOF>>`.
        if rest.is_empty() {
            return emit(0, Token::Eof);
        }
        None
    }

    /// The `NODE` rules, in the order of `kanban.jison:72-87`.
    fn node(&mut self) -> Option<Hit> {
        let rest = self.rest();
        if rest.starts_with("\"`") {
            self.states.push(State::NStr2);
            return silent(2);
        }
        if rest.starts_with('"') {
            self.states.push(State::NStr);
            return silent(1);
        }
        // The closing brackets end the state. `))` is tried before `)`, `((` before `(`, `(-` before
        // `(`, and `-)` is a closing bracket here, not a description.
        for bracket in ["))", ")", "]", "}}", "(-", "-)", "((", "("] {
            if rest.starts_with(bracket) {
                self.states.pop();
                return emit(bracket.len(), Token::NodeDEnd);
            }
        }
        // `[^\)\]\(\}]+`.
        let len = rest.find([')', ']', '(', '}']).unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::NodeDescr(rest[..len].to_owned()));
        }
        // `.+(?!\(\()`: only a lone `}` reaches it, and takes the rest of the line.
        if rest.starts_with('}') {
            let len = dot_len(rest);
            return emit(len, Token::NodeDescr(rest[..len].to_owned()));
        }
        None
    }

    /// `NSTR2`: a Markdown description, which ends with a backquote and a quote.
    fn nstr2(&mut self) -> Option<Hit> {
        let rest = self.rest();
        let len = rest.find(['`', '"']).unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::NodeDescr(rest[..len].to_owned()));
        }
        if rest.starts_with("`\"") {
            self.states.pop();
            return silent(2);
        }
        None
    }

    /// `NSTR`: a quoted description, which may run over line breaks.
    fn nstr(&mut self) -> Option<Hit> {
        let rest = self.rest();
        let len = rest.find('"').unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::NodeDescr(rest[..len].to_owned()));
        }
        if rest.starts_with('"') {
            self.states.pop();
            return silent(1);
        }
        None
    }

    /// `ICON`: the text up to `)`, which may run over line breaks.
    fn icon(&mut self) -> Option<Hit> {
        let rest = self.rest();
        let len = rest.find(')').unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::Icon(rest[..len].to_owned()));
        }
        if rest.starts_with(')') {
            self.states.pop();
            return silent(1);
        }
        None
    }

    /// `CLASS`: the rest of the line, or nothing when the line is empty.
    fn class(&mut self) -> Option<Hit> {
        let rest = self.rest();
        let len = dot_len(rest);
        let text = rest[..len].to_owned();
        if len > 0 {
            self.states.pop();
            return emit(len, Token::Class(text));
        }
        if rest.starts_with('\n') {
            self.states.pop();
            return silent(1);
        }
        None
    }

    /// `shapeData`: a quote starts a string, and the text runs up to `}`, `^` or a quote. The `^`
    /// is no character of shape data, so it is a lexical error (`kanban.jison:40`).
    fn shape_data(&mut self) -> Option<Hit> {
        let rest = self.rest();
        if rest.starts_with('"') {
            self.states.push(State::ShapeDataStr);
            return emit(1, Token::ShapeData("\"".to_owned()));
        }
        let len = rest.find(['}', '^', '"']).unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::ShapeData(rest[..len].to_owned()));
        }
        if rest.starts_with('}') {
            self.states.pop();
            return silent(1);
        }
        None
    }

    /// `shapeDataStr`: the text up to a quote, with its line breaks as `<br/>`.
    fn shape_data_str(&mut self) -> Option<Hit> {
        let rest = self.rest();
        if rest.starts_with('"') {
            self.states.pop();
            return emit(1, Token::ShapeData("\"".to_owned()));
        }
        let len = rest.find('"').unwrap_or(rest.len());
        if len > 0 {
            return emit(len, Token::ShapeData(line_breaks_to_br(&rest[..len])));
        }
        None
    }
}
