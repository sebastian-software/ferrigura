//! The grammar of `treeView.langium`, read by hand over the tokens of [`super::scanner`].
//!
//! ```text
//! TreeView:     'treeView-beta' (ACC_DESCR | ACC_TITLE | TITLE)* TreeNode*
//! TreeNode:     INDENTATION? (QUOTED_NAME | BARE_NAME) (CLASS_ANNOTATION | ICON_ANNOTATION | DESC_ANNOTATION)*
//! ```
//!
//! The header terminals are the entry's `TitleAndAccessibilities` fragment, read before the first
//! node; each one assigns its property, so the last value given wins, even an empty one. The
//! annotations are read in the same way: a second `:::` or `icon(` replaces the first. A token the
//! grammar does not take is an error, which the reference reports as a rejection too.

use super::scanner::{Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// A node line: its indentation, its name, and its annotations, each as the last one given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NodeAst {
    /// The number of indentation characters; 0 when the line has none.
    pub indent: usize,
    pub name: String,
    pub class: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
}

/// The syntax tree of a diagram: the header values, then the nodes in text order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Ast {
    /// The last `title` line's value; empty when there was none or the last one was empty.
    pub title: String,
    /// The last `accTitle` line's value.
    pub acc_title: String,
    /// The last `accDescr` line's value.
    pub acc_descr: String,
    pub nodes: Vec<NodeAst>,
}

/// Parses the prepared text of a treeView diagram, which ends in a line break, into its tree.
pub(crate) fn parse_text(text: &str) -> Result<Ast, ParseError> {
    let source = Source::new(text, 0);
    let tokens =
        tokenize(text).map_err(|failure| source.error(failure.offset, "unexpected character"))?;
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
    };

    parser.expect_keyword()?;
    let mut ast = Ast::default();
    while let Some(token) = parser.peek() {
        match token {
            Token::AccDescr(value) => ast.acc_descr = value.clone(),
            Token::AccTitle(value) => ast.acc_title = value.clone(),
            Token::Title(value) => ast.title = value.clone(),
            _ => break,
        }
        parser.advance();
    }
    while parser.peek().is_some() {
        let node = parser.node()?;
        ast.nodes.push(node);
    }
    Ok(ast)
}

/// A cursor over the tokens, with the error positions of [`Source`].
struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    /// The byte offset of the current token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start)
    }

    fn error_here(&self, expected: &str) -> ParseError {
        let found = match self.peek() {
            Some(token) => describe(token),
            None => "the end of the diagram".to_owned(),
        };
        self.source
            .error(self.offset(), format!("expected {expected}, found {found}"))
    }

    fn expect_keyword(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            Some(Token::Keyword) => {
                self.advance();
                Ok(())
            }
            _ => Err(self.error_here("'treeView-beta'")),
        }
    }

    /// One node line: an optional indentation, a name, and its annotations.
    fn node(&mut self) -> Result<NodeAst, ParseError> {
        let indent = match self.peek() {
            Some(Token::Indentation(width)) => {
                let width = *width;
                self.advance();
                width
            }
            _ => 0,
        };
        let name = match self.peek() {
            Some(Token::QuotedName(name) | Token::BareName(name)) => name.clone(),
            _ => return Err(self.error_here("a node name")),
        };
        self.advance();

        let mut node = NodeAst {
            indent,
            name,
            ..NodeAst::default()
        };
        loop {
            match self.peek() {
                Some(Token::ClassAnnotation(value)) => node.class = Some(value.clone()),
                Some(Token::IconAnnotation(value)) => node.icon = Some(value.clone()),
                Some(Token::DescAnnotation(value)) => node.description = Some(value.clone()),
                _ => break,
            }
            self.advance();
        }
        Ok(node)
    }
}

/// A token, as the error message names it.
fn describe(token: &Token) -> String {
    match token {
        Token::Keyword => "'treeView-beta'".to_owned(),
        Token::AccDescr(_) => "an accessible description".to_owned(),
        Token::AccTitle(_) => "an accessible title".to_owned(),
        Token::Title(_) => "a title".to_owned(),
        Token::ClassAnnotation(_) => "a class annotation".to_owned(),
        Token::IconAnnotation(_) => "an icon annotation".to_owned(),
        Token::DescAnnotation(_) => "a description annotation".to_owned(),
        Token::Indentation(_) => "an indentation".to_owned(),
        Token::QuotedName(_) | Token::BareName(_) => "a name".to_owned(),
    }
}
