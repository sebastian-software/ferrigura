//! Layout and SVG output of ferrigura diagrams.
//!
//! A diagram is laid out from its model, its configuration and its theme, and written as SVG
//! text. Label sizes come from `ferrigura-text`; every other size is calculated here
//! (`docs-port/decisions/0005`). Labels are SVG text with every run placed at an explicit `x`, and
//! the fonts are embedded as subsets (`docs-port/decisions/0004`).

#![warn(missing_docs)]

pub mod a11y;
pub mod architecture;
pub mod block;
pub mod c4;
pub mod class;
pub mod common;
pub mod er;
pub mod event_modeling;
pub mod flowchart;
pub mod gantt;
pub mod git_graph;
pub mod info;
pub mod ishikawa;
pub mod journey;
pub mod kanban;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant_chart;
pub mod requirement;
pub mod sankey;
pub mod sequence;
pub mod state;
pub mod svg;
pub mod theme;
pub mod timeline;
pub mod treemap;
pub mod unified;
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
