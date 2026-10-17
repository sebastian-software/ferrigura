//! The block scanner: the rules of `block.jison` (its `%lex` section, lines 11-162) in the order the
//! grammar writes them. Jison takes the first rule that matches at the current position, not the
//! longest one, and the start conditions select the rules of each state. The rule table and the
//! quirks it gives are in `docs-port/block/PLAN.md`, sections 2 and 3.
//!
//! The scanner reproduces Jison's `next()`: a call that starts with the input empty sets Jison's
//! `done` flag and reads the rules of the current state at the end of the input; the next call
//! returns Jison's raw end symbol, which the grammar does not accept.

use crate::scan::{StateStack, literal_len, space_len, word_len};
use crate::text::{is_js_space, is_word};

/// The start conditions of the lexer. The base state is `Initial`; the others are entered and
/// left by the rules, as the `%x` states of the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    /// The rules without a start condition.
    Initial,
    /// `accTitle:`, up to the end of the line.
    AccTitle,
    /// `accDescr:`, up to the end of the line.
    AccDescr,
    /// `accDescr { … }`, up to its `}`.
    AccDescrMulti,
    /// A double-quoted string.
    Quoted,
    /// A Markdown string, between a quote and a backtick.
    MdString,
    /// Inside a shape: the opening of a node's label up to its closing bracket.
    Node,
    /// Inside a block arrow `<[ … ]>`, before its directions.
    BlockArrow,
    /// The directions of a block arrow, up to `)`.
    ArrowDir,
    /// The label of a link: `-- "text" -->`.
    LinkLabel,
    /// The name of a `classDef`.
    ClassDef,
    /// The style options of a `classDef`.
    ClassDefId,
    /// The ids of a `class` statement.
    Class,
    /// The class names of a `class` statement.
    ClassStyle,
    /// The ids of a `style` statement.
    StyleStmt,
    /// The style definition of a `style` statement.
    StyleDefinition,
}

/// The tokens of the grammar. A keyword that no production uses is `Unused`: the grammar rejects
/// it as an unexpected token, as the reference does.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `block:`, the header of a composite with an id.
    IdBlock,
    /// `block`, the header of the diagram or of an anonymous composite.
    BlockKey,
    /// `columns N` or `columns auto`, with `-1` for auto.
    Columns(f64),
    /// `space` or `space:N`, with the number of cells.
    SpaceBlock(f64),
    /// A keyword the grammar has no production for (`default`, `linkStyle`, `interpolate`, the
    /// accessibility keywords and their values, the Markdown strings, a `classDef DEFAULT`).
    Unused,
    /// `classDef`.
    ClassDef,
    /// The name of a `classDef`, with the blanks after it.
    ClassDefId(String),
    /// The style options of a `classDef`, up to the end of the line.
    ClassDefStyleOpts(String),
    /// `class`.
    Class,
    /// The ids of a `class` statement.
    ClassEntityIds(String),
    /// The class names of a `class` statement, up to the end of the line; possibly empty.
    StyleClass(String),
    /// `style`.
    Style,
    /// The ids of a `style` statement.
    StyleEntityIds(String),
    /// The definition of a `style` statement, up to the end of the line; possibly empty.
    StyleDefinitionData(String),
    /// `end`, which closes a composite.
    End,
    /// The opening bracket of a node's label, such as `[`, `((` or `[(`.
    NodeDStart(String),
    /// The closing bracket of a node's label, such as `]` or `))`.
    NodeDEnd(String),
    /// `<[`, the start of a block arrow.
    BlockArrowStart,
    /// An id: any run of characters other than brackets, dashes, blanks and the other signs.
    NodeId(String),
    /// A double-quoted string, without its quotes; possibly empty only at the end of the text.
    Str(String),
    /// A direction of a block arrow, with the blanks around it as written.
    Dir(String),
    /// `]>` and the `(` before the directions are closed by `)`.
    BlockArrowEnd,
    /// A link, as its text (`-->`, `===`, `-.-`, `~~~`, with the marks and the blanks around it).
    Link(String),
    /// The start of a labelled link (`-- `, `== `, `-. `).
    StartLink,
    /// The opening quote of a link label.
    LinkLabel,
    /// `:N` after a standalone node, the span of the node.
    Size(f64),
    /// The end of the input, as the grammar's `EOF` terminal.
    Eof,
    /// Jison's raw end symbol, returned once the input is done. The grammar does not accept it.
    RawEof,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// What one rule does when it matches: the length it consumes, the token it returns (if any), the
/// states it leaves, and the state it enters.
struct Hit {
    len: usize,
    token: Option<Token>,
    pops: usize,
    push: Option<State>,
}

impl Hit {
    /// A rule that returns a token and changes no state.
    fn token(len: usize, token: Token) -> Self {
        Self {
            len,
            token: Some(token),
            pops: 0,
            push: None,
        }
    }

    /// A rule that returns no token: the text is skipped or a state is entered.
    fn silent(len: usize, push: Option<State>) -> Self {
        Self {
            len,
            token: None,
            pops: 0,
            push,
        }
    }

    /// A rule that leaves `pops` states, enters `push`, and returns `token`.
    fn leave(len: usize, pops: usize, push: Option<State>, token: Option<Token>) -> Self {
        Self {
            len,
            token,
            pops,
            push,
        }
    }
}

/// The scanner of one text. It is the lexer of the grammar, with Jison's `next()` behaviour.
pub(crate) struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    states: StateStack<State>,
    done: bool,
}

impl<'a> Scanner<'a> {
    /// A scanner at the start of `text`, in the initial state.
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            text,
            pos: 0,
            states: StateStack::new(State::Initial),
            done: false,
        }
    }

    /// The next token and the byte offset where it starts. An error holds the byte offset of the
    /// text that no rule takes.
    pub(crate) fn next_token(&mut self) -> Result<Spanned, usize> {
        loop {
            let start = self.pos;
            if let Some(token) = self.next_call()? {
                return Ok(Spanned { token, start });
            }
        }
    }

    /// One call of Jison's `next()`: the first rule of the current state that matches. A rule that
    /// returns no token gives `None`, and the caller calls again, as `lex()` does.
    fn next_call(&mut self) -> Result<Option<Token>, usize> {
        if self.done {
            return Ok(Some(Token::RawEof));
        }
        if self.pos >= self.text.len() {
            self.done = true;
        }
        let rest = &self.text[self.pos..];
        let hit = match self.states.top() {
            State::Initial => initial(rest),
            State::AccTitle => Some(Hit::leave(line_len(rest), 1, None, Some(Token::Unused))),
            State::AccDescr => Some(Hit::leave(line_len(rest), 1, None, Some(Token::Unused))),
            State::AccDescrMulti => accessibility_block(rest),
            State::Quoted => quoted(rest),
            State::MdString => markdown(rest),
            State::Node => node(rest),
            State::BlockArrow => block_arrow(rest),
            State::ArrowDir => arrow_dir(rest),
            State::LinkLabel => link_label(rest),
            State::ClassDef => class_def(rest),
            State::ClassDefId => Some(Hit::leave(
                line_len(rest),
                1,
                None,
                Some(Token::ClassDefStyleOpts(rest[..line_len(rest)].to_owned())),
            )),
            State::Class => ids_rule(rest, State::ClassStyle, Token::ClassEntityIds),
            State::ClassStyle => Some(Hit::leave(
                line_len(rest),
                1,
                None,
                Some(Token::StyleClass(rest[..line_len(rest)].to_owned())),
            )),
            State::StyleStmt => ids_rule(rest, State::StyleDefinition, Token::StyleEntityIds),
            State::StyleDefinition => Some(Hit::leave(
                line_len(rest),
                1,
                None,
                Some(Token::StyleDefinitionData(
                    rest[..line_len(rest)].to_owned(),
                )),
            )),
        };
        match hit {
            Some(hit) => {
                self.pos += hit.len;
                for _ in 0..hit.pops {
                    self.states.pop();
                }
                if let Some(state) = hit.push {
                    self.states.push(state);
                }
                Ok(hit.token)
            }
            None if rest.is_empty() => Ok(Some(Token::RawEof)),
            None => Err(self.pos),
        }
    }
}

/// The rules of the initial state, in the order of `block.jison` (lines 39-160).
fn initial(rest: &str) -> Option<Hit> {
    if let Some(len) = literal_len(rest, "block:") {
        return Some(Hit::token(len, Token::IdBlock));
    }
    if let Some(len) = word_len(rest, "block") {
        return Some(Hit::token(len, Token::BlockKey));
    }
    let blank = space_len(rest);
    if blank > 0 {
        return Some(Hit::silent(blank, None));
    }
    if let Some(len) = columns_auto_len(rest) {
        return Some(Hit::token(len, Token::Columns(-1.0)));
    }
    if let Some((len, value)) = columns_number(rest) {
        return Some(Hit::token(len, Token::Columns(value)));
    }
    if rest.starts_with("\"`") {
        return Some(Hit::silent(2, Some(State::MdString)));
    }
    if rest.starts_with('"') {
        return Some(Hit::silent(1, Some(State::Quoted)));
    }
    if let Some((len, value)) = space_number(rest) {
        return Some(Hit::token(len, Token::SpaceBlock(value)));
    }
    if let Some(len) = word_len(rest, "space") {
        return Some(Hit::token(len, Token::SpaceBlock(1.0)));
    }
    for keyword in ["default", "linkStyle", "interpolate"] {
        if let Some(len) = word_len(rest, keyword) {
            return Some(Hit::token(len, Token::Unused));
        }
    }
    if let Some(len) = keyword_then_blank(rest, "classDef") {
        return Some(Hit::leave(
            len,
            0,
            Some(State::ClassDef),
            Some(Token::ClassDef),
        ));
    }
    if let Some(len) = keyword_then_blank(rest, "class") {
        return Some(Hit::leave(len, 0, Some(State::Class), Some(Token::Class)));
    }
    if let Some(len) = keyword_then_blank(rest, "style") {
        return Some(Hit::leave(
            len,
            0,
            Some(State::StyleStmt),
            Some(Token::Style),
        ));
    }
    if let Some(len) = keyword_colon(rest, "accTitle") {
        return Some(Hit::leave(
            len,
            0,
            Some(State::AccTitle),
            Some(Token::Unused),
        ));
    }
    if let Some(len) = keyword_colon(rest, "accDescr") {
        return Some(Hit::leave(
            len,
            0,
            Some(State::AccDescr),
            Some(Token::Unused),
        ));
    }
    if let Some(len) = keyword_brace(rest, "accDescr") {
        return Some(Hit::silent(len, Some(State::AccDescrMulti)));
    }
    if let Some(len) = word_len(rest, "end") {
        let len = len + space_len(&rest[len..]);
        return Some(Hit::token(len, Token::End));
    }
    if let Some(hit) = node_start(rest) {
        return Some(hit);
    }
    if rest.starts_with("<[") {
        return Some(Hit::leave(
            2,
            0,
            Some(State::BlockArrow),
            Some(Token::BlockArrowStart),
        ));
    }
    let id_len = node_id_len(rest);
    if id_len > 0 {
        return Some(Hit::token(id_len, Token::NodeId(rest[..id_len].to_owned())));
    }
    if rest.is_empty() {
        return Some(Hit::token(0, Token::Eof));
    }
    if let Some(hit) = link_in_initial(rest) {
        return Some(hit);
    }
    if let Some(len) = size(rest) {
        return Some(Hit::token(len, Token::Size(digits_value(&rest[1..len]))));
    }
    None
}

/// The opening brackets of a node's label, in the order of `block.jison` (lines 103-121). The
/// first one that matches is taken, so `))` comes before `)`, and `(((` before `((`.
const NODE_STARTS: [&str; 17] = [
    "-)", "(-", "))", ")", "(((", "((", "{{", "{", ">", "([", "(", "[[", "[|", "[(", ")))", "[\\",
    "[/",
];

/// The `NODE_DSTART` rules: a node's label opens with one of [`NODE_STARTS`], or with `[` alone.
fn node_start(rest: &str) -> Option<Hit> {
    NODE_STARTS
        .iter()
        .chain(std::iter::once(&"["))
        .find(|start| rest.starts_with(**start))
        .map(|start| {
            Hit::leave(
                start.len(),
                0,
                Some(State::Node),
                Some(Token::NodeDStart((*start).to_owned())),
            )
        })
}

/// The closing brackets of a node's label, in the order of `block.jison` (lines 84-100).
const NODE_ENDS: [&str; 16] = [
    "(((", ")))", "))", "}}", "}", "(-", "-)", "((", "]]", "(", "])", "\\]", "/]", ")]", ")", "]>",
];

/// The rules of the node state (lines 84-100 and 130-133).
fn node(rest: &str) -> Option<Hit> {
    for end in NODE_ENDS.iter().chain(std::iter::once(&"]")) {
        if rest.starts_with(end) {
            return Some(Hit::leave(
                end.len(),
                1,
                None,
                Some(Token::NodeDEnd((*end).to_owned())),
            ));
        }
    }
    if rest.starts_with("\"`") {
        return Some(Hit::silent(2, Some(State::MdString)));
    }
    if rest.starts_with('"') {
        return Some(Hit::silent(1, Some(State::Quoted)));
    }
    None
}

/// The rules of a quoted string (lines 53-54): the closing quote, then the text up to the next one.
fn quoted(rest: &str) -> Option<Hit> {
    if rest.starts_with('"') {
        return Some(Hit::silent(1, None).with_pop());
    }
    let len = rest.find('"').unwrap_or(rest.len());
    Some(Hit::token(len, Token::Str(rest[..len].to_owned())))
}

/// The rules of a Markdown string (lines 50-51): the text is an unused token, and a backtick and a
/// quote close it.
fn markdown(rest: &str) -> Option<Hit> {
    if rest.starts_with("`\"") {
        return Some(Hit::silent(2, None).with_pop());
    }
    let len = rest.find(['`', '"']).unwrap_or(rest.len());
    if len == 0 {
        return None;
    }
    Some(Hit::token(len, Token::Unused))
}

/// The rules of the block arrow state (lines 129, 134 and 138).
fn block_arrow(rest: &str) -> Option<Hit> {
    if rest.starts_with("\"`") {
        return Some(Hit::silent(2, Some(State::MdString)));
    }
    if rest.starts_with('"') {
        return Some(Hit::silent(1, Some(State::Quoted)));
    }
    if let Some(after) = rest.strip_prefix("]>") {
        let blank = space_len(after);
        if after[blank..].starts_with('(') {
            return Some(Hit::silent(2 + blank + 1, Some(State::ArrowDir)));
        }
    }
    None
}

/// The directions of a block arrow (lines 139-145): `,?\s*word\s*`, then `)` with its blanks.
fn arrow_dir(rest: &str) -> Option<Hit> {
    for word in ["right", "left", "x", "y", "up", "down"] {
        if let Some(len) = direction_len(rest, word) {
            let text = &rest[..len];
            let value = match text.strip_prefix(',') {
                Some(after) => after[space_len(after)..].to_owned(),
                None => text.to_owned(),
            };
            return Some(Hit::token(len, Token::Dir(value)));
        }
    }
    if let Some(after) = rest.strip_prefix(')') {
        let len = 1 + space_len(after);
        return Some(Hit::leave(len, 2, None, Some(Token::BlockArrowEnd)));
    }
    None
}

/// `,?\s*word\s*` at the start of `rest`, as its byte length.
fn direction_len(rest: &str, word: &str) -> Option<usize> {
    let comma = usize::from(rest.starts_with(','));
    let after_comma = &rest[comma..];
    let blank = space_len(after_comma);
    let after_blank = &after_comma[blank..];
    let word_part = after_blank.strip_prefix(word)?;
    Some(comma + blank + word.len() + space_len(word_part))
}

/// The rules of the label of a link (lines 155-159): the quotes, then the closing link.
fn link_label(rest: &str) -> Option<Hit> {
    if rest.starts_with("\"`") {
        return Some(Hit::silent(2, Some(State::MdString)));
    }
    if rest.starts_with('"') {
        return Some(Hit::leave(
            1,
            0,
            Some(State::Quoted),
            Some(Token::LinkLabel),
        ));
    }
    let closing = [dash_core, eq_core, dot_core]
        .into_iter()
        .find_map(|core| link_len(rest, core))?;
    Some(Hit::leave(
        closing,
        1,
        None,
        Some(Token::Link(rest[..closing].to_owned())),
    ))
}

/// The rules of a `classDef` name (lines 62-63): `DEFAULT` with blanks, or a word with blanks.
fn class_def(rest: &str) -> Option<Hit> {
    if let Some(after) = rest.strip_prefix("DEFAULT") {
        let blank = space_len(after);
        if blank > 0 {
            return Some(Hit::leave(
                7 + blank,
                1,
                Some(State::ClassDefId),
                Some(Token::Unused),
            ));
        }
    }
    let word = word_run(rest);
    let blank = space_len(&rest[word..]);
    if word > 0 && blank > 0 {
        return Some(Hit::leave(
            word + blank,
            1,
            Some(State::ClassDefId),
            Some(Token::ClassDefId(rest[..word + blank].to_owned())),
        ));
    }
    None
}

/// The ids of a `class` or `style` statement (lines 67 and 71): `(\w+)+((","\s*\w+)*)`. The
/// result enters `next` and returns the token with the ids.
fn ids_rule(rest: &str, next: State, token: fn(String) -> Token) -> Option<Hit> {
    let len = id_list_len(rest)?;
    Some(Hit::leave(
        len,
        1,
        Some(next),
        Some(token(rest[..len].to_owned())),
    ))
}

/// Byte length of `\w+((","\s*\w+)*)` at the start of `rest`.
fn id_list_len(rest: &str) -> Option<usize> {
    let mut len = word_run(rest);
    if len == 0 {
        return None;
    }
    loop {
        let after = &rest[len..];
        let Some(tail) = after.strip_prefix(',') else {
            break;
        };
        let blank = space_len(tail);
        let word = word_run(&tail[blank..]);
        if word == 0 {
            break;
        }
        len += 1 + blank + word;
    }
    Some(len)
}

/// The rules of the accessibility block (lines 79-80): a `}` closes it, and the text up to a `}`
/// is an unused token, possibly empty.
fn accessibility_block(rest: &str) -> Option<Hit> {
    if rest.starts_with('}') {
        return Some(Hit::leave(1, 1, None, None));
    }
    let len = rest.find('}').unwrap_or(rest.len());
    Some(Hit::token(len, Token::Unused))
}

impl Hit {
    /// The same hit with one state left, for the rules that pop after a string.
    fn with_pop(mut self) -> Self {
        self.pops = 1;
        self
    }
}

/// `columns\s+auto` with a word boundary after `auto`.
fn columns_auto_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix("columns")?;
    let blank = space_len(after);
    if blank == 0 {
        return None;
    }
    let word = after[blank..].strip_prefix("auto")?;
    if word.chars().next().is_some_and(is_word) {
        return None;
    }
    Some("columns".len() + blank + "auto".len())
}

/// `columns\s+\d+`, with the number.
fn columns_number(rest: &str) -> Option<(usize, f64)> {
    let after = rest.strip_prefix("columns")?;
    let blank = space_len(after);
    let digits = digit_run(&after[blank..]);
    if blank == 0 || digits == 0 {
        return None;
    }
    let len = "columns".len() + blank + digits;
    Some((len, digits_value(&rest[len - digits..len])))
}

/// `space:\d+`, with the number of cells.
fn space_number(rest: &str) -> Option<(usize, f64)> {
    let after = rest.strip_prefix("space:")?;
    let digits = digit_run(after);
    if digits == 0 {
        return None;
    }
    Some(("space:".len() + digits, digits_value(&after[..digits])))
}

/// `word\s+`: a keyword followed by at least one blank, as `classDef\s+` is written.
fn keyword_then_blank(rest: &str, word: &str) -> Option<usize> {
    let after = rest.strip_prefix(word)?;
    let blank = space_len(after);
    (blank > 0).then_some(word.len() + blank)
}

/// `word\s*:\s*`: a keyword, a colon between optional blanks, and blanks after it.
fn keyword_colon(rest: &str, word: &str) -> Option<usize> {
    keyword_with_delimiter(rest, word, ':')
}

/// `word\s*{\s*`: a keyword, an opening brace between optional blanks, and blanks after it.
fn keyword_brace(rest: &str, word: &str) -> Option<usize> {
    keyword_with_delimiter(rest, word, '{')
}

fn keyword_with_delimiter(rest: &str, word: &str, delimiter: char) -> Option<usize> {
    let after = rest.strip_prefix(word)?;
    let before = space_len(after);
    let tail = after[before..].strip_prefix(delimiter)?;
    Some(word.len() + before + delimiter.len_utf8() + space_len(tail))
}

/// `:\d+` at the start of `rest`: the size of a standalone node.
fn size(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix(':')?;
    let digits = digit_run(after);
    (digits > 0).then_some(1 + digits)
}

/// The rules of the initial state for links (lines 148-154): the links, and the start of a labelled
/// link, which enters the link label state.
fn link_in_initial(rest: &str) -> Option<Hit> {
    if let Some(len) = link_len(rest, dash_core) {
        return Some(Hit::token(len, Token::Link(rest[..len].to_owned())));
    }
    if let Some(len) = link_len(rest, eq_core) {
        return Some(Hit::token(len, Token::Link(rest[..len].to_owned())));
    }
    if let Some(len) = link_len(rest, dot_core) {
        return Some(Hit::token(len, Token::Link(rest[..len].to_owned())));
    }
    if let Some(len) = link_len(rest, tilde_core) {
        return Some(Hit::token(len, Token::Link(rest[..len].to_owned())));
    }
    for core in [start_dash_core, start_eq_core, start_dot_core] {
        if let Some(total) = link_len(rest, core) {
            return Some(Hit::leave(
                total,
                0,
                Some(State::LinkLabel),
                Some(Token::StartLink),
            ));
        }
    }
    None
}

/// `\s*[xo<]?` then the core of a link, then `\s*`: the byte length of the match.
fn link_len(rest: &str, core: fn(&str) -> Option<usize>) -> Option<usize> {
    let lead = space_len(rest);
    let after_lead = &rest[lead..];
    let mark = match after_lead.chars().next() {
        Some(c @ ('x' | 'o' | '<')) => c.len_utf8(),
        _ => 0,
    };
    let body = &after_lead[mark..];
    let body_len = core(body)?;
    let trail = space_len(&body[body_len..]);
    Some(lead + mark + body_len + trail)
}

/// `\-\-+[-xo>]`: two or more dashes, then a dash, `x`, `o` or `>`. The last dash of a run of three
/// or more is the closing character when no mark follows it, as the backtracking of the rule gives.
fn dash_core(body: &str) -> Option<usize> {
    run_then_mark(body, '-', "-xo>", 2)
}

/// `==+[=xo>]`, as [`dash_core`] for equals signs.
fn eq_core(body: &str) -> Option<usize> {
    run_then_mark(body, '=', "=xo>", 2)
}

/// `\-?\.+\-[xo>]?`: an optional dash, one or more dots, a dash, and an optional mark.
fn dot_core(body: &str) -> Option<usize> {
    let lead = usize::from(body.starts_with('-'));
    let dots = body[lead..].bytes().take_while(|b| *b == b'.').count();
    if dots == 0 {
        return None;
    }
    let dash_at = lead + dots;
    if !body[dash_at..].starts_with('-') {
        return None;
    }
    let after = dash_at + 1;
    let mark = match body[after..].chars().next() {
        Some(c @ ('x' | 'o' | '>')) => c.len_utf8(),
        _ => 0,
    };
    Some(after + mark)
}

/// `\~\~[\~]+`: three or more tildes.
fn tilde_core(body: &str) -> Option<usize> {
    let tildes = body.bytes().take_while(|b| *b == b'~').count();
    (tildes >= 3).then_some(tildes)
}

/// `\-\-`, the start of a labelled link.
fn start_dash_core(body: &str) -> Option<usize> {
    body.starts_with("--").then_some(2)
}

/// `==`, the start of a labelled link.
fn start_eq_core(body: &str) -> Option<usize> {
    body.starts_with("==").then_some(2)
}

/// `-.`, the start of a labelled link.
fn start_dot_core(body: &str) -> Option<usize> {
    body.starts_with("-.").then_some(2)
}

/// `c{2,}[marks]` for a run of `c`: the run is taken greedily, and the closing character is the
/// next one when it is a mark, or the last character of the run when the run has `min + 1` or more
/// characters. A run of exactly `min` characters needs a mark after it.
fn run_then_mark(body: &str, run: char, marks: &str, min: usize) -> Option<usize> {
    let count = body.chars().take_while(|c| *c == run).count();
    if count < min {
        return None;
    }
    let after = &body[count..];
    match after.chars().next() {
        Some(c) if marks.contains(c) => Some(count + c.len_utf8()),
        _ if count > min => Some(count),
        _ => None,
    }
}

/// The node id rule (line 125): `[^\(\[\n\-\)\{\}\s\<\>:=]+`, in bytes.
fn node_id_len(rest: &str) -> usize {
    rest.char_indices()
        .find(|&(_, c)| {
            matches!(
                c,
                '(' | '[' | '\n' | '-' | ')' | '{' | '}' | '<' | '>' | ':' | '='
            ) || is_js_space(c)
        })
        .map_or(rest.len(), |(index, _)| index)
}

/// The byte length of the run of word characters at the start of `text`.
fn word_run(text: &str) -> usize {
    text.char_indices()
        .find(|&(_, c)| !is_word(c))
        .map_or(text.len(), |(index, _)| index)
}

/// The byte length of the run of ASCII digits at the start of `text`.
fn digit_run(text: &str) -> usize {
    text.bytes().take_while(u8::is_ascii_digit).count()
}

/// The value of a run of ASCII digits, as `parseInt` reads it.
fn digits_value(digits: &str) -> f64 {
    digits.parse::<f64>().unwrap_or(f64::NAN)
}

/// Byte length of the text before the next line feed, or of the text when there is none.
fn line_len(rest: &str) -> usize {
    rest.find('\n').unwrap_or(rest.len())
}
