//! Parse errors with a position in the source text.

use std::fmt;

/// A syntax error. Positions are 1-based and count lines of the original input, front matter
/// included; columns count characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Line of the offending token, 1-based.
    pub line: usize,
    /// Column of the offending token, 1-based.
    pub column: usize,
    /// What went wrong, for people reading the message.
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// The prepared text a parser reads, with the number of lines that were removed before it so
/// that positions can be reported against the original input.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Source<'a> {
    text: &'a str,
    line_offset: usize,
}

impl<'a> Source<'a> {
    pub(crate) fn new(text: &'a str, line_offset: usize) -> Self {
        Self { text, line_offset }
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    /// An error at byte `offset` of the prepared text.
    pub(crate) fn error(&self, offset: usize, message: impl Into<String>) -> ParseError {
        let offset = offset.min(self.text.len());
        let before = &self.text[..offset];
        let line = before.matches('\n').count() + 1 + self.line_offset;
        let column = match before.rfind('\n') {
            Some(newline) => before[newline + 1..].chars().count() + 1,
            None => before.chars().count() + 1,
        };
        ParseError {
            line,
            column,
            message: message.into(),
        }
    }
}
