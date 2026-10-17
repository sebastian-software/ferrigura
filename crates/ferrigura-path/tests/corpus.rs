//! The port against Chromium 151's own `getBBox()`, on the corpus of `tests/corpus/paths.json`, written by
//! `tools/port/path-bounds.ts`. The boxes are floats in the browser, so a box that differs is a difference
//! in the arithmetic, not in rounding.
//!
//! The port matches every box of the corpus except the ones that `tests/corpus/known-differences.json`
//! lists: arc paths whose boxes differ from Chromium's by one or two float units, where the arc's
//! parameters make Skia's scale factor (`sqrt(1/d - 1/4)`) sensitive to the last bit. The list is checked
//! exactly, so a new difference, or a difference that goes away, fails the test.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value;

fn corpus_file(name: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "corpus", name]
        .iter()
        .collect()
}

/// The cases of the corpus: each path's data and the four values Chromium returned.
fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(corpus_file("paths.json")).expect("paths.json");
    serde_json::from_str(&text).expect("paths.json parses")
}

#[test]
fn every_box_matches_chromium_bit_for_bit_but_the_known_differences() {
    let cases = cases();
    assert!(cases.len() >= 3000, "the corpus is the floor of the cases");
    let mut differing = BTreeSet::new();
    for case in &cases {
        let d = case["d"].as_str().expect("d");
        let expected = [
            case["x"].as_f64().expect("x") as f32,
            case["y"].as_f64().expect("y") as f32,
            case["width"].as_f64().expect("width") as f32,
            case["height"].as_f64().expect("height") as f32,
        ];
        let got = ferrigura_path::bounding_box(d);
        let actual = [got.x, got.y, got.width, got.height];
        let same = actual
            .iter()
            .zip(expected.iter())
            .all(|(a, e)| a.to_bits() == e.to_bits());
        if !same {
            differing.insert(d.to_owned());
        }
    }
    if std::env::var_os("FERRIGURA_PATH_WRITE_KNOWN").is_some() {
        let list: Vec<&String> = differing.iter().collect();
        std::fs::write(
            corpus_file("known-differences.json"),
            format!("{}\n", serde_json::to_string_pretty(&list).expect("json")),
        )
        .expect("write the known differences");
        return;
    }
    let known: BTreeSet<String> = serde_json::from_str::<Vec<String>>(
        &std::fs::read_to_string(corpus_file("known-differences.json"))
            .expect("known-differences.json"),
    )
    .expect("known-differences.json parses")
    .into_iter()
    .collect();
    let new: Vec<&String> = differing.difference(&known).collect();
    let fixed: Vec<&String> = known.difference(&differing).collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "{} boxes differ from Chromium, {} of them new:\n{}\nno longer differing: {}",
        differing.len(),
        new.len(),
        new.iter()
            .take(8)
            .map(|d| d.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        fixed.len()
    );
}
