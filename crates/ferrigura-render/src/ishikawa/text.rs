//! The text of an ishikawa diagram: the lines of a label, the wrapping of the head and of the
//! sub-causes, and the box `getBBox` gives a drawn label.
//!
//! The renderer writes every label as a `text` with one `tspan` per line (`ishikawaRenderer.ts`
//! 439-473). The reference reads four of those boxes back (the viewBox, the spine, the head and the
//! label rectangles), so [`Label`] computes the same box from the font: the horizontal extent of
//! each line from its anchor, with the glyph ink where it reaches past the advance
//! (`ferrigura_text::measure_box`), and the vertical extent of the text box that the baseline of the
//! CSS `dominant-baseline` places (see [`baseline_top`]).

use ferrigura_text::{Face, Run, layout_width, measure, measure_box, text_box, x_height};

use crate::RenderError;
use crate::common::Element;
use crate::common::is_js_whitespace;
use crate::svg::number;
use crate::unified::geom::BBox;

/// The `text-anchor` a label is laid out with. The stylesheet decides it for every class, and the
/// attribute of the head (`start`) is overridden by its rule, so the layout uses the computed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// The line is centred on `x`.
    Middle,
    /// The line ends at `x`.
    End,
}

/// The `dominant-baseline` a label is set with, as the stylesheet computes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    /// The alphabetic baseline, which is also `auto`. The reference's `baseline` value is not a
    /// `dominant-baseline` keyword, so its declaration is ignored and the computed value is `auto`
    /// (`ishikawaStyles.ts` 59-61).
    Alphabetic,
    /// The middle of the x-height sits on `y` (`ishikawaStyles.ts` 49-52, 54-57).
    Middle,
    /// The hanging baseline sits on `y` (`ishikawaStyles.ts` 63-65).
    Hanging,
}

/// The hanging baseline's height above the alphabetic one, as a share of the ascent. Chromium
/// takes 0.8 of the rounded ascent: 10.4 px at 14 px, which the reference's boxes show (`top` at
/// `y - 2.59375`, `NOTES.md` 6.3). The unrounded ascent would give 10.64 px.
const HANGING_SHARE: f64 = 0.8;

/// The top of a label's text box relative to its `y`, for a baseline: the box runs from the
/// alphabetic baseline's ascent above it, and the baseline is moved by the stylesheet's
/// `dominant-baseline`. The shift is on Chromium's layout grid of 1/64 px (`NOTES.md` 6.3: `y -
/// 9.53125` at 14 px, where the x-height is 6.944 px, and `y - 2.59375` for the hanging baseline).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
fn baseline_top(face: &Face, size: f64, baseline: Baseline) -> Result<f64, RenderError> {
    let ascent = text_box(face, size)?.ascent;
    let shift = match baseline {
        Baseline::Alphabetic => 0.0,
        // The middle of the x-height sits on `y`, so the alphabetic baseline is half of it below.
        Baseline::Middle => layout_unit(x_height(face, size)? / 2.0),
        // The hanging baseline sits on `y`, so the alphabetic one is the hanging share below it.
        Baseline::Hanging => layout_unit(HANGING_SHARE * ascent),
    };
    Ok(shift - ascent)
}

/// A length rounded to the layout unit of 1/64 px, which Chromium's baseline shifts are on.
fn layout_unit(length: f64) -> f64 {
    (length * 64.0).round() / 64.0
}

/// How a label is set: its class, its anchor and baseline as the stylesheet computes them, its face,
/// the size it is drawn at, and the distance between its lines.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// The `class` attribute.
    pub class: &'static str,
    /// The `text-anchor` attribute as the reference writes it. The head writes `start` and the
    /// stylesheet centres it, so this can differ from [`Style::anchor`].
    pub anchor_attribute: &'static str,
    /// The anchor the layout uses, as the stylesheet computes it.
    pub anchor: Anchor,
    /// The baseline the layout uses, as the stylesheet computes it.
    pub baseline: Baseline,
    /// The face the lines are set in.
    pub face: &'static Face,
    /// The size the lines are drawn at, from the stylesheet.
    pub size: f64,
    /// The distance between two lines (`fontSize * 1.05` of the configuration, `ishikawaRenderer.ts`
    /// 467).
    pub line_height: f64,
}

/// A drawn label: a `text` element of the diagram, its lines and the box `getBBox` gives it.
#[derive(Debug, Clone)]
pub struct Label {
    /// How the label is set.
    pub style: Style,
    /// The lines, in order: one `tspan` each.
    pub lines: Vec<String>,
    /// The `x` of the text, and of every line.
    pub x: f64,
    /// The `y` of the text: the first line's baseline, moved up by half of the extra lines.
    pub y: f64,
    /// The box `getBBox` gives the text, in the user space the text is written in.
    pub bbox: BBox,
}

impl Label {
    /// The label of `text` at `(x, y)`, where `y` is the baseline of the text's middle, as
    /// `drawMultilineText` places it (`ishikawaRenderer.ts` 466-473).
    ///
    /// # Errors
    ///
    /// Returns an error if the bundled font cannot be read.
    pub fn new(style: Style, text: &str, x: f64, y: f64) -> Result<Self, RenderError> {
        let lines: Vec<String> = split_lines(text).into_iter().map(str::to_owned).collect();
        let line_height = style.line_height;
        let y = y - ((lines.len() as f64 - 1.0) * line_height) / 2.0;
        let top = baseline_top(style.face, style.size, style.baseline)?;
        let height = text_box(style.face, style.size)?.height;
        let mut bbox: Option<BBox> = None;
        // A `dy` belongs to the characters of its `tspan`, so an empty line takes no step: the pen
        // moves one line step for each line after the first that has characters.
        let mut steps = 0_usize;
        for (index, line) in lines.iter().enumerate() {
            // A line without characters makes no text fragment, so it adds nothing to the box. A
            // label of such lines alone has the empty box at the origin (`getBBox` of a text with no
            // glyphs, as the reference's empty head gives: `h = 40` and `translate(33,0)`).
            if line.is_empty() {
                continue;
            }
            if index > 0 {
                steps += 1;
            }
            // The anchor shifts the line by its advance: the snapped advance, centred or at its end.
            let advance = layout_width(measure(style.face, line, style.size)?);
            let offset = match style.anchor {
                Anchor::Middle => -advance / 2.0,
                Anchor::End => -advance,
            };
            let ink = measure_box(style.face, line, style.size)?;
            let line_top = y + steps as f64 * line_height + top;
            let line_box = BBox::new(x + offset + ink.left, line_top, ink.width(), height);
            bbox = Some(match bbox {
                None => line_box,
                Some(so_far) => so_far.union(line_box),
            });
        }
        Ok(Self {
            style,
            lines,
            x,
            y,
            bbox: bbox.unwrap_or_default(),
        })
    }

    /// The `text` element of the label, with its `tspan`s, and the fill of its colour. The caller
    /// adds a transform where the reference sets one.
    #[must_use]
    pub fn element(&self, fill: &str) -> Element {
        let mut text = Element::new("text")
            .with("class", self.style.class)
            .with("text-anchor", self.style.anchor_attribute)
            .with("x", number(self.x))
            .with("y", number(self.y))
            .with("fill", fill);
        for (index, line) in self.lines.iter().enumerate() {
            let dy = if index == 0 {
                0.0
            } else {
                self.style.line_height
            };
            text.push_child(
                Element::new("tspan")
                    .with("x", number(self.x))
                    .with("dy", number(dy))
                    .text(line.clone()),
            );
        }
        text
    }

    /// The runs of the label's lines, for the font subset of the diagram.
    #[must_use]
    pub fn runs(&self) -> Vec<(String, &'static Face)> {
        self.lines
            .iter()
            .map(|line| (line.clone(), self.style.face))
            .collect()
    }
}

/// The lines of a label: the text split at `<br>`, `<br/>` or `<br />` (`<br` in lower case, then
/// optional white space, an optional slash and `>`) and at line feeds, as `splitLines` does
/// (`ishikawaRenderer.ts` 439).
#[must_use]
pub fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        match separator_length(rest) {
            Some(length) => {
                lines.push(&text[start..index]);
                index += length;
                start = index;
            }
            None => {
                index += rest.chars().next().map_or(1, char::len_utf8);
            }
        }
    }
    lines.push(&text[start..]);
    lines
}

/// The length of the line separator at the start of `text`, if there is one.
fn separator_length(text: &str) -> Option<usize> {
    if text.starts_with('\n') {
        return Some(1);
    }
    let after_tag = text.strip_prefix("<br")?;
    let mut length = "<br".len();
    let mut rest = after_tag;
    while let Some(first) = rest.chars().next() {
        if !is_js_whitespace(first) {
            break;
        }
        length += first.len_utf8();
        rest = &rest[first.len_utf8()..];
    }
    if let Some(stripped) = rest.strip_prefix('/') {
        length += 1;
        rest = stripped;
    }
    rest.starts_with('>').then_some(length + 1)
}

/// The text of a long label, wrapped at `max_chars` (`ishikawaRenderer.ts` 441-455). A text no
/// longer than `max_chars` is returned as it is. Otherwise the words, split at white space as
/// `split(/\s+/)` splits them, are packed greedily, and the lines are joined with line feeds. The
/// lengths are JavaScript's, in UTF-16 units.
#[must_use]
pub fn wrap_text(text: &str, max_chars: f64) -> String {
    if utf16_length(text) as f64 <= max_chars {
        return text.to_owned();
    }
    let mut lines: Vec<String> = Vec::new();
    for word in split_on_white_space(text) {
        let fits = lines.last().is_some_and(|line| {
            utf16_length(line) as f64 + 1.0 + utf16_length(word) as f64 <= max_chars
        });
        if fits {
            let last = lines.last_mut().expect("a line was checked");
            last.push(' ');
            last.push_str(word);
        } else {
            lines.push(word.to_owned());
        }
    }
    lines.join("\n")
}

/// `text.split(/\s+/)` with JavaScript's white space: a leading or a trailing run gives an empty
/// piece, as it does in JavaScript.
fn split_on_white_space(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut characters = text.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if !is_js_whitespace(character) {
            continue;
        }
        pieces.push(&text[start..index]);
        let mut end = index + character.len_utf8();
        while let Some(&(next_index, next)) = characters.peek() {
            if !is_js_whitespace(next) {
                break;
            }
            characters.next();
            end = next_index + next.len_utf8();
        }
        start = end;
    }
    pieces.push(&text[start..]);
    pieces
}

/// The length of a string in UTF-16 units, as JavaScript's `length` counts it.
#[must_use]
pub fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The runs of the given labels, as the font subset of a diagram takes them.
#[must_use]
pub fn as_runs<'a>(texts: &'a [(String, &'static Face)]) -> Vec<Run<'a>> {
    texts
        .iter()
        .map(|(text, face)| Run { text, face })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrigura_text::{FontFamily, FontStyle, FontWeight, face};

    fn regular() -> &'static Face {
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
    }

    #[test]
    fn a_line_break_tag_or_a_line_feed_splits_a_label() {
        assert_eq!(split_lines("a<br>b"), ["a", "b"]);
        assert_eq!(split_lines("a<br/>b<br />c\nd"), ["a", "b", "c", "d"]);
        assert_eq!(split_lines("a<BR>b"), ["a<BR>b"]);
        assert_eq!(split_lines("a<br b"), ["a<br b"]);
        assert_eq!(split_lines(""), [""]);
        assert_eq!(split_lines("<br>"), ["", ""]);
    }

    #[test]
    fn a_long_head_is_wrapped_at_eleven_characters() {
        assert_eq!(
            wrap_text("A long effect title for wrapping", 11.0),
            "A long\neffect\ntitle for\nwrapping"
        );
        assert_eq!(wrap_text("Very deep", 11.0), "Very deep");
    }

    #[test]
    fn a_word_longer_than_the_width_keeps_its_own_line() {
        assert_eq!(
            wrap_text("a sub cause that is quite long indeed", 15.0),
            "a sub cause\nthat is quite\nlong indeed"
        );
        assert_eq!(
            wrap_text("supercalifragilisticexpialidocious word", 15.0),
            "supercalifragilisticexpialidocious\nword"
        );
    }

    #[test]
    fn the_lengths_count_utf16_units() {
        // An emoji is two units of JavaScript's length, so it fills a line of 15 by itself with one
        // more character.
        assert_eq!(utf16_length("\u{1F600}"), 2);
    }

    #[test]
    fn a_middle_baseline_and_a_hanging_baseline_are_placed_as_the_reference_places_them() {
        let middle = baseline_top(regular(), 14.0, Baseline::Middle).unwrap();
        assert!((middle + 9.53125).abs() < 0.01, "{middle}");
        let hanging = baseline_top(regular(), 14.0, Baseline::Hanging).unwrap();
        assert!((hanging + 2.59375).abs() < 0.01, "{hanging}");
        let alphabetic = baseline_top(regular(), 14.0, Baseline::Alphabetic).unwrap();
        assert_eq!(alphabetic, -13.0);
    }

    #[test]
    fn an_empty_line_takes_no_step_and_adds_no_box() {
        // `Empty line<br><br>cause`: the `cause` line is one step below the first, since the empty
        // tspan's `dy` is never applied (the box is 16.8 + 17 high, `probes/ishikawa/p15`).
        let style = Style {
            class: "ishikawa-label cause",
            anchor_attribute: "middle",
            anchor: Anchor::Middle,
            baseline: Baseline::Middle,
            face: regular(),
            size: 14.0,
            line_height: 16.8,
        };
        let label = Label::new(style, "Empty line<br><br>cause", 0.0, 0.0).unwrap();
        assert!((label.bbox.height - 33.8).abs() < 1e-9, "{:?}", label.bbox);
        // A label of empty lines alone has the empty box at the origin, as `getBBox` gives it.
        let blank = Label::new(style, "<br>", 40.0, 40.0).unwrap();
        assert_eq!(blank.bbox, BBox::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn a_centred_line_is_centred_on_its_advance() {
        let style = Style {
            class: "ishikawa-label cause",
            anchor_attribute: "middle",
            anchor: Anchor::Middle,
            baseline: Baseline::Middle,
            face: regular(),
            size: 14.0,
            line_height: 16.8,
        };
        let label = Label::new(style, "Process", 0.0, 0.0).unwrap();
        // The advance of `Process` is 48.640625 at 14 px (`NOTES.md` 6.3), so the box runs from
        // -24.3203125 to 24.3203125.
        assert!((label.bbox.x + 24.3203125).abs() < 1e-6, "{:?}", label.bbox);
        assert!(
            (label.bbox.width - 48.640625).abs() < 1e-6,
            "{:?}",
            label.bbox
        );
        assert!((label.bbox.y + 9.53125).abs() < 0.01, "{:?}", label.bbox);
        assert!((label.bbox.height - 17.0).abs() < 1e-9, "{:?}", label.bbox);
    }
}
