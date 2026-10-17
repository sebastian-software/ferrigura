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

/// The width of a text run as Chromium lays it out, in pixels: its advance, rounded up to a
/// multiple of 1/64 px.
///
/// Blink keeps positions in layout units of 1/64 px and converts each run's advance with a round up
/// (`LayoutUnit::FromFloatCeil`), so the run after it starts on the grid, and a line's width, the
/// `getComputedTextLength` that Mermaid's wrapping compares, is the sum of the snapped runs. A width
/// within float noise of a grid point is taken to be on it. Verified against Chromium in
/// `tests/labels_chromium.rs` (`docs-port/flowchart/render-notes/labels.md`).
#[must_use]
pub fn layout_width(advance: f64) -> f64 {
    const UNITS_PER_PIXEL: f64 = 64.0;
    ((advance * UNITS_PER_PIXEL) - 1e-3).ceil() / UNITS_PER_PIXEL
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
    let shaped = shape_run(font, text)?;
    Ok(shaped.advance as f64 * size / f64::from(font.units_per_em()))
}

/// One glyph of a shaped run, with the pen position its outline is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlacedGlyph {
    /// The glyph id in the font the run was shaped in.
    pub(crate) glyph: u32,
    /// The x of the glyph's origin from the start of the run, in font units: the pen position plus
    /// the glyph's own offset.
    pub(crate) x: i64,
}

/// A shaped run: its glyphs in order and its advance, both in font units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShapedRun {
    pub(crate) glyphs: Vec<PlacedGlyph>,
    /// The sum of the glyph advances, in font units.
    pub(crate) advance: i64,
}

/// Shapes `text` with the font's default features, and reports where each glyph is drawn.
pub(crate) fn shape_run(font: &Font, text: &str) -> Result<ShapedRun, TextError> {
    let shaper = ShaperFont::new(font);
    let mut buffer = Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    shape(&shaper, &mut buffer, ShapeOptions::new()).map_err(TextError::Shape)?;
    let mut pen: i64 = 0;
    let mut glyphs = Vec::with_capacity(buffer.len());
    for (info, position) in buffer.glyph_infos().iter().zip(buffer.glyph_positions()) {
        glyphs.push(PlacedGlyph {
            glyph: info.glyph_id,
            x: pen + i64::from(position.x_offset),
        });
        pen += i64::from(position.x_advance);
    }
    Ok(ShapedRun {
        glyphs,
        advance: pen,
    })
}

#[cfg(test)]
mod tests {
    use super::layout_width;

    #[test]
    fn a_width_is_rounded_up_to_the_layout_unit() {
        // 38.1 px is not on the grid of 1/64 px; the next grid point above it is 38.109375.
        assert_eq!(layout_width(38.1), 38.109375);
        // 26 px is on the grid and stays where it is, even with float noise.
        assert_eq!(layout_width(26.0), 26.0);
        assert_eq!(layout_width(26.000_000_01), 26.0);
        // Float noise just above a grid point is still on it; a clear excess is rounded up.
        assert_eq!(layout_width(10.015_625_5), 10.015625);
        assert_eq!(layout_width(10.02), 10.03125);
    }
}
