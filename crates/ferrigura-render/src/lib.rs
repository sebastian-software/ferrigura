//! Layout and SVG output of ferrigura diagrams.
//!
//! A diagram is laid out from its model, its configuration and its theme, and written as SVG
//! text. Label sizes come from `ferrigura-text`; every other size is calculated here
//! (`docs-port/decisions/0005`). Labels are SVG text with every run placed at an explicit `x`, and
//! the fonts are embedded as subsets (`docs-port/decisions/0004`).

#![warn(missing_docs)]

pub mod pie;
pub mod svg;

use std::fmt;

use ferrigura_syntax::ParseError;
use ferrigura_text::TextError;

/// An error while rendering a diagram.
#[derive(Debug)]
pub enum RenderError {
    /// The text is not a diagram this crate renders, or it does not parse.
    Parse(ParseError),
    /// The configuration holds a value of the wrong type.
    Config(String),
    /// A bundled font could not be read or subset.
    Text(TextError),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Config(message) => write!(f, "invalid configuration: {message}"),
            Self::Text(error) => write!(f, "text: {error:?}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<ParseError> for RenderError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<TextError> for RenderError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}
