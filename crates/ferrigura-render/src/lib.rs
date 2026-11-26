//! Layout and SVG output of ferrigura diagrams.
//!
//! A diagram is laid out from its model, its configuration and its theme, and written as SVG
//! text. Label sizes come from `ferrigura-text`; every other size is calculated here
//! (`docs/decisions/0010-text-measurement-and-fonts.md`). Labels are SVG text with every run placed at an explicit `x`, and
//! the fonts are embedded as subsets (`docs/decisions/0009-svg-text-labels.md`).

#![warn(missing_docs)]

pub mod a11y;
pub mod animation;
pub mod class;
pub mod common;
pub mod context;
pub mod design;
pub mod er;
pub mod flowchart;
pub mod fonts;
pub mod gantt;
pub mod mindmap;
pub mod pie;
pub mod quadrant_chart;
mod scheme;
pub mod sequence;
pub mod state;
pub mod svg;
pub mod unified;
#[cfg(test)]
mod variables;
pub mod xychart;

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
    /// An option of the render is not valid, such as an id that is not a CSS identifier.
    Options(String),
    /// A bundled font could not be read or subset.
    Text(TextError),
    /// The layout failed.
    Layout(String),
    /// The diagram type is not rendered yet.
    Unsupported(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Config(message) => write!(f, "invalid configuration: {message}"),
            Self::Options(message) => write!(f, "invalid option: {message}"),
            Self::Text(error) => write!(f, "text: {error:?}"),
            Self::Layout(message) => write!(f, "layout: {message}"),
            Self::Unsupported(message) => write!(f, "{message}"),
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
