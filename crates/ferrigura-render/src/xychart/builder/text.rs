//! The size of the texts of the chart: the reference's `TextDimensionCalculatorWithFont`
//! (`chartBuilder/textDimensionCalculator.ts`) over `computeDimensionOfText` (`createText.ts`).
//!
//! The reference measures each text in a hidden `text` element of the given font size, and reads the
//! box of its tspan with `getBoundingClientRect`. That box is the glyph box of the text
//! ([`ferrigura_text::measure_box`] through [`collapsed_box`]), not its advance, and its height is the
//! text box, a whole number of pixels. The box is in screen pixels: the reference divides it by the
//! scale of the SVG on the screen (`getScreenCTM`, `textDimensionCalculator.ts` 31-42). So the text
//! is laid out at its size in screen pixels, `fontSize * scale`, and the box is divided by the scale
//! again. At a scale of 1 that changes nothing.

use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, text_box};

use crate::RenderError;
use crate::common::text::collapsed_box;

/// The width and the height of a text box, in user units.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextBox {
    pub width: f64,
    pub height: f64,
}

/// `getMaxDimension(texts, fontSize)`: the largest width and the largest height of the texts, not
/// their sum. No text gives zero. `scale` is the scale of the SVG on the screen.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn max_dimension<S: AsRef<str>>(
    texts: &[S],
    font_size: f64,
    scale: f64,
) -> Result<TextBox, RenderError> {
    let mut max = TextBox::default();
    for text in texts {
        let measured = measure(text.as_ref(), font_size, scale)?;
        max.width = max.width.max(measured.width);
        max.height = max.height.max(measured.height);
    }
    Ok(max)
}

/// The box of one text at `font_size`, measured on the screen at `scale`. An empty text has the empty
/// box, as the reference's tspan has no glyph to box.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn measure(text: &str, font_size: f64, scale: f64) -> Result<TextBox, RenderError> {
    if text.is_empty() {
        return Ok(TextBox::default());
    }
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let screen_size = font_size * scale;
    let extent = collapsed_box(sans, text, laid_out_size(screen_size))?;
    let height = text_box(sans, screen_size)?.height;
    Ok(TextBox {
        width: extent.width() / scale,
        height: height / scale,
    })
}

/// The size Chromium lays text out at: floored to 1/100 px (`common::text`, `laid_out_size`).
fn laid_out_size(size: f64) -> f64 {
    ((size * 100.0) + 1e-6).floor() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_a_scale_of_one_the_box_is_the_text_box() {
        // The probe of the reference (NOTES 6.4): `Sales Revenue` at 14 px is 89.734375 wide, and its
        // box is 17 px high. The height is a whole number of pixels.
        let text = measure("Sales Revenue", 14.0, 1.0).expect("the font reads");
        assert_eq!(text.height, 17.0);
        assert_eq!(text.width, 89.734375);
    }

    #[test]
    fn a_scaled_svg_measures_the_text_on_the_screen_and_divides_back() {
        // Measured at 6 px (20 px at 0.3) the box is 8 screen pixels: 8 / 0.3 user units.
        let text = measure("Wide chart scaled down to fit", 20.0, 0.3).expect("the font reads");
        assert_eq!(text.height, 8.0 / 0.3);
        assert_eq!(text.width, 255.10416666666669);
    }

    #[test]
    fn an_empty_text_has_no_box() {
        assert_eq!(
            measure("", 14.0, 1.0).expect("the font reads"),
            TextBox::default()
        );
    }
}
