//! Differential run over the fcose corpus: each run of the port against the centres that
//! fcose (cytoscape-fcose 2.2.0 on cose-base 2.2.0 and layout-base 2.0.1) wrote in Chromium, as
//! the Mermaid bundle runs it for architecture diagrams.
//!
//! A case in `tests/fcose-corpus` is one render: its `runs` are the two runs of fcose (run 1 from
//! the input positions, run 2 from run 1's output), each with the node and edge lists in
//! cytoscape's order, the options, the seed, the constraints, the component bounds and the
//! positions the run wrote. The corpus is written by `tools/capture-fcose-corpus.ts` of the tag
//! `fork-mermaid-12.2.0`, which ran against the fork; see `NOTICE.md` for the sources.
//!
//! Only the leaves (nodes without children) are compared: fcose writes their positions, and
//! cytoscape places the compounds from their children. Every compared centre must agree exactly
//! (tolerance 0). The node order must agree too.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use ferrigura_cose::fcose::{Alignment, Bounds, Edge, Input, Node, Options, Relative, layout};
use serde_json::Value;

/// Cases the differential must cover. The floor only goes up.
const CASE_FLOOR: usize = 419;

fn corpus_dir() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fcose-corpus"]
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

fn text(value: &Value) -> String {
    value.as_str().expect("a string").to_string()
}

fn strings(value: &Value) -> Vec<Vec<String>> {
    value
        .as_array()
        .map(|lists| {
            lists
                .iter()
                .map(|list| list.as_array().expect("a list").iter().map(text).collect())
                .collect()
        })
        .unwrap_or_default()
}

/// The input of one run, as the engine takes it.
fn run_input(run: &Value) -> Input {
    let nodes: Vec<Node> = run["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|node| Node {
            id: text(&node["id"]),
            parent: node["parent"].as_str().map(str::to_string),
            width: number(&node["width"]),
            height: number(&node["height"]),
            padding: number(&node["padding"]),
            x: number(&node["x"]),
            y: number(&node["y"]),
        })
        .collect();
    let edges = run["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| Edge {
            id: text(&edge["id"]),
            source: text(&edge["source"]),
            target: text(&edge["target"]),
            ideal_length: number(&edge["idealLength"]),
            elasticity: number(&edge["elasticity"]),
        })
        .collect();
    let alignment = match &run["alignmentConstraint"] {
        Value::Object(object) => Some(Alignment {
            horizontal: object.get("horizontal").map(strings).unwrap_or_default(),
            vertical: object.get("vertical").map(strings).unwrap_or_default(),
        }),
        _ => None,
    };
    let relative = run["relativePlacementConstraint"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|r| Relative {
                    left: r["left"].as_str().map(str::to_string),
                    right: r["right"].as_str().map(str::to_string),
                    top: r["top"].as_str().map(str::to_string),
                    bottom: r["bottom"].as_str().map(str::to_string),
                    gap: r["gap"].as_f64(),
                })
                .collect()
        })
        .unwrap_or_default();
    let bounds = &run["bounds"];
    Input {
        nodes,
        edges,
        alignment,
        relative,
        bounds: Bounds {
            x1: number(&bounds["x1"]),
            y1: number(&bounds["y1"]),
            w: number(&bounds["w"]),
            h: number(&bounds["h"]),
        },
        seed: run["seed"].as_u64().expect("seed") as u32,
        options: Options {
            randomize: run["options"]["randomize"].as_bool().expect("randomize"),
            node_separation: number(&run["options"]["nodeSeparation"]),
            num_iter: number(&run["options"]["numIter"]),
        },
    }
}

/// The largest difference over the leaves of one run, or the reason they do not line up.
fn run_difference(run: &Value) -> Result<f64, String> {
    let input = run_input(run);
    let positions = layout(&input).map_err(|error| format!("engine: {error}"))?;
    let expected = run["output"].as_array().expect("output");
    if expected.len() != input.nodes.len() {
        return Err(format!(
            "{} output entries for {} nodes",
            expected.len(),
            input.nodes.len()
        ));
    }
    let parents: HashSet<&str> = input
        .nodes
        .iter()
        .filter_map(|node| node.parent.as_deref())
        .collect();
    let mut largest = 0.0_f64;
    for ((node, position), reference) in input.nodes.iter().zip(&positions).zip(expected) {
        if text(&reference["id"]) != node.id {
            return Err(format!("node order differs at {}", node.id));
        }
        if parents.contains(node.id.as_str()) {
            continue;
        }
        for (ours, theirs) in [
            (position.x, number(&reference["x"])),
            (position.y, number(&reference["y"])),
        ] {
            if ours != theirs {
                // A NaN gap is kept, so a NaN cannot hide a difference.
                let gap = (ours - theirs).abs();
                if gap.is_nan() || gap > largest {
                    largest = gap;
                }
            }
        }
    }
    Ok(largest)
}

/// The differences of every run of a case: one entry per run, `Ok(0.0)` when it agrees.
fn case_difference(case: &Value) -> Vec<Result<f64, String>> {
    case["runs"]
        .as_array()
        .expect("runs")
        .iter()
        .map(run_difference)
        .collect()
}

#[test]
fn leaf_centres_match_the_reference_exactly() {
    let names = case_names();
    let mut cases_ok = 0;
    let mut runs_total = 0;
    let mut differing = Vec::new();
    let mut by_group: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for name in &names {
        let case = load(name);
        let group = text(&case["group"]);
        let entry = by_group.entry(group).or_insert((0, 0));
        entry.1 += 1;
        let mut ok = true;
        for (index, result) in case_difference(&case).into_iter().enumerate() {
            runs_total += 1;
            let randomize = case["runs"][index]["options"]["randomize"] == Value::Bool(true);
            let tag = if randomize { " (randomize)" } else { "" };
            match result {
                Ok(0.0) => {}
                Ok(d) => {
                    ok = false;
                    differing.push(format!("{name} run {}{tag}: differs by {d}", index + 1));
                }
                Err(reason) => {
                    ok = false;
                    differing.push(format!("{name} run {}{tag}: {reason}", index + 1));
                }
            }
        }
        if ok {
            cases_ok += 1;
            entry.0 += 1;
        }
    }
    let randomized = differing
        .iter()
        .filter(|l| l.contains("(randomize)"))
        .count();
    println!(
        "{} runs differ; {randomized} of them are randomize: true",
        differing.len()
    );
    for line in differing.iter().take(80) {
        println!("differs  {line}");
    }
    for (group, (ok, total)) in &by_group {
        println!("{group}: {ok} of {total} cases agree");
    }
    println!(
        "{cases_ok} of {} cases match the reference exactly ({runs_total} runs)",
        names.len()
    );
    assert!(
        names.len() >= CASE_FLOOR,
        "the corpus has {} cases",
        names.len()
    );
    assert!(differing.is_empty(), "{} runs differ", differing.len());
}
