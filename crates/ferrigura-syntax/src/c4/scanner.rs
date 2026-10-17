//! The C4 scanner: the rules of `parser/c4Diagram.jison`, in the order the file gives them, with its
//! start conditions. The first rule that matches wins, as in Jison, and the rules of the
//! structure states apply to every element, boundary, node, relation and update state alike. See
//! `docs-port/c4/PLAN.md`, sections 2 and 3, for the rule table and the quirks it keeps.
//!
//! The scanner reproduces Jison's `next()`, including its `done` flag: once the input is empty,
//! the next call reads the rules of the current state at the end of the input, and after that every
//! call returns Jison's raw end symbol, which the grammar does not accept.

use crate::scan::{StateStack, delimited_form_len, line_len, space_len, word_len};
use crate::text::is_js_space;

/// The start conditions the lexer enters: the initial state, the structure states that the
/// keywords enter, and the states of the attributes and of the accessibility texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    /// Any element, boundary, node, relation or update state: `(` opens its attributes.
    Struct,
    Attribute,
    Str,
    StrKv,
    StrKvKey,
    StrKvValue,
    AccTitle,
    AccDescr,
    AccDescrMulti,
}

/// The kind of a boundary keyword: which DB function it calls, and the kind it splices in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundaryKind {
    /// `Boundary`: the arguments are passed on unchanged.
    Plain,
    /// `Enterprise_Boundary`: `ENTERPRISE` is spliced in as the third argument.
    Enterprise,
    /// `System_Boundary`: `SYSTEM` is spliced in as the third argument.
    System,
    /// `Container_Boundary`: `CONTAINER` is spliced in, and the container DB function is called.
    Container,
    /// `Node`, `Deployment_Node`, `Node_L` or `Node_R`: a deployment node with its own kind.
    Node(&'static str),
}

/// The macro that a keyword begins, with the DB function and kind it calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Macro {
    /// `addPersonOrSystem(kind, …)`.
    PersonOrSystem(&'static str),
    /// `addContainer(kind, …)`.
    Container(&'static str),
    /// `addComponent(kind, …)`.
    Component(&'static str),
    /// A boundary or a deployment node: its statement has a body.
    Boundary(BoundaryKind),
    /// `addRel(kind, …)`.
    Rel(&'static str),
    /// `RelIndex`: the first argument, the index, is dropped, then `addRel('rel', …)`.
    RelIndex,
    /// `updateElStyle`.
    UpdateElementStyle,
    /// `updateRelStyle`.
    UpdateRelStyle,
    /// `updateLayoutConfig`.
    UpdateLayoutConfig,
}

/// The tokens of the grammar. The keywords that carry no value are unit variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// One of the five headers, with its text.
    Header(&'static str),
    /// A line that contains `direction` followed by `TB`, `BT`, `RL` or `LR`: an error in the
    /// grammar (`start : direction` calls a DB function that does not exist).
    Direction,
    /// `title`, with the value after its first six characters.
    Title(String),
    /// `accDescription`, with the value after its first fifteen characters.
    AccDescription(String),
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
    /// One or more line breaks.
    Newline,
    /// A keyword that begins a statement.
    Keyword(Macro),
    /// `{`.
    LBrace,
    /// `}`.
    RBrace,
    /// An empty attribute: `(,`, `,,` or `""`.
    AttrEmpty,
    /// An attribute text, quoted or not, before trimming.
    Str(String),
    /// The key of a `$key="value"` attribute, before trimming.
    StrKey(String),
    /// The value of a `$key="value"` attribute, before trimming.
    StrValue(String),
    /// The end of the input inside a structure: the grammar rejects it.
    EofInStruct,
    /// The end of the input, as the `EOF` terminal of the grammar.
    Eof,
    /// Jison's raw end symbol, returned once the input is done. The grammar does not accept it.
    RawEof,
    /// A character no rule takes: an error in the grammar.
    Invalid,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// The keywords, in the order of the lexer rules. Each keyword ends with a word boundary, so a
/// longer keyword with the same start is tried first only where the order matters.
const KEYWORDS: &[(&str, Macro)] = &[
    ("Person_Ext", Macro::PersonOrSystem("external_person")),
    ("Person", Macro::PersonOrSystem("person")),
    (
        "SystemQueue_Ext",
        Macro::PersonOrSystem("external_system_queue"),
    ),
    ("SystemDb_Ext", Macro::PersonOrSystem("external_system_db")),
    ("System_Ext", Macro::PersonOrSystem("external_system")),
    ("SystemQueue", Macro::PersonOrSystem("system_queue")),
    ("SystemDb", Macro::PersonOrSystem("system_db")),
    ("System", Macro::PersonOrSystem("system")),
    ("Boundary", Macro::Boundary(BoundaryKind::Plain)),
    (
        "Enterprise_Boundary",
        Macro::Boundary(BoundaryKind::Enterprise),
    ),
    ("System_Boundary", Macro::Boundary(BoundaryKind::System)),
    (
        "ContainerQueue_Ext",
        Macro::Container("external_container_queue"),
    ),
    ("ContainerDb_Ext", Macro::Container("external_container_db")),
    ("Container_Ext", Macro::Container("external_container")),
    ("ContainerQueue", Macro::Container("container_queue")),
    ("ContainerDb", Macro::Container("container_db")),
    ("Container", Macro::Container("container")),
    (
        "Container_Boundary",
        Macro::Boundary(BoundaryKind::Container),
    ),
    (
        "ComponentQueue_Ext",
        Macro::Component("external_component_queue"),
    ),
    ("ComponentDb_Ext", Macro::Component("external_component_db")),
    ("Component_Ext", Macro::Component("external_component")),
    ("ComponentQueue", Macro::Component("component_queue")),
    ("ComponentDb", Macro::Component("component_db")),
    ("Component", Macro::Component("component")),
    (
        "Deployment_Node",
        Macro::Boundary(BoundaryKind::Node("node")),
    ),
    ("Node", Macro::Boundary(BoundaryKind::Node("node"))),
    ("Node_L", Macro::Boundary(BoundaryKind::Node("nodeL"))),
    ("Node_R", Macro::Boundary(BoundaryKind::Node("nodeR"))),
    ("Rel", Macro::Rel("rel")),
    ("BiRel", Macro::Rel("birel")),
    ("Rel_Up", Macro::Rel("rel_u")),
    ("Rel_U", Macro::Rel("rel_u")),
    ("Rel_Down", Macro::Rel("rel_d")),
    ("Rel_D", Macro::Rel("rel_d")),
    ("Rel_Left", Macro::Rel("rel_l")),
    ("Rel_L", Macro::Rel("rel_l")),
    ("Rel_Right", Macro::Rel("rel_r")),
    ("Rel_R", Macro::Rel("rel_r")),
    ("Rel_Back", Macro::Rel("rel_b")),
    ("RelIndex", Macro::RelIndex),
    ("UpdateElementStyle", Macro::UpdateElementStyle),
    ("UpdateRelStyle", Macro::UpdateRelStyle),
    ("UpdateLayoutConfig", Macro::UpdateLayoutConfig),
];

/// The five headers, in the order of the lexer rules.
const HEADERS: &[&str] = &[
    "C4Context",
    "C4Container",
    "C4Component",
    "C4Dynamic",
    "C4Deployment",
];

/// Reads the whole text. The result ends with the first end token, `Eof` or `RawEof`.
pub(crate) fn scan(text: &str) -> Vec<Spanned> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        stack: StateStack::new(State::Initial),
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

/// What a rule did: a token of the given byte length, or a skipped run of the given length.
enum Step {
    Emit(Token, usize),
    Skip(usize),
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    stack: StateStack<State>,
    done: bool,
}

impl<'a> Scanner<'a> {
    /// The text that is not read yet. It borrows the text, not the scanner, so a rule can read it
    /// and then change the state.
    fn rest(&self) -> &'a str {
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
            match self.rules() {
                Some(Step::Emit(token, consumed)) => {
                    self.pos = start + consumed;
                    return Spanned { token, start };
                }
                Some(Step::Skip(consumed)) => {
                    self.pos = start + consumed;
                }
                None if !self.rest().is_empty() => {
                    // No rule takes this character: an error in the grammar.
                    let consumed = self.rest().chars().next().map_or(1, char::len_utf8);
                    self.pos = start + consumed;
                    return Spanned {
                        token: Token::Invalid,
                        start,
                    };
                }
                // The input is empty and no rule matched: the next call returns the raw end.
                None => {}
            }
        }
    }

    /// The rules of the current state, in the order of the grammar file.
    fn rules(&mut self) -> Option<Step> {
        match self.stack.top() {
            State::Initial => self.initial_rules(),
            State::Struct => self.struct_rules(),
            State::Attribute => self.attribute_rules(),
            State::Str => self.string_rules(),
            State::StrKv => self.string_kv_rules(),
            State::StrKvKey => self.string_kv_key_rules(),
            State::StrKvValue => self.string_kv_value_rules(),
            State::AccTitle => self.value_rules(State::AccTitle),
            State::AccDescr => self.value_rules(State::AccDescr),
            State::AccDescrMulti => self.multi_value_rules(),
        }
    }

    /// The rules that apply in the initial state, lines 81-186 of the grammar.
    fn initial_rules(&mut self) -> Option<Step> {
        let rest = self.rest();

        // 81-84: a line with `direction` followed by a direction name, anywhere in the line.
        if direction_on_line(rest) {
            return Some(Step::Emit(Token::Direction, line_len(rest)));
        }
        // 87: `title`, one white space character, then one or more characters that are not `#`,
        // a line break or `;`. The value is the text after the first six characters.
        if let Some((value, consumed)) = keyword_with_value(rest, "title") {
            return Some(Step::Emit(Token::Title(value), consumed));
        }
        // 88: `accDescription`, the same shape, with fifteen characters before the value.
        if let Some((value, consumed)) = keyword_with_value(rest, "accDescription") {
            return Some(Step::Emit(Token::AccDescription(value), consumed));
        }
        // 89: `accTitle\s*":"\s*` enters the accessibility title state.
        if let Some(consumed) = delimited_form_len(rest, "accTitle", ':') {
            self.stack.push(State::AccTitle);
            return Some(Step::Emit(Token::AccTitle, consumed));
        }
        // 91: `accDescr\s*":"\s*` enters the accessibility description state.
        if let Some(consumed) = delimited_form_len(rest, "accDescr", ':') {
            self.stack.push(State::AccDescr);
            return Some(Step::Emit(Token::AccDescr, consumed));
        }
        // 93: `accDescr\s*"{"\s*` enters the multi-line state, without a token.
        if let Some(consumed) = delimited_form_len(rest, "accDescr", '{') {
            self.stack.push(State::AccDescrMulti);
            return Some(Step::Skip(consumed));
        }
        // 98: `%%` and the rest of the line, with the line breaks after it: a comment.
        if let Some(consumed) = comment_len(rest) {
            return Some(Step::Skip(consumed));
        }
        // 101: `\s*(\r?\n)+`, the white space up to the last line break.
        if let Some(consumed) = newline_len(rest) {
            return Some(Step::Emit(Token::Newline, consumed));
        }
        // 102: `\s+` without a line break.
        let blanks = space_len(rest);
        if blanks > 0 {
            return Some(Step::Skip(blanks));
        }
        // 103-107: the headers.
        for &header in HEADERS {
            if let Some(consumed) = word_len(rest, header) {
                return Some(Step::Emit(Token::Header(header), consumed));
            }
        }
        // 109-159: the keywords. Each one enters its structure state.
        for (word, macro_kind) in KEYWORDS {
            if let Some(consumed) = word_len(rest, word) {
                self.stack.push(State::Struct);
                return Some(Step::Emit(Token::Keyword(*macro_kind), consumed));
            }
        }
        // 181-182: the braces.
        match rest.chars().next() {
            Some('{') => return Some(Step::Emit(Token::LBrace, 1)),
            Some('}') => return Some(Step::Emit(Token::RBrace, 1)),
            _ => {}
        }
        // 186: the end of the input.
        if rest.is_empty() {
            return Some(Step::Emit(Token::Eof, 0));
        }
        None
    }

    /// The rules of the structure states, lines 161-164. They apply to every structure state, and
    /// the `)` rule also applies in the attribute state.
    fn struct_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        if rest.is_empty() {
            return Some(Step::Emit(Token::EofInStruct, 0));
        }
        if let Some(step) = self.opening_paren(rest) {
            return Some(step);
        }
        self.closing_paren(rest)
    }

    /// `[(][ ]*[,]` and `[(]`: the attribute state begins, and `(,` gives an empty attribute.
    fn opening_paren(&mut self, rest: &str) -> Option<Step> {
        let after = rest.strip_prefix('(')?;
        let spaces = after.len() - after.trim_start_matches(' ').len();
        if after[spaces..].starts_with(',') {
            self.stack.push(State::Attribute);
            return Some(Step::Emit(Token::AttrEmpty, 1 + spaces + 1));
        }
        self.stack.push(State::Attribute);
        Some(Step::Skip(1))
    }

    /// `[)]`: pops the attribute state and the structure state beneath it.
    fn closing_paren(&mut self, rest: &str) -> Option<Step> {
        if rest.starts_with(')') {
            self.stack.pop();
            self.stack.pop();
            return Some(Step::Skip(1));
        }
        None
    }

    /// The rules of the attribute state, lines 164-179, in their order.
    fn attribute_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        if let Some(step) = self.closing_paren(rest) {
            return Some(step);
        }
        // 166: `,,` is an empty attribute.
        if rest.starts_with(",,") {
            return Some(Step::Emit(Token::AttrEmpty, 2));
        }
        // 167: a single comma separates the attributes and is skipped.
        if rest.starts_with(',') {
            return Some(Step::Skip(1));
        }
        let spaces = rest.len() - rest.trim_start_matches(' ').len();
        let after = &rest[spaces..];
        // 168: `[ ]*""`, an empty quoted attribute.
        if after.starts_with("\"\"") {
            return Some(Step::Emit(Token::AttrEmpty, spaces + 2));
        }
        // 169: `[ ]*"` begins a quoted text.
        if after.starts_with('"') {
            self.stack.push(State::Str);
            return Some(Step::Skip(spaces + 1));
        }
        // 173: `[ ]*$` begins a `$key="value"` attribute.
        if after.starts_with('$') {
            self.stack.push(State::StrKv);
            return Some(Step::Skip(spaces + 1));
        }
        // 179: `[^,]+` is an unquoted text, up to the next comma or the end of the input. It runs
        // over the closing parenthesis and the line breaks (`c4-grammar` quirk P2).
        let run = run_until(rest, &[',']);
        if run > 0 {
            return Some(Step::Emit(Token::Str(rest[..run].to_owned()), run));
        }
        None
    }

    /// The `string` state: the closing quote pops it, and `[^"]*` is the text up to the next quote.
    fn string_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        if rest.starts_with('"') {
            self.stack.pop();
            return Some(Step::Skip(1));
        }
        let run = run_until(rest, &['"']);
        Some(Step::Emit(Token::Str(rest[..run].to_owned()), run))
    }

    /// The `string_kv` state: `[^=]*` is the key, and the key state follows it.
    fn string_kv_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        let run = run_until(rest, &['=']);
        self.stack.push(State::StrKvKey);
        Some(Step::Emit(Token::StrKey(rest[..run].to_owned()), run))
    }

    /// The `string_kv_key` state: `=[ ]*"` moves on to the value.
    fn string_kv_key_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        let after = rest.strip_prefix('=')?;
        let spaces = after.len() - after.trim_start_matches(' ').len();
        if after[spaces..].starts_with('"') {
            self.stack.pop();
            self.stack.push(State::StrKvValue);
            return Some(Step::Skip(1 + spaces + 1));
        }
        None
    }

    /// The `string_kv_value` state: `[^"]+` is the value, and the closing quote pops both states.
    fn string_kv_value_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        if rest.starts_with('"') {
            self.stack.pop();
            self.stack.pop();
            return Some(Step::Skip(1));
        }
        let run = run_until(rest, &['"']);
        if run > 0 {
            return Some(Step::Emit(Token::StrValue(rest[..run].to_owned()), run));
        }
        None
    }

    /// The accessibility value states: `[^\n]*`, the rest of the line, possibly empty. The state
    /// is left at once.
    fn value_rules(&mut self, state: State) -> Option<Step> {
        let rest = self.rest();
        let run = line_len(rest);
        let value = rest[..run].to_owned();
        self.stack.pop();
        let token = if state == State::AccTitle {
            Token::AccTitleValue(value)
        } else {
            Token::AccDescrValue(value)
        };
        Some(Step::Emit(token, run))
    }

    /// The `acc_descr_multiline` state: `}` leaves it, and the text up to a `}` is one value.
    fn multi_value_rules(&mut self) -> Option<Step> {
        let rest = self.rest();
        if rest.starts_with('}') {
            self.stack.pop();
            return Some(Step::Skip(1));
        }
        let run = run_until(rest, &['}']);
        Some(Step::Emit(
            Token::AccDescrMultiValue(rest[..run].to_owned()),
            run,
        ))
    }
}

/// The byte length of the text before the first of `stops`, or of all of `text` when none is there.
fn run_until(text: &str, stops: &[char]) -> usize {
    text.find(|c: char| stops.contains(&c))
        .unwrap_or(text.len())
}

/// `title\s[^#\n;]+` and `accDescription\s[^#\n;]+`: the keyword, one white space character, then
/// the value. Returns the value, which is the text after the white space character, and the length
/// of the match.
fn keyword_with_value(rest: &str, word: &str) -> Option<(String, usize)> {
    let after = rest.strip_prefix(word)?;
    let blank = after.chars().next().filter(|c| is_js_space(*c))?;
    let body = &after[blank.len_utf8()..];
    let run = run_until(body, &['#', '\n', ';']);
    if run == 0 {
        return None;
    }
    Some((body[..run].to_owned(), word.len() + blank.len_utf8() + run))
}

/// `.*direction\s+(TB|BT|RL|LR)`: whether a line from the current position holds `direction`, then
/// white space (which may run over a line break), then a direction name. The `.` of the pattern
/// does not cross a line terminator, but the `\s+` does.
fn direction_on_line(rest: &str) -> bool {
    let line = run_until_terminator(rest);
    line.match_indices("direction").any(|(index, _)| {
        let after = &rest[index + "direction".len()..];
        let blanks = space_len(after);
        if blanks == 0 {
            return false;
        }
        let name = &after[blanks..];
        ["TB", "BT", "RL", "LR"]
            .iter()
            .any(|direction| name.starts_with(direction))
    })
}

/// The text before the first character that `.` does not match in JavaScript: a line break, a
/// carriage return, or U+2028 or U+2029.
fn run_until_terminator(rest: &str) -> &str {
    let end = rest
        .find(['\n', '\r', '\u{2028}', '\u{2029}'])
        .unwrap_or(rest.len());
    &rest[..end]
}

/// `\%\%(?!\{)*[^\n]*(\r?\n?)+`: `%%`, the rest of the line, and the line breaks after it. The
/// quantified look-ahead matches nothing, so `%%{` is a comment too (the directives are removed
/// before the lexer runs).
fn comment_len(rest: &str) -> Option<usize> {
    if !rest.starts_with("%%") {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut end = 2;
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }
    while end < bytes.len() && (bytes[end] == b'\n' || bytes[end] == b'\r') {
        end += 1;
    }
    Some(end)
}

/// `\s*(\r?\n)+`: the white space up to and including its last line feed, if it has one.
fn newline_len(rest: &str) -> Option<usize> {
    let blanks = space_len(rest);
    rest[..blanks].rfind('\n').map(|last| last + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<Token> {
        scan(text)
            .into_iter()
            .map(|spanned| spanned.token)
            .collect()
    }

    #[test]
    fn a_keyword_takes_the_structure_state_and_the_parenthesis_its_attributes() {
        assert_eq!(
            tokens("Person(a, \"A\")\n"),
            vec![
                Token::Keyword(Macro::PersonOrSystem("person")),
                Token::Str("a".to_owned()),
                Token::Str("A".to_owned()),
                Token::Newline,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn a_space_before_the_parenthesis_is_a_lexical_error() {
        assert!(tokens("Person (a, \"A\")\n").contains(&Token::Invalid));
    }

    #[test]
    fn a_direction_anywhere_in_a_line_is_a_direction_token() {
        assert_eq!(
            tokens("System(a, \"go direction TB now\")")[0],
            Token::Direction
        );
        assert_eq!(tokens("direction\n  TB")[0], Token::Direction);
        assert_ne!(tokens("directionTB")[0], Token::Direction);
    }

    #[test]
    fn an_unquoted_attribute_runs_over_the_closing_parenthesis() {
        assert_eq!(
            tokens("System(a)\nSystem(b, \"B\")")[1],
            Token::Str("a)\nSystem(b".to_owned())
        );
    }

    #[test]
    fn the_end_of_the_input_inside_a_quoted_text_is_the_raw_end() {
        let tokens = tokens("System(a, \"A");
        assert_eq!(tokens.last(), Some(&Token::RawEof));
    }
}
