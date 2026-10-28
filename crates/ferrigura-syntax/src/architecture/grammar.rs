//! The grammar of `packages/parser/src/language/architecture/architecture.langium`, read by hand
//! over the tokens of [`super::scanner`]. Every rule is a function, and every rule is LL(1) in the
//! tokens the scanner gives, so the reference's parser makes the same choices.
//!
//! ```text
//! Architecture: NEWLINE* ('architecture' | 'architecture-beta') (NEWLINE | TitleAndAccessibilities | Statement)*
//! Statement:    Group | Service | Junction | Edge | Alignment
//! Group:        'group' ID ARCH_ICON? ARCH_TITLE? ('in' ID)? EOL
//! Service:      'service' ID (STRING | ARCH_ICON)? ARCH_TITLE? ('in' ID)? EOL
//! Junction:     'junction' ID ('in' ID)? EOL
//! Edge:         ID ARROW_GROUP? ':' DIR ARROW_INTO? ('--' | '-' ARCH_TITLE '-') ARROW_INTO? DIR ':' ID ARROW_GROUP? EOL
//! Alignment:    'align' ('row' | 'column') ID ID+ EOL
//! EOL:          NEWLINE+ | EOF
//! ```
//!
//! The header terminals keep the last value given, as Langium's assignments do, and the DB reads
//! them after the whole text parsed.

use ferrigura_model::architecture::{AlignDirection, Direction};

use super::scanner::{Kw, Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// A `group` statement.
#[derive(Debug, Clone)]
pub(crate) struct GroupStmt {
    pub id: String,
    pub icon: Option<String>,
    pub title: Option<String>,
    pub parent: Option<String>,
    pub offset: usize,
}

/// A `service` statement.
#[derive(Debug, Clone)]
pub(crate) struct ServiceStmt {
    pub id: String,
    pub icon: Option<String>,
    pub icon_text: Option<String>,
    pub title: Option<String>,
    pub parent: Option<String>,
    pub offset: usize,
}

/// A `junction` statement.
#[derive(Debug, Clone)]
pub(crate) struct JunctionStmt {
    pub id: String,
    pub parent: Option<String>,
    pub offset: usize,
}

/// An edge: the two ends, each with its `{group}` mark, and the arrow between them.
#[derive(Debug, Clone)]
pub(crate) struct EdgeStmt {
    pub lhs_id: String,
    pub lhs_group: bool,
    pub lhs_dir: Direction,
    pub lhs_into: bool,
    pub title: Option<String>,
    pub rhs_into: bool,
    pub rhs_dir: Direction,
    pub rhs_id: String,
    pub rhs_group: bool,
    pub offset: usize,
}

/// An `align` statement.
#[derive(Debug, Clone)]
pub(crate) struct AlignStmt {
    pub direction: AlignDirection,
    pub members: Vec<String>,
    pub offset: usize,
}

/// The syntax tree of a diagram: the header values, then each statement list in text order.
#[derive(Debug, Default, Clone)]
pub(crate) struct Ast {
    pub acc_descr: String,
    pub acc_title: String,
    pub title: String,
    pub groups: Vec<GroupStmt>,
    pub services: Vec<ServiceStmt>,
    pub junctions: Vec<JunctionStmt>,
    pub edges: Vec<EdgeStmt>,
    pub alignments: Vec<AlignStmt>,
}

/// Parses the prepared text of a diagram, which ends in a line break, into its tree.
pub(crate) fn parse_text(source: Source<'_>) -> Result<Ast, ParseError> {
    let tokens = tokenize(source.text())
        .map_err(|failure| source.error(failure.offset, "unexpected character"))?;
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
    };
    parser.skip_newlines();
    parser.expect_keyword(Kw::Architecture, "'architecture' or 'architecture-beta'")?;

    let mut ast = Ast::default();
    while let Some(token) = parser.peek().cloned() {
        let offset = parser.offset();
        match token {
            Token::Newline => parser.skip_newlines(),
            Token::AccDescr(text) => {
                parser.advance();
                parser.expect_eol()?;
                ast.acc_descr = text;
            }
            Token::AccTitle(text) => {
                parser.advance();
                parser.expect_eol()?;
                ast.acc_title = text;
            }
            Token::Title(text) => {
                parser.advance();
                parser.expect_eol()?;
                ast.title = text;
            }
            Token::Keyword(Kw::Group) => ast.groups.push(parser.group(offset)?),
            Token::Keyword(Kw::Service) => ast.services.push(parser.service(offset)?),
            Token::Keyword(Kw::Junction) => ast.junctions.push(parser.junction(offset)?),
            Token::Keyword(Kw::Align) => ast.alignments.push(parser.alignment(offset)?),
            Token::Id(_) => ast.edges.push(parser.edge(offset)?),
            _ => {
                return Err(parser.error(
                    "expected a statement, a title, an accessibility line or a line break",
                ));
            }
        }
    }
    Ok(ast)
}

struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    /// The byte offset of the next token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn skip_newlines(&mut self) {
        while self.peek() == Some(&Token::Newline) {
            self.advance();
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

    fn expect_keyword(&mut self, kw: Kw, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(&Token::Keyword(kw)) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {what}")))
        }
    }

    /// `ID`: a name, as the scanner gives it.
    fn id(&mut self) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Token::Id(name)) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("expected an id")),
        }
    }

    /// `('in' ID)?`: the enclosing group, when the `in` keyword is next.
    fn optional_parent(&mut self) -> Result<Option<String>, ParseError> {
        if self.peek() == Some(&Token::Keyword(Kw::In)) {
            self.advance();
            return self.id().map(Some);
        }
        Ok(None)
    }

    /// `ARCH_ICON?`.
    fn optional_icon(&mut self) -> Option<String> {
        self.take_token(|token| match token {
            Token::Icon(name) => Some(name.clone()),
            _ => None,
        })
    }

    /// `ARCH_TITLE?`.
    fn optional_label(&mut self) -> Option<String> {
        self.take_token(|token| match token {
            Token::Label(text) => Some(text.clone()),
            _ => None,
        })
    }

    /// Consumes the next token when `pick` accepts it, and returns what `pick` read from it.
    fn take_token<T>(&mut self, pick: impl Fn(&Token) -> Option<T>) -> Option<T> {
        let value = self.peek().and_then(pick)?;
        self.advance();
        Some(value)
    }

    /// `'group' ID ARCH_ICON? ARCH_TITLE? ('in' ID)? EOL`.
    fn group(&mut self, offset: usize) -> Result<GroupStmt, ParseError> {
        self.advance();
        let id = self.id()?;
        let icon = self.optional_icon();
        let title = self.optional_label();
        let parent = self.optional_parent()?;
        self.expect_eol()?;
        Ok(GroupStmt {
            id,
            icon,
            title,
            parent,
            offset,
        })
    }

    /// `'service' ID (STRING | ARCH_ICON)? ARCH_TITLE? ('in' ID)? EOL`.
    fn service(&mut self, offset: usize) -> Result<ServiceStmt, ParseError> {
        self.advance();
        let id = self.id()?;
        let icon_text = self.take_token(|token| match token {
            Token::Str(text) => Some(text.clone()),
            _ => None,
        });
        let icon = if icon_text.is_none() {
            self.optional_icon()
        } else {
            None
        };
        let title = self.optional_label();
        let parent = self.optional_parent()?;
        self.expect_eol()?;
        Ok(ServiceStmt {
            id,
            icon,
            icon_text,
            title,
            parent,
            offset,
        })
    }

    /// `'junction' ID ('in' ID)? EOL`.
    fn junction(&mut self, offset: usize) -> Result<JunctionStmt, ParseError> {
        self.advance();
        let id = self.id()?;
        let parent = self.optional_parent()?;
        self.expect_eol()?;
        Ok(JunctionStmt { id, parent, offset })
    }

    /// `'align' ('row' | 'column') ID ID+ EOL`.
    fn alignment(&mut self, offset: usize) -> Result<AlignStmt, ParseError> {
        self.advance();
        let direction = match self.peek() {
            Some(Token::Keyword(Kw::Row)) => AlignDirection::Row,
            Some(Token::Keyword(Kw::Column)) => AlignDirection::Column,
            _ => return Err(self.error("expected 'row' or 'column'")),
        };
        self.advance();
        let mut members = vec![self.id()?];
        members.push(self.id()?);
        while let Some(Token::Id(_)) = self.peek() {
            members.push(self.id()?);
        }
        self.expect_eol()?;
        Ok(AlignStmt {
            direction,
            members,
            offset,
        })
    }

    fn direction(&mut self) -> Result<Direction, ParseError> {
        match self.peek() {
            Some(Token::Direction(direction)) => {
                let direction = *direction;
                self.advance();
                Ok(direction)
            }
            _ => Err(self.error("expected a side: L, R, T or B")),
        }
    }

    fn expect_colon(&mut self) -> Result<(), ParseError> {
        self.expect_keyword(Kw::Colon, "':'")
    }

    /// `ID ARROW_GROUP? ':' DIR ARROW_INTO? ('--' | '-' ARCH_TITLE '-') ARROW_INTO? DIR ':' ID
    /// ARROW_GROUP? EOL`.
    fn edge(&mut self, offset: usize) -> Result<EdgeStmt, ParseError> {
        let lhs_id = self.id()?;
        let lhs_group = self.take_mark();
        self.expect_colon()?;
        let lhs_dir = self.direction()?;
        let lhs_into = self.take_into();
        let title = match self.peek() {
            Some(Token::Keyword(Kw::DashDash)) => {
                self.advance();
                None
            }
            Some(Token::Keyword(Kw::Dash)) => {
                self.advance();
                let label = self.take_token(|token| match token {
                    Token::Label(text) => Some(text.clone()),
                    _ => None,
                });
                let Some(label) = label else {
                    return Err(self.error("expected an edge label in brackets"));
                };
                self.expect_keyword(Kw::Dash, "'-' after the edge label")?;
                Some(label)
            }
            _ => return Err(self.error("expected '--' or '-[label]-'")),
        };
        let rhs_into = self.take_into();
        let rhs_dir = self.direction()?;
        self.expect_colon()?;
        let rhs_id = self.id()?;
        let rhs_group = self.take_mark();
        self.expect_eol()?;
        Ok(EdgeStmt {
            lhs_id,
            lhs_group,
            lhs_dir,
            lhs_into,
            title,
            rhs_into,
            rhs_dir,
            rhs_id,
            rhs_group,
            offset,
        })
    }

    /// `ARROW_GROUP?`.
    fn take_mark(&mut self) -> bool {
        if self.peek() == Some(&Token::GroupMark) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// `ARROW_INTO?`.
    fn take_into(&mut self) -> bool {
        if self.peek() == Some(&Token::Into) {
            self.advance();
            true
        } else {
            false
        }
    }
}
