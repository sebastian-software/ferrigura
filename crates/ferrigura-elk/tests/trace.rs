//! The processor trace of the layered driver against elkjs's, on corpus graphs.
//!
//! The first test compares the task names and their nesting, which is the
//! processor sequence. The second also compares the log messages that the
//! processors write; they come from Brandes & Koepf and the other processors
//! whose ports are still to do, so it stays ignored until they are.

use std::path::PathBuf;

use ferrigura_elk::json::JsonNode;
use serde_json::Value;

fn corpus_case(name: &str) -> Value {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "corpus",
        &format!("{name}.json"),
    ]
    .iter()
    .collect();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// Removes the `logs` keys, leaving names and nesting.
fn without_logs(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| key.as_str() != "logs")
                .map(|(key, value)| (key.clone(), without_logs(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_logs).collect()),
        other => other.clone(),
    }
}

fn layout_trace(case: &Value) -> Value {
    let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
    let layout = ferrigura_elk::layout(&input, true).expect("layout");
    layout.trace.to_json()
}

#[test]
fn nodes_only_trace_matches_elkjs_names_and_nesting() {
    for name in [
        "synthetic__nodes-only-1",
        "synthetic__nodes-only-3",
        "synthetic__nodes-only-5",
    ] {
        let case = corpus_case(name);
        let ours = layout_trace(&case);
        assert_eq!(without_logs(&ours), without_logs(&case["trace"]), "{name}");
    }
}

#[test]
#[ignore = "port: Brandes & Koepf and the other processors do not log their messages yet"]
fn nodes_only_trace_matches_elkjs_including_logs() {
    let case = corpus_case("synthetic__nodes-only-3");
    assert_eq!(layout_trace(&case), case["trace"]);
}

#[test]
#[ignore = "diagnostic: prints the log messages side by side"]
fn print_logs_side_by_side() {
    let case = corpus_case("synthetic__nodes-only-3");
    let ours = layout_trace(&case);
    fn collect(value: &Value, out: &mut Vec<(String, Vec<String>)>) {
        if let Some(name) = value["name"].as_str() {
            let logs = value["logs"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|i| i.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            out.push((name.to_string(), logs));
        }
        for child in value["children"].as_array().into_iter().flatten() {
            collect(child, out);
        }
    }
    let (mut ours_list, mut theirs_list) = (Vec::new(), Vec::new());
    collect(&ours, &mut ours_list);
    collect(&case["trace"], &mut theirs_list);
    for (ours_item, theirs_item) in ours_list.iter().zip(&theirs_list) {
        if ours_item != theirs_item {
            println!("ours:   {:?} {:?}", ours_item.0, ours_item.1);
            println!("elkjs:  {:?} {:?}", theirs_item.0, theirs_item.1);
        }
    }
}
