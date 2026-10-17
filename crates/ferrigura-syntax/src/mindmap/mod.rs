//! The mindmap parser (P2): `mindmap.jison` and the mindmap DB `mindmapDb.ts`, ported by hand.
//! The plan, the lexer's rules and the quirks kept are in `docs-port/mindmap/PLAN.md`.
//!
//! The text goes through the reference's steps before the scanner sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The grammar then reads the
//! statements, and the DB calls them in document order. The config the text carries sets the padding
//! and the width of the nodes when they are added, so [`parse`] reads it first.

mod db;
mod scanner;

use ferrigura_model::mindmap::MindmapDiagram;
use ferrigura_model::value::Value;

use crate::config;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

use db::{Db, Settings, node_type};
use scanner::{Spanned, Token, scan};

/// Parses a mindmap and returns its model.
///
/// The error position is the token the grammar rejects, counted from the first line of the original
/// input. A DB error (a node without a parent) is reported at the node.
pub fn parse(raw: &str) -> Result<MindmapDiagram, ParseError> {
    let settings = settings_of(raw)?;
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let statements = parse_tokens(&scan(&text), source)?;
    let mut db = Db::new(settings);
    for statement in statements {
        let applied = match &statement {
            Statement::Node {
                level,
                id,
                descr,
                node_type,
                ..
            } => db.add_node(*level, id, descr, *node_type),
            Statement::Icon { text, .. } => db.decorate_icon(text),
            Statement::Class { text, .. } => db.decorate_class(text),
        };
        if let Err(message) = applied {
            return Err(source.error(statement.offset(), message));
        }
    }
    Ok(db.into_model())
}

/// The settings the DB reads when it adds a node: `mindmap.padding` and `mindmap.maxNodeWidth` of
/// the configuration the text carries, with the defaults of the reference.
fn settings_of(raw: &str) -> Result<Settings, ParseError> {
    let defaults = Settings::default();
    let settings = config::read(raw)?;
    let section = settings.get("mindmap");
    // `conf.mindmap?.padding ?? default`: a missing or null value takes the default. The merge of
    // the configuration (`assignWithDepth`) does not put an array or an object in place of a number,
    // so those take the default too; the defaults of both keys are numbers.
    let read = |key: &str| {
        section
            .and_then(|s| s.get(key))
            .filter(|value| !matches!(value, Value::Null | Value::Array(_) | Value::Object(_)))
            .cloned()
    };
    Ok(Settings {
        padding: read("padding").unwrap_or(defaults.padding),
        max_node_width: read("maxNodeWidth").unwrap_or(defaults.max_node_width),
    })
}

/// A statement of the grammar, with the offset of its first token. The DB calls it makes are
/// `addNode`, `decorateNode({icon})` and `decorateNode({class})`.
#[derive(Debug, Clone, PartialEq)]
enum Statement {
    /// `SPACELIST node` (the level is the indentation) or `node` (level 0).
    Node {
        offset: usize,
        level: i64,
        id: String,
        descr: String,
        node_type: u8,
    },
    /// `SPACELIST ICON` or `ICON`.
    Icon { offset: usize, text: String },
    /// `SPACELIST CLASS` or `CLASS`.
    Class { offset: usize, text: String },
}

impl Statement {
    fn offset(&self) -> usize {
        match self {
            Self::Node { offset, .. } | Self::Icon { offset, .. } | Self::Class { offset, .. } => {
                *offset
            }
        }
    }
}

/// The grammar of `mindmap.jison`, read from the tokens:
///
/// ```text
/// start     : mindMap | spaceLines mindMap
/// spaceLines: SPACELINE | spaceLines SPACELINE | spaceLines NL
/// mindMap   : MINDMAP document | MINDMAP NL document
/// document  : document statement stop | statement stop
/// stop      : NL | EOF | SPACELINE | stop NL | stop EOF
/// ```
///
/// A statement is what the grammar's `statement` rule takes; a `SPACELINE` after a stop starts a
/// statement of its own, since a stop takes only line breaks after its first token. The result is
/// the statements that call the DB, in order, and the statements that call nothing are dropped.
fn parse_tokens(tokens: &[Spanned], source: Source<'_>) -> Result<Vec<Statement>, ParseError> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        source,
    };
    if parser.is(&Token::SpaceLine) {
        parser.pos += 1;
        while parser.is(&Token::SpaceLine) || parser.is(&Token::Newline) {
            parser.pos += 1;
        }
    }
    parser.expect(&Token::Mindmap, "expected 'mindmap'")?;
    if parser.is(&Token::Newline) {
        parser.pos += 1;
    }
    let mut statements = Vec::new();
    loop {
        if let Some(statement) = parser.statement()? {
            statements.push(statement);
        }
        parser.stop()?;
        if parser.pos >= tokens.len() {
            return Ok(statements);
        }
    }
}

struct Parser<'a> {
    tokens: &'a [Spanned],
    pos: usize,
    source: Source<'a>,
}

impl Parser<'_> {
    fn token(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    fn is(&self, token: &Token) -> bool {
        self.token() == Some(token)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or_else(|| self.source.text().len(), |spanned| spanned.start)
    }

    fn error(&self, message: &str) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn expect(&mut self, token: &Token, message: &str) -> Result<(), ParseError> {
        if self.is(token) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    /// `statement`: returns the statement, or nothing for the statements that do not call the DB.
    fn statement(&mut self) -> Result<Option<Statement>, ParseError> {
        let offset = self.offset();
        let Some(token) = self.token().cloned() else {
            return Err(self.error("expected a statement"));
        };
        match token {
            Token::SpaceList(level) => {
                self.pos += 1;
                let level = i64::try_from(level).unwrap_or(i64::MAX);
                match self.token().cloned() {
                    Some(Token::NodeId(_) | Token::NodeStart(_)) => {
                        let (id, descr, node_type) = self.node()?;
                        Ok(Some(Statement::Node {
                            offset,
                            level,
                            id,
                            descr,
                            node_type,
                        }))
                    }
                    Some(Token::Icon(text)) => {
                        self.pos += 1;
                        Ok(Some(Statement::Icon { offset, text }))
                    }
                    Some(Token::Class(text)) => {
                        self.pos += 1;
                        Ok(Some(Statement::Class { offset, text }))
                    }
                    _ => Ok(None),
                }
            }
            Token::SpaceLine => {
                self.pos += 1;
                Ok(None)
            }
            Token::NodeId(_) | Token::NodeStart(_) => {
                let (id, descr, node_type) = self.node()?;
                Ok(Some(Statement::Node {
                    offset,
                    level: 0,
                    id,
                    descr,
                    node_type,
                }))
            }
            Token::Icon(text) => {
                self.pos += 1;
                Ok(Some(Statement::Icon { offset, text }))
            }
            Token::Class(text) => {
                self.pos += 1;
                Ok(Some(Statement::Class { offset, text }))
            }
            _ => Err(self.error("expected a statement")),
        }
    }

    /// `node`: `NODE_ID`, or `NODE_ID NODE_DSTART NODE_DESCR NODE_DEND`, or
    /// `NODE_DSTART NODE_DESCR NODE_DEND`. Returns the id, the description and the shape.
    fn node(&mut self) -> Result<(String, String, u8), ParseError> {
        match self.token().cloned() {
            Some(Token::NodeId(id)) => {
                self.pos += 1;
                if let Some(Token::NodeStart(start)) = self.token().cloned() {
                    self.pos += 1;
                    let descr = self.node_descr()?;
                    let end = self.node_end()?;
                    Ok((id, descr, node_type(&start, &end)))
                } else {
                    Ok((id.clone(), id, db::DEFAULT))
                }
            }
            Some(Token::NodeStart(start)) => {
                self.pos += 1;
                let descr = self.node_descr()?;
                let end = self.node_end()?;
                Ok((descr.clone(), descr, node_type(&start, &end)))
            }
            _ => Err(self.error("expected a node")),
        }
    }

    fn node_descr(&mut self) -> Result<String, ParseError> {
        match self.token().cloned() {
            Some(Token::NodeDescr(descr)) => {
                self.pos += 1;
                Ok(descr)
            }
            _ => Err(self.error("expected the text of the node")),
        }
    }

    fn node_end(&mut self) -> Result<String, ParseError> {
        match self.token().cloned() {
            Some(Token::NodeEnd(end)) => {
                self.pos += 1;
                Ok(end)
            }
            _ => Err(self.error("expected the end of the node")),
        }
    }

    /// `stop`: one of a line break, the end, or a `SPACELINE`, then any number of line breaks and
    /// ends.
    fn stop(&mut self) -> Result<(), ParseError> {
        if !(self.is(&Token::Newline) || self.is(&Token::Eof) || self.is(&Token::SpaceLine)) {
            return Err(self.error("expected the end of the line"));
        }
        self.pos += 1;
        while self.is(&Token::Newline) || self.is(&Token::Eof) {
            self.pos += 1;
        }
        Ok(())
    }
}
