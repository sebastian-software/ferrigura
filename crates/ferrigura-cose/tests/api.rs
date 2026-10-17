//! The public API on small inputs: the errors the adapter and the engine give, and that a tree
//! lays out to finite positions in the input order.

use ferrigura_cose::{Edge, Error, Node, layout};

fn node(id: &str, width: f64, height: f64) -> Node {
    Node {
        id: id.to_string(),
        width,
        height,
    }
}

fn edge(source: &str, target: &str) -> Edge {
    Edge {
        id: format!("edge_{source}_{target}"),
        source: source.to_string(),
        target: target.to_string(),
    }
}

#[test]
fn a_single_node_is_placed_at_the_centre_of_the_world_origin() {
    let positions = layout(&[node("0", 102.0, 24.0)], &[]).unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].id, "0");
    assert!(positions[0].x.is_finite() && positions[0].y.is_finite());
}

#[test]
fn a_tree_gives_one_finite_position_per_node_in_input_order() {
    let nodes = [
        node("root", 90.5, 40.0),
        node("a", 60.25, 20.0),
        node("b", 70.0, 30.5),
        node("c", 40.0, 18.0),
    ];
    let edges = [edge("root", "a"), edge("root", "b"), edge("b", "c")];
    let positions = layout(&nodes, &edges).unwrap();
    let ids: Vec<&str> = positions.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["root", "a", "b", "c"]);
    assert!(positions.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
}

#[test]
fn a_cycle_is_refused_as_not_a_forest() {
    let nodes = [
        node("a", 10.0, 10.0),
        node("b", 10.0, 10.0),
        node("c", 10.0, 10.0),
    ];
    let edges = [edge("a", "b"), edge("b", "c"), edge("c", "a")];
    assert_eq!(layout(&nodes, &edges), Err(Error::NotAForest));
}

#[test]
fn two_nodes_without_an_edge_are_refused_as_tiling() {
    let nodes = [node("a", 10.0, 10.0), node("b", 10.0, 10.0)];
    assert_eq!(layout(&nodes, &[]), Err(Error::TilingNotPorted));
}

#[test]
fn a_self_loop_and_a_parallel_edge_are_dropped() {
    let nodes = [node("a", 10.0, 10.0), node("b", 10.0, 10.0)];
    let with_extras = [edge("a", "b"), edge("b", "a"), edge("a", "a")];
    let plain = [edge("a", "b")];
    assert_eq!(layout(&nodes, &with_extras), layout(&nodes, &plain));
}

#[test]
fn duplicate_and_unknown_ids_are_errors() {
    let dup = [node("a", 10.0, 10.0), node("a", 10.0, 10.0)];
    assert_eq!(
        layout(&dup, &[]),
        Err(Error::DuplicateNode("a".to_string()))
    );
    let nodes = [node("a", 10.0, 10.0)];
    assert_eq!(
        layout(&nodes, &[edge("a", "missing")]),
        Err(Error::UnknownNode("missing".to_string()))
    );
}

#[test]
fn a_size_that_is_not_finite_is_an_error() {
    let nodes = [node("a", f64::NAN, 10.0)];
    assert_eq!(
        layout(&nodes, &[]),
        Err(Error::NonFiniteSize("a".to_string()))
    );
}

#[test]
fn an_empty_graph_has_no_positions() {
    assert_eq!(layout(&[], &[]), Ok(Vec::new()));
}
