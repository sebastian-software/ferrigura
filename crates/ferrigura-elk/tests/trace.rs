//! The processor trace of the layered driver against elkjs's, on corpus graphs.
//!
//! The first test compares the task names and their nesting, which is the
//! processor sequence, on the nodes-only cases. The second compares the full
//! trace with the log messages, on every case the port lays out.

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

/// The names of all corpus cases, from the manifest.
fn corpus_names() -> Vec<String> {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "corpus",
        "manifest.json",
    ]
    .iter()
    .collect();
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["name"].as_str().unwrap().to_string())
        .collect()
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

/// Cases whose full trace, logs included, matches elkjs. The floor only goes up.
const TRACE_MATCHING_FLOOR: usize = 118;

#[test]
fn supported_traces_match_elkjs_including_logs() {
    let mut matching = Vec::new();
    for name in corpus_names() {
        let case = corpus_case(&name);
        let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
        let Ok(layout) = ferrigura_elk::layout(&input, true) else {
            continue;
        };
        if layout.trace.to_json() == case["trace"] {
            matching.push(name);
        } else {
            println!("logs differ  {name}");
        }
    }
    println!(
        "{} supported cases have the same trace, logs included",
        matching.len()
    );
    assert!(
        matching.len() >= TRACE_MATCHING_FLOOR,
        "{} cases have the same trace; the floor is {TRACE_MATCHING_FLOOR}",
        matching.len()
    );
}

#[test]
#[ignore = "diagnostic: prints the log messages side by side"]
fn print_logs_side_by_side() {
    let name = std::env::var("ELK_CASE").unwrap_or_else(|_| "synthetic__nodes-only-3".to_string());
    let case = corpus_case(&name);
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
    if let Ok(path) = std::env::var("ELK_TRACE_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&ours).unwrap()).unwrap();
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

/// Names, logs and the first difference, for each corpus case the driver supports.
#[test]
#[ignore = "diagnostic: prints how the supported corpus cases agree with elkjs"]
fn report_agreement_on_supported_cases() {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(
            [
                env!("CARGO_MANIFEST_DIR"),
                "tests",
                "corpus",
                "manifest.json",
            ]
            .iter()
            .collect::<PathBuf>(),
        )
        .unwrap(),
    )
    .unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let case = corpus_case(name);
        let input: JsonNode = serde_json::from_value(case["input"].clone()).unwrap();
        match ferrigura_elk::layout(&input, true) {
            Err(error) => println!("skip   {name}: {error}"),
            Ok(layout) => {
                let ours = layout.trace.to_json();
                let names = without_logs(&ours) == without_logs(&case["trace"]);
                let logs = ours == case["trace"];
                println!("names={names} logs={logs}  {name}");
            }
        }
    }
}
