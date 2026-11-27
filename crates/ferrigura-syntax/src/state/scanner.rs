//! The state lexer: the rules of `parser/stateDiagram.jison`, with its start conditions, in the
//! grammar's order. The first rule that matches wins, as in Jison, and each rule is listed with
//! its line of the grammar. See
//! `docs/diagrams/state.md#parsing`.
//!
//! The lexer runs over the whole prepared text before the parser. Its state changes depend only on
//! the text, so the token sequence is the same as the reference's lazy one.

use crate::scan::{StateStack, line_len};
use crate::text::{is_js_space, is_line_term, is_word, js_trim};

/// The start conditions of the state lexer. `INITIAL` is the base; the others are pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    /// `INITIAL`: the statement level, the rules without a condition.
    Initial,
    /// `struct`: the body of a composite state.
    Struct,
    /// `STATE`: the rest of a `state` line.
    Line,
    /// `STATE_STRING`: the quoted description after `state`.
    Quoted,
    /// `STATE_ID`: the id after `as`.
    Alias,
    /// `SCALE`: the width of a `scale` line.
    Scale,
    /// `acc_title`: the value of `accTitle:`.
    AccTitle,
    /// `acc_descr`: the value of `accDescr:`.
    AccDescr,
    /// `acc_descr_multiline`: the body of `accDescr { … }`.
    AccDescrMultiline,
    /// `CLASSDEF`: the name of a `classDef` line.
    ClassDef,
    /// `CLASSDEFID`: the styles of a `classDef` line.
    ClassDefId,
    /// `CLASS`: the ids of a `class` line.
    Class,
    /// `CLASS_STYLE`: the classes of a `class` line.
    ClassStyle,
    /// `STYLE`: the ids of a `style` line.
    Style,
    /// `STYLEDEF_STYLES`: the styles of a `style` line.
    StyleDefStyles,
    /// `NOTE`: after `note`, before the position or the quote.
    Note,
    /// `NOTE_ID`: the id of a note with a position.
    NoteId,
    /// `NOTE_TEXT`: the text of a note with a position.
    NoteText,
    /// `FLOATING_NOTE`: the text of `note "…" as …`.
    FloatingNote,
    /// `FLOATING_NOTE_ID`: the id after `as` of a floating note.
    FloatingNoteId,
}

/// The token kinds of the state grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The end of the input.
    Eof,
    /// A line break (`NL`).
    Nl,
    /// `SD`: the `stateDiagram` keyword.
    Sd,
    /// `CLICK`.
    Click,
    /// `HREF`.
    Href,
    /// `STRING`: a double-quoted text, quotes included.
    Str,
    /// `DEFAULT`: the word `default` on its own, which the grammar does not accept.
    Default,
    /// `direction_tb`.
    DirTb,
    /// `direction_bt`.
    DirBt,
    /// `direction_rl`.
    DirRl,
    /// `direction_lr`.
    DirLr,
    /// `HIDE_EMPTY`.
    HideEmpty,
    /// `scale`, with its trailing blanks.
    Scale,
    /// `WIDTH`.
    Width,
    /// `acc_title`.
    AccTitle,
    /// `acc_title_value`.
    AccTitleValue,
    /// `acc_descr`.
    AccDescr,
    /// `acc_descr_value`.
    AccDescrValue,
    /// `acc_descr_multiline_value`.
    AccDescrMultiValue,
    /// `classDef`.
    ClassDef,
    /// `DEFAULT_CLASSDEF_ID`.
    DefaultClassDefId,
    /// `CLASSDEF_ID`.
    ClassDefIdName,
    /// `CLASSDEF_STYLEOPTS`.
    ClassDefStyleOpts,
    /// `class`.
    Class,
    /// `CLASSENTITY_IDS`.
    ClassEntityIds,
    /// `STYLECLASS`.
    StyleClass,
    /// `style`.
    Style,
    /// `STYLE_IDS`.
    StyleIds,
    /// `STYLEDEF_STYLEOPTS`.
    StyleDefStyleOpts,
    /// `FORK`.
    Fork,
    /// `JOIN`.
    Join,
    /// `CHOICE`.
    Choice,
    /// `COMPOSIT_STATE`.
    CompositState,
    /// `STATE_DESCR`.
    StateDescr,
    /// `AS`.
    As,
    /// `ID`.
    Id,
    /// `STRUCT_START`.
    StructStart,
    /// `STRUCT_STOP`.
    StructStop,
    /// `CONCURRENT`: a `--` line of a composite.
    Concurrent,
    /// `note`.
    Note,
    /// `left_of`.
    LeftOf,
    /// `right_of`.
    RightOf,
    /// `NOTE_TEXT`.
    NoteText,
    /// `EDGE_STATE`: `[*]`.
    EdgeState,
    /// `DESCR`: a description, colon included.
    Descr,
    /// `-->`.
    Arrow,
    /// `STYLE_SEPARATOR`: `:::`.
    StyleSeparator,
    /// `INVALID`: a character no rule takes. The grammar never accepts it.
    Invalid,
}

/// One token: its kind, its value as the lexer's `yytext` gives it after the rule's action, and its
/// byte offset in the prepared text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: Kind,
    pub value: String,
    pub start: usize,
}

/// Why the text could not be tokenized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanError {
    /// No rule matches at `offset`.
    Unrecognized { offset: usize },
    /// The rule `\w+\s+\w+.*?\{` in the `STATE` condition: a state name of more than one word.
    /// `text` is the matched text, which the reference's message quotes trimmed.
    TwoWordName { offset: usize, text: String },
}

/// What a matching rule does: how far it reads, the token it gives, and its change of state. The
/// pop comes before the push, as the rules' actions call them.
struct Hit {
    len: usize,
    token: Option<(Kind, String)>,
    pop: bool,
    push: Option<State>,
}

impl Hit {
    fn token(len: usize, kind: Kind, value: &str) -> Option<Self> {
        Some(Self {
            len,
            token: Some((kind, value.to_owned())),
            pop: false,
            push: None,
        })
    }

    fn skip(len: usize) -> Option<Self> {
        Some(Self {
            len,
            token: None,
            pop: false,
            push: None,
        })
    }

    fn with_state(mut self, pop: bool, push: Option<State>) -> Option<Self> {
        self.pop = pop;
        self.push = push;
        Some(self)
    }

    /// Enters `state`.
    fn enter(self, state: State) -> Option<Self> {
        self.with_state(false, Some(state))
    }

    /// Leaves the current state.
    fn leave(self) -> Option<Self> {
        self.with_state(true, None)
    }

    /// Leaves the current state and enters `state`.
    fn swap(self, state: State) -> Option<Self> {
        self.with_state(true, Some(state))
    }
}

/// `processId` of the reference: an ID that holds `%%` is cut there, and the rest goes back to the
/// input. A match that starts with `%%` is consumed with no token and no change of state.
fn id_hit(matched: &str, pop: bool, push: Option<State>) -> Option<Hit> {
    match matched.find("%%") {
        None => Hit {
            len: matched.len(),
            token: Some((Kind::Id, matched.to_owned())),
            pop: false,
            push: None,
        }
        .with_state(pop, push),
        Some(0) => Hit::skip(matched.len()),
        Some(index) => Hit {
            len: index,
            token: Some((Kind::Id, matched[..index].to_owned())),
            pop: false,
            push: None,
        }
        .with_state(pop, push),
    }
}

/// ASCII case-insensitive prefix test: the rules are written with the `i` flag, and every literal
/// they use is ASCII.
fn ci_prefix(rest: &str, literal: &str) -> bool {
    rest.as_bytes()
        .get(..literal.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(literal.as_bytes()))
}

/// Byte length of `literal` (case-insensitive) when it starts `rest` and no word character follows,
/// Jison's implicit `\b` after a literal that ends in a word character.
fn ci_word(rest: &str, literal: &str) -> Option<usize> {
    let after = rest.get(literal.len()..).unwrap_or("");
    (ci_prefix(rest, literal) && !after.chars().next().is_some_and(is_word))
        .then_some(literal.len())
}

/// Byte length of the run of characters from the start of `rest` that satisfy `keep`.
fn run_len(rest: &str, keep: impl Fn(char) -> bool) -> usize {
    rest.char_indices()
        .find(|&(_, c)| !keep(c))
        .map_or(rest.len(), |(index, _)| index)
}

/// Byte length of the run of JavaScript whitespace at the start of `rest`.
fn ws_len(rest: &str) -> usize {
    run_len(rest, is_js_space)
}

/// Byte length of the run of word characters at the start of `rest`.
fn word_len(rest: &str) -> usize {
    run_len(rest, is_word)
}

/// Byte length of `word` plus its trailing whitespace, when `rest` starts with the word and at
/// least one whitespace character (`word\s+`, case-insensitive).
fn word_then_space(rest: &str, word: &str) -> Option<usize> {
    if !ci_prefix(rest, word) {
        return None;
    }
    let spaces = ws_len(&rest[word.len()..]);
    (spaces > 0).then_some(word.len() + spaces)
}

/// Precomputed line boundaries, `direction\s+WORD` occurrences, state marker occurrences, and
/// opening brace offsets so per-token rules run in O(log N) instead of rescanning to end of line.
struct Index {
    len: usize,
    line_breaks: Vec<usize>,
    directions: [Vec<(usize, usize)>; 4],
    markers: [Vec<usize>; 6],
    open_braces: Vec<usize>,
}

impl Index {
    fn new(text: &str) -> Self {
        let mut line_breaks = Vec::new();
        let mut directions: [Vec<(usize, usize)>; 4] = Default::default();
        let mut markers: [Vec<usize>; 6] = Default::default();
        let mut open_braces = Vec::new();
        let bytes = text.as_bytes();
        for (index, c) in text.char_indices() {
            if is_line_term(c) {
                line_breaks.push(index);
            } else if c == '{' {
                open_braces.push(index);
            } else if matches!(c, 'd' | 'D')
                && bytes
                    .get(index..index + 9)
                    .is_some_and(|slice| slice.eq_ignore_ascii_case(b"direction"))
            {
                let after = index + 9;
                let spaces = ws_len(&text[after..]);
                if spaces > 0 {
                    let word_at = after + spaces;
                    for ((word, _), occ) in DIRECTIONS.iter().zip(directions.iter_mut()) {
                        if ci_prefix(&text[word_at..], word) {
                            occ.push((index, word_at + word.len()));
                        }
                    }
                }
            } else if matches!(c, '<' | '[') {
                for ((marker, _), occ) in MARKERS.iter().zip(markers.iter_mut()) {
                    if ci_prefix(&text[index..], marker) {
                        occ.push(index);
                    }
                }
            }
        }
        Self {
            len: text.len(),
            line_breaks,
            directions,
            markers,
            open_braces,
        }
    }

    fn next_line_term(&self, pos: usize) -> usize {
        self.line_breaks
            .get(self.line_breaks.partition_point(|&b| b < pos))
            .copied()
            .unwrap_or(self.len)
    }

    /// `.*direction\s+WORD[^\n]*` at byte offset `pos`.
    fn direction_len(&self, pos: usize, rest: &str, word_index: usize) -> Option<usize> {
        let occurrences = &self.directions[word_index];
        if occurrences.is_empty() {
            return None;
        }
        let line_end = self.next_line_term(pos);
        let upper = occurrences.partition_point(|&(start, _)| start < line_end);
        let &(start, end) = occurrences[..upper].last()?;
        if start < pos {
            return None;
        }
        let rel_end = end - pos;
        Some(rel_end + line_len(&rest[rel_end..]))
    }

    /// `.*MARKER` at byte offset `pos`.
    fn marker_len(&self, pos: usize, marker_index: usize) -> Option<usize> {
        let occurrences = &self.markers[marker_index];
        if occurrences.is_empty() {
            return None;
        }
        let line_end = self.next_line_term(pos);
        let upper = occurrences.partition_point(|&start| start < line_end);
        let &start = occurrences[..upper].last()?;
        if start < pos {
            return None;
        }
        Some((start - pos) + MARKERS[marker_index].0.len())
    }

    /// `\w+\s+\w+.*?\{` in the `STATE` condition at byte offset `pos`.
    fn two_word_name_len(&self, pos: usize, rest: &str) -> Option<usize> {
        if self.open_braces.is_empty() {
            return None;
        }
        let first = word_len(rest);
        if first == 0 {
            return None;
        }
        let spaces = ws_len(&rest[first..]);
        if spaces == 0 {
            return None;
        }
        let second_at = first + spaces;
        if word_len(&rest[second_at..]) == 0 {
            return None;
        }
        let abs_second = pos + second_at;
        let line_end = self.next_line_term(abs_second);
        let idx = self.open_braces.partition_point(|&b| b < abs_second);
        let &brace = self.open_braces.get(idx)?;
        (brace < line_end).then_some((brace - pos) + 1)
    }
}

/// `\s*":"(?:[^:\n;]|":"[^:\n;])+`: a description, from the colon on. A colon is taken only with
/// the character after it, which must not be a colon, a line break or a semicolon.
fn descr_len(rest: &str) -> Option<usize> {
    let spaces = ws_len(rest);
    let after = rest[spaces..].strip_prefix(':')?;
    let mut taken = 0;
    let mut chars = after.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if !matches!(c, ':' | '\n' | ';') {
            taken = index + c.len_utf8();
        } else if c == ':' {
            match chars.peek() {
                Some(&(next_index, next)) if !matches!(next, ':' | '\n' | ';') => {
                    taken = next_index + next.len_utf8();
                    chars.next();
                }
                _ => break,
            }
        } else {
            break;
        }
    }
    (taken > 0).then_some(spaces + 1 + taken)
}

/// `[\s\S]*?\n\s*"end note"`: the lazy match up to the first line break that is followed by blanks
/// and the words `end note`. Returns the length, which ends with `end note`.
fn end_note_len(rest: &str) -> Option<usize> {
    rest.char_indices()
        .filter(|&(_, c)| c == '\n')
        .find_map(|(index, _)| {
            let after = index + 1;
            let at_word = after + ws_len(&rest[after..]);
            ci_word(&rest[at_word..], "end note").map(|len| at_word + len)
        })
}

/// The lexer. It reads the prepared text from the start and yields the tokens of the grammar.
struct Lexer {
    pos: usize,
    states: StateStack<State>,
    done: bool,
    tokens: Vec<Token>,
    index: Index,
}

/// Wraps the result of a rule that matched.
fn found(hit: Option<Hit>) -> Result<Option<Hit>, ScanError> {
    Ok(hit)
}

/// Whether `state` is one of `states`: the start conditions of a rule.
fn when(state: State, states: &[State]) -> bool {
    states.contains(&state)
}

/// `accTitle\s*":"\s*` and its siblings: the keyword, blanks, a colon and blanks.
fn keyword_colon_len(rest: &str, keyword: &str) -> Option<usize> {
    if !ci_prefix(rest, keyword) {
        return None;
    }
    let after = &rest[keyword.len()..];
    let before_colon = ws_len(after);
    let colon = after[before_colon..].strip_prefix(':')?;
    Some(keyword.len() + before_colon + 1 + ws_len(colon))
}

/// `accDescr\s*"{"\s*`: the keyword, blanks, an opening brace and blanks.
fn keyword_brace_len(rest: &str, keyword: &str) -> Option<usize> {
    if !ci_prefix(rest, keyword) {
        return None;
    }
    let after = &rest[keyword.len()..];
    let before_brace = ws_len(after);
    let brace = after[before_brace..].strip_prefix('{')?;
    Some(keyword.len() + before_brace + 1 + ws_len(brace))
}

/// `(\w+)+((","\s*\w+)*)`: the word at the start (`word` bytes long), then any number of
/// `,` blanks word groups. A group is taken only when a word follows the comma and its blanks.
fn class_ids_len(rest: &str, word: usize) -> usize {
    let mut end = word;
    loop {
        let Some(after_comma) = rest[end..].strip_prefix(',') else {
            return end;
        };
        let spaces = ws_len(after_comma);
        let name = word_len(&after_comma[spaces..]);
        if name == 0 {
            return end;
        }
        end += 1 + spaces + name;
    }
}

impl Lexer {
    fn push(&mut self, kind: Kind, value: String, start: usize) {
        self.tokens.push(Token { kind, value, start });
    }

    /// The first rule of the state that matches the rest of the text, in the grammar's order. The
    /// comment on each arm gives the grammar's line.
    fn rule(&self, rest: &str, state: State) -> Result<Option<Hit>, ScanError> {
        use State::{
            AccDescr, AccDescrMultiline, AccTitle, Alias, Class, ClassDef, ClassDefId, ClassStyle,
            FloatingNote, FloatingNoteId, Initial, Line, Note, NoteId, NoteText, Quoted, Scale,
            Struct, Style, StyleDefStyles,
        };
        // 69-73: the keywords of INITIAL
        if when(state, &[Initial]) {
            if let Some(len) = ci_word(rest, "click") {
                return found(Hit::token(len, Kind::Click, &rest[..len]));
            }
            if let Some(len) = ci_word(rest, "href") {
                return found(Hit::token(len, Kind::Href, &rest[..len]));
            }
            // 71: \"[^"]*\"
            if rest.starts_with('"')
                && let Some(close) = rest[1..].find('"')
            {
                let len = close + 2;
                return found(Hit::token(len, Kind::Str, &rest[..len]));
            }
            if let Some(len) = ci_word(rest, "default") {
                return found(Hit::token(len, Kind::Default, &rest[..len]));
            }
            // 75-78: .*direction\s+TB|BT|RL|LR[^\n]*
            for (word_index, &(_, kind)) in DIRECTIONS.iter().enumerate() {
                if let Some(len) = self.index.direction_len(self.pos, rest, word_index) {
                    return found(Hit::token(len, kind, &rest[..len]));
                }
            }
            // 80: [\n]+
            if rest.starts_with('\n') {
                let len = run_len(rest, |c| c == '\n');
                return found(Hit::token(len, Kind::Nl, &rest[..len]));
            }
            // 81: [\s]+ (skipped)
            if rest.starts_with(is_js_space) {
                return found(Hit::skip(ws_len(rest)));
            }
        }
        // 82: ((?!\n)\s)+ (skipped, same-line blanks)
        if when(state, &[Line, Struct]) && rest.starts_with(|c| is_js_space(c) && c != '\n') {
            return found(Hit::skip(run_len(rest, |c| is_js_space(c) && c != '\n')));
        }
        // 83-84: #… and %%… (not %%{) are comments, skipped
        if when(state, &[Initial, Line, Struct]) {
            if rest.starts_with('#') {
                return found(Hit::skip(line_len(rest)));
            }
            if rest.starts_with("%%") && !rest[2..].starts_with('{') {
                return found(Hit::skip(line_len(rest)));
            }
        }
        // 85: "scale"\s+
        if when(state, &[Initial])
            && let Some(len) = word_then_space(rest, "scale")
        {
            return found(Hit::token(len, Kind::Scale, &rest[..len]).and_then(|h| h.enter(Scale)));
        }
        // 86: <SCALE>\d+
        if when(state, &[Scale]) {
            let digits = run_len(rest, |c| c.is_ascii_digit());
            if digits > 0 {
                return found(Hit::token(digits, Kind::Width, &rest[..digits]));
            }
            // 87: <SCALE>\s+"width"
            let spaces = ws_len(rest);
            if spaces > 0
                && let Some(len) = ci_word(&rest[spaces..], "width")
            {
                return found(Hit::skip(spaces + len).and_then(|h| h.leave()));
            }
        }
        // 89-90: accTitle\s*":"\s* and the rest of the line
        if when(state, &[Initial])
            && let Some(len) = keyword_colon_len(rest, "accTitle")
        {
            return found(
                Hit::token(len, Kind::AccTitle, &rest[..len]).and_then(|h| h.enter(AccTitle)),
            );
        }
        if when(state, &[AccTitle]) {
            let len = line_len(rest);
            return found(
                Hit::token(len, Kind::AccTitleValue, &rest[..len]).and_then(|h| h.leave()),
            );
        }
        // 91-92: accDescr\s*":"\s* and the rest of the line
        if when(state, &[Initial])
            && let Some(len) = keyword_colon_len(rest, "accDescr")
        {
            return found(
                Hit::token(len, Kind::AccDescr, &rest[..len]).and_then(|h| h.enter(AccDescr)),
            );
        }
        if when(state, &[AccDescr]) {
            let len = line_len(rest);
            return found(
                Hit::token(len, Kind::AccDescrValue, &rest[..len]).and_then(|h| h.leave()),
            );
        }
        // 93: accDescr\s*"{"\s* (no token)
        if when(state, &[Initial])
            && let Some(len) = keyword_brace_len(rest, "accDescr")
        {
            return found(Hit::skip(len).and_then(|h| h.enter(AccDescrMultiline)));
        }
        // 94-95: the body of accDescr { … }
        if when(state, &[AccDescrMultiline]) {
            if rest.starts_with('}') {
                return found(Hit::skip(1).and_then(|h| h.leave()));
            }
            let len = run_len(rest, |c| c != '}');
            return found(Hit::token(len, Kind::AccDescrMultiValue, &rest[..len]));
        }
        // 97: classDef\s+
        if when(state, &[Initial, Struct])
            && let Some(len) = word_then_space(rest, "classDef")
        {
            return found(
                Hit::token(len, Kind::ClassDef, &rest[..len]).and_then(|h| h.enter(ClassDef)),
            );
        }
        // 98-99: the name of a classDef line
        if when(state, &[ClassDef]) {
            if let Some(len) = word_then_space(rest, "DEFAULT") {
                return found(
                    Hit::token(len, Kind::DefaultClassDefId, &rest[..len])
                        .and_then(|h| h.swap(ClassDefId)),
                );
            }
            let word = word_len(rest);
            let spaces = ws_len(&rest[word..]);
            if word > 0 && spaces > 0 {
                let len = word + spaces;
                return found(
                    Hit::token(len, Kind::ClassDefIdName, &rest[..len])
                        .and_then(|h| h.swap(ClassDefId)),
                );
            }
        }
        // 100: the styles of a classDef line
        if when(state, &[ClassDefId]) {
            let len = line_len(rest);
            return found(
                Hit::token(len, Kind::ClassDefStyleOpts, &rest[..len]).and_then(|h| h.leave()),
            );
        }
        // 102: class\s+
        if when(state, &[Initial, Struct])
            && let Some(len) = word_then_space(rest, "class")
        {
            return found(Hit::token(len, Kind::Class, &rest[..len]).and_then(|h| h.enter(Class)));
        }
        // 103: the ids of a class line
        if when(state, &[Class]) {
            let word = word_len(rest);
            if word > 0 {
                let len = class_ids_len(rest, word);
                return found(
                    Hit::token(len, Kind::ClassEntityIds, &rest[..len])
                        .and_then(|h| h.swap(ClassStyle)),
                );
            }
        }
        // 104: the classes of a class line
        if when(state, &[ClassStyle]) {
            let len = line_len(rest);
            return found(Hit::token(len, Kind::StyleClass, &rest[..len]).and_then(|h| h.leave()));
        }
        // 106: style\s+
        if when(state, &[Initial, Struct])
            && let Some(len) = word_then_space(rest, "style")
        {
            return found(Hit::token(len, Kind::Style, &rest[..len]).and_then(|h| h.enter(Style)));
        }
        // 107: [\w,]+\s+ (the ids of a style line)
        if when(state, &[Style]) {
            let word = run_len(rest, |c| is_word(c) || c == ',');
            let spaces = ws_len(&rest[word..]);
            if word > 0 && spaces > 0 {
                let len = word + spaces;
                return found(
                    Hit::token(len, Kind::StyleIds, &rest[..len])
                        .and_then(|h| h.swap(StyleDefStyles)),
                );
            }
        }
        // 108: the styles of a style line
        if when(state, &[StyleDefStyles]) {
            let len = line_len(rest);
            return found(
                Hit::token(len, Kind::StyleDefStyleOpts, &rest[..len]).and_then(|h| h.leave()),
            );
        }
        // 114: state\s+ (no token)
        if when(state, &[Initial, Struct])
            && let Some(len) = word_then_space(rest, "state")
        {
            return found(Hit::skip(len).and_then(|h| h.enter(Line)));
        }
        // 116-121: .*MARKER, the text before the last marker on the line
        if when(state, &[Line]) {
            for (marker_index, &(marker, kind)) in MARKERS.iter().enumerate() {
                if let Some(len) = self.index.marker_len(self.pos, marker_index) {
                    let value = js_trim(&rest[..len - marker.len()]);
                    return found(Hit::token(len, kind, value).and_then(|h| h.leave()));
                }
            }
        }
        // 123-126: .*direction\s+WORD[^\n]* in a composite
        if when(state, &[Struct]) {
            for (word_index, &(_, kind)) in DIRECTIONS.iter().enumerate() {
                if let Some(len) = self.index.direction_len(self.pos, rest, word_index) {
                    return found(Hit::token(len, kind, &rest[..len]));
                }
            }
        }
        // 128: "
        if when(state, &[Line]) && rest.starts_with('"') {
            return found(Hit::skip(1).and_then(|h| h.enter(Quoted)));
        }
        // 129: \s*"as"\s+
        if when(state, &[Line]) {
            let spaces = ws_len(rest);
            if let Some(len) = word_then_space(&rest[spaces..], "as") {
                let len = spaces + len;
                return found(Hit::token(len, Kind::As, &rest[..len]).and_then(|h| h.enter(Alias)));
            }
        }
        // 130: [^\n\{]* (the id after as)
        if when(state, &[Alias]) {
            let len = run_len(rest, |c| c != '\n' && c != '{');
            return found(id_hit(&rest[..len], true, None));
        }
        // 131-132: the quoted description
        if when(state, &[Quoted]) {
            if rest.starts_with('"') {
                return found(Hit::skip(1).and_then(|h| h.leave()));
            }
            let len = run_len(rest, |c| c != '"');
            return found(Hit::token(len, Kind::StateDescr, &rest[..len]));
        }
        // 133: \w+\s+\w+.*?\{ is an error
        if when(state, &[Line])
            && let Some(len) = self.index.two_word_name_len(self.pos, rest)
        {
            return Err(ScanError::TwoWordName {
                offset: self.pos,
                text: rest[..len].to_owned(),
            });
        }
        // 134: [^\n\s\{]+
        if when(state, &[Line]) {
            let len = run_len(rest, |c| c != '\n' && !is_js_space(c) && c != '{');
            if len > 0 {
                return found(Hit::token(len, Kind::CompositState, &rest[..len]));
            }
        }
        // 135: \n leaves the state
        if when(state, &[Line]) && rest.starts_with('\n') {
            return found(Hit::skip(1).and_then(|h| h.leave()));
        }
        // 136: { opens a composite body
        if when(state, &[Initial, Line]) && rest.starts_with('{') {
            return found(Hit::token(1, Kind::StructStart, "{").and_then(|h| h.swap(Struct)));
        }
        // 137-138: } closes it; newlines inside it are skipped
        if when(state, &[Struct]) {
            if rest.starts_with('}') {
                return found(Hit::token(1, Kind::StructStop, "}").and_then(|h| h.leave()));
            }
            if rest.starts_with('\n') {
                return found(Hit::skip(1));
            }
        }
        // 140: note\s+
        if when(state, &[Initial, Struct])
            && let Some(len) = word_then_space(rest, "note")
        {
            return found(Hit::token(len, Kind::Note, &rest[..len]).and_then(|h| h.enter(Note)));
        }
        // 141-142: left of, right of
        if when(state, &[Note]) {
            if let Some(len) = ci_word(rest, "left of") {
                return found(
                    Hit::token(len, Kind::LeftOf, &rest[..len]).and_then(|h| h.swap(NoteId)),
                );
            }
            if let Some(len) = ci_word(rest, "right of") {
                return found(
                    Hit::token(len, Kind::RightOf, &rest[..len]).and_then(|h| h.swap(NoteId)),
                );
            }
        }
        // 143: a quote starts a floating note
        if when(state, &[Note]) && rest.starts_with('"') {
            return found(Hit::skip(1).and_then(|h| h.swap(FloatingNote)));
        }
        // 144: \s*"as"\s*
        if when(state, &[FloatingNote]) {
            let spaces = ws_len(rest);
            if ci_word(&rest[spaces..], "as").is_some() {
                let len = spaces + 2 + ws_len(&rest[spaces + 2..]);
                return found(
                    Hit::token(len, Kind::As, &rest[..len]).and_then(|h| h.swap(FloatingNoteId)),
                );
            }
        }
        // 145-146: the floating note's text
        if when(state, &[FloatingNote]) {
            if rest.starts_with('"') {
                return found(Hit::skip(1));
            }
            let len = run_len(rest, |c| c != '"');
            return found(Hit::token(len, Kind::NoteText, &rest[..len]));
        }
        // 147: [^\n]* (the id after as of a floating note)
        if when(state, &[FloatingNoteId]) {
            let len = line_len(rest);
            return found(id_hit(&rest[..len], true, None));
        }
        // 148: \s*[^:\n\s\-]+ (the id of a note with a position)
        if when(state, &[NoteId]) {
            let spaces = ws_len(rest);
            let run = run_len(&rest[spaces..], |c| {
                !matches!(c, ':' | '\n' | '-') && !is_js_space(c)
            });
            if run > 0 {
                let len = spaces + run;
                return found(id_hit(&rest[..len], true, Some(NoteText)));
            }
        }
        // 149: \s*":"[^:\n;]+ (the text of a note with a position, after a colon)
        if when(state, &[NoteText]) {
            let spaces = ws_len(rest);
            if rest[spaces..].starts_with(':') {
                let run = run_len(&rest[spaces + 1..], |c| !matches!(c, ':' | '\n' | ';'));
                if run > 0 {
                    let len = spaces + 1 + run;
                    // `yytext.substr(2).trim()`: the first two characters go, whatever they are.
                    let value: String = rest[..len].chars().skip(2).collect();
                    return found(
                        Hit::token(len, Kind::NoteText, js_trim(&value)).and_then(|h| h.leave()),
                    );
                }
            }
        }
        // 150: [\s\S]*?\n\s*"end note" (the text of a block note)
        if when(state, &[NoteText])
            && let Some(len) = end_note_len(rest)
        {
            let value = js_trim(&rest[..len - 8]);
            return found(Hit::token(len, Kind::NoteText, value).and_then(|h| h.leave()));
        }
        // 152-153: stateDiagram\s+ and stateDiagram-v2\s+
        if when(state, &[Initial]) {
            if let Some(len) = word_then_space(rest, "stateDiagram") {
                return found(Hit::token(len, Kind::Sd, &rest[..len]));
            }
            if let Some(len) = word_then_space(rest, "stateDiagram-v2") {
                return found(Hit::token(len, Kind::Sd, &rest[..len]));
            }
        }
        // 155: hide empty description
        if when(state, &[Initial])
            && let Some(len) = ci_word(rest, "hide empty description")
        {
            return found(Hit::token(len, Kind::HideEmpty, &rest[..len]));
        }
        // 157: [*]
        if when(state, &[Initial, Struct]) && rest.starts_with("[*]") {
            return found(Hit::token(3, Kind::EdgeState, "[*]"));
        }
        // 158: [^:\n\s\-\{]+ (an id; a %% inside is cut off, as processId does)
        if when(state, &[Initial, Struct]) {
            let len = run_len(rest, |c| {
                !matches!(c, ':' | '\n' | '-' | '{') && !is_js_space(c)
            });
            if len > 0 {
                return found(id_hit(&rest[..len], false, None));
            }
        }
        // 160: \s*":"(?:[^:\n;]|":"[^:\n;])+ (a description)
        if when(state, &[Initial, Struct])
            && let Some(len) = descr_len(rest)
        {
            return found(Hit::token(len, Kind::Descr, js_trim(&rest[..len])));
        }
        // 162-164: -->, --, :::
        if when(state, &[Initial, Struct]) && rest.starts_with("-->") {
            return found(Hit::token(3, Kind::Arrow, "-->"));
        }
        if when(state, &[Struct]) && rest.starts_with("--") {
            return found(Hit::token(2, Kind::Concurrent, "--"));
        }
        if when(state, &[Initial, Struct]) && rest.starts_with(":::") {
            return found(Hit::token(3, Kind::StyleSeparator, ":::"));
        }
        // 165: <<EOF>> is a line break, in INITIAL only
        if when(state, &[Initial]) && rest.is_empty() {
            return found(Hit::token(0, Kind::Nl, ""));
        }
        // 166: . (INITIAL only): a character no rule takes
        if when(state, &[Initial])
            && let Some(c) = rest.chars().next()
            && !is_line_term(c)
        {
            return found(Hit::token(
                c.len_utf8(),
                Kind::Invalid,
                &rest[..c.len_utf8()],
            ));
        }
        Ok(None)
    }
}

/// The direction words of the rules 75-78 and 123-126, in the grammar's order.
const DIRECTIONS: [(&str, Kind); 4] = [
    ("TB", Kind::DirTb),
    ("BT", Kind::DirBt),
    ("RL", Kind::DirRl),
    ("LR", Kind::DirLr),
];

/// The marker rules 116-121, in the grammar's order.
const MARKERS: [(&str, Kind); 6] = [
    ("<<fork>>", Kind::Fork),
    ("<<join>>", Kind::Join),
    ("<<choice>>", Kind::Choice),
    ("[[fork]]", Kind::Fork),
    ("[[join]]", Kind::Join),
    ("[[choice]]", Kind::Choice),
];

/// Tokenizes the prepared text. The last token is always [`Kind::Eof`].
///
/// This follows Jison's `next()`: the input is marked done when it is empty, the rules are tried
/// once more on the empty input (which can still give a zero-length token), and then EOF follows.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Token>, ScanError> {
    let mut lexer = Lexer {
        pos: 0,
        states: StateStack::new(State::Initial),
        done: false,
        tokens: Vec::new(),
        index: Index::new(text),
    };
    loop {
        if lexer.done {
            lexer.push(Kind::Eof, String::new(), text.len());
            break;
        }
        if lexer.pos == text.len() {
            lexer.done = true;
        }
        let start = lexer.pos;
        let state = lexer.states.top();
        let Some(hit) = lexer.rule(&text[start..], state)? else {
            if lexer.pos == text.len() {
                continue;
            }
            return Err(ScanError::Unrecognized { offset: start });
        };
        // A zero-length match that changes no state would repeat forever; the grammar's rules
        // never produce one away from the end, so this is a bug if it happens.
        if hit.len == 0 && !hit.pop && hit.push.is_none() && start < text.len() {
            return Err(ScanError::Unrecognized { offset: start });
        }
        lexer.pos += hit.len;
        if hit.pop {
            lexer.states.pop();
        }
        if let Some(state) = hit.push {
            lexer.states.push(state);
        }
        if let Some((kind, value)) = hit.token {
            lexer.push(kind, value, start);
        }
    }
    Ok(lexer.tokens)
}
