//! The reference corpus of `runSwimlaneLayoutCore`, for the stages that the ported phases produce: for each
//! case, the stages of the port must equal the stages the reference recorded. The cases and the comparison
//! are the shared harness's (`tests/support/corpus.rs`).
//!
//! The `sugiyama` stage is the `LayoutResult` of `sugiyamaLayout`: the acyclic edges (`id`, `src`, `dst`),
//! the reversed edges, the layering with its ranks and dummy nodes, the ordering, and the coordinates. The
//! `writeback` stage is the layout data after `writeBackToLayoutData`, compared on every key. A failure
//! names the case, the stage, the path of the first difference, and the id of the node or edge.
//!
//! The `output` stage (the layout after routing and post-processing) needs the router and the geometry
//! passes, which are not ported yet; its test is ignored until they land.

mod support;

use ferrigura_swimlane::testing::{
    LayoutOptions, LayoutResult, layout_data_from_json, layout_data_to_json, sugiyama_layout,
    to_graph_view, write_back_to_layout_data,
};
use ferrigura_swimlane::{Direction, run_swimlane_layout_core};
use serde_json::{Value, json};
use support::corpus::{Case, compare_snapshot, compare_values, load_cases};

/// Adds the id of the node or edge that a harness path names (`nodes[3]`, `edges[0]`) to the message.
fn with_id(message: String, snapshot: &Value) -> String {
    for list in ["nodes", "edges"] {
        let marker = format!(".{list}[");
        let Some(start) = message.find(&marker) else {
            continue;
        };
        let rest = &message[start + marker.len()..];
        let Some(end) = rest.find(']') else {
            continue;
        };
        let Ok(index) = rest[..end].parse::<usize>() else {
            continue;
        };
        if let Some(id) = snapshot[list][index]["id"].as_str() {
            return format!("{message} ({list} id {id})");
        }
    }
    message
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
    let edges = |edges: &[ferrigura_swimlane::testing::EdgeRef]| -> Vec<Value> {
        edges
            .iter()
            .map(|edge| json!({"id": edge.id, "src": edge.src, "dst": edge.dst}))
            .collect()
    };
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
        "acyclic": {"nodes": result.acyclic.nodes, "edges": edges(&result.acyclic.edges)},
        "reversed": edges(&result.reversed),
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

/// The `sugiyama` stage of the reference, with the parts the port compares. `acyclic.layout`,
/// `acyclic.nodeById` and the `ref` of each edge are the reference's objects, which the port does not hold.
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
fn compare_case(case: &Case) -> Vec<String> {
    let name = &case.name;
    let mut failures = Vec::new();

    let mut data = layout_data_from_json(&case.input)
        .unwrap_or_else(|error| panic!("{name}: the input does not read: {error}"));
    let opts = options(&case.input);
    let g = to_graph_view(&data);
    let result = match sugiyama_layout(&g, &opts) {
        Ok(result) => result,
        Err(error) => return vec![format!("{name}: sugiyama: {error}")],
    };

    let reference = case
        .stage("sugiyama")
        .expect("every case has a sugiyama stage");
    if let Err(message) = compare_values(
        &sugiyama_expected(reference),
        &sugiyama_actual(&result),
        "sugiyama",
    ) {
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
    let reference = case
        .stage("writeback")
        .expect("every case has a writeback stage");
    if let Err(message) = compare_snapshot(reference, &layout_data_to_json(&data)) {
        failures.push(format!(
            "{name}: writeback: {}",
            with_id(message, reference)
        ));
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
        "{stage}: {} failures in {cases} cases\n{}",
        failures.len(),
        shown.join("\n")
    );
}

#[test]
fn sugiyama_and_writeback_stages_match_the_reference() {
    let cases = load_cases();
    assert_eq!(cases.len(), 131, "the corpus has 131 cases");
    let failures: Vec<String> = cases.iter().flat_map(compare_case).collect();
    assert_no_failures("sugiyama and writeback", &failures, cases.len());
}

#[test]
#[ignore = "port: the router and the post-processing are not ported; run with --ignored once they land"]
fn output_matches_the_reference_after_routing() {
    let cases = load_cases();
    let mut failures = Vec::new();
    for case in &cases {
        let name = &case.name;
        let mut data = layout_data_from_json(&case.input).expect("the input reads");
        match run_swimlane_layout_core(&mut data) {
            Ok(direction) => {
                if direction.as_str() != case.returned_direction {
                    failures.push(format!(
                        "{name}: returnedDirection: expected {}, got {}",
                        case.returned_direction,
                        direction.as_str()
                    ));
                }
                if let Err(message) = compare_snapshot(&case.output, &layout_data_to_json(&data)) {
                    failures.push(format!(
                        "{name}: output: {}",
                        with_id(message, &case.output)
                    ));
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert_no_failures("output", &failures, cases.len());
}
