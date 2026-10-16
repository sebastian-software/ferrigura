//! Differential run over the corpus: the port's coordinates against elkjs's.
//!
//! Each case in `tests/corpus` holds the ELK JSON graph Mermaid gave elkjs and
//! elkjs's result. `corpus_summary` lists, per case, whether the port lays it
//! out and why not; `coordinates_match_elkjs` compares the node boxes and the
//! root size with elkjs's, and lists the cases that differ.

use std::path::PathBuf;

mod support;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;
use support::largest_difference;

/// Coordinates must agree exactly: the port reproduces elkjs's arithmetic, so a
/// difference is a bug, not rounding. A tolerance needs a documented cause here.
const TOLERANCE: f64 = 0.0;

fn corpus_dir() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "corpus"]
        .iter()
        .collect()
}

fn case_names() -> Vec<String> {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(corpus_dir().join("manifest.json")).expect("manifest.json"),
    )
    .unwrap();
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["name"].as_str().unwrap().to_string())
        .collect()
}

fn load(name: &str) -> Value {
    let text = std::fs::read_to_string(corpus_dir().join(format!("{name}.json")))
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    serde_json::from_str(&text).unwrap()
}

#[test]
#[ignore = "port: prints the coverage of the corpus, run with --nocapture"]
fn corpus_summary() {
    let (mut supported, mut total) = (0, 0);
    for name in case_names() {
        total += 1;
        let case = load(&name);
        let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
        match ferrigura_elk::layout(&input, false) {
            Ok(_) => {
                supported += 1;
                println!("driver ok    {name}");
            }
            Err(error) => println!("unsupported  {name}: {error}"),
        }
    }
    println!(
        "{supported} of {total} corpus cases pass the driver (coordinates are not computed yet)"
    );
}

/// Cases whose coordinates match elkjs. The floor only goes up, as `SUPPORTED_FLOOR` does.
const MATCHING_FLOOR: usize = 160;

#[test]
fn coordinates_match_elkjs() {
    let mut matching = Vec::new();
    let mut differing = Vec::new();
    for name in case_names() {
        let case = load(&name);
        let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
        let Ok(layout) = ferrigura_elk::layout(&input, false) else {
            continue;
        };
        match largest_difference(&layout.graph, &case["output"]) {
            Ok(difference) if difference <= TOLERANCE => matching.push(name),
            Ok(difference) => differing.push(format!("{name}: differs by {difference}")),
            Err(reason) => differing.push(format!("{name}: {reason}")),
        }
    }
    for line in &differing {
        println!("differs  {line}");
    }
    println!(
        "{} cases match elkjs, {} differ",
        matching.len(),
        differing.len()
    );
    assert!(
        matching.len() >= MATCHING_FLOOR,
        "{} cases match elkjs; the floor is {MATCHING_FLOOR}",
        matching.len()
    );
}

/// Cases that pass the driver today. The floor only goes up: a change that makes a
/// case unsupported again fails here, and so does CI. Raise it as the port grows.
const SUPPORTED_FLOOR: usize = 160;

#[test]
fn supported_cases_do_not_regress() {
    let supported = case_names()
        .into_iter()
        .filter(|name| {
            let case = load(name);
            let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
            ferrigura_elk::layout(&input, false).is_ok()
        })
        .count();
    assert!(
        supported >= SUPPORTED_FLOOR,
        "{supported} corpus cases pass the driver; the floor is {SUPPORTED_FLOOR}"
    );
}
