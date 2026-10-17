//! The rules of `event-modeling.langium`, as recursive descent over the scanner's tokens.
//!
//! The entry is `'eventmodeling' (statement)*`, and every statement starts with a token no other
//! statement starts with, so one token of lookahead decides each statement. The statements are
//! `title`, `accTitle`, `accDescr`, `entity`, a frame, `data`, `note` and `gwt`.

use super::ast::{
    DataEntity, EventModel, Frame, FrameKind, Gwt, ModelEntity, NoteEntity, Statement,
};
use super::scanner::{Spanned, Token};
use crate::error::{ParseError, Source};

/// The entity types of `EmModelEntityType`.
const ENTITY_TYPES: &[&str] = &[
    "rmo",
    "readmodel",
    "ui",
    "cmd",
    "command",
    "evt",
    "event",
    "pcr",
    "processor",
];

/// The data types of `EmDataType`, which the reference parses and never reads.
const DATA_TYPES: &[&str] = &[
    "json", "jsobj", "figma", "salt", "uri", "md", "html", "text",
];

/// Parses the scanned tokens of one text.
pub(crate) fn parse_model(
    tokens: &[Spanned],
    source: Source<'_>,
) -> Result<EventModel, ParseError> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        source,
    };
    parser.expect_keyword("eventmodeling")?;
    let mut model = EventModel::default();
    loop {
        let spanned = parser.peek();
        match &spanned.token {
            Token::Eof => return Ok(model),
            Token::AccDescr(text) => {
                model.acc_descr = Some(text.clone());
                parser.advance();
            }
            Token::AccTitle(text) => {
                model.acc_title = Some(text.clone());
                parser.advance();
            }
            Token::Title(text) => {
                model.title = Some(text.clone());
                parser.advance();
            }
            Token::Keyword("entity") => {
                parser.advance();
                let name = parser.qualified_name()?;
                model.model_entities.push(ModelEntity { name });
            }
            Token::Keyword("tf" | "timeframe") => {
                let frame = parser.frame(FrameKind::Time)?;
                model.frames.push(frame);
            }
            Token::Keyword("rf" | "resetframe") => {
                let frame = parser.frame(FrameKind::Reset)?;
                model.frames.push(frame);
            }
            Token::Keyword("data") => {
                parser.advance();
                let name = parser.id("expected the name of a data block")?;
                let data_block_value = parser.data_block()?;
                model.data_entities.push(DataEntity {
                    name,
                    data_block_value,
                });
            }
            Token::Keyword("note") => {
                parser.advance();
                let source_frame = parser.fid("expected the frame number of a note")?;
                let data_block_value = parser.data_block()?;
                model.note_entities.push(NoteEntity {
                    source_frame,
                    data_block_value,
                });
            }
            Token::Keyword("gwt") => {
                let gwt = parser.gwt()?;
                model.gwt_entities.push(gwt);
            }
            _ => return Err(parser.unexpected("expected a statement")),
        }
    }
}

/// The cursor over the tokens.
struct Parser<'a, 'b> {
    tokens: &'a [Spanned],
    pos: usize,
    source: Source<'b>,
}

impl Parser<'_, '_> {
    /// The token at the cursor. The scanner ends every list with `Eof`, which is never passed.
    fn peek(&self) -> &Spanned {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    /// Whether the token at the cursor is the keyword `word`.
    fn at_keyword(&self, word: &str) -> bool {
        matches!(&self.peek().token, Token::Keyword(keyword) if *keyword == word)
    }

    /// Takes the keyword `word` if it is at the cursor.
    fn eat_keyword(&mut self, word: &str) -> bool {
        if self.at_keyword(word) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, word: &str) -> Result<(), ParseError> {
        if self.eat_keyword(word) {
            Ok(())
        } else {
            Err(self.unexpected(&format!("expected '{word}'")))
        }
    }

    /// An error at the token at the cursor.
    fn unexpected(&self, message: &str) -> ParseError {
        let spanned = self.peek();
        let found = match &spanned.token {
            Token::Eof => "the end of the text".to_owned(),
            Token::Keyword(word) => format!("'{word}'"),
            _ => "a token that does not fit".to_owned(),
        };
        self.source
            .error(spanned.start, format!("{message}, found {found}"))
    }

    /// `EM_ID`: a name.
    fn id(&mut self, message: &str) -> Result<String, ParseError> {
        match &self.peek().token {
            Token::Id(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.unexpected(message)),
        }
    }

    /// `EM_FID`: a frame number.
    fn fid(&mut self, message: &str) -> Result<String, ParseError> {
        match &self.peek().token {
            Token::Fid(number) => {
                let number = number.clone();
                self.advance();
                Ok(number)
            }
            _ => Err(self.unexpected(message)),
        }
    }

    /// `QualifiedName`: `EM_ID ('.' EM_ID)*`, joined with dots.
    fn qualified_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.id("expected a name")?;
        while self.eat_keyword(".") {
            name.push('.');
            name.push_str(&self.id("expected a name after '.'")?);
        }
        Ok(name)
    }

    /// `EmTimeFrame` and `EmResetFrame`: the keyword, the number, the entity type, the qualified
    /// name, the sources, the data reference and the inline data, in that order.
    fn frame(&mut self, kind: FrameKind) -> Result<Frame, ParseError> {
        let offset = self.peek().start;
        self.advance();
        let name = self.fid("expected a frame number")?;
        let model_entity_type = self.entity_type("expected an entity type")?;
        let entity_identifier = self.qualified_name()?;
        let mut source_frames = Vec::new();
        while self.eat_keyword("->>") {
            source_frames.push(self.fid("expected a frame number after '->>'")?);
        }
        let data_reference = if self.eat_keyword("[[") {
            let reference = self.id("expected the name of a data block")?;
            self.expect_keyword("]]")?;
            Some(reference)
        } else {
            None
        };
        let data_inline = self.data_inline()?;
        Ok(Frame {
            kind,
            name,
            model_entity_type,
            entity_identifier,
            source_frames,
            data_reference,
            data_inline,
            offset,
        })
    }

    /// `EmModelEntityType`: one of the entity type keywords.
    fn entity_type(&mut self, message: &str) -> Result<String, ParseError> {
        match &self.peek().token {
            Token::Keyword(word) if ENTITY_TYPES.contains(word) => {
                let word = (*word).to_owned();
                self.advance();
                Ok(word)
            }
            _ => Err(self.unexpected(message)),
        }
    }

    /// `EmDataType` in backticks, which the grammar requires once the opening backtick is read.
    fn data_type_in_backticks(&mut self) -> Result<(), ParseError> {
        match &self.peek().token {
            Token::Keyword(word) if DATA_TYPES.contains(word) => self.advance(),
            _ => return Err(self.unexpected("expected a data type")),
        }
        self.expect_keyword("`")
    }

    /// `EmDataInline?`: an optional type in backticks, then the inline token.
    fn data_inline(&mut self) -> Result<Option<String>, ParseError> {
        if self.eat_keyword("`") {
            self.data_type_in_backticks()?;
            return self.inline_token().map(Some);
        }
        match &self.peek().token {
            Token::DataInline(_) => self.inline_token().map(Some),
            _ => Ok(None),
        }
    }

    fn inline_token(&mut self) -> Result<String, ParseError> {
        match &self.peek().token {
            Token::DataInline(text) => {
                let text = text.clone();
                self.advance();
                Ok(text)
            }
            _ => Err(self.unexpected("expected inline data")),
        }
    }

    /// `EmDataBlock`: an optional type in backticks, then the block token.
    fn data_block(&mut self) -> Result<String, ParseError> {
        if self.eat_keyword("`") {
            self.data_type_in_backticks()?;
        }
        match &self.peek().token {
            Token::DataBlock(text) => {
                let text = text.clone();
                self.advance();
                Ok(text)
            }
            _ => Err(self.unexpected("expected a data block")),
        }
    }

    /// `gwt` with its `given` statements, its optional `when` statements and its `then` statements.
    fn gwt(&mut self) -> Result<Gwt, ParseError> {
        self.advance();
        let source_frame = self.fid("expected the frame number of a gwt")?;
        self.expect_keyword("given")?;
        let given = self.statements("expected a given statement")?;
        let when = if self.eat_keyword("when") {
            self.statements("expected a when statement")?
        } else {
            Vec::new()
        };
        self.expect_keyword("then")?;
        let then = self.statements("expected a then statement")?;
        Ok(Gwt {
            source_frame,
            given,
            when,
            then,
        })
    }

    /// One or more statements: an entity type and a name, each.
    fn statements(&mut self, message: &str) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        loop {
            let is_type =
                matches!(&self.peek().token, Token::Keyword(word) if ENTITY_TYPES.contains(word));
            if !is_type {
                break;
            }
            let model_entity_type = self.entity_type(message)?;
            let entity = self.id("expected the name of an entity")?;
            statements.push(Statement {
                model_entity_type,
                entity,
            });
        }
        if statements.is_empty() {
            return Err(self.unexpected(message));
        }
        Ok(statements)
    }
}
