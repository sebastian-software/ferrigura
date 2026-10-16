//! The port against elkjs on random compound graphs: nested compound nodes, edges across the levels,
//! and the trace with its logs.
//!
//! The graphs come from `tools/random-compound-graphs.ts`, which lays them out with Node elkjs and
//! writes one case per graph. This test is ignored by default, because it needs that run first; see
//! the tool for the command. It lays out every graph the run wrote. It fails on any graph the port
//! lays out differently from elkjs, and on a pass count below the floor. A graph the port does not
//! lay out yet is counted under its message, so the remaining gaps are visible by name.

mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;
use support::largest_difference;

/// The graphs that match elkjs in coordinates and in the trace with logs, for the graphs the tool
/// wrote with its default seed. The floor only goes up.
const PASS_FLOOR: usize = 537;

/// The directory the tool wrote to: `FERRIGURA_COMPOUND_GRAPHS`, else the workspace's `target/`.
fn graph_dir() -> PathBuf {
    match std::env::var_os("FERRIGURA_COMPOUND_GRAPHS") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("ferrigura-compound-graphs"),
    }
}

/// A message with its node names replaced, so that the same gap counts once: `an inverted port on
/// g3 (...)` and `an inverted port on g7 (...)` are one entry.
fn gap_of(message: &str) -> String {
    message
        .split(' ')
        .map(|word| {
            if word.chars().any(|character| character.is_ascii_digit()) {
                "#".to_string()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
#[ignore = "needs the graphs of tools/random-compound-graphs.ts; run with --ignored --show-output"]
fn random_compound_graphs_match_elkjs() {
    let dir = graph_dir();
    let names_path = dir.join("names.json");
    let names: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&names_path)
            .unwrap_or_else(|error| panic!("{}: {error}", names_path.display())),
    )
    .expect("names.json lists the graphs");
    let mut passing = 0;
    let mut mismatched = Vec::new();
    let mut gaps: BTreeMap<String, usize> = BTreeMap::new();
    for name in &names {
        let text = std::fs::read_to_string(dir.join(format!("{name}.json")))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let case: Value = serde_json::from_str(&text).expect("case file parses");
        let input: JsonNode = serde_json::from_value(case["input"].clone()).expect("input");
        let layout = match ferrigura_elk::layout(&input, true) {
            Ok(layout) => layout,
            Err(error) => {
                *gaps.entry(gap_of(&error.to_string())).or_default() += 1;
                continue;
            }
        };
        let same_trace = layout.trace.to_json() == case["trace"];
        match largest_difference(&layout.graph, &case["output"]) {
            Ok(difference) if difference <= 0.0 && same_trace => passing += 1,
            Ok(difference) => mismatched.push(format!(
                "{name}: coordinates differ by {difference}, trace equal: {same_trace}"
            )),
            Err(reason) => mismatched.push(format!("{name}: {reason}")),
        }
    }
    let unsupported: usize = gaps.values().sum();
    println!(
        "{passing} of {} compound graphs match elkjs in coordinates and in the trace with logs; \
         {unsupported} are unsupported, {} differ",
        names.len(),
        mismatched.len(),
    );
    for (gap, count) in &gaps {
        println!("unsupported  {count:4}  {gap}");
    }
    for line in mismatched.iter().take(30) {
        println!("differs  {line}");
    }
    assert!(
        mismatched.is_empty(),
        "{} compound graphs differ from elkjs",
        mismatched.len()
    );
    assert!(
        passing >= PASS_FLOOR,
        "{passing} compound graphs match elkjs; the floor is {PASS_FLOOR}"
    );
}
