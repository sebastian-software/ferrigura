//! Vertical metrics of a line: the text box browsers draw around it.
//!
//! The font's ascent and descent are rounded to whole pixels, each on its own, and the text box
//! is built from the rounded values. That is how Chromium lays out SVG text, and with the same
//! rounding the box matches the browser exactly.

use crate::error::TextError;
use crate::fonts::Face;

/// A face's vertical metrics at one size, in pixels and not rounded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    /// The distance from the baseline up to the top of the ascent.
    pub ascent: f64,
    /// The distance from the baseline down to the bottom of the descent, a positive number.
    pub descent: f64,
    /// The line gap the font declares. It is not part of the text box.
    pub line_gap: f64,
}

/// The text box of one line at one size, in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextBox {
    /// The ascent, rounded to a whole pixel.
    pub ascent: f64,
    /// The descent, rounded to a whole pixel.
    pub descent: f64,
    /// The distance from the baseline up to the top of the box. Equal to [`TextBox::ascent`].
    pub top: f64,
    /// The height of the box: the rounded ascent plus the rounded descent.
    pub height: f64,
}

/// The unrounded vertical metrics of `face` at `size` pixels.
///
/// The typographic metrics are used when the font asks for them, and the `hhea` metrics
/// otherwise, as browsers choose them.
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or has no line metrics.
pub fn font_metrics(face: &Face, size: f64) -> Result<FontMetrics, TextError> {
    let font = face.font()?;
    let metrics = font.metrics();
    let line = if metrics.use_typo_metrics {
        metrics.typo_line.or(metrics.hhea_line)
    } else {
        metrics.hhea_line.or(metrics.typo_line)
    }
    .ok_or(TextError::MissingLineMetrics)?;
    let units_per_em = f64::from(metrics.units_per_em);
    Ok(FontMetrics {
        ascent: line.ascender.to_f64() * size / units_per_em,
        descent: -line.descender.to_f64() * size / units_per_em,
        line_gap: line.line_gap.to_f64() * size / units_per_em,
    })
}

/// The text box of `face` at `size` pixels, with ascent and descent rounded to whole pixels.
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or has no line metrics.
pub fn text_box(face: &Face, size: f64) -> Result<TextBox, TextError> {
    let metrics = font_metrics(face, size)?;
    Ok(text_box_from(metrics))
}

/// Builds the text box from unrounded metrics: each of ascent and descent is rounded on its own.
#[must_use]
pub fn text_box_from(metrics: FontMetrics) -> TextBox {
    let ascent = metrics.ascent.round();
    let descent = metrics.descent.round();
    TextBox {
        ascent,
        descent,
        top: ascent,
        height: ascent + descent,
    }
}
