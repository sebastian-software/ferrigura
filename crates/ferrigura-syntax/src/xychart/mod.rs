//! The xychart parser (P2): `xychart.jison` and the DB `xychartDb.ts`, ported by hand.
//! The plan, the rule table and the quirks are in `docs-port/xychart/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The configuration of the text
//! (`xyChart.chartOrientation`, and the palette of the theme) is read before the DB is cleared. The
//! grammar then runs its DB calls in the order the reductions of the Jison parser make them, and
//! the DB's state becomes the model.

mod db;
mod scanner;

use ferrigura_model::config::effective;
use ferrigura_model::value::Value;
use ferrigura_model::xychart::XyChartDiagram;

use crate::config::read;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::{js_number, js_trim};

use db::XyChartDb;
use scanner::{Spanned, Token, scan};

/// The palette of the redux colour theme, the theme a configuration without a shipped theme
/// resolves to. It is the fallback when the resolved configuration has no palette of its own.
const REDUX_COLOR_PALETTE: &str =
    "#FFF4DD,#FFD8B1,#FFA07A,#ECEFF1,#D6DBDF,#C3E0A8,#FFB6A4,#FFD74D,#738FA7,#FFFFF0";

/// Parses an xychart diagram and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<XyChartDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    // `clear()` of the DB reads the configuration that the text carries, as `Diagram.fromText`
    // leaves it after the directives and the front matter.
    let directive = read(raw)?;
    let resolved = effective(None, Some("xyChart"), &directive);
    let orientation = resolved
        .get("xyChart")
        .and_then(|section| section.get("chartOrientation"))
        .and_then(Value::as_str)
        .unwrap_or("vertical")
        .to_owned();
    let palette = resolved
        .get("themeVariables")
        .and_then(|variables| variables.get("xyChart"))
        .and_then(|section| section.get("plotColorPalette"))
        .and_then(Value::as_str)
        .unwrap_or(REDUX_COLOR_PALETTE);

    let mut db = XyChartDb::new(orientation, palette);
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

/// The statements of `xychart.jison`, read from the tokens. Each reduction that calls the DB does
/// so at the point where the Jison parser would, so the DB sees the same calls in the same order.
struct Grammar<'a, 'b> {
    tokens: &'a [Spanned],
    pos: usize,
    source: Source<'a>,
    db: &'b mut XyChartDb,
}

/// The token that ends the input when the scan has run out of tokens.
static END: Token = Token::RawEof;

impl<'a> Grammar<'a, '_> {
    /// The token at the position, with the input's lifetime so that it can be read while the DB
    /// is changed.
    fn peek(&self) -> &'a Token {
        let tokens: &'a [Spanned] = self.tokens;
        tokens.get(self.pos).map_or(&END, |spanned| &spanned.token)
    }

    /// Takes the token at the position.
    fn advance(&mut self) -> &'a Token {
        let token = self.peek();
        self.pos += 1;
        token
    }

    /// An error at the token at the position.
    fn error(&self, message: &str) -> ParseError {
        let offset = self
            .tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start);
        self.source.error(offset, message)
    }

    /// `start: eol start | XYCHART chartConfig start | XYCHART start | document`.
    fn start(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Token::Newline | Token::Semi | Token::Eof => self.pos += 1,
                Token::XyChart => {
                    self.pos += 1;
                    if let Token::ChartOrientation(word) = self.peek() {
                        self.pos += 1;
                        self.db.set_orientation(word);
                    }
                }
                _ => break,
            }
        }
        self.document()
    }

    /// `document: /* empty */ | document statement`, with the `eol` that follows each statement.
    fn document(&mut self) -> Result<(), ParseError> {
        while !matches!(self.peek(), Token::RawEof) {
            self.statement()?;
            while matches!(self.peek(), Token::Newline | Token::Semi | Token::Eof) {
                self.pos += 1;
            }
        }
        Ok(())
    }

    /// `statement`: a title, an axis, a plot, or an accessibility statement.
    fn statement(&mut self) -> Result<(), ParseError> {
        match self.advance() {
            Token::Title => {
                let Some(text) = self.text() else {
                    return Err(self.error("expected the title text"));
                };
                self.db.set_diagram_title(js_trim(&text));
            }
            Token::XAxis => self.x_axis()?,
            Token::YAxis => self.y_axis()?,
            Token::Line => {
                let title = self.text().unwrap_or_default();
                let points = self.plot_data()?;
                self.db.set_line_data(&title, &points);
            }
            Token::Bar => {
                let title = self.text().unwrap_or_default();
                let points = self.plot_data()?;
                self.db.set_bar_data(&title, &points);
            }
            Token::AccTitle => match self.advance() {
                Token::AccTitleValue(value) => self.db.set_acc_title(js_trim(value)),
                _ => return Err(self.error("expected the accessible title")),
            },
            Token::AccDescr => match self.advance() {
                Token::AccDescrValue(value) => self.db.set_acc_description(js_trim(value)),
                _ => return Err(self.error("expected the accessible description")),
            },
            Token::AccDescrMultiValue(value) => self.db.set_acc_description(js_trim(value)),
            _ => return Err(self.error("expected a statement")),
        }
        Ok(())
    }

    /// `text: alphaNum | STR | MD_STR`. The words of `alphaNum` join with no space. Markdown never
    /// occurs (quirk P4).
    fn text(&mut self) -> Option<String> {
        match self.peek() {
            Token::Str(text) => {
                self.pos += 1;
                Some(text.clone())
            }
            Token::AlphaNum(_) => {
                let mut joined = String::new();
                while let Token::AlphaNum(word) = self.peek() {
                    joined.push_str(word);
                    self.pos += 1;
                }
                Some(joined)
            }
            _ => None,
        }
    }

    /// `parseXAxis: text | text xAxisData | xAxisData`. The axis data is read first, and the title
    /// is set after it, as the reductions order them.
    fn x_axis(&mut self) -> Result<(), ParseError> {
        let title = self.text();
        match self.peek() {
            Token::SquareBracesStart => {
                let categories = self.band_data()?;
                self.db.set_x_axis_band(&categories);
                self.db.set_x_axis_title(title.as_deref().unwrap_or(""));
            }
            Token::NumberWithDecimal(_) => {
                let (min, max) = self.range()?;
                self.db.set_x_axis_range(min, max);
                self.db.set_x_axis_title(title.as_deref().unwrap_or(""));
            }
            _ => match title {
                Some(title) => self.db.set_x_axis_title(&title),
                None => return Err(self.error("expected the x-axis title or data")),
            },
        }
        Ok(())
    }

    /// `parseYAxis: text | text yAxisData | yAxisData`, as `x_axis`, with a range only.
    fn y_axis(&mut self) -> Result<(), ParseError> {
        let title = self.text();
        match self.peek() {
            Token::NumberWithDecimal(_) => {
                let (min, max) = self.range()?;
                self.db.set_y_axis_range(min, max);
                self.db.set_y_axis_title(title.as_deref().unwrap_or(""));
            }
            _ => match title {
                Some(title) => self.db.set_y_axis_title(&title),
                None => return Err(self.error("expected the y-axis title or range")),
            },
        }
        Ok(())
    }

    /// `NUMBER_WITH_DECIMAL ARROW_DELIMITER NUMBER_WITH_DECIMAL`, the range of an axis.
    fn range(&mut self) -> Result<(f64, f64), ParseError> {
        let first = self.number()?;
        if !matches!(self.advance(), Token::ArrowDelimiter) {
            return Err(self.error("expected '-->'"));
        }
        let second = self.number()?;
        Ok((first, second))
    }

    /// One `NUMBER_WITH_DECIMAL`, as `Number()` reads it.
    fn number(&mut self) -> Result<f64, ParseError> {
        match self.advance() {
            Token::NumberWithDecimal(text) => Ok(js_number(text)),
            _ => Err(self.error("expected a number")),
        }
    }

    /// `bandData: [ commaSeparatedTexts ]`, the categories in order.
    fn band_data(&mut self) -> Result<Vec<String>, ParseError> {
        if !matches!(self.advance(), Token::SquareBracesStart) {
            return Err(self.error("expected '['"));
        }
        let mut categories = Vec::new();
        loop {
            let Some(category) = self.text() else {
                return Err(self.error("expected a category"));
            };
            categories.push(category);
            if !matches!(self.peek(), Token::Comma) {
                break;
            }
            self.pos += 1;
        }
        if !matches!(self.advance(), Token::SquareBracesEnd) {
            return Err(self.error("expected ']'"));
        }
        Ok(categories)
    }

    /// `plotData: [ dataPoints ]`: each point is a number with an optional label. The points are
    /// returned as their values and labels, the labels raw.
    fn plot_data(&mut self) -> Result<Vec<(f64, String)>, ParseError> {
        if !matches!(self.advance(), Token::SquareBracesStart) {
            return Err(self.error("expected '['"));
        }
        let mut points = Vec::new();
        loop {
            let value = self.number()?;
            let label = match self.peek() {
                Token::Str(label) => {
                    self.pos += 1;
                    label.clone()
                }
                _ => String::new(),
            };
            points.push((value, label));
            if !matches!(self.peek(), Token::Comma) {
                break;
            }
            self.pos += 1;
        }
        if !matches!(self.advance(), Token::SquareBracesEnd) {
            return Err(self.error("expected ']'"));
        }
        Ok(points)
    }
}
