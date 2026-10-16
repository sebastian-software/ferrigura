//! The grammar of `sequenceDiagram.jison` (lines 130-423), as recursive descent over the tokens.
//!
//! The Jison actions build a list of items, and `yy.apply` runs them in order after the parse
//! (line 139). This parser builds the same list, flat: the nesting of the blocks is not needed
//! once `apply` flattens it. The parse-time actions of the reference (`setDiagramTitle`,
//! `setAccTitle`, `setAccDescription`, `parseMessage`, `parseBoxData`) run here too.

use ferrigura_model::sequence::{CentralConnection, LineType, Placement};

use super::lexer::{Spanned, Tok};
use crate::error::{ParseError, Source};
use crate::sanitize::sanitize_text;
use crate::text::{is_js_space, js_trim};

/// The text of a message, a note, a block or an actor link, as `parseMessage` gives it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MsgText {
    /// The text without its `wrap:` prefix, trimmed.
    pub(crate) text: String,
    /// `Some(true)` for `wrap:`, `Some(false)` for `nowrap:`, `None` when neither is given.
    pub(crate) wrap: Option<bool>,
}

/// The data of a `box`, as `parseBoxData` gives it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoxData {
    /// The sanitised title, `None` when the box has none.
    pub(crate) name: Option<String>,
    /// The colour, or `transparent`.
    pub(crate) color: String,
    /// `wrap` of the title, if given.
    pub(crate) wrap: Option<bool>,
}

/// Which link statement an [`Item::Actions`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActorAction {
    Links,
    ALink,
    Properties,
    Details,
}

/// One item of the list the grammar gives to `yy.apply`, in the reference's `type` names.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    /// `addParticipant`, or `createParticipant` when `create` is set.
    Participant {
        create: bool,
        actor: String,
        draw: Option<&'static str>,
        description: Option<MsgText>,
        config: Option<String>,
    },
    /// `destroyParticipant`.
    Destroy(String),
    /// `addMessage`: a message between two actors.
    Message {
        from: String,
        to: String,
        line_type: LineType,
        text: MsgText,
        activate: bool,
        central: CentralConnection,
    },
    /// `addSignal` without a receiver: an activation, a central connection, or a block marker.
    Signal {
        from: Option<String>,
        text: Option<MsgText>,
        line_type: LineType,
    },
    /// `sequenceIndex`: `autonumber`.
    Index {
        start: Option<f64>,
        step: Option<f64>,
        visible: bool,
    },
    /// `addNote`.
    Note {
        from: String,
        to: String,
        placement: Placement,
        text: MsgText,
    },
    /// `addLinks`, `addALink`, `addProperties` and `addDetails`.
    Actions {
        action: ActorAction,
        actor: String,
        text: MsgText,
    },
    /// `boxStart`.
    BoxStart(BoxData),
    /// `boxEnd`.
    BoxEnd,
}

/// What the grammar read: the items in order, and the header values the parse set.
#[derive(Debug, Default)]
pub(crate) struct Parsed {
    /// The items, each with the byte offset of its statement.
    pub(crate) items: Vec<(Item, usize)>,
    /// The last `title` of the body, as written (sanitised by the DB).
    pub(crate) title: Option<String>,
    /// The last `accTitle`, trimmed.
    pub(crate) acc_title: Option<String>,
    /// The last `accDescr`, trimmed.
    pub(crate) acc_descr: Option<String>,
}

/// Parses the tokens of a prepared text into the items of the DB.
pub(crate) fn parse(source: Source<'_>, tokens: Vec<Spanned>) -> Result<Parsed, ParseError> {
    let mut parser = Parser {
        source,
        tokens,
        at: 0,
        parsed: Parsed::default(),
    };
    parser.start()?;
    Ok(parser.parsed)
}

struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    at: usize,
    parsed: Parsed,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.at).map(|spanned| &spanned.tok)
    }

    fn peek_second(&self) -> Option<&Tok> {
        self.tokens.get(self.at + 1).map(|spanned| &spanned.tok)
    }

    /// The byte offset of the next token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.at)
            .map_or(self.source.text().len(), |spanned| spanned.start)
    }

    fn error(&self, message: &str) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn push(&mut self, offset: usize, item: Item) {
        self.parsed.items.push((item, offset));
    }

    /// Consumes the next token when it is `expected`.
    fn eat(&mut self, expected: &Tok) -> bool {
        if self.peek() == Some(expected) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    /// `NEWLINE`: the end of a statement.
    fn newline(&mut self) -> Result<(), ParseError> {
        if self.eat(&Tok::Newline) {
            Ok(())
        } else {
            Err(self.error("expected the end of the statement"))
        }
    }

    /// `ACTOR`, or the `link` keyword used as an actor name (the grammar's `actor`).
    fn actor_name(&mut self) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Tok::Actor(name)) | Some(Tok::KwLink(name)) => {
                self.at += 1;
                Ok(name)
            }
            _ => Err(self.error("expected a participant name")),
        }
    }

    /// `TXT` as `text2`: the message text, without its colon.
    fn text2(&mut self) -> Result<MsgText, ParseError> {
        match self.peek().cloned() {
            Some(Tok::Txt(text)) => {
                self.at += 1;
                Ok(parse_message(&js_trim(&text)[1..]))
            }
            _ => Err(self.error("expected ':' and the text of the message")),
        }
    }

    /// `restOfLine`, read with `parseMessage`.
    fn rest_of_line(&mut self) -> Result<MsgText, ParseError> {
        Ok(parse_message(&self.rest_of_line_raw()?))
    }

    /// `restOfLine` as written.
    fn rest_of_line_raw(&mut self) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Tok::RestOfLine(text)) => {
                self.at += 1;
                Ok(text)
            }
            _ => Err(self.error("expected the rest of the line")),
        }
    }

    fn arrow(&mut self) -> Result<LineType, ParseError> {
        match self.peek().cloned() {
            Some(Tok::Arrow(line_type)) => {
                self.at += 1;
                Ok(line_type)
            }
            _ => Err(self.error("expected an arrow")),
        }
    }

    /// `start: SD document`, with blank lines before the keyword.
    fn start(&mut self) -> Result<(), ParseError> {
        while self.eat(&Tok::Newline) {}
        if !self.eat(&Tok::KwSd) {
            return Err(self.error("expected 'sequenceDiagram'"));
        }
        self.document()?;
        if self.peek().is_some() {
            return Err(self.error("expected the end of the diagram"));
        }
        Ok(())
    }

    /// `document`: lines until a token that closes a block, or the end of the text.
    fn document(&mut self) -> Result<(), ParseError> {
        while let Some(tok) = self.peek() {
            if matches!(tok, Tok::KwEnd | Tok::KwElse | Tok::KwAnd | Tok::KwOption) {
                break;
            }
            match tok {
                Tok::Newline | Tok::Invalid => self.at += 1,
                _ => self.statement()?,
            }
        }
        Ok(())
    }

    fn expect_end(&mut self) -> Result<(), ParseError> {
        if self.eat(&Tok::KwEnd) {
            Ok(())
        } else {
            Err(self.error("expected 'end'"))
        }
    }

    /// `statement`: one statement of a document.
    fn statement(&mut self) -> Result<(), ParseError> {
        let start = self.offset();
        let Some(tok) = self.peek().cloned() else {
            return Err(self.error("expected a statement"));
        };
        match tok {
            Tok::Actor(_) => self.signal(start),
            Tok::KwLink(_) if self.link_starts_signal() => self.signal(start),
            Tok::KwLink(_) => self.actions(start, ActorAction::ALink),
            Tok::KwLinks => self.actions(start, ActorAction::Links),
            Tok::KwProperties => self.actions(start, ActorAction::Properties),
            Tok::KwDetails => self.actions(start, ActorAction::Details),
            Tok::KwBox => self.box_statement(start),
            Tok::KwParticipant | Tok::KwActor | Tok::KwDestroy => self.participant(start, false),
            Tok::KwCreate => {
                self.at += 1;
                if matches!(
                    self.peek(),
                    Some(Tok::KwParticipant | Tok::KwActor | Tok::KwDestroy)
                ) {
                    self.participant(start, true)
                } else {
                    Err(self.error("expected 'participant' or 'actor' after 'create'"))
                }
            }
            Tok::KwAutonumber => self.autonumber(start),
            Tok::KwActivate => self.activation(start, LineType::ActiveStart),
            Tok::KwDeactivate => self.activation(start, LineType::ActiveEnd),
            Tok::KwNote => self.note(start),
            Tok::Title(text) | Tok::LegacyTitle(text) => {
                self.at += 1;
                self.parsed.title = Some(text);
                Ok(())
            }
            Tok::KwAccTitle => {
                self.at += 1;
                let value = self.acc_value(|tok| match tok {
                    Tok::AccTitleValue(value) => Some(value),
                    _ => None,
                })?;
                self.parsed.acc_title = Some(js_trim(&value).to_owned());
                Ok(())
            }
            Tok::KwAccDescr => {
                self.at += 1;
                let value = self.acc_value(|tok| match tok {
                    Tok::AccDescrValue(value) => Some(value),
                    _ => None,
                })?;
                self.parsed.acc_descr = Some(js_trim(&value).to_owned());
                Ok(())
            }
            Tok::AccDescrMultiValue(value) => {
                self.at += 1;
                self.parsed.acc_descr = Some(js_trim(&value).to_owned());
                Ok(())
            }
            Tok::KwLoop
            | Tok::KwRect
            | Tok::KwOpt
            | Tok::KwAlt
            | Tok::KwPar
            | Tok::KwParOver
            | Tok::KwCritical
            | Tok::KwBreak => self.block(start, &tok),
            _ => Err(self.error("expected a statement")),
        }
    }

    /// The value token after an accessibility keyword.
    fn acc_value(&mut self, pick: impl Fn(Tok) -> Option<String>) -> Result<String, ParseError> {
        match self.peek().cloned().and_then(pick) {
            Some(value) => {
                self.at += 1;
                Ok(value)
            }
            None => Err(self.error("expected the value of the accessibility statement")),
        }
    }

    /// Whether a `link` keyword starts a signal, which is the case when an arrow or `()` follows.
    fn link_starts_signal(&self) -> bool {
        matches!(
            self.peek_second(),
            Some(Tok::Arrow(_) | Tok::CentralConnection)
        )
    }

    /// `actor signaltype actor text2`, and the forms with `+`, `-` and `()` (lines 331-356).
    fn signal(&mut self, start: usize) -> Result<(), ParseError> {
        let from = self.actor_name()?;
        if self.eat(&Tok::CentralConnection) {
            // `actor '()' signaltype actor` and `actor '()' signaltype '()' actor`.
            let line_type = self.arrow()?;
            if self.eat(&Tok::CentralConnection) {
                let to = self.actor_name()?;
                let text = self.text2()?;
                self.newline()?;
                self.push_participant(start, &from);
                self.push_participant(start, &to);
                self.push(
                    start,
                    Item::Message {
                        from: from.clone(),
                        to: to.clone(),
                        line_type,
                        text,
                        activate: true,
                        central: CentralConnection::Dual,
                    },
                );
                self.push_signal(start, Some(to), LineType::CentralConnection);
                self.push_signal(start, Some(from), LineType::CentralConnectionReverse);
            } else {
                let to = self.actor_name()?;
                let text = self.text2()?;
                self.newline()?;
                self.push_participant(start, &from);
                self.push_participant(start, &to);
                self.push(
                    start,
                    Item::Message {
                        from: from.clone(),
                        to,
                        line_type,
                        text,
                        activate: false,
                        central: CentralConnection::Reverse,
                    },
                );
                self.push_signal(start, Some(from), LineType::CentralConnectionReverse);
            }
            return Ok(());
        }
        let line_type = self.arrow()?;
        let (activate, central, extra) = match self.peek() {
            Some(Tok::Plus) => {
                self.at += 1;
                (
                    true,
                    CentralConnection::NoConnection,
                    Some(LineType::ActiveStart),
                )
            }
            Some(Tok::Minus) => {
                self.at += 1;
                (
                    false,
                    CentralConnection::NoConnection,
                    Some(LineType::ActiveEnd),
                )
            }
            Some(Tok::CentralConnection) => {
                self.at += 1;
                (
                    true,
                    CentralConnection::Central,
                    Some(LineType::CentralConnection),
                )
            }
            _ => (false, CentralConnection::NoConnection, None),
        };
        let to = self.actor_name()?;
        let text = self.text2()?;
        self.newline()?;
        self.push_participant(start, &from);
        self.push_participant(start, &to);
        self.push(
            start,
            Item::Message {
                from: from.clone(),
                to: to.clone(),
                line_type,
                text,
                activate,
                central,
            },
        );
        match extra {
            // `+` activates the receiver, `-` deactivates the sender, `()` connects the receiver.
            Some(LineType::ActiveStart) | Some(LineType::CentralConnection) => {
                self.push_signal(start, Some(to), extra.unwrap_or(LineType::ActiveStart));
            }
            Some(LineType::ActiveEnd) => self.push_signal(start, Some(from), LineType::ActiveEnd),
            _ => {}
        }
        Ok(())
    }

    fn push_participant(&mut self, offset: usize, actor: &str) {
        self.push(
            offset,
            Item::Participant {
                create: false,
                actor: actor.to_owned(),
                draw: None,
                description: None,
                config: None,
            },
        );
    }

    fn push_signal(&mut self, offset: usize, from: Option<String>, line_type: LineType) {
        self.push(
            offset,
            Item::Signal {
                from,
                text: None,
                line_type,
            },
        );
    }

    /// `links`, `link`, `properties` and `details` (lines 289-315): an actor, then its text.
    fn actions(&mut self, start: usize, action: ActorAction) -> Result<(), ParseError> {
        self.at += 1;
        let actor = self.actor_name()?;
        let text = self.text2()?;
        self.newline()?;
        self.push_participant(start, &actor);
        self.push(
            start,
            Item::Actions {
                action,
                actor,
                text,
            },
        );
        Ok(())
    }

    /// `activate` and `deactivate` (lines 179-180): no participant is declared.
    fn activation(&mut self, start: usize, line_type: LineType) -> Result<(), ParseError> {
        self.at += 1;
        let actor = self.actor_name()?;
        self.newline()?;
        self.push_signal(start, Some(actor), line_type);
        Ok(())
    }

    /// `autonumber`, with its forms (lines 175-178).
    fn autonumber(&mut self, start: usize) -> Result<(), ParseError> {
        self.at += 1;
        let item = match self.peek().cloned() {
            Some(Tok::Num(first)) => {
                self.at += 1;
                match self.peek().cloned() {
                    Some(Tok::Num(second)) => {
                        self.at += 1;
                        Item::Index {
                            start: Some(first),
                            step: Some(second),
                            visible: true,
                        }
                    }
                    _ => Item::Index {
                        start: Some(first),
                        step: Some(1.0),
                        visible: true,
                    },
                }
            }
            Some(Tok::KwOff) => {
                self.at += 1;
                Item::Index {
                    start: None,
                    step: None,
                    visible: false,
                }
            }
            _ => Item::Index {
                start: None,
                step: None,
                visible: true,
            },
        };
        self.newline()?;
        self.push(start, item);
        Ok(())
    }

    /// `note` with a side or `over` (lines 276-287).
    fn note(&mut self, start: usize) -> Result<(), ParseError> {
        self.at += 1;
        match self.peek().cloned() {
            Some(Tok::KwLeftOf | Tok::KwRightOf) => {
                let placement = if self.eat(&Tok::KwLeftOf) {
                    Placement::LeftOf
                } else {
                    self.at += 1;
                    Placement::RightOf
                };
                let actor = self.actor_name()?;
                let text = self.text2()?;
                self.newline()?;
                self.push_participant(start, &actor);
                self.push(
                    start,
                    Item::Note {
                        from: actor.clone(),
                        to: actor,
                        placement,
                        text,
                    },
                );
                Ok(())
            }
            Some(Tok::KwOver) => {
                self.at += 1;
                let first = self.actor_name()?;
                let second = if self.eat(&Tok::Comma) {
                    Some(self.actor_name()?)
                } else {
                    None
                };
                let text = self.text2()?;
                self.newline()?;
                self.push_participant(start, &first);
                if let Some(second) = &second {
                    self.push_participant(start, second);
                }
                // A single actor is both ends of the note (`[].concat(actor, actor).slice(0, 2)`).
                let to = second.clone().unwrap_or_else(|| first.clone());
                self.push(
                    start,
                    Item::Note {
                        from: first,
                        to,
                        placement: Placement::Over,
                        text,
                    },
                );
                Ok(())
            }
            _ => Err(self.error("expected 'left of', 'right of' or 'over' after 'note'")),
        }
    }

    /// `box restOfLine box_section end` (lines 169-173), with participants and blank lines inside.
    fn box_statement(&mut self, start: usize) -> Result<(), ParseError> {
        self.at += 1;
        let raw = self.rest_of_line_raw()?;
        self.push(start, Item::BoxStart(parse_box_data(&raw)));
        loop {
            match self.peek() {
                Some(Tok::Newline) => self.at += 1,
                Some(Tok::KwParticipant | Tok::KwActor | Tok::KwDestroy) => {
                    let offset = self.offset();
                    self.participant(offset, false)?;
                }
                Some(Tok::KwEnd) => break,
                _ => return Err(self.error("expected a participant or 'end' in a box")),
            }
        }
        self.at += 1;
        self.push(start, Item::BoxEnd);
        Ok(())
    }

    /// `participant_statement` (lines 263-273), with `create` (line 168) when `create` is set.
    fn participant(&mut self, start: usize, create: bool) -> Result<(), ParseError> {
        let keyword = self.peek().cloned();
        self.at += 1;
        if keyword == Some(Tok::KwDestroy) {
            let actor = self.actor_name()?;
            self.newline()?;
            let item = if create {
                Item::Participant {
                    create: true,
                    actor,
                    draw: None,
                    description: None,
                    config: None,
                }
            } else {
                Item::Destroy(actor)
            };
            self.push(start, item);
            return Ok(());
        }
        let draw = if keyword == Some(Tok::KwActor) {
            "actor"
        } else {
            "participant"
        };
        let actor = self.actor_name()?;
        let config = if self.eat(&Tok::ConfigStart) {
            let content = match self.peek().cloned() {
                Some(Tok::ConfigContent(content)) => {
                    self.at += 1;
                    content
                }
                _ => return Err(self.error("expected the metadata of the participant")),
            };
            if !self.eat(&Tok::ConfigEnd) {
                return Err(self.error("expected '}' after the metadata"));
            }
            Some(js_trim(&content).to_owned())
        } else {
            None
        };
        let description = if self.eat(&Tok::KwAs) {
            Some(self.rest_of_line()?)
        } else {
            None
        };
        self.newline()?;
        self.push(
            start,
            Item::Participant {
                create,
                actor,
                draw: Some(draw),
                description,
                config,
            },
        );
        Ok(())
    }

    /// `loop`, `rect`, `opt`, `alt`, `par`, `par_over`, `critical` and `break` (lines 191-242).
    fn block(&mut self, start: usize, keyword: &Tok) -> Result<(), ParseError> {
        self.at += 1;
        let text = self.rest_of_line()?;
        match keyword {
            Tok::KwLoop => {
                self.push_control(start, LineType::LoopStart, Some(text));
                self.document()?;
                self.expect_end()?;
                self.push_control(start, LineType::LoopEnd, None);
            }
            Tok::KwRect => {
                self.push_control(start, LineType::RectStart, Some(text));
                self.document()?;
                self.expect_end()?;
                self.push_control(start, LineType::RectEnd, None);
            }
            Tok::KwOpt => {
                self.push_control(start, LineType::OptStart, Some(text));
                self.document()?;
                self.expect_end()?;
                self.push_control(start, LineType::OptEnd, None);
            }
            Tok::KwBreak => {
                self.push_control(start, LineType::BreakStart, Some(text));
                self.document()?;
                self.expect_end()?;
                // `breakEnd` keeps no text in the DB (quirk Q7).
                self.push_control(start, LineType::BreakEnd, None);
            }
            Tok::KwAlt => {
                self.push_control(start, LineType::AltStart, Some(text));
                self.document()?;
                while self.eat(&Tok::KwElse) {
                    let text = self.rest_of_line()?;
                    self.push_control(start, LineType::AltElse, Some(text));
                    self.document()?;
                }
                self.expect_end()?;
                self.push_control(start, LineType::AltEnd, None);
            }
            Tok::KwPar | Tok::KwParOver => {
                let line_type = if keyword == &Tok::KwParOver {
                    LineType::ParOverStart
                } else {
                    LineType::ParStart
                };
                self.push_control(start, line_type, Some(text));
                self.document()?;
                while self.eat(&Tok::KwAnd) {
                    let text = self.rest_of_line()?;
                    self.push_control(start, LineType::ParAnd, Some(text));
                    self.document()?;
                }
                self.expect_end()?;
                self.push_control(start, LineType::ParEnd, None);
            }
            _ => {
                // `critical` (the only keyword left).
                self.push_control(start, LineType::CriticalStart, Some(text));
                self.document()?;
                while self.eat(&Tok::KwOption) {
                    let text = self.rest_of_line()?;
                    self.push_control(start, LineType::CriticalOption, Some(text));
                    self.document()?;
                }
                self.expect_end()?;
                self.push_control(start, LineType::CriticalEnd, None);
            }
        }
        Ok(())
    }

    fn push_control(&mut self, offset: usize, line_type: LineType, text: Option<MsgText>) {
        self.push(
            offset,
            Item::Signal {
                from: None,
                text,
                line_type,
            },
        );
    }
}

/// `parseMessage` (`sequenceDb.ts:380-389`): the text with its `wrap:` prefix read off.
pub(crate) fn parse_message(text: &str) -> MsgText {
    let (text, wrap) = extract_wrap(js_trim(text));
    MsgText { text, wrap }
}

/// `extractWrap` (`sequenceDb.ts:355-364`): the trimmed text without a `wrap:` or `nowrap:`
/// prefix, and the flag the prefix gives. The prefixes are case-sensitive, as in the reference.
fn extract_wrap(text: &str) -> (String, Option<bool>) {
    let text = js_trim(text);
    let (flag, prefix_len) = if let Some(len) = wrap_prefix(text, "wrap:") {
        (Some(true), len)
    } else if let Some(len) = wrap_prefix(text, "nowrap:") {
        (Some(false), len)
    } else {
        return (text.to_owned(), None);
    };
    (js_trim(&text[prefix_len..]).to_owned(), flag)
}

/// The length of `word` at the start of `text`, with an optional leading colon.
fn wrap_prefix(text: &str, word: &str) -> Option<usize> {
    let skip = usize::from(text.starts_with(':'));
    text[skip..].starts_with(word).then_some(skip + word.len())
}

/// `parseBoxData` (`sequenceDb.ts:394-419`): the colour is the first word or a function such as
/// `rgb(…)`, and the rest of the line is the title, unless the word is not a colour.
pub(crate) fn parse_box_data(text: &str) -> BoxData {
    let (first, rest) = split_box_colour(text);
    let (mut color, mut title) = (
        if first.is_empty() {
            "transparent".to_owned()
        } else {
            js_trim(first).to_owned()
        },
        (!rest.is_empty()).then(|| js_trim(rest).to_owned()),
    );
    if !is_css_colour(&color) {
        color = "transparent".to_owned();
        title = Some(js_trim(text).to_owned());
    }
    let (cleaned, wrap) = match title {
        Some(title) => {
            let (cleaned, wrap) = extract_wrap(&title);
            (Some(cleaned), wrap)
        }
        None => (None, None),
    };
    BoxData {
        name: cleaned
            .filter(|cleaned| !cleaned.is_empty())
            .map(|cleaned| sanitize_text(&cleaned)),
        color,
        wrap,
    }
}

/// The two halves of `/^((?:rgba?|hsla?)\s*\(.*\)|\w*)(.*)$/`.
fn split_box_colour(text: &str) -> (&str, &str) {
    if let Some(end) = colour_function_end(text) {
        return text.split_at(end);
    }
    let word = text
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    text.split_at(word)
}

/// The end of a colour function at the start of `text`: `(?:rgba?|hsla?)\s*\(.*\)`, where `.*`
/// takes up to the last closing parenthesis.
fn colour_function_end(text: &str) -> Option<usize> {
    for name in ["rgba", "rgb", "hsla", "hsl"] {
        let Some(after_name) = text.strip_prefix(name) else {
            continue;
        };
        let blanks = after_name.len() - after_name.trim_start_matches(is_js_space).len();
        let Some(inside) = after_name[blanks..].strip_prefix('(') else {
            continue;
        };
        if let Some(close) = inside.rfind(')') {
            return Some(text.len() - inside.len() + close + 1);
        }
    }
    None
}

/// Whether `value` is a colour that `CSS.supports('color', value)` accepts in Chromium: a CSS
/// named colour or `transparent` or `currentcolor` (any case), or a colour function with three or
/// four numeric arguments. See `docs-port/sequence/PLAN.md`, section 9.
pub(crate) fn is_css_colour(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower == "transparent" || lower == "currentcolor" || NAMED_COLOURS.contains(&lower.as_str())
    {
        return true;
    }
    let Some(open) = lower.find('(') else {
        return false;
    };
    let name = &lower[..open];
    if !matches!(name, "rgb" | "rgba" | "hsl" | "hsla") || !lower.ends_with(')') {
        return false;
    }
    let inside = &lower[open + 1..lower.len() - 1];
    let parts: Vec<&str> = inside
        .split(|c: char| c == ',' || c == '/' || is_js_space(c))
        .filter(|part| !part.is_empty())
        .collect();
    (parts.len() == 3 || parts.len() == 4) && parts.iter().all(|part| is_colour_number(part))
}

/// A number, optionally with `%` or an angle unit.
fn is_colour_number(part: &str) -> bool {
    let body = part
        .strip_suffix('%')
        .or_else(|| part.strip_suffix("deg"))
        .unwrap_or(part);
    let body = body.strip_prefix(['+', '-']).unwrap_or(body);
    let digits = body.chars().filter(|c| c.is_ascii_digit()).count();
    digits > 0
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.matches('.').count() <= 1
}

/// The CSS named colours (CSS Color Module Level 4), lower case.
const NAMED_COLOURS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];
