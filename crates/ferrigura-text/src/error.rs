//! Errors of measuring, wrapping and embedding text.

use std::fmt;

/// An error from measuring, shaping or subsetting text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextError {
    /// The font data does not parse as a font.
    InvalidFont,
    /// A font registration or resolution failed.
    Registration(String),
    /// A selected face shaped part of a visible grapheme to a missing glyph.
    MissingGlyph {
        /// The grapheme containing the missing glyph.
        sequence: String,
        /// The requested family list.
        family: String,
        /// The UTF-8 byte offset in the supplied text, not a source-document position.
        byte_offset: usize,
    },
    /// The font has no line metrics, so no text box can be derived from it.
    MissingLineMetrics,
    /// The shaper rejected the text.
    Shape(harfrust::ShapeError),
    /// Subsetting the font failed; the message comes from `skera`.
    Subset(String),
    /// A word holds a newline, so the line cannot be wrapped; the reference throws for it.
    NewlineInLine,
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registration(message) => f.write_str(message),
            Self::MissingGlyph {
                sequence,
                family,
                byte_offset,
            } => write!(
                f,
                "selected font for {family:?} cannot shape {sequence:?} at text byte {byte_offset}"
            ),
            Self::InvalidFont => write!(f, "the font data does not parse"),
            Self::MissingLineMetrics => write!(f, "the font has no line metrics"),
            Self::Shape(error) => write!(f, "shaping failed: {error:?}"),
            Self::Subset(message) => write!(f, "subsetting failed: {message}"),
            Self::NewlineInLine => write!(
                f,
                "splitLineToFitWidth does not support newlines in the line"
            ),
        }
    }
}

impl std::error::Error for TextError {}
