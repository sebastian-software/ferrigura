//! Mermaid's SVG line wrapping: the lines match the JS algorithm run in Chromium, and the algorithm's
//! rules hold on hand-picked strings.

mod support;

use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, measure, wrap_line};
use support::{chromium, entries, face_for, number, size_of, style_of, text_of};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn wrapped_lines_match_mermaid_in_chromium() {
    let reference = chromium();
    let wraps = entries(&reference, "wraps");
    assert_eq!(wraps.len(), 27, "nine strings at three widths");
    for entry in &wraps {
        let text = text_of(entry);
        let width = number(entry, "wrapWidth");
        let lines =
            wrap_line(text, face_for(style_of(entry)), size_of(entry), width).expect("text wraps");
        let expected: Vec<String> = entry["lines"]
            .as_array()
            .expect("entry has lines")
            .iter()
            .map(|line| line.as_str().expect("line is text").to_string())
            .collect();
        assert_eq!(lines, expected, "{text:?} at {width} px");
    }
}

#[test]
fn a_word_too_wide_for_a_line_is_split_into_graphemes() {
    // The word is wider than 80 px, so it is cut where the next grapheme would not fit.
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let lines =
        wrap_line("Supercalifragilisticexpialidocious", regular, 16.0, 80.0).expect("text wraps");
    assert_eq!(
        lines,
        ["Supercalifr", "agilisticex", "pialidociou", "s"],
        "{lines:?}"
    );
}

#[test]
fn each_line_fits_or_holds_one_grapheme() {
    let reference = chromium();
    for entry in entries(&reference, "wraps") {
        let width = number(&entry, "wrapWidth");
        let face = face_for(style_of(&entry));
        for line in entry["lines"].as_array().expect("entry has lines") {
            let line = line.as_str().expect("line is text");
            let graphemes = line.graphemes(true).count();
            let fits = measure(face, line, size_of(&entry)).expect("line measures") <= width;
            assert!(
                fits || graphemes == 1,
                "{line:?} is wider than {width} px and holds {graphemes} graphemes"
            );
        }
    }
}

#[test]
fn no_text_is_lost_or_reordered() {
    let reference = chromium();
    for entry in entries(&reference, "wraps") {
        let text = text_of(&entry);
        let lines = wrap_line(
            text,
            face_for(style_of(&entry)),
            size_of(&entry),
            number(&entry, "wrapWidth"),
        )
        .expect("text wraps");
        let wrapped: String = lines.concat();
        let original: String = text.chars().filter(|c| *c != ' ').collect();
        assert_eq!(wrapped.replace(' ', ""), original, "{text:?}");
    }
}

#[test]
fn a_line_that_fits_is_not_split() {
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let lines = wrap_line("The quick brown fox", regular, 16.0, 1000.0).expect("text wraps");
    assert_eq!(lines, ["The quick brown fox"]);
}

#[test]
fn spaces_at_the_ends_and_between_words_are_dropped() {
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let lines =
        wrap_line("  leading   and  trailing  ", regular, 16.0, 1000.0).expect("text wraps");
    assert_eq!(lines, ["leading and trailing"]);
}

#[test]
fn zero_width_gives_one_grapheme_per_line() {
    // A combining accent belongs to the letter before it, so the letter and accent stay together.
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let lines = wrap_line("e\u{301}x", regular, 16.0, 0.0).expect("text wraps");
    assert_eq!(lines, ["e\u{301}", "x"]);
}

#[test]
fn empty_text_has_no_lines() {
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    assert!(
        wrap_line("", regular, 16.0, 80.0)
            .expect("text wraps")
            .is_empty()
    );
    assert!(
        wrap_line("   ", regular, 16.0, 80.0)
            .expect("text wraps")
            .is_empty()
    );
}
