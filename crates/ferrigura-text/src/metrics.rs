//! Vertical metrics of a line: the text box browsers draw around it.
//!
//! Chromium's macOS TrueType line metrics use normalized 16.16 values from CoreText, scaled to the size,
//! then rounded independently to whole pixels. Decimal ratios such as Lexica's 0.95 fall just
//! below a half-pixel tie; exactly representable ratios such as Inter's 1984/2048 round up at a tie.
//! The size is floored to 1/100 px first. CFF follows its outline-unit precision instead.
//! These are empirical Chromium/macOS compatibility rules, verified by the retained fixtures;
//! other SVG viewers can use different line metrics.

use read_fonts::{FontRef, TableProvider};

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
/// The typographic metrics are used when `USE_TYPO_METRICS` is set; otherwise the first
/// nonzero `hhea` or typographic metrics are used, with the OS/2 win metrics as the final
/// fallback. The selection is independent of the host operating system.
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or has no line metrics.
pub fn font_metrics(face: &Face, size: f64) -> Result<FontMetrics, TextError> {
    let font = face.font()?;
    let metrics = font.metrics();
    let line = metrics.h_line().ok_or(TextError::MissingLineMetrics)?;
    let units_per_em = f64::from(metrics.units_per_em);
    Ok(FontMetrics {
        ascent: line.ascender.to_f64() * size / units_per_em,
        descent: -line.descender.to_f64() * size / units_per_em,
        line_gap: line.line_gap.to_f64() * size / units_per_em,
    })
}

/// The text box of `face` at `size` pixels, with ascent and descent rounded to whole pixels.
///
/// The size is first floored to 1/100 px, as Chromium sets SVG text (see `laid_out_size`).
/// TrueType normalized metrics are truncated to 16.16 before scaling, matching CoreText's line
/// metrics. CFF metrics retain their outline-unit precision;
/// Chromium then rounds to the nearest pixel, with exact positive ties going up.
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or has no line metrics.
pub fn text_box(face: &Face, size: f64) -> Result<TextBox, TextError> {
    let size = laid_out_size(size);
    let metrics = font_metrics(face, 1.0)?;
    let font = FontRef::new(face.data()).map_err(|_| TextError::InvalidFont)?;
    let is_cff = font.cff().is_ok() || font.cff2().is_ok();
    let normalize = |metric: f64| {
        if is_cff {
            metric
        } else {
            (metric * 65536.0).trunc() / 65536.0
        }
    };
    let ascent = f64::from((normalize(metrics.ascent) * size) as f32).round();
    let descent = f64::from((normalize(metrics.descent) * size) as f32).round();
    Ok(TextBox {
        ascent,
        descent,
        top: ascent,
        height: ascent + descent,
    })
}

/// The x-height of `face` at `size` pixels, unrounded: the `sxHeight` of the OS/2 table scaled to the
/// size. Browsers take the length `ex` from it, so `font-size: 4ex` is four times this value
/// (`timeline` titles, `docs/diagrams/timeline.md#learnings`). The bundled faces declare 496 units in 1000.
///
/// # Errors
///
/// Returns an error if the font does not parse or has no `sxHeight`.
pub fn x_height(face: &Face, size: f64) -> Result<f64, TextError> {
    let font = FontRef::new(face.data()).map_err(|_| TextError::InvalidFont)?;
    let units_per_em = font
        .head()
        .map_err(|_| TextError::InvalidFont)?
        .units_per_em();
    let os2 = font.os2().map_err(|_| TextError::MissingLineMetrics)?;
    let sx_height = os2.sx_height().ok_or(TextError::MissingLineMetrics)?;
    Ok(f64::from(sx_height) * size / f64::from(units_per_em))
}

/// Builds the text box from unrounded metrics: each of ascent and descent is rounded on its own,
/// a tie going down.
#[must_use]
pub fn text_box_from(metrics: FontMetrics) -> TextBox {
    let ascent = round_half_down(metrics.ascent);
    let descent = round_half_down(metrics.descent);
    TextBox {
        ascent,
        descent,
        top: ascent,
        height: ascent + descent,
    }
}

/// The size Chromium lays SVG text out at: `size` floored to 1/100 px. At 10.005 px the box is the
/// one of 10.00 px (9 and 3), not of 10.005 px (10 and 3).
///
/// The epsilon keeps a size whose product with 100 falls just short of a whole number, such as
/// 0.29 (28.999999999999996), from flooring one hundredth too low.
fn laid_out_size(size: f64) -> f64 {
    ((size * 100.0) + 1e-6).floor() / 100.0
}

/// Rounds a positive metric to a whole pixel, a tie going down: 28.5 becomes 28, 28.6 becomes 29.
///
/// This is the compatibility rule of `text_box_from`, which accepts already-scaled metrics.
/// `text_box` can instead normalize the original face metrics before scaling and rounding.
fn round_half_down(value: f64) -> f64 {
    if value.fract() == 0.5 {
        value.floor()
    } else {
        value.round()
    }
}
