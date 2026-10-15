//! Differential run over the corpus: the port's coordinates against elkjs's.
//!
//! Each case in `tests/corpus` holds the ELK JSON graph Mermaid gave elkjs and
//! elkjs's result. `corpus_summary` lists, per case, whether the port lays it
//! out and why not; `coordinates_match_elkjs` compares every node position that
//! the port produced. Both are ignored until the port covers the cases.

use std::path::PathBuf;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;

const TOLERANCE: f64 = 1e-9;

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

/// Collects `(id, x, y, width, height)` of every node below `value`, in document order.
fn node_boxes(value: &Value, out: &mut Vec<(String, f64, f64, f64, f64)>) {
    for child in value["children"].as_array().into_iter().flatten() {
        let field = |key: &str| child[key].as_f64().unwrap_or(0.0);
        out.push((
            child["id"].as_str().unwrap_or_default().to_string(),
            field("x"),
            field("y"),
            field("width"),
            field("height"),
        ));
        node_boxes(child, out);
    }
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

#[test]
#[ignore = "port: coordinates are compared only once the processors that place nodes are ported"]
fn coordinates_match_elkjs() {
    let mut worst: f64 = 0.0;
    let mut compared = 0;
    for name in case_names() {
        let case = load(&name);
        let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
        let Ok(layout) = ferrigura_elk::layout(&input, false) else {
            continue;
        };
        let ours_value = serde_json::to_value(&layout.graph).unwrap();
        let (mut ours, mut theirs) = (Vec::new(), Vec::new());
        node_boxes(&ours_value, &mut ours);
        node_boxes(&case["output"], &mut theirs);
        assert_eq!(ours.len(), theirs.len(), "{name}: node count");
        for (a, b) in ours.iter().zip(&theirs) {
            assert_eq!(a.0, b.0, "{name}: node order");
            let difference = [a.1 - b.1, a.2 - b.2, a.3 - b.3, a.4 - b.4]
                .into_iter()
                .map(f64::abs)
                .fold(0.0, f64::max);
            worst = worst.max(difference);
            assert!(
                difference <= TOLERANCE,
                "{name}: node {} differs by {difference}",
                a.0
            );
        }
        compared += 1;
    }
    println!("compared {compared} cases; largest coordinate difference {worst}");
}

/// Cases that pass the driver today. The floor only goes up: a change that makes a
/// case unsupported again fails here, and so does CI. Raise it as the port grows.
const SUPPORTED_FLOOR: usize = 22;

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
