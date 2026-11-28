use serde::Deserialize;
use serde_json::Value;

use crate::{Edge, Graph, Node};

use super::{LayoutGraph, Preprocessing};
use crate::layout_graph::tree_preprocess_checkpoint;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    input: Graph,
    expected: Value,
}

#[test]
fn measured_tree_rewrites_and_hierarchy_inputs_match_pinned_go() {
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!("fixtures.json")).unwrap();
    assert_eq!(fixtures.len(), 3);
    for Fixture {
        name,
        input,
        expected,
    } in fixtures
    {
        let actual = serde_json::to_value(tree_preprocess_checkpoint(&input).unwrap()).unwrap();
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn extract_directed_pair() {
    let authored = directed_line(2);
    let mut stage = Preprocessing::new(&authored);
    stage.extract(0).unwrap();
    assert_eq!(stage.state.nodes, [0]);
    assert!(stage.state.edges.is_empty());
    assert_eq!(stage.state.tree_roots, [vec![1], vec![]]);
    assert_eq!(stage.state.members[1].sentinel_edge, Some(0));
    assert!(stage.state.members[1].children.is_empty());
}

#[test]
fn extract_line_graph() {
    let authored = directed_line(3);
    let mut stage = Preprocessing::new(&authored);
    stage.extract(0).unwrap();
    assert_eq!(stage.state.nodes, [0]);
    assert!(stage.state.edges.is_empty());
    assert_eq!(stage.state.tree_roots, [vec![1], vec![], vec![]]);
    assert_eq!(stage.state.members[1].children, [2]);
    assert_eq!(stage.state.members[1].sentinel_edge, Some(0));
    assert_eq!(stage.state.members[2].sentinel_edge, Some(1));
}

fn directed_line(size: usize) -> LayoutGraph {
    let graph = Graph {
        direction: None,
        nodes: (0..size)
            .map(|index| Node {
                id: format!("n{index}"),
                parent_id: None,
                direction: None,
                is_group: false,
                shape: "rect".to_owned(),
                width: 20.0,
                height: 10.0,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
        edges: (0..size - 1)
            .map(|index| Edge {
                id: format!("e{index}"),
                source: format!("n{index}"),
                target: format!("n{}", index + 1),
                source_arrow: false,
                target_arrow: true,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
    };
    LayoutGraph::translate(&graph).unwrap()
}
