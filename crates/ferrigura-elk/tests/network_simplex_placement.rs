//! The network simplex node placement against elkjs: every corpus case named `synthetic__ns-…` lays
//! out, and its node boxes and root size equal elkjs's exactly.
//!
//! The cases derive from other corpus cases with `elk.layered.nodePlacement.strategy` set to
//! `NETWORK_SIMPLEX`: at the root only (`ns-root`, what Mermaid's `modelOrder` and `depthFirst`
//! presets give), in every graph (`ns-all`), and with
//! `elk.layered.nodePlacement.networkSimplex.nodeFlexibility.default` set to `PORT_POSITION`
//! (`ns-ports`) or `NODE_SIZE` (`ns-size`). `FERRIGURA_NS_FILTER` narrows the run to the cases whose
//! name contains its value.

use std::path::PathBuf;

mod support;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;
use support::largest_difference;

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
    let filter = std::env::var("FERRIGURA_NS_FILTER").unwrap_or_default();
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["name"].as_str().unwrap().to_string())
        .filter(|name| name.starts_with("synthetic__ns-") && name.contains(&filter))
        .collect()
}

#[test]
fn network_simplex_cases_match_elkjs() {
    let names = case_names();
    assert!(!names.is_empty(), "no network simplex cases in the corpus");
    let mut failures = Vec::new();
    for name in &names {
        let text = std::fs::read_to_string(corpus_dir().join(format!("{name}.json")))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let case: Value = serde_json::from_str(&text).unwrap();
        let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
        match ferrigura_elk::layout(&input, false) {
            Err(error) => failures.push(format!("{name}: does not lay out: {error}")),
            Ok(layout) => match largest_difference(&layout.graph, &case["output"]) {
                Ok(difference) if difference == 0.0 => {}
                Ok(difference) => failures.push(format!("{name}: differs by {difference}")),
                Err(reason) => failures.push(format!("{name}: {reason}")),
            },
        }
    }
    for failure in &failures {
        println!("{failure}");
    }
    assert!(
        failures.is_empty(),
        "{} of {} network simplex cases do not match elkjs",
        failures.len(),
        names.len()
    );
}
