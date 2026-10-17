//! The wrapping of a node's text, as `svgDraw.js` `wrap` (456-496) writes it, and the box that
//! `getBBox` gives the wrapped text (`docs-port/timeline/NOTES.md` 5.4, measured in Chromium).
//!
//! The reference splits the text at blanks and at `<br>` tokens, adds one token at a time to a line,
//! and starts a new `tspan` when the line gets wider than the width. The width of a line is its
//! `getComputedTextLength`: the advance of the collapsed text, snapped to the layout unit. Every
//! `tspan` is `x="0"` and centred by `text-anchor: middle`, so each line is centred on the text's
//! origin. Lines are one `em` apart after the first, and an empty `tspan` takes no glyph, so it adds
//! no height (`getBBox`), and it moves no line.

use ferrigura_text::Face;

use super::Context;
use crate::RenderError;
use crate::common::text::{collapse_white_space, collapsed_width};
use crate::common::{is_js_whitespace, js_trim};

/// The token `wrap` splits the text at, besides the blanks.
const BREAK: &str = "<br>";

/// The line spacing of the wrapped text, in `em`: `lineHeight` of `svgDraw.js` `wrap` (465).
const LINE_HEIGHT_EM: f64 = 1.1;

/// One `tspan` of a wrapped text.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Tspan {
    /// The text of the line, trimmed; the text of a line that was just started may be raw until the
    /// next token updates it, but `wrap` always leaves the final text trimmed.
    pub text: String,
    /// Whether this is the first `tspan`. It has `dy="1em"`; the others have `dy="1.1em"`.
    pub first: bool,
}

/// The box `getBBox` gives a wrapped text, relative to the text's origin: the lines are centred on
/// `x = 0`, so the box is symmetric around it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Extent {
    /// Half the width of the widest line.
    pub half_width: f64,
    /// The top of the first line with text.
    pub top: f64,
    /// The bottom of the last line with text.
    pub bottom: f64,
}

/// A wrapped text and its box. The box is absent when no line has text.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Laid {
    /// The `tspan`s, in order.
    pub tspans: Vec<Tspan>,
    /// The box of the lines with text.
    pub extent: Option<Extent>,
}

/// Wraps `text` to `width` in `face` at the CSS size, and gives the box of the result.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn layout(
    cx: &Context<'_>,
    text: &str,
    width: f64,
    face: &'static Face,
) -> Result<Laid, RenderError> {
    let tspans = wrap(text, width, |line| {
        Ok(collapsed_width(face, line, cx.text_size)?)
    })?;
    let extent = extent(cx, &tspans, face)?;
    Ok(Laid { tspans, extent })
}

/// `wrap(text, width)` of `svgDraw.js` (456-496). `width_of` measures a line the way the browser
/// measures the `tspan` (the text is set before it is measured, as the reference does).
pub(super) fn wrap(
    text: &str,
    width: f64,
    mut width_of: impl FnMut(&str) -> Result<f64, RenderError>,
) -> Result<Vec<Tspan>, RenderError> {
    let words = split_words(text);
    let mut spans = vec![String::new()];
    let mut line: Vec<String> = Vec::new();
    for word in words {
        let is_break = word == BREAK;
        line.push(word.clone());
        let last = spans.len() - 1;
        spans[last] = js_trim(&line.join(" ")).to_owned();
        if width_of(&spans[last])? > width || is_break {
            line.pop();
            spans[last] = js_trim(&line.join(" ")).to_owned();
            line = if is_break {
                vec![String::new()]
            } else {
                vec![word.clone()]
            };
            spans.push(word);
        }
    }
    Ok(spans
        .into_iter()
        .enumerate()
        .map(|(index, text)| Tspan {
            text,
            first: index == 0,
        })
        .collect())
}

/// `text.split(/(\s+|<br>)/)`: the pieces between the separators, with the separators kept in
/// order. A piece can be empty, at the ends of the text and between two separators.
fn split_words(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        let separator = if rest.starts_with(BREAK) {
            BREAK.len()
        } else {
            rest.chars()
                .take_while(|c| is_js_whitespace(*c))
                .map(char::len_utf8)
                .sum()
        };
        if separator > 0 {
            parts.push(text[start..index].to_owned());
            parts.push(text[index..index + separator].to_owned());
            index += separator;
            start = index;
        } else {
            index += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    parts.push(text[start..].to_owned());
    parts
}

/// The box of the lines with text. Each line's baseline is one `dy` below the previous one with
/// text; its glyphs run from the ascent above the baseline to the descent below it, and the middle of
/// the x-height sits on the position (`dominant-baseline: middle`).
fn extent(
    cx: &Context<'_>,
    tspans: &[Tspan],
    face: &'static Face,
) -> Result<Option<Extent>, RenderError> {
    let mut y = 0.0;
    let mut extent: Option<Extent> = None;
    for tspan in tspans {
        let shown = super::shown(&tspan.text);
        if collapse_white_space(&shown).is_empty() {
            // No glyph: the `tspan` takes no position, and the next one moves from the last glyph.
            continue;
        }
        let em = if tspan.first { 1.0 } else { LINE_HEIGHT_EM };
        y += em * cx.text_size;
        let baseline = y + cx.middle;
        let half_width = collapsed_width(face, &shown, cx.text_size)? / 2.0;
        let top = baseline - cx.ascent;
        let bottom = baseline + cx.descent;
        extent = Some(match extent {
            None => Extent {
                half_width,
                top,
                bottom,
            },
            Some(so_far) => Extent {
                half_width: so_far.half_width.max(half_width),
                top: so_far.top,
                bottom,
            },
        });
    }
    Ok(extent)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in measure: one unit per character of the collapsed line, as the browser lays it out,
    /// so the widths are easy to read.
    fn per_character(line: &str) -> Result<f64, RenderError> {
        Ok(collapse_white_space(line).chars().count() as f64)
    }

    fn texts(spans: &[Tspan]) -> Vec<&str> {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn a_text_that_fits_is_one_line_and_the_blanks_are_joined_with_blanks() {
        // The separator token is joined with a blank on each side, so one blank becomes three in the
        // text; the browser collapses them when it lays the line out (the reference's markup has
        // `Machinery,   Water` for `Machinery, Water`).
        let spans = wrap("Machinery, Water", 150.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["Machinery,   Water"]);
        assert!(spans[0].first);
    }

    #[test]
    fn a_line_breaks_before_the_word_that_makes_it_wider_than_the_width() {
        // "one two" is 7 characters once collapsed and fits in 7; "one two three" does not.
        let spans = wrap("one two three", 7.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["one   two", "three"]);
        assert!(!spans[1].first);
    }

    #[test]
    fn a_break_tag_starts_a_line_and_a_trailing_one_leaves_an_empty_line() {
        let spans = wrap("a<br>b<br>", 150.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["a", "b", ""]);
    }

    #[test]
    fn a_leading_break_tag_leaves_an_empty_first_line() {
        let spans = wrap("<br>abc", 150.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["", "abc"]);
    }

    #[test]
    fn an_overlong_word_is_not_split_and_takes_a_line_of_its_own() {
        let spans = wrap("abcdefghij", 4.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["", "abcdefghij"]);
    }

    #[test]
    fn other_break_spellings_are_text() {
        let spans = wrap("a<br/>b", 150.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["a<br/>b"]);
        let spans = wrap("a<BR>b", 150.0, per_character).expect("wrap");
        assert_eq!(texts(&spans), ["a<BR>b"]);
    }

    #[test]
    fn the_split_keeps_the_separators_and_the_empty_ends() {
        assert_eq!(split_words(""), [""]);
        assert_eq!(split_words("a b"), ["a", " ", "b"]);
        assert_eq!(split_words(" a "), ["", " ", "a", " ", ""]);
        assert_eq!(split_words("a<br>"), ["a", "<br>", ""]);
    }
}
