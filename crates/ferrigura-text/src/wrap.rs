//! Mermaid's line wrapping for SVG labels, ported from `splitLineToFitWidth` in
//! `packages/mermaid/src/rendering-util/splitText.ts` and `createFormattedText` in `createText.ts`.
//!
//! A line is split at single spaces and the words are packed greedily. A line is measured as one
//! `tspan` per word, every word after the first with a leading space, so a candidate line is the
//! sum of those word widths. A word that does not fit on an empty line is split into grapheme
//! clusters, and each line gets at least one grapheme.

use std::collections::VecDeque;

use unicode_segmentation::UnicodeSegmentation;

use crate::error::TextError;
use crate::fonts::Face;
use crate::measure::measure;

/// Wraps one line of text, with no line breaks in it, into lines no wider than `max_width` pixels,
/// as Mermaid does for SVG labels.
///
/// Words are the pieces between single spaces, so empty pieces disappear: leading, trailing and
/// repeated spaces are not kept in the wrapped lines. A word wider than `max_width` on its own is
/// split into grapheme clusters, and each line takes at least one grapheme even when that grapheme
/// is wider than `max_width`.
///
/// # Errors
///
/// Returns an error if the face cannot be shaped.
pub fn wrap_line(
    text: &str,
    face: &Face,
    size: f64,
    max_width: f64,
) -> Result<Vec<String>, TextError> {
    let mut pending: VecDeque<String> = text
        .split(' ')
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    let mut lines = Vec::new();
    let mut current: Vec<String> = Vec::new();
    // The width of `current` as Mermaid's tspans: the first word and then each word with its space.
    let mut current_width = 0.0;

    while let Some(word) = pending.pop_front() {
        let candidate = if current.is_empty() {
            measure(face, &word, size)?
        } else {
            current_width + measure(face, &format!(" {word}"), size)?
        };
        if candidate <= max_width {
            current_width = candidate;
            current.push(word);
        } else if !current.is_empty() {
            lines.push(current.join(" "));
            current.clear();
            current_width = 0.0;
            pending.push_front(word);
        } else {
            let graphemes: Vec<&str> = word.graphemes(true).collect();
            let mut used = 0;
            while used < graphemes.len()
                && measure(face, &graphemes[..=used].concat(), size)? <= max_width
            {
                used += 1;
            }
            let used = used.max(1);
            lines.push(graphemes[..used].concat());
            let rest = graphemes[used..].concat();
            if !rest.is_empty() {
                pending.push_front(rest);
            }
        }
    }
    if !current.is_empty() {
        lines.push(current.join(" "));
    }
    Ok(lines)
}
