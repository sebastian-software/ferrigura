//! The calm node placement (`layered::p4nodes::settle`), which is not a part of ELK: it runs only
//! inside [`ferrigura_elk::settled`], on a graph that places its nodes with network simplex.

use ferrigura_elk::json::JsonNode;
use serde_json::json;

/// A graph from top to bottom with nodes of 100 by 40 and the given edges.
fn graph(strategy: &str, nodes: &[&str], edges: &[(&str, &str)]) -> JsonNode {
    serde_json::from_value(json!({
        "id": "root",
        "layoutOptions": {
            "elk.algorithm": "elk.layered",
            "elk.direction": "DOWN",
            "elk.layered.nodePlacement.strategy": strategy,
        },
        "children": nodes
            .iter()
            .map(|id| json!({ "id": id, "width": 100, "height": 40 }))
            .collect::<Vec<_>>(),
        "edges": edges
            .iter()
            .enumerate()
            .map(|(index, (from, to))| {
                json!({ "id": format!("e{index}"), "sources": [from], "targets": [to] })
            })
            .collect::<Vec<_>>(),
    }))
    .expect("a graph")
}

/// The middle of a node across the flow.
fn middle(layout: &JsonNode, id: &str) -> f64 {
    let node = layout
        .children
        .iter()
        .find(|node| node.id == id)
        .expect("the node");
    node.x.expect("a place") + node.width.expect("a width") / 2.0
}

#[test]
fn a_node_stands_in_the_middle_of_an_even_fan_and_on_the_axis_of_a_chain() {
    let nodes = ["top", "left", "right", "below", "end"];
    let edges = [
        ("top", "left"),
        ("top", "right"),
        ("left", "below"),
        ("right", "below"),
        ("below", "end"),
    ];
    let input = graph("NETWORK_SIMPLEX", &nodes, &edges);
    let settled = ferrigura_elk::settled(|| ferrigura_elk::layout(&input, false))
        .expect("a layout")
        .graph;
    let between = (middle(&settled, "left") + middle(&settled, "right")) / 2.0;
    for node in ["top", "below", "end"] {
        assert_eq!(middle(&settled, node), between, "{node}");
    }

    // ELK's own placement puts the node over one end of its fan.
    let plain = ferrigura_elk::layout(&input, false)
        .expect("a layout")
        .graph;
    let between = (middle(&plain, "left") + middle(&plain, "right")) / 2.0;
    assert_ne!(middle(&plain, "top"), between);
}

#[test]
fn another_placement_strategy_is_left_alone() {
    let nodes = ["top", "left", "right"];
    let edges = [("top", "left"), ("top", "right")];
    let input = graph("BRANDES_KOEPF", &nodes, &edges);
    let plain = ferrigura_elk::layout(&input, false).expect("a layout");
    let settled =
        ferrigura_elk::settled(|| ferrigura_elk::layout(&input, false)).expect("a layout");
    assert_eq!(
        serde_json::to_value(&plain.graph).unwrap(),
        serde_json::to_value(&settled.graph).unwrap()
    );
}

#[test]
fn the_switch_ends_with_its_run() {
    let nodes = ["top", "left", "right"];
    let edges = [("top", "left"), ("top", "right")];
    let input = graph("NETWORK_SIMPLEX", &nodes, &edges);
    let before = ferrigura_elk::layout(&input, false).expect("a layout");
    ferrigura_elk::settled(|| ferrigura_elk::layout(&input, false)).expect("a layout");
    let after = ferrigura_elk::layout(&input, false).expect("a layout");
    assert_eq!(
        serde_json::to_value(&before.graph).unwrap(),
        serde_json::to_value(&after.graph).unwrap()
    );
}
