//! Subsets for embedding: the subset measures every label as the full font does, kerning survives,
//! and family names are unique to their subset.

mod support;

use std::collections::BTreeMap;

use base64::Engine;
use ferrigura_text::{
    Face, FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure, measure_data,
    subset_face,
};
use support::{chromium, entries, face_for, size_of, style_of, text_of};

fn regular() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

fn bold() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal)
}

#[test]
fn subset_widths_equal_the_full_font_widths() {
    let reference = chromium();
    let widths = entries(&reference, "widths");
    // Each face is subset to the characters of every label drawn in it, as a diagram would do.
    let mut texts: BTreeMap<String, String> = BTreeMap::new();
    for entry in &widths {
        let joined = texts.entry(style_of(entry).to_string()).or_default();
        joined.push_str(text_of(entry));
        joined.push(' ');
    }
    let mut checked = 0;
    for (style, text) in &texts {
        let full = face_for(style);
        let subset = subset_face(full, text).expect("face subsets");
        for entry in widths.iter().filter(|entry| style_of(entry) == style) {
            let label = text_of(entry);
            let size = size_of(entry);
            let with_full = measure(full, label, size).expect("measures");
            let with_subset = measure_data(&subset, label, size).expect("measures");
            assert_eq!(with_subset, with_full, "{style} {size}px {label:?}");
            checked += 1;
        }
    }
    assert_eq!(checked, widths.len());
}

#[test]
fn a_subset_is_smaller_than_the_full_font() {
    let subset = subset_face(regular(), "Hello world").expect("face subsets");
    assert!(
        subset.len() < regular().data().len() / 4,
        "{}",
        subset.len()
    );
}

#[test]
fn a_subset_keeps_the_kerning_of_its_face() {
    // The subset holds only these letters, and A V is still kerned in it.
    let subset = subset_face(regular(), "AVATAR").expect("face subsets");
    let kerned = measure_data(&subset, "AV", 16.0).expect("measures");
    let apart = measure_data(&subset, "A", 16.0).expect("measures")
        + measure_data(&subset, "V", 16.0).expect("measures");
    assert!(kerned < apart, "kerned {kerned} px, apart {apart} px");
    assert_eq!(kerned, measure(regular(), "AV", 16.0).expect("measures"));
}

#[test]
fn equal_subsets_share_a_family_name() {
    let first = [
        Run {
            text: "Hello",
            face: regular(),
        },
        Run {
            text: "world",
            face: bold(),
        },
    ];
    // The same characters in each face, in other labels and another order.
    let second = [
        Run {
            text: "dlrow",
            face: bold(),
        },
        Run {
            text: "olleH",
            face: regular(),
        },
    ];
    let a = embed_runs(&first).expect("embeds");
    let b = embed_runs(&second).expect("embeds");
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].name, b[0].name);
    assert_eq!(a[0].css, b[0].css);
}

#[test]
fn different_subsets_get_different_family_names() {
    let base = [Run {
        text: "Hello",
        face: regular(),
    }];
    let more = [Run {
        text: "Hello!",
        face: regular(),
    }];
    let other_face = [Run {
        text: "Hello",
        face: bold(),
    }];
    let name = |runs: &[Run<'_>]| embed_runs(runs).expect("embeds")[0].name.clone();
    assert_ne!(name(&base), name(&more));
    assert_ne!(name(&base), name(&other_face));
    assert_ne!(name(&more), name(&other_face));
}

#[test]
fn the_name_has_the_documented_form() {
    let name = embed_runs(&[Run {
        text: "Hello",
        face: regular(),
    }])
    .expect("embeds")[0]
        .name
        .clone();
    let parts: Vec<&str> = name.split('-').collect();
    assert_eq!(parts.len(), 3, "{name}");
    assert_eq!(parts[0], "fg");
    assert_eq!(parts[1].len(), 16, "{name}");
    assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()), "{name}");
    assert_eq!(parts[2], "sans");
}

#[test]
fn each_family_gets_its_own_name() {
    let runs = [
        Run {
            text: "Hello",
            face: regular(),
        },
        Run {
            text: "code",
            face: face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal),
        },
    ];
    let families = embed_runs(&runs).expect("embeds");
    assert_eq!(families.len(), 2);
    assert_eq!(families[0].family, FontFamily::Sans);
    assert!(families[0].name.ends_with("-sans"), "{}", families[0].name);
    assert_eq!(families[1].family, FontFamily::Mono);
    assert!(families[1].name.ends_with("-mono"), "{}", families[1].name);
    assert_ne!(families[0].name, families[1].name);
}

#[test]
fn the_css_has_one_rule_per_face_in_use() {
    let runs = [
        Run {
            text: "Hello",
            face: regular(),
        },
        Run {
            text: "world",
            face: bold(),
        },
    ];
    let family = &embed_runs(&runs).expect("embeds")[0];
    assert_eq!(
        family.css.matches("@font-face").count(),
        2,
        "{}",
        family.css
    );
    assert!(family.css.contains("font-weight:400;font-style:normal;"));
    assert!(family.css.contains("font-weight:700;font-style:normal;"));
    assert!(!family.css.contains("font-style:italic"));
    assert!(
        family
            .css
            .contains(&format!("font-family:\"{}\"", family.name))
    );
    assert!(
        !family.css.contains("white-space"),
        "the renderer sets white-space"
    );
}

#[test]
fn the_css_embeds_exactly_the_subset() {
    let text = "Hello world";
    let runs = [Run {
        text,
        face: regular(),
    }];
    let family = &embed_runs(&runs).expect("embeds")[0];
    let prefix = "url(data:font/ttf;base64,";
    let start = family.css.find(prefix).expect("data URL") + prefix.len();
    let end = start + family.css[start..].find(')').expect("data URL ends");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&family.css[start..end])
        .expect("base64 decodes");
    assert_eq!(decoded, subset_face(regular(), text).expect("face subsets"));
}

#[test]
fn embedding_is_deterministic() {
    let runs = [Run {
        text: "Determinism",
        face: regular(),
    }];
    assert_eq!(
        embed_runs(&runs).expect("embeds"),
        embed_runs(&runs).expect("embeds")
    );
}

#[test]
fn the_family_name_of_a_known_subset_is_fixed() {
    // A golden value: the hash of this subset must not change between builds.
    let runs = [Run {
        text: "Hello",
        face: regular(),
    }];
    let name = &embed_runs(&runs).expect("embeds")[0].name;
    assert_eq!(name, GOLDEN_NAME);
}

const GOLDEN_NAME: &str = "fg-03d728be4d324b53-sans";

#[test]
fn no_runs_embed_nothing() {
    assert!(embed_runs(&[]).expect("embeds").is_empty());
}

#[test]
fn an_empty_label_still_embeds_its_face() {
    // A run with no characters uses its face, so the face is embedded with no glyphs of its own.
    let families = embed_runs(&[Run {
        text: "",
        face: regular(),
    }])
    .expect("embeds");
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].css.matches("@font-face").count(), 1);
    let empty = subset_face(regular(), "").expect("face subsets");
    assert!(empty.len() < regular().data().len() / 4);
}

#[test]
fn a_line_break_brings_the_space_glyph_along() {
    // SVG text draws a line break or a tab as a space, so the subset of a text with one needs the
    // space glyph even when the text has no space of its own (the sankey labels, "name\nvalue").
    for text in ["a\nb", "a\r\nb", "a\tb"] {
        let subset = subset_face(regular(), text).expect("face subsets");
        let with_subset = measure_data(&subset, "a b", 16.0).expect("measures");
        let with_full = measure(regular(), "a b", 16.0).expect("measures");
        assert_eq!(with_subset, with_full, "{text:?}");
    }
}
