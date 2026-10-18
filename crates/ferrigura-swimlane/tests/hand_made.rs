//! Hand-made layouts whose results are worked out by hand from the reference sources, to check the phases
//! and the seam where the corpus is not yet in the branch. The corpus tests (`tests/corpus.rs`) are the
//! proof; these fix the arithmetic of a small case that a reader can follow.

use ferrigura_swimlane::testing::{
    LayoutOptions, layout_data_from_json, layout_data_to_json, sugiyama_layout, to_graph_view,
};
use ferrigura_swimlane::{Direction, Error, LayoutData, run_swimlane_layout_core};
use serde_json::{Value, json};

/// Two lanes, `A` and `B`, with one node each and an edge from `a` to `b`. Each node is 100 by 50, and the
/// lanes have the default padding of 20.
fn two_lanes() -> Value {
    json!({
        "direction": "TB",
        "config": {
            "flowchart": {"nodeSpacing": 40, "rankSpacing": 100},
            "swimlane": {"ignoreCrossLaneEdges": true, "automaticLaneOrdering": false},
            "look": "neo"
        },
        "nodes": [
            {"id": "A", "isGroup": true, "shape": "swimlane", "padding": 20, "label": "A", "direction": "TB"},
            {"id": "B", "isGroup": true, "shape": "swimlane", "padding": 20, "label": "B"},
            {"id": "a", "isGroup": false, "parentId": "A", "width": 100, "height": 50, "shape": "rect"},
            {"id": "b", "isGroup": false, "parentId": "B", "width": 100, "height": 50, "shape": "rect"}
        ],
        "edges": [{"id": "e1", "start": "a", "end": "b", "curve": "basis"}]
    })
}

/// One lane with `a -> x -> b` and the long edge `a -> b` that skips `x`'s layer.
fn long_edge() -> Value {
    json!({
        "direction": "TB",
        "config": {
            "flowchart": {"nodeSpacing": 40, "rankSpacing": 100},
            "swimlane": {"ignoreCrossLaneEdges": true, "automaticLaneOrdering": false}
        },
        "nodes": [
            {"id": "A", "isGroup": true, "shape": "swimlane", "padding": 20},
            {"id": "a", "isGroup": false, "parentId": "A", "width": 100, "height": 50},
            {"id": "x", "isGroup": false, "parentId": "A", "width": 100, "height": 50},
            {"id": "b", "isGroup": false, "parentId": "A", "width": 100, "height": 50}
        ],
        "edges": [
            {"id": "e1", "start": "a", "end": "x"},
            {"id": "e2", "start": "x", "end": "b"},
            {"id": "e3", "start": "a", "end": "b"}
        ]
    })
}

fn load(value: &Value) -> LayoutData {
    layout_data_from_json(value).expect("the hand-made layout reads")
}

fn node<'a>(data: &'a LayoutData, id: &str) -> &'a ferrigura_swimlane::Node {
    data.nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

#[test]
fn two_lanes_are_written_back_and_then_routing_is_reported_missing() {
    let mut data = load(&two_lanes());
    let error = run_swimlane_layout_core(&mut data).expect_err("the router is not ported");
    assert_eq!(error, Error::NotPorted("router"));

    // The lanes are columns in the reverse of their source order: `B` on the left, `A` on the right.
    // The columns are 100 wide, with a lane gap of 2 * 40, so `b` sits at -90 and `a` at 90.
    let a = node(&data, "a");
    assert_eq!(
        (a.layer, a.order, a.x, a.y),
        (Some(0.0), Some(0.0), Some(90.0), Some(25.0))
    );
    let b = node(&data, "b");
    assert_eq!(
        (b.layer, b.order, b.x, b.y),
        (Some(0.0), Some(1.0), Some(-90.0), Some(25.0))
    );

    // The lane bounds are 140 wide before the solver, then 180 for both lanes: their centres are 180 apart,
    // so the two widths meet at x = 0. The height is the content (50) plus twice the margin (36).
    let lane_a = node(&data, "A");
    assert_eq!(lane_a.x, Some(90.0));
    assert_eq!(lane_a.y, Some(25.0));
    assert_eq!(lane_a.width, Some(180.0));
    assert_eq!(lane_a.height, Some(122.0));
    assert_eq!(lane_a.swimlane_content_top, Some(0.0));
    let title = lane_a.group_title_rect.expect("the title band");
    assert_eq!(
        (title.left, title.right, title.top, title.bottom),
        (0.0, 180.0, -36.0, -15.0)
    );
    let lane_b = node(&data, "B");
    assert_eq!(lane_b.x, Some(-90.0));
    assert_eq!(lane_b.width, Some(180.0));
    let title = lane_b.group_title_rect.expect("the title band");
    assert_eq!(
        (title.left, title.right, title.top, title.bottom),
        (-180.0, 0.0, -36.0, -15.0)
    );
}

#[test]
fn the_edge_points_are_deleted_before_the_routing() {
    let mut input = two_lanes();
    input["edges"][0]["points"] = json!([{"x": 1, "y": 2}]);
    let mut data = load(&input);
    run_swimlane_layout_core(&mut data).expect_err("the router is not ported");
    assert_eq!(data.edges[0].points, None);
}

#[test]
fn non_default_options_are_refused_before_any_work() {
    let mut input = two_lanes();
    input["config"]["swimlane"]["ignoreCrossLaneEdges"] = json!(false);
    let before = load(&input);
    let mut data = before.clone();
    assert_eq!(
        run_swimlane_layout_core(&mut data),
        Err(Error::NonDefaultOption("ignoreCrossLaneEdges: false"))
    );
    assert_eq!(data, before, "a refused layout is not changed");

    let mut input = two_lanes();
    input["config"]["swimlane"]["automaticLaneOrdering"] = json!(true);
    let mut data = load(&input);
    assert_eq!(
        run_swimlane_layout_core(&mut data),
        Err(Error::NonDefaultOption("automaticLaneOrdering: true"))
    );
}

#[test]
fn a_direction_outside_the_four_is_refused() {
    let mut input = two_lanes();
    input["direction"] = json!("TD");
    let mut data = load(&input);
    assert_eq!(
        run_swimlane_layout_core(&mut data),
        Err(Error::UnknownDirection("TD".to_owned()))
    );
}

#[test]
fn a_non_ascii_id_is_refused() {
    let mut input = two_lanes();
    input["nodes"][2]["id"] = json!("ä");
    input["edges"][0]["start"] = json!("ä");
    let mut data = load(&input);
    assert_eq!(
        run_swimlane_layout_core(&mut data),
        Err(Error::NonAsciiId("ä".to_owned()))
    );
}

#[test]
fn a_long_edge_gets_one_dummy_on_its_middle_layer() {
    let data = load(&long_edge());
    let g = to_graph_view(&data);
    let opts = LayoutOptions {
        node_gap: 40.0,
        layer_gap: 100.0,
        ignore_cross_lane_edges: true,
        automatic_lane_ordering: false,
        direction: Direction::TB,
    };
    let result = sugiyama_layout(&g, &opts).expect("the default path");

    // `a` on layer 0, `x` on layer 1 with the dummy of `e3`, `b` on layer 2.
    assert_eq!(
        result.ordered.layers,
        vec![
            vec!["a".to_owned()],
            vec!["x".to_owned(), "placeholder-0".to_owned()],
            vec!["b".to_owned()],
        ]
    );
    assert_eq!(result.layering.rank_of.get("placeholder-0"), Some(&1));
    assert!(result.layering.dummy.contains("placeholder-0"));

    // The dummy sits at the rounded midpoint of its ends, which share one lane column at x = 40.
    assert_eq!(result.coordinates.x.get("placeholder-0"), Some(&40.0));
    assert_eq!(result.coordinates.x.get("a"), Some(&40.0));
    assert_eq!(result.coordinates.x.get("b"), Some(&40.0));
}

#[test]
fn a_layout_that_is_read_and_written_back_keeps_every_key() {
    let mut input = two_lanes();
    input["diagramId"] = json!("swimlane-1");
    input["nodes"][2]["labelBBox"] = json!({"width": 12.5, "height": 8});
    input["nodes"][2]["dir"] = json!("TB");
    input["edges"][0]["label"] = json!("");
    input["edges"][0]["isUserDefinedId"] = json!(true);
    let data = load(&input);
    assert_eq!(layout_data_to_json(&data), input);
}

#[test]
fn a_field_of_an_unexpected_type_stays_in_extra() {
    let mut input = two_lanes();
    input["nodes"][2]["width"] = json!(null);
    let data = load(&input);
    assert_eq!(node(&data, "a").width, None);
    assert_eq!(node(&data, "a").extra.get("width"), Some(&Value::Null));
    assert_eq!(layout_data_to_json(&data), input);
}
