//! The text of the venn diagram: the words of a set label as `wrapText` leaves them, and the lines
//! of a text node as the browser breaks its HTML (`vennRenderer.ts` 344-367, `src/diagram.js`
//! 384-436).

use crate::RenderError;
use crate::common::is_js_whitespace;
use crate::journey::text::{Line, break_lines_in, min_content_width_in, regular};

/// `label.split(/\s+/)`: the runs of white space separate the words, and a white space at either
/// end gives an empty word at that end, as JavaScript's split does.
fn split_js_whitespace(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_space = false;
    for character in text.chars() {
        if is_js_whitespace(character) {
            if !in_space {
                words.push(std::mem::take(&mut current));
            }
            in_space = true;
        } else {
            in_space = false;
            current.push(character);
        }
    }
    words.push(current);
    words
}

/// The text one set label becomes in the reference's `wrapText` (`src/diagram.js` 384-436).
///
/// `wrapText` breaks a label into lines when the line is wider than the circle, but it measures
/// the line with `getComputedTextLength`, which is 0 on the detached element the chart draws into
/// (`docs-port/venn/NOTES.md` 6.6). No line is ever too wide, so the label is one `tspan`: its
/// words joined by one blank. The loop stops at the first empty word, so a white space at the end
/// is left out, and one at the start stays as a blank.
#[must_use]
pub(super) fn set_label_text(label: &str) -> String {
    let mut words = split_js_whitespace(label).into_iter();
    let mut line = vec![words.next().unwrap_or_default()];
    for word in words {
        if word.is_empty() {
            break;
        }
        line.push(word);
    }
    line.join(" ")
}

/// The lines a text node's text takes in a box `box_width` wide at the font size `size`: the
/// browser's flex item is as wide as the box or its widest piece, whichever is wider, and each
/// line is centred in it (`docs-port/venn/NOTES.md` 6.5).
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn text_node_lines(
    text: &str,
    box_width: f64,
    size: f64,
) -> Result<Vec<Line>, RenderError> {
    let face = regular();
    let width = box_width.max(min_content_width_in(face, text, size)?);
    break_lines_in(face, text, width, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_label_is_one_line_with_its_runs_of_blanks_collapsed() {
        assert_eq!(set_label_text("a    b  c"), "a b c");
        assert_eq!(set_label_text(""), "");
        assert_eq!(set_label_text("a b  "), "a b");
        assert_eq!(set_label_text(" a"), " a");
    }
}
