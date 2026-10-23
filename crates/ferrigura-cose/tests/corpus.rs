//! Differential run over the corpus: the port's node centres against cytoscape-cose-bilkent's, as
//! the Mermaid bundle computes them in Chromium.
//!
//! Each case in `tests/corpus` holds the input Mermaid's integration handed the engine (the node
//! sizes it measured, and the edges in order) and the centres the engine returned. The corpus is
//! written by `tools/capture-corpus.ts` of the tag `fork-mermaid-12.2.0`, which ran against the fork; see
//! `NOTICE.md` for the sources.
//!
//! The edge endpoints in the corpus (cytoscape's `rscratch`) are not compared here: they are the
//! mindmap renderer's to produce, not this crate's (see the crate docs).

use std::collections::BTreeMap;
use std::path::PathBuf;

use ferrigura_cose::{Edge, Node, layout};
use serde_json::Value;

/// Centres must agree exactly: the port reproduces the engine's arithmetic, so a difference is a
/// bug, not rounding. A JSON number reads as the double it was written from (`float_roundtrip`),
/// but `JSON.stringify` writes -0 as 0, so the comparison is `==` (which takes -0 as 0).
const TOLERANCE: f64 = 0.0;

/// Cases the differential must cover. The floor only goes up.
const CASE_FLOOR: usize = 328;

fn corpus_dir() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "corpus"]
        .iter()
        .collect()
}

fn load(name: &str) -> Value {
    let text = std::fs::read_to_string(corpus_dir().join(format!("{name}.json")))
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn case_names() -> Vec<String> {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(corpus_dir().join("manifest.json")).expect("manifest.json"),
    )
    .expect("manifest.json parses");
    manifest["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| case["name"].as_str().expect("name").to_string())
        .collect()
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

/// The input of a case, as the engine takes it.
fn input(case: &Value) -> (Vec<Node>, Vec<Edge>) {
    let nodes = case["input"]["nodes"]
        .as_array()
        .expect("input nodes")
        .iter()
        .map(|node| Node {
            id: node["id"].as_str().expect("id").to_string(),
            width: number(&node["width"]),
            height: number(&node["height"]),
        })
        .collect();
    let edges = case["input"]["edges"]
        .as_array()
        .expect("input edges")
        .iter()
        .map(|edge| Edge {
            id: edge["id"].as_str().expect("id").to_string(),
            source: edge["source"].as_str().expect("source").to_string(),
            target: edge["target"].as_str().expect("target").to_string(),
        })
        .collect();
    (nodes, edges)
}

/// The largest difference between the port's centres and the reference's, or the reason they do
/// not line up (a different node list, or an error from the engine).
fn difference(case: &Value) -> Result<f64, String> {
    let (nodes, edges) = input(case);
    let positions = layout(&nodes, &edges).map_err(|error| format!("engine: {error}"))?;
    let expected = case["output"]["positions"]
        .as_array()
        .expect("output positions");
    if expected.len() != positions.len() {
        return Err(format!(
            "{} nodes, the reference has {}",
            positions.len(),
            expected.len()
        ));
    }
    let mut largest = 0.0_f64;
    for (position, reference) in positions.iter().zip(expected) {
        if reference["id"].as_str() != Some(position.id.as_str()) {
            return Err(format!("node order differs at {}", position.id));
        }
        for (ours, theirs) in [
            (position.x, number(&reference["x"])),
            (position.y, number(&reference["y"])),
        ] {
            if ours != theirs {
                largest = largest.max((ours - theirs).abs());
            }
        }
    }
    Ok(largest)
}

/// The group of a case, and whether its centres agree, for the summary.
fn agreement() -> BTreeMap<String, (usize, usize)> {
    let mut by_group: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for name in case_names() {
        let case = load(&name);
        let group = case["group"].as_str().expect("group").to_string();
        let entry = by_group.entry(group).or_insert((0, 0));
        entry.1 += 1;
        if matches!(difference(&case), Ok(d) if d <= TOLERANCE) {
            entry.0 += 1;
        }
    }
    by_group
}

#[test]
fn centres_match_the_reference_exactly() {
    let mut matching = 0;
    let mut differing = Vec::new();
    let names = case_names();
    for name in &names {
        let case = load(name);
        match difference(&case) {
            Ok(d) if d <= TOLERANCE => matching += 1,
            Ok(d) => differing.push(format!("{name}: differs by {d}")),
            Err(reason) => differing.push(format!("{name}: {reason}")),
        }
    }
    for line in &differing {
        println!("differs  {line}");
    }
    for (group, (ok, total)) in agreement() {
        println!("{group}: {ok} of {total} cases agree");
    }
    println!("{matching} of {} cases match the reference", names.len());
    assert!(
        names.len() >= CASE_FLOOR,
        "the corpus has {} cases",
        names.len()
    );
    assert!(differing.is_empty(), "{} cases differ", differing.len());
}
