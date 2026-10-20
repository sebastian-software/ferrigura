//! The text of the diagram: what a text is written as in the SVG, the characters it shows, and the
//! width of an item's text as the reference measures it (`cynefinRenderer.ts` 276-286).
//!
//! The reference sets the labels as DOM text. Its whole SVG string is decoded afterwards
//! (`mermaidAPI.ts` 359), so an entity placeholder of the DB (`ﬂ°amp¶ß`) is written as `&amp;` and
//! shows as `&`; the badge is measured before that, on the placeholder text. Both are kept here.

use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, TextError, face};

use crate::common::text::collapsed_box;
use crate::svg::entity_markup;

use super::layout::ITEM_PADDING_X;

/// The markup of a text: its characters escaped as the DOM serialises them, then the entity
/// placeholders decoded (the whole-SVG pass of `mermaidAPI.ts` 359).
#[must_use]
pub(super) fn markup(text: &str) -> String {
    entity_markup(text)
}

/// The width of a label's text as `getBBox().width` gives it, or the fallback of the reference when
/// that is zero (`label.length * 7`, lines 277-284): the box of the collapsed text at `size` pixels,
/// in the regular face. The label is measured as it is in the DB, with its placeholders.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn label_width(label: &str, size: f64) -> Result<f64, TextError> {
    let measured = collapsed_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        label,
        size,
    )?
    .width();
    if measured > 0.0 {
        Ok(measured)
    } else {
        // `label.length` counts UTF-16 units.
        Ok(label.encode_utf16().count() as f64 * 7.0)
    }
}

/// The width of an item's badge: the label's width and the padding on both sides (line 286).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn badge_width(label: &str, size: f64) -> Result<f64, TextError> {
    Ok(label_width(label, size)? + ITEM_PADDING_X * 2.0)
}

/// The regular face of the item and arrow texts.
#[must_use]
pub(super) fn regular() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

/// The italic face of the subtitles.
#[must_use]
pub(super) fn italic() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Italic)
}

/// The bold face of the labels and the title.
#[must_use]
pub(super) fn bold() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::shown_text;

    #[test]
    fn a_placeholder_is_written_as_its_entity_and_shown_as_its_character() {
        assert_eq!(markup("ﬂ°amp¶ß"), "&amp;");
        assert_eq!(shown_text("ﬂ°amp¶ß"), "&");
        assert_eq!(shown_text("a ﬂ°°35¶ß b"), "a # b");
    }

    #[test]
    fn a_markup_character_is_escaped_and_shown_as_itself() {
        assert_eq!(markup("a < b & c"), "a &lt; b &amp; c");
        assert_eq!(shown_text("a < b & c"), "a < b & c");
        assert_eq!(shown_text("x &y z"), "x &y z");
    }
}
