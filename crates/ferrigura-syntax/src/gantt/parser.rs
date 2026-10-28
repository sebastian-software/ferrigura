//! The grammar of `gantt.jison` (lines 107-186) over the token list. Each statement makes the DB
//! call its action makes, in the order of the actions. The statement keywords keep their text:
//! the value is the text after the keyword and one whitespace character, as the actions' `substr`
//! calls take it.

use crate::error::{ParseError, Source};
use crate::text::js_trim;

use super::db::Db;
use super::scanner::{Spanned, Token};

/// Parses the statements of a gantt diagram into the DB.
///
/// `start: gantt document 'EOF'`: the first token is the `gantt` keyword. After it, the statements
/// follow in any order, without separators, and line breaks are skipped.
pub(crate) fn parse_statements(
    tokens: &[Spanned],
    source: Source<'_>,
    db: &mut Db,
) -> Result<(), ParseError> {
    let mut cursor = Cursor {
        tokens,
        pos: 0,
        source,
    };
    match cursor.take() {
        Some(Spanned {
            token: Token::Gantt,
            ..
        }) => {}
        _ => return Err(cursor.error_at_current("expected the 'gantt' keyword")),
    }
    loop {
        match cursor.peek_token() {
            Some(Token::Eof) | None => return Ok(()),
            Some(Token::Newline) => {
                cursor.take();
            }
            Some(_) => statement(&mut cursor, db)?,
        }
    }
}

/// The token list with a read position. The list always ends with an `Eof` token.
struct Cursor<'a> {
    tokens: &'a [Spanned],
    pos: usize,
    source: Source<'a>,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<&Spanned> {
        self.tokens.get(self.pos)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.peek().map(|spanned| &spanned.token)
    }

    fn take(&mut self) -> Option<Spanned> {
        let spanned = self.tokens.get(self.pos).cloned();
        if spanned.is_some() {
            self.pos += 1;
        }
        spanned
    }

    /// An error at the next token.
    fn error_at_current(&self, message: &str) -> ParseError {
        let offset = self.peek().map_or(self.source.text().len(), |s| s.start);
        self.source.error(offset, message)
    }

    /// An error at the offset of the token just taken.
    fn error_at(&self, offset: usize, message: &str) -> ParseError {
        self.source.error(offset, message)
    }

    /// Takes the next token if it is a value token of the kind `pick` accepts.
    fn expect_value(
        &mut self,
        what: &str,
        pick: impl Fn(&Token) -> Option<String>,
    ) -> Result<String, ParseError> {
        match self.peek().and_then(|spanned| pick(&spanned.token)) {
            Some(value) => {
                self.pos += 1;
                Ok(value)
            }
            None => Err(self.error_at_current(&format!("expected {what}"))),
        }
    }
}

/// The value of a statement keyword: the text after the keyword and the one whitespace character
/// that follows it (`substr(n)` of the reference's actions).
fn after_keyword(text: &str, length: usize) -> String {
    text.chars().skip(length).collect()
}

/// Parses one statement, which starts with a token that is not a newline.
fn statement(cursor: &mut Cursor<'_>, db: &mut Db) -> Result<(), ParseError> {
    let Some(Spanned { token, start }) = cursor.take() else {
        return Ok(());
    };
    match token {
        Token::DateFormat(text) => db.set_date_format(&after_keyword(&text, 11)),
        Token::InclusiveEndDates => db.enable_inclusive_end_dates(),
        Token::TopAxis => db.enable_top_axis(),
        Token::AxisFormat(text) => db.set_axis_format(&after_keyword(&text, 11)),
        Token::TickInterval(text) => db.set_tick_interval(&after_keyword(&text, 13)),
        Token::Excludes(text) => db.set_excludes(&after_keyword(&text, 9)),
        Token::Includes(text) => db.set_includes(&after_keyword(&text, 9)),
        Token::TodayMarker(text) => db.set_today_marker(&after_keyword(&text, 12)),
        Token::Weekday(name) => db.set_weekday(name),
        Token::Weekend(name) => db.set_weekend(name),
        Token::Title(text) => db.set_diagram_title(&after_keyword(&text, 6)),
        Token::AccTitle => {
            let value = cursor.expect_value("the accessible title", |t| match t {
                Token::AccTitleValue(v) => Some(v.clone()),
                _ => None,
            })?;
            db.set_acc_title(js_trim(&value));
        }
        Token::AccDescr => {
            let value = cursor.expect_value("the accessible description", |t| match t {
                Token::AccDescrValue(v) => Some(v.clone()),
                _ => None,
            })?;
            db.set_acc_description(js_trim(&value));
        }
        Token::AccDescrMultiValue(value) => db.set_acc_description(js_trim(&value)),
        Token::Section(text) => db.add_section(&after_keyword(&text, 8)),
        Token::ClickId(id) => click_statement(cursor, db, &id)?,
        Token::TaskTxt(name) => {
            let data = cursor.expect_value("the data of the task", |t| match t {
                Token::TaskData(v) => Some(v.clone()),
                _ => None,
            })?;
            db.add_task(name, &data, start)
                .map_err(|error| cursor.error_at(error.offset, &error.message))?;
        }
        _ => return Err(cursor.error_at(start, "unexpected token")),
    }
    Ok(())
}

/// `clickStatement` (lines 162-173): `click` with an id, then a callback (`call name(args)`) and a
/// link (`href "url"`) in the orders the grammar allows, either callback first or link first.
fn click_statement(cursor: &mut Cursor<'_>, db: &mut Db, id: &str) -> Result<(), ParseError> {
    let mut callback = false;
    let mut href = None;
    match cursor.peek_token() {
        Some(Token::CallbackName(_)) => {
            callback = true;
            cursor.take();
            if matches!(cursor.peek_token(), Some(Token::CallbackArgs(_))) {
                cursor.take();
            }
            if let Some(Token::Href(url)) = cursor.peek_token() {
                href = Some(url.clone());
                cursor.take();
            }
        }
        Some(Token::Href(url)) => {
            href = Some(url.clone());
            cursor.take();
            if matches!(cursor.peek_token(), Some(Token::CallbackName(_))) {
                callback = true;
                cursor.take();
                if matches!(cursor.peek_token(), Some(Token::CallbackArgs(_))) {
                    cursor.take();
                }
            }
        }
        _ => return Err(cursor.error_at_current("expected a call or an href after 'click'")),
    }
    if callback {
        db.set_click_event(id);
    }
    if let Some(url) = href {
        db.set_link(id, &url);
    }
    Ok(())
}
