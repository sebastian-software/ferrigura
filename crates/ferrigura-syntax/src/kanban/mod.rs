//! The kanban parser: `kanban.jison` and the DB `kanbanDb.ts`, ported by hand. The plan, the rule
//! table and the quirks are in `docs-port/kanban/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar then calls the DB in
//! document order, one call per statement, and the DB's state becomes the model.

mod db;
mod scanner;

use ferrigura_model::kanban::KanbanDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

use db::{DbError, KanbanDb};
use scanner::{Spanned, Token, scan};

/// Parses a kanban diagram and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input; a DB error is reported at the statement that raised it.
pub fn parse(raw: &str) -> Result<KanbanDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let tokens = scan(&text).map_err(|offset| source.error(offset, "unexpected character"))?;
    let mut db = KanbanDb::default();
    Grammar {
        tokens: &tokens,
        at: 0,
        source,
        db: &mut db,
    }
    .mind_map()?;
    Ok(db.into_model())
}

/// The grammar of `kanban.jison`, as a recursive descent over the tokens.
///
/// `start: spaceLines? mindMap`, `mindMap: KANBAN NL? document`, `document: (statement stop)+`,
/// `stop: (NL | EOF | SPACELINE) (NL | EOF)*`, and the statements of `kanban.jison:126-137`.
struct Grammar<'a, 'db> {
    tokens: &'a [Spanned],
    at: usize,
    source: Source<'a>,
    db: &'db mut KanbanDb,
}

impl<'a> Grammar<'a, '_> {
    /// The token at the cursor. The list ends with [`Token::End`], which nothing consumes.
    fn token(&self) -> &'a Token {
        let tokens: &'a [Spanned] = self.tokens;
        &tokens[self.at].token
    }

    fn start_of_token(&self) -> usize {
        self.tokens[self.at].start
    }

    fn unexpected(&self, message: &str) -> ParseError {
        self.source.error(self.start_of_token(), message)
    }

    fn mind_map(&mut self) -> Result<(), ParseError> {
        while matches!(self.token(), Token::SpaceLine | Token::Nl) {
            self.at += 1;
        }
        if !matches!(self.token(), Token::Kanban) {
            return Err(self.unexpected("expected 'kanban'"));
        }
        self.at += 1;
        if matches!(self.token(), Token::Nl) {
            self.at += 1;
        }
        self.document()
    }

    fn document(&mut self) -> Result<(), ParseError> {
        loop {
            self.statement()?;
            self.stop()?;
            if matches!(self.token(), Token::End) {
                return Ok(());
            }
        }
    }

    /// `stop: NL | EOF | SPACELINE | stop NL | stop EOF`.
    fn stop(&mut self) -> Result<(), ParseError> {
        if !matches!(self.token(), Token::Nl | Token::Eof | Token::SpaceLine) {
            return Err(self.unexpected("expected the end of the statement"));
        }
        self.at += 1;
        while matches!(self.token(), Token::Nl | Token::Eof) {
            self.at += 1;
        }
        Ok(())
    }

    /// The statements of `kanban.jison:126-137`. The DB call comes at the reduction, after the
    /// statement's last token.
    fn statement(&mut self) -> Result<(), ParseError> {
        let start = self.start_of_token();
        match self.token() {
            Token::SpaceList(level) => {
                self.at += 1;
                match self.token() {
                    Token::NodeId(_) | Token::NodeDStart => self.node_statement(*level, start),
                    Token::Icon(icon) => {
                        self.at += 1;
                        self.decorate_icon(icon, start)
                    }
                    Token::Class(class) => {
                        self.at += 1;
                        self.decorate_class(class, start)
                    }
                    // `SPACELIST` on its own.
                    _ => Ok(()),
                }
            }
            Token::SpaceLine => {
                self.at += 1;
                Ok(())
            }
            Token::NodeId(_) | Token::NodeDStart => self.node_statement(0, start),
            Token::Icon(icon) => {
                self.at += 1;
                self.decorate_icon(icon, start)
            }
            Token::Class(class) => {
                self.at += 1;
                self.decorate_class(class, start)
            }
            _ => Err(self.unexpected("expected a statement")),
        }
    }

    /// `node shapeData?` after an optional level.
    fn node_statement(&mut self, level: u32, start: usize) -> Result<(), ParseError> {
        let (id, descr) = self.node()?;
        let shape_data = self.shape_data();
        self.db
            .add_node(level, id, descr, shape_data.as_deref())
            .map_err(|DbError(message)| self.source.error(start, message))
    }

    /// `nodeWithId | nodeWithoutId`: the id and the description. A node without an id takes its
    /// description as its id (`kanban.jison:146-155`).
    fn node(&mut self) -> Result<(&'a str, &'a str), ParseError> {
        match self.token() {
            Token::NodeId(id) => {
                self.at += 1;
                if matches!(self.token(), Token::NodeDStart) {
                    self.at += 1;
                    let descr = self.node_descr()?;
                    self.node_dend()?;
                    Ok((id, descr))
                } else {
                    Ok((id, id))
                }
            }
            Token::NodeDStart => {
                self.at += 1;
                let descr = self.node_descr()?;
                self.node_dend()?;
                Ok((descr, descr))
            }
            _ => Err(self.unexpected("expected a node")),
        }
    }

    fn node_descr(&mut self) -> Result<&'a str, ParseError> {
        match self.token() {
            Token::NodeDescr(descr) => {
                self.at += 1;
                Ok(descr)
            }
            _ => Err(self.unexpected("expected the description of the node")),
        }
    }

    fn node_dend(&mut self) -> Result<(), ParseError> {
        if matches!(self.token(), Token::NodeDEnd) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.unexpected("expected the end of the node"))
        }
    }

    /// `shapeData: shapeData SHAPE_DATA | SHAPE_DATA`, concatenated. `None` when there is none.
    fn shape_data(&mut self) -> Option<String> {
        let mut text: Option<String> = None;
        while let Token::ShapeData(part) = self.token() {
            text.get_or_insert_with(String::new).push_str(part);
            self.at += 1;
        }
        text
    }

    fn decorate_icon(&mut self, icon: &str, start: usize) -> Result<(), ParseError> {
        self.db
            .decorate_icon(icon)
            .map_err(|DbError(message)| self.source.error(start, message))
    }

    fn decorate_class(&mut self, class: &str, start: usize) -> Result<(), ParseError> {
        self.db
            .decorate_class(class)
            .map_err(|DbError(message)| self.source.error(start, message))
    }
}
