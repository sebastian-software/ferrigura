//! The horizontal box `getBBox` gives a line of SVG text.
//!
//! Chromium takes the box of a `<text>` (and of each `<tspan>`) as the union of two things for every
//! run: its advance, snapped to the layout unit ([`layout_width`]), and the outline of every glyph in
//! it. A glyph whose ink reaches past its advance widens the box: the `j` of Lexica Ultralegible
//! starts 0.812 px left of its origin at 14 px, and its italic `f` ends past its advance. The box of
//! a whole line is therefore not its advance. Its vertical extent is the text box of
//! [`crate::text_box`]; the ink never changes it (probed at 10 to 18.5 px in all four styles).
//!
//! The rule is checked against Chromium in `tests/extents.rs`, on the `extents` section of
//! `tests/fixtures/chromium.json` (`tools/port/text/chromium-fixture.ts`): every printable glyph of
//! the Lexica faces in four styles, words that overhang at either end, and lines whose runs change
//! style. Chromium's canvas ink bounds give the same glyph boxes as the outlines read here.

use read_fonts::types::GlyphId;
use read_fonts::{FontRef, TableProvider};

use crate::error::TextError;
use crate::fonts::Face;
use crate::measure::{Run, fallback_runs, layout_width, shape_run};

/// The horizontal extent of a line, relative to the start of its first run: the left edge and the
/// right edge in pixels, as `getBBox` reports them (`x` and `x + width`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Extent {
    /// The left edge. Zero, or negative where a glyph's ink reaches left of its origin.
    pub left: f64,
    /// The right edge: the end of the advance, or further where a glyph's ink reaches past it.
    pub right: f64,
}

impl Extent {
    /// The width of the box, `right - left`.
    #[must_use]
    pub fn width(self) -> f64 {
        self.right - self.left
    }
}

/// The box `getBBox` gives a run of text set in one face at `size` pixels, relative to the start of
/// the run. A text with no glyphs has the empty box at the origin.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read or shaped.
pub fn measure_box(face: &'static Face, text: &str, size: f64) -> Result<Extent, TextError> {
    measure_line_box(&[Run { text, face }], size)
}

/// The box `getBBox` gives a line made of runs at `size` pixels: the runs follow each other, each
/// starts where the snapped advances of the runs before it end, and the box is the union of the
/// advance of every run and the ink of every glyph. This is the box of a `<text>` whose `<tspan>`s
/// carry the runs' styles, as Mermaid's styled words are. The line's own advance, the sum of the
/// snapped run advances, is its width for anchoring (`text-anchor` shifts the line by it), so the
/// box of an anchored line is this extent moved by the anchor's shift.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or shaped.
pub fn measure_line_box(runs: &[Run<'_>], size: f64) -> Result<Extent, TextError> {
    let mut extent = Extent::default();
    let mut start = 0.0_f64;
    for run in runs {
        // A run is one `tspan` and is snapped once, as a whole, whatever faces its characters fall
        // back to (`fallback_runs`): the stretches' advances are summed unsnapped, and the glyphs of
        // a later stretch start where the earlier ones end.
        let mut offset = 0.0_f64;
        for stretch in fallback_runs(*run) {
            let font = stretch.face.font()?;
            let units_per_em = f64::from(font.units_per_em());
            let shaped = shape_run(font, stretch.text)?;
            if !shaped.glyphs.is_empty() {
                let outlines = Outlines::of(stretch.face)?;
                for placed in &shaped.glyphs {
                    let Some((x_min, x_max)) = outlines.bounds(placed.glyph)? else {
                        continue;
                    };
                    let scale = size / units_per_em;
                    let left = start + offset + placed.x as f64 * scale + x_min * scale;
                    let right = start + offset + placed.x as f64 * scale + x_max * scale;
                    extent.left = extent.left.min(left);
                    extent.right = extent.right.max(right);
                }
            }
            offset += shaped.advance as f64 * size / units_per_em;
        }
        let advance = layout_width(offset);
        extent.right = extent.right.max(start + advance);
        start += advance;
    }
    Ok(extent)
}

/// The glyph outlines of one face, read from its `loca` and `glyf` tables.
struct Outlines<'a> {
    loca: read_fonts::tables::loca::Loca<'a>,
    glyf: read_fonts::tables::glyf::Glyf<'a>,
}

impl<'a> Outlines<'a> {
    fn of(face: &'a Face) -> Result<Self, TextError> {
        let font = FontRef::new(face.data()).map_err(|_| TextError::InvalidFont)?;
        Ok(Self {
            loca: font.loca(None).map_err(|_| TextError::InvalidFont)?,
            glyf: font.glyf().map_err(|_| TextError::InvalidFont)?,
        })
    }

    /// The horizontal bounds of a glyph's outline in font units, or `None` for a glyph without one
    /// (a space).
    fn bounds(&self, glyph: u32) -> Result<Option<(f64, f64)>, TextError> {
        let outline = self
            .loca
            .get_glyf(GlyphId::new(glyph), &self.glyf)
            .map_err(|_| TextError::InvalidFont)?;
        Ok(outline.map(|outline| (f64::from(outline.x_min()), f64::from(outline.x_max()))))
    }
}
