//! The reference corpus of `runSwimlaneLayoutCore`: for each case in `tests/corpus/manifest.json`, the stages
//! of the port must equal the stages the reference recorded (`tests/corpus/<name>.json`).
//!
//! The `sugiyama` stage is the `LayoutResult` of `sugiyamaLayout`: the acyclic edges (`id`, `src`, `dst`),
//! the reversed edges, the layering with its ranks and dummy nodes, the ordering, and the coordinates. The
//! `writeback` stage is the layout data after `writeBackToLayoutData`. Numbers are compared with `==` on
//! `f64`, so `-0` equals `0`; strings, booleans and key sets are compared exactly; every key of the
//! reference's snapshot must be present, and every key of the port's must be in the reference.
//!
//! The `output` stage (the layout after routing and post-processing) needs the router and the geometry
//! passes, which are not ported yet; its test is ignored until they land.

use std::path::PathBuf;

use ferrigura_swimlane::testing::{
    LayoutOptions, LayoutResult, layout_data_from_json, layout_data_to_json, sugiyama_layout,
    to_graph_view, write_back_to_layout_data,
};
use ferrigura_swimlane::{Direction, LayoutData, run_swimlane_layout_core};
use serde_json::{Value, json};

fn corpus_dir() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "corpus"]
        .iter()
        .collect()
}

fn load(name: &str) -> Value {
    let path = corpus_dir().join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn case_names() -> Vec<String> {
    let manifest = load("manifest");
    manifest["cases"]
        .as_array()
        .expect("the manifest lists its cases")
        .iter()
        .map(|case| case["name"].as_str().expect("a case name").to_owned())
        .collect()
}

/// The stage of the case with the given name.
fn stage<'a>(case: &'a Value, name: &str) -> &'a Value {
    case["stages"]
        .as_array()
        .expect("the stages of a case")
        .iter()
        .find(|stage| stage["stage"] == name)
        .map(|stage| &stage["data"])
        .unwrap_or_else(|| panic!("no stage {name}"))
}

/// How a list item is named in a message: its id when it has one, its text when it is a string, and its
/// index otherwise.
fn item_label(index: usize, item: &Value) -> String {
    if let Some(id) = item.get("id").and_then(Value::as_str) {
        return format!("[{id}]");
    }
    if let Some(text) = item.as_str() {
        return format!("[{text}]");
    }
    format!("[{index}]")
}

/// The first difference between the reference's value and the port's, with the path to it.
fn first_difference(path: &str, expected: &Value, actual: &Value) -> Option<String> {
    match (expected, actual) {
        (Value::Number(_), Value::Number(_)) => {
            if expected.as_f64() == actual.as_f64() {
                None
            } else {
                Some(format!("{path}: expected {expected}, got {actual}"))
            }
        }
        (Value::Array(expected_items), Value::Array(actual_items)) => {
            if expected_items.len() != actual_items.len() {
                return Some(format!(
                    "{path}: expected {} items, got {}",
                    expected_items.len(),
                    actual_items.len()
                ));
            }
            expected_items
                .iter()
                .zip(actual_items)
                .enumerate()
                .find_map(|(index, (x, y))| {
                    first_difference(&format!("{path}{}", item_label(index, x)), x, y)
                })
        }
        (Value::Object(expected_map), Value::Object(actual_map)) => {
            for (key, x) in expected_map {
                match actual_map.get(key) {
                    None => return Some(format!("{path}.{key}: missing")),
                    Some(y) => {
                        if let Some(message) = first_difference(&format!("{path}.{key}"), x, y) {
                            return Some(message);
                        }
                    }
                }
            }
            actual_map
                .keys()
                .find(|key| !expected_map.contains_key(*key))
                .map(|key| format!("{path}.{key}: not in the reference"))
        }
        _ => {
            if expected == actual {
                None
            } else {
                Some(format!("{path}: expected {expected}, got {actual}"))
            }
        }
    }
}

/// The edge fields that the `sugiyama` stage compares: `id`, `src` and `dst`.
fn edge_ids(edges: &Value) -> Value {
    Value::Array(
        edges
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .map(|edge| json!({"id": edge["id"], "src": edge["src"], "dst": edge["dst"]}))
            .collect(),
    )
}

/// The `sugiyama` stage of the port, in the shape of the reference's stage.
fn sugiyama_actual(result: &LayoutResult) -> Value {
    let acyclic_edges: Vec<Value> = result
        .acyclic
        .edges
        .iter()
        .map(|edge| json!({"id": edge.id, "src": edge.src, "dst": edge.dst}))
        .collect();
    let reversed: Vec<Value> = result
        .reversed
        .iter()
        .map(|edge| json!({"id": edge.id, "src": edge.src, "dst": edge.dst}))
        .collect();
    let rank_of: serde_json::Map<String, Value> = result
        .layering
        .rank_of
        .iter()
        .map(|(id, rank)| (id.clone(), json!(rank)))
        .collect();
    let coordinates = |map: &indexmap::IndexMap<String, f64>| -> serde_json::Map<String, Value> {
        map.iter()
            .map(|(id, value)| (id.clone(), json!(value)))
            .collect()
    };
    json!({
        "acyclic": {"nodes": result.acyclic.nodes, "edges": acyclic_edges},
        "reversed": reversed,
        "layering": {
            "layers": result.layering.layers,
            "rankOf": rank_of,
            "dummy": result.layering.dummy.iter().collect::<Vec<_>>(),
        },
        "ordered": {"layers": result.ordered.layers},
        "coordinates": {
            "x": coordinates(&result.coordinates.x),
            "y": coordinates(&result.coordinates.y),
        },
    })
}

/// The `sugiyama` stage of the reference, with the edge fields the port compares.
fn sugiyama_expected(stage: &Value) -> Value {
    json!({
        "acyclic": {"nodes": stage["acyclic"]["nodes"], "edges": edge_ids(&stage["acyclic"]["edges"])},
        "reversed": edge_ids(&stage["reversed"]),
        "layering": {
            "layers": stage["layering"]["layers"],
            "rankOf": stage["layering"]["rankOf"],
            "dummy": stage["layering"]["dummy"],
        },
        "ordered": {"layers": stage["ordered"]["layers"]},
        "coordinates": {
            "x": stage["coordinates"]["x"],
            "y": stage["coordinates"]["y"],
        },
    })
}

/// The options of the default path, read from the case's configuration as the core reads them.
fn options(input: &Value) -> LayoutOptions {
    let flowchart = &input["config"]["flowchart"];
    let swimlane = &input["config"]["swimlane"];
    LayoutOptions {
        node_gap: flowchart["nodeSpacing"].as_f64().unwrap_or(40.0),
        layer_gap: flowchart["rankSpacing"].as_f64().unwrap_or(100.0),
        ignore_cross_lane_edges: swimlane["ignoreCrossLaneEdges"].as_bool().unwrap_or(true),
        automatic_lane_ordering: swimlane["automaticLaneOrdering"].as_bool().unwrap_or(false),
        direction: Direction::parse(input["direction"].as_str().unwrap_or("TB"))
            .expect("the corpus directions are TB, LR, BT or RL"),
    }
}

/// Runs the phases on a case, and returns the first difference of each compared stage.
fn compare_case(name: &str) -> Vec<String> {
    let case = load(name);
    let input = &case["input"];
    let mut failures = Vec::new();

    let mut data: LayoutData = layout_data_from_json(input)
        .unwrap_or_else(|error| panic!("{name}: the input does not read: {error}"));
    let opts = options(input);
    let g = to_graph_view(&data);
    let result = match sugiyama_layout(&g, &opts) {
        Ok(result) => result,
        Err(error) => return vec![format!("{name}: sugiyama: {error}")],
    };

    let expected = sugiyama_expected(stage(&case, "sugiyama"));
    if let Some(message) = first_difference("sugiyama", &expected, &sugiyama_actual(&result)) {
        failures.push(format!("{name}: {message}"));
    }

    write_back_to_layout_data(
        &g,
        &mut data,
        &result.ordered,
        &result.coordinates,
        opts.layer_gap,
        opts.node_gap,
    );
    let actual = layout_data_to_json(&data);
    if let Some(message) = first_difference("writeback", stage(&case, "writeback"), &actual) {
        failures.push(format!("{name}: {message}"));
    }
    failures
}

/// Reports the failures of a stage, and fails when there are any: the count, then the first few.
fn assert_no_failures(stage: &str, failures: &[String], cases: usize) {
    if failures.is_empty() {
        return;
    }
    let shown: Vec<&str> = failures.iter().take(25).map(String::as_str).collect();
    panic!(
        "{stage}: {} of {cases} cases differ from the reference\n{}",
        failures.len(),
        shown.join("\n")
    );
}

#[test]
fn sugiyama_and_writeback_stages_match_the_reference() {
    let names = case_names();
    let failures: Vec<String> = names.iter().flat_map(|name| compare_case(name)).collect();
    assert_no_failures("sugiyama and writeback", &failures, names.len());
}

#[test]
#[ignore = "port: the router and the post-processing are not ported; run with --ignored once they land"]
fn output_matches_the_reference_after_routing() {
    let mut failures = Vec::new();
    let names = case_names();
    for name in &names {
        let case = load(name);
        let mut data = layout_data_from_json(&case["input"]).expect("the input reads");
        match run_swimlane_layout_core(&mut data) {
            Ok(direction) => {
                let expected_direction = case["returnedDirection"].as_str().unwrap_or("");
                if direction.as_str() != expected_direction {
                    failures.push(format!(
                        "{name}: returnedDirection: expected {expected_direction}, got {}",
                        direction.as_str()
                    ));
                }
                let actual = layout_data_to_json(&data);
                if let Some(message) = first_difference("output", &case["output"], &actual) {
                    failures.push(format!("{name}: {message}"));
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert_no_failures("output", &failures, names.len());
}
