//! Errors of measuring, wrapping and embedding text.

use std::fmt;

/// An error from measuring, shaping or subsetting text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextError {
    /// The font data does not parse as a font.
    InvalidFont,
    /// The font has no line metrics, so no text box can be derived from it.
    MissingLineMetrics,
    /// The shaper rejected the text.
    Shape(harfrust::ShapeError),
    /// Subsetting the font failed; the message comes from `skera`.
    Subset(String),
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont => write!(f, "the font data does not parse"),
            Self::MissingLineMetrics => write!(f, "the font has no line metrics"),
            Self::Shape(error) => write!(f, "shaping failed: {error:?}"),
            Self::Subset(message) => write!(f, "subsetting failed: {message}"),
        }
    }
}

impl std::error::Error for TextError {}
