//! Mermaid's line wrapping for SVG labels, ported from `splitLineToFitWidth` and `splitWordToFitWidth`
//! in `packages/mermaid/src/rendering-util/splitText.ts`, and from `createFormattedText` in
//! `createText.ts`.
//!
//! A line is a list of words, each set in its own face, so bold and italic words from Markdown are
//! measured in the face they are drawn in. A line is measured as the reference measures it: one
//! `tspan` per word, every word after the first with a leading space. A line that fits is kept
//! whole; otherwise the words are packed greedily, and a word that does not fit on an empty line is
//! split into grapheme clusters, each line taking at least one grapheme. Whatever the caller
//! measures with is passed in as `fits`, so the same algorithm serves the spec tests (which count
//! characters) and the label layout (which measures text).

use std::collections::VecDeque;

use unicode_segmentation::UnicodeSegmentation;

use crate::error::TextError;
use crate::fonts::Face;
use crate::measure::{layout_width, measure};

/// A word of a line: its text and the face it is set in.
#[derive(Debug, Clone)]
pub struct Word<'a> {
    /// The text of the word. Wrapping splits it into graphemes, and it is measured as the caller
    /// shows it.
    pub text: String,
    /// The face the word is set in.
    pub face: &'a Face,
}

/// `splitTextToChars` in `splitText.ts`: the grapheme clusters of `text`.
#[must_use]
pub fn split_text_to_chars(text: &str) -> Vec<&str> {
    text.graphemes(true).collect()
}

/// `splitLineToWords` in `splitText.ts`: the word segments of `text` by Unicode word boundaries,
/// where the spaces between words are segments of their own.
#[must_use]
pub fn split_line_to_words(text: &str) -> Vec<&str> {
    text.split_word_bounds().collect()
}

/// The width of a line of words: each word is measured in its face at `size` pixels, every word
/// after the first with a leading space, as the reference measures its `tspan`s, and each word's
/// width is snapped as Chromium lays it out ([`layout_width`]), so the sum is the reference's
/// `getComputedTextLength`. `shown` gives the
/// text a word is drawn with, which can differ from its text (the reference decodes entities
/// before it measures).
///
/// # Errors
///
/// Returns the first error [`measure`] reports for a word.
pub fn measure_words(
    words: &[Word],
    size: f64,
    shown: impl Fn(&str) -> String,
) -> Result<f64, TextError> {
    let mut total = 0.0;
    for (index, word) in words.iter().enumerate() {
        let shown = shown(&word.text);
        let text = if index == 0 {
            shown
        } else {
            format!(" {shown}")
        };
        total += layout_width(measure(word.face, &text, size)?);
    }
    Ok(total)
}

/// `splitWordToFitWidth` in `splitText.ts`: the longest prefix of `word`, in whole graphemes, that
/// `fits` accepts as a line of its own, and the rest. A first grapheme that does not fit is kept
/// anyway, so the split always makes progress.
///
/// # Errors
///
/// Returns the first error `fits` reports.
pub fn split_word_to_fit_width<'a, F>(
    word: &Word<'a>,
    fits: &mut F,
) -> Result<(Word<'a>, Word<'a>), TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
{
    let mut used = Word {
        text: String::with_capacity(word.text.len()),
        face: word.face,
    };
    let mut rest_offset = 0;
    for grapheme in word.text.graphemes(true) {
        let prev_len = used.text.len();
        used.text.push_str(grapheme);
        if fits(std::slice::from_ref(&used))? {
            rest_offset = used.text.len();
            continue;
        }
        if prev_len == 0 {
            // The first grapheme does not fit, but it is kept: a line holds at least one.
            rest_offset = used.text.len();
        } else {
            used.text.truncate(prev_len);
        }
        break;
    }
    let rest = Word {
        text: word.text[rest_offset..].to_owned(),
        face: word.face,
    };
    Ok((used, rest))
}

/// `splitLineToFitWidth` in `splitText.ts`: packs the words of a line greedily into lines that
/// `fits` accepts. A word that does not fit on a line of its own is split by
/// [`split_word_to_fit_width`]. A joiner word that is a single space is taken as the space between
/// two words, and it is dropped where the line breaks, as the reference drops it.
///
/// # Errors
///
/// Returns [`TextError::NewlineInLine`] if a word holds a newline, as the reference throws for
/// it, or the first error `fits` reports.
pub fn split_line_to_fit_width<'a, F>(
    line: &[Word<'a>],
    fits: &mut F,
) -> Result<Vec<Vec<Word<'a>>>, TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
{
    split_line_keeping_words(line, fits, None::<&mut F>)
}

/// [`split_line_to_fit_width`] with an optional second test, `whole`: a word that does not fit on
/// a line of its own but that `whole` accepts stays one line instead of being split inside. A word
/// that `whole` rejects is split as before. With `None` this is exactly the reference's algorithm.
///
/// # Errors
///
/// As [`split_line_to_fit_width`], and the first error `whole` reports.
pub fn split_line_keeping_words<'a, F, G>(
    line: &[Word<'a>],
    fits: &mut F,
    mut whole: Option<&mut G>,
) -> Result<Vec<Vec<Word<'a>>>, TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
    G: FnMut(&[Word]) -> Result<bool, TextError>,
{
    if line.iter().any(|word| word.text.contains('\n')) {
        return Err(TextError::NewlineInLine);
    }
    let mut words: VecDeque<Word> = line.iter().cloned().collect();
    let mut lines: Vec<Vec<Word>> = Vec::new();
    let mut new_line: Vec<Word> = Vec::new();
    // Set when a word was too wide to stay whole: its rest is broken at the width too.
    let mut breaking = false;
    while let Some(first) = words.pop_front() {
        let broken_rest = std::mem::take(&mut breaking);
        let (joiner, next) = if first.text == " " {
            let next = words.pop_front().unwrap_or_else(|| Word {
                text: " ".to_owned(),
                face: first.face,
            });
            (Some(first), next)
        } else {
            (None, first)
        };

        let has_joiner = joiner.is_some();
        if let Some(joiner) = joiner {
            new_line.push(joiner);
        }
        new_line.push(next);
        if fits(&new_line)? {
            continue;
        }
        let Some(next) = new_line.pop() else {
            continue;
        };
        if has_joiner {
            new_line.pop();
        }

        if !new_line.is_empty() {
            // The word starts the next line; the joiner before it is dropped.
            lines.push(std::mem::take(&mut new_line));
            words.push_front(next);
        } else if !next.text.is_empty() {
            if let Some(whole) = whole.as_deref_mut()
                && !broken_rest
                && whole(std::slice::from_ref(&next))?
            {
                lines.push(vec![next]);
                continue;
            }
            let (first_part, rest) = split_word_to_fit_width(&next, fits)?;
            lines.push(vec![first_part]);
            if !rest.text.is_empty() {
                breaking = whole.is_some();
                words.push_front(rest);
            }
        }
    }
    if !new_line.is_empty() {
        lines.push(new_line);
    }
    Ok(lines)
}

/// [`fit_line`] that keeps a word whole where `whole` accepts it (see [`split_line_keeping_words`]).
///
/// # Errors
///
/// As [`fit_line`].
pub fn fit_line_keeping_words<'a, F, G>(
    line: &[Word<'a>],
    mut fits: F,
    mut whole: G,
) -> Result<Vec<Vec<Word<'a>>>, TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
    G: FnMut(&[Word]) -> Result<bool, TextError>,
{
    if fits(line)? {
        return Ok(vec![line.to_vec()]);
    }
    split_line_keeping_words(line, &mut fits, Some(&mut whole))
}

/// `createFormattedText` in `createText.ts`: a line that fits is kept as it is, including an empty
/// line, which fits every width; otherwise it is split by [`split_line_to_fit_width`].
///
/// # Errors
///
/// Returns the errors of [`split_line_to_fit_width`] and of `fits`.
pub fn fit_line<'a, F>(line: &[Word<'a>], mut fits: F) -> Result<Vec<Vec<Word<'a>>>, TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
{
    if fits(line)? {
        return Ok(vec![line.to_vec()]);
    }
    split_line_to_fit_width(line, &mut fits)
}

/// Wraps one line of text, with no line breaks in it, into lines no wider than `max_width` pixels,
/// as Mermaid does for SVG labels, with every word in `face`.
///
/// Words are the pieces between single spaces, so empty pieces disappear: leading, trailing and
/// repeated spaces are not kept in the wrapped lines. A text without words has no lines.
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
    let words: Vec<Word> = text
        .split(' ')
        .filter(|word| !word.is_empty())
        .map(|word| Word {
            text: word.to_owned(),
            face,
        })
        .collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let lines = fit_line(&words, |line| {
        Ok(measure_words(line, size, str::to_owned)? <= max_width)
    })?;
    Ok(lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{FontFamily, FontStyle, FontWeight, face};

    fn word(text: &str) -> Word<'static> {
        Word {
            text: text.to_owned(),
            face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        }
    }

    #[test]
    fn split_word_in_place_handles_graphemes_empty_and_single_grapheme_overflow() {
        // Empty word stays empty.
        let (first, rest) =
            split_word_to_fit_width(&word(""), &mut |w| Ok(w[0].text.chars().count() <= 3))
                .unwrap();
        assert_eq!((first.text.as_str(), rest.text.as_str()), ("", ""));

        // Multi-codepoint grapheme clusters are never split mid-cluster, and an oversized first
        // grapheme still makes progress.
        let (first, rest) =
            split_word_to_fit_width(&word("👍🏽e\u{301}z"), &mut |_| Ok(false)).unwrap();
        assert_eq!(
            (first.text.as_str(), rest.text.as_str()),
            ("👍🏽", "e\u{301}z")
        );

        let (first, rest) = split_word_to_fit_width(&word("👍🏽e\u{301}z"), &mut |w| {
            Ok(w[0].text.graphemes(true).count() <= 2)
        })
        .unwrap();
        assert_eq!(
            (first.text.as_str(), rest.text.as_str()),
            ("👍🏽e\u{301}", "z")
        );
    }

    #[test]
    fn split_line_push_pop_drops_joiners_on_break_and_preserves_order() {
        let line = vec![
            word("alpha"),
            word(" "),
            word("beta"),
            word(" "),
            word("gamma"),
            word(" "),
            word("superlongword"),
        ];
        let wrapped = split_line_to_fit_width(&line, &mut |words| {
            let chars: usize = words.iter().map(|w| w.text.chars().count()).sum();
            Ok(chars <= 10)
        })
        .unwrap();
        let texts: Vec<String> = wrapped
            .iter()
            .map(|l| l.iter().map(|w| w.text.as_str()).collect())
            .collect();
        assert_eq!(texts, vec!["alpha beta", "gamma", "superlongw", "ord"]);
    }

    #[test]
    fn split_line_rejects_embedded_newline() {
        let line = vec![word("ok"), word("bad\nword")];
        assert!(matches!(
            split_line_to_fit_width(&line, &mut |_| Ok(true)),
            Err(TextError::NewlineInLine)
        ));
    }
}
