//! Lines whose arrows fall back to the symbol face, against Chromium: the boxes, advances and wrapped
//! lines of the `fallback` and `fallbackWraps` sections of `fixtures/chromium.json`, where the
//! font-family list of each run is its own face and then `ft-symbols`.

mod support;

use base64::Engine;
use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, Run, embed_runs, face, fallback_runs, layout_width, measure,
    measure_box, measure_data, measure_line_box, text_box, wrap_line,
};
use serde_json::Value;
use support::{chromium, entries, face_for, number, size_of, style_of, text_of};

/// The largest difference allowed between a box or an advance here and in Chromium, as for the
/// `extents` section.
const TOLERANCE: f64 = 0.001;

/// The arrows block, which the symbol face covers and the text faces do not.
fn arrows() -> impl Iterator<Item = char> {
    (0x2190..=0x21FF).filter_map(char::from_u32)
}

fn symbols() -> &'static ferrigura_text::Face {
    face(FontFamily::Symbols, FontWeight::Normal, FontStyle::Normal)
}

fn regular() -> &'static ferrigura_text::Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

/// The runs of a fixture request, each in its face.
fn runs_of(entry: &Value) -> Vec<(String, &'static ferrigura_text::Face)> {
    entry["parts"]
        .as_array()
        .expect("request has parts")
        .iter()
        .map(|part| (text_of(part).to_owned(), face_for(style_of(part))))
        .collect()
}

/// The offset of the anchor point from the start of the line, for a line of advance `width`.
fn anchor_shift(anchor: &str, width: f64) -> f64 {
    match anchor {
        "start" => 0.0,
        "middle" => -width / 2.0,
        "end" => -width,
        other => panic!("unknown anchor {other}"),
    }
}

#[test]
fn the_symbol_face_covers_the_arrows_block_and_the_text_faces_do_not() {
    for character in arrows() {
        assert!(symbols().covers(character), "{character:?} is not covered");
        for text_face in [
            regular(),
            face(FontFamily::Sans, FontWeight::Bold, FontStyle::Italic),
            face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal),
        ] {
            assert!(!text_face.covers(character), "{character:?} is covered");
        }
    }
    assert!(regular().covers('a'));
    assert!(!symbols().covers('a'));
}

#[test]
fn a_run_is_split_where_the_text_face_lacks_an_arrow() {
    let text = "a→b↔c";
    let stretches = fallback_runs(Run {
        text,
        face: regular(),
    });
    let shape: Vec<(&str, bool)> = stretches
        .iter()
        .map(|stretch| (stretch.text, stretch.face.family() == FontFamily::Symbols))
        .collect();
    assert_eq!(
        shape,
        [
            ("a", false),
            ("→", true),
            ("b", false),
            ("↔", true),
            ("c", false)
        ]
    );
    let joined: String = stretches.iter().map(|stretch| stretch.text).collect();
    assert_eq!(joined, text, "the stretches cover the text in order");
}

#[test]
fn text_without_arrows_and_empty_text_keep_one_run() {
    for text in ["Hello", ""] {
        let stretches = fallback_runs(Run {
            text,
            face: regular(),
        });
        assert_eq!(stretches.len(), 1, "{text:?}");
        assert_eq!(stretches[0].text, text);
        assert_eq!(stretches[0].face.family(), FontFamily::Sans);
    }
    let arrows_only = fallback_runs(Run {
        text: "→→",
        face: regular(),
    });
    assert_eq!(arrows_only.len(), 1, "a run of arrows is one stretch");
    assert_eq!(arrows_only[0].face.family(), FontFamily::Symbols);
}

#[test]
fn an_arrow_has_the_advance_of_the_symbol_face() {
    for size in [12.0, 14.0, 18.5] {
        let through_fallback = measure(regular(), "→", size).expect("measures");
        let through_symbols = measure_data(symbols().data(), "→", size).expect("measures");
        assert_eq!(through_fallback, through_symbols, "{size} px");
    }
}

#[test]
fn lines_with_arrows_have_the_boxes_and_advances_chromium_gives_them() {
    let reference = chromium();
    let requests = entries(&reference, "fallback");
    let mut failures = Vec::new();
    let mut with_arrows = 0;
    for entry in &requests {
        let size = size_of(entry);
        let anchor = entry["anchor"].as_str().expect("request has an anchor");
        let runs = runs_of(entry);
        let borrowed: Vec<Run<'_>> = runs.iter().map(|(text, face)| Run { text, face }).collect();
        let label = format!(
            "{:?} at {size} px, {anchor}",
            runs.iter()
                .map(|(text, _)| text.as_str())
                .collect::<String>()
        );

        // The advance of the line: each run's advance, the fallback summed, snapped once per run.
        let mut advance = 0.0;
        for (text, face) in &runs {
            advance += layout_width(measure(face, text, size).expect("run measures"));
        }
        let extent = measure_line_box(&borrowed, size).expect("line measures");
        let x = anchor_shift(anchor, advance) + extent.left;
        let width = extent.width();

        let expected_x = number(entry, "x");
        let expected_width = number(entry, "width");
        let expected_length = number(entry, "computedLength");
        if (advance - expected_length).abs() > TOLERANCE {
            failures.push(format!("{label}: advance {advance} vs {expected_length}"));
        }
        if (x - expected_x).abs() > TOLERANCE || (width - expected_width).abs() > TOLERANCE {
            failures.push(format!(
                "{label}: box x {x} width {width} vs {expected_x} and {expected_width}"
            ));
        }
        // The vertical box is the text face's, as the arrows leave it alone (`text_box`).
        let vertical = text_box(runs[0].1, size).expect("box measures");
        let (ascent, descent) = (number(entry, "ascent"), number(entry, "descent"));
        if vertical.ascent != ascent || vertical.descent != descent {
            failures.push(format!(
                "{label}: box {}/{} vs {ascent}/{descent}",
                vertical.ascent, vertical.descent
            ));
        }
        if runs
            .iter()
            .any(|(text, _)| text.chars().any(|c| symbols().covers(c)))
        {
            with_arrows += 1;
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} lines differ from Chromium:\n{}",
        failures.len(),
        requests.len(),
        failures.join("\n")
    );
    assert!(
        with_arrows > 800,
        "only {with_arrows} of {} requests have an arrow",
        requests.len()
    );
}

#[test]
fn a_single_arrow_has_the_box_of_its_glyph_and_advance() {
    let reference = chromium();
    for entry in entries(&reference, "fallback") {
        let parts = entry["parts"].as_array().expect("request has parts");
        if parts.len() != 1 {
            continue;
        }
        let text = text_of(&parts[0]);
        let size = size_of(&entry);
        let found = measure_box(face_for(style_of(&parts[0])), text, size).expect("box measures");
        let expected_width = number(&entry, "width");
        assert!(
            (found.width() - expected_width).abs() <= TOLERANCE,
            "{text:?} {} at {size} px: width {} vs {expected_width}",
            style_of(&parts[0]),
            found.width()
        );
    }
}

#[test]
fn wrapped_lines_with_arrows_match_mermaid_in_chromium() {
    let reference = chromium();
    let wraps = entries(&reference, "fallbackWraps");
    assert_eq!(
        wraps.len(),
        24,
        "four strings in two styles at three widths"
    );
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
fn an_embedded_symbol_subset_names_both_families_and_draws_the_arrows() {
    let runs = [Run {
        text: "a→b",
        face: regular(),
    }];
    let families = embed_runs(&runs).expect("embeds");
    assert_eq!(families.len(), 2, "sans and symbols");
    let sans = &families[0];
    let symbol = &families[1];
    assert_eq!(sans.family, FontFamily::Sans);
    assert_eq!(symbol.family, FontFamily::Symbols);
    assert!(symbol.name.ends_with("-symbols"), "{}", symbol.name);
    assert_eq!(
        sans.css_family,
        format!("\"{}\", \"{}\"", sans.name, symbol.name)
    );
    assert_eq!(symbol.css_family, format!("\"{}\"", symbol.name));

    // The symbol subset holds the arrow and measures it as the bundled face does.
    let data = base64_of(&symbol.css);
    assert_eq!(
        measure_data(&data, "→", 14.0).expect("measures"),
        measure(symbols(), "→", 14.0).expect("measures")
    );
    assert!(data.len() < symbols().data().len(), "the subset is smaller");
    assert!(
        symbol
            .css
            .contains(&format!("@font-face{{font-family:\"{}\";", symbol.name)),
        "{}",
        symbol.css
    );
}

#[test]
fn text_without_arrows_embeds_no_symbol_family() {
    let runs = [Run {
        text: "Hello",
        face: regular(),
    }];
    let families = embed_runs(&runs).expect("embeds");
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].css_family, format!("\"{}\"", families[0].name));
}

/// The TrueType bytes of the first `@font-face` data URL of a CSS text.
fn base64_of(css: &str) -> Vec<u8> {
    let start = css
        .find("base64,")
        .map(|index| index + "base64,".len())
        .expect("the css has a data URL");
    let end = css[start..].find(')').expect("the data URL ends") + start;
    base64::engine::general_purpose::STANDARD
        .decode(&css[start..end])
        .expect("the data URL is base64")
}

#[test]
fn a_run_with_arrows_snaps_once_as_a_whole() {
    // "a→b" is one run, so its stretches add up unsnapped and the run snaps as a whole, which is
    // what Chromium gives (29.46875 px). Snapping each stretch would give 29.484375 px, as three
    // separate `tspan`s do.
    let text = "a→b";
    let whole = layout_width(measure(regular(), text, 14.0).expect("measures"));
    let stretches: f64 = fallback_runs(Run {
        text,
        face: regular(),
    })
    .iter()
    .map(|stretch| layout_width(measure(stretch.face, stretch.text, 14.0).expect("measures")))
    .sum();
    assert_eq!(whole, 29.46875);
    assert_eq!(stretches, 29.484375);
}
