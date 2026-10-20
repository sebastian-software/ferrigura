//! Advance widths of text, shaped with the font's default features.
//!
//! The width of a run is the sum of its shaped advances, so kerning and ligatures are included,
//! as they are in the browser. Shaping a run never crosses the run's own boundaries.

use harfrust::font::Font;
use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};

use crate::error::TextError;
use crate::fonts::{Face, FontFamily, FontStyle, FontWeight, face};

/// One run of text set in one face, for [`measure_line`].
#[derive(Debug, Clone, Copy)]
pub struct Run<'a> {
    /// The text of the run.
    pub text: &'a str,
    /// The face the run is set in.
    pub face: &'static Face,
}

/// The faces a text's font-family list falls back to, in order, after the text's own face: the
/// arrows, the text symbols and the emoji.
///
/// The three cover disjoint code points (the emoji face holds no text-default symbol, and the text
/// symbols hold no emoji-presentation code point), so the order decides nothing among them.
#[must_use]
pub fn fallback_faces() -> [&'static Face; 3] {
    [
        face(FontFamily::Arrows, FontWeight::Normal, FontStyle::Normal),
        face(
            FontFamily::TextSymbols,
            FontWeight::Normal,
            FontStyle::Normal,
        ),
        face(FontFamily::Emoji, FontWeight::Normal, FontStyle::Normal),
    ]
}

/// The stretches of a run as a browser sets them with the run's font-family list, which is the
/// run's own face and then the fallback faces of [`fallback_faces`].
///
/// A character the run's face lacks is set in the first fallback face that covers it. Every other
/// character stays in the run's face, where a character that no face covers is drawn with the
/// face's own missing glyph. Consecutive characters in one face form one stretch, so the stretches
/// follow each other in the order of the text and cover all of it. A run without any such character
/// comes back as itself, and so does an empty run, which keeps its place in the embedded subsets.
/// A run in a fallback face is not split.
///
/// This is the per-character fallback that Chromium makes for a font-family list: a run of text is
/// split where the first family lacks a character, and each part is shaped in the family that has
/// it. Every measurement and every embedded subset goes through it, so a viewer draws the characters
/// in the face they were measured in.
#[must_use]
pub fn fallback_runs(run: Run<'_>) -> Vec<Run<'_>> {
    if run.text.is_empty() || !matches!(run.face.family(), FontFamily::Sans | FontFamily::Mono) {
        return vec![run];
    }
    let fallbacks = fallback_faces();
    let mut stretches = Vec::new();
    let mut start = 0;
    let mut current: &'static Face = run.face;
    for (index, character) in run.text.char_indices() {
        let chosen = if run.face.covers(character) {
            run.face
        } else {
            fallbacks
                .into_iter()
                .find(|fallback| fallback.covers(character))
                .unwrap_or(run.face)
        };
        if !std::ptr::eq(chosen, current) {
            if index > start {
                stretches.push(Run {
                    text: &run.text[start..index],
                    face: current,
                });
            }
            start = index;
            current = chosen;
        }
    }
    stretches.push(Run {
        text: &run.text[start..],
        face: current,
    });
    stretches
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

/// The advance width, in pixels, of `text` set as one run in `face` at `size` pixels. The characters
/// that `face` lacks and a fallback face has are set in that face ([`fallback_runs`]), and the
/// advances of the stretches are summed without any snapping: a caller snaps the whole run.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be parsed or the shaper rejects the text.
pub fn measure(face: &'static Face, text: &str, size: f64) -> Result<f64, TextError> {
    let mut total = 0.0;
    for stretch in fallback_runs(Run { text, face }) {
        total += advance(stretch.face.font()?, stretch.text, size)?;
    }
    Ok(total)
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
/// Each run is measured in its own face, with the fallback of [`measure`], and the widths are summed.
/// No kerning or ligature is applied across two runs, which matches the reference: Mermaid puts each
/// styled word in its own `tspan`, and a change of face starts a new shaping run in the browser too.
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
