//! The box `getBBox` gives a line of SVG text, against Chromium: the union of each run's advance
//! and each glyph's outline, from the `extents` section of `fixtures/chromium.json`.

mod support;

use ferrigura_text::{Run, layout_width, measure, measure_box, measure_line_box};
use serde_json::Value;
use support::{chromium, entries, face_for, number, size_of, style_of, text_of};

/// The largest difference allowed between a box or an advance here and in Chromium. Chromium
/// reports boxes as float32 values, so they agree to far better than a thousandth of a pixel.
const TOLERANCE: f64 = 0.001;

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
fn lines_have_the_boxes_and_advances_chromium_gives_them() {
    let reference = chromium();
    let requests = entries(&reference, "extents");
    let mut failures = Vec::new();
    let mut overhanging = 0;
    for entry in &requests {
        let size = size_of(entry);
        let anchor = entry["anchor"].as_str().expect("request has an anchor");
        let runs = runs_of(entry);
        let borrowed: Vec<Run<'_>> = runs.iter().map(|(text, face)| Run { text, face }).collect();

        // The advance of the line, the sum of the runs' advances snapped to the layout unit.
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
        let text = runs
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();
        let label = format!("{text:?} at {size} px, {anchor}");
        if (advance - expected_length).abs() > TOLERANCE {
            failures.push(format!("{label}: advance {advance} vs {expected_length}"));
        }
        if (x - expected_x).abs() > TOLERANCE || (width - expected_width).abs() > TOLERANCE {
            failures.push(format!(
                "{label}: box x {x} width {width} vs {expected_x} and {expected_width}"
            ));
        }
        // The advance-only box, which is what a box of the advances alone would give.
        if (expected_width - advance).abs() > 0.01 {
            overhanging += 1;
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} lines differ from Chromium:\n{}",
        failures.len(),
        requests.len(),
        failures.join("\n")
    );
    // The requests must include lines whose ink reaches past their advance, or the test proves
    // nothing about the ink.
    assert!(
        overhanging > 300,
        "only {overhanging} of {} requests have a box wider than their advance",
        requests.len()
    );
}

#[test]
fn a_single_run_has_the_box_of_its_glyphs_and_advance() {
    let reference = chromium();
    for entry in entries(&reference, "extents") {
        let parts = entry["parts"].as_array().expect("request has parts");
        if parts.len() != 1 {
            continue;
        }
        let text = text_of(&parts[0]);
        let face = face_for(style_of(&parts[0]));
        let size = size_of(&entry);
        let found = measure_box(face, text, size).expect("box measures");
        let expected_width = number(&entry, "width");
        assert!(
            (found.width() - expected_width).abs() <= TOLERANCE,
            "{text:?} {} at {size} px: width {} vs {expected_width}",
            style_of(&parts[0]),
            found.width()
        );
    }
}
