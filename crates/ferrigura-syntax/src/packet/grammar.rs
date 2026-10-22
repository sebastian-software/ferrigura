//! The packet grammar, `packages/parser/src/language/packet/packet.langium`, by recursive descent
//! over the scanner's tokens:
//!
//! ```text
//! Packet:      NEWLINE* ("packet" | "packet-beta") ( TitleAndAccessibilities | PacketBlock | NEWLINE )*
//! PacketBlock: ( INT ( '-' INT )? | '+' INT ) ':' STRING EOL
//! EOL:         NEWLINE+ | EOF
//! ```
//!
//! The header values follow the grammar's assignments: the last `title`, `accTitle` and `accDescr`
//! line wins, and an empty value is kept as empty. The blocks keep the numbers that were written.
//! The DB's rules for the missing numbers and the rows are in `db.rs`.

use super::scanner::{Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// A block as the grammar gives it: the numbers that were written, `None` where they were not.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawBlock {
    /// `start`, or `None` for the `+bits` form.
    pub start: Option<f64>,
    /// `end`, given only by the `start-end` form.
    pub end: Option<f64>,
    /// `bits`, given only by the `+bits` form.
    pub bits: Option<f64>,
    /// The label, as the string converter gives it.
    pub label: String,
    /// The byte offset of the block in the prepared text, for error positions.
    pub offset: usize,
}

/// The syntax tree of a packet diagram: the header values and the blocks in source order.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Ast {
    pub title: String,
    pub acc_title: String,
    pub acc_descr: String,
    pub blocks: Vec<RawBlock>,
}

/// Parses the prepared text of a packet diagram.
pub(crate) fn parse_text(source: Source<'_>) -> Result<Ast, ParseError> {
    let tokens = tokenize(&source)?;
    let mut cursor = Cursor {
        source,
        tokens,
        pos: 0,
    };
    cursor.skip_newlines();
    cursor.expect_packet()?;

    let mut ast = Ast::default();
    while let Some(token) = cursor.peek().cloned() {
        match token {
            Token::Newline => cursor.skip_newlines(),
            Token::Title(text) => {
                cursor.pos += 1;
                cursor.expect_eol()?;
                ast.title = text;
            }
            Token::AccTitle(text) => {
                cursor.pos += 1;
                cursor.expect_eol()?;
                ast.acc_title = text;
            }
            Token::AccDescr(text) => {
                cursor.pos += 1;
                cursor.expect_eol()?;
                ast.acc_descr = text;
            }
            Token::Int(_) | Token::Plus => ast.blocks.push(cursor.block()?),
            _ => return Err(cursor.error("expected a block, a title or the end of the diagram")),
        }
    }
    Ok(ast)
}

/// Reads the tokens in order and reports errors at the next one.
struct Cursor<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    /// The byte offset of the next token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start)
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn skip_newlines(&mut self) {
        while self.peek() == Some(&Token::Newline) {
            self.pos += 1;
        }
    }

    fn expect_packet(&mut self) -> Result<(), ParseError> {
        if self.peek() == Some(&Token::Packet) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error("expected 'packet'"))
        }
    }

    /// `EOL: NEWLINE+ | EOF`.
    fn expect_eol(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            None => Ok(()),
            Some(Token::Newline) => {
                self.skip_newlines();
                Ok(())
            }
            Some(_) => Err(self.error("expected the end of the line")),
        }
    }

    fn expect_int(&mut self) -> Result<f64, ParseError> {
        match self.peek() {
            Some(Token::Int(value)) => {
                let value = *value;
                self.pos += 1;
                Ok(value)
            }
            _ => Err(self.error("expected a number")),
        }
    }

    /// `PacketBlock`: `start`, `start-end` or `+bits`, then `:`, the label and the end of the line.
    fn block(&mut self) -> Result<RawBlock, ParseError> {
        let offset = self.offset();
        let (start, end, bits) = match self.peek() {
            Some(Token::Int(start)) => {
                let start = *start;
                self.pos += 1;
                if self.peek() == Some(&Token::Dash) {
                    self.pos += 1;
                    let end = self.expect_int()?;
                    (Some(start), Some(end), None)
                } else {
                    (Some(start), None, None)
                }
            }
            Some(Token::Plus) => {
                self.pos += 1;
                (None, None, Some(self.expect_int()?))
            }
            _ => return Err(self.error("expected a block")),
        };
        if self.peek() != Some(&Token::Colon) {
            return Err(self.error("expected ':' after the bit range"));
        }
        self.pos += 1;
        let label = match self.peek() {
            Some(Token::Str(label)) => label.clone(),
            _ => return Err(self.error("expected a label in quotes")),
        };
        self.pos += 1;
        self.expect_eol()?;
        Ok(RawBlock {
            start,
            end,
            bits,
            label,
            offset,
        })
    }
}
