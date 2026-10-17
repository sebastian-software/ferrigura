//! The text measurement of `packages/mermaid/src/utils.ts`: `calculateTextDimensions` and the
//! `calculateTextWidth` and `calculateTextHeight` that wrap it, `wrapLabel` and `breakString`.
//!
//! The reference measures each line with `getBBox` of a `<text>` drawn in the configured font, and
//! ferrigura measures it with `ferrigura-text`, as Chromium lays it out (`docs-port/decisions/0005`).
//! The rules, each checked against Chromium in `tests/common_text_chromium.rs`:
//!
//! - the lines are the text split at `<br>` in any case, with an optional slash and blanks, as
//!   `lineBreakRegex` (`diagrams/common/common.ts`) splits them;
//! - white space collapses as CSS does in an SVG `<text>`: runs of blanks, tabs and newlines become
//!   one space and the ends are dropped, so `"a  b"` measures as `"a b"`;
//! - a line that is empty is measured as a zero-width space (`line || ZERO_WIDTH_SPACE`): it has no
//!   width and the height of one text line;
//! - a line that is only white space has no glyphs, and `getBBox` returns an empty box, which the
//!   reference reports as `svg element not in render tree`; that is an error here too;
//! - the width of a line is the box `getBBox` gives it ([`measure_box`]): its advance snapped to
//!   Chromium's 1/64 px layout unit ([`layout_width`]), and the glyph outlines where they reach past
//!   the advance (a leading `j`, an italic `f`), rounded to a whole pixel (`Math.round`); the height
//!   of a line is its text box, a whole number of
//!   pixels, rounded the same way; the block is the widest line, the sum of the line heights, and the
//!   height of one line;
//! - the reference also measures in `sans-serif` and keeps those values only when they are larger in
//!   width, height and line height at once. ferrigura has no system sans-serif face, so it measures
//!   the configured font. On this machine the reference keeps the configured font for all 492 probed
//!   requests that have both measurements (`tools/port/text/common-fixture.ts`); the test
//!   `the_reference_keeps_the_configured_font_for_every_probed_text` holds that finding.
//!
//! The reference memoises `calculateTextDimensions`, `wrapLabel` and `breakString` by a string key
//! built from the arguments (`${text}${fontSize}${fontWeight}${fontFamily}` and so on), and the key
//! of two different requests can be the same: the second then returns the first result. [`TextMemo`]
//! keeps the same keys for one render, so the same collisions happen as in one page of the
//! reference. The reference's cache lives as long as its page, which holds several renders; a
//! ferrigura render does not share a cache with another, so a collision across renders is not
//! reproduced (open in the report of the task).

use std::collections::HashMap;

use ferrigura_text::{
    Extent, Face, FontFamily, FontStyle, FontWeight, TextError, face, layout_width, measure,
    measure_box, text_box,
};

use super::is_js_whitespace;
use crate::svg::number;

/// What the reference passes as `TextDimensionConfig` (`types.ts`) to the measurement.
///
/// `font_size` is in pixels: the reference writes it as `${fontSize}px`. `font_weight` is the CSS
/// number (400 is normal, 700 bold). `font_family` is the configured family as the reference's
/// CSS takes it, quotes included (the harness forces `"Lexica Ultralegible"`); it is part of the
/// memo key and is measured in the bundled sans face.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// The font size in pixels.
    pub font_size: f64,
    /// The CSS font weight.
    pub font_weight: f64,
    /// The configured font family, as the CSS text of the reference has it.
    pub font_family: String,
}

/// What the reference passes as `WrapLabelConfig` to `wrapLabel`: the measurement's style and the
/// string that joins the lines (`<br/>` by default).
#[derive(Debug, Clone, PartialEq)]
pub struct WrapStyle {
    /// The measurement's style.
    pub text: TextStyle,
    /// The string the lines are joined with.
    pub join_with: String,
}

/// The dimensions of a text, as `calculateTextDimensions` returns them. `line_height` is absent for
/// an empty text, which the reference returns as `{ width: 0, height: 0 }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextDimensions {
    /// The widest line, in whole pixels.
    pub width: f64,
    /// The sum of the line heights, in whole pixels.
    pub height: f64,
    /// The height of one line, in whole pixels.
    pub line_height: Option<f64>,
}

/// The result of `breakString`: the pieces of the word that fit, each with its hyphen (the last one
/// without), and the part that is left over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakString {
    /// The pieces that fit, in order.
    pub hyphenated: Vec<String>,
    /// The end of the word that did not fill a piece.
    pub remaining: String,
}

/// Why a measurement failed: the reference's own error, or a font that ferrigura cannot read.
#[derive(Debug)]
pub enum MeasureError {
    /// The reference throws `svg element not in render tree` for a line with no glyphs (only white
    /// space), so its renderer fails too.
    NotInRenderTree,
    /// The bundled font could not be read or shaped.
    Text(TextError),
}

impl std::fmt::Display for MeasureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInRenderTree => write!(f, "svg element not in render tree"),
            Self::Text(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for MeasureError {}

impl From<TextError> for MeasureError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}

/// The face a style is measured in. `font-weight` above 500 takes the bold face, as the CSS font
/// matching of Chromium does with the two faces that exist (400 and 700): 600 measures as 700 and
/// 300 as 400 (checked in the probe).
fn face_of(style: &TextStyle) -> &'static Face {
    let weight = if style.font_weight > 500.0 {
        FontWeight::Bold
    } else {
        FontWeight::Normal
    };
    face(FontFamily::Sans, weight, FontStyle::Normal)
}

/// Splits a text at its `<br>` tags, as `text.split(lineBreakRegex)` does: the tag is `<br>`,
/// `<br/>`, `<br />` or `</br>`, in any case, with blanks (JavaScript's `\s`) allowed between its
/// parts.
#[must_use]
pub fn split_line_breaks(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        let found = if bytes[index] == b'<' {
            line_break_end(text, index)
        } else {
            None
        };
        match found {
            Some(end) => {
                parts.push(&text[start..index]);
                start = end;
                index = end;
            }
            None => index += 1,
        }
    }
    parts.push(&text[start..]);
    parts
}

/// Whether a text holds a `<br>` tag: `hasBreaks` in `common.ts`.
#[must_use]
pub fn has_breaks(text: &str) -> bool {
    split_line_breaks(text).len() > 1
}

/// The end of the `<br>` tag that starts at `start` (a `<`), or `None` if none starts there.
fn line_break_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = start + 1;
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    let is_b = bytes.get(index)?.eq_ignore_ascii_case(&b'b');
    let is_r = bytes.get(index + 1)?.eq_ignore_ascii_case(&b'r');
    if !(is_b && is_r) {
        return None;
    }
    index += 2;
    while let Some(c) = text[index..].chars().next() {
        if !is_js_whitespace(c) {
            break;
        }
        index += c.len_utf8();
    }
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    (bytes.get(index) == Some(&b'>')).then_some(index + 1)
}

/// The text as Chromium draws it in an SVG `<text>`: runs of blanks, tabs, line feeds and returns
/// become one space, and the spaces at the ends are dropped (CSS `white-space: normal`).
#[must_use]
pub fn collapse_white_space(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for c in text.chars() {
        if matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(c);
        }
    }
    out
}

/// The width of a text in an SVG `<text>` as Chromium reports it through `getComputedTextLength`: the
/// advance of the text with its white space collapsed ([`collapse_white_space`]), snapped to the
/// 1/64 px layout unit. A text with no glyph left measures 0. The advance is what Mermaid's wrapping
/// compares; a box that `getBBox` or `getBoundingClientRect` gives is [`collapsed_box`].
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn collapsed_width(face: &Face, text: &str, size: f64) -> Result<f64, TextError> {
    let collapsed = collapse_white_space(text);
    if collapsed.is_empty() {
        return Ok(0.0);
    }
    Ok(layout_width(measure(face, &collapsed, size)?))
}

/// The box of a text in an SVG `<text>` as Chromium reports it through `getBBox` and through
/// `getBoundingClientRect` (the two agree on SVG text): [`collapsed_width`]'s advance, and the glyph
/// outlines where they reach past it, relative to the start of the text. A text with no glyph left
/// has the empty box at the origin.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn collapsed_box(face: &'static Face, text: &str, size: f64) -> Result<Extent, TextError> {
    let collapsed = collapse_white_space(text);
    if collapsed.is_empty() {
        return Ok(Extent::default());
    }
    measure_box(face, &collapsed, size)
}

/// The width and the height of one line, before rounding: the width of the box `getBBox` gives it,
/// and the text box.
fn line_metrics(line: &str, style: &TextStyle) -> Result<(f64, f64), MeasureError> {
    let face = face_of(style);
    let size = style.font_size;
    if line.is_empty() {
        // `line || ZERO_WIDTH_SPACE`: the zero-width space has no advance, and the box of one line.
        return Ok((0.0, text_box(face, size)?.height));
    }
    let shown = collapse_white_space(line);
    if shown.is_empty() {
        return Err(MeasureError::NotInRenderTree);
    }
    let width = measure_box(face, &shown, laid_out_size(size))?.width();
    Ok((width, text_box(face, size)?.height))
}

/// The size Chromium lays SVG text out at: floored to 1/100 px. The probe shows the advance of a
/// 10.005 px line equal to that of 10 px (`tools/port/text/common-fixture.ts`), as the box is in
/// `ferrigura-text` (`metrics.rs`, where the same rule is applied to the box).
fn laid_out_size(size: f64) -> f64 {
    ((size * 100.0) + 1e-6).floor() / 100.0
}

/// `calculateTextDimensions` without the memo: the dimensions of a text in the style given.
///
/// # Errors
///
/// Returns [`MeasureError::NotInRenderTree`] for a line of white space, as the reference does.
pub fn text_dimensions(text: &str, style: &TextStyle) -> Result<TextDimensions, MeasureError> {
    if text.is_empty() {
        return Ok(TextDimensions {
            width: 0.0,
            height: 0.0,
            line_height: None,
        });
    }
    let mut width = 0.0_f64;
    let mut height = 0.0_f64;
    let mut line_height = 0.0_f64;
    for line in split_line_breaks(text) {
        let (advance, box_height) = line_metrics(line, style)?;
        width = width.max(advance.round());
        let rounded = box_height.round();
        height += rounded;
        line_height = line_height.max(rounded);
    }
    Ok(TextDimensions {
        width,
        height,
        line_height: Some(line_height),
    })
}

/// The reference's memo keys and caches for one render: `calculateTextDimensions`, `wrapLabel` and
/// `breakString` (see the module documentation).
#[derive(Debug, Default)]
pub struct TextMemo {
    dimensions: HashMap<String, TextDimensions>,
    wraps: HashMap<String, String>,
    breaks: HashMap<String, BreakString>,
}

impl TextMemo {
    /// An empty cache, for one render.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `calculateTextDimensions(text, style)`, memoised by `${text}${fontSize}${fontWeight}${fontFamily}`.
    ///
    /// # Errors
    ///
    /// See [`text_dimensions`]. A failed measurement is not cached, as the reference does not cache a
    /// call that throws.
    pub fn dimensions(
        &mut self,
        text: &str,
        style: &TextStyle,
    ) -> Result<TextDimensions, MeasureError> {
        let key = format!(
            "{text}{}{}{}",
            number(style.font_size),
            number(style.font_weight),
            style.font_family
        );
        if let Some(cached) = self.dimensions.get(&key) {
            return Ok(*cached);
        }
        let result = text_dimensions(text, style)?;
        self.dimensions.insert(key, result);
        Ok(result)
    }

    /// `calculateTextWidth(text, style)`: the width of [`TextMemo::dimensions`].
    ///
    /// # Errors
    ///
    /// See [`TextMemo::dimensions`].
    pub fn width(&mut self, text: &str, style: &TextStyle) -> Result<f64, MeasureError> {
        Ok(self.dimensions(text, style)?.width)
    }

    /// `calculateTextHeight(text, style)`: the height of [`TextMemo::dimensions`].
    ///
    /// # Errors
    ///
    /// See [`TextMemo::dimensions`].
    pub fn height(&mut self, text: &str, style: &TextStyle) -> Result<f64, MeasureError> {
        Ok(self.dimensions(text, style)?.height)
    }

    /// `breakString(word, maxWidth, hyphen, config)`: memoised by
    /// `${word}${maxWidth}${hyphen}${fontSize}${fontWeight}${fontFamily}`.
    ///
    /// # Errors
    ///
    /// See [`TextMemo::dimensions`].
    pub fn break_string(
        &mut self,
        word: &str,
        max_width: f64,
        hyphen: &str,
        style: &TextStyle,
    ) -> Result<BreakString, MeasureError> {
        let key = format!(
            "{word}{}{hyphen}{}{}{}",
            number(max_width),
            number(style.font_size),
            number(style.font_weight),
            style.font_family
        );
        if let Some(cached) = self.breaks.get(&key) {
            return Ok(cached.clone());
        }
        let mut hyphenated = Vec::new();
        let mut current = String::new();
        let characters: Vec<char> = word.chars().collect();
        for (index, character) in characters.iter().enumerate() {
            let next = format!("{current}{character}");
            let line_width = self.width(&next, style)?;
            if line_width >= max_width {
                let is_last = index + 1 == characters.len();
                hyphenated.push(if is_last {
                    next
                } else {
                    format!("{next}{hyphen}")
                });
                current.clear();
            } else {
                current = next;
            }
        }
        let result = BreakString {
            hyphenated,
            remaining: current,
        };
        self.breaks.insert(key, result.clone());
        Ok(result)
    }

    /// `wrapLabel(label, maxWidth, config)`: the label broken into lines of at most `max_width`,
    /// joined with `join_with`. A label with a `<br>` is returned as it is. Memoised by
    /// `${label}${maxWidth}${fontSize}${fontWeight}${fontFamily}${joinWith}`.
    ///
    /// # Errors
    ///
    /// See [`TextMemo::dimensions`].
    pub fn wrap_label(
        &mut self,
        label: &str,
        max_width: f64,
        style: &WrapStyle,
    ) -> Result<String, MeasureError> {
        let key = format!(
            "{label}{}{}{}{}{}",
            number(max_width),
            number(style.text.font_size),
            number(style.text.font_weight),
            style.text.font_family,
            style.join_with
        );
        if let Some(cached) = self.wraps.get(&key) {
            return Ok(cached.clone());
        }
        let result = self.wrap_uncached(label, max_width, style)?;
        self.wraps.insert(key, result.clone());
        Ok(result)
    }

    fn wrap_uncached(
        &mut self,
        label: &str,
        max_width: f64,
        style: &WrapStyle,
    ) -> Result<String, MeasureError> {
        if label.is_empty() || has_breaks(label) {
            return Ok(label.to_owned());
        }
        let words: Vec<&str> = label.split(' ').filter(|word| !word.is_empty()).collect();
        let mut completed: Vec<String> = Vec::new();
        let mut next_line = String::new();
        for (index, word) in words.iter().enumerate() {
            // Both widths are measured before the branches, as the reference does, so an error in
            // either one is raised here.
            let word_length = self.width(&format!("{word} "), &style.text)?;
            let next_line_length = self.width(&next_line, &style.text)?;
            if word_length > max_width {
                let broken = self.break_string(word, max_width, "-", &style.text)?;
                completed.push(std::mem::take(&mut next_line));
                completed.extend(broken.hyphenated);
                next_line = broken.remaining;
            } else if next_line_length + word_length >= max_width {
                completed.push(std::mem::take(&mut next_line));
                next_line = (*word).to_owned();
            } else if next_line.is_empty() {
                next_line = (*word).to_owned();
            } else {
                next_line = format!("{next_line} {word}");
            }
            if index + 1 == words.len() {
                completed.push(next_line.clone());
            }
        }
        Ok(completed
            .into_iter()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(&style.join_with))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(size: f64) -> TextStyle {
        TextStyle {
            font_size: size,
            font_weight: 400.0,
            font_family: "\"Lexica Ultralegible\"".to_owned(),
        }
    }

    #[test]
    fn breaks_split_at_every_spelling_of_the_tag() {
        assert_eq!(split_line_breaks("a<br>b"), ["a", "b"]);
        assert_eq!(split_line_breaks("a<br/>b"), ["a", "b"]);
        assert_eq!(split_line_breaks("a<br />b"), ["a", "b"]);
        assert_eq!(split_line_breaks("a<BR>b"), ["a", "b"]);
        assert_eq!(split_line_breaks("a</br>b"), ["a", "b"]);
        assert_eq!(split_line_breaks("a<br/><br/>b"), ["a", "", "b"]);
        assert_eq!(split_line_breaks("a<br/>"), ["a", ""]);
        assert_eq!(split_line_breaks("a<br/ >b"), ["a<br/ >b"]);
        assert_eq!(split_line_breaks("a<brb>c"), ["a<brb>c"]);
        assert_eq!(split_line_breaks("abr>c"), ["abr>c"]);
    }

    #[test]
    fn white_space_collapses_as_css_does() {
        assert_eq!(collapse_white_space("a  b"), "a b");
        assert_eq!(collapse_white_space("a\tb\nc"), "a b c");
        assert_eq!(collapse_white_space(" trailing "), "trailing");
        assert_eq!(collapse_white_space("   "), "");
        assert_eq!(collapse_white_space("a\u{a0}b"), "a\u{a0}b");
    }

    #[test]
    fn a_line_of_white_space_is_not_in_the_render_tree() {
        assert!(matches!(
            text_dimensions(" ", &style(16.0)),
            Err(MeasureError::NotInRenderTree)
        ));
        assert!(matches!(
            text_dimensions("a<br/>   ", &style(16.0)),
            Err(MeasureError::NotInRenderTree)
        ));
    }

    #[test]
    fn the_size_is_floored_to_a_hundredth_as_chromium_lays_it_out() {
        assert_eq!(laid_out_size(10.005), 10.0);
        assert_eq!(laid_out_size(13.337), 13.33);
        assert_eq!(laid_out_size(16.25), 16.25);
        assert_eq!(laid_out_size(0.29), 0.29);
    }

    #[test]
    fn an_empty_text_has_no_line_height() {
        let dims = text_dimensions("", &style(16.0)).expect("empty text");
        assert_eq!(dims.line_height, None);
        assert_eq!((dims.width, dims.height), (0.0, 0.0));
    }

    #[test]
    fn a_wrap_breaks_a_long_label_at_the_width() {
        let mut memo = TextMemo::new();
        let wrap = WrapStyle {
            text: style(16.0),
            join_with: "<br/>".to_owned(),
        };
        let lines = memo
            .wrap_label("one two three four five six", 40.0, &wrap)
            .expect("wrap");
        assert_eq!(lines.matches("<br/>").count(), 5, "{lines}");
    }
}
