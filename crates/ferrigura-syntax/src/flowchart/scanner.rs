//! The flowchart scanner: the rules of `flow.jison`, with its start conditions, in the grammar's
//! order. The first rule that matches wins, as in Jison; keyword literals match only at a word
//! boundary. See `docs-port/flowchart/PLAN.md`, section 3.
//!
//! The scanner runs over the whole prepared text before parsing. The grammar's actions never
//! change the lexer's state, so the token sequence does not depend on the parser, except for the
//! first `graph` keyword, which the scanner decides itself.
//!
//! The agentflow lexer (`agentflow.jison`) differs from this one in a few rules. Those differences
//! are the [`Dialect`] parameters: the header words, the keyword table, the link operators, the
//! `%%` comments and the blank rule. The agentflow values are in `agentflow.rs`, next to its
//! grammar (`docs-port/agentflow/PLAN.md`, section 2).

use crate::scan::{StateStack, delimited_form_len, line_len, literal_len, space_len, word_len};
use crate::text::{is_js_space, is_line_term};

use super::unicode::is_unicode_text;

/// The start conditions of the flowchart lexer (the states of `flow.jison`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    Quoted,
    MdString,
    AccTitle,
    AccDescr,
    AccDescrMulti,
    Dir,
    Text,
    EllipseText,
    TrapText,
    EdgeText,
    ThickEdgeText,
    DottedEdgeText,
    Click,
    CallbackName,
    CallbackArgs,
    ShapeData,
    ShapeDataStr,
}

/// The token kinds of the flowchart grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Eof,
    Newline,
    Space,
    Semi,
    Graph,
    NoDir,
    Dir,
    Subgraph,
    End,
    LinkTarget,
    Style,
    LinkStyle,
    ClassDef,
    Class,
    Default,
    Interpolate,
    Href,
    Click,
    CallbackName,
    CallbackArgs,
    DirectionTb,
    DirectionBt,
    DirectionRl,
    DirectionLr,
    DirectionTd,
    AccTitle,
    AccTitleValue,
    AccDescr,
    AccDescrValue,
    AccDescrMultiValue,
    ShapeData,
    Link,
    StartLink,
    LinkId,
    EdgeText,
    Str,
    MdStr,
    Text,
    Num,
    NodeString,
    Minus,
    Brkt,
    Down,
    Up,
    Mult,
    Colon,
    StyleSeparator,
    Amp,
    Comma,
    UnicodeText,
    Pipe,
    Ps,
    Pe,
    Sqs,
    Sqe,
    DoubleCircleStart,
    DoubleCircleEnd,
    StadiumStart,
    StadiumEnd,
    SubroutineStart,
    SubroutineEnd,
    CylinderStart,
    CylinderEnd,
    DiamondStart,
    DiamondStop,
    TagStart,
    TagEnd,
    TrapStart,
    TrapEnd,
    InvTrapStart,
    InvTrapEnd,
    EllipseStart,
    EllipseEnd,
    VertexWithPropsStart,
    Flow,
    Connector,
    Global,
    Comment,
}

/// Which link operators the lexer knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Links {
    /// `flow.jison`: solid, thick, dotted and invisible links, each with its start and its end.
    Flowchart,
    /// `agentflow.jison`: solid links that end in `x` or `>`, and dotted links `-.-` with no text.
    Agentflow,
}

/// What a keyword takes after itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Trailing {
    /// Nothing: the keyword is the token.
    Nothing,
    /// `[ \t]*`.
    Blanks,
    /// `\s*`: line breaks included.
    Spaces,
}

/// One keyword literal of the INITIAL state, with its token and what it takes after itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Keyword {
    pub word: &'static str,
    pub kind: Kind,
    pub trailing: Trailing,
}

/// The parameters of a lexer that follows Jison's rules. Every rule the two diagram lexers share
/// is in the scanner once; these are the values in which they differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dialect {
    /// The words that open the header (`graph` and the others), each followed by a direction.
    pub graph_words: &'static [&'static str],
    /// The keyword literals of the INITIAL state, in the order of the grammar's rules. `href` is
    /// not here: it takes one blank after it, and both lexers have it.
    pub keywords: &'static [Keyword],
    /// The link operators.
    pub links: Links,
    /// Whether `%%` starts a comment, which the lexer returns as a token in INITIAL and skips in
    /// the text states.
    pub comments: bool,
    /// Whether a run of blanks is one SPACE token (`[^\S\n\r]+`), or each blank is one (`\s`).
    pub space_run: bool,
}

/// The flowchart's values: `flow.jison`.
pub(crate) const FLOWCHART: Dialect = Dialect {
    graph_words: &["swimlane-beta", "graph", "flowchart"],
    keywords: &[
        Keyword {
            word: "style",
            kind: Kind::Style,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "default",
            kind: Kind::Default,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "linkStyle",
            kind: Kind::LinkStyle,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "interpolate",
            kind: Kind::Interpolate,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "classDef",
            kind: Kind::ClassDef,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "class",
            kind: Kind::Class,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "subgraph",
            kind: Kind::Subgraph,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "_self",
            kind: Kind::LinkTarget,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "_blank",
            kind: Kind::LinkTarget,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "_parent",
            kind: Kind::LinkTarget,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "_top",
            kind: Kind::LinkTarget,
            trailing: Trailing::Nothing,
        },
        Keyword {
            word: "end",
            kind: Kind::End,
            trailing: Trailing::Spaces,
        },
    ],
    links: Links::Flowchart,
    comments: false,
    space_run: true,
};

/// One token: its kind, its value as the lexer's `yytext` would give it, and its byte offset.
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
    value: Option<String>,
}

fn hit(len: usize, kind: Kind) -> Option<Hit> {
    Some(Hit {
        len,
        kind: Some(kind),
        value: None,
    })
}

fn silent(len: usize) -> Option<Hit> {
    Some(Hit {
        len,
        kind: None,
        value: None,
    })
}

/// Tokenises the prepared text with the rules of `dialect`. The last token is always [`Kind::Eof`].
pub(crate) fn tokenize(text: &str, dialect: &'static Dialect) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer {
        pos: 0,
        states: StateStack::new(State::Initial),
        first_graph: true,
        index: Index::new(text),
        dialect,
    };
    let mut tokens = Vec::new();
    while lexer.pos < text.len() {
        let start = lexer.pos;
        let state = lexer.states.top();
        let Some(hit) = lexer.rule(&text[start..], state) else {
            return Err(LexError { offset: start });
        };
        // A rule that matches the empty text gives an empty token only for the values of the
        // accessibility lines, which are `[^\n]*` and `[^}]*` in Jison and so can be empty.
        if hit.len == 0 && !hit.kind.is_some_and(is_value_kind) {
            return Err(LexError { offset: start });
        }
        let value = hit
            .value
            .unwrap_or_else(|| text[start..start + hit.len].to_owned());
        lexer.pos += hit.len;
        if let Some(kind) = hit.kind {
            tokens.push(Token { kind, value, start });
        }
    }
    // Jison reads the end of the input with every rule of the state it is in. The rules that
    // match the empty text give one more token there: the value of an open accessibility line,
    // an open callback, or an open click. The other states give nothing.
    let end = text.len();
    let state = lexer.states.top();
    if matches!(
        state,
        State::AccTitle
            | State::AccDescr
            | State::AccDescrMulti
            | State::CallbackName
            | State::CallbackArgs
            | State::Click
    ) && let Some(hit) = lexer.rule("", state)
        && let Some(kind) = hit.kind
    {
        tokens.push(Token {
            kind,
            value: String::new(),
            start: end,
        });
    }
    tokens.push(Token {
        kind: Kind::Eof,
        value: String::new(),
        start: text.len(),
    });
    Ok(tokens)
}

/// The kinds of the values that a rule may give empty: the accessibility lines' values.
fn is_value_kind(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::AccTitleValue | Kind::AccDescrValue | Kind::AccDescrMultiValue
    )
}

/// The words of the `.*direction\s+WORD` rules, in the order the rules are written.
const DIRECTION_WORDS: [&str; 5] = ["TB", "BT", "RL", "LR", "TD"];

/// Positions the look-ahead rules need, found once per text, so that each token costs a binary
/// search instead of a scan of the rest of the line.
struct Index {
    /// The length of the text.
    len: usize,
    /// Blanks and quotes: a run of an id stops at one of these.
    breaks: Vec<usize>,
    /// Line terminators: the `.` of the direction rules cannot cross one.
    line_breaks: Vec<usize>,
    /// Every `@`.
    ats: Vec<usize>,
    /// For each direction word, the `direction` occurrences that a `\s+WORD` follows, as the start
    /// of `direction` and the end of the word.
    directions: Vec<Vec<(usize, usize)>>,
}

impl Index {
    fn new(text: &str) -> Self {
        let mut breaks = Vec::new();
        let mut line_breaks = Vec::new();
        let mut ats = Vec::new();
        for (index, c) in text.char_indices() {
            if is_js_space(c) || c == '"' {
                breaks.push(index);
            }
            if is_line_term(c) {
                line_breaks.push(index);
            }
            if c == '@' {
                ats.push(index);
            }
        }
        let mut directions = vec![Vec::new(); DIRECTION_WORDS.len()];
        for (start, _) in text.match_indices("direction") {
            let after = start + "direction".len();
            let spaces = space_len(&text[after..]);
            if spaces == 0 {
                continue;
            }
            let word_start = after + spaces;
            for (word, occurrences) in DIRECTION_WORDS.iter().zip(directions.iter_mut()) {
                if text[word_start..].starts_with(word) {
                    occurrences.push((start, word_start + word.len()));
                }
            }
        }
        Self {
            len: text.len(),
            breaks,
            line_breaks,
            ats,
            directions,
        }
    }
}

struct Lexer {
    pos: usize,
    states: StateStack<State>,
    /// Whether the next graph keyword is the first one, which starts the header (`firstGraph`).
    first_graph: bool,
    index: Index,
    dialect: &'static Dialect,
}

impl Lexer {
    /// The first rule of the current state that matches at the start of `rest`, in the order of
    /// `flow.jison`. Every condition names its start condition, as the grammar does: a rule
    /// without one applies in `Initial` only, and a rule that the grammar gives to every state
    /// (`<*>`) has no condition.
    fn rule(&mut self, rest: &str, state: State) -> Option<Hit> {
        use State::*;

        // accTitle\s*":"\s*  and  accDescr\s*":"\s*  (INITIAL), then the value up to the line end.
        if state == Initial
            && let Some(len) = delimited_form_len(rest, "accTitle", ':')
        {
            self.states.push(AccTitle);
            return hit(len, Kind::AccTitle);
        }
        if state == AccTitle {
            self.states.pop();
            return hit(line_len(rest), Kind::AccTitleValue);
        }
        if state == Initial
            && let Some(len) = delimited_form_len(rest, "accDescr", ':')
        {
            self.states.push(AccDescr);
            return hit(len, Kind::AccDescr);
        }
        if state == AccDescr {
            self.states.pop();
            return hit(line_len(rest), Kind::AccDescrValue);
        }
        // accDescr\s*"{"\s*  opens the block, which runs to its closing brace.
        if state == Initial
            && let Some(len) = delimited_form_len(rest, "accDescr", '{')
        {
            self.states.push(AccDescrMulti);
            return silent(len);
        }
        if state == AccDescrMulti {
            if rest.starts_with('}') {
                self.states.pop();
                return silent(1);
            }
            let len = rest.find('}').unwrap_or(rest.len());
            return hit(len, Kind::AccDescrMultiValue);
        }

        // Shape data: `@{` opens it; quoted strings in it keep their line breaks as <br/>.
        if state == Initial && rest.starts_with("@{") {
            self.states.push(ShapeData);
            return Some(Hit {
                len: 2,
                kind: Some(Kind::ShapeData),
                value: Some(String::new()),
            });
        }
        if state == ShapeData && rest.starts_with('"') {
            self.states.push(ShapeDataStr);
            return hit(1, Kind::ShapeData);
        }
        if state == ShapeDataStr && rest.starts_with('"') {
            self.states.pop();
            return hit(1, Kind::ShapeData);
        }
        if state == ShapeDataStr {
            let len = rest.find('"').unwrap_or(rest.len());
            return Some(Hit {
                len,
                kind: Some(Kind::ShapeData),
                value: Some(line_breaks_to_br(&rest[..len])),
            });
        }
        if state == ShapeData {
            let len = rest.find(['}', '^', '"']).unwrap_or(rest.len());
            if len > 0 {
                return hit(len, Kind::ShapeData);
            }
            if rest.starts_with('}') {
                self.states.pop();
                return silent(1);
            }
            return None;
        }

        // call\s+ opens the callback name; the name runs to `(`, the arguments to `)`.
        if state == Initial
            && let Some(word_end) = literal_len(rest, "call")
            && space_len(&rest[word_end..]) > 0
        {
            let spaces = space_len(&rest[word_end..]);
            self.states.push(CallbackName);
            return silent(word_end + spaces);
        }
        if state == CallbackName {
            if let Some(len) = callback_empty_args_len(rest) {
                self.states.pop();
                return silent(len);
            }
            if rest.starts_with('(') {
                self.states.pop();
                self.states.push(CallbackArgs);
                return silent(1);
            }
            let len = rest.find('(').unwrap_or(rest.len());
            return hit(len, Kind::CallbackName);
        }
        if state == CallbackArgs {
            if rest.starts_with(')') {
                self.states.pop();
                return silent(1);
            }
            let len = rest.find(')').unwrap_or(rest.len());
            return hit(len, Kind::CallbackArgs);
        }

        // Markdown strings: the text between backticks, closed by a backtick and a quote.
        if state == MdString {
            if rest.starts_with("`\"") {
                self.states.pop();
                return silent(2);
            }
            let len = rest.find(['`', '"']).unwrap_or(rest.len());
            if len > 0 {
                return hit(len, Kind::MdStr);
            }
            return None;
        }
        // A quote followed by a backtick opens a Markdown string, in every state.
        if rest.starts_with("\"`") {
            self.states.push(MdString);
            return silent(2);
        }
        // Plain strings: the text between quotes, which may not contain one.
        if state == Quoted {
            if rest.starts_with('"') {
                self.states.pop();
                return silent(1);
            }
            let len = rest.find('"').unwrap_or(rest.len());
            if len > 0 {
                return hit(len, Kind::Str);
            }
            return None;
        }
        if rest.starts_with('"') {
            self.states.push(Quoted);
            return silent(1);
        }

        // Keywords of the INITIAL state; each literal ends at a word boundary.
        if state == Initial
            && let Some(hit) = keyword(rest, self.dialect)
        {
            return Some(hit);
        }

        // `click` opens the click state; there, the id runs to the next blank, which ends it.
        if state == Initial
            && let Some(word_end) = literal_len(rest, "click")
            && space_len(&rest[word_end..]) > 0
        {
            let spaces = space_len(&rest[word_end..]);
            self.states.push(Click);
            return silent(word_end + spaces);
        }
        if state == Click {
            if let Some(c) = rest.chars().next().filter(|&c| is_js_space(c)) {
                self.states.pop();
                return silent(c.len_utf8());
            }
            let len = rest.find(is_js_space).unwrap_or(rest.len());
            return hit(len, Kind::Click);
        }

        // The first graph keyword opens the direction state.
        if state == Initial
            && let Some(len) = graph_keyword_len(rest, self.dialect)
        {
            if self.first_graph {
                self.first_graph = false;
                self.states.push(Dir);
            }
            return hit(len, Kind::Graph);
        }
        if state == Dir {
            return self.direction_state(rest);
        }

        // `.*direction\s+TB…` and the other directions: the rest of the line, from the start of
        // the token, which may take earlier text of the same line.
        if state == Initial {
            for (word, kind) in DIRECTION_WORDS.iter().zip([
                Kind::DirectionTb,
                Kind::DirectionBt,
                Kind::DirectionRl,
                Kind::DirectionLr,
                Kind::DirectionTd,
            ]) {
                if let Some(len) = self.direction_len(rest, word) {
                    return hit(len, kind);
                }
            }
        }

        // Link ids, numbers, and the single-character tokens of the INITIAL state.
        if state == Initial
            && let Some(len) = self.link_id_len(rest)
        {
            return hit(len, Kind::LinkId);
        }
        if state == Initial
            && let Some(len) = digits_len(rest)
        {
            return hit(len, Kind::Num);
        }
        if state == Initial {
            for (literal, kind) in [
                (":::", Kind::StyleSeparator),
                ("#", Kind::Brkt),
                (":", Kind::Colon),
                ("&", Kind::Amp),
                (";", Kind::Semi),
                (",", Kind::Comma),
                ("*", Kind::Mult),
            ] {
                if let Some(len) = literal_len(rest, literal) {
                    return hit(len, kind);
                }
            }
        }

        // The agentflow link operators, where the dialect has them (see `Links`).
        if self.dialect.links == Links::Agentflow
            && let Some(hit) = self.agentflow_links(rest, state)
        {
            return Some(hit);
        }

        // Links: the solid, thick and dotted arrows, each with its start and its end; the text
        // inside a link is read in the state its start opens.
        if self.dialect.links == Links::Flowchart
            && (state == Initial || state == EdgeText)
            && let Some(len) = link_end_len(rest, '-')
        {
            self.states.pop();
            return hit(len, Kind::Link);
        }
        if self.dialect.links == Links::Flowchart
            && state == Initial
            && let Some(len) = link_start_len(rest, '-')
        {
            self.states.push(EdgeText);
            return hit(len, Kind::StartLink);
        }
        if self.dialect.links == Links::Flowchart
            && state == EdgeText
            && let Some(len) = edge_text_char_len(rest, '-')
        {
            return hit(len, Kind::EdgeText);
        }
        if self.dialect.links == Links::Flowchart
            && (state == Initial || state == ThickEdgeText)
            && let Some(len) = link_end_len(rest, '=')
        {
            self.states.pop();
            return hit(len, Kind::Link);
        }
        if self.dialect.links == Links::Flowchart
            && state == Initial
            && let Some(len) = link_start_len(rest, '=')
        {
            self.states.push(ThickEdgeText);
            return hit(len, Kind::StartLink);
        }
        if self.dialect.links == Links::Flowchart
            && state == ThickEdgeText
            && let Some(len) = edge_text_char_len(rest, '=')
        {
            return hit(len, Kind::EdgeText);
        }
        if self.dialect.links == Links::Flowchart
            && (state == Initial || state == DottedEdgeText)
            && let Some(len) = dotted_link_end_len(rest)
        {
            self.states.pop();
            return hit(len, Kind::Link);
        }
        if self.dialect.links == Links::Flowchart
            && state == Initial
            && let Some(len) = dotted_link_start_len(rest)
        {
            self.states.push(DottedEdgeText);
            return hit(len, Kind::StartLink);
        }
        if self.dialect.links == Links::Flowchart
            && state == DottedEdgeText
            && let Some(len) = dotted_edge_text_len(rest)
        {
            return hit(len, Kind::EdgeText);
        }
        if self.dialect.links == Links::Flowchart
            && let Some(len) = invisible_link_len(rest)
        {
            return hit(len, Kind::Link);
        }

        // Comments of the dialect that skip the text of an ellipse or a trapezoid, and the text of
        // a shape's brackets, up to the end of the line.
        if self.dialect.comments
            && (state == EllipseText || state == TrapText || state == Text)
            && let Some(len) = comment_len(rest)
        {
            return silent(len);
        }

        // Ellipse text: `(-` … `-)`.
        if state == EllipseText
            && let Some(len) = ellipse_end_len(rest)
        {
            self.states.pop();
            return hit(len, Kind::EllipseEnd);
        }
        if state == EllipseText
            && let Some(c) = rest.chars().next()
            && !matches!(c, '(' | ')' | '[' | ']' | '{' | '}')
        {
            return hit(c.len_utf8(), Kind::Text);
        }
        if rest.starts_with("(-") {
            self.states.push(EllipseText);
            return hit(2, Kind::EllipseStart);
        }

        // Shape brackets: the openers push the text state, the closers pop it.
        if state == Text && rest.starts_with("])") {
            self.states.pop();
            return hit(2, Kind::StadiumEnd);
        }
        if rest.starts_with("([") {
            self.states.push(Text);
            return hit(2, Kind::StadiumStart);
        }
        if state == Text && rest.starts_with("]]") {
            self.states.pop();
            return hit(2, Kind::SubroutineEnd);
        }
        if rest.starts_with("[[") {
            self.states.push(Text);
            return hit(2, Kind::SubroutineStart);
        }
        if state == Initial && rest.starts_with("[|") {
            return hit(2, Kind::VertexWithPropsStart);
        }
        if state == Initial && rest.starts_with('>') {
            self.states.push(Text);
            return hit(1, Kind::TagEnd);
        }
        if state == Text && rest.starts_with(")]") {
            self.states.pop();
            return hit(2, Kind::CylinderEnd);
        }
        if rest.starts_with("[(") {
            self.states.push(Text);
            return hit(2, Kind::CylinderStart);
        }
        if state == Text && rest.starts_with(")))") {
            self.states.pop();
            return hit(3, Kind::DoubleCircleEnd);
        }
        if rest.starts_with("(((") {
            self.states.push(Text);
            return hit(3, Kind::DoubleCircleStart);
        }

        // Trapezoids: the ends are a bracket and a closing character; the body keeps slashes.
        if state == TrapText
            && let Some(kind) = trap_end_kind(rest)
        {
            self.states.pop();
            return hit(2, kind);
        }
        if state == TrapText
            && let Some(len) = trap_text_len(rest, self.dialect.comments)
        {
            return hit(len, Kind::Text);
        }
        if rest.starts_with("[/") {
            self.states.push(TrapText);
            return hit(2, Kind::TrapStart);
        }
        if rest.starts_with("[\\") {
            self.states.push(TrapText);
            return hit(2, Kind::InvTrapStart);
        }

        // Single characters and keywords of the INITIAL state.
        if state == Initial && rest.starts_with('<') {
            return hit(1, Kind::TagStart);
        }
        if state == Initial && rest.starts_with('^') {
            return hit(1, Kind::Up);
        }
        if state == Initial
            && let Some(len) = word_len(rest, "v")
        {
            return hit(len, Kind::Down);
        }

        // The comment token of the INITIAL state (agentflow): blanks, then `%%` to the end of the
        // line. It comes before the node strings, whose class holds `%`.
        if state == Initial
            && self.dialect.comments
            && let Some(len) = comment_token_len(rest)
        {
            return hit(len, Kind::Comment);
        }

        // Node strings, the minus sign, and characters of UNICODE_TEXT (INITIAL only).
        if state == Initial {
            let len = node_string_len(rest);
            if len > 0 {
                return hit(len, Kind::NodeString);
            }
            if rest.starts_with('-') {
                return hit(1, Kind::Minus);
            }
            if let Some(c) = rest.chars().next()
                && is_unicode_text(c)
            {
                return hit(c.len_utf8(), Kind::UnicodeText);
            }
        }

        // Pipes open a text state; the closing pipe pops it.
        if state == Text && rest.starts_with('|') {
            self.states.pop();
            return hit(1, Kind::Pipe);
        }
        if rest.starts_with('|') {
            self.states.push(Text);
            return hit(1, Kind::Pipe);
        }

        // Plain brackets and the text between them.
        if state == Text && rest.starts_with(')') {
            self.states.pop();
            return hit(1, Kind::Pe);
        }
        if rest.starts_with('(') {
            self.states.push(Text);
            return hit(1, Kind::Ps);
        }
        if state == Text && rest.starts_with(']') {
            self.states.pop();
            return hit(1, Kind::Sqe);
        }
        if rest.starts_with('[') {
            self.states.push(Text);
            return hit(1, Kind::Sqs);
        }
        if state == Text && rest.starts_with('}') {
            self.states.pop();
            return hit(1, Kind::DiamondStop);
        }
        if rest.starts_with('{') {
            self.states.push(Text);
            return hit(1, Kind::DiamondStart);
        }
        if state == Text {
            let len = run_len(
                rest,
                &['[', ']', '(', ')', '{', '}', '|', '"'],
                self.dialect.comments,
            );
            if len > 0 {
                return hit(len, Kind::Text);
            }
        }

        // Line breaks and spaces (INITIAL only).
        if state == Initial {
            if let Some(len) = newline_len(rest) {
                return hit(len, Kind::Newline);
            }
            let len = if self.dialect.space_run {
                rest.char_indices()
                    .find(|&(_, c)| !is_js_space(c) || c == '\n' || c == '\r')
                    .map_or(rest.len(), |(index, _)| index)
            } else {
                rest.chars()
                    .next()
                    .filter(|&c| is_js_space(c))
                    .map_or(0, char::len_utf8)
            };
            if len > 0 {
                return hit(len, Kind::Space);
            }
        }
        None
    }

    /// The agentflow link operators (`agentflow.jison`, lines 181 to 185): `\s*\-\-+[x>]\s*` ends a
    /// solid link, `\s*\-\-\s*` starts one with its text, and `\s*\-\.+\-\s*` is a dotted link with
    /// no text.
    fn agentflow_links(&mut self, rest: &str, state: State) -> Option<Hit> {
        use State::*;

        if (state == Initial || state == EdgeText)
            && let Some(len) = agentflow_solid_end_len(rest)
        {
            self.states.pop();
            return hit(len, Kind::Link);
        }
        if state == Initial
            && let Some(len) = agentflow_solid_start_len(rest)
        {
            self.states.push(EdgeText);
            return hit(len, Kind::StartLink);
        }
        if state == EdgeText
            && let Some(len) = edge_text_char_len(rest, '-')
        {
            return hit(len, Kind::EdgeText);
        }
        if state == Initial
            && let Some(len) = agentflow_dotted_len(rest)
        {
            return hit(len, Kind::Link);
        }
        None
    }

    /// The direction state, after `graph`: the header's direction, or the line break that ends
    /// the header without one.
    fn direction_state(&mut self, rest: &str) -> Option<Hit> {
        if let Some(len) = newline_then_blank_len(rest) {
            self.states.pop();
            return hit(len, Kind::NoDir);
        }
        let spaces = space_len(rest);
        for word in ["LR", "RL", "TB", "BT", "TD", "BR", "<", ">", "^", "v"] {
            if rest[spaces..].starts_with(word) {
                self.states.pop();
                return hit(spaces + word.len(), Kind::Dir);
            }
        }
        None
    }
}

impl Lexer {
    /// `[^\s"]+@(?=[^{"])` at the current position: an id followed by `@`, with the longest run
    /// the look-ahead allows. The `@` must come after at least one character of the id.
    fn link_id_len(&self, rest: &str) -> Option<usize> {
        let pos = self.pos;
        let breaks = &self.index.breaks;
        let run_end = breaks
            .get(breaks.partition_point(|&b| b < pos))
            .copied()
            .unwrap_or(self.index.len);
        let ats = &self.index.ats;
        let upper = ats.partition_point(|&at| at < run_end);
        for &at in ats[..upper].iter().rev() {
            if at <= pos {
                break;
            }
            let rel = at + 1 - pos;
            if let Some(next) = rest.get(rel..).and_then(|tail| tail.chars().next())
                && next != '{'
                && next != '"'
            {
                return Some(rel);
            }
        }
        None
    }

    /// `.*direction\s+WORD[^\n]*` at the current position: the last `direction` on the line that a
    /// `\s+WORD` follows, and the rest of the line. Earlier text of the line belongs to the token.
    fn direction_len(&self, rest: &str, word: &str) -> Option<usize> {
        let pos = self.pos;
        let word_index = DIRECTION_WORDS.iter().position(|&w| w == word)?;
        let line_breaks = &self.index.line_breaks;
        let line_end = line_breaks
            .get(line_breaks.partition_point(|&b| b < pos))
            .copied()
            .unwrap_or(self.index.len);
        let occurrences = &self.index.directions[word_index];
        let upper = occurrences.partition_point(|&(start, _)| start < line_end);
        let &(start, end) = occurrences[..upper].last()?;
        if start < pos {
            return None;
        }
        let rel_end = end - pos;
        let line_rest = rest.get(rel_end..)?;
        Some(rel_end + line_rest.find('\n').unwrap_or(line_rest.len()))
    }
}

/// The keyword of the INITIAL state at the start of `rest`, with its length: the keyword table of
/// the dialect, then `href`, which takes the blank after it.
fn keyword(rest: &str, dialect: &Dialect) -> Option<Hit> {
    for keyword in dialect.keywords {
        if let Some(len) = word_len(rest, keyword.word) {
            let trailing = match keyword.trailing {
                Trailing::Nothing => 0,
                Trailing::Blanks => {
                    rest[len..].len() - rest[len..].trim_start_matches([' ', '\t']).len()
                }
                Trailing::Spaces => space_len(&rest[len..]),
            };
            return hit(len + trailing, keyword.kind);
        }
    }
    if let Some(len) = literal_len(rest, "href")
        && let Some(c) = rest[len..].chars().next().filter(|&c| is_js_space(c))
    {
        return hit(len + c.len_utf8(), Kind::Href);
    }
    None
}

/// The graph keyword at the start of `rest`, with its length: the header words of the dialect.
fn graph_keyword_len(rest: &str, dialect: &Dialect) -> Option<usize> {
    dialect
        .graph_words
        .iter()
        .find_map(|word| word_len(rest, word))
}

/// `%%(?!\{)[^\n]*`: a comment at the start of `rest`, with its length. The `%%{` of a directive
/// is not one.
fn comment_len(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix("%%")?;
    if after.starts_with('{') {
        return None;
    }
    Some(2 + line_len(after))
}

/// `[ \t]*%%(?!\{)[^\n]*`: a comment token at the start of `rest`, the blanks before it included.
fn comment_token_len(rest: &str) -> Option<usize> {
    let blanks = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    Some(blanks + comment_len(&rest[blanks..])?)
}

/// The length of a run of characters that are none of `specials`. With `comments`, a `%%` ends the
/// run too, as the agentflow text rule `(?:[^…%]+|%(?!%))+` does; a single `%` is text.
fn run_len(rest: &str, specials: &[char], comments: bool) -> usize {
    let mut chars = rest.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if specials.contains(&c) {
            return index;
        }
        if comments && c == '%' && chars.peek().is_some_and(|&(_, next)| next == '%') {
            return index;
        }
    }
    rest.len()
}

/// `\s*\-\-+[x>]\s*`: the end of an agentflow solid link.
fn agentflow_solid_end_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    let dashes = rest[spaces..]
        .find(|c: char| c != '-')
        .unwrap_or(rest.len() - spaces);
    if dashes < 2 {
        return None;
    }
    let end = spaces + dashes;
    if !rest[end..].starts_with(['x', '>']) {
        return None;
    }
    let end = end + 1;
    Some(end + space_len(&rest[end..]))
}

/// `\s*\-\-\s*`: the start of an agentflow solid link that has text.
fn agentflow_solid_start_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    if !rest[spaces..].starts_with("--") {
        return None;
    }
    let end = spaces + 2;
    Some(end + space_len(&rest[end..]))
}

/// `\s*\-\.+\-\s*`: an agentflow dotted link, which has no text.
fn agentflow_dotted_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    let after_dash = rest[spaces..].strip_prefix('-')?;
    let dots = after_dash
        .find(|c: char| c != '.')
        .unwrap_or(after_dash.len());
    if dots == 0 {
        return None;
    }
    let after_dots = after_dash[dots..].strip_prefix('-')?;
    let end = rest.len() - after_dots.len();
    Some(end + space_len(after_dots))
}

/// The end of a trapezoid whose two characters `\(?=]`-class and `]` or `/]` close it.
fn trap_end_kind(rest: &str) -> Option<Kind> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    if second != ']' {
        return None;
    }
    match first {
        '\\' | '(' | '?' | '=' | ']' | ')' => Some(Kind::TrapEnd),
        '/' => Some(Kind::InvTrapEnd),
        _ => None,
    }
}

/// `\(\s*\)`: the empty argument list of a callback.
fn callback_empty_args_len(rest: &str) -> Option<usize> {
    let after_open = rest.strip_prefix('(')?;
    let spaces = space_len(after_open);
    after_open[spaces..]
        .starts_with(')')
        .then_some(1 + spaces + 1)
}

/// `[0-9]+`.
fn digits_len(rest: &str) -> Option<usize> {
    let len = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    (len > 0).then_some(len)
}

/// Length of `\s*[xo<]?` followed by a run of `bar` and a closing character from `[bar xo>]`, and
/// the spaces after it: the end of a solid (`-`) or thick (`=`) link. Greedy matching takes all
/// the bars first; a closing bar is given back when the run has three or more.
fn link_end_len(rest: &str, bar: char) -> Option<usize> {
    let (start, bars, after_bars) = bar_run(rest, bar)?;
    let next = rest[after_bars..].chars().next();
    let end = match next {
        Some(c) if bars >= 2 && matches!(c, 'x' | 'o' | '>') => after_bars + c.len_utf8(),
        Some(c) if c == bar && bars >= 3 => after_bars,
        _ if bars >= 3 => after_bars,
        _ => return None,
    };
    let _ = start;
    Some(end + space_len(&rest[end..]))
}

/// Length of `\s*[xo<]?\=\=\s*` (thick start) or `\s*[xo<]?\-\-\s*` (solid start).
fn link_start_len(rest: &str, bar: char) -> Option<usize> {
    let spaces = space_len(rest);
    let mut p = spaces;
    if rest[p..].starts_with(['x', 'o', '<']) {
        p += 1;
    }
    let two = format!("{bar}{bar}");
    if !rest[p..].starts_with(&two) {
        return None;
    }
    let end = p + 2;
    Some(end + space_len(&rest[end..]))
}

/// The run of `bar` after the optional leading space and `[xo<]`: its start, its length in bytes
/// of the run, and the byte offset just after it.
fn bar_run(rest: &str, bar: char) -> Option<(usize, usize, usize)> {
    let spaces = space_len(rest);
    let mut p = spaces;
    if rest[p..].starts_with(['x', 'o', '<']) {
        p += 1;
    }
    let run = rest[p..].find(|c: char| c != bar).unwrap_or(rest.len() - p);
    (run > 0).then_some((p, run, p + run))
}

/// `\s*[xo<]?\-?\.+\-[xo>]?\s*`: the end of a dotted link.
fn dotted_link_end_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    let mut p = spaces;
    if rest[p..].starts_with(['x', 'o', '<']) {
        p += 1;
    }
    if rest[p..].starts_with('-') && rest[p + 1..].starts_with('.') {
        p += 1;
    }
    let dots = rest[p..].find(|c: char| c != '.').unwrap_or(rest.len() - p);
    if dots == 0 || !rest[p + dots..].starts_with('-') {
        return None;
    }
    let mut end = p + dots + 1;
    if rest[end..].starts_with(['x', 'o', '>']) {
        end += 1;
    }
    Some(end + space_len(&rest[end..]))
}

/// `\s*[xo<]?\-\.\s*`: the start of a dotted link.
fn dotted_link_start_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    let mut p = spaces;
    if rest[p..].starts_with(['x', 'o', '<']) {
        p += 1;
    }
    if !rest[p..].starts_with("-.") {
        return None;
    }
    let end = p + 2;
    Some(end + space_len(&rest[end..]))
}

/// `[^-]|\-(?!\-)` in the solid edge text state, with `bar` the character that is not text.
fn edge_text_char_len(rest: &str, bar: char) -> Option<usize> {
    let c = rest.chars().next()?;
    if c != bar {
        return Some(c.len_utf8());
    }
    let after = rest[1..].chars().next();
    (after != Some(bar)).then_some(1)
}

/// `[^.]|\.(?!-)` in the dotted edge text state.
fn dotted_edge_text_len(rest: &str) -> Option<usize> {
    let c = rest.chars().next()?;
    if c != '.' {
        return Some(c.len_utf8());
    }
    let after = rest[1..].chars().next();
    (after != Some('-')).then_some(1)
}

/// `\s*~~[~]+\s*`: an invisible link.
fn invisible_link_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    let tildes = rest[spaces..]
        .find(|c: char| c != '~')
        .unwrap_or(rest.len() - spaces);
    if tildes < 3 {
        return None;
    }
    let end = spaces + tildes;
    Some(end + space_len(&rest[end..]))
}

/// `[-/)][)]`: the end of ellipse text.
fn ellipse_end_len(rest: &str) -> Option<usize> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    (matches!(first, '-' | '/' | ')') && second == ')').then_some(first.len_utf8() + 1)
}

/// The trapezoid text rules of `flow.jison` after the end patterns: a slash or a backslash not
/// followed by `]`, or a run of characters that are none of `\[](){}/`.
fn trap_text_len(rest: &str, comments: bool) -> Option<usize> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    if matches!(first, '/' | '\\') {
        return (!rest[first.len_utf8()..].starts_with(']')).then_some(first.len_utf8());
    }
    let len = run_len(rest, &['\\', '[', ']', '(', ')', '{', '}', '/'], comments);
    (len > 0).then_some(len)
}

/// The characters of `NODE_STRING`: `[A-Za-z0-9!"#$%&'*+._`?\/]`, a `-` not followed by `>`,
/// `-` or `.`, and an `=` not followed by `=`. The quote is inside the class, so a quote after
/// the first character of an id belongs to the id.
fn node_string_len(rest: &str) -> usize {
    let mut end = 0;
    let chars = rest.char_indices();
    for (index, c) in chars {
        let next = rest[index + c.len_utf8()..].chars().next();
        let take = if c.is_ascii_alphanumeric() || "!\"#$%&'*+._`?\\/".contains(c) {
            true
        } else if c == '-' {
            next.is_some_and(|n| !matches!(n, '>' | '-' | '.'))
        } else if c == '=' {
            next != Some('=')
        } else {
            false
        };
        if !take {
            break;
        }
        end = index + c.len_utf8();
    }
    end
}

/// `(\r?\n)+`.
fn newline_len(rest: &str) -> Option<usize> {
    let mut end = 0;
    loop {
        let tail = &rest[end..];
        if tail.starts_with("\r\n") {
            end += 2;
        } else if tail.starts_with('\n') {
            end += 1;
        } else {
            break;
        }
    }
    (end > 0).then_some(end)
}

/// `(\r?\n)*\s*\n` in the direction state: the leading whitespace up to its last line feed.
fn newline_then_blank_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    rest[..spaces].rfind('\n').map(|index| index + 1)
}

/// `\n\s*` replaced by `<br/>`, as the shape data of a quoted string is written.
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
