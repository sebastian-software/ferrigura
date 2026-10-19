//! The text of a railroad diagram: its measurement and the characters it shows.
//!
//! The reference measures each text with `getBBox` of a temporary `<text>` in the configured font
//! (`railroadRenderer.ts` 74-93). The width is the box of the text (its advance and the glyph ink
//! that reaches past it), and the height is the text box of the font, whatever the text
//! (`railroadRenderer.ts` 86, `measureText`). Chromium's rules for both are in `ferrigura-text`, as
//! [`crate::common::text::collapsed_box`] and `text_box` apply them; the numbers are float32 values
//! of the browser, so the width and the height are rounded to float32 as `getBBox` returns them.
//!
//! The text is measured as the database holds it, with the entity placeholders that the decoding of
//! the whole SVG string later turns into characters (`mermaidAPI.ts` 359). The text that is drawn is
//! the decoded one ([`shown_text`]), so the two differ where the user wrote an entity.

use std::collections::HashMap;

use ferrigura_text::{
    Face, FontFamily, FontStyle, FontWeight, TextError, face, measure_box, text_box,
};

use super::layout::{Measure, TextSize};
use crate::common::text::collapse_white_space;

/// The text the reference measures with: the configured font, which ferrigura draws in its bundled
/// sans face (decision 0005), at the configured size. The rule names are measured in the regular
/// weight, as the reference does, though they are drawn in bold (`.railroad-rule-name`).
pub struct TextMeasure {
    face: &'static Face,
    size: f64,
    /// The measurements of the texts so far, by text: `measureText` caches them the same way
    /// (`railroadRenderer.ts` 64, 75-77 and 90), so one text has one measurement.
    cache: HashMap<String, TextSize>,
}

impl TextMeasure {
    /// A measurement at `size` pixels in the regular face.
    #[must_use]
    pub fn new(size: f64) -> Self {
        Self {
            face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
            size,
            cache: HashMap::new(),
        }
    }
}

impl Measure for TextMeasure {
    fn text_size(&mut self, text: &str) -> Result<TextSize, TextError> {
        if let Some(size) = self.cache.get(text) {
            return Ok(*size);
        }
        let size = measure_text(self.face, text, self.size)?;
        self.cache.insert(text.to_owned(), size);
        Ok(size)
    }
}

/// The box `getBBox` gives a text of the reference's `<text>` at `size` pixels.
///
/// A text with no glyph (the empty string, or only white space, which SVG collapses away) has the
/// empty box, 0 by 0 (`svg/c12-emptyterminal.svg`).
fn measure_text(face: &'static Face, text: &str, size: f64) -> Result<TextSize, TextError> {
    let collapsed = collapse_white_space(text);
    if collapsed.is_empty() {
        return Ok(TextSize {
            width: 0.0,
            height: 0.0,
        });
    }
    // The size Chromium lays SVG text out at, floored to 1/100 px (`common::text`'s `laid_out_size`,
    // which is private to that module); the text box applies the same rule to its own size.
    let laid_out = ((size * 100.0) + 1e-6).floor() / 100.0;
    let width = measure_box(face, &collapsed, laid_out)?.width();
    let height = text_box(face, size)?.height;
    Ok(TextSize {
        width: f64::from(width as f32),
        height: f64::from(height as f32),
    })
}

/// The text an SVG `<text>` shows for a string of the diagram ([`crate::common::shown_text`]).
pub use crate::common::shown_text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_text_has_the_empty_box() {
        let mut measure = TextMeasure::new(14.0);
        let size = measure.text_size("").unwrap();
        assert_eq!((size.width, size.height), (0.0, 0.0));
        let blank = measure.text_size("  \n ").unwrap();
        assert_eq!((blank.width, blank.height), (0.0, 0.0));
    }

    /// Probed (`render-probe.mts`, `results.json`): `0` at 14 px is 9.078125 wide and 17 high, and
    /// `digit =` is 40.40625 wide. The height does not depend on the text.
    #[test]
    fn the_probed_boxes_of_the_harness_face_are_the_browsers() {
        let mut measure = TextMeasure::new(14.0);
        let zero = measure.text_size("0").unwrap();
        assert_eq!((zero.width, zero.height), (9.078_125, 17.0));
        let rule = measure.text_size("digit =").unwrap();
        assert_eq!((rule.width, rule.height), (40.406_25, 17.0));
    }

    #[test]
    fn a_line_break_in_a_text_measures_as_a_space() {
        // `svg/c13-newline.svg`: `a\nb` is drawn as `a b`, with the box of that line.
        let mut measure = TextMeasure::new(14.0);
        let joined = measure.text_size("a\nb").unwrap();
        let spaced = measure.text_size("a b").unwrap();
        assert_eq!(joined, spaced);
    }

    /// The helper gives the content of the text node: the element writes it escaped, so the markup
    /// is the reference's, and the browser reads the same characters.
    #[test]
    fn shown_text_is_the_content_of_the_text_node() {
        assert_eq!(shown_text("a < b"), "a < b");
        // A placeholder of a typed `&` decodes to the character (`mermaidAPI.ts` 359).
        assert_eq!(shown_text("x ﬂ° y"), "x & y");
        // A `&amp;` the database holds is shown as those five characters, as the reference shows it.
        assert_eq!(shown_text("a&amp;b"), "a&amp;b");
    }
}
