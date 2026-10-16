//! The state grammar of `parser/stateDiagram.jison` (lines 178-352) as recursive descent over the
//! scanner's tokens. Each production has a function named after it; the statement values are the
//! ones its action gives, so the document keeps the grammar's objects and strings. See
//! `docs-port/state/PLAN.md`, section 4.
//!
//! The DB calls that the actions make at parse time are made here as well, in source order: the
//! divider counter (`getDividerId`) and the accessibility setters.

use ferrigura_model::state::{
    ApplyClassStmt, ClassDefStmt, ClickStmt, Description, DirStmt, Note, RelationStmt, StateStmt,
    Statement, StyleStmt,
};

use super::scanner::{Kind, ScanError, Token};
use crate::error::{ParseError, Source};
use crate::sanitize::sanitize_text;
use crate::text::{js_trim, newline_then_space_to_newline, strip_leading_space};

/// What the grammar gives for a whole diagram: the root statements and the header values the DB
/// holds after the parse.
pub(crate) struct Parsed {
    pub document: Vec<Statement>,
    pub acc_title: String,
    pub acc_descr: String,
}

/// Maps a scanner error to a parse error at its position.
pub(crate) fn scan_error(source: &Source<'_>, error: ScanError) -> ParseError {
    match error {
        ScanError::Unrecognized { offset } => source.error(offset, "unrecognized text"),
        ScanError::TwoWordName { offset, text } => source.error(
            offset,
            format!(
                "Error: State name must be a single word. Found: \"{}\"",
                js_trim(&text)
            ),
        ),
    }
}

/// `trimColon`: a leading colon goes, and the text is trimmed.
pub(crate) fn trim_colon(text: &str) -> String {
    match text.strip_prefix(':') {
        Some(rest) => js_trim(rest).to_owned(),
        None => js_trim(text).to_owned(),
    }
}

/// The recursive-descent parser over the tokens of one diagram.
pub(crate) struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Token>,
    pos: usize,
    divider_count: usize,
    acc_title: String,
    acc_descr: String,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: Source<'a>, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
            divider_count: 0,
            acc_title: String::new(),
            acc_descr: String::new(),
        }
    }

    /// `start`: line breaks, then `SD document`, then the end of the input.
    pub(crate) fn start(mut self) -> Result<Parsed, ParseError> {
        while self.kind() == Kind::Nl {
            self.pos += 1;
        }
        self.expect(Kind::Sd, "'stateDiagram' at the start of the diagram")?;
        let document = self.document(false)?;
        Ok(Parsed {
            document,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
        })
    }

    fn token(&self) -> &Token {
        // The scanner always ends with EOF, and the parser never reads past it.
        let last = self.tokens.len().saturating_sub(1);
        &self.tokens[self.pos.min(last)]
    }

    fn kind(&self) -> Kind {
        self.token().kind
    }

    /// Consumes the next token and returns it.
    fn bump(&mut self) -> Token {
        let token = self.token().clone();
        if token.kind != Kind::Eof {
            self.pos += 1;
        }
        token
    }

    fn error_here(&self, expected: &str) -> ParseError {
        let token = self.token();
        let found = match token.kind {
            Kind::Eof => "end of input".to_owned(),
            _ => format!("'{}'", token.value.escape_debug()),
        };
        self.source
            .error(token.start, format!("expected {expected}, found {found}"))
    }

    fn expect(&mut self, kind: Kind, expected: &str) -> Result<Token, ParseError> {
        if self.kind() == kind {
            Ok(self.bump())
        } else {
            Err(self.error_here(expected))
        }
    }

    /// `document: document line`, where `line` is a statement or a line break. A string `nl`
    /// is not kept, as the grammar's action drops it. Inside a composite the document ends at `}`;
    /// at the root, at the end of the input.
    fn document(&mut self, in_composite: bool) -> Result<Vec<Statement>, ParseError> {
        let mut document = Vec::new();
        loop {
            match self.kind() {
                Kind::Nl => {
                    self.bump();
                }
                Kind::StructStop if in_composite => return Ok(document),
                Kind::Eof if !in_composite => return Ok(document),
                _ => {
                    let statement = self.statement()?;
                    if statement != Statement::Text("nl".to_owned()) {
                        document.push(statement);
                    }
                }
            }
        }
    }

    /// `statement`: the alternatives of lines 201-289 and the direction, `acc_*` and `classDef`
    /// statements.
    fn statement(&mut self) -> Result<Statement, ParseError> {
        match self.kind() {
            Kind::ClassDef => self.class_def(),
            Kind::Style => self.style(),
            Kind::Class => self.css_class(),
            Kind::Id | Kind::EdgeState => self.id_line(),
            Kind::CompositState => self.composite(),
            Kind::StateDescr => self.described_state(),
            Kind::Fork => self.marked_state(Kind::Fork, "fork"),
            Kind::Join => self.marked_state(Kind::Join, "join"),
            Kind::Choice => self.marked_state(Kind::Choice, "choice"),
            Kind::Concurrent => {
                self.bump();
                self.divider_count += 1;
                let id = format!("divider-id-{}", self.divider_count);
                Ok(Statement::State(StateStmt::new(id, Some("divider"))))
            }
            Kind::Note => self.note(),
            Kind::DirTb => self.direction("TB"),
            Kind::DirBt => self.direction("BT"),
            Kind::DirRl => self.direction("RL"),
            Kind::DirLr => self.direction("LR"),
            Kind::AccTitle => {
                self.bump();
                let value = self.expect(Kind::AccTitleValue, "the title text")?;
                let value = js_trim(&value.value).to_owned();
                self.acc_title = strip_leading_space(&sanitize_text(&value)).to_owned();
                Ok(Statement::Text(value))
            }
            Kind::AccDescr => {
                self.bump();
                let value = self.expect(Kind::AccDescrValue, "the description text")?;
                self.set_acc_descr(&value.value);
                Ok(Statement::Text(js_trim(&value.value).to_owned()))
            }
            Kind::AccDescrMultiValue => {
                let value = self.bump();
                self.set_acc_descr(&value.value);
                Ok(Statement::Text(js_trim(&value.value).to_owned()))
            }
            Kind::Click => self.click(),
            Kind::HideEmpty | Kind::Scale => self.keyword_line(),
            _ => Err(self.error_here("a statement")),
        }
    }

    fn set_acc_descr(&mut self, raw: &str) {
        let value = js_trim(raw);
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(value));
    }

    /// `classDef CLASSDEF_ID CLASSDEF_STYLEOPTS`. The `DEFAULT` form of the grammar cannot match: the
    /// lexer gives `DEFAULT_CLASSDEF_ID` for it.
    fn class_def(&mut self) -> Result<Statement, ParseError> {
        self.bump();
        let id = self.expect(Kind::ClassDefIdName, "a class name")?;
        let classes = self.expect(Kind::ClassDefStyleOpts, "the class styles")?;
        Ok(Statement::ClassDef(ClassDefStmt {
            stmt: "classDef".to_owned(),
            id: js_trim(&id.value).to_owned(),
            classes: js_trim(&classes.value).to_owned(),
        }))
    }

    /// `style STYLE_IDS STYLEDEF_STYLEOPTS`.
    fn style(&mut self) -> Result<Statement, ParseError> {
        self.bump();
        let ids = self.expect(Kind::StyleIds, "the state ids of the style")?;
        let styles = self.expect(Kind::StyleDefStyleOpts, "the style list")?;
        Ok(Statement::Style(StyleStmt {
            stmt: "style".to_owned(),
            id: js_trim(&ids.value).to_owned(),
            style_class: js_trim(&styles.value).to_owned(),
        }))
    }

    /// `class CLASSENTITY_IDS STYLECLASS`.
    fn css_class(&mut self) -> Result<Statement, ParseError> {
        self.bump();
        let ids = self.expect(Kind::ClassEntityIds, "the state ids of the class")?;
        let classes = self.expect(Kind::StyleClass, "the class names")?;
        Ok(Statement::ApplyClass(ApplyClassStmt {
            stmt: "applyClass".to_owned(),
            id: js_trim(&ids.value).to_owned(),
            style_class: js_trim(&classes.value).to_owned(),
        }))
    }

    /// `idStatement`, then nothing, `DESCR`, or `'-->' idStatement [DESCR]`.
    fn id_line(&mut self) -> Result<Statement, ParseError> {
        let state = self.id_statement()?;
        match self.kind() {
            Kind::Descr => {
                let description = self.bump();
                let mut state = state;
                state.description = Some(Description::Text(trim_colon(&description.value)));
                Ok(Statement::State(state))
            }
            Kind::Arrow => {
                self.bump();
                let target = self.id_statement()?;
                let description = if self.kind() == Kind::Descr {
                    Some(trim_colon(&self.bump().value))
                } else {
                    None
                };
                Ok(Statement::Relation(RelationStmt {
                    stmt: "relation".to_owned(),
                    state1: state,
                    state2: target,
                    description,
                }))
            }
            _ => Ok(Statement::State(state)),
        }
    }

    /// `idStatement: ID | EDGE_STATE | ID STYLE_SEPARATOR ID | EDGE_STATE STYLE_SEPARATOR ID`.
    fn id_statement(&mut self) -> Result<StateStmt, ParseError> {
        let head = self.bump();
        if !matches!(head.kind, Kind::Id | Kind::EdgeState) {
            return Err(self.source.error(head.start, "expected a state id"));
        }
        let mut state = StateStmt::new(js_trim(&head.value), Some("default"));
        state.description = Some(Description::Text(String::new()));
        if self.kind() == Kind::StyleSeparator {
            self.bump();
            let class = self.expect(Kind::Id, "a class name after ':::'")?;
            state.classes = Some(vec![js_trim(&class.value).to_owned()]);
        }
        Ok(state)
    }

    /// `COMPOSIT_STATE` alone (a string), or with `{ document }` (a state with a document).
    fn composite(&mut self) -> Result<Statement, ParseError> {
        let name = self.bump();
        if self.kind() != Kind::StructStart {
            return Ok(Statement::Text(name.value));
        }
        self.bump();
        let document = self.document(true)?;
        self.expect(Kind::StructStop, "'}' to end the composite state")?;
        let mut state = StateStmt::new(name.value, Some("default"));
        state.description = Some(Description::Text(String::new()));
        state.doc = Some(document);
        Ok(Statement::State(state))
    }

    /// `STATE_DESCR AS ID`, with or without `{ document }`. Without, a colon in the id splits it,
    /// and the description becomes the list of the two parts; with, the texts stay as written.
    fn described_state(&mut self) -> Result<Statement, ParseError> {
        let description = self.bump();
        self.expect(Kind::As, "'as' after the state description")?;
        let id = self.expect(Kind::Id, "the state id after 'as'")?;
        if self.kind() == Kind::StructStart {
            self.bump();
            let document = self.document(true)?;
            self.expect(Kind::StructStop, "'}' to end the composite state")?;
            let mut state = StateStmt::new(id.value, Some("default"));
            state.description = Some(Description::Text(description.value));
            state.doc = Some(document);
            return Ok(Statement::State(state));
        }
        let mut text = js_trim(&description.value).to_owned();
        let mut state_id = id.value.clone();
        let mut state_description = None;
        if id.value.contains(':') {
            let mut parts = id.value.split(':');
            state_id = parts.next().unwrap_or_default().to_owned();
            let second = parts.next().unwrap_or_default().to_owned();
            state_description = Some(Description::List(vec![text, second]));
            text = String::new();
        }
        let mut state = StateStmt::new(state_id, Some("default"));
        state.description = Some(state_description.unwrap_or(Description::Text(text)));
        Ok(Statement::State(state))
    }

    /// `FORK`, `JOIN` or `CHOICE` on its own.
    fn marked_state(&mut self, _kind: Kind, name: &str) -> Result<Statement, ParseError> {
        let token = self.bump();
        Ok(Statement::State(StateStmt::new(token.value, Some(name))))
    }

    /// `note notePosition ID NOTE_TEXT`, or `note NOTE_TEXT AS ID`, which the grammar gives the
    /// value of the `note` token.
    fn note(&mut self) -> Result<Statement, ParseError> {
        let keyword = self.bump();
        match self.kind() {
            Kind::LeftOf | Kind::RightOf => {
                let position = self.bump();
                let id = self.expect(Kind::Id, "the state id of the note")?;
                let text = self.expect(Kind::NoteText, "the note text")?;
                let mut state = StateStmt::new(js_trim(&id.value), None);
                state.note = Some(Note {
                    position: js_trim(&position.value).to_owned(),
                    text: js_trim(&text.value).to_owned(),
                });
                Ok(Statement::State(state))
            }
            Kind::NoteText => {
                self.bump();
                self.expect(Kind::As, "'as' in a floating note")?;
                self.expect(Kind::Id, "the state id of the note")?;
                Ok(Statement::Text(keyword.value))
            }
            _ => Err(self.error_here("'left of', 'right of' or the note text")),
        }
    }

    /// `direction_*`: the DB's direction and the statement.
    fn direction(&mut self, value: &str) -> Result<Statement, ParseError> {
        self.bump();
        Ok(Statement::Dir(DirStmt {
            stmt: "dir".to_owned(),
            value: value.to_owned(),
        }))
    }

    /// `CLICK idStatement STRING STRING NL` and `CLICK idStatement HREF STRING NL`.
    fn click(&mut self) -> Result<Statement, ParseError> {
        self.bump();
        let id = self.id_statement()?;
        let (url, tooltip) = match self.kind() {
            Kind::Href => {
                self.bump();
                let url = self.expect(Kind::Str, "the address")?;
                (url.value, String::new())
            }
            Kind::Str => {
                let url = self.bump();
                let tooltip = self.expect(Kind::Str, "the tooltip")?;
                (url.value, tooltip.value)
            }
            _ => return Err(self.error_here("an address after the state id")),
        };
        self.expect(Kind::Nl, "the end of the line after 'click'")?;
        Ok(Statement::Click(ClickStmt {
            stmt: "click".to_owned(),
            id,
            url,
            tooltip,
        }))
    }

    /// `hide empty description` and `scale WIDTH`: the grammar gives the keyword's text.
    fn keyword_line(&mut self) -> Result<Statement, ParseError> {
        let keyword = self.bump();
        if keyword.kind == Kind::Scale {
            self.expect(Kind::Width, "the width after 'scale'")?;
        }
        Ok(Statement::Text(keyword.value))
    }
}
