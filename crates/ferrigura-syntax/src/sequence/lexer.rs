//! The lexer of `sequenceDiagram.jison` (`packages/mermaid/src/diagrams/sequence/parser/`).
//!
//! Jison takes the first rule that matches at the position, not the longest, and a rule runs only
//! in its start state (`%x`, lines 17-21). The rules below are in the order of the grammar, and
//! the comments give its line numbers. A literal keyword gets Jison's implicit `\b` when it ends in
//! a word character. The lexer runs before the grammar, and its states depend on nothing the
//! grammar does, so the whole text is tokenised at once.

use ferrigura_model::sequence::LineType;

use crate::scan::StateStack;
use crate::text::{is_js_space, js_trim};

/// The start states of the lexer (`%x` in the grammar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Initial,
    Id,
    Alias,
    Line,
    Config,
    AccTitle,
    AccDescr,
    AccDescrMulti,
}

/// A token of the grammar. Keywords are named after the Jison token that the reference returns.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    /// `NEWLINE`, also `;` and the end of the text.
    Newline,
    /// `NUM`, the number as `Number` reads it.
    Num(f64),
    ConfigStart,
    ConfigContent(String),
    ConfigEnd,
    /// `ACTOR`, trimmed.
    Actor(String),
    /// `INVALID`: a piece of text no statement can use. The grammar accepts it and drops it.
    Invalid,
    KwBox,
    KwParticipant,
    KwActor,
    KwCreate,
    KwDestroy,
    KwAs,
    KwLoop,
    KwRect,
    KwOpt,
    KwAlt,
    KwElse,
    KwPar,
    KwParOver,
    KwAnd,
    KwCritical,
    KwOption,
    KwBreak,
    /// `restOfLine`: the rest of a line, with an optional `wrap:` prefix.
    RestOfLine(String),
    KwEnd,
    KwLeftOf,
    KwRightOf,
    KwLinks,
    /// `link`, with the text it was written with, which is the actor's name in `link ->> B`.
    KwLink(String),
    KwProperties,
    KwDetails,
    KwOver,
    KwNote,
    KwActivate,
    KwDeactivate,
    /// `title`: the value after the keyword and one blank (`substring(6)`).
    Title(String),
    /// `title:` (the legacy form): the value after the keyword and one blank (`substring(7)`).
    LegacyTitle(String),
    KwAccTitle,
    AccTitleValue(String),
    KwAccDescr,
    AccDescrValue(String),
    AccDescrMultiValue(String),
    KwSd,
    KwAutonumber,
    KwOff,
    Comma,
    /// Every arrow token, with the line type the grammar's `signaltype` gives it.
    Arrow(LineType),
    /// `TXT`: the text of a message, a note or a link, starting with its colon.
    Txt(String),
    Plus,
    Minus,
    /// `()`, the central connection marker.
    CentralConnection,
}

/// A token and the byte offset where it starts.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    pub(crate) tok: Tok,
    pub(crate) start: usize,
}

/// A character sequence no rule matches, at a byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexError {
    pub(crate) offset: usize,
    pub(crate) message: &'static str,
}

/// Tokenises the prepared text of a sequence diagram.
pub(crate) fn lex(text: &str) -> Result<Vec<Spanned>, LexError> {
    let mut lexer = Lexer {
        text,
        pos: 0,
        states: StateStack::new(State::Initial),
        out: Vec::new(),
    };
    while lexer.pos < text.len() {
        lexer.step()?;
    }
    // `<<EOF>>` (line 125) gives a NEWLINE in the initial state.
    if lexer.states.top() == State::Initial {
        lexer.out.push(Spanned {
            tok: Tok::Newline,
            start: text.len(),
        });
    }
    Ok(lexer.out)
}

/// The arrow tokens, in the order of the rules at lines 85-118. `\\` is one backslash in the
/// source text of the grammar's regular expressions.
const ARROWS: &[(&str, LineType)] = &[
    ("->>", LineType::Solid),
    ("<<->>", LineType::BidirectionalSolid),
    ("-->>", LineType::Dotted),
    ("<<-->>", LineType::BidirectionalDotted),
    ("->", LineType::SolidOpen),
    ("-->", LineType::DottedOpen),
    ("-x", LineType::SolidCross),
    ("--x", LineType::DottedCross),
    ("-)", LineType::SolidPoint),
    ("--)", LineType::DottedPoint),
    ("--|\\", LineType::SolidTopDotted),
    ("--|/", LineType::SolidBottomDotted),
    ("--\\\\", LineType::StickTopDotted),
    ("--//", LineType::StickBottomDotted),
    ("/|--", LineType::SolidArrowTopReverseDotted),
    ("\\|--", LineType::SolidArrowBottomReverseDotted),
    ("//--", LineType::StickArrowTopReverseDotted),
    ("\\\\--", LineType::StickArrowBottomReverseDotted),
    ("-|\\", LineType::SolidTop),
    ("-|/", LineType::SolidBottom),
    ("-\\\\", LineType::StickTop),
    ("-//", LineType::StickBottom),
    ("/|-", LineType::SolidArrowTopReverse),
    ("\\|-", LineType::SolidArrowBottomReverse),
    ("//-", LineType::StickArrowTopReverse),
    ("\\\\-", LineType::StickArrowBottomReverse),
];

/// The prefixes that end an actor name: the lookahead of the ACTOR rule (line 84).
const ARROW_PREFIXES: &[&str] = &[
    "-x", "--x", "-)", "--)", "-|\\", "-\\", "-/", "-//", "-|/", "/|-", "\\|-", "//-", "\\\\-",
    "--|\\", "--", "()",
];

struct Lexer<'a> {
    text: &'a str,
    pos: usize,
    states: StateStack<State>,
    out: Vec<Spanned>,
}

impl<'a> Lexer<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    fn fail(&self, message: &'static str) -> LexError {
        LexError {
            offset: self.pos,
            message,
        }
    }

    /// Emits `tok` for the `len` bytes at `start`, and moves past them.
    fn take(&mut self, start: usize, len: usize, tok: Tok) {
        self.pos = start + len;
        self.out.push(Spanned { tok, start });
    }

    /// A keyword that carries a state change, when it matches at the position.
    fn keyword(&mut self, start: usize, word: &str, tok: Tok, push: Option<State>) -> bool {
        let Some(len) = keyword_len(self.rest(), word) else {
            return false;
        };
        if let Some(state) = push {
            self.states.push(state);
        }
        self.take(start, len, tok);
        true
    }

    fn step(&mut self) -> Result<(), LexError> {
        match self.states.top() {
            State::Initial => self.initial(),
            State::Id => self.id(),
            State::Alias => {
                self.alias();
                Ok(())
            }
            State::Line => {
                self.line();
                Ok(())
            }
            State::Config => {
                self.config();
                Ok(())
            }
            State::AccTitle => {
                self.acc_value(Tok::AccTitleValue);
                Ok(())
            }
            State::AccDescr => {
                self.acc_value(Tok::AccDescrValue);
                Ok(())
            }
            State::AccDescrMulti => {
                self.acc_descr_multi();
                Ok(())
            }
        }
    }

    /// The initial state: the rules at lines 24-126 that have no state prefix.
    fn initial(&mut self) -> Result<(), LexError> {
        let start = self.pos;
        let rest = self.rest();
        let Some(first) = rest.chars().next() else {
            return Ok(());
        };

        // 24: `[\n]+` is one NEWLINE.
        if first == '\n' {
            let len = run_while(rest, |c| c == '\n');
            self.take(start, len, Tok::Newline);
            return Ok(());
        }
        // 25: `\s+` is skipped, line breaks included. A line break after blanks is lost (Q1).
        if is_js_space(first) {
            self.pos += js_space_len(rest);
            return Ok(());
        }
        // 27: `#` comment to the end of the line.
        if first == '#' {
            self.pos += line_len(rest);
            return Ok(());
        }
        // 28: `%%` comment, unless a `{` follows (a directive, removed before the lexer).
        if let Some(after) = rest.strip_prefix("%%")
            && !after.starts_with('{')
        {
            self.pos += 2 + line_len(after);
            return Ok(());
        }
        // 29: any character but `}` followed by `%%` starts a comment at that character.
        if first != '}' {
            let after_first = &rest[first.len_utf8()..];
            if let Some(after_comment) = after_first.strip_prefix("%%") {
                self.pos += first.len_utf8() + 2 + line_len(after_comment);
                return Ok(());
            }
        }
        // 30: `NUM`, with a lookahead of a blank or a line break.
        if let Some(len) = number_len(rest) {
            let value = rest[..len]
                .parse::<f64>()
                .map_err(|_| self.fail("malformed number"))?;
            self.take(start, len, Tok::Num(value));
            return Ok(());
        }

        // 40-69: the keywords.
        if self.keyword(start, "box", Tok::KwBox, Some(State::Line))
            || self.keyword(start, "participant", Tok::KwParticipant, Some(State::Id))
            || self.keyword(start, "actor", Tok::KwActor, Some(State::Id))
            || self.keyword(start, "create", Tok::KwCreate, None)
            || self.keyword(start, "destroy", Tok::KwDestroy, Some(State::Id))
            || self.keyword(start, "loop", Tok::KwLoop, Some(State::Line))
            || self.keyword(start, "rect", Tok::KwRect, Some(State::Line))
            || self.keyword(start, "opt", Tok::KwOpt, Some(State::Line))
            || self.keyword(start, "alt", Tok::KwAlt, Some(State::Line))
            || self.keyword(start, "else", Tok::KwElse, Some(State::Line))
            || self.keyword(start, "par", Tok::KwPar, Some(State::Line))
            || self.keyword(start, "par_over", Tok::KwParOver, Some(State::Line))
            || self.keyword(start, "and", Tok::KwAnd, Some(State::Line))
            || self.keyword(start, "critical", Tok::KwCritical, Some(State::Line))
            || self.keyword(start, "option", Tok::KwOption, Some(State::Line))
            || self.keyword(start, "break", Tok::KwBreak, Some(State::Line))
            || self.keyword(start, "end", Tok::KwEnd, None)
            || self.keyword(start, "left of", Tok::KwLeftOf, None)
            || self.keyword(start, "right of", Tok::KwRightOf, None)
        {
            return Ok(());
        }
        // 62-65: `links`, `link`, `properties` and `details` need a blank and an actor after them.
        for (word, tok) in [
            ("links", Tok::KwLinks),
            ("link", Tok::KwLink(String::new())),
            ("properties", Tok::KwProperties),
            ("details", Tok::KwDetails),
        ] {
            if let Some(len) =
                keyword_len(rest, word).filter(|&len| starts_actor_after_blank(&rest[len..]))
            {
                let tok = match tok {
                    Tok::KwLink(_) => Tok::KwLink(rest[..len].to_owned()),
                    other => other,
                };
                self.take(start, len, tok);
                return Ok(());
            }
        }
        if self.keyword(start, "over", Tok::KwOver, None)
            || self.keyword(start, "note", Tok::KwNote, None)
            || self.keyword(start, "activate", Tok::KwActivate, Some(State::Id))
            || self.keyword(start, "deactivate", Tok::KwDeactivate, Some(State::Id))
        {
            return Ok(());
        }
        // 70-71: `title` and `title:`, each with one blank (any `\s`) and a value.
        if let Some(len) = keyword_len(rest, "title")
            && let Some((consumed, value)) = blank_value(&rest[len..])
        {
            self.take(start, len + consumed, Tok::Title(value));
            return Ok(());
        }
        if let Some((consumed, value)) = rest
            .get(..6)
            .filter(|word| word.eq_ignore_ascii_case("title:"))
            .and_then(|_| blank_value(&rest[6..]))
        {
            self.take(start, 6 + consumed, Tok::LegacyTitle(value));
            return Ok(());
        }
        // 72-76: `accTitle:`, `accDescr:` and `accDescr {`, with the blanks around the delimiter.
        if let Some(len) = delimited_len(rest, "accTitle", ":") {
            self.states.push(State::AccTitle);
            self.take(start, len, Tok::KwAccTitle);
            return Ok(());
        }
        if let Some(len) = delimited_len(rest, "accDescr", ":") {
            self.states.push(State::AccDescr);
            self.take(start, len, Tok::KwAccDescr);
            return Ok(());
        }
        if let Some(len) = delimited_len(rest, "accDescr", "{") {
            // Line 76 has no action: the block's value follows, with no token of its own.
            self.states.push(State::AccDescrMulti);
            self.pos = start + len;
            return Ok(());
        }
        // 79-83: `sequenceDiagram`, `autonumber`, `off`, `,` and `;`.
        if self.keyword(start, "sequenceDiagram", Tok::KwSd, None)
            || self.keyword(start, "autonumber", Tok::KwAutonumber, None)
            || self.keyword(start, "off", Tok::KwOff, None)
        {
            return Ok(());
        }
        if first == ',' {
            self.take(start, 1, Tok::Comma);
            return Ok(());
        }
        if first == ';' {
            self.take(start, 1, Tok::Newline);
            return Ok(());
        }
        // 84: an actor name, which may hold blanks and characters that are not arrows.
        let actor = actor_run_len(rest);
        if actor > 0 {
            let name = js_trim(&rest[..actor]).to_owned();
            self.take(start, actor, Tok::Actor(name));
            return Ok(());
        }
        // 85-118: the arrows, in order.
        for (literal, line_type) in ARROWS {
            if rest.starts_with(literal) {
                self.take(start, literal.len(), Tok::Arrow(*line_type));
                return Ok(());
            }
        }
        // 120-121: `:` and the text after it, with an optional `wrap:` prefix.
        if first == ':' {
            let after = &rest[1..];
            let prefix = wrap_prefix_len(after, false).unwrap_or(0);
            let run = run_while(&after[prefix..], is_text_char);
            let len = 1 + prefix + run;
            self.take(start, len, Tok::Txt(rest[..len].to_owned()));
            return Ok(());
        }
        // 122-124: `+`, `-` and `()`.
        if first == '+' {
            self.take(start, 1, Tok::Plus);
            return Ok(());
        }
        if first == '-' {
            self.take(start, 1, Tok::Minus);
            return Ok(());
        }
        if rest.starts_with("()") {
            self.take(start, 2, Tok::CentralConnection);
            return Ok(());
        }
        // 126: `.` is INVALID; a line terminator is not matched by `.`.
        if matches!(first, '\r' | '\u{2028}' | '\u{2029}') {
            return Err(self.fail("unexpected character"));
        }
        self.take(start, first.len_utf8(), Tok::Invalid);
        Ok(())
    }

    /// The ID state (lines 26, 27, 31, 35-39): the name of a participant or of an activation.
    fn id(&mut self) -> Result<(), LexError> {
        let start = self.pos;
        let rest = self.rest();
        let Some(first) = rest.chars().next() else {
            return Ok(());
        };
        // 26: blanks on the same line.
        if is_js_space(first) && first != '\n' {
            self.pos += run_while(rest, |c| is_js_space(c) && c != '\n');
            return Ok(());
        }
        // 27: a comment.
        if first == '#' {
            self.pos += line_len(rest);
            return Ok(());
        }
        // 31: `@{` starts the metadata of the actor.
        if rest.starts_with("@{") {
            self.states.push(State::Config);
            self.take(start, 2, Tok::ConfigStart);
            return Ok(());
        }
        // 35-36: a name without blanks, before `@{` or before `as`.
        let word = run_while(rest, |c| is_name_char(c) && !is_js_space(c));
        if word > 0 && starts_config_open(&rest[word..]) {
            self.take(start, word, Tok::Actor(rest[..word].to_owned()));
            return Ok(());
        }
        if word > 0 && starts_as(&rest[word..]) {
            self.states.push(State::Alias);
            self.take(start, word, Tok::Actor(rest[..word].to_owned()));
            return Ok(());
        }
        // 37: a name with blanks, when a line break, `;` or `#` follows it (the longest such).
        let line_name = run_while(rest, is_line_name_char);
        if line_name > 0 {
            let ends: Vec<usize> = rest[..line_name]
                .char_indices()
                .map(|(index, c)| index + c.len_utf8())
                .collect();
            if let Some(&end) = ends.iter().rev().find(|&&end| ends_name(&rest[end..])) {
                self.states.pop();
                self.take(start, end, Tok::Actor(js_trim(&rest[..end]).to_owned()));
                return Ok(());
            }
        }
        // 38: text up to a `<`, which makes it INVALID.
        if rest[line_name..].starts_with('<') {
            let len = line_name + 1 + line_len(&rest[line_name + 1..]);
            self.states.pop();
            self.take(start, len, Tok::Invalid);
            return Ok(());
        }
        // 39: the rest of the line, INVALID.
        if first != '\n' {
            let len = line_len(rest);
            self.states.pop();
            self.take(start, len, Tok::Invalid);
            return Ok(());
        }
        Err(self.fail("unexpected line break"))
    }

    /// The ALIAS state (lines 26, 27, 45, 46): `as` after a name with a blank.
    fn alias(&mut self) {
        let start = self.pos;
        let rest = self.rest();
        if let Some(first) = rest.chars().next() {
            if is_js_space(first) && first != '\n' {
                self.pos += run_while(rest, |c| is_js_space(c) && c != '\n');
                return;
            }
            if first == '#' {
                self.pos += line_len(rest);
                return;
            }
        }
        if rest
            .get(..2)
            .is_some_and(|word| word.eq_ignore_ascii_case("as"))
        {
            // `popState(); popState(); begin('LINE')`: the alias and the ID state both end.
            self.states.pop();
            self.states.pop();
            self.states.push(State::Line);
            self.take(start, 2, Tok::KwAs);
            return;
        }
        // 46: an empty match ends the alias with a NEWLINE, and leaves the ID state too.
        self.states.pop();
        self.states.pop();
        self.out.push(Spanned {
            tok: Tok::Newline,
            start,
        });
    }

    /// The LINE state (lines 26, 27, 58): the rest of a line, with an optional `wrap:` prefix.
    fn line(&mut self) {
        let start = self.pos;
        let rest = self.rest();
        if let Some(first) = rest.chars().next() {
            if is_js_space(first) && first != '\n' {
                self.pos += run_while(rest, |c| is_js_space(c) && c != '\n');
                return;
            }
            if first == '#' {
                self.pos += line_len(rest);
                return;
            }
        }
        let prefix = wrap_prefix_len(rest, true).unwrap_or(0);
        let run = run_while(&rest[prefix..], is_text_char);
        let len = prefix + run;
        self.states.pop();
        self.take(start, len, Tok::RestOfLine(rest[..len].to_owned()));
    }

    /// The CONFIG state (lines 32-34): the metadata of an actor, up to its closing brace.
    fn config(&mut self) {
        let start = self.pos;
        let rest = self.rest();
        if !rest.starts_with('}') {
            let len = run_while(rest, |c| c != '}');
            self.take(start, len, Tok::ConfigContent(rest[..len].to_owned()));
            return;
        }
        if starts_as_after_blanks(&rest[1..]) {
            // `}` followed by `\s+as\s`: the alias state follows the ID state.
            self.states.pop();
            self.states.push(State::Alias);
        } else {
            self.states.pop();
            self.states.pop();
        }
        self.take(start, 1, Tok::ConfigEnd);
    }

    /// The accessibility value states (lines 73 and 75): the rest of the line, with no
    /// lookahead, after which the state ends.
    fn acc_value(&mut self, make: fn(String) -> Tok) {
        let start = self.pos;
        let rest = self.rest();
        let len = line_len(rest);
        self.states.pop();
        self.take(start, len, make(rest[..len].to_owned()));
    }

    /// The block state (lines 77 and 78): the text up to the closing brace, which ends the state.
    fn acc_descr_multi(&mut self) {
        let start = self.pos;
        let rest = self.rest();
        if rest.starts_with('}') {
            self.states.pop();
            self.pos += 1;
            return;
        }
        let len = run_while(rest, |c| c != '}');
        self.take(start, len, Tok::AccDescrMultiValue(rest[..len].to_owned()));
    }
}

/// `keyword` of the `\b` rule: `word`, ignoring ASCII case, and a word boundary after it when the
/// word ends in a word character. Returns the length of the match.
fn keyword_len(rest: &str, word: &str) -> Option<usize> {
    let len = word.len();
    let head = rest.get(..len)?;
    if !head.eq_ignore_ascii_case(word) {
        return None;
    }
    let ends_in_word = word.bytes().last().is_some_and(is_word_byte);
    if ends_in_word && rest.as_bytes().get(len).is_some_and(|&b| is_word_byte(b)) {
        return None;
    }
    Some(len)
}

/// `word`, ignoring ASCII case, followed by optional blanks, `delimiter`, and optional blanks:
/// `word\s*"delimiter"\s*`. The blanks may be line breaks.
fn delimited_len(rest: &str, word: &str, delimiter: &str) -> Option<usize> {
    let head = rest.get(..word.len())?;
    if !head.eq_ignore_ascii_case(word) {
        return None;
    }
    let after_word = &rest[word.len()..];
    let blanks = js_space_len(after_word);
    let after_blanks = &after_word[blanks..];
    let after_delimiter = after_blanks.strip_prefix(delimiter)?;
    Some(word.len() + blanks + delimiter.len() + js_space_len(after_delimiter))
}

/// The value after `title` (or after `title:`): one blank, which may be a line break, and then one
/// or more characters that are not `#`, a line break or `;`. Returns the bytes consumed and the
/// value (`substring` after the blank).
fn blank_value(after_keyword: &str) -> Option<(usize, String)> {
    let blank = after_keyword.chars().next()?;
    if !is_js_space(blank) {
        return None;
    }
    let run = run_while(&after_keyword[blank.len_utf8()..], is_text_char);
    if run == 0 {
        return None;
    }
    let end = blank.len_utf8() + run;
    Some((end, after_keyword[blank.len_utf8()..end].to_owned()))
}

/// `NUM`: `([0-9]+(\.[0-9]{1,2})?|\.[0-9]{1,2})(?=[ \n]+)` at the start of `rest`.
fn number_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    let end = if digits > 0 {
        let mut end = digits;
        if bytes.get(digits) == Some(&b'.') {
            let fraction = bytes[digits + 1..]
                .iter()
                .take(2)
                .take_while(|b| b.is_ascii_digit())
                .count();
            if fraction >= 1 {
                end = digits + 1 + fraction;
            }
        }
        end
    } else if bytes.first() == Some(&b'.') {
        let fraction = bytes[1..]
            .iter()
            .take(2)
            .take_while(|b| b.is_ascii_digit())
            .count();
        if fraction == 0 {
            return None;
        }
        1 + fraction
    } else {
        return None;
    };
    matches!(bytes.get(end), Some(b' ' | b'\n')).then_some(end)
}

/// The end of an actor name at the start of `rest`, rule 84: a first run of characters that are
/// not `/ \ + ( ) < - > : , ;` and line breaks, then groups of dashes and characters, each not
/// starting an arrow. Returns the length, or 0 when no name starts here.
fn actor_run_len(rest: &str) -> usize {
    let first = run_while(rest, |c| !is_name_start_excluded(c));
    if first == 0 {
        return 0;
    }
    let mut end = first;
    loop {
        let tail = &rest[end..];
        if ARROW_PREFIXES.iter().any(|prefix| tail.starts_with(prefix)) {
            break;
        }
        let dashes = tail.bytes().take_while(|&b| b == b'-').count();
        let body = run_while(&tail[dashes..], |c| !is_group_excluded(c));
        if body == 0 {
            break;
        }
        end += dashes + body;
    }
    end
}

/// `(?:[:]?(?:no)?wrap:)?` at the start of `text`, matched case-insensitively. With `colon`, a
/// leading colon is allowed. Returns the length of the prefix.
fn wrap_prefix_len(text: &str, colon: bool) -> Option<usize> {
    let skip = usize::from(colon && text.starts_with(':'));
    let rest = &text[skip..];
    if rest
        .get(..7)
        .is_some_and(|word| word.eq_ignore_ascii_case("nowrap:"))
    {
        return Some(skip + 7);
    }
    if rest
        .get(..5)
        .is_some_and(|word| word.eq_ignore_ascii_case("wrap:"))
    {
        return Some(skip + 5);
    }
    None
}

/// The `\s+as\s` lookahead of rule 36, after the name.
fn starts_as(after: &str) -> bool {
    let blanks = js_space_len(after);
    blanks >= 1 && starts_as_word(&after[blanks..])
}

/// `as\s` after the blanks of `after`, which has been matched by `\s+`.
fn starts_as_word(after: &str) -> bool {
    after
        .get(..2)
        .is_some_and(|word| word.eq_ignore_ascii_case("as"))
        && after[2..].chars().next().is_some_and(is_js_space)
}

/// The `\s+as\s` lookahead of rule 81 (after a metadata block, line 33).
fn starts_as_after_blanks(after: &str) -> bool {
    starts_as(after)
}

/// The `\s*@\{` lookahead of rule 35.
fn starts_config_open(after: &str) -> bool {
    after[js_space_len(after)..].starts_with("@{")
}

/// The `\s*[\n;#]|$` lookahead of rule 37, after a name that ends at `after`.
fn ends_name(after: &str) -> bool {
    let blanks = js_space_len(after);
    if after[..blanks].contains('\n') {
        return true;
    }
    matches!(
        after[blanks..].chars().next(),
        None | Some('\n' | ';' | '#')
    )
}

/// The `(?=[ \t]+[^\/\\\+\(\)<>:\n,;\s\-])` lookahead of the `links`, `link`, `properties` and
/// `details` keywords.
fn starts_actor_after_blank(after: &str) -> bool {
    let blanks = after
        .bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count();
    if blanks == 0 {
        return false;
    }
    match after[blanks..].chars().next() {
        Some(c) => !is_keyword_lookahead_excluded(c),
        None => false,
    }
}

/// The length of the leading run of JavaScript whitespace.
fn js_space_len(text: &str) -> usize {
    run_while(text, is_js_space)
}

/// The length up to, not including, the next line feed.
fn line_len(text: &str) -> usize {
    text.find('\n').unwrap_or(text.len())
}

/// The length of the leading run of characters that satisfy `keep`.
fn run_while(text: &str, keep: impl Fn(char) -> bool) -> usize {
    text.char_indices()
        .find(|&(_, c)| !keep(c))
        .map_or(text.len(), |(index, _)| index)
}

/// The text characters of a message, a note or a restOfLine: `[^#\n;]`.
fn is_text_char(c: char) -> bool {
    !matches!(c, '#' | '\n' | ';')
}

/// The characters of an unspaced participant name: `[^<>:\n,;@]`, plus blanks for rule 37.
fn is_name_char(c: char) -> bool {
    !matches!(c, '<' | '>' | ':' | '\n' | ',' | ';' | '@')
}

/// Rule 37's class, which may hold blanks.
fn is_line_name_char(c: char) -> bool {
    is_name_char(c)
}

/// The first character of an actor name (rule 84): `[^\/\\\+\()\+<\->\->:\n,;]`.
fn is_name_start_excluded(c: char) -> bool {
    matches!(
        c,
        '/' | '\\' | '+' | '(' | ')' | '<' | '-' | '>' | ':' | '\n' | ',' | ';'
    )
}

/// The characters after the first run of an actor name (rule 84): `[^\+<\->\->:\n,;]`.
fn is_group_excluded(c: char) -> bool {
    matches!(c, '+' | '<' | '-' | '>' | ':' | '\n' | ',' | ';')
}

/// The characters that end the `links`/`details` lookahead: `[\/\\\+\(\)<>:\n,;\s\-]`.
fn is_keyword_lookahead_excluded(c: char) -> bool {
    matches!(
        c,
        '/' | '\\' | '+' | '(' | ')' | '<' | '>' | ':' | '\n' | ',' | ';' | '-'
    ) || is_js_space(c)
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<Tok> {
        lex(text)
            .expect("lexes")
            .into_iter()
            .map(|s| s.tok)
            .collect()
    }

    #[test]
    fn a_line_break_after_blanks_is_skipped() {
        assert_eq!(
            kinds("sequenceDiagram  \nA->>B: x\n"),
            vec![
                Tok::KwSd,
                Tok::Actor("A".into()),
                Tok::Arrow(LineType::Solid),
                Tok::Actor("B".into()),
                Tok::Txt(": x".into()),
                Tok::Newline,
                Tok::Newline,
            ]
        );
    }

    #[test]
    fn keywords_take_a_word_boundary() {
        assert_eq!(kinds("offline")[0], Tok::Actor("offline".into()));
        assert_eq!(kinds("off ")[0], Tok::KwOff);
    }

    #[test]
    fn an_id_state_line_break_is_a_lexical_error() {
        assert!(lex("participant\n").is_err());
    }

    #[test]
    fn arrows_are_longest_first() {
        assert_eq!(kinds("-->>")[0], Tok::Arrow(LineType::Dotted));
        assert_eq!(kinds("--\\\\")[0], Tok::Arrow(LineType::StickTopDotted));
    }
}
