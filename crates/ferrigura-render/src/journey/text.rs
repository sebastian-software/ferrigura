//! The text of a journey: the faces, the widths the reference reads from the browser, and the three
//! placements of a label (`svgDraw.js` 274-390).
//!
//! The default placement, `fo`, is HTML in the reference: a `foreignObject` holding a `div` whose
//! text is laid out by the browser (`NOTES.md` 7.6.1). ferrigura draws the label as SVG text
//! (`docs-port/decisions/0004`), with the lines the browser breaks the HTML into: the break
//! opportunities of the text, each line centred in the cell, and the line box of the font.

use ferrigura_model::config::JourneyDiagramConfig;
use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face, text_box};

use crate::RenderError;
use crate::common::Element;
use crate::common::text::{collapse_white_space, collapsed_box, collapsed_width};
use crate::svg::number;

/// The regular face: the legend, the sections, the tasks and their labels.
#[must_use]
pub fn regular() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

/// The bold face: the title, which the reference sets with `font-weight="bold"`.
#[must_use]
pub fn bold() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal)
}

/// The width of a text as `getBoundingClientRect().width` reports it for an SVG `text` in the
/// regular face ([`collapsed_box`]): the box of the glyphs, which is wider than the advance where a
/// glyph's outline reaches past it. The reference measures the legend this way (`journeyRenderer.ts`
/// 38-40, 57-73 and 105). Text with no glyphs measures as 0, not as an error.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn svg_width(text: &str, size: f64) -> Result<f64, RenderError> {
    Ok(collapsed_box(regular(), text, size)?.width())
}

/// One line of a laid-out label: its text without the blank that ends it, and its width.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The text of the line.
    pub text: String,
    /// The width of the text, as [`width`] gives it.
    pub width: f64,
}

/// The pieces a text breaks into: each piece ends where the browser may break the line after it.
/// A line may break after a blank, and after a hyphen that is followed by a letter (UAX 14: a
/// break after HY before AL, and none before a digit). A text with no blank and no hyphen is one
/// piece, which the browser never breaks.
///
/// The text is collapsed already, so a blank is never doubled and never at either end.
fn pieces(collapsed: &str) -> Vec<String> {
    let characters: Vec<char> = collapsed.chars().collect();
    let mut pieces = Vec::new();
    let mut current = String::new();
    for (index, &character) in characters.iter().enumerate() {
        current.push(character);
        let next = characters.get(index + 1).copied();
        let breaks_after =
            character == ' ' || (character == '-' && next.is_some_and(|next| next.is_alphabetic()));
        if breaks_after {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

/// The width of the widest piece of a text: the least width the browser can give the cell of the
/// text, since a piece has no break opportunity inside it. The table cell of the `fo` label takes
/// at least this width (`min-content`), so a long word widens the cell rather than overflowing it.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(crate) fn min_content_width_in(
    face: &'static Face,
    text: &str,
    size: f64,
) -> Result<f64, RenderError> {
    let mut widest: f64 = 0.0;
    for piece in pieces(&collapse_white_space(text)) {
        widest = widest.max(collapsed_width(face, piece.trim_end(), size)?);
    }
    Ok(widest)
}

/// The lines the browser sets a text in, in a cell `cell_width` wide, at the font size `size`.
///
/// A line takes the next piece while its text, without the blank at its end (the blank hangs), is
/// no wider than the cell. The first piece of a line always goes on it; the cell is never narrower
/// than its widest piece ([`min_content_width_in`]), so every piece fits on a line of its own.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn break_lines(text: &str, cell_width: f64, size: f64) -> Result<Vec<Line>, RenderError> {
    break_lines_in(regular(), text, cell_width, size)
}

/// [`break_lines`] in the face given, for the labels a renderer sets in another face (the bold
/// names of the eventmodeling boxes, the mono code of their data).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(crate) fn break_lines_in(
    face: &'static Face,
    text: &str,
    cell_width: f64,
    size: f64,
) -> Result<Vec<Line>, RenderError> {
    let collapsed = collapse_white_space(text);
    let mut texts = Vec::new();
    let mut current = String::new();
    for piece in pieces(&collapsed) {
        let candidate = format!("{current}{piece}");
        if current.is_empty() || collapsed_width(face, candidate.trim_end(), size)? <= cell_width {
            current = candidate;
        } else {
            texts.push(current.trim_end().to_owned());
            current = piece;
        }
    }
    if !current.is_empty() {
        texts.push(current.trim_end().to_owned());
    }
    let mut lines = Vec::with_capacity(texts.len());
    for text in texts {
        let line_width = collapsed_width(face, &text, size)?;
        lines.push(Line {
            text,
            width: line_width,
        });
    }
    Ok(lines)
}

/// The box of a label: the reference's `foreignObject` for the `fo` placement, and the rectangle
/// the other two placements centre their text on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

/// The `textPlacement` of the configuration: `fo`, `old`, or anything else, which is `tspan`
/// (`svgDraw.js` 387-389).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// HTML in a `foreignObject`, drawn here as SVG text in the lines the browser would break.
    Fo,
    /// One `text` for each `<br>` piece, each with a `tspan`.
    Tspan,
    /// One `text` for the whole label.
    Old,
}

impl Placement {
    /// The placement a configuration value names.
    #[must_use]
    pub fn of(text_placement: &str) -> Self {
        match text_placement {
            "fo" => Self::Fo,
            "old" => Self::Old,
            _ => Self::Tspan,
        }
    }
}

/// What a label needs to be drawn: its text and box, the classes and colours its element takes,
/// and the sizes of the text.
#[derive(Debug, Clone, Copy)]
pub struct Label<'a> {
    /// The text, as the reader sees it.
    pub text: &'a str,
    /// The box the text sits in.
    pub cell: Cell,
    /// The class of the `text` of the `tspan` and `old` placements (`task`, or the section's pair).
    pub class: &'a str,
    /// The colour of the `tspan` placement (`colour` of `svgDraw.js` 317).
    pub colour: &'a str,
    /// The colour of the `fo` text: the theme's text colour, which the reference's CSS gives the
    /// `div` (`styles.js` 4-7).
    pub text_colour: &'a str,
    /// The font size of the root, in pixels, which the `fo` lines are set in.
    pub font_size: f64,
    /// The `taskFontSize` of the configuration, in pixels: the spacing of the `tspan` placement.
    pub task_font_size: f64,
}

/// The SVG elements of a label, in the order the placement writes them. The `fo` placement is a
/// `switch` holding the text: the reference's `switch` holds a `foreignObject` first, which the
/// browser draws, then the `tspan` text as the fallback (`svgDraw.js` 346-372). The fallback is not
/// drawn, so it is not written.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn label(placement: Placement, label: &Label<'_>) -> Result<Vec<Element>, RenderError> {
    match placement {
        Placement::Fo => Ok(vec![fo_label(label)?]),
        Placement::Tspan => Ok(tspan_label(label)),
        Placement::Old => Ok(vec![old_label(label)]),
    }
}

/// The `fo` label: each line of the browser's layout, as a `tspan` at its own position.
///
/// The line box is the font's text box (`ferrigura_text::text_box`); a block shorter than the cell
/// is centred vertically (`vertical-align: middle` in a `table-cell` of the cell's height), and a
/// block taller than it starts at the top of the cell (`NOTES.md` 7.6.1). Each line is centred
/// horizontally in the cell, which is widened to its widest piece when that is wider.
fn fo_label(label: &Label<'_>) -> Result<Element, RenderError> {
    let cell = label.cell;
    // The reference's cell is a table cell, which is never narrower than its min-content width: a
    // piece wider than the foreignObject widens the cell, and the lines are broken and centred in
    // the widened cell. Measured in Chromium, with the word `reallyvery…` of 270.797 px in a 150 px
    // cell, the line `A` starts at 550 + (270.797 - 8.766) / 2 = 681.016, as the reference puts it.
    let width = cell.width.max(min_content_width_in(
        regular(),
        label.text,
        label.font_size,
    )?);
    let lines = break_lines(label.text, width, label.font_size)?;
    let metrics = text_box(regular(), label.font_size)?;
    let block = lines.len() as f64 * metrics.height;
    let top = if block < cell.height {
        cell.y + (cell.height - block) / 2.0
    } else {
        cell.y
    };
    let mut text = Element::new("text")
        .with("class", "label")
        .with("fill", label.text_colour);
    for (index, line) in lines.iter().enumerate() {
        // The centring offset is in layout units of 1/64 px, rounded down, as Chromium places it
        // (the reference's line of 70.765625 px in a 150 px cell starts 39.609375 px in, not 39.6171875).
        let x = cell.x + floor_layout_unit((width - line.width) / 2.0);
        let y = top + index as f64 * metrics.height + metrics.top;
        text = text.child(
            Element::new("tspan")
                .with("x", number(x))
                .with("y", number(y))
                .text(line.text.as_str()),
        );
    }
    Ok(Element::new("switch").child(text))
}

/// A length rounded down to the layout unit of 1/64 px, as Chromium keeps a position.
fn floor_layout_unit(length: f64) -> f64 {
    (length * 64.0 + 1e-6).floor() / 64.0
}

/// The `tspan` placement (`svgDraw.js` 307-334): one `text` for each piece of the label split at
/// `<br>`, centred on the cell, with its `tspan` shifted down by the font size, so the block of
/// lines is centred on the cell's middle.
fn tspan_label(label: &Label<'_>) -> Vec<Element> {
    let cell = label.cell;
    let pieces = split_br(label.text);
    let count = pieces.len() as f64;
    let size = label.task_font_size;
    let centre = cell.x + cell.width / 2.0;
    pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            let dy = index as f64 * size - (size * (count - 1.0)) / 2.0;
            let mut text = Element::new("text")
                .with("x", number(centre))
                .with("y", number(cell.y))
                .with("fill", label.colour)
                .with(
                    "style",
                    format!("text-anchor: middle; font-size: {}px;", number(size)),
                )
                .child(
                    Element::new("tspan")
                        .with("x", number(centre))
                        .with("dy", number(dy))
                        .text(piece.as_str()),
                );
            text.set("y", number(cell.y + cell.height / 2.0));
            text.set("dominant-baseline", "central");
            text.set("alignment-baseline", "central");
            text.set("class", label.class);
            text
        })
        .collect()
}

/// The `old` placement (`svgDraw.js` 285-294): one `text` for the whole label, at the middle of
/// the cell and 5 px below it. The reference's `font-color` style is not written: it is not a CSS
/// property, and Chromium drops it.
fn old_label(label: &Label<'_>) -> Element {
    let cell = label.cell;
    Element::new("text")
        .with("x", number(cell.x + cell.width / 2.0))
        .with("y", number(cell.y + cell.height / 2.0 + 5.0))
        .with("style", "text-anchor: middle;")
        .with("class", label.class)
        .text(label.text)
}

/// `content.split(/<br\s*\/?>/gi)`: the pieces between the `<br>` tags, which are `<br>`, `<br/>`
/// and `<br />` in any case, with the blanks JavaScript's `\s` allows. The closing `</br>` is not
/// a tag here, as in the reference's expression.
#[must_use]
pub fn split_br(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut index = 0;
    while index < characters.len() {
        if let Some(end) = br_tag_end(&characters, index) {
            pieces.push(std::mem::take(&mut current));
            index = end;
        } else {
            current.push(characters[index]);
            index += 1;
        }
    }
    pieces.push(current);
    pieces
}

/// The index after a `<br>` tag that starts at `start`, or `None` when no tag starts there.
fn br_tag_end(characters: &[char], start: usize) -> Option<usize> {
    if characters.get(start) != Some(&'<') {
        return None;
    }
    let is_b = characters
        .get(start + 1)
        .is_some_and(|c| c.eq_ignore_ascii_case(&'b'));
    let is_r = characters
        .get(start + 2)
        .is_some_and(|c| c.eq_ignore_ascii_case(&'r'));
    if !(is_b && is_r) {
        return None;
    }
    let mut index = start + 3;
    while characters
        .get(index)
        .is_some_and(|&c| crate::common::is_js_whitespace(c))
    {
        index += 1;
    }
    if characters.get(index) == Some(&'/') {
        index += 1;
    }
    (characters.get(index) == Some(&'>')).then_some(index + 1)
}

/// The font size of the `tspan` placement, as the configuration gives it: the number, or 14 when
/// the configuration gives none (the schema default, `taskFontSize`).
#[must_use]
pub fn task_font_size(config: &JourneyDiagramConfig) -> f64 {
    match &config.task_font_size {
        Some(ferrigura_model::value::Value::Number(size)) => *size,
        _ => 14.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference's test vector: the default theme's font at 14 px, wrapped at 150 px
    /// (`NOTES.md` 7.6.1), measured in Chromium.
    #[test]
    fn a_long_task_name_breaks_at_the_blanks_the_browser_breaks_at() {
        let lines = break_lines(
            "This is a very long task name that should wrap across several lines in the box",
            150.0,
            14.0,
        )
        .expect("measures");
        let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "This is a very long task",
                "name that should wrap",
                "across several lines in",
                "the box",
            ]
        );
    }

    /// The reference's measurement: the word of 270.796875 px is the widest piece, so the cell is
    /// that wide, and the line `A` is broken off before it (`NOTES.md` 7.6.1; the dump of the
    /// reference's label boxes).
    #[test]
    fn a_long_word_is_the_least_width_of_its_cell() {
        let text = "A reallyveryveryveryveryverylongunbrokenword in a cell";
        let widest = min_content_width_in(regular(), text, 14.0).expect("measures");
        assert_eq!(widest, 270.796875);
        let lines = break_lines(text, widest, 14.0).expect("measures");
        let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "A",
                "reallyveryveryveryveryverylongunbrokenword",
                "in a cell"
            ]
        );
    }

    #[test]
    fn a_word_wider_than_the_cell_is_set_alone() {
        let lines =
            break_lines("Supercalifragilisticexpialidocious", 50.0, 14.0).expect("measures");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].width > 50.0);
    }

    #[test]
    fn a_hyphen_before_a_letter_is_a_break_opportunity() {
        assert_eq!(pieces("well-known"), ["well-", "known"]);
        assert_eq!(pieces("3-4"), ["3-4"]);
        assert_eq!(pieces("a - b"), ["a ", "- ", "b"]);
    }

    #[test]
    fn br_tags_split_in_any_case_with_blanks() {
        assert_eq!(split_br("a<br>b"), ["a", "b"]);
        assert_eq!(split_br("a<BR/>b"), ["a", "b"]);
        assert_eq!(split_br("a<br\t/>b"), ["a", "b"]);
        // Not tags: a blank between `<` and `br`, a blank between the slash and `>`, and `</br>`.
        assert_eq!(split_br("a< br />b"), ["a< br />b"]);
        assert_eq!(split_br("a<br\t/ >b"), ["a<br\t/ >b"]);
        assert_eq!(split_br("a</br>b"), ["a</br>b"]);
        assert_eq!(split_br(""), [""]);
    }
}
