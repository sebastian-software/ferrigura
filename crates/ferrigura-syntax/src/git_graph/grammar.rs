//! The gitGraph grammar, `packages/parser/src/language/gitGraph/gitGraph.langium`, by recursive
//! descent over the tokens of [`scanner`](super::scanner). Every statement ends with an end of
//! line: a line break or the end of the text.

use ferrigura_model::git_graph::{CommitType, Direction};

use super::ast::{Branch, Checkout, CherryPick, Commit, GitGraph, Merge, Statement, StatementKind};
use super::scanner::{Kw, Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// Parses the text of a gitGraph as the grammar reads it. The positions of errors are counted
/// from `source`.
pub(crate) fn parse_text(source: Source<'_>) -> Result<GitGraph, ParseError> {
    let tokens =
        tokenize(source.text()).map_err(|failure| source.error(failure.offset, failure.message))?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        source,
    };
    parser.graph()
}

struct Parser<'a> {
    tokens: Vec<Spanned>,
    pos: usize,
    source: Source<'a>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn peek_keyword(&self) -> Option<Kw> {
        match self.peek() {
            Some(Token::Keyword(kw)) => Some(*kw),
            _ => None,
        }
    }

    /// The offset of the next token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |t| t.start)
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    /// `EOL: NEWLINE+ | EOF`.
    fn end_of_line(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            None => Ok(()),
            Some(Token::Newline) => {
                while self.peek() == Some(&Token::Newline) {
                    self.bump();
                }
                Ok(())
            }
            Some(_) => Err(self.error("expected the end of the line")),
        }
    }

    fn graph(&mut self) -> Result<GitGraph, ParseError> {
        let mut ast = GitGraph {
            direction: None,
            acc_descr: String::new(),
            acc_title: String::new(),
            title: String::new(),
            statements: Vec::new(),
        };
        while self.peek() == Some(&Token::Newline) {
            self.bump();
        }
        self.header(&mut ast)?;
        while let Some(token) = self.peek().cloned() {
            match token {
                Token::Newline => self.bump(),
                Token::Title(value) => {
                    ast.title = value;
                    self.bump();
                    self.end_of_line()?;
                }
                Token::AccTitle(value) => {
                    ast.acc_title = value;
                    self.bump();
                    self.end_of_line()?;
                }
                Token::AccDescr(value) => {
                    ast.acc_descr = value;
                    self.bump();
                    self.end_of_line()?;
                }
                Token::Keyword(
                    kw @ (Kw::Commit
                    | Kw::Branch
                    | Kw::Merge
                    | Kw::Checkout
                    | Kw::Switch
                    | Kw::CherryPick),
                ) => {
                    let statement = self.statement(kw)?;
                    ast.statements.push(statement);
                }
                _ => {
                    return Err(
                        self.error("expected a statement, a title or an accessibility line")
                    );
                }
            }
        }
        Ok(ast)
    }

    /// `'gitGraph' | 'gitGraph' ':' | 'gitGraph:' | 'gitGraph' Direction ':'`.
    fn header(&mut self, ast: &mut GitGraph) -> Result<(), ParseError> {
        match self.peek_keyword() {
            Some(Kw::GitGraphColon) => {
                self.bump();
                Ok(())
            }
            Some(Kw::GitGraph) => {
                self.bump();
                if self.peek_keyword() == Some(Kw::Colon) {
                    self.bump();
                    return Ok(());
                }
                let direction = match self.peek_keyword() {
                    Some(Kw::Lr) => Direction::LR,
                    Some(Kw::Tb) => Direction::TB,
                    Some(Kw::Bt) => Direction::BT,
                    _ => return Ok(()),
                };
                self.bump();
                ast.direction = Some(direction);
                if self.peek_keyword() != Some(Kw::Colon) {
                    return Err(self.error("expected ':' after the direction"));
                }
                self.bump();
                Ok(())
            }
            _ => Err(self.error("expected 'gitGraph'")),
        }
    }

    fn statement(&mut self, kw: Kw) -> Result<Statement, ParseError> {
        let offset = self.offset();
        self.bump();
        let kind = match kw {
            Kw::Commit => StatementKind::Commit(self.commit()?),
            Kw::Branch => StatementKind::Branch(self.branch()?),
            Kw::Merge => StatementKind::Merge(self.merge()?),
            Kw::Checkout | Kw::Switch => {
                let branch = self.name()?;
                self.end_of_line()?;
                StatementKind::Checkout(Checkout { branch })
            }
            _ => StatementKind::CherryPick(self.cherry_pick()?),
        };
        Ok(Statement { offset, kind })
    }

    fn commit(&mut self) -> Result<Commit, ParseError> {
        let mut commit = Commit::default();
        loop {
            match self.peek().cloned() {
                Some(Token::Keyword(Kw::Id)) => {
                    self.bump();
                    commit.id = Some(self.string("a string after 'id:'")?);
                }
                Some(Token::Keyword(Kw::Msg)) => {
                    self.bump();
                    commit.message = Some(self.string("a string after 'msg:'")?);
                }
                Some(Token::Str(text)) => {
                    self.bump();
                    commit.message = Some(text);
                }
                Some(Token::Keyword(Kw::Tag)) => {
                    self.bump();
                    commit.tags.push(self.string("a string after 'tag:'")?);
                }
                Some(Token::Keyword(Kw::Type)) => {
                    self.bump();
                    commit.kind = Some(self.commit_type()?);
                }
                _ => break,
            }
        }
        self.end_of_line()?;
        Ok(commit)
    }

    fn branch(&mut self) -> Result<Branch, ParseError> {
        let name = self.name()?;
        let order = if self.peek_keyword() == Some(Kw::Order) {
            self.bump();
            Some(self.number()?)
        } else {
            None
        };
        self.end_of_line()?;
        Ok(Branch { name, order })
    }

    fn merge(&mut self) -> Result<Merge, ParseError> {
        let mut merge = Merge {
            branch: self.name()?,
            ..Merge::default()
        };
        loop {
            match self.peek_keyword() {
                Some(Kw::Id) => {
                    self.bump();
                    merge.id = Some(self.string("a string after 'id:'")?);
                }
                Some(Kw::Tag) => {
                    self.bump();
                    merge.tags.push(self.string("a string after 'tag:'")?);
                }
                Some(Kw::Type) => {
                    self.bump();
                    merge.kind = Some(self.commit_type()?);
                }
                _ => break,
            }
        }
        self.end_of_line()?;
        Ok(merge)
    }

    fn cherry_pick(&mut self) -> Result<CherryPick, ParseError> {
        let mut pick = CherryPick::default();
        loop {
            match self.peek_keyword() {
                Some(Kw::Id) => {
                    self.bump();
                    pick.id = Some(self.string("a string after 'id:'")?);
                }
                Some(Kw::Tag) => {
                    self.bump();
                    pick.tags.push(self.string("a string after 'tag:'")?);
                }
                Some(Kw::Parent) => {
                    self.bump();
                    pick.parent = Some(self.string("a string after 'parent:'")?);
                }
                _ => break,
            }
        }
        self.end_of_line()?;
        Ok(pick)
    }

    /// `REFERENCE | STRING`, a branch name.
    fn name(&mut self) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Token::Reference(name) | Token::Str(name)) => {
                self.bump();
                Ok(name)
            }
            _ => Err(self.error("expected a branch name")),
        }
    }

    fn string(&mut self, expected: &str) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Token::Str(text)) => {
                self.bump();
                Ok(text)
            }
            _ => Err(self.error(format!("expected {expected}"))),
        }
    }

    fn number(&mut self) -> Result<f64, ParseError> {
        match self.peek() {
            Some(Token::Int(value)) => {
                let value = *value;
                self.bump();
                Ok(value)
            }
            _ => Err(self.error("expected a whole number after 'order:'")),
        }
    }

    /// `'NORMAL' | 'REVERSE' | 'HIGHLIGHT'` after `type:`.
    fn commit_type(&mut self) -> Result<CommitType, ParseError> {
        let kind = match self.peek_keyword() {
            Some(Kw::Normal) => CommitType::Normal,
            Some(Kw::Reverse) => CommitType::Reverse,
            Some(Kw::Highlight) => CommitType::Highlight,
            _ => return Err(self.error("expected NORMAL, REVERSE or HIGHLIGHT after 'type:'")),
        };
        self.bump();
        Ok(kind)
    }
}
