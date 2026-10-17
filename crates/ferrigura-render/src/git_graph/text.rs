//! The text of a gitGraph: the faces it is set in, the rows of a label, and the box that `getBBox`
//! gives a line or a multi-line label. The reference measures every label in the browser, so the
//! widths come from `ferrigura-text`, which measures as Chromium does: the advance
//! (`measure::layout_width`) where the reference centres a label by the browser, and the box of its
//! glyphs (`bbox::measure_box`) where the reference reads `getBBox`.

use ferrigura_text::{
    Extent, Face, FontFamily, FontStyle, FontWeight, face, layout_width, measure, measure_box,
    text_box,
};

use super::tree::Element;
use crate::RenderError;
use crate::unified::geom::BBox;

/// The regular face: the root's text, the tags and the title.
#[must_use]
pub fn regular() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

/// The bold face: the commit labels and the branch labels, which the redux themes set at weight
/// 600. The bundled family has no face between its regular and bold, so 600 takes the bold one, as
/// Chromium's font matching picks the heavier face for a weight above 500.
#[must_use]
pub fn bold() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal)
}

/// The advance of a line of text in a face at a size, snapped to the browser's layout unit. A
/// `text-anchor: middle` label is shifted left by half of it, so this is the width to centre by.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn width(face: &'static Face, text: &str, size: f64) -> Result<f64, RenderError> {
    Ok(layout_width(measure(face, text, size)?))
}

/// The box `getBBox` reports for a single line of text: its advance, and the glyph outlines where
/// they reach past it, relative to the start of the text. The reference sizes its label backgrounds
/// and places its tags by this box (`gitGraphRenderer.ts` 351-358 and 424-432).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn extent(face: &'static Face, text: &str, size: f64) -> Result<Extent, RenderError> {
    Ok(measure_box(face, text, size)?)
}

/// The rows of a label. The reference splits a string on a backslash followed by `n`, on a line
/// feed and on a `<br>` tag (any case, with optional space and slash), and trims each row.
#[must_use]
pub fn rows(text: &str) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        let separator = if rest.starts_with("\\n") {
            Some(2)
        } else if first == '\n' {
            Some(1)
        } else {
            break_tag_length(rest)
        };
        match separator {
            Some(length) => {
                rows.push(std::mem::take(&mut current));
                rest = &rest[length..];
            }
            None => {
                current.push(first);
                rest = &rest[first.len_utf8()..];
            }
        }
    }
    rows.push(current);
    rows.into_iter().map(|row| row.trim().to_owned()).collect()
}

/// The length of a `<br>` tag at the start of `text`, as `<br\s*\/?>` matches it with the `i`
/// flag, or `None`.
fn break_tag_length(text: &str) -> Option<usize> {
    let head = text.get(..3)?;
    if !head.eq_ignore_ascii_case("<br") {
        return None;
    }
    let mut length = 3;
    let bytes = text.as_bytes();
    while length < bytes.len() && bytes[length].is_ascii_whitespace() {
        length += 1;
    }
    if length < bytes.len() && bytes[length] == b'/' {
        length += 1;
    }
    (length < bytes.len() && bytes[length] == b'>').then_some(length + 1)
}

/// The text element that `drawText` makes: one `tspan` per row, each at `x = 0` and one em below
/// the row before (the first one em below the origin), together with the box `getBBox` reports
/// for it: the union of the rows' boxes, each from the start of the line to its advance and from
/// the ascent above its baseline to the descent below it.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn draw_text(
    text: &str,
    face: &'static Face,
    size: f64,
) -> Result<(Element, BBox), RenderError> {
    let metrics = text_box(face, size)?;
    let rows = rows(text);
    let mut tspans = Vec::with_capacity(rows.len());
    let mut bounds: Option<BBox> = None;
    for (line, row) in rows.iter().enumerate() {
        tspans.push(Element::tspan(row));
        let baseline = size * (line as f64 + 1.0);
        let ink = extent(face, row, size)?;
        let row_box = BBox::new(
            ink.left,
            baseline - metrics.ascent,
            ink.width(),
            metrics.height,
        );
        bounds = Some(match bounds {
            None => row_box,
            Some(so_far) => union_nonempty(so_far, row_box),
        });
    }
    let bounds = bounds.unwrap_or_default();
    let mut element = Element::text_lines(bounds);
    for tspan in tspans {
        element.push(tspan);
    }
    Ok((element, bounds))
}

/// The union of two boxes, leaving out an empty one as `getBBox` does.
fn union_nonempty(a: BBox, b: BBox) -> BBox {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => a,
        (true, false) => b,
        (false, true) => a,
        (false, false) => a.union(b),
    }
}

/// The box of a single line set at `x` with its baseline at `baseline`, as `getBBox` gives a `text`
/// element: horizontally the `extent` of the line from `x`, and vertically from the rounded ascent
/// above the baseline to the rounded descent below it.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn line_box(
    face: &'static Face,
    size: f64,
    x: f64,
    baseline: f64,
    extent: Extent,
) -> Result<BBox, RenderError> {
    let metrics = text_box(face, size)?;
    Ok(BBox::new(
        x + extent.left,
        baseline - metrics.ascent,
        extent.width(),
        metrics.height,
    ))
}

/// The height of a line's box in a face at a size: the rounded ascent plus the rounded descent.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn line_height(face: &'static Face, size: f64) -> Result<f64, RenderError> {
    Ok(text_box(face, size)?.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `drawText` of the reference's in-source test: the label is one row, `main`.
    #[test]
    fn should_draw_text() {
        let (element, bounds) = draw_text("main", regular(), 14.0).unwrap();
        let mut markup = String::new();
        element.write(&mut markup);
        assert!(markup.contains(">main</tspan>"), "{markup}");
        assert!(bounds.width > 0.0);
    }

    #[test]
    fn rows_split_on_a_backslash_n_a_line_feed_and_a_break_tag() {
        assert_eq!(rows("a\\nb<BR/>c\nd"), ["a", "b", "c", "d"]);
        assert_eq!(rows(" spaced <br > end "), ["spaced", "end"]);
        assert_eq!(rows(""), [""]);
    }
}
