//! The hook for a node placement of the caller's (`ferrigura_elk::placement`): it runs after the
//! network simplex placement of a graph and nowhere else, and a layout without it is ELK's.

use ferrigura_elk::json::JsonNode;
use ferrigura_elk::placement::{Kind, Layers, Placement, layout_with};
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

/// The left edge of a node.
fn left(layout: &JsonNode, id: &str) -> f64 {
    layout
        .children
        .iter()
        .find(|node| node.id == id)
        .expect("the node")
        .x
        .expect("a place")
}

const FAN: [&str; 3] = ["top", "left", "right"];
const FAN_EDGES: [(&str, &str); 2] = [("top", "left"), ("top", "right")];

/// Moves every node by 500 along its layer.
struct Shifted;

impl Placement for Shifted {
    fn place(&self, layers: &mut Layers) {
        // The view is the graph: three nodes in two layers, two edges, a port for each end.
        assert_eq!(
            layers.layers.iter().map(Vec::len).collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(layers.nodes.iter().all(|node| node.kind == Kind::Node));
        assert_eq!((layers.edges.len(), layers.ports.len()), (2, 4));
        assert_eq!(layers.nodes[layers.layers[1][1]].gap, 120.0);
        for node in &mut layers.nodes {
            node.place += 500.0;
        }
    }
}

/// Places the nodes by the lengths of the steps alone, with the same weight for each edge.
struct Solved;

impl Placement for Solved {
    fn place(&self, layers: &mut Layers) {
        let places = layers.solve(&vec![1.0; layers.edges.len()]);
        for (node, place) in places.into_iter().enumerate() {
            layers.nodes[node].place = place;
        }
        // One of the two edges of the fan is straight, as the lengths ask.
        assert!((0..layers.edges.len()).any(|edge| layers.step(edge) == 0.0));
    }
}

/// Puts the second node of each layer on the first.
struct Overlapping;

impl Placement for Overlapping {
    fn place(&self, layers: &mut Layers) {
        for members in layers.layers.clone() {
            for pair in members.windows(2) {
                layers.nodes[pair[1]].place = layers.nodes[pair[0]].place;
            }
        }
    }
}

#[test]
fn the_places_of_the_callers_placement_are_the_layouts() {
    let input = graph("NETWORK_SIMPLEX", &FAN, &FAN_EDGES);
    let plain = ferrigura_elk::layout(&input, false)
        .expect("a layout")
        .graph;
    let shifted = layout_with(&input, false, Shifted).expect("a layout").graph;
    // The drawing starts at the same corner, so a common shift leaves every node where it was.
    for node in FAN {
        assert_eq!(left(&shifted, node), left(&plain, node), "{node}");
    }
    let solved = layout_with(&input, false, Solved).expect("a layout").graph;
    assert_eq!(
        left(&solved, "right") - left(&solved, "left"),
        left(&plain, "right") - left(&plain, "left")
    );
}

#[test]
fn a_placement_that_breaks_the_least_distances_is_dropped() {
    let input = graph("NETWORK_SIMPLEX", &FAN, &FAN_EDGES);
    let plain = ferrigura_elk::layout(&input, false).expect("a layout");
    let dropped = layout_with(&input, false, Overlapping).expect("a layout");
    assert_eq!(
        serde_json::to_value(&plain.graph).unwrap(),
        serde_json::to_value(&dropped.graph).unwrap()
    );
}

#[test]
fn another_placement_strategy_is_left_alone() {
    /// Fails when it is run.
    struct Unused;
    impl Placement for Unused {
        fn place(&self, _: &mut Layers) {
            panic!("the placement ran for a graph that does not place with network simplex");
        }
    }
    let input = graph("BRANDES_KOEPF", &FAN, &FAN_EDGES);
    let plain = ferrigura_elk::layout(&input, false).expect("a layout");
    let hooked = layout_with(&input, false, Unused).expect("a layout");
    assert_eq!(
        serde_json::to_value(&plain.graph).unwrap(),
        serde_json::to_value(&hooked.graph).unwrap()
    );
}

#[test]
fn the_placement_ends_with_its_layout() {
    let input = graph("NETWORK_SIMPLEX", &FAN, &FAN_EDGES);
    let before = ferrigura_elk::layout(&input, false).expect("a layout");
    layout_with(&input, false, Solved).expect("a layout");
    let after = ferrigura_elk::layout(&input, false).expect("a layout");
    assert_eq!(
        serde_json::to_value(&before.graph).unwrap(),
        serde_json::to_value(&after.graph).unwrap()
    );
}
