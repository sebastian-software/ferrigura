//! Vertical metrics of a line: the text box browsers draw around it.
//!
//! The font's ascent and descent are rounded to whole pixels, each on its own, a tie going down
//! (28.5 becomes 28), and the text box is built from the rounded values. The size is floored to
//! 1/100 px first. That is how Chromium lays out SVG text, and with the same rounding the box
//! matches the browser exactly. The fixture in `tests/fixtures/chromium.json` holds the evidence.

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
/// The size is first floored to 1/100 px, as Chromium sets SVG text (see `laid_out_size`).
///
/// # Errors
///
/// Returns an error if the font cannot be parsed or has no line metrics.
pub fn text_box(face: &Face, size: f64) -> Result<TextBox, TextError> {
    let metrics = font_metrics(face, laid_out_size(size))?;
    Ok(text_box_from(metrics))
}

/// The x-height of `face` at `size` pixels, unrounded: the `sxHeight` of the OS/2 table scaled to the
/// size. Browsers take the length `ex` from it, so `font-size: 4ex` is four times this value
/// (`timeline` titles, `docs-port/timeline/NOTES.md` 5.4). The bundled faces declare 496 units in 1000.
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
/// Chromium does this for both ascent and descent at every whole size tested, and no other rule
/// fits them all (`tests/fixtures/chromium.json`): round half up misses every tie (10, 30 and 50 px
/// among them), round half to even misses the ties at 10 and 50 px, and rounding ascent plus
/// descent together gives a height one pixel too large at 50 px.
fn round_half_down(value: f64) -> f64 {
    if value.fract() == 0.5 {
        value.floor()
    } else {
        value.round()
    }
}
