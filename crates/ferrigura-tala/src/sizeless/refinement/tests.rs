// SPDX-License-Identifier: MPL-2.0
// From pinned D2 TALA: internal/placement/centered_neighbors_test.go and
// internal/placementcost/symmetry_test.go.

use super::*;

fn graph_with_bodies(
    positions: &[(f64, f64, f64, f64)],
    edges: &[(usize, usize)],
) -> (Graph, Vec<NodeGeometry>, Vec<usize>) {
    let bodies = positions
        .iter()
        .enumerate()
        .map(|(index, &(x, y, width, height))| NodeGeometry {
            id: index.to_string(),
            x,
            y,
            width,
            height,
        })
        .collect::<Vec<_>>();
    let graph = Graph {
        direction: None,
        nodes: bodies
            .iter()
            .map(|body| crate::Node {
                id: body.id.clone(),
                parent_id: None,
                direction: None,
                is_group: false,
                shape: "rect".to_owned(),
                width: body.width,
                height: body.height,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
        edges: edges
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| crate::Edge {
                id: index.to_string(),
                source: source.to_string(),
                target: target.to_string(),
                source_arrow: false,
                target_arrow: false,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
    };
    let order = (0..bodies.len()).collect();
    (graph, bodies, order)
}

#[test]
fn symmetry_uses_the_exact_peer_centerline() {
    let positions = [
        (300.0, 400.0, 80.0, 60.0),
        (0.0, 220.0, 80.0, 60.0),
        (250.0, 200.0, 80.0, 100.0),
        (375.0, 220.0, 80.0, 60.0),
    ];
    for transpose in [false, true] {
        let positions = positions.map(|(x, y, w, h)| {
            if transpose {
                (y, x, h, w)
            } else {
                (x, y, w, h)
            }
        });
        let (graph, bodies, order) = graph_with_bodies(&positions, &[(0, 1), (0, 2), (0, 3)]);
        let mut stage = Refinement::new(&graph, &bodies, &order).unwrap();
        assert!(stage.axis_score(&[1, 2, 3]) > 1.0);
        stage
            .balance_symmetry(&mut Work(MAX_OPTIMIZATION_WORK))
            .unwrap();
        let center = stage.center(0);
        assert_eq!(if transpose { center.y } else { center.x }, 227.0);
    }
}

#[test]
fn equidistance_leaves_already_centered_nodes_in_place() {
    let (graph, bodies, order) = graph_with_bodies(
        &[
            (0.0, 0.0, 40.0, 40.0),
            (120.0, 0.0, 40.0, 40.0),
            (240.0, 0.0, 40.0, 40.0),
        ],
        &[(0, 1), (1, 2)],
    );
    let mut stage = Refinement::new(&graph, &bodies, &order).unwrap();
    assert_eq!(
        stage.equidistance(&mut Work(MAX_OPTIMIZATION_WORK)),
        Ok(false)
    );
    assert_eq!(stage.nodes, bodies);
}

#[test]
fn axis_score_matches_the_go_alignment_cases() {
    let (graph, bodies, order) = graph_with_bodies(
        &[
            (0.0, 0.0, 101.0, 10.0),
            (0.0, 100.0, 100.0, 10.0),
            (0.0, 200.0, 100.0, 10.0),
        ],
        &[],
    );
    let mut stage = Refinement::new(&graph, &bodies, &order).unwrap();
    assert_eq!(stage.axis_score(&[0, 1]), 1.0);
    assert_eq!(stage.axis_score(&[0, 1, 2]), 1.0);
    stage.nodes[2].x = 70.0;
    assert_eq!(stage.axis_score(&[0, 1, 2]), 1.33 / 2.0);
    stage.nodes[1].x = 700.0;
    stage.nodes[1].y = 200.0;
    assert_eq!(stage.axis_score(&[0, 1, 2]), 0.0);
    stage.nodes[1].x = -70.0;
    stage.nodes[1].y = 100.0;
    assert_eq!(stage.axis_score(&[0, 1, 2]), 0.66 / 2.0);
    stage.nodes[1].x = 30.0;
    stage.nodes[2].x = 30.0;
    assert_eq!(stage.axis_score(&[0, 1, 2]), (0.66 + 0.66) / 2.0);
}

#[test]
fn normalization_commits_a_smaller_gap_without_replacing_the_position() {
    let (graph, bodies, order) = graph_with_bodies(
        &[(0.0, 0.0, 10.0, 10.0), (1000.0, 0.0, 10.0, 10.0)],
        &[(0, 1)],
    );
    let mut stage = Refinement::new(&graph, &bodies, &order).unwrap();
    let position = std::ptr::addr_of!(stage.nodes[1].x);
    assert_eq!(
        stage.normalize_gaps(&mut Work(MAX_OPTIMIZATION_WORK)),
        Ok(true)
    );
    assert_eq!(std::ptr::addr_of!(stage.nodes[1].x), position);
    assert_eq!(stage.nodes[1].x, 160.0);
    assert_eq!(cell_size(&stage.nodes), 10.0);
}
