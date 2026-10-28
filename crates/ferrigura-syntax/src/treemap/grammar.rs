//! The rules of the treemap grammar, `packages/parser/src/language/treemap/treemap.langium`:
//!
//! ```text
//! Treemap:     TREEMAP_KEYWORD ( TitleAndAccessibilities | TreemapRow )*
//! TreemapRow:  INDENTATION? ( Item | ClassDef )
//! Item:        Leaf | Section
//! Section:     STRING2 ( STYLE_SEPARATOR ID2 )?
//! Leaf:        STRING2 INDENTATION? ( SEPARATOR | COMMA ) INDENTATION? NUMBER2 ( STYLE_SEPARATOR ID2 )?
//! ```
//!
//! The header texts take the last value given, as the grammar assigns them. A leaf is decided by
//! the token after the name (and a blank before a separator), so a blank that ends a row is the
//! indentation of the next row, as the reference's parser reads it.

use super::scanner::{Spanned, Token};
use crate::common::Header;
use crate::error::{ParseError, Source};

/// The syntax tree of a treemap: its header texts and its rows, in order.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Ast {
    pub header: Header,
    pub rows: Vec<Row>,
}

/// A row: a `classDef`, or an item with the indentation the row gave (zero without any).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Row {
    ClassDef { name: String, style: String },
    Item { level: usize, item: Item },
}

/// A section or a leaf, with its class selector when it has one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Leaf {
        name: String,
        value: f64,
        class: Option<String>,
    },
    Section {
        name: String,
        class: Option<String>,
    },
}

/// Parses the tokens of a treemap (the text after the preparation) into its syntax tree.
pub(crate) fn parse(tokens: &[Spanned], source: Source<'_>) -> Result<Ast, ParseError> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        source,
    };
    match parser.peek() {
        Some(Token::Keyword) => parser.pos += 1,
        _ => return Err(parser.error("expected the keyword treemap at the start of the diagram")),
    }

    let mut ast = Ast::default();
    while let Some(token) = parser.peek().cloned() {
        match token {
            Token::Title(text) => {
                ast.header.title = text;
                parser.pos += 1;
            }
            Token::AccTitle(text) => {
                ast.header.acc_title = text;
                parser.pos += 1;
            }
            Token::AccDescr(text) => {
                ast.header.acc_descr = text;
                parser.pos += 1;
            }
            Token::Indentation(level) => {
                parser.pos += 1;
                ast.rows.push(parser.row(level)?);
            }
            _ => ast.rows.push(parser.row(0)?),
        }
    }
    Ok(ast)
}

struct Parser<'a> {
    tokens: &'a [Spanned],
    pos: usize,
    source: Source<'a>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    fn peek_at(&self, ahead: usize) -> Option<&Token> {
        self.tokens
            .get(self.pos + ahead)
            .map(|spanned| &spanned.token)
    }

    /// An error at the token the parser is at, or at the end of the text.
    fn error(&self, message: &str) -> ParseError {
        let offset = self
            .tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start);
        let found = match self.peek() {
            Some(token) => format!("{message}, found {}", describe(token)),
            None => format!("{message}, found the end of the diagram"),
        };
        self.source.error(offset, found)
    }

    /// A row after its indentation: a `classDef`, or a name and the item it starts.
    fn row(&mut self, level: usize) -> Result<Row, ParseError> {
        match self.peek().cloned() {
            Some(Token::ClassDef { name, style }) => {
                self.pos += 1;
                Ok(Row::ClassDef { name, style })
            }
            Some(Token::Str(name)) => {
                self.pos += 1;
                Ok(Row::Item {
                    level,
                    item: self.item(name)?,
                })
            }
            _ => Err(self.error("expected a name in quotes or a classDef")),
        }
    }

    /// A leaf when a separator follows the name (after a blank, if there is one), else a section.
    fn item(&mut self, name: String) -> Result<Item, ParseError> {
        let is_leaf = match self.peek() {
            Some(Token::Separator | Token::Comma) => true,
            Some(Token::Indentation(_)) => {
                matches!(self.peek_at(1), Some(Token::Separator | Token::Comma))
            }
            _ => false,
        };
        if !is_leaf {
            let class = self.class_selector()?;
            return Ok(Item::Section { name, class });
        }
        if matches!(self.peek(), Some(Token::Indentation(_))) {
            self.pos += 1;
        }
        self.pos += 1;
        if matches!(self.peek(), Some(Token::Indentation(_))) {
            self.pos += 1;
        }
        let Some(Token::Number(value)) = self.peek().cloned() else {
            return Err(self.error("expected a number after the separator"));
        };
        self.pos += 1;
        let class = self.class_selector()?;
        Ok(Item::Leaf { name, value, class })
    }

    /// The `:::` class of an item, when there is one. A blank before `:::` is not a class.
    fn class_selector(&mut self) -> Result<Option<String>, ParseError> {
        if !matches!(self.peek(), Some(Token::StyleSeparator)) {
            return Ok(None);
        }
        self.pos += 1;
        let Some(Token::Id(class)) = self.peek().cloned() else {
            return Err(self.error("expected a class name after :::"));
        };
        self.pos += 1;
        Ok(Some(class))
    }
}

/// A token as the message names it.
fn describe(token: &Token) -> String {
    match token {
        Token::Keyword => "the keyword treemap".to_owned(),
        Token::Title(_) => "a title".to_owned(),
        Token::AccTitle(_) => "an accTitle".to_owned(),
        Token::AccDescr(_) => "an accDescr".to_owned(),
        Token::ClassDef { .. } => "a classDef".to_owned(),
        Token::StyleSeparator => "':::'".to_owned(),
        Token::Separator => "':'".to_owned(),
        Token::Comma => "','".to_owned(),
        Token::Indentation(_) => "a blank run".to_owned(),
        Token::Id(name) => format!("the name {name}"),
        Token::Number(_) => "a number".to_owned(),
        Token::Str(_) => "a quoted name".to_owned(),
    }
}
