//! The text of a treemap, measured as the reference's browser measures it: `getComputedTextLength`
//! (the advance of the collapsed text, snapped to the layout unit, which the fitting loops compare)
//! and the `getBBox` of one line (its ink and advance, and the text box moved by the dominant
//! baseline), which the viewBox takes the union of.
//!
//! The treemap's text is set in the sans face of the bundled faces, in regular weight, bold for the
//! section labels and italic for the section values. The fitting loops read the advance at the
//! current font size; a width that is a fraction of a pixel off can flip a decision.

use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, text_box, x_height};

use crate::RenderError;
use crate::common::text::{collapse_white_space, collapsed_box, collapsed_width};
use crate::common::viewbox::BBox;

/// `value` rounded to Chromium's 1/64 px layout unit.
fn layout_unit(value: f64) -> f64 {
    (value * 64.0).round() / 64.0
}

/// The `text-anchor` of a line: where its box starts relative to its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// `start` (the default): the text starts at the position.
    Start,
    /// `middle`: the text is centred on the position.
    Middle,
    /// `end`: the text ends at the position.
    End,
}

/// The `dominant-baseline` of a line, which moves its baseline from the position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    /// `middle`: the middle of the x-height sits on the position.
    Middle,
    /// `hanging`: the top of the text box sits on the position.
    Hanging,
}

/// The font of a line: its weight and its style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Font {
    /// Bold for the section labels, regular otherwise.
    pub weight: FontWeight,
    /// Italic for the section values.
    pub style: FontStyle,
}

impl Font {
    /// Regular text: the labels and values of the leaves, and the title.
    pub const REGULAR: Self = Self {
        weight: FontWeight::Normal,
        style: FontStyle::Normal,
    };
    /// The section labels.
    pub const BOLD: Self = Self {
        weight: FontWeight::Bold,
        style: FontStyle::Normal,
    };
    /// The section values.
    pub const ITALIC: Self = Self {
        weight: FontWeight::Normal,
        style: FontStyle::Italic,
    };
}

/// `getComputedTextLength()` of a text at `size` pixels in `font`: the advance of the collapsed text
/// (white space collapsed as the SVG text does), and 0 for a text with no glyph.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn advance(text: &str, size: f64, font: Font) -> Result<f64, RenderError> {
    let face = face(FontFamily::Sans, font.weight, font.style);
    Ok(collapsed_width(face, text, size)?)
}

/// The `getBBox` of one line of `text`, drawn at `(x, y)` with the anchor and the baseline, or
/// `None` when the text has no glyph. The box is the ink and advance of the line shifted by the
/// anchor, and the text box (rounded ascent and descent) moved by the baseline.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn line_box(
    text: &str,
    (x, y): (f64, f64),
    size: f64,
    font: Font,
    anchor: Anchor,
    baseline: Baseline,
) -> Result<Option<BBox>, RenderError> {
    if collapse_white_space(text).is_empty() {
        return Ok(None);
    }
    let face = face(FontFamily::Sans, font.weight, font.style);
    let ink = collapsed_box(face, text, size)?;
    let width = collapsed_width(face, text, size)?;
    let shift = match anchor {
        Anchor::Start => 0.0,
        Anchor::Middle => width / 2.0,
        Anchor::End => width,
    };
    let metrics = text_box(face, size)?;
    let baseline_y = y + match baseline {
        // The x-height is laid out in 1/64 px (14 px gives 6.9375, not 6.944), and the middle is half of it.
        Baseline::Middle => layout_unit((x_height(face, size)?) / 2.0),
        // The hanging baseline is four fifths of the ascent, in the same layout unit (23 px: 17.59375).
        Baseline::Hanging => layout_unit(0.8 * metrics.ascent),
    };
    let left = x + ink.left - shift;
    let right = x + ink.right - shift;
    let top = baseline_y - metrics.ascent;
    let bottom = baseline_y + metrics.descent;
    Ok(Some(BBox {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The box of a line, which the tests expect to have one.
    fn boxed(text: &str, x: f64, anchor: Anchor) -> BBox {
        match line_box(
            text,
            (x, 50.0),
            12.0,
            Font::REGULAR,
            anchor,
            Baseline::Middle,
        ) {
            Ok(Some(bounds)) => bounds,
            _ => BBox {
                x: f64::NAN,
                y: f64::NAN,
                width: f64::NAN,
                height: f64::NAN,
            },
        }
    }

    #[test]
    fn a_text_with_no_glyph_has_no_box_and_no_advance() {
        assert_eq!(advance("  ", 12.0, Font::REGULAR).ok(), Some(0.0));
        let boxed = line_box(
            " ",
            (0.0, 0.0),
            12.0,
            Font::REGULAR,
            Anchor::Start,
            Baseline::Middle,
        );
        assert!(matches!(boxed, Ok(None)));
    }

    #[test]
    fn a_bold_label_is_wider_than_the_regular_one() {
        let regular = advance("Label", 12.0, Font::REGULAR).unwrap_or(0.0);
        let bold = advance("Label", 12.0, Font::BOLD).unwrap_or(0.0);
        assert!(bold > regular);
    }

    #[test]
    fn a_centred_line_is_shifted_by_half_its_advance() {
        let width = advance("Label", 12.0, Font::REGULAR).unwrap_or(0.0);
        let centred = boxed("Label", 100.0, Anchor::Middle);
        let start = boxed("Label", 100.0, Anchor::Start);
        assert!((start.x - centred.x - width / 2.0).abs() < 0.5);
    }

    #[test]
    fn the_hanging_baseline_is_four_fifths_of_the_rounded_ascent_in_layout_units() {
        // 23 px: the rounded ascent is 22, and 0.8 of it is 17.59375 on the 1/64 px grid.
        assert_eq!(layout_unit(0.8 * 22.0), 17.59375);
        assert_eq!(layout_unit(3.472), 3.46875);
    }
}
