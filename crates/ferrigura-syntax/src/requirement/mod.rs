//! The requirement diagram parser (P2): `requirementDiagram.jison` and `requirementDb.ts`, ported
//! by hand. The plan, the rule table and the quirks are in `docs-port/requirement/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar is read as a
//! recursive descent over the tokens. Each production makes its DB calls where Jison's actions run
//! in document order, and a block's properties are collected first, as the actions of its body run
//! before the block's own action.

mod db;
mod scanner;

use ferrigura_model::requirement::{RelationshipKind, RequirementDiagram, RequirementKind};

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::js_trim;

use db::{ElementFields, RequirementDb, RequirementFields, set_first};
use scanner::{Spanned, Token, scan};

/// Parses a requirement diagram and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<RequirementDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let mut db = RequirementDb::default();
    // `Diagram.fromText` sets the front matter title before the parse.
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    let tokens = scan(source)?;
    Parser {
        source,
        tokens,
        pos: 0,
        db: &mut db,
    }
    .document()?;
    Ok(db.into_model())
}

/// The grammar's productions as functions. `pos` is the next token; the last token is always an
/// end token, which no production consumes past.
struct Parser<'a, 'd> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
    db: &'d mut RequirementDb,
}

impl Parser<'_, '_> {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    /// Consumes the next token and returns it; the end token stays where it is.
    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos].token.clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    /// An error at the next token.
    fn error_here(&self, message: &str) -> ParseError {
        self.source.error(
            self.tokens[self.pos].start,
            format!("unexpected {}: {message}", describe(self.peek())),
        )
    }

    /// Consumes the token `expected`, or fails.
    fn expect(&mut self, expected: &Token, message: &str) -> Result<(), ParseError> {
        if self.peek() == expected {
            self.bump();
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    /// The name of a requirement, an element, a relationship end or a class: `unqString | qString`.
    fn name(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            Token::Word(text) | Token::Quoted(text) => {
                self.bump();
                Ok(text)
            }
            _ => Err(self.error_here("expected a name")),
        }
    }

    /// `ID COLONSEP id NEWLINE`: the value after the colon, and the line break that ends it.
    fn property_value(&mut self) -> Result<String, ParseError> {
        self.expect(&Token::ColonSep, "expected ':'")?;
        let value = self.name()?;
        self.expect(&Token::Newline, "expected the end of the line")?;
        Ok(value)
    }

    /// `direction`, `directive` and the header: the accessibility productions set the DB at once.
    fn directive(&mut self) -> Result<(), ParseError> {
        match self.peek().clone() {
            Token::AccTitle => {
                self.bump();
                let value = self.accessibility_value("expected the text of accTitle")?;
                self.db.set_acc_title(js_trim(&value));
            }
            Token::AccDescr => {
                self.bump();
                let value = self.accessibility_value("expected the text of accDescr")?;
                self.db.set_acc_description(js_trim(&value));
            }
            Token::AccDescrMultiValue(value) => {
                self.bump();
                self.db.set_acc_description(js_trim(&value));
            }
            _ => return Err(self.error_here("expected an accessibility statement")),
        }
        Ok(())
    }

    fn accessibility_value(&mut self, message: &str) -> Result<String, ParseError> {
        match self.peek().clone() {
            Token::AccTitleValue(value) | Token::AccDescrValue(value) => {
                self.bump();
                Ok(value)
            }
            _ => Err(self.error_here(message)),
        }
    }

    /// `start: directive NEWLINE start | directive start | RD NEWLINE diagram EOF`.
    fn document(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Token::AccTitle | Token::AccDescr | Token::AccDescrMultiValue(_) => {
                    self.directive()?;
                    if self.peek() == &Token::Newline {
                        self.bump();
                    }
                }
                Token::Rd => break,
                _ => return Err(self.error_here("expected 'requirementDiagram'")),
            }
        }
        self.bump();
        self.expect(
            &Token::Newline,
            "expected a line break after 'requirementDiagram'",
        )?;
        self.diagram()
    }

    /// `diagram`: the statements up to the end of the input.
    fn diagram(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek().clone() {
                Token::Eof => {
                    self.bump();
                    return Ok(());
                }
                Token::Newline => {
                    self.bump();
                }
                Token::AccTitle | Token::AccDescr | Token::AccDescrMultiValue(_) => {
                    self.directive()?;
                }
                Token::Direction(direction) => {
                    self.bump();
                    self.db.set_direction(direction);
                }
                Token::Kind(kind) => self.requirement_def(kind)?,
                Token::ElementKw => self.element_def()?,
                Token::Word(_) | Token::Quoted(_) => self.relationship_or_class()?,
                Token::StyleKw => self.style_statement()?,
                Token::ClassDefKw => self.class_def_statement()?,
                Token::ClassKw => self.class_statement()?,
                _ => return Err(self.error_here("expected a statement")),
            }
        }
    }

    /// `requirementDef`: the shorthand `:::` adds the class after the requirement.
    fn requirement_def(&mut self, kind: RequirementKind) -> Result<(), ParseError> {
        self.bump();
        let name = self.name()?;
        match self.peek() {
            Token::StructStart => {
                self.bump();
                let fields = self.block_body(Self::requirement_body)?;
                self.db.add_requirement(name, kind, fields);
            }
            Token::StyleSeparator => {
                self.bump();
                let classes = self.id_list()?;
                self.expect(&Token::StructStart, "expected '{'")?;
                let fields = self.block_body(Self::requirement_body)?;
                self.db.add_requirement(name.clone(), kind, fields);
                self.db.set_class(std::slice::from_ref(&name), &classes);
            }
            _ => return Err(self.error_here("expected '{' or ':::'")),
        }
        Ok(())
    }

    /// `elementDef`: the same shape as a requirement.
    fn element_def(&mut self) -> Result<(), ParseError> {
        self.bump();
        let name = self.name()?;
        match self.peek() {
            Token::StructStart => {
                self.bump();
                let fields = self.block_body(Self::element_body)?;
                self.db.add_element(name, fields);
            }
            Token::StyleSeparator => {
                self.bump();
                let classes = self.id_list()?;
                self.expect(&Token::StructStart, "expected '{'")?;
                let fields = self.block_body(Self::element_body)?;
                self.db.add_element(name.clone(), fields);
                self.db.set_class(std::slice::from_ref(&name), &classes);
            }
            _ => return Err(self.error_here("expected '{' or ':::'")),
        }
        Ok(())
    }

    /// The line break after `{`, then the body up to its `}`.
    fn block_body<F, T>(&mut self, body: F) -> Result<T, ParseError>
    where
        F: FnOnce(&mut Self) -> Result<T, ParseError>,
    {
        self.expect(&Token::Newline, "expected a line break after '{'")?;
        body(self)
    }

    /// `requirementBody`: properties, each on its own line, then `}`.
    fn requirement_body(&mut self) -> Result<RequirementFields, ParseError> {
        let mut fields = RequirementFields::default();
        loop {
            match self.peek().clone() {
                Token::IdKw => {
                    self.bump();
                    let value = self.property_value()?;
                    set_first(&mut fields.requirement_id, value);
                }
                Token::TextKw => {
                    self.bump();
                    let value = self.property_value()?;
                    set_first(&mut fields.text, value);
                }
                Token::RiskKw => {
                    self.bump();
                    self.expect(&Token::ColonSep, "expected ':'")?;
                    let level = self.level()?;
                    self.expect(&Token::Newline, "expected the end of the line")?;
                    set_first(&mut fields.risk, level);
                }
                Token::VerifyMethodKw => {
                    self.bump();
                    self.expect(&Token::ColonSep, "expected ':'")?;
                    let method = self.verify()?;
                    self.expect(&Token::Newline, "expected the end of the line")?;
                    set_first(&mut fields.verify_method, method);
                }
                Token::Newline => {
                    self.bump();
                }
                Token::StructStop => {
                    self.bump();
                    return Ok(fields);
                }
                _ => return Err(self.error_here("expected a property or '}'")),
            }
        }
    }

    /// `elementBody`: `type` and `docref` properties, then `}`.
    fn element_body(&mut self) -> Result<ElementFields, ParseError> {
        let mut fields = ElementFields::default();
        loop {
            match self.peek().clone() {
                Token::TypeKw => {
                    self.bump();
                    let value = self.property_value()?;
                    set_first(&mut fields.kind, value);
                }
                Token::DocRefKw => {
                    self.bump();
                    let value = self.property_value()?;
                    set_first(&mut fields.doc_ref, value);
                }
                Token::Newline => {
                    self.bump();
                }
                Token::StructStop => {
                    self.bump();
                    return Ok(fields);
                }
                _ => return Err(self.error_here("expected a property or '}'")),
            }
        }
    }

    fn level(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            Token::Level(level) => {
                self.bump();
                Ok(level.to_owned())
            }
            _ => Err(self.error_here("expected low, medium or high")),
        }
    }

    fn verify(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            Token::Verify(method) => {
                self.bump();
                Ok(method.to_owned())
            }
            _ => Err(self.error_here("expected analysis, demonstration, inspection or test")),
        }
    }

    fn relation(&mut self) -> Result<RelationshipKind, ParseError> {
        match self.peek().clone() {
            Token::Relation(kind) => {
                self.bump();
                Ok(kind)
            }
            _ => Err(self.error_here("expected a relationship keyword")),
        }
    }

    /// `relationshipDef`, or the shorthand `id:::classes`, which begin with the same name.
    fn relationship_or_class(&mut self) -> Result<(), ParseError> {
        let first = self.name()?;
        match self.peek() {
            Token::EndArrowL => {
                self.bump();
                let kind = self.relation()?;
                self.expect(&Token::Line, "expected '-'")?;
                let second = self.name()?;
                // `X <- T - Y` is `Y T X`: the source is the name after the line.
                self.db.add_relationship(kind, second, first);
            }
            Token::Line => {
                self.bump();
                let kind = self.relation()?;
                self.expect(&Token::EndArrowR, "expected '->'")?;
                let second = self.name()?;
                self.db.add_relationship(kind, first, second);
            }
            Token::StyleSeparator => {
                self.bump();
                let classes = self.id_list()?;
                self.db.set_class(std::slice::from_ref(&first), &classes);
            }
            _ => return Err(self.error_here("expected a relationship or ':::'")),
        }
        Ok(())
    }

    /// `idList`: names separated by commas. In a style the names are words (`ALPHA`), elsewhere
    /// they are names; both are accepted in either place, as the grammar's alternatives allow.
    fn id_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut ids = vec![self.list_id()?];
        while self.peek() == &Token::Comma {
            self.bump();
            ids.push(self.list_id()?);
        }
        Ok(ids)
    }

    fn list_id(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            Token::Alpha(text) | Token::Word(text) | Token::Quoted(text) => {
                self.bump();
                Ok(text)
            }
            _ => Err(self.error_here("expected a name")),
        }
    }

    /// `stylesOpt`: one or more styles separated by commas.
    fn styles_opt(&mut self) -> Result<Vec<String>, ParseError> {
        let mut styles = vec![self.style()?];
        while self.peek() == &Token::Comma {
            self.bump();
            styles.push(self.style()?);
        }
        Ok(styles)
    }

    /// `style`: one or more style components, concatenated with no spaces (`fill: red` is
    /// `fill:red`).
    fn style(&mut self) -> Result<String, ParseError> {
        let mut text = String::new();
        loop {
            let piece = match self.peek() {
                Token::Alpha(word) => word.clone(),
                Token::StyleColon => ":".to_owned(),
                Token::Semicolon => ";".to_owned(),
                Token::StyleMinus => "-".to_owned(),
                Token::StyleHash => "#".to_owned(),
                _ => break,
            };
            text.push_str(&piece);
            self.bump();
        }
        if text.is_empty() {
            return Err(self.error_here("expected a style"));
        }
        Ok(text)
    }

    /// `styleStatement`: `style idList stylesOpt`.
    fn style_statement(&mut self) -> Result<(), ParseError> {
        self.bump();
        let ids = self.id_list()?;
        let styles = self.styles_opt()?;
        self.db.set_css_style(&ids, &styles);
        Ok(())
    }

    /// `classDefStatement`: `classDef idList stylesOpt`.
    fn class_def_statement(&mut self) -> Result<(), ParseError> {
        self.bump();
        let ids = self.id_list()?;
        let styles = self.styles_opt()?;
        self.db.define_class(&ids, &styles);
        Ok(())
    }

    /// `classStatement`: `class idList idList`.
    fn class_statement(&mut self) -> Result<(), ParseError> {
        self.bump();
        let nodes = self.id_list()?;
        let classes = self.id_list()?;
        self.db.set_class(&nodes, &classes);
        Ok(())
    }
}

/// A short description of a token for an error message.
fn describe(token: &Token) -> String {
    match token {
        Token::Eof => "end of the input".to_owned(),
        Token::RawEof => "end of the input".to_owned(),
        Token::Newline => "line break".to_owned(),
        Token::Word(text) | Token::Quoted(text) | Token::Alpha(text) => format!("'{text}'"),
        other => format!("{other:?}").to_lowercase(),
    }
}
