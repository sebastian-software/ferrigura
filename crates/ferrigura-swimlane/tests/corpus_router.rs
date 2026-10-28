//! The router against the reference: for each case of the corpus, the `writeback` stage is the input of the
//! router, with the points of every edge removed as the core removes them, and the `routed` stage is what the
//! reference's `routeEdgesOrthogonal` left in the layout. The layout-only edges are gone from `routed`, as the
//! router drops them at its last line (`router.ts:2220`).
//!
//! The comparison is exact (see `support::corpus::compare_snapshot`). A failure names the case, the path of the
//! first difference and the two values. Run with `--nocapture` to see the count of matching cases.

mod support;

use std::collections::BTreeMap;

use ferrigura_swimlane::Direction;
use ferrigura_swimlane::testing::{
    layout_data_from_json, layout_data_to_json, route_edges_orthogonal,
};
use serde_json::Value;
use support::corpus::{Case, compare_snapshot, load_cases};

/// Routes the `writeback` stage of a case, and returns the router's layout as JSON, or the reason it failed.
fn route(case: &Case) -> Result<Value, String> {
    let writeback = case
        .stage("writeback")
        .ok_or_else(|| "no writeback stage".to_string())?;
    let mut data = layout_data_from_json(writeback)?;
    for edge in &mut data.edges {
        edge.points = None;
    }
    let direction = Direction::parse(&case.returned_direction)
        .ok_or_else(|| format!("direction {}", case.returned_direction))?;
    route_edges_orthogonal(&mut data, direction).map_err(|error| error.to_string())?;
    Ok(layout_data_to_json(&data))
}

/// A difference with its indices removed, so that the differences of one kind are counted together.
fn kind_of(message: &str) -> String {
    let path = message.split(": ").next().unwrap_or(message);
    let mut kind = String::new();
    let mut in_index = false;
    for c in path.chars() {
        match c {
            '[' => {
                in_index = true;
                kind.push_str("[]");
            }
            ']' => in_index = false,
            _ if in_index => {}
            _ => kind.push(c),
        }
    }
    kind
}

#[test]
fn every_routed_stage_matches_the_reference() {
    let cases = load_cases();
    let mut matched = 0;
    let mut kinds: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for case in &cases {
        let expected = case
            .stage("routed")
            .unwrap_or_else(|| panic!("{}: no routed stage", case.name));
        let outcome = match route(case) {
            Ok(actual) => compare_snapshot(expected, &actual),
            Err(error) => Err(error),
        };
        match outcome {
            Ok(()) => matched += 1,
            Err(message) => {
                let entry = kinds
                    .entry(kind_of(&message))
                    .or_insert_with(|| (0, format!("{}: {message}", case.name)));
                entry.0 += 1;
            }
        }
    }
    println!("matched {matched} of {} cases", cases.len());
    for (kind, (count, first)) in &kinds {
        println!("{count:4} x {kind}\n       first: {first}");
    }
    assert_eq!(
        matched,
        cases.len(),
        "{} cases differ from the reference",
        cases.len() - matched
    );
}

#[test]
fn the_comparison_sees_a_one_ulp_difference() {
    // A control: the exact comparison must reject a point that differs in its last bit, or every match above
    // would be vacuous.
    let cases = load_cases();
    let case = &cases[0];
    let expected = case.stage("routed").expect("the routed stage").clone();
    let mut perturbed = expected.clone();
    let x = perturbed["edges"]
        .as_array_mut()
        .expect("edges")
        .iter_mut()
        .find_map(|edge| edge["points"].as_array_mut()?.first_mut())
        .expect("a point")["x"]
        .as_f64()
        .expect("a number");
    perturbed["edges"]
        .as_array_mut()
        .expect("edges")
        .iter_mut()
        .find_map(|edge| edge["points"].as_array_mut()?.first_mut())
        .expect("a point")["x"] = serde_json::json!(f64::from_bits(x.to_bits() + 1));
    assert!(compare_snapshot(&expected, &perturbed).is_err());
    assert!(compare_snapshot(&expected, &expected).is_ok());
}
