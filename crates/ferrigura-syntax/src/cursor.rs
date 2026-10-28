//! The token cursor the recursive-descent parsers share.

use crate::error::{ParseError, Source};
use crate::lexer::{Spanned, Token};

/// Reads tokens in order and reports errors at the next token.
pub(crate) struct Cursor<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(source: Source<'a>, tokens: Vec<Spanned>) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    /// The next token, without consuming it.
    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    /// The byte offset of the next token, or the end of the text.
    pub(crate) fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |t| t.start)
    }

    /// Consumes the next token and returns it with its offset.
    pub(crate) fn advance(&mut self) -> Option<(Token, usize)> {
        let spanned = self.tokens.get(self.pos)?.clone();
        self.pos += 1;
        Some((spanned.token, spanned.start))
    }

    /// An error at the next token.
    pub(crate) fn error(&self, message: impl Into<String>) -> ParseError {
        self.source.error(self.offset(), message)
    }

    /// Consumes any number of line breaks.
    pub(crate) fn skip_newlines(&mut self) {
        while self.peek() == Some(&Token::Newline) {
            self.pos += 1;
        }
    }

    /// Whether the next token is the word `word`.
    pub(crate) fn at_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w == word)
    }

    /// Consumes the word `word` if it is next; reports whether it was there.
    pub(crate) fn eat_word(&mut self, word: &str) -> bool {
        let found = self.at_word(word);
        if found {
            self.pos += 1;
        }
        found
    }

    /// Consumes the word `word` or fails.
    pub(crate) fn expect_word(&mut self, word: &str) -> Result<(), ParseError> {
        if self.eat_word(word) {
            Ok(())
        } else {
            Err(self.error(format!("expected '{word}'")))
        }
    }

    /// Consumes the token `expected` or fails with `what` as the description.
    pub(crate) fn expect(&mut self, expected: &Token, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected {what}")))
        }
    }

    /// Consumes a number and returns it with its offset.
    pub(crate) fn expect_number(&mut self) -> Result<(f64, usize), ParseError> {
        match self.peek() {
            Some(Token::Number(value)) => {
                let value = *value;
                let offset = self.offset();
                self.pos += 1;
                Ok((value, offset))
            }
            _ => Err(self.error("expected a number")),
        }
    }

    /// The end of a line: one or more line breaks, or the end of the text.
    pub(crate) fn expect_eol(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            None => Ok(()),
            Some(Token::Newline) => {
                self.skip_newlines();
                Ok(())
            }
            Some(_) => Err(self.error("expected the end of the line")),
        }
    }
}
