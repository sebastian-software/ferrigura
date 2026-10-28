//! The class grammar of `parser/classDiagram.jison` (its productions, L221–438), as recursive
//! descent over the tokens of [`super::lexer`]. Each DB call is made where the Jison reduction
//! makes it, so the order of the DB's state (class order, namespace order) is the reference's.
//!
//! The grammar is LALR(1) with one token of lookahead. Where a production starts with a name, the
//! token after the name decides the statement: a label is a member, a relation token or a quoted
//! string starts a relation, and anything else is a name with no effect.

use ferrigura_model::class::{ClassDiagram, LineType, RelationEnd};

use super::db::{ClassDb, DbError, RawRelation};
use super::lexer::{Kind, Token, tokenize};
use crate::error::{ParseError, Source};
use crate::text::js_trim;

/// Parses the prepared text of a class diagram into the DB's state, and returns the model.
pub(crate) fn parse(
    text: &str,
    line_offset: usize,
    db: ClassDb,
) -> Result<ClassDiagram, ParseError> {
    let source = Source::new(text, line_offset);
    let tokens = tokenize(text).map_err(|error| source.error(error.offset, "Unrecognized text"))?;
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
        db,
    };
    parser.graph_config()?;
    Ok(parser.db.into_model())
}

/// The state of the recursive descent: the tokens, the position, and the DB the actions update.
struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Token>,
    pos: usize,
    db: ClassDb,
}

impl Parser<'_> {
    fn kind(&self) -> Kind {
        self.tokens
            .get(self.pos)
            .map_or(Kind::Eof, |token| token.kind)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or_else(|| self.source.text().len(), |token| token.start)
    }

    /// Consumes the next token and returns it. The lexer ends the list with `Eof` and nothing
    /// reads past it, so the fallback is never used.
    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        token.unwrap_or(Token {
            kind: Kind::Eof,
            text: String::new(),
            start: self.source.text().len(),
        })
    }

    fn error(&self, message: &str) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn db_error(&self, error: DbError) -> ParseError {
        self.source.error(self.offset(), error.0)
    }

    /// Consumes a token of `kind` or fails.
    fn expect(&mut self, kind: Kind, what: &str) -> Result<Token, ParseError> {
        if self.kind() == kind {
            Ok(self.advance())
        } else {
            Err(self.error(&format!("expected {what}")))
        }
    }

    /// `graphConfig`: `CLASS_DIAGRAM NEWLINE statements EOF`.
    fn graph_config(&mut self) -> Result<(), ParseError> {
        if self.kind() != Kind::ClassDiagram {
            return Err(self.error("expected 'classDiagram'"));
        }
        self.advance();
        self.expect(Kind::Newline, "a line break after the header")?;
        self.statements()?;
        self.expect(Kind::Eof, "the end of the diagram")?;
        Ok(())
    }

    /// `statements`: statements separated by line breaks, with one break allowed at the end.
    fn statements(&mut self) -> Result<(), ParseError> {
        loop {
            self.statement()?;
            match self.kind() {
                Kind::Newline => {
                    self.advance();
                    if self.kind() == Kind::Eof {
                        return Ok(());
                    }
                }
                Kind::Eof => return Ok(()),
                _ => return Err(self.error("expected the end of the line")),
            }
        }
    }

    /// `statement`: one of the productions that start a line.
    fn statement(&mut self) -> Result<(), ParseError> {
        match self.kind() {
            Kind::Namespace => self.namespace_statement(),
            Kind::Class => self.class_statement().map(|_| ()),
            Kind::AnnotationStart => self.annotation_statement(),
            Kind::Callback | Kind::Link | Kind::Click => self.click_statement(),
            Kind::Style => self.style_statement(),
            Kind::CssClass => self.css_class_statement(),
            Kind::NoteFor | Kind::Note => self.note_statement().map(|_| ()),
            Kind::ClassDef => self.class_def_statement(),
            Kind::DirectionTb | Kind::DirectionBt | Kind::DirectionRl | Kind::DirectionLr => {
                let direction = match self.advance().kind {
                    Kind::DirectionTb => "TB",
                    Kind::DirectionBt => "BT",
                    Kind::DirectionRl => "RL",
                    _ => "LR",
                };
                self.db.set_direction(direction);
                Ok(())
            }
            Kind::AccTitle => {
                self.advance();
                let value = self.expect(Kind::AccTitleValue, "the title")?;
                self.db.set_acc_title(js_trim(&value.text));
                Ok(())
            }
            Kind::AccDescr => {
                self.advance();
                let value = self.expect(Kind::AccDescrValue, "the description")?;
                self.db.set_acc_descr(js_trim(&value.text));
                Ok(())
            }
            Kind::AccDescrMultiValue => {
                let value = self.advance();
                self.db.set_acc_descr(js_trim(&value.text));
                Ok(())
            }
            Kind::Alpha | Kind::UnicodeText | Kind::Minus | Kind::BqStr => self.name_statement(),
            _ => Err(self.error("unexpected token")),
        }
    }

    /// The statements that start with a name: a member label, a relation, or a bare name.
    fn name_statement(&mut self) -> Result<(), ParseError> {
        let name = self.class_name()?;
        match self.kind() {
            Kind::Label => {
                let label = self.advance();
                let cleaned = self.db.clean_up_label(&label.text);
                self.db
                    .add_member(&name, &cleaned)
                    .map_err(|error| self.db_error(error))
            }
            Kind::Str
            | Kind::Aggregation
            | Kind::Extension
            | Kind::Composition
            | Kind::Dependency
            | Kind::Lollipop
            | Kind::Line
            | Kind::DottedLine => self.relation_statement(name),
            _ => Ok(()),
        }
    }

    /// `relationStatement` with the `LABEL` of the statement that follows it.
    fn relation_statement(&mut self, id1: String) -> Result<(), ParseError> {
        let title1 = self.optional_string().unwrap_or_else(|| "none".to_owned());
        let (type1, line_type, type2) = self.relation()?;
        let title2 = self.optional_string().unwrap_or_else(|| "none".to_owned());
        let id2 = self.class_name()?;
        let title = if self.kind() == Kind::Label {
            let label = self.advance();
            Some(self.db.clean_up_label(&label.text))
        } else {
            None
        };
        self.db.add_relation(RawRelation {
            id1,
            id2,
            type1,
            type2,
            line_type,
            title1,
            title2,
            title,
        });
        Ok(())
    }

    /// `relation`: an optional marker, a line, and an optional marker.
    fn relation(&mut self) -> Result<(RelationEnd, LineType, RelationEnd), ParseError> {
        let type1 = self.optional_relation_end().unwrap_or(RelationEnd::None);
        let line_type = match self.kind() {
            Kind::Line => LineType::Solid,
            Kind::DottedLine => LineType::Dotted,
            _ => return Err(self.error("expected a relation line")),
        };
        self.advance();
        let type2 = self.optional_relation_end().unwrap_or(RelationEnd::None);
        Ok((type1, line_type, type2))
    }

    /// A marker at the end of a relation, if the next token is one.
    fn optional_relation_end(&mut self) -> Option<RelationEnd> {
        let end = match self.kind() {
            Kind::Aggregation => RelationEnd::Aggregation,
            Kind::Extension => RelationEnd::Extension,
            Kind::Composition => RelationEnd::Composition,
            Kind::Dependency => RelationEnd::Dependency,
            Kind::Lollipop => RelationEnd::Lollipop,
            _ => return None,
        };
        self.advance();
        Some(end)
    }

    /// A quoted string, if the next token is one.
    fn optional_string(&mut self) -> Option<String> {
        (self.kind() == Kind::Str).then(|| self.advance().text)
    }

    /// `className` (with generics) and `namespaceName` (without): a chain of names joined by
    /// dots, with the generic type of the last piece.
    fn class_name(&mut self) -> Result<String, ParseError> {
        self.name_chain(true)
    }

    fn name_chain(&mut self, generics: bool) -> Result<String, ParseError> {
        let mut name = String::new();
        loop {
            match self.kind() {
                Kind::Alpha | Kind::UnicodeText | Kind::Minus => {
                    name.push_str(&self.advance().text);
                    match self.kind() {
                        Kind::Dot => {
                            self.advance();
                            name.push('.');
                        }
                        Kind::GenericType if generics => {
                            self.push_generic(&mut name);
                            return Ok(name);
                        }
                        Kind::Alpha | Kind::UnicodeText | Kind::Minus | Kind::BqStr => {}
                        _ => return Ok(name),
                    }
                }
                Kind::BqStr => {
                    name.push_str(&self.advance().text);
                    if generics && self.kind() == Kind::GenericType {
                        self.push_generic(&mut name);
                    }
                    return Ok(name);
                }
                _ => return Err(self.error("expected a class name")),
            }
        }
    }

    /// `~T~` after a name.
    fn push_generic(&mut self, name: &mut String) {
        let generic = self.advance().text;
        name.push('~');
        name.push_str(&generic);
        name.push('~');
    }

    /// `alphaNumToken`: a single word, unicode letters or a minus sign.
    fn alpha_num(&mut self) -> Result<String, ParseError> {
        match self.kind() {
            Kind::Alpha | Kind::UnicodeText | Kind::Minus => Ok(self.advance().text),
            _ => Err(self.error("expected a name")),
        }
    }

    /// `namespaceStatement`: the identifier, its braces and the classes and notes inside.
    fn namespace_statement(&mut self) -> Result<(), ParseError> {
        self.advance();
        let name = self.name_chain(false)?;
        let label = if self.kind() == Kind::Sqs {
            self.advance();
            let label = self.expect(Kind::Str, "a label")?.text;
            self.expect(Kind::Sqe, "']'")?;
            Some(label)
        } else {
            None
        };
        let namespace = self.db.add_namespace(&name, label.as_deref());
        self.expect(Kind::StructStart, "'{'")?;
        if self.kind() == Kind::Newline {
            self.advance();
        }
        let (classes, notes) = self.class_statements()?;
        self.expect(Kind::StructStop, "'}'")?;
        self.db
            .add_classes_to_namespace(&namespace, &classes, &notes)
            .map_err(|error| self.db_error(error))?;
        self.db.pop_namespace();
        Ok(())
    }

    /// `classStatements`: classes, notes and nested namespaces, separated by line breaks. The
    /// result lists the classes and the notes of this namespace; a nested namespace keeps its own.
    fn class_statements(&mut self) -> Result<(Vec<String>, Vec<String>), ParseError> {
        let mut classes = Vec::new();
        let mut notes = Vec::new();
        loop {
            match self.kind() {
                Kind::Class => classes.push(self.class_statement()?),
                Kind::NoteFor | Kind::Note => notes.push(self.note_statement()?),
                Kind::Namespace => self.namespace_statement()?,
                _ => return Err(self.error("expected a class, a note or a namespace")),
            }
            match self.kind() {
                Kind::Newline => {
                    self.advance();
                    if self.kind() == Kind::StructStop {
                        return Ok((classes, notes));
                    }
                }
                Kind::StructStop => return Ok((classes, notes)),
                _ => return Err(self.error("expected the end of the line")),
            }
        }
    }

    /// `classStatement`: the identifier with its optional label, and the css class, the members or
    /// the annotation that may follow it. Returns the name of the class.
    fn class_statement(&mut self) -> Result<String, ParseError> {
        self.advance();
        let name = self.class_name()?;
        let label = if self.kind() == Kind::Sqs {
            self.advance();
            let label = self.expect(Kind::Str, "a label")?.text;
            self.expect(Kind::Sqe, "']'")?;
            Some(label)
        } else {
            None
        };
        self.db.add_class(&name);
        if let Some(label) = label {
            self.db
                .set_class_label(&name, &label)
                .map_err(|error| self.db_error(error))?;
        }
        match self.kind() {
            Kind::StyleSeparator => {
                self.advance();
                let css = self.alpha_num()?;
                if self.kind() == Kind::StructStart {
                    let members = self.struct_members(true)?;
                    self.db.set_css_class(&name, &css);
                    self.add_members(&name, &members)?;
                } else {
                    self.db.set_css_class(&name, &css);
                }
            }
            Kind::StructStart => {
                let members = self.struct_members(false)?;
                self.add_members(&name, &members)?;
            }
            Kind::AnnotationStart => {
                self.advance();
                let annotation = self.alpha_num()?;
                self.expect(Kind::AnnotationEnd, "'>>'")?;
                self.db
                    .add_annotation(&name, &annotation)
                    .map_err(|error| self.db_error(error))?;
                if self.kind() == Kind::StructStart {
                    let members = self.struct_members(false)?;
                    self.add_members(&name, &members)?;
                }
            }
            _ => {}
        }
        Ok(name)
    }

    fn add_members(&mut self, name: &str, members: &[String]) -> Result<(), ParseError> {
        self.db
            .add_members(name, members)
            .map_err(|error| self.db_error(error))
    }

    /// `{ members }`: the member lines of a class body. `require_one` is set after `:::`, where
    /// the grammar has no empty body.
    fn struct_members(&mut self, require_one: bool) -> Result<Vec<String>, ParseError> {
        self.expect(Kind::StructStart, "'{'")?;
        let mut members = Vec::new();
        while self.kind() == Kind::Member {
            members.push(self.advance().text);
        }
        if require_one && members.is_empty() {
            return Err(self.error("expected a member"));
        }
        self.expect(Kind::StructStop, "'}'")?;
        Ok(members)
    }

    /// `annotationStatement`: `<<name>> className`.
    fn annotation_statement(&mut self) -> Result<(), ParseError> {
        self.advance();
        let annotation = self.alpha_num()?;
        self.expect(Kind::AnnotationEnd, "'>>'")?;
        let name = self.class_name()?;
        self.db
            .add_annotation(&name, &annotation)
            .map_err(|error| self.db_error(error))
    }

    /// `clickStatement`: a callback (`call`, or the `callback` keyword), a link or an `href`, with
    /// an optional tooltip and an optional link target.
    fn click_statement(&mut self) -> Result<(), ParseError> {
        let keyword = self.advance().kind;
        let id = self.class_name()?;
        match keyword {
            Kind::Callback => {
                self.expect(Kind::Str, "a callback")?;
                self.db.set_click_event(&id);
                if let Some(tooltip) = self.optional_string() {
                    self.db.set_tooltip(&id, &tooltip);
                }
                Ok(())
            }
            Kind::Link => {
                let url = self.expect(Kind::Str, "a link")?.text;
                self.link_tail(&id, &url)
            }
            _ => match self.kind() {
                Kind::CallbackName => {
                    self.advance();
                    self.db.set_click_event(&id);
                    if self.kind() == Kind::CallbackArgs {
                        self.advance();
                    }
                    if let Some(tooltip) = self.optional_string() {
                        self.db.set_tooltip(&id, &tooltip);
                    }
                    Ok(())
                }
                Kind::Href => {
                    self.advance();
                    let url = self.expect(Kind::Str, "a link")?.text;
                    self.link_tail(&id, &url)
                }
                _ => Err(self.error("expected a callback or a link")),
            },
        }
    }

    /// The end of a link: an optional tooltip, then an optional target. The link is set before the
    /// tooltip, as the reduction of the productions does.
    fn link_tail(&mut self, id: &str, url: &str) -> Result<(), ParseError> {
        let tooltip = self.optional_string();
        let target = if self.kind() == Kind::LinkTarget {
            Some(self.advance().text)
        } else {
            None
        };
        self.db.set_link(id, url, target.as_deref());
        if let Some(tooltip) = tooltip {
            self.db.set_tooltip(id, &tooltip);
        }
        Ok(())
    }

    /// `styleStatement`: `style ALPHA stylesOpt`.
    fn style_statement(&mut self) -> Result<(), ParseError> {
        self.advance();
        let id = self.expect(Kind::Alpha, "a class name")?.text;
        let styles = self.styles_opt()?;
        self.db.set_css_style(&id, &styles);
        Ok(())
    }

    /// `cssClassStatement`: `cssClass STR ALPHA`.
    fn css_class_statement(&mut self) -> Result<(), ParseError> {
        self.advance();
        let ids = self.expect(Kind::Str, "a quoted list of classes")?.text;
        let css = self.expect(Kind::Alpha, "a css class")?.text;
        self.db.set_css_class(&ids, &css);
        Ok(())
    }

    /// `noteStatement`: `note for className "text"` or `note "text"`. Returns the note's id.
    fn note_statement(&mut self) -> Result<String, ParseError> {
        let keyword = self.advance().kind;
        if keyword == Kind::NoteFor {
            let class = self.class_name()?;
            let text = self.expect(Kind::Str, "the text of the note")?.text;
            Ok(self.db.add_note(&text, Some(&class)))
        } else {
            let text = self.expect(Kind::Str, "the text of the note")?.text;
            Ok(self.db.add_note(&text, None))
        }
    }

    /// `classDefStatement`: `classDef ALPHA (COMMA ALPHA)* stylesOpt`.
    fn class_def_statement(&mut self) -> Result<(), ParseError> {
        self.advance();
        let mut ids = vec![self.expect(Kind::Alpha, "a style class name")?.text];
        while self.kind() == Kind::Comma {
            self.advance();
            ids.push(self.expect(Kind::Alpha, "a style class name")?.text);
        }
        let styles = self.styles_opt()?;
        self.db.define_class(&ids, &styles);
        Ok(())
    }

    /// `stylesOpt`: declarations separated by commas, each the text of its components.
    fn styles_opt(&mut self) -> Result<Vec<String>, ParseError> {
        let mut styles = vec![self.style()?];
        while self.kind() == Kind::Comma {
            self.advance();
            styles.push(self.style()?);
        }
        Ok(styles)
    }

    /// `style`: one or more components, joined as written.
    fn style(&mut self) -> Result<String, ParseError> {
        let mut out = String::new();
        while matches!(
            self.kind(),
            Kind::Alpha
                | Kind::Colon
                | Kind::Space
                | Kind::Brkt
                | Kind::Style
                | Kind::Pct
                | Kind::Label
        ) {
            out.push_str(&self.advance().text);
        }
        if out.is_empty() {
            return Err(self.error("expected a style"));
        }
        Ok(out)
    }
}
