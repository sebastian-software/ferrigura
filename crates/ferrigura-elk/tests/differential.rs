//! Differential run over the corpus: the port's coordinates against elkjs's.
//!
//! Each case in `tests/corpus` holds the ELK JSON graph Mermaid gave elkjs and
//! elkjs's result. `corpus_summary` lists, per case, whether the port lays it
//! out and why not; `coordinates_match_elkjs` compares the node boxes and the
//! root size with elkjs's, and lists the cases that differ.

use std::path::PathBuf;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;

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

/// Every edge below `value`, level by level: the edges a level holds, then the levels of its
/// children, in document order. Both the input and the output keep each edge at its level.
fn all_edges(value: &Value) -> Vec<&Value> {
    let mut edges: Vec<&Value> = value["edges"].as_array().into_iter().flatten().collect();
    for child in value["children"].as_array().into_iter().flatten() {
        edges.extend(all_edges(child));
    }
    edges
}

/// Each edge's route as a flat list of numbers: the number of sections, then per section its
/// start, the number of its bend points and the bend points, and its end.
fn edge_routes(value: &Value) -> Vec<Vec<f64>> {
    let point = |value: &Value| {
        [
            value["x"].as_f64().unwrap_or(0.0),
            value["y"].as_f64().unwrap_or(0.0),
        ]
    };
    all_edges(value)
        .into_iter()
        .map(|edge| {
            let sections = edge["sections"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            let mut route = vec![sections.len() as f64];
            for section in sections {
                route.extend(point(&section["startPoint"]));
                let bends = section["bendPoints"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                route.push(bends.len() as f64);
                for bend in bends {
                    route.extend(point(bend));
                }
                route.extend(point(&section["endPoint"]));
            }
            route
        })
        .collect()
}

/// Each edge label's box as `[x, y, width, height]`, in the order of the edges and their labels.
fn label_boxes(value: &Value) -> Vec<[f64; 4]> {
    all_edges(value)
        .into_iter()
        .flat_map(|edge| edge["labels"].as_array().into_iter().flatten())
        .map(|label| {
            [
                label["x"].as_f64().unwrap_or(0.0),
                label["y"].as_f64().unwrap_or(0.0),
                label["width"].as_f64().unwrap_or(0.0),
                label["height"].as_f64().unwrap_or(0.0),
            ]
        })
        .collect()
}

/// The largest difference between the port's node boxes and elkjs's, and between the root sizes.
/// Errors when the nodes differ in number or order: that is not a difference in coordinates.
fn largest_difference(ours: &JsonNode, theirs: &Value) -> Result<f64, String> {
    let ours_value = serde_json::to_value(ours).unwrap();
    let (mut ours_boxes, mut theirs_boxes) = (Vec::new(), Vec::new());
    node_boxes(&ours_value, &mut ours_boxes);
    node_boxes(theirs, &mut theirs_boxes);
    if ours_boxes.len() != theirs_boxes.len() {
        return Err(format!(
            "{} nodes, elkjs has {}",
            ours_boxes.len(),
            theirs_boxes.len()
        ));
    }
    let mut worst = 0.0_f64;
    for (a, b) in ours_boxes.iter().zip(&theirs_boxes) {
        if a.0 != b.0 {
            return Err(format!("node {} where elkjs has {}", a.0, b.0));
        }
        worst = [a.1 - b.1, a.2 - b.2, a.3 - b.3, a.4 - b.4]
            .into_iter()
            .map(f64::abs)
            .fold(worst, f64::max);
    }
    let (ours_routes, theirs_routes) = (edge_routes(&ours_value), edge_routes(theirs));
    if ours_routes.len() != theirs_routes.len() {
        return Err(format!(
            "{} edges, elkjs has {}",
            ours_routes.len(),
            theirs_routes.len()
        ));
    }
    for (ours_route, theirs_route) in ours_routes.iter().zip(&theirs_routes) {
        if ours_route.len() != theirs_route.len() {
            return Err("an edge route with another shape".to_string());
        }
        for (a, b) in ours_route.iter().zip(theirs_route) {
            worst = worst.max((a - b).abs());
        }
    }
    let (ours_labels, theirs_labels) = (label_boxes(&ours_value), label_boxes(theirs));
    if ours_labels.len() != theirs_labels.len() {
        return Err(format!(
            "{} labels, elkjs has {}",
            ours_labels.len(),
            theirs_labels.len()
        ));
    }
    for (a, b) in ours_labels.iter().zip(&theirs_labels) {
        for (x, y) in a.iter().zip(b) {
            worst = worst.max((x - y).abs());
        }
    }
    let size = |value: &Value| {
        (
            value["width"].as_f64().unwrap_or(0.0),
            value["height"].as_f64().unwrap_or(0.0),
        )
    };
    let ((ours_width, ours_height), (theirs_width, theirs_height)) =
        (size(&ours_value), size(theirs));
    Ok(worst
        .max((ours_width - theirs_width).abs())
        .max((ours_height - theirs_height).abs()))
}

/// Cases whose coordinates match elkjs. The floor only goes up, as `SUPPORTED_FLOOR` does.
const MATCHING_FLOOR: usize = 130;

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
const SUPPORTED_FLOOR: usize = 130;

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
