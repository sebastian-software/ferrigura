//! The venn diagram parser (P3): `venn.jison` and the venn DB `vennDB.ts`, ported by hand. The plan,
//! the rules and the quirks are in `docs-port/venn/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar's actions then call
//! the DB in document order, and the DB's state is the model.

mod db;
mod scanner;

use ferrigura_model::venn::VennDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::js_parse_float;

use db::VennDb;
use scanner::{Scanner, Spanned, Token};

/// Parses a venn diagram and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<VennDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let mut db = VennDb::default();
    // `Diagram.fromText` sets the front matter title before the parse; a grammar title replaces it.
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    Parser::new(source, &mut db)?.document()?;
    Ok(db.into_model())
}

/// The grammar, as a recursive-descent parser with one token of lookahead. The lookahead is the
/// token the grammar reads next: it is lexed when the token before it is consumed, so the grammar's
/// reductions happen where Jison's do, before the next line is lexed.
struct Parser<'a, 'd> {
    scanner: Scanner<'a>,
    source: Source<'a>,
    db: &'d mut VennDb,
    current: Spanned,
}

impl<'a, 'd> Parser<'a, 'd> {
    fn new(source: Source<'a>, db: &'d mut VennDb) -> Result<Self, ParseError> {
        let mut scanner = Scanner::new(source);
        let current = scanner.next_token()?;
        Ok(Self {
            scanner,
            source,
            db,
            current,
        })
    }

    /// Consumes the current token and lexes the next one. Returns the consumed token.
    fn bump(&mut self) -> Result<Spanned, ParseError> {
        let next = self.scanner.next_token()?;
        Ok(std::mem::replace(&mut self.current, next))
    }

    fn error_here(&self, message: &str) -> ParseError {
        self.source.error(self.current.offset, message)
    }

    /// The reduction of a `set` or `union` statement: `setIndentMode(true)`.
    fn reduce_indent(&mut self) {
        self.scanner.indent_mode = true;
    }

    /// `start: optNewlines VENN document EOF`.
    fn document(&mut self) -> Result<(), ParseError> {
        while self.current.token == Token::Newline {
            self.bump()?;
        }
        if self.current.token != Token::Venn {
            return Err(self.error_here("expected 'venn-beta'"));
        }
        self.bump()?;
        loop {
            match self.current.token {
                Token::Newline => {
                    self.bump()?;
                }
                Token::Eof => return Ok(()),
                Token::Title(_)
                | Token::Set
                | Token::Union
                | Token::Text
                | Token::IndentText
                | Token::Style => self.statement()?,
                _ => return Err(self.error_here("expected a statement")),
            }
        }
    }

    /// `statement`: each alternative of the grammar, with its action.
    fn statement(&mut self) -> Result<(), ParseError> {
        let keyword = self.bump()?;
        match keyword.token {
            Token::Title(text) => {
                // `$1.substr(6)`: the keyword and the blank after it.
                let title: String = text.chars().skip(6).collect();
                self.db.set_diagram_title(&title);
            }
            Token::Set => self.set_statement()?,
            Token::Union => self.union_statement(keyword.offset)?,
            Token::Text => self.text_statement()?,
            Token::IndentText => self.indent_text_statement(keyword.offset)?,
            Token::Style => self.style_statement()?,
            _ => return Err(self.source.error(keyword.offset, "expected a statement")),
        }
        Ok(())
    }

    /// `SET identifier [BRACKET_LABEL] [COLON NUMERIC]`.
    fn set_statement(&mut self) -> Result<(), ParseError> {
        let id = self.identifier()?;
        let (label, size) = self.label_and_size()?;
        self.reduce_indent();
        self.db.add_subset(&[id], label, size);
        Ok(())
    }

    /// `UNION identifierList [BRACKET_LABEL] [COLON NUMERIC]`, with the arity and the known ids
    /// checked before the subset is added.
    fn union_statement(&mut self, offset: usize) -> Result<(), ParseError> {
        let ids = self.identifier_list()?;
        let (label, size) = self.label_and_size()?;
        if ids.len() < 2 {
            return Err(self
                .source
                .error(offset, "union requires multiple identifiers"));
        }
        let source = self.source;
        self.db
            .validate_union(&ids)
            .map_err(|message| source.error(offset, message))?;
        self.reduce_indent();
        self.db.add_subset(&ids, label, size);
        Ok(())
    }

    /// `TEXT identifierList (IDENTIFIER | STRING | NUMERIC) [BRACKET_LABEL]`, where a number takes no
    /// label, as the grammar gives it.
    fn text_statement(&mut self) -> Result<(), ParseError> {
        let ids = self.identifier_list()?;
        let (id, label) = self.text_id_and_label()?;
        self.db.add_text(&ids, &id, label);
        Ok(())
    }

    /// `INDENT_TEXT indentedTextTail`: the text joins the current sets, which an earlier `set` or
    /// `union` gave.
    fn indent_text_statement(&mut self, offset: usize) -> Result<(), ParseError> {
        let (id, label) = self.text_id_and_label()?;
        let Some(sets) = self.db.current_sets() else {
            return Err(self.source.error(offset, "text requires set"));
        };
        self.db.add_text(&sets, &id, label);
        Ok(())
    }

    /// `STYLE identifierList styleField (COMMA styleField)*`, where a field is `IDENTIFIER COLON
    /// styleValue`.
    fn style_statement(&mut self) -> Result<(), ParseError> {
        let ids = self.identifier_list()?;
        let mut properties = Vec::new();
        loop {
            let key = match self.current.token.clone() {
                Token::Identifier(key) => key,
                _ => return Err(self.error_here("expected a style property")),
            };
            self.bump()?;
            if self.current.token != Token::Colon {
                return Err(self.error_here("expected ':' after the style property"));
            }
            self.bump()?;
            properties.push((key, self.style_value()?));
            if self.current.token != Token::Comma {
                break;
            }
            self.bump()?;
        }
        self.db.add_style(&ids, properties);
        Ok(())
    }

    /// `styleValue: STRING | valueToken+`, where the tokens are joined with one space.
    fn style_value(&mut self) -> Result<String, ParseError> {
        if let Token::Str(text) = self.current.token.clone() {
            self.bump()?;
            return Ok(text);
        }
        let mut words = Vec::new();
        while let Some(word) = value_word(&self.current.token) {
            words.push(word.to_owned());
            self.bump()?;
        }
        if words.is_empty() {
            return Err(self.error_here("expected a style value"));
        }
        Ok(words.join(" "))
    }

    /// `identifier (COMMA identifier)*`.
    fn identifier_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut ids = vec![self.identifier()?];
        while self.current.token == Token::Comma {
            self.bump()?;
            ids.push(self.identifier()?);
        }
        Ok(ids)
    }

    /// `identifier: IDENTIFIER | STRING`, as written.
    fn identifier(&mut self) -> Result<String, ParseError> {
        match self.current.token.clone() {
            Token::Identifier(text) | Token::Str(text) => {
                self.bump()?;
                Ok(text)
            }
            _ => Err(self.error_here("expected an identifier")),
        }
    }

    /// `[BRACKET_LABEL] [COLON NUMERIC]`, the tail of a `set` or `union`.
    fn label_and_size(&mut self) -> Result<(Option<String>, Option<f64>), ParseError> {
        let label = match self.current.token.clone() {
            Token::BracketLabel(text) => {
                self.bump()?;
                Some(text)
            }
            _ => None,
        };
        let size = if self.current.token == Token::Colon {
            self.bump()?;
            Some(self.number()?)
        } else {
            None
        };
        Ok((label, size))
    }

    /// `NUMERIC`, read as `parseFloat` reads it.
    fn number(&mut self) -> Result<f64, ParseError> {
        match self.current.token.clone() {
            Token::Numeric(text) => {
                self.bump()?;
                Ok(js_parse_float(&text))
            }
            _ => Err(self.error_here("expected a number")),
        }
    }

    /// The id of a text node and its label. The grammar gives a number no label, and an identifier
    /// or a string an optional one.
    fn text_id_and_label(&mut self) -> Result<(String, Option<String>), ParseError> {
        let token = self.current.token.clone();
        let (id, takes_label) = match token {
            Token::Identifier(text) | Token::Str(text) => (text, true),
            Token::Numeric(text) => (text, false),
            _ => return Err(self.error_here("expected the id of a text node")),
        };
        self.bump()?;
        let label = match self.current.token.clone() {
            Token::BracketLabel(text) if takes_label => {
                self.bump()?;
                Some(text)
            }
            _ => None,
        };
        Ok((id, label))
    }
}

/// The text of a `valueToken`: an identifier, a number or a colour, as written.
fn value_word(token: &Token) -> Option<&str> {
    match token {
        Token::Identifier(text) | Token::Numeric(text) | Token::Colour(text) => Some(text),
        _ => None,
    }
}
