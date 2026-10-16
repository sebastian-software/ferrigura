//! The error type of the layout pipeline.

use std::fmt;

/// Why a graph cannot be laid out by this port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// The input is malformed for elkjs too (for example an unsupported value).
    Format(String),
    /// The graph needs something this port does not implement yet.
    Unsupported(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::Format(message) => write!(f, "{message}"),
            LayoutError::Unsupported(message) => write!(f, "not supported yet: {message}"),
        }
    }
}

impl std::error::Error for LayoutError {}
