//! Widths of labels, against Chromium and against the properties kerning and ligatures give them.

mod support;

use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face, measure, measure_data};
use support::{
    CHROMIUM_TOLERANCE, chromium, entries, face_for, number, size_of, style_of, text_of,
};

#[test]
fn widths_agree_with_chromium_within_a_hundredth_of_a_pixel() {
    let reference = chromium();
    let widths = entries(&reference, "widths");
    let mut worst: f64 = 0.0;
    let mut failures = Vec::new();
    for entry in &widths {
        let text = text_of(entry);
        let style = style_of(entry);
        let size = size_of(entry);
        let expected = number(entry, "width");
        let actual = measure(face_for(style), text, size).expect("label measures");
        let difference = (actual - expected).abs();
        worst = worst.max(difference);
        if difference > CHROMIUM_TOLERANCE {
            failures.push(format!(
                "{style} {size}px {text:?}: {actual} vs Chromium {expected}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} widths differ by more than {CHROMIUM_TOLERANCE} px:\n{}",
        failures.len(),
        widths.len(),
        failures.join("\n")
    );
    println!(
        "largest difference over {} widths: {worst:.4} px",
        widths.len()
    );
}

#[test]
fn the_fixture_covers_every_face() {
    let reference = chromium();
    let mut styles: Vec<String> = entries(&reference, "widths")
        .iter()
        .map(|entry| style_of(entry).to_string())
        .collect();
    styles.sort();
    styles.dedup();
    assert_eq!(styles.len(), 8, "{styles:?}");
}

#[test]
fn kerning_pairs_make_a_word_narrower_than_its_letters() {
    let face = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    for word in ["AVATAR", "Type", "WAVE", "Yo", "Te", "LT"] {
        let letters: f64 = word
            .chars()
            .map(|c| measure(face, &c.to_string(), 16.0).expect("letter measures"))
            .sum();
        let whole = measure(face, word, 16.0).expect("word measures");
        assert!(
            whole < letters,
            "{word}: kerned {whole} px, letters {letters} px"
        );
    }
}

#[test]
fn a_leading_space_adds_its_advance() {
    let face = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let plain = measure(face, "leading", 16.0).expect("measures");
    let spaced = measure(face, " leading", 16.0).expect("measures");
    assert!(spaced > plain, "{spaced} should exceed {plain}");
}

#[test]
fn widths_scale_linearly_with_the_size() {
    let face = face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal);
    let at_16 = measure(face, "Hyperlegible", 16.0).expect("measures");
    let at_32 = measure(face, "Hyperlegible", 32.0).expect("measures");
    assert!((at_32 - 2.0 * at_16).abs() < 1e-9, "{at_16} {at_32}");
}

#[test]
fn measuring_the_bundled_bytes_matches_the_face() {
    let reference = chromium();
    for entry in entries(&reference, "widths").iter().take(40) {
        let text = text_of(entry);
        let size = size_of(entry);
        let found: &Face = face_for(style_of(entry));
        let through_face = measure(found, text, size).expect("measures");
        let through_bytes = measure_data(found.data(), text, size).expect("measures");
        assert_eq!(through_face, through_bytes, "{text:?}");
    }
}
