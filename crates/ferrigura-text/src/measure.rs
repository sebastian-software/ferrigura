//! Advance widths of text, shaped with the font's default features.
//!
//! The width of a run is the sum of its shaped advances, so kerning and ligatures are included,
//! as they are in the browser. Shaping a run never crosses the run's own boundaries.

use harfrust::font::Font;
use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};

use crate::error::TextError;
use crate::fonts::Face;

/// One run of text set in one face, for [`measure_line`].
#[derive(Debug, Clone, Copy)]
pub struct Run<'a> {
    /// The text of the run.
    pub text: &'a str,
    /// The face the run is set in.
    pub face: &'static Face,
}

/// The advance width, in pixels, of `text` set as one run in `face` at `size` pixels.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be parsed or the shaper rejects the text.
pub fn measure(face: &Face, text: &str, size: f64) -> Result<f64, TextError> {
    advance(face.font()?, text, size)
}

/// As [`measure`], for font data that is not one of the bundled faces, such as a subset made by
/// [`crate::embed`].
///
/// # Errors
///
/// Returns an error if `data` does not parse as a font or the shaper rejects the text.
pub fn measure_data(data: &[u8], text: &str, size: f64) -> Result<f64, TextError> {
    let font = Font::new(data.to_vec(), 0).ok_or(TextError::InvalidFont)?;
    advance(&font, text, size)
}

/// The advance width, in pixels, of a line made of runs at `size` pixels.
///
/// Each run is measured in its own face and the widths are summed. No kerning or ligature is
/// applied across two runs, which matches the reference: Mermaid puts each styled word in its own
/// `tspan`, and a change of face starts a new shaping run in the browser too.
///
/// Mermaid draws every word after the first with a leading space in its own `tspan`, and that
/// space takes the style of the word. To lay a line out the way Mermaid does, start each run with
/// the space that precedes it, as in `" bold"` in bold after `"Mixed"` in regular.
///
/// # Errors
///
/// Returns the first error [`measure`] reports for a run.
pub fn measure_line(runs: &[Run<'_>], size: f64) -> Result<f64, TextError> {
    let mut total = 0.0;
    for run in runs {
        total += measure(run.face, run.text, size)?;
    }
    Ok(total)
}

fn advance(font: &Font, text: &str, size: f64) -> Result<f64, TextError> {
    let shaper = ShaperFont::new(font);
    let mut buffer = Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    shape(&shaper, &mut buffer, ShapeOptions::new()).map_err(TextError::Shape)?;
    let units: i64 = buffer
        .glyph_positions()
        .iter()
        .map(|position| i64::from(position.x_advance))
        .sum();
    Ok(units as f64 * size / f64::from(font.units_per_em()))
}
