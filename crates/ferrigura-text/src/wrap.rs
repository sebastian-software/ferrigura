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
pub struct Word {
    /// The text of the word. Wrapping splits it into graphemes, and it is measured as the caller
    /// shows it.
    pub text: String,
    /// The face the word is set in.
    pub face: &'static Face,
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
pub fn split_word_to_fit_width<F>(word: &Word, fits: &mut F) -> Result<(Word, Word), TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
{
    let characters = split_text_to_chars(&word.text);
    let mut used = String::new();
    let mut index = 0;
    while index < characters.len() {
        let candidate = format!("{used}{}", characters[index]);
        let single = [Word {
            text: candidate.clone(),
            face: word.face,
        }];
        if fits(&single)? {
            used = candidate;
            index += 1;
            continue;
        }
        if used.is_empty() {
            // The first grapheme does not fit, but it is kept: a line holds at least one.
            used.push_str(characters[index]);
            index += 1;
        }
        break;
    }
    let rest: String = characters[index..].concat();
    Ok((
        Word {
            text: used,
            face: word.face,
        },
        Word {
            text: rest,
            face: word.face,
        },
    ))
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
pub fn split_line_to_fit_width<F>(line: &[Word], fits: &mut F) -> Result<Vec<Vec<Word>>, TextError>
where
    F: FnMut(&[Word]) -> Result<bool, TextError>,
{
    if line.iter().any(|word| word.text.contains('\n')) {
        return Err(TextError::NewlineInLine);
    }
    let mut words: VecDeque<Word> = line.iter().cloned().collect();
    let mut lines: Vec<Vec<Word>> = Vec::new();
    let mut new_line: Vec<Word> = Vec::new();
    while let Some(first) = words.pop_front() {
        let (joiner, next) = if first.text == " " {
            let next = words.pop_front().unwrap_or_else(|| Word {
                text: " ".to_owned(),
                face: first.face,
            });
            (Some(first), next)
        } else {
            (None, first)
        };

        let mut with_next = new_line.clone();
        if let Some(joiner) = joiner {
            with_next.push(joiner);
        }
        with_next.push(next.clone());
        if fits(&with_next)? {
            new_line = with_next;
            continue;
        }

        if !new_line.is_empty() {
            // The word starts the next line; the joiner before it is dropped.
            lines.push(std::mem::take(&mut new_line));
            words.push_front(next);
        } else if !next.text.is_empty() {
            let (first_part, rest) = split_word_to_fit_width(&next, fits)?;
            lines.push(vec![first_part]);
            if !rest.text.is_empty() {
                words.push_front(rest);
            }
        }
    }
    if !new_line.is_empty() {
        lines.push(new_line);
    }
    Ok(lines)
}

/// `createFormattedText` in `createText.ts`: a line that fits is kept as it is, including an empty
/// line, which fits every width; otherwise it is split by [`split_line_to_fit_width`].
///
/// # Errors
///
/// Returns the errors of [`split_line_to_fit_width`] and of `fits`.
pub fn fit_line<F>(line: &[Word], mut fits: F) -> Result<Vec<Vec<Word>>, TextError>
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
    face: &'static Face,
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
