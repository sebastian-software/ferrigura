//! Lines made of runs in different faces: each run is measured in its own face and the widths are
//! summed, and no kerning crosses a change of face.

mod support;

use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, face, measure, measure_line};
use support::{
    CHROMIUM_TOLERANCE, chromium, entries, face_for, number, size_of, style_of, text_of,
};

#[test]
fn mixed_runs_agree_with_chromium() {
    // Chromium rounds the advance of each text fragment up to the next 1/64 px, so each run in a
    // line can add up to 1/64 px to the reference. The tolerance is therefore per run.
    let reference = chromium();
    let lines = entries(&reference, "runs");
    assert!(!lines.is_empty());
    for line in &lines {
        let runs: Vec<(&str, &str)> = line["runs"]
            .as_array()
            .expect("line has runs")
            .iter()
            .map(|run| (text_of(run), style_of(run)))
            .collect();
        let faces: Vec<_> = runs.iter().map(|(_, style)| face_for(style)).collect();
        let parts: Vec<Run<'_>> = runs
            .iter()
            .zip(&faces)
            .map(|((text, _), face)| Run { text, face })
            .collect();
        let size = size_of(line);
        let actual = measure_line(&parts, size).expect("line measures");
        let expected = number(line, "width");
        let tolerance = CHROMIUM_TOLERANCE * runs.len() as f64;
        assert!(
            (actual - expected).abs() <= tolerance,
            "{runs:?} at {size}px: {actual} vs Chromium {expected}"
        );
    }
}

#[test]
fn a_line_is_the_sum_of_its_runs() {
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let bold = face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal);
    let line = [
        Run {
            text: "Type ",
            face: regular,
        },
        Run {
            text: "WAVE",
            face: bold,
        },
    ];
    let sum = measure(regular, "Type ", 16.0).expect("measures")
        + measure(bold, "WAVE", 16.0).expect("measures");
    assert_eq!(measure_line(&line, 16.0).expect("line measures"), sum);
}

#[test]
fn no_kerning_crosses_a_change_of_face() {
    // In one face the pair A V is kerned, so the word is narrower than its letters. Split into two
    // faces, the pair is measured apart, as Chromium draws it.
    let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let bold = face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal);
    let kerned = measure(regular, "AV", 16.0).expect("measures");
    let apart = measure(regular, "A", 16.0).expect("measures")
        + measure(regular, "V", 16.0).expect("measures");
    assert!(kerned < apart, "AV kerned {kerned} px, apart {apart} px");

    let across = measure_line(
        &[
            Run {
                text: "A",
                face: regular,
            },
            Run {
                text: "V",
                face: bold,
            },
        ],
        16.0,
    )
    .expect("line measures");
    let letters = measure(regular, "A", 16.0).expect("measures")
        + measure(bold, "V", 16.0).expect("measures");
    assert_eq!(across, letters);
}

#[test]
fn an_empty_line_is_zero_wide() {
    assert_eq!(measure_line(&[], 16.0).expect("empty line measures"), 0.0);
}
