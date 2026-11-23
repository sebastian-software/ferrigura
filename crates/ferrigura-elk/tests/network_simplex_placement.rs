//! The network simplex node placement against elkjs: every corpus case named `synthetic__ns-…` lays
//! out, and its node boxes and root size equal elkjs's exactly.
//!
//! The cases derive from other corpus cases with `elk.layered.nodePlacement.strategy` set to
//! `NETWORK_SIMPLEX`: at the root only (`ns-root`, what Mermaid's `modelOrder` and `depthFirst`
//! presets give), in every graph (`ns-all`), and with
//! `elk.layered.nodePlacement.networkSimplex.nodeFlexibility.default` set to `PORT_POSITION`
//! (`ns-ports`), `NODE_SIZE` (`ns-size`) or `NODE_SIZE_WHERE_SPACE_PERMITS` (`ns-where`), or with
//! `elk.layered.nodePlacement.networkSimplex.nodeFlexibility` set to `NODE_SIZE` on every second node
//! without children (`ns-node`). `FERRIGURA_NS_FILTER` narrows the run to the cases whose name
//! contains its value, and `FERRIGURA_NS_CASES` names another directory of case files to compare in
//! place of the corpus, as the random sets have theirs.

use std::path::PathBuf;

mod support;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;
use support::largest_difference;

fn corpus_dir() -> PathBuf {
    match std::env::var_os("FERRIGURA_NS_CASES") {
        Some(directory) => PathBuf::from(directory),
        None => [env!("CARGO_MANIFEST_DIR"), "tests", "corpus"]
            .iter()
            .collect(),
    }
}

/// The cases to compare: the corpus's network simplex cases by its manifest, or every case file of
/// the directory `FERRIGURA_NS_CASES` names.
fn case_names() -> Vec<String> {
    let filter = std::env::var("FERRIGURA_NS_FILTER").unwrap_or_default();
    let mut names: Vec<String> = if std::env::var_os("FERRIGURA_NS_CASES").is_some() {
        std::fs::read_dir(corpus_dir())
            .expect("the case directory")
            .filter_map(|entry| {
                let path = entry.expect("a directory entry").path();
                (path
                    .extension()
                    .is_some_and(|extension| extension == "json"))
                .then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
            })
            .collect()
    } else {
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
    };
    names.retain(|name| name.starts_with("synthetic__ns-") && name.contains(&filter));
    names.sort();
    names
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
                Ok(difference) => {
                    if difference != 0.0 {
                        failures.push(format!("{name}: differs by {difference}"));
                    }
                }
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
