//! The quadrant chart parser (P2): `quadrant.jison` and the DB `quadrantDb.ts`, ported by hand.
//! The plan, the rule table and the quirks are in `docs-port/quadrant/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The grammar then runs its DB
//! calls in the order the reductions of the Jison parser make them, and the DB's state becomes the
//! model. The chart builder is not ported: its layout is the renderer's.

mod db;
mod scanner;

use ferrigura_model::quadrant_chart::QuadrantChartDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::js_trim;

use db::QuadrantDb;
use scanner::{Spanned, Token, scan};

/// Parses a quadrant chart diagram and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<QuadrantChartDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let mut db = QuadrantDb::new();
    // `Diagram.fromText` sets the front matter title before the parse; a grammar title replaces it.
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    let tokens = scan(&text);
    let mut grammar = Grammar {
        tokens: &tokens,
        pos: 0,
        source,
        db: &mut db,
    };
    grammar.start()?;
    Ok(db.into_model())
}

/// Whether a token starts a text: an `alphaNumToken`, or a quoted or Markdown string.
fn starts_text(token: Token) -> bool {
    is_alpha_num(token) || matches!(token, Token::Str | Token::MdStr)
}

/// The `alphaNumToken` of the grammar.
fn is_alpha_num(token: Token) -> bool {
    matches!(
        token,
        Token::Punctuation
            | Token::Amp
            | Token::Num
            | Token::Alpha
            | Token::Comma
            | Token::Plus
            | Token::Equals
            | Token::Mult
            | Token::Dot
            | Token::Brkt
            | Token::Underscore
            | Token::UnicodeText
    )
}

/// The tokens that may follow the first token of a text: `textNoTagsToken`.
fn continues_text(token: Token) -> bool {
    is_alpha_num(token) || matches!(token, Token::Space | Token::Minus)
}

/// The tokens of `idStringToken`, which make a class name.
fn is_id(token: Token) -> bool {
    matches!(
        token,
        Token::Alpha
            | Token::Num
            | Token::Minus
            | Token::Comma
            | Token::Colon
            | Token::Amp
            | Token::Brkt
            | Token::Mult
            | Token::UnicodeText
    )
}

/// The tokens of `styleComponent`.
fn is_style_component(token: Token) -> bool {
    matches!(
        token,
        Token::Alpha | Token::Num | Token::Colon | Token::Space | Token::Brkt | Token::Minus
    )
}

/// The tokens of `eol`.
fn is_eol(token: Token) -> bool {
    matches!(token, Token::Newline | Token::Semi | Token::Eof)
}

/// The statements of `quadrant.jison`, read from the tokens. Each reduction that calls the DB does
/// so where the Jison parser would, so the DB sees the same calls in the same order.
struct Grammar<'a, 'b> {
    tokens: &'a [Spanned],
    pos: usize,
    source: Source<'a>,
    db: &'b mut QuadrantDb,
}

impl<'a> Grammar<'a, '_> {
    /// The token at the position, or the raw end once the tokens run out.
    fn peek(&self) -> Token {
        self.tokens
            .get(self.pos)
            .map_or(Token::RawEof, |spanned| spanned.token)
    }

    /// The text of the token at the position, and the position moves past it.
    fn take(&mut self) -> &'a str {
        let spanned = self.tokens.get(self.pos).copied();
        self.pos += 1;
        match spanned {
            Some(spanned) => &self.source.text()[spanned.start..spanned.end],
            None => "",
        }
    }

    /// Takes the token if it is `expected`, and reports whether it did.
    fn accept(&mut self, expected: Token) -> bool {
        if self.peek() == expected {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Takes a token of `expected` and returns its text, or fails.
    fn expect(&mut self, expected: Token, what: &str) -> Result<&'a str, ParseError> {
        if self.peek() == expected {
            Ok(self.take())
        } else {
            Err(self.error(what))
        }
    }

    /// An error at the token at the position.
    fn error(&self, message: &str) -> ParseError {
        let offset = self
            .tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start);
        self.source.error(offset, message)
    }

    /// A rejection from the DB, reported at the token at the position.
    fn db_error(&self, message: String) -> ParseError {
        self.error(&message)
    }

    /// `start: eol start | SPACE start | QUADRANT document`.
    fn start(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Token::Space => self.pos += 1,
                token if is_eol(token) => self.pos += 1,
                Token::Quadrant => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error("expected quadrantChart")),
            }
        }
        self.document()
    }

    /// `document: /* empty */ | document line`.
    fn document(&mut self) -> Result<(), ParseError> {
        while self.peek() != Token::RawEof {
            self.line()?;
        }
        Ok(())
    }

    /// `line: statement eol`.
    fn line(&mut self) -> Result<(), ParseError> {
        self.statement()?;
        if is_eol(self.peek()) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error("expected the end of the line"))
        }
    }

    /// `statement`: the blanks, then one of the statements, or none.
    fn statement(&mut self) -> Result<(), ParseError> {
        while self.accept(Token::Space) {}
        match self.peek() {
            token if is_eol(token) => Ok(()),
            Token::ClassDef => self.class_def(),
            Token::XAxis => self.axis(true),
            Token::YAxis => self.axis(false),
            Token::Quadrant1 => self.quadrant_details(1),
            Token::Quadrant2 => self.quadrant_details(2),
            Token::Quadrant3 => self.quadrant_details(3),
            Token::Quadrant4 => self.quadrant_details(4),
            Token::Title => {
                self.pos += 1;
                let value = self.expect(Token::TitleValue, "expected the title")?;
                self.db.set_diagram_title(js_trim(value));
                Ok(())
            }
            Token::AccTitle => {
                self.pos += 1;
                let value = self.expect(Token::AccTitleValue, "expected the accessible title")?;
                self.db.set_acc_title(js_trim(value));
                Ok(())
            }
            Token::AccDescr => {
                self.pos += 1;
                let value =
                    self.expect(Token::AccDescrValue, "expected the accessible description")?;
                self.db.set_acc_description(js_trim(value));
                Ok(())
            }
            Token::AccDescrMultiValue => {
                let value = self.take();
                self.db.set_acc_description(js_trim(value));
                Ok(())
            }
            token if starts_text(token) => self.points(),
            _ => Err(self.error("unexpected token")),
        }
    }

    /// `text: alphaNumToken | text textNoTagsToken | STR | MD_STR`: the text, whose parts are
    /// concatenated.
    fn text(&mut self) -> Result<String, ParseError> {
        if !starts_text(self.peek()) {
            return Err(self.error("expected text"));
        }
        let mut text = self.take().to_owned();
        while continues_text(self.peek()) {
            text.push_str(self.take());
        }
        Ok(text)
    }

    /// `classDefStatement: CLASSDEF SPACE idString SPACE stylesOpt`.
    fn class_def(&mut self) -> Result<(), ParseError> {
        self.pos += 1;
        self.expect(Token::Space, "expected a blank after classDef")?;
        let mut name = String::new();
        while is_id(self.peek()) {
            name.push_str(self.take());
        }
        if name.is_empty() {
            return Err(self.error("expected a class name"));
        }
        self.expect(Token::Space, "expected a blank after the class name")?;
        let styles = self.styles_opt()?;
        self.db
            .add_class(&name, &styles)
            .map_err(|message| self.db_error(message))
    }

    /// `stylesOpt: style | stylesOpt COMMA style`, each style trimmed.
    fn styles_opt(&mut self) -> Result<Vec<String>, ParseError> {
        let mut styles = vec![self.style()?];
        while self.accept(Token::Comma) {
            styles.push(self.style()?);
        }
        Ok(styles)
    }

    /// `style: styleComponent | style styleComponent`, trimmed.
    fn style(&mut self) -> Result<String, ParseError> {
        if !is_style_component(self.peek()) {
            return Err(self.error("expected a style"));
        }
        let mut style = String::new();
        while is_style_component(self.peek()) {
            style.push_str(self.take());
        }
        Ok(js_trim(&style).to_owned())
    }

    /// `axisDetails`: the `x-axis` or `y-axis` texts, with an optional delimiter. A text with a
    /// delimiter and no text after it gets the arrow appended, as the grammar does.
    fn axis(&mut self, is_x: bool) -> Result<(), ParseError> {
        self.pos += 1;
        let mut first = self.text()?;
        let (left_or_bottom, right_or_top) = if self.accept(Token::AxisDelimiter) {
            if starts_text(self.peek()) {
                (first, Some(self.text()?))
            } else {
                first.push_str(" \u{27F6} ");
                (first, None)
            }
        } else {
            (first, None)
        };
        match (is_x, right_or_top) {
            (true, right) => {
                self.db.set_x_axis_left_text(&left_or_bottom);
                if let Some(right) = right {
                    self.db.set_x_axis_right_text(&right);
                }
            }
            (false, top) => {
                self.db.set_y_axis_bottom_text(&left_or_bottom);
                if let Some(top) = top {
                    self.db.set_y_axis_top_text(&top);
                }
            }
        }
        Ok(())
    }

    /// `QUADRANT_n text`.
    fn quadrant_details(&mut self, quadrant: usize) -> Result<(), ParseError> {
        self.pos += 1;
        let text = self.text()?;
        self.db.set_quadrant_text(quadrant, &text);
        Ok(())
    }

    /// `points`: `text point_start point_x point_y stylesOpt?`, with the class name between the text
    /// and the point's start when there is one.
    fn points(&mut self) -> Result<(), ParseError> {
        let text = self.text()?;
        let class_name = if self.peek() == Token::ClassName {
            self.take().to_owned()
        } else {
            String::new()
        };
        self.expect(Token::PointStart, "expected the start of a point")?;
        let x = self.expect(Token::PointX, "expected the x coordinate")?;
        let y = self.expect(Token::PointY, "expected the y coordinate")?;
        let styles = if is_style_component(self.peek()) {
            self.styles_opt()?
        } else {
            Vec::new()
        };
        self.db
            .add_point(&text, &class_name, x, y, &styles)
            .map_err(|message| self.db_error(message))
    }
}
