// SPDX-License-Identifier: MPL-2.0
// Ported from pinned D2 TALA: internal/placement/dejitter_test.go.

use super::*;
use crate::{Edge, Node};

#[test]
fn basic_dejitter() {
    let mut nodes = bodies(&[(4.0, 4.0), (20.0, 3.0)]);
    let mut paths = [points(&[
        (12.0, 4.0),
        (16.0, 4.0),
        (16.0, 3.0),
        (20.0, 3.0),
    ])];
    assert!(run(&mut nodes, &[[0, 1]], &mut paths));
    assert_eq!(paths[0], points(&[(12.0, 3.0), (20.0, 3.0)]));
}

#[test]
fn prohibit_moving_a_node_with_another_tangent_edge() {
    let mut nodes = bodies(&[(20.0, 4.0), (40.0, 3.0), (4.0, 4.0)]);
    let mut paths = [
        points(&[(28.0, 4.0), (34.0, 4.0), (34.0, 3.0), (40.0, 3.0)]),
        points(&[(12.0, 4.0), (20.0, 4.0)]),
    ];
    assert!(run(&mut nodes, &[[0, 1], [2, 0]], &mut paths));
    assert_eq!(paths[0], points(&[(28.0, 4.0), (40.0, 4.0)]));
}

#[test]
fn preserve_jitter_when_both_nodes_have_tangent_edges() {
    let mut nodes = bodies(&[(20.0, 4.0), (40.0, 3.0), (60.0, 3.0), (4.0, 4.0)]);
    let before = [
        points(&[(28.0, 4.0), (34.0, 4.0), (34.0, 3.0), (40.0, 3.0)]),
        points(&[(12.0, 4.0), (20.0, 4.0)]),
        points(&[(48.0, 3.0), (60.0, 3.0)]),
    ];
    let mut paths = before.clone();
    assert!(!run(&mut nodes, &[[0, 1], [3, 0], [1, 2]], &mut paths));
    assert_eq!(paths, before);
}

fn run(nodes: &mut [NodeGeometry], owners: &[[usize; 2]], paths: &mut [Vec<Point>]) -> bool {
    let graph = Graph {
        direction: None,
        nodes: nodes
            .iter()
            .map(|body| Node {
                id: body.id.clone(),
                parent_id: None,
                direction: None,
                is_group: false,
                shape: "rect".into(),
                width: body.width,
                height: body.height,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
        edges: owners
            .iter()
            .enumerate()
            .map(|(index, &[source, target])| Edge {
                id: index.to_string(),
                source: nodes[source].id.clone(),
                target: nodes[target].id.clone(),
                source_arrow: false,
                target_arrow: true,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
    };
    let order: Vec<_> = (0..nodes.len()).collect();
    dejitter(&graph, owners, &order, nodes, paths).unwrap()
}

fn bodies(coordinates: &[(f64, f64)]) -> Vec<NodeGeometry> {
    coordinates
        .iter()
        .enumerate()
        .map(|(index, &(x, y))| NodeGeometry {
            id: index.to_string(),
            x,
            y,
            width: 8.0,
            height: 8.0,
        })
        .collect()
}

fn points(coordinates: &[(f64, f64)]) -> Vec<Point> {
    coordinates.iter().map(|&(x, y)| Point { x, y }).collect()
}
