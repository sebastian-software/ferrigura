//! The class lexer: the rules of `parser/classDiagram.jison` in the order they are written, with its
//! start conditions. The first rule that matches wins, as in Jison; a literal keyword that ends in
//! a word character matches only at a word boundary. See `docs-port/class/PLAN.md`, section 3.
//!
//! The lexer runs over the whole prepared text before parsing. No action of the grammar changes a
//! start condition, so the token sequence does not depend on the parser, and a lexical error
//! anywhere rejects the input (accepting needs the lexer at the end of the text).
//!
//! `NUM` is not produced: the `\w+` rule (`ALPHA`) comes first and matches digits, as in the
//! reference, where the `NUM` rule is dead.

use crate::flowchart::unicode::is_unicode_text;
use crate::scan::{StateStack, delimited_form_len, line_len, space_len, word_len};
use crate::text::{is_js_space, is_word};

/// The start conditions of the class lexer (the `%x` states of the grammar, and INITIAL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    String,
    BqString,
    Generic,
    AccTitle,
    AccDescr,
    AccDescrMulti,
    CallbackName,
    CallbackArgs,
    Namespace,
    NamespaceBody,
    Class,
    ClassBody,
}

/// The token kinds of the class grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Eof,
    EofInStruct,
    Newline,
    ClassDiagram,
    EdgeState,
    CallbackName,
    CallbackArgs,
    Str,
    Style,
    ClassDef,
    Namespace,
    StructStart,
    StructStop,
    Class,
    Member,
    OpenInStruct,
    CssClass,
    Callback,
    Link,
    Click,
    NoteFor,
    Note,
    AnnotationStart,
    AnnotationEnd,
    Href,
    GenericType,
    BqStr,
    LinkTarget,
    Extension,
    Dependency,
    Composition,
    Aggregation,
    Lollipop,
    Line,
    DottedLine,
    Label,
    StyleSeparator,
    Minus,
    Dot,
    Plus,
    Colon,
    Comma,
    Brkt,
    Pct,
    Equals,
    Alpha,
    Sqs,
    Sqe,
    Punctuation,
    UnicodeText,
    Space,
    DirectionTb,
    DirectionBt,
    DirectionRl,
    DirectionLr,
    AccTitle,
    AccTitleValue,
    AccDescr,
    AccDescrValue,
    AccDescrMultiValue,
}

/// One token: its kind, its value as Jison's `yytext` gives it, and its byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: Kind,
    pub text: String,
    pub start: usize,
}

/// No rule matches at byte `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexError {
    pub offset: usize,
}

/// What a rule does to the start-condition stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Nothing.
    Keep,
    /// `begin(state)`.
    Push(State),
    /// `popState()`.
    Pop,
    /// `popState()` and then `begin(state)`: the state on top is replaced.
    Replace(State),
    /// `popState()` twice.
    PopTwice,
    /// `popState()` and `less(0)`: the match is read again in the state below.
    PopRetry,
}

/// A rule that matched: how far it reads, the token it gives (if any) with its value, and the
/// change of start condition.
struct Hit<'a> {
    len: usize,
    token: Option<(Kind, &'a str)>,
    action: Action,
}

fn token<'a>(len: usize, kind: Kind, value: &'a str, action: Action) -> Option<Hit<'a>> {
    Some(Hit {
        len,
        token: Some((kind, value)),
        action,
    })
}

fn skip<'a>(len: usize, action: Action) -> Option<Hit<'a>> {
    Some(Hit {
        len,
        token: None,
        action,
    })
}

/// Whether `c` ends a line for the `.` of a JavaScript regular expression.
fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `.*direction\s+WORD[^\n]*` (INITIAL): the length of the match at the start of `rest`, if there
/// is one. The `.*` may not cross a line terminator, but `\s+` may. Among the possible starts the
/// last one wins, as the greedy `.*` takes it.
fn direction_len(rest: &str, word: &str) -> Option<usize> {
    let line_end = rest.find(is_line_terminator).unwrap_or(rest.len());
    let mut end = None;
    for (position, _) in rest[..line_end].match_indices("direction") {
        let after = &rest[position + "direction".len()..];
        let spaces = space_len(after);
        if spaces > 0 && after[spaces..].starts_with(word) {
            end = Some(position + "direction".len() + spaces + word.len());
        }
    }
    let end = end?;
    let tail = &rest[end..];
    Some(end + tail.find('\n').unwrap_or(tail.len()))
}

/// `%%[^\n]*(\r?\n)*`-like comment: the text after `%%` up to a line break, then all the line
/// breaks that follow, as `(\r?\n?)+` takes them. The line breaks are part of the token, so the
/// statement separator is lost (`classDiagram.jison` L28).
fn comment_len(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let mut index = 2;
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    while index < bytes.len() && matches!(bytes[index], b'\n' | b'\r') {
        index += 1;
    }
    index
}

/// `\s*(\r?\n)+`: the whitespace up to the last line break in its run, if the run has one.
fn newline_len(rest: &str) -> Option<usize> {
    let mut last = None;
    for (index, c) in rest.char_indices() {
        if !is_js_space(c) {
            break;
        }
        if c == '\n' {
            last = Some(index + 1);
        }
    }
    last
}

/// `\s*LITERAL`: the length of the blanks and the literal, if the literal follows the blanks.
fn after_space(rest: &str, literal: &str) -> Option<usize> {
    let spaces = space_len(rest);
    rest[spaces..]
        .starts_with(literal)
        .then_some(spaces + literal.len())
}

/// `call\s+` (INITIAL): the length of the word and the blanks after it.
fn call_len(rest: &str) -> Option<usize> {
    let spaces = rest.strip_prefix("call").map(space_len)?;
    (spaces > 0).then_some(4 + spaces)
}

/// `:{1}[^:\n;]+`: a colon and at least one character up to the next colon, line break or
/// semicolon.
fn label_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix(':')?;
    let len = body.find([':', '\n', ';']).unwrap_or(body.len());
    (len > 0).then_some(1 + len)
}

/// `\w+`: the run of ASCII word characters at the start of `rest`.
fn alpha_len(rest: &str) -> usize {
    rest.char_indices()
        .find(|&(_, c)| !is_word(c))
        .map_or(rest.len(), |(index, _)| index)
}

/// `[!"#$%&'*+,-.\`?\\/]`: the punctuation characters of the `PUNCTUATION` rule.
fn is_punctuation(c: char) -> bool {
    matches!(
        c,
        '!' | '"'
            | '#'
            | '$'
            | '%'
            | '&'
            | '\''
            | '*'
            | '+'
            | ','
            | '-'
            | '.'
            | '`'
            | '?'
            | '\\'
            | '/'
    )
}

/// The rules of the lexer for the current start condition and text, in the order of the grammar.
/// `rest` is the text from the current position. Returns `None` when no rule matches.
fn rule(state: State, rest: &str) -> Option<Hit<'_>> {
    let initial = state == State::Initial;
    if initial {
        for (word, kind) in [
            ("TB", Kind::DirectionTb),
            ("BT", Kind::DirectionBt),
            ("RL", Kind::DirectionRl),
            ("LR", Kind::DirectionLr),
        ] {
            if let Some(len) = direction_len(rest, word) {
                return token(len, kind, &rest[..len], Action::Keep);
            }
        }
        if rest.starts_with("%%") {
            return skip(comment_len(rest), Action::Keep);
        }
        if let Some(len) = delimited_form_len(rest, "accTitle", ':') {
            return token(
                len,
                Kind::AccTitle,
                &rest[..len],
                Action::Push(State::AccTitle),
            );
        }
        if let Some(len) = delimited_form_len(rest, "accDescr", ':') {
            return token(
                len,
                Kind::AccDescr,
                &rest[..len],
                Action::Push(State::AccDescr),
            );
        }
        if let Some(len) = delimited_form_len(rest, "accDescr", '{') {
            return skip(len, Action::Push(State::AccDescrMulti));
        }
    }
    match state {
        State::AccTitle => {
            let len = line_len(rest);
            return token(len, Kind::AccTitleValue, &rest[..len], Action::Pop);
        }
        State::AccDescr => {
            let len = line_len(rest);
            return token(len, Kind::AccDescrValue, &rest[..len], Action::Pop);
        }
        State::AccDescrMulti => {
            if rest.starts_with('}') {
                return skip(1, Action::Pop);
            }
            let len = rest.find('}').unwrap_or(rest.len());
            return token(len, Kind::AccDescrMultiValue, &rest[..len], Action::Keep);
        }
        State::CallbackName => {
            if let Some(after) = rest.strip_prefix('(') {
                let spaces = space_len(after);
                if after[spaces..].starts_with(')') {
                    return skip(2 + spaces, Action::Pop);
                }
                return skip(1, Action::Replace(State::CallbackArgs));
            }
            let len = rest.find('(').unwrap_or(rest.len());
            return token(len, Kind::CallbackName, &rest[..len], Action::Keep);
        }
        State::CallbackArgs => {
            if rest.starts_with(')') {
                return skip(1, Action::Pop);
            }
            let len = rest.find(')').unwrap_or(rest.len());
            return token(len, Kind::CallbackArgs, &rest[..len], Action::Keep);
        }
        State::String => {
            if rest.starts_with('"') {
                return skip(1, Action::Pop);
            }
            let len = rest.find('"').unwrap_or(rest.len());
            return token(len, Kind::Str, &rest[..len], Action::Keep);
        }
        State::Generic => {
            if rest.starts_with('~') {
                return skip(1, Action::Pop);
            }
            let len = rest.find('~').unwrap_or(rest.len());
            return token(len, Kind::GenericType, &rest[..len], Action::Keep);
        }
        State::BqString => {
            if rest.starts_with('`') {
                return skip(1, Action::Pop);
            }
            let len = rest.find('`').unwrap_or(rest.len());
            if len > 0 {
                return token(len, Kind::BqStr, &rest[..len], Action::Keep);
            }
        }
        _ => {}
    }

    // The rules of the other states, which come before the `<*>` rules where they apply.
    if initial {
        if let Some(len) = newline_len(rest) {
            return token(len, Kind::Newline, &rest[..len], Action::Keep);
        }
        let spaces = space_len(rest);
        if spaces > 0 {
            return skip(spaces, Action::Keep);
        }
        if let Some(len) =
            word_len(rest, "classDiagram-v2").or_else(|| word_len(rest, "classDiagram"))
        {
            return token(len, Kind::ClassDiagram, &rest[..len], Action::Keep);
        }
        if rest.starts_with("[*]") {
            return token(3, Kind::EdgeState, "[*]", Action::Keep);
        }
        if let Some(len) = call_len(rest) {
            return skip(len, Action::Push(State::CallbackName));
        }
    }
    if rest.starts_with('"') {
        return skip(1, Action::Push(State::String));
    }
    if initial {
        if let Some(len) = word_len(rest, "style") {
            return token(len, Kind::Style, &rest[..len], Action::Keep);
        }
        if let Some(len) = word_len(rest, "classDef") {
            return token(len, Kind::ClassDef, &rest[..len], Action::Keep);
        }
    }
    if matches!(
        state,
        State::Initial | State::Namespace | State::NamespaceBody
    ) && let Some(len) = word_len(rest, "namespace")
    {
        return token(
            len,
            Kind::Namespace,
            &rest[..len],
            Action::Push(State::Namespace),
        );
    }
    match state {
        State::Namespace => {
            if let Some(len) = newline_len(rest) {
                return token(len, Kind::Newline, &rest[..len], Action::Pop);
            }
            let spaces = space_len(rest);
            if spaces > 0 {
                return skip(spaces, Action::Keep);
            }
            if rest.starts_with('{') {
                return token(
                    1,
                    Kind::StructStart,
                    "{",
                    Action::Push(State::NamespaceBody),
                );
            }
            if rest.starts_with('}') {
                return skip(0, Action::PopRetry);
            }
        }
        State::NamespaceBody => {
            if rest.starts_with('}') {
                return token(1, Kind::StructStop, "}", Action::Pop);
            }
            if rest.is_empty() {
                return token(0, Kind::EofInStruct, "", Action::Keep);
            }
            if let Some(len) = newline_len(rest) {
                return token(len, Kind::Newline, &rest[..len], Action::Keep);
            }
            let spaces = space_len(rest);
            if spaces > 0 {
                return skip(spaces, Action::Keep);
            }
            if rest.starts_with("[*]") {
                return token(3, Kind::EdgeState, "[*]", Action::Keep);
            }
        }
        State::Class => {
            if let Some(len) = newline_len(rest) {
                return token(len, Kind::Newline, &rest[..len], Action::Pop);
            }
            let spaces = space_len(rest);
            if spaces > 0 {
                return skip(spaces, Action::Keep);
            }
            if rest.starts_with('}') {
                return token(1, Kind::StructStop, "}", Action::PopTwice);
            }
            if rest.starts_with('{') {
                return token(1, Kind::StructStart, "{", Action::Push(State::ClassBody));
            }
        }
        State::ClassBody => {
            if rest.starts_with('}') {
                return token(1, Kind::StructStop, "}", Action::Pop);
            }
            if rest.is_empty() {
                return token(0, Kind::EofInStruct, "", Action::Keep);
            }
            if rest.starts_with("[*]") {
                return token(3, Kind::EdgeState, "[*]", Action::Keep);
            }
            if rest.starts_with('{') {
                return token(1, Kind::OpenInStruct, "{", Action::Keep);
            }
            if rest.starts_with('\n') {
                return skip(1, Action::Keep);
            }
            let len = rest.find(['{', '}', '\n']).unwrap_or(rest.len());
            return token(len, Kind::Member, &rest[..len], Action::Keep);
        }
        _ => {}
    }
    if matches!(state, State::Initial | State::NamespaceBody)
        && let Some(len) = word_len(rest, "class")
    {
        return token(len, Kind::Class, &rest[..len], Action::Push(State::Class));
    }
    general(initial, rest)
}

/// The `<*>` rules, after the rules of the start conditions. `initial` says whether the rules
/// that only apply in INITIAL do apply.
fn general(initial: bool, rest: &str) -> Option<Hit<'_>> {
    for (word, kind) in [
        ("cssClass", Kind::CssClass),
        ("callback", Kind::Callback),
        ("link", Kind::Link),
        ("click", Kind::Click),
        ("note for", Kind::NoteFor),
        ("note", Kind::Note),
        ("href", Kind::Href),
    ] {
        if let Some(len) = word_len(rest, word) {
            return token(len, kind, &rest[..len], Action::Keep);
        }
    }
    if rest.starts_with("<<") {
        return token(2, Kind::AnnotationStart, "<<", Action::Keep);
    }
    if rest.starts_with(">>") {
        return token(2, Kind::AnnotationEnd, ">>", Action::Keep);
    }
    if rest.starts_with('~') {
        return skip(1, Action::Push(State::Generic));
    }
    if rest.starts_with('`') {
        return skip(1, Action::Push(State::BqString));
    }
    for word in ["_self", "_blank", "_parent", "_top"] {
        if let Some(len) = word_len(rest, word) {
            return token(len, Kind::LinkTarget, &rest[..len], Action::Keep);
        }
    }
    if let Some(len) = after_space(rest, "<|").or_else(|| after_space(rest, "|>")) {
        return token(len, Kind::Extension, &rest[..len], Action::Keep);
    }
    if let Some(len) = after_space(rest, ">").or_else(|| after_space(rest, "<")) {
        return token(len, Kind::Dependency, &rest[..len], Action::Keep);
    }
    if let Some(len) = after_space(rest, "*") {
        return token(len, Kind::Composition, &rest[..len], Action::Keep);
    }
    if let Some(len) = aggregation_len(rest) {
        return token(len, Kind::Aggregation, &rest[..len], Action::Keep);
    }
    if let Some(len) = after_space(rest, "()") {
        return token(len, Kind::Lollipop, &rest[..len], Action::Keep);
    }
    if rest.starts_with("--") {
        return token(2, Kind::Line, "--", Action::Keep);
    }
    if rest.starts_with("..") {
        return token(2, Kind::DottedLine, "..", Action::Keep);
    }
    if let Some(len) = label_len(rest) {
        return token(len, Kind::Label, &rest[..len], Action::Keep);
    }
    if rest.starts_with(":::") {
        return token(3, Kind::StyleSeparator, ":::", Action::Keep);
    }
    if rest.starts_with('-') {
        return token(1, Kind::Minus, "-", Action::Keep);
    }
    if rest.starts_with('.') {
        return token(1, Kind::Dot, ".", Action::Keep);
    }
    if rest.starts_with('+') {
        return token(1, Kind::Plus, "+", Action::Keep);
    }
    if initial {
        if rest.starts_with(':') {
            return token(1, Kind::Colon, ":", Action::Keep);
        }
        if rest.starts_with(',') {
            return token(1, Kind::Comma, ",", Action::Keep);
        }
        if rest.starts_with('#') {
            return token(1, Kind::Brkt, "#", Action::Keep);
        }
    }
    if rest.starts_with('%') {
        return token(1, Kind::Pct, "%", Action::Keep);
    }
    if rest.starts_with('=') {
        return token(1, Kind::Equals, "=", Action::Keep);
    }
    let alpha = alpha_len(rest);
    if alpha > 0 {
        return token(alpha, Kind::Alpha, &rest[..alpha], Action::Keep);
    }
    if rest.starts_with('[') {
        return token(1, Kind::Sqs, "[", Action::Keep);
    }
    if rest.starts_with(']') {
        return token(1, Kind::Sqe, "]", Action::Keep);
    }
    let first = rest.chars().next()?;
    if is_punctuation(first) {
        return token(
            first.len_utf8(),
            Kind::Punctuation,
            &rest[..first.len_utf8()],
            Action::Keep,
        );
    }
    if is_unicode_text(first) {
        return token(
            first.len_utf8(),
            Kind::UnicodeText,
            &rest[..first.len_utf8()],
            Action::Keep,
        );
    }
    if is_js_space(first) {
        return token(
            first.len_utf8(),
            Kind::Space,
            &rest[..first.len_utf8()],
            Action::Keep,
        );
    }
    None
}

/// `\s*o\b`: an aggregation marker, a standalone `o` after blanks.
fn aggregation_len(rest: &str) -> Option<usize> {
    let spaces = space_len(rest);
    word_len(&rest[spaces..], "o").map(|len| spaces + len)
}

/// The lexer state: the text, the position, the start-condition stack and whether the end of the
/// text has been reached (Jison's `done`).
struct Lexer<'a> {
    text: &'a str,
    pos: usize,
    states: StateStack<State>,
    done: bool,
}

impl Lexer<'_> {
    /// The next token, following Jison's `next()`: rules are tried on the empty remainder too, and
    /// after the end has been reported the lexer keeps giving `EOF`.
    fn next_token(&mut self) -> Result<Token, LexError> {
        loop {
            if self.pos >= self.text.len() {
                if self.done {
                    return Ok(Token {
                        kind: Kind::Eof,
                        text: String::new(),
                        start: self.text.len(),
                    });
                }
                self.done = true;
            }
            let start = self.pos;
            let rest = &self.text[start..];
            let Some(hit) = rule(self.states.top(), rest) else {
                if rest.is_empty() {
                    return Ok(Token {
                        kind: Kind::Eof,
                        text: String::new(),
                        start,
                    });
                }
                return Err(LexError { offset: start });
            };
            self.pos += hit.len;
            match hit.action {
                Action::Keep => {}
                Action::Push(state) => self.states.push(state),
                Action::Replace(state) => {
                    self.states.pop();
                    self.states.push(state);
                }
                Action::Pop | Action::PopRetry => self.states.pop(),
                Action::PopTwice => {
                    self.states.pop();
                    self.states.pop();
                }
            }
            if let Some((kind, value)) = hit.token {
                return Ok(Token {
                    kind,
                    text: value.to_owned(),
                    start,
                });
            }
        }
    }
}

/// Reads the whole text into tokens. The last token is `Eof`, or `EofInStruct` when the text ends
/// inside a namespace or a class body.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer {
        text,
        pos: 0,
        states: StateStack::new(State::Initial),
        done: false,
    };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token()?;
        let last = matches!(token.kind, Kind::Eof | Kind::EofInStruct);
        tokens.push(token);
        if last {
            return Ok(tokens);
        }
    }
}
