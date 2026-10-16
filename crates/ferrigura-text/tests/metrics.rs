//! Line metrics: ascent and descent rounded to whole pixels, and the text box built from them.

mod support;

use ferrigura_text::metrics::{FontMetrics, font_metrics, text_box, text_box_from};
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face};
use support::{chromium, entries, face_for, number, size_of, style_of};

#[test]
fn text_box_matches_chromium_for_every_face_and_size() {
    let reference = chromium();
    let boxes = entries(&reference, "boxes");
    for entry in &boxes {
        let style = style_of(entry);
        let size = size_of(entry);
        let found = text_box(face_for(style), size).expect("box derives");
        assert_eq!(
            found.ascent,
            number(entry, "ascent"),
            "{style} {size}px ascent"
        );
        assert_eq!(
            found.descent,
            number(entry, "descent"),
            "{style} {size}px descent"
        );
        assert_eq!(found.top, found.ascent, "{style} {size}px top");
        assert_eq!(
            found.height,
            found.ascent + found.descent,
            "{style} {size}px height"
        );
    }
    assert_eq!(
        boxes.len(),
        230,
        "the eight faces at the key sizes, the regular faces at 2 to 60 px, and fractional sizes"
    );
}

#[test]
fn ascent_and_descent_are_the_unrounded_metrics_rounded_with_ties_down() {
    let reference = chromium();
    for entry in entries(&reference, "boxes") {
        let style = style_of(&entry);
        let size = size_of(&entry);
        let found = face_for(style);
        // Chromium sets the text at the size floored to 1/100 px.
        let laid_out = (size * 100.0 + 1e-6).floor() / 100.0;
        let unrounded = font_metrics(found, laid_out).expect("metrics read");
        let boxed = text_box(found, size).expect("box derives");
        assert_rounded_to_whole_pixel(
            boxed.ascent,
            unrounded.ascent,
            &format!("{style} {size}px ascent"),
        );
        assert_rounded_to_whole_pixel(
            boxed.descent,
            unrounded.descent,
            &format!("{style} {size}px descent"),
        );
    }
}

/// Asserts that `rounded` is `unrounded` rounded to a whole pixel, with a tie going down.
fn assert_rounded_to_whole_pixel(rounded: f64, unrounded: f64, what: &str) {
    assert_eq!(rounded.fract(), 0.0, "{what}");
    assert!(
        (rounded - unrounded).abs() <= 0.5,
        "{what}: {rounded} for {unrounded}"
    );
    if unrounded.fract() == 0.5 {
        assert_eq!(rounded, unrounded - 0.5, "{what}: a tie goes down");
    }
}

#[test]
fn a_tie_rounds_down_for_ascent_and_descent_alike() {
    let tie = text_box_from(FontMetrics {
        ascent: 28.5,
        descent: 8.5,
        line_gap: 0.0,
    });
    assert_eq!((tie.ascent, tie.descent, tie.height), (28.0, 8.0, 36.0));
    let above = text_box_from(FontMetrics {
        ascent: 28.6,
        descent: 8.6,
        line_gap: 0.0,
    });
    assert_eq!(
        (above.ascent, above.descent, above.height),
        (29.0, 9.0, 38.0)
    );
}

#[test]
fn the_size_is_floored_to_a_hundredth_before_the_box_is_taken() {
    // Chromium's box at 30.001 px is that of 30.00 px: an ascent of 28, where 28.5009 would round to 29.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    assert_eq!(text_box(sans, 30.001).expect("box derives").ascent, 28.0);
    assert_eq!(text_box(sans, 10.005).expect("box derives").ascent, 9.0);
    // At 30.999 px the mono ascent is 30.99 * 0.984 = 30.49, not 31 as the exact size gives.
    let mono = face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal);
    assert_eq!(text_box(mono, 30.999).expect("box derives").ascent, 30.0);
}

#[test]
fn rounding_changes_the_box_for_some_sizes() {
    // The rounded box is not the unrounded one, so the rounding is observable.
    let found = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let unrounded = font_metrics(found, 16.0).expect("metrics read");
    let boxed = text_box(found, 16.0).expect("box derives");
    assert!(
        unrounded.ascent.fract() != 0.0 || unrounded.descent.fract() != 0.0,
        "{unrounded:?}"
    );
    assert!(boxed.height != unrounded.ascent + unrounded.descent);
}

#[test]
fn the_box_grows_with_the_size() {
    let found = face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal);
    let small = text_box(found, 14.0).expect("box derives");
    let large = text_box(found, 24.0).expect("box derives");
    assert!(large.height > small.height, "{small:?} {large:?}");
    assert!(large.top > small.top, "{small:?} {large:?}");
}
