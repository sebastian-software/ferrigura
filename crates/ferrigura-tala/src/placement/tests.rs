// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Deserialize;

use super::{MAX_COMPONENTS, place};
use crate::{Graph, NodeGeometry};

#[derive(Deserialize)]
struct Case {
    name: String,
    graph: Graph,
    nodes: Vec<NodeGeometry>,
}

fn fixtures() -> Vec<Case> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

#[test]
fn matches_pinned_go_rectangles() {
    // Produced by the pinned Go bridge in tools/tala-pilot/reference, with
    // seed 1 and source-order input. Check positions and dimensions exactly.
    for Case { name, graph, nodes } in fixtures() {
        assert_eq!(place(&graph).as_ref(), Some(&nodes), "{name}");
    }
}

#[test]
fn rejects_graphs_outside_flat_packing_contract() {
    let mut single = fixtures()
        .into_iter()
        .find(|case| case.name == "single")
        .unwrap()
        .graph;
    single.nodes[0].label_width = single.nodes[0].width + 1.0;
    assert!(place(&single).is_none());
    single.nodes[0].label_width = 0.0;
    single.nodes[0].parent_id = Some("container".to_owned());
    assert!(place(&single).is_none());

    let mut full = fixtures()
        .into_iter()
        .find(|case| case.name == "component-limit")
        .unwrap()
        .graph;
    assert_eq!(full.nodes.len(), MAX_COMPONENTS);
    let extra = serde_json::from_str(
        r#"{
        "id": "extra", "isGroup": false, "shape": "rect",
        "width": 100, "height": 50, "label": "", "labelWidth": 0, "labelHeight": 0
    }"#,
    )
    .unwrap();
    full.nodes.push(extra);
    assert!(place(&full).is_none());
}

#[test]
fn refuses_coincident_rectangles_from_reference_corner_rounding() {
    // Verified with the pinned Go runner: both rectangles end at (0, 0).
    let input = r#"{
        "direction": "TB",
        "nodes": [
          {"id":"a","shape":"rect","isGroup":false,"width":100.1,"height":60.1,
           "label":"","labelWidth":0,"labelHeight":0},
          {"id":"b","shape":"rect","isGroup":false,"width":100.1,"height":60.1,
           "label":"","labelWidth":0,"labelHeight":0}
        ],
        "edges": []
    }"#;
    assert!(place(&serde_json::from_str(input).unwrap()).is_none());
}
