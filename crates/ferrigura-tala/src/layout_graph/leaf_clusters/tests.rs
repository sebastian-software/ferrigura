use serde_json::Value;

use super::*;
use crate::layout_graph::leaf_cluster_checkpoint;
use crate::{Edge, Graph, Node};

#[test]
fn publishing_flow_cluster_and_edge_rebindings_match_pinned_go() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../tree_preprocess/fixtures.json")).unwrap();
    let publishing = cases
        .into_iter()
        .find(|fixture| fixture["name"] == "publishing-flow")
        .unwrap();
    let input = serde_json::from_value(publishing["input"].clone()).unwrap();
    let expected: Capture = serde_json::from_str(include_str!("publishing.json")).unwrap();
    let result = leaf_cluster_checkpoint(&input).unwrap();
    let actual: Capture = serde_json::from_value(serde_json::to_value(result).unwrap()).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn add_clusters() {
    let graph = authored(3, &[(0, 1), (0, 2)]);
    let workspace = TreePreprocess::from_authored(&graph);
    let result = graph.leaf_clusters(&workspace, &[]).unwrap();
    assert_eq!(result.clusters.len(), 1);
    assert_eq!(result.clusters[0].nodes, [1, 2]);
    assert_eq!(result.clusters[0].edges, [0, 1]);
    assert_eq!(
        result.endpoints,
        [(Endpoint::Node(0), Endpoint::Vessel(0)); 2]
    );
    assert_eq!((graph.edges[0].from, graph.edges[0].to), (0, 1));
    assert_eq!((graph.edges[1].from, graph.edges[1].to), (0, 2));
}

#[test]
fn add_clusters_hierarchy() {
    let graph = authored(3, &[(0, 1), (0, 2)]);
    let workspace = TreePreprocess::from_authored(&graph);
    let result = graph.leaf_clusters(&workspace, &[0, 1, 2]).unwrap();
    assert!(result.clusters.is_empty());
}

#[test]
fn multiple_shared_adjacent() {
    let graph = authored(4, &[(0, 1), (0, 2), (1, 3)]);
    let workspace = TreePreprocess::from_authored(&graph);
    let result = graph.leaf_clusters(&workspace, &[]).unwrap();
    assert!(result.clusters.is_empty());
}

#[test]
fn consistent_multiple_shared_adjacent() {
    let graph = authored(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
    let workspace = TreePreprocess::from_authored(&graph);
    let result = graph.leaf_clusters(&workspace, &[]).unwrap();
    assert_eq!(result.clusters.len(), 1);
    assert_eq!(result.clusters[0].nodes, [1, 2]);
}

#[test]
fn derives_vessel_from_the_authored_measurements_and_one_tie_draw() {
    let mut graph = authored(3, &[(0, 1), (0, 2)]);
    let workspace = TreePreprocess::from_authored(&graph);
    let result = graph.leaf_clusters(&workspace, &[]).unwrap();
    let cluster = &result.clusters[0];
    assert!(cluster.geometry(&graph, || None).is_none());
    let row = cluster.geometry(&graph, || Some(0.5)).unwrap();
    assert_eq!(
        row,
        VesselGeometry {
            arrangement: Arrangement::Row,
            padding: 20.0,
            width: 30.0,
            height: 5.0,
            member_sizes: vec![[5.0, 5.0]; 2],
        }
    );
    let column = cluster.geometry(&graph, || Some(0.500_000_000_01)).unwrap();
    assert_eq!(
        column,
        VesselGeometry {
            arrangement: Arrangement::Column,
            padding: 20.0,
            width: 5.0,
            height: 30.0,
            member_sizes: vec![[5.0, 5.0]; 2],
        }
    );
    graph.nodes[1].width = 11.0;
    assert_eq!(
        cluster
            .geometry(&graph, || panic!("non-square clusters do not draw"))
            .unwrap(),
        VesselGeometry {
            arrangement: Arrangement::Column,
            padding: 20.0,
            width: 11.0,
            height: 30.0,
            member_sizes: vec![[11.0, 5.0]; 2],
        }
    );
}

fn authored(size: usize, edges: &[(usize, usize)]) -> LayoutGraph {
    let graph = Graph {
        direction: None,
        nodes: (0..size)
            .map(|index| Node {
                id: format!("n{index}"),
                parent_id: None,
                direction: None,
                is_group: false,
                shape: "rect".to_owned(),
                width: 5.0,
                height: 5.0,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
        edges: edges
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| Edge {
                id: format!("e{index}"),
                source: format!("n{source}"),
                target: format!("n{target}"),
                source_arrow: false,
                target_arrow: false,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
    };
    LayoutGraph::translate(&graph).unwrap()
}
