//! The labels against the reference: `label::layout` compared with `createText` (`htmlLabels: false`)
//! run in Chromium, from `fixtures/labels-chromium.json`, which `tools/port/text/label-fixture.ts`
//! writes. For every label the lines, their words and their styles, the start of every word, the
//! width of every line, the box of the text and the background must agree within 0.001 px.

use ferrigura_model::flowchart::LabelType;
use ferrigura_render::RenderError;
use ferrigura_render::unified::label::{Anchor, LabelLayout, LabelSpec, layout};
use ferrigura_text::TextError;
use serde_json::Value;

/// The largest difference allowed in pixels. Widths and starts are on Chromium's 1/64 px grid and
/// the boxes add the glyph outlines past the advances (`ferrigura_text::measure_box`), which Chromium
/// reads as float32 values, so the tolerance is a thousandth of a pixel.
const TOLERANCE: f64 = 0.001;

const FIXTURE: &str = include_str!("fixtures/labels-chromium.json");

#[test]
fn labels_match_the_reference_in_chromium() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("the fixture is valid JSON");
    let labels = fixture["labels"]
        .as_array()
        .expect("the fixture has labels");
    let mut deviation = Deviation::default();
    let mut failures = Vec::new();
    for label in labels {
        if let Err(message) = check(label, &mut deviation) {
            failures.push(message);
        }
    }
    eprintln!(
        "labels against Chromium {}: {} labels, largest deviation {:.4} px ({})",
        fixture["chromium"].as_str().unwrap_or("?"),
        labels.len(),
        deviation.largest,
        deviation.at,
    );
    assert!(
        failures.is_empty(),
        "{} of {} labels differ from the reference:\n{}",
        failures.len(),
        labels.len(),
        failures.join("\n")
    );
}

/// The largest difference seen, and where it is.
#[derive(Default)]
struct Deviation {
    largest: f64,
    at: String,
}

impl Deviation {
    /// Records `difference` if it is the largest so far, and reports whether it is within tolerance.
    fn near(&mut self, actual: f64, expected: f64, what: impl FnOnce() -> String) -> bool {
        let difference = (actual - expected).abs();
        if difference > self.largest {
            self.largest = difference;
            self.at = what();
        }
        difference <= TOLERANCE
    }
}

/// Lays out one fixture label and compares it with the reference's measurements.
fn check(label: &Value, deviation: &mut Deviation) -> Result<(), String> {
    let id = label["id"].as_str().unwrap_or_default();
    let text = label["text"].as_str().unwrap_or_default();
    let label_type = match label["labelType"].as_str() {
        Some("markdown") => LabelType::Markdown,
        Some("string") => LabelType::String,
        _ => LabelType::Text,
    };
    let is_node = label["isNode"].as_bool().unwrap_or(true);
    // An edge label has no width of its own; the reference's `createText` then wraps at 200.
    let width = label["width"].as_f64().unwrap_or(200.0);
    let styles: Vec<String> = label["style"]
        .as_str()
        .unwrap_or_default()
        .split(';')
        .filter(|declaration| !declaration.trim().is_empty())
        .map(str::to_owned)
        .collect();
    let anchor = match label["anchor"].as_str() {
        Some("start") => Anchor::Start,
        _ => Anchor::Middle,
    };
    let spec = LabelSpec {
        text,
        label_type,
        width,
        is_node,
        anchor,
        // The rule the fixtures were measured under: a centred node label wraps at its style's size.
        wrap_in_style: is_node && anchor == Anchor::Middle,
        add_background: label["addBackground"].as_bool().unwrap_or(false),
        styles: &styles,
        font_size: label["fontSize"].as_f64().unwrap_or(16.0),
        title: false,
    };

    let result = layout(&spec);
    if let Some(error) = label.get("error").and_then(Value::as_str) {
        return match result {
            Err(RenderError::Text(TextError::NewlineInLine)) => Ok(()),
            Err(message) => Err(format!("{id}: {message}, the reference throws {error}")),
            Ok(_) => Err(format!("{id}: laid out, but the reference throws {error}")),
        };
    }
    let layout = result.map_err(|error| format!("{id}: {error}"))?;
    compare(id, label, &layout, deviation)
}

/// The comparison of a laid-out label with the fixture's record of the reference.
fn compare(
    id: &str,
    label: &Value,
    layout: &LabelLayout,
    deviation: &mut Deviation,
) -> Result<(), String> {
    let mut problems = Vec::new();
    let reference_lines = label["lines"].as_array().map_or(&[][..], Vec::as_slice);
    if reference_lines.len() != layout.lines.len() {
        return Err(format!(
            "{id}: {} lines, the reference has {}",
            layout.lines.len(),
            reference_lines.len()
        ));
    }
    let font_size = layout.font_size;
    let text_lengths = label["lineTextLengths"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);

    for (row, (line, reference)) in layout.lines.iter().zip(reference_lines).enumerate() {
        let words = reference.as_array().map_or(&[][..], Vec::as_slice);
        if words.len() != line.runs.len() {
            problems.push(format!(
                "{id} line {row}: {} words, the reference has {}",
                line.runs.len(),
                words.len()
            ));
            continue;
        }
        for (column, (run, word)) in line.runs.iter().zip(words).enumerate() {
            let where_ = format!("{id} line {row} word {column}");
            if run.text != word["text"].as_str().unwrap_or_default() {
                problems.push(format!(
                    "{where_}: {:?}, the reference has {:?}",
                    run.text,
                    word["text"].as_str().unwrap_or_default()
                ));
            }
            let weight = f64::from(run.weight.css_value());
            if weight != number(word, "weight") {
                problems.push(format!("{where_}: weight {weight}"));
            }
            if run.style.css_keyword() != word["style"].as_str().unwrap_or_default() {
                problems.push(format!("{where_}: style {}", run.style.css_keyword()));
            }
            if !deviation.near(font_size, number(word, "fontSize"), || {
                format!("{where_}: font size")
            }) {
                problems.push(format!("{where_}: font size {}", number(word, "fontSize")));
            }
            if !deviation.near(run.x, number(word, "x"), || format!("{where_}: x")) {
                problems.push(format!("{where_}: x {} vs {}", run.x, number(word, "x")));
            }
            if !deviation.near(line.baseline, number(word, "y"), || format!("{where_}: y")) {
                problems.push(format!(
                    "{where_}: y {} vs {}",
                    line.baseline,
                    number(word, "y")
                ));
            }
        }
        let reference_width = text_lengths
            .get(row)
            .and_then(Value::as_f64)
            .unwrap_or_default();
        if !deviation.near(line.width, reference_width, || {
            format!("{id} line {row}: width")
        }) {
            problems.push(format!(
                "{id} line {row}: width {} vs {reference_width}",
                line.width
            ));
        }
    }

    let bbox = [
        ("x", layout.bbox.x),
        ("y", layout.bbox.y),
        ("width", layout.bbox.width),
        ("height", layout.bbox.height),
    ];
    for (key, actual) in bbox {
        let expected = number(&label["bbox"], key);
        if !deviation.near(actual, expected, || format!("{id}: box {key}")) {
            problems.push(format!("{id}: box {key} {actual} vs {expected}"));
        }
    }

    match (
        &layout.background,
        label.get("background").filter(|value| !value.is_null()),
    ) {
        (None, None) => {}
        (Some(actual), Some(reference)) => {
            for (key, value) in [
                ("x", actual.x),
                ("y", actual.y),
                ("width", actual.width),
                ("height", actual.height),
            ] {
                let expected = number(reference, key);
                if !deviation.near(value, expected, || format!("{id}: background {key}")) {
                    problems.push(format!("{id}: background {key} {value} vs {expected}"));
                }
            }
        }
        (Some(_), None) => problems.push(format!("{id}: a background, the reference has none")),
        (None, Some(_)) => problems.push(format!("{id}: no background, the reference has one")),
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n  "))
    }
}

/// A number field of a fixture record. A missing number is an error in the fixture, so it is NaN
/// here and every comparison with it fails.
fn number(record: &Value, key: &str) -> f64 {
    record[key].as_f64().unwrap_or(f64::NAN)
}
