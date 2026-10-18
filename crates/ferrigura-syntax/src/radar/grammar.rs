//! The grammar of `packages/parser/src/language/radar/radar.langium`, read by hand over the tokens
//! of [`super::scanner`]. Every rule is a function, and the choices are the ones the reference's
//! LL parser makes.
//!
//! ```text
//! Radar:        NEWLINE* ('radar-beta' | 'radar-beta:' | 'radar-beta' ':') NEWLINE*
//!               ( TitleAndAccessibilities | 'axis' Axis (',' Axis)* | 'curve' Curve (',' Curve)*
//!               | Option (',' Option)* | NEWLINE )*
//! Axis:         ID Label?
//! Curve:        ID Label? '{' Entries '}'
//! Entries:      NEWLINE* NumberEntry (',' NEWLINE* NumberEntry)* NEWLINE*
//!             | NEWLINE* DetailedEntry (',' NEWLINE* DetailedEntry)* NEWLINE*
//! DetailedEntry: ID ':'? NUMBER
//! Label:        '[' STRING ']'
//! Option:       'showLegend' BOOLEAN | 'ticks' NUMBER | 'max' NUMBER | 'min' NUMBER
//!               | 'graticule' GRATICULE
//! TitleAndAccessibilities: ((ACC_DESCR | ACC_TITLE | TITLE) EOL)+
//! EOL:          NEWLINE+ | EOF
//! ```
//!
//! Statements need no separator between them, and a line break is allowed only where the rule has a
//! `NEWLINE` slot. The header texts keep the last value given, as Langium's assignments do, and the
//! DB reads them after the whole text parsed.

use ferrigura_model::radar::Graticule;

use super::scanner::{Kw, Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// An axis statement: the name, and the label when a `[ ]` follows it.
#[derive(Debug, Clone)]
pub(crate) struct AxisStmt {
    pub name: String,
    pub label: Option<String>,
}

/// A curve statement: its name, its label, and its entries.
#[derive(Debug, Clone)]
pub(crate) struct CurveStmt {
    pub name: String,
    pub label: Option<String>,
    pub entries: Entries,
    pub offset: usize,
}

/// The entries of a curve: all numbers, or all detailed entries that name an axis.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Entries {
    Numbers(Vec<f64>),
    Named(Vec<NamedEntry>),
}

/// A detailed entry: the name of the axis as written, and its value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NamedEntry {
    pub axis: String,
    pub value: f64,
}

/// An option statement, with the value its token gives.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OptionStmt {
    ShowLegend(bool),
    Ticks(f64),
    Max(f64),
    Min(f64),
    Graticule(Graticule),
}

/// The syntax tree of a diagram: the header values, then each statement list in text order.
#[derive(Debug, Default, Clone)]
pub(crate) struct Ast {
    pub acc_descr: String,
    pub acc_title: String,
    pub title: String,
    pub axes: Vec<AxisStmt>,
    pub curves: Vec<CurveStmt>,
    pub options: Vec<OptionStmt>,
}

/// Parses the prepared text of a diagram, which ends in a line break, into its tree.
pub(crate) fn parse_text(source: Source<'_>) -> Result<Ast, ParseError> {
    let tokens =
        tokenize(source.text()).map_err(|offset| source.error(offset, "unexpected character"))?;
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
    };
    parser.header()?;

    let mut ast = Ast::default();
    while let Some(token) = parser.peek().cloned() {
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
            Token::Keyword(Kw::Axis) => {
                parser.advance();
                parser.axis_list(&mut ast.axes)?;
            }
            Token::Keyword(Kw::Curve) => {
                parser.advance();
                parser.curve_list(&mut ast.curves)?;
            }
            Token::Keyword(Kw::ShowLegend | Kw::Ticks | Kw::Max | Kw::Min | Kw::Graticule) => {
                parser.option_list(&mut ast.options)?;
            }
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

    /// `NEWLINE* ('radar-beta' | 'radar-beta:' | 'radar-beta' ':') NEWLINE*`. The colon after the
    /// keyword is a token of its own unless the keyword already holds it.
    fn header(&mut self) -> Result<(), ParseError> {
        self.skip_newlines();
        match self.peek() {
            Some(Token::Header { with_colon }) => {
                let with_colon = *with_colon;
                self.advance();
                if !with_colon && self.peek() == Some(&Token::Colon) {
                    self.advance();
                }
                self.skip_newlines();
                Ok(())
            }
            _ => Err(self.error("expected 'radar-beta'")),
        }
    }

    /// `ID`: a name, as the scanner gives it.
    fn id(&mut self, what: &str) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Token::Id(name)) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(format!("expected {what}"))),
        }
    }

    /// `NUMBER`, as the scanner gives it.
    fn number(&mut self) -> Result<f64, ParseError> {
        match self.peek() {
            Some(Token::Number(value)) => {
                let value = *value;
                self.advance();
                Ok(value)
            }
            _ => Err(self.error("expected a number")),
        }
    }

    /// `Label: '[' label=STRING ']'`, when the bracket is next.
    fn optional_label(&mut self) -> Result<Option<String>, ParseError> {
        if self.peek() != Some(&Token::LBracket) {
            return Ok(None);
        }
        self.advance();
        let label = match self.peek().cloned() {
            Some(Token::Str(text)) => {
                self.advance();
                text
            }
            _ => return Err(self.error("expected a quoted label")),
        };
        self.expect_token(&Token::RBracket, "']' after the label")?;
        Ok(Some(label))
    }

    fn expect_token(&mut self, token: &Token, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(token) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {what}")))
        }
    }

    /// `Axis (',' Axis)*`, after the keyword.
    fn axis_list(&mut self, out: &mut Vec<AxisStmt>) -> Result<(), ParseError> {
        loop {
            let name = self.id("an axis name")?;
            let label = self.optional_label()?;
            out.push(AxisStmt { name, label });
            if self.peek() != Some(&Token::Comma) {
                return Ok(());
            }
            self.advance();
        }
    }

    /// `Curve (',' Curve)*`, after the keyword.
    fn curve_list(&mut self, out: &mut Vec<CurveStmt>) -> Result<(), ParseError> {
        loop {
            let offset = self.offset();
            let name = self.id("a curve name")?;
            let label = self.optional_label()?;
            self.expect_token(&Token::LBrace, "'{' after the curve name")?;
            let entries = self.entries()?;
            self.expect_token(&Token::RBrace, "'}' after the entries")?;
            out.push(CurveStmt {
                name,
                label,
                entries,
                offset,
            });
            if self.peek() != Some(&Token::Comma) {
                return Ok(());
            }
            self.advance();
        }
    }

    /// `Entries`: numbers, or detailed entries, each list with the newlines the grammar allows.
    fn entries(&mut self) -> Result<Entries, ParseError> {
        self.skip_newlines();
        match self.peek() {
            Some(Token::Number(_)) => {
                let mut values = Vec::new();
                loop {
                    values.push(self.number()?);
                    if self.peek() != Some(&Token::Comma) {
                        break;
                    }
                    self.advance();
                    self.skip_newlines();
                }
                self.skip_newlines();
                Ok(Entries::Numbers(values))
            }
            Some(Token::Id(_)) => {
                let mut entries = Vec::new();
                loop {
                    let axis = self.id("an axis name in an entry")?;
                    if self.peek() == Some(&Token::Colon) {
                        self.advance();
                    }
                    let value = self.number()?;
                    entries.push(NamedEntry { axis, value });
                    if self.peek() != Some(&Token::Comma) {
                        break;
                    }
                    self.advance();
                    self.skip_newlines();
                }
                self.skip_newlines();
                Ok(Entries::Named(entries))
            }
            _ => Err(self.error("expected a number or an axis entry")),
        }
    }

    /// `Option (',' Option)*`, from the first keyword on.
    fn option_list(&mut self, out: &mut Vec<OptionStmt>) -> Result<(), ParseError> {
        loop {
            out.push(self.option()?);
            if self.peek() != Some(&Token::Comma) {
                return Ok(());
            }
            self.advance();
        }
    }

    fn option(&mut self) -> Result<OptionStmt, ParseError> {
        let Some(Token::Keyword(kw)) = self.peek().cloned() else {
            return Err(self.error("expected an option"));
        };
        self.advance();
        match (kw, self.peek().cloned()) {
            (Kw::ShowLegend, Some(Token::Boolean(value))) => {
                self.advance();
                Ok(OptionStmt::ShowLegend(value))
            }
            (Kw::Ticks, Some(Token::Number(value))) => {
                self.advance();
                Ok(OptionStmt::Ticks(value))
            }
            (Kw::Max, Some(Token::Number(value))) => {
                self.advance();
                Ok(OptionStmt::Max(value))
            }
            (Kw::Min, Some(Token::Number(value))) => {
                self.advance();
                Ok(OptionStmt::Min(value))
            }
            (Kw::Graticule, Some(Token::Shape(shape))) => {
                self.advance();
                Ok(OptionStmt::Graticule(shape))
            }
            _ => Err(self.error("expected a value for the option")),
        }
    }
}
