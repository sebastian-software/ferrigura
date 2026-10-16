//! Line metrics: ascent and descent rounded to whole pixels, and the text box built from them.

mod support;

use ferrigura_text::metrics::{font_metrics, text_box};
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
fn ascent_and_descent_are_the_unrounded_metrics_rounded() {
    let reference = chromium();
    for entry in entries(&reference, "boxes") {
        let style = style_of(&entry);
        let size = size_of(&entry);
        let found = face_for(style);
        let unrounded = font_metrics(found, size).expect("metrics read");
        let boxed = text_box(found, size).expect("box derives");
        assert_eq!(boxed.ascent, unrounded.ascent.round(), "{style} {size}px");
        assert_eq!(boxed.descent, unrounded.descent.round(), "{style} {size}px");
        assert_eq!(boxed.ascent.fract(), 0.0, "{style} {size}px");
        assert_eq!(boxed.descent.fract(), 0.0, "{style} {size}px");
    }
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
