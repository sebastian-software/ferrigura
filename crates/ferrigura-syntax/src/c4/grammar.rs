//! The C4 grammar, `c4Diagram.jison` lines 196-322, as a recursive descent over the scanner's
//! tokens. Each statement calls the DB when the grammar reduces it, in document order, and a
//! boundary calls its DB function before its body and closes it after the `}`.
//!
//! The statements follow the grammar's rules: `statements` is a list of other and diagram
//! statements, where two statements are separated by one line break, except that an other
//! statement may be followed by a diagram statement on the same line (an accessibility block
//! `accDescr { … }` can end mid-line). A boundary's body starts with a diagram statement, and its
//! `{` needs a line break before it or after it.

use crate::error::{ParseError, Source};
use crate::text::js_trim;

use super::db::{Arg, Db, DbError};
use super::scanner::{BoundaryKind, Macro, Spanned, Token};

/// The kind of a statement, which decides the separator that may come before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `otherStatement`: a title or an accessibility text.
    Other,
    /// `diagramStatement`: an element, a boundary, a relation or an update.
    Diagram,
}

/// The token that ends a list of statements: the end of the text, or the `}` of a boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closer {
    Eof,
    RBrace,
}

impl Closer {
    fn matches(self, token: &Token) -> bool {
        match self {
            Closer::Eof => *token == Token::Eof,
            Closer::RBrace => *token == Token::RBrace,
        }
    }
}

/// Parses the tokens of a C4 text, which the scanner read from `source`, into the DB.
pub(crate) fn parse(source: Source<'_>, tokens: &[Spanned], db: &mut Db) -> Result<(), ParseError> {
    let mut parser = Parser {
        source,
        tokens,
        index: 0,
        db,
    };
    // `graphConfig: C4_* NEWLINE statements EOF`.
    let header = match &parser.bump().token {
        Token::Header(text) => *text,
        _ => {
            return Err(parser.error_at(parser.previous(), "expected a C4 header"));
        }
    };
    if parser.peek().token != Token::Newline {
        return Err(parser.error_at(parser.peek(), "expected a line break after the header"));
    }
    parser.index += 1;
    parser.items(Closer::Eof, false)?;
    parser.index += 1;
    parser.db.set_c4_type(header);
    Ok(())
}

struct Parser<'a, 'db> {
    source: Source<'a>,
    tokens: &'a [Spanned],
    index: usize,
    db: &'db mut Db,
}

impl<'a> Parser<'a, '_> {
    /// The token at the cursor. The scanner ends the list with an end token, which is returned
    /// again when the cursor goes past it.
    fn peek(&self) -> &'a Spanned {
        self.tokens
            .get(self.index)
            .or_else(|| self.tokens.last())
            .unwrap_or(&END)
    }

    /// The token the cursor is past, which is the one just taken.
    fn previous(&self) -> &'a Spanned {
        self.tokens
            .get(self.index.saturating_sub(1))
            .unwrap_or(&END)
    }

    /// Takes the token at the cursor.
    fn bump(&mut self) -> &'a Spanned {
        let spanned = self.peek();
        if self.index < self.tokens.len() {
            self.index += 1;
        }
        spanned
    }

    fn error_at(&self, spanned: &Spanned, message: &str) -> ParseError {
        self.source.error(spanned.start, message)
    }

    fn unexpected(&self, spanned: &Spanned, expected: &str) -> ParseError {
        let found = describe(&spanned.token);
        self.source
            .error(spanned.start, format!("expected {expected}, found {found}"))
    }

    /// `statements`, or the body of a boundary when `diagram_first` is set, up to the closer. The
    /// closer is left for the caller to take.
    fn items(&mut self, closer: Closer, diagram_first: bool) -> Result<(), ParseError> {
        let mut previous: Option<Kind> = None;
        let mut newline = false;
        loop {
            let spanned = self.peek();
            match &spanned.token {
                Token::Newline if previous.is_some() => {
                    newline = true;
                    self.index += 1;
                }
                token if closer.matches(token) => {
                    return match previous {
                        Some(_) => Ok(()),
                        None => Err(self.unexpected(spanned, "a statement")),
                    };
                }
                token => {
                    let Some(kind) = statement_kind(token) else {
                        return Err(self.unexpected(spanned, "a statement"));
                    };
                    if diagram_first && previous.is_none() && kind != Kind::Diagram {
                        return Err(self.unexpected(spanned, "a diagram statement"));
                    }
                    if let Some(before) = previous {
                        // An other statement may be followed by a diagram statement on the same
                        // line, which only a multi-line accessibility block allows.
                        let same_line_allowed = before == Kind::Other && kind == Kind::Diagram;
                        if !newline && !same_line_allowed {
                            return Err(self.unexpected(spanned, "a line break"));
                        }
                    }
                    match kind {
                        Kind::Other => self.other_statement()?,
                        Kind::Diagram => self.diagram_statement()?,
                    }
                    previous = Some(kind);
                    newline = false;
                }
            }
        }
    }

    /// `otherStatement`: the title and the accessibility texts. Each calls its DB setter.
    fn other_statement(&mut self) -> Result<(), ParseError> {
        let spanned = self.bump();
        match &spanned.token {
            // `setTitle(substring(6))`: the value as the lexer took it.
            Token::Title(value) => self.db.set_title(value),
            // `setAccDescription(substring(15))`: not trimmed.
            Token::AccDescription(value) => self.db.set_acc_description(value),
            Token::AccTitle => {
                let value = self.value_after(spanned, "accTitle")?;
                self.db.set_title(js_trim(&value));
            }
            Token::AccDescr => {
                let value = self.value_after(spanned, "accDescr")?;
                self.db.set_acc_description(js_trim(&value));
            }
            Token::AccDescrMultiValue(value) => self.db.set_acc_description(js_trim(value)),
            _ => return Err(self.unexpected(spanned, "a statement")),
        }
        Ok(())
    }

    /// The value token after an accessibility keyword, which the scanner always emits.
    fn value_after(&mut self, keyword: &Spanned, name: &str) -> Result<String, ParseError> {
        let spanned = self.bump();
        match &spanned.token {
            Token::AccTitleValue(value) | Token::AccDescrValue(value) => Ok(value.clone()),
            _ => Err(self.unexpected(keyword, &format!("the text of {name}"))),
        }
    }

    /// `diagramStatement`: a keyword and its attributes, then the DB call of its macro. A boundary
    /// or a node also has a body.
    fn diagram_statement(&mut self) -> Result<(), ParseError> {
        let keyword = self.bump();
        let Token::Keyword(macro_kind) = keyword.token else {
            return Err(self.unexpected(keyword, "a keyword"));
        };
        let args = self.attributes()?;
        let outcome = match macro_kind {
            Macro::PersonOrSystem(kind) => self.db.add_person_or_system(kind, &args),
            Macro::Container(kind) | Macro::Component(kind) => self.db.add_container(kind, &args),
            Macro::Boundary(kind) => return self.boundary(keyword, kind, args),
            Macro::Rel(kind) => self.db.add_rel(kind, &args),
            // `$2.splice(0, 1)`: the index is dropped, and the relation is a plain one.
            Macro::RelIndex => self.db.add_rel("rel", args.get(1..).unwrap_or(&[])),
            Macro::UpdateElementStyle => self.db.update_el_style(&args),
            Macro::UpdateRelStyle => self.db.update_rel_style(&args),
            Macro::UpdateLayoutConfig => {
                self.db.update_layout_config(&args);
                Ok(())
            }
        };
        outcome.map_err(|DbError(message)| self.source.error(keyword.start, message))
    }

    /// `boundaryStatement`: the start (a DB call), the braces, the body, and the `}` that pops the
    /// boundary. The keyword that splices a kind in passes it as the third argument.
    fn boundary(
        &mut self,
        keyword: &Spanned,
        kind: BoundaryKind,
        args: Vec<Arg>,
    ) -> Result<(), ParseError> {
        let outcome = match kind {
            BoundaryKind::Plain => self.db.add_person_or_system_boundary(&args),
            BoundaryKind::Enterprise => self
                .db
                .add_person_or_system_boundary(&splice(args, "ENTERPRISE")),
            BoundaryKind::System => self
                .db
                .add_person_or_system_boundary(&splice(args, "SYSTEM")),
            BoundaryKind::Container => self.db.add_container_boundary(&splice(args, "CONTAINER")),
            BoundaryKind::Node(node_type) => self.db.add_deployment_node(node_type, &args),
        };
        outcome.map_err(|DbError(message)| self.source.error(keyword.start, message))?;
        self.boundary_body()?;
        self.db.pop_boundary();
        Ok(())
    }

    /// `boundaryStartStatement`, the body, and `boundaryStopStatement`. The `{` needs a line break
    /// before it or after it, and the body is a diagram statement list.
    fn boundary_body(&mut self) -> Result<(), ParseError> {
        let newline_before = self.eat(&Token::Newline);
        if self.peek().token != Token::LBrace {
            return Err(self.unexpected(self.peek(), "'{'"));
        }
        self.index += 1;
        let newline_after = self.eat(&Token::Newline);
        if !newline_before && !newline_after {
            return Err(self.unexpected(self.peek(), "a line break around '{'"));
        }
        self.items(Closer::RBrace, true)?;
        self.index += 1;
        Ok(())
    }

    /// Takes the token if it is `expected`, and says whether it did.
    fn eat(&mut self, expected: &Token) -> bool {
        if self.peek().token == *expected {
            self.index += 1;
            true
        } else {
            false
        }
    }

    /// `attributes`: one or more attributes, each a text, a `$key="value"`, or an empty one.
    fn attributes(&mut self) -> Result<Vec<Arg>, ParseError> {
        let mut args = Vec::new();
        loop {
            let spanned = self.peek();
            match &spanned.token {
                Token::Str(text) => {
                    self.index += 1;
                    args.push(Arg::Text(js_trim(text).to_owned()));
                }
                Token::AttrEmpty => {
                    self.index += 1;
                    args.push(Arg::Text(String::new()));
                }
                Token::StrKey(key) => {
                    self.index += 1;
                    let value = self.bump();
                    match &value.token {
                        Token::StrValue(text) => args.push(Arg::Named(
                            js_trim(key).to_owned(),
                            js_trim(text).to_owned(),
                        )),
                        _ => return Err(self.unexpected(value, "the value of an attribute")),
                    }
                }
                _ => break,
            }
        }
        if args.is_empty() {
            return Err(self.unexpected(self.peek(), "an attribute"));
        }
        Ok(args)
    }
}

/// The kind a token starts a statement with, if any.
fn statement_kind(token: &Token) -> Option<Kind> {
    match token {
        Token::Title(_)
        | Token::AccDescription(_)
        | Token::AccTitle
        | Token::AccDescr
        | Token::AccDescrMultiValue(_) => Some(Kind::Other),
        Token::Keyword(_) => Some(Kind::Diagram),
        _ => None,
    }
}

/// `Array.prototype.splice(2, 0, kind)`: the kind goes in as the third argument, or at the end
/// when there are fewer than two.
fn splice(mut args: Vec<Arg>, kind: &str) -> Vec<Arg> {
    let at = args.len().min(2);
    args.insert(at, Arg::Text(kind.to_owned()));
    args
}

/// A token as the error message names it.
fn describe(token: &Token) -> &'static str {
    match token {
        Token::Eof | Token::RawEof => "the end of the text",
        Token::EofInStruct => "the end of the text inside a statement",
        Token::Newline => "a line break",
        Token::Invalid => "a character that no rule takes",
        Token::Direction => "a direction line",
        Token::LBrace => "'{'",
        Token::RBrace => "'}'",
        Token::Header(_) => "a C4 header",
        Token::Keyword(_) => "a keyword",
        _ => "an unexpected text",
    }
}

/// The token that stands in for a missing one: the scanner always ends the list with an end token.
static END: Spanned = Spanned {
    token: Token::RawEof,
    start: 0,
};
