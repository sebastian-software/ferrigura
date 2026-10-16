//! The port against elkjs on random flat graphs: the coordinates, and the trace with its logs.
//!
//! The graphs come from `tools/random-graphs.ts`, which lays them out with Node elkjs and writes one
//! case per graph. This test is ignored by default, because it needs that run first; see the tool for
//! the command. It lays out every graph the run wrote and fails on any that differs.

mod support;

use std::path::PathBuf;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;
use support::largest_difference;

/// The directory the tool wrote to: `FERRIGURA_RANDOM_GRAPHS`, else the workspace's `target/`.
fn graph_dir() -> PathBuf {
    match std::env::var_os("FERRIGURA_RANDOM_GRAPHS") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("ferrigura-random-graphs"),
    }
}

#[test]
#[ignore = "needs the graphs of tools/random-graphs.ts; run with --ignored --nocapture"]
fn random_graphs_match_elkjs() {
    let dir = graph_dir();
    let names_path = dir.join("names.json");
    let names: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&names_path)
            .unwrap_or_else(|error| panic!("{}: {error}", names_path.display())),
    )
    .expect("names.json lists the graphs");
    let (mut matching, mut mismatched, mut unsupported) = (0, Vec::new(), Vec::new());
    let mut traces_equal = 0;
    for name in &names {
        let text = std::fs::read_to_string(dir.join(format!("{name}.json")))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let case: Value = serde_json::from_str(&text).expect("case file parses");
        let input: JsonNode = serde_json::from_value(case["input"].clone()).expect("input");
        let layout = match ferrigura_elk::layout(&input, true) {
            Ok(layout) => layout,
            Err(error) => {
                unsupported.push(format!("{name}: {error}"));
                continue;
            }
        };
        let same_trace = layout.trace.to_json() == case["trace"];
        traces_equal += usize::from(same_trace);
        match largest_difference(&layout.graph, &case["output"]) {
            Ok(difference) if difference <= 0.0 && same_trace => matching += 1,
            Ok(difference) => mismatched.push(format!(
                "{name}: coordinates differ by {difference}, trace equal: {same_trace}"
            )),
            Err(reason) => mismatched.push(format!("{name}: {reason}")),
        }
    }
    for line in mismatched.iter().chain(&unsupported).take(30) {
        println!("differs  {line}");
    }
    println!(
        "{matching} of {} random graphs match elkjs in coordinates and in the trace with logs \
         ({traces_equal} traces equal, {} unsupported, {} differ)",
        names.len(),
        unsupported.len(),
        mismatched.len(),
    );
    assert!(
        mismatched.is_empty() && unsupported.is_empty(),
        "{} graphs differ and {} are unsupported",
        mismatched.len(),
        unsupported.len()
    );
}
