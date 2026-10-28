//! The grammar of `packages/parser/src/language/cynefin/cynefin.langium`, read by hand over the
//! tokens of [`super::scanner`]. Every rule is a function. The decisions that need lookahead are
//! the same as the reference's: a `DOMAIN_NAME` that is followed by `-->` starts a transition, and
//! any other one starts a block.
//!
//! ```text
//! Cynefin:          NEWLINE* HEADER NEWLINE* (TitleAndAccessibilities | DomainBlock | Transition | NEWLINE)*
//! DomainBlock:      DOMAIN_NAME NEWLINE* (STRING NEWLINE*)*
//! Transition:       DOMAIN_NAME '-->' DOMAIN_NAME (':' STRING)? EOL
//! TitleAndAccessibilities: ((ACC_DESCR | ACC_TITLE | TITLE) EOL)+
//! EOL:              NEWLINE+ | EOF
//! ```
//!
//! The header terminals keep the last value given, as Langium's assignments do, and the DB reads
//! them after the whole text parsed.

use super::scanner::{Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// What the grammar gives: the header texts, the domain blocks and the transitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Ast {
    pub title: String,
    pub acc_title: String,
    pub acc_descr: String,
    pub domains: Vec<DomainBlock>,
    pub transitions: Vec<Transition>,
}

/// A `DOMAIN_NAME` with its items, in the order the text gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DomainBlock {
    pub name: &'static str,
    pub items: Vec<String>,
}

/// A transition, with its label when the text gave one (an empty label is `Some("")`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Transition {
    pub from: &'static str,
    pub to: &'static str,
    pub label: Option<String>,
}

/// Parses the prepared text of `source` into the AST.
///
/// # Errors
///
/// A lexical error, or a parser error at the first token the grammar does not take.
pub(crate) fn parse_text(source: Source<'_>) -> Result<Ast, ParseError> {
    let mut parser = Parser {
        source,
        tokens: tokenize(source)?,
        pos: 0,
    };
    parser.skip_newlines();
    parser.expect_header()?;
    parser.skip_newlines();

    let mut ast = Ast::default();
    while let Some(token) = parser.peek().cloned() {
        match token {
            Token::Newline => parser.pos += 1,
            Token::AccDescr(_) | Token::AccTitle(_) | Token::Title(_) => {
                parser.header_lines(&mut ast)?;
            }
            Token::Domain(_) if parser.peek_at(1) == Some(&Token::Arrow) => {
                let transition = parser.transition()?;
                ast.transitions.push(transition);
            }
            Token::Domain(_) => {
                let block = parser.domain_block()?;
                ast.domains.push(block);
            }
            _ => return Err(parser.error("unexpected token")),
        }
    }
    Ok(ast)
}

/// The token reader: the tokens and the position of the next one.
struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn peek_at(&self, ahead: usize) -> Option<&Token> {
        self.tokens.get(self.pos + ahead).map(|t| &t.token)
    }

    /// The byte offset of the next token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |t| t.start)
    }

    /// Consumes the next token, which the caller has checked is there.
    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos)?.token.clone();
        self.pos += 1;
        Some(token)
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn skip_newlines(&mut self) {
        while self.peek() == Some(&Token::Newline) {
            self.pos += 1;
        }
    }

    fn expect_header(&mut self) -> Result<(), ParseError> {
        if self.peek() == Some(&Token::Header) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error("expected 'cynefin-beta'"))
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

    /// `TitleAndAccessibilities: ((ACC_DESCR | ACC_TITLE | TITLE) EOL)+`. Each line sets its
    /// header text, so the last one given is the one that stays.
    fn header_lines(&mut self, ast: &mut Ast) -> Result<(), ParseError> {
        loop {
            match self.take() {
                Some(Token::AccDescr(text)) => ast.acc_descr = text,
                Some(Token::AccTitle(text)) => ast.acc_title = text,
                Some(Token::Title(text)) => ast.title = text,
                _ => return Err(self.error("expected a title or an accessibility line")),
            }
            self.expect_eol()?;
            if !matches!(
                self.peek(),
                Some(Token::AccDescr(_) | Token::AccTitle(_) | Token::Title(_))
            ) {
                return Ok(());
            }
        }
    }

    /// `DomainBlock: DOMAIN_NAME NEWLINE* (DomainItem NEWLINE*)*`.
    fn domain_block(&mut self) -> Result<DomainBlock, ParseError> {
        let Some(Token::Domain(name)) = self.take() else {
            return Err(self.error("expected a domain name"));
        };
        self.skip_newlines();
        let mut items = Vec::new();
        while let Some(Token::Str(label)) = self.peek().cloned() {
            self.pos += 1;
            items.push(label);
            self.skip_newlines();
        }
        Ok(DomainBlock { name, items })
    }

    /// `Transition: DOMAIN_NAME '-->' DOMAIN_NAME (':' STRING)? EOL`.
    fn transition(&mut self) -> Result<Transition, ParseError> {
        let Some(Token::Domain(from)) = self.take() else {
            return Err(self.error("expected a domain name"));
        };
        if self.take() != Some(Token::Arrow) {
            return Err(self.error("expected '-->'"));
        }
        let Some(Token::Domain(to)) = self.take() else {
            return Err(self.error("expected a domain name"));
        };
        let label = if self.peek() == Some(&Token::Colon) {
            self.pos += 1;
            let Some(Token::Str(label)) = self.take() else {
                return Err(self.error("expected a string label"));
            };
            Some(label)
        } else {
            None
        };
        self.expect_eol()?;
        Ok(Transition { from, to, label })
    }
}
