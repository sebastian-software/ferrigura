//! The texts of the map. The reference sets each label with d3's `text`, so the element holds the
//! label as the DB has it, with the placeholders of `encodeEntities` (`ﬂ°°`, `ﬂ°`, `¶ß`). The
//! core decodes the whole serialised SVG afterwards (`mermaidAPI.ts`, `cleanUpSvgCode`), so the
//! markup of a text is its escaped text with the placeholders decoded; the characters the reader sees
//! are the font's input.

use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, face, layout_width, measure_line, text_box,
};

use crate::RenderError;
use crate::common::Element;
use crate::common::decode_entities;
use crate::svg::escape;

/// The font size of the annotation lines (`wardleyRenderer.ts` 886, `fontSize`).
pub const ANNOTATION_FONT_SIZE: f64 = 11.0;

/// The markup of a label: the text escaped, then the placeholders of `encodeEntities` decoded, as
/// the final pass of the core does on the whole SVG.
#[must_use]
pub fn markup(text: &str) -> String {
    decode_entities(&escape(text))
}

/// The characters a label shows: the placeholders decoded, and the numeric entities they stand for
/// (`#35;` shows `#`) read as their characters. The font subset covers these characters.
#[must_use]
pub fn shown(text: &str) -> String {
    let decoded = decode_entities(text);
    let mut out = String::with_capacity(decoded.len());
    let mut rest = decoded.as_str();
    while let Some(start) = rest.find("&#") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find(';') {
            Some(end) if end > 0 && after[..end].bytes().all(|byte| byte.is_ascii_digit()) => {
                out.extend(after[..end].parse::<u32>().ok().and_then(char::from_u32));
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str("&#");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The characters the map shows, by the face they are set in, for the font subset. The reference
/// sets its titles, axis labels, notes, arrow names, annotation numbers and anchors in bold, and
/// the rest in the regular weight, so the subset covers both faces of the bundled family.
#[derive(Debug, Default)]
pub struct Drawn {
    regular: String,
    bold: String,
}

impl Drawn {
    /// A label set in the regular weight.
    pub fn regular(&mut self, text: &str) {
        self.regular.push_str(&shown(text));
    }

    /// A label set in the bold weight.
    pub fn bold(&mut self, text: &str) {
        self.bold.push_str(&shown(text));
    }

    /// The runs of the subset: one for each weight that draws text. An empty run would embed an
    /// empty subset, so it is left out.
    #[must_use]
    pub fn runs(&self) -> Vec<Run<'_>> {
        let mut runs = Vec::with_capacity(2);
        if !self.regular.is_empty() {
            runs.push(Run {
                text: &self.regular,
                face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
            });
        }
        if !self.bold.is_empty() {
            runs.push(Run {
                text: &self.bold,
                face: face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal),
            });
        }
        runs
    }
}

/// A `text` element whose content is the label's markup, with the attributes given in order.
#[must_use]
pub fn label(text: &str, attributes: Vec<(&str, String)>) -> Element {
    let mut element = Element::new("text");
    for (name, value) in attributes {
        element.set(name, value);
    }
    element.raw_text(markup(text))
}

/// The advance of a line of annotation text, as `getComputedTextLength` gives it: the text as the
/// DOM holds it (its placeholders included), in the regular face at the annotation size, snapped to
/// the layout unit as Chromium lays a run out (`layout_width`).
///
/// # Errors
///
/// Returns an error if the bundled face cannot be read or shaped.
pub fn annotation_width(text: &str) -> Result<f64, RenderError> {
    let runs = [Run {
        text,
        face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
    }];
    Ok(layout_width(measure_line(&runs, ANNOTATION_FONT_SIZE)?))
}

/// The height of the text box of an annotation line, as `getBBox().height` gives it: the box of the
/// regular face at the annotation size.
///
/// # Errors
///
/// Returns an error if the bundled face cannot be read.
pub fn annotation_height() -> Result<f64, RenderError> {
    let box_ = text_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        ANNOTATION_FONT_SIZE,
    )?;
    Ok(box_.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeholder_is_decoded_after_the_escape() {
        assert_eq!(markup("a<b ﬂ°°35¶ß"), "a&lt;b &#35;");
    }

    #[test]
    fn a_numeric_entity_is_shown_as_its_character() {
        assert_eq!(shown("ﬂ°°35¶ß and ﬂ°°233¶ß"), "# and é");
    }
}
