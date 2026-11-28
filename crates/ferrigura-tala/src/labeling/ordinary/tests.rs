use serde::Deserialize;

use super::*;
use crate::labeling::place_ordinary;
use crate::layout_graph::LayoutGraph;
use crate::route_output::Anchor;
use crate::{EdgeGeometry, Graph};

#[test]
fn ordinary_observatory_seed_one_matches_pinned_go() {
    #[derive(Deserialize)]
    struct Fixture {
        graph: Graph,
        before: Before,
        after: Vec<EdgeGeometry>,
    }
    #[derive(Deserialize)]
    struct Before {
        nodes: Vec<Box>,
    }

    let fixture: Fixture =
        serde_json::from_str(include_str!("../../balance/observatory.json")).unwrap();
    let graph = LayoutGraph::translate(&fixture.graph).unwrap();
    let mut node_labels = vec![None; graph.nodes.len()];
    for node in &graph.nodes {
        node_labels[node.input_index] = node.label.as_ref();
    }
    let paths: Vec<_> = fixture.after.into_iter().map(|edge| edge.points).collect();
    let edge_labels: Vec<_> = graph.edges.iter().map(|edge| edge.label.as_ref()).collect();
    let result = place_ordinary(&fixture.before.nodes, &node_labels, &paths, &edge_labels).unwrap();
    let mut remaining = MAX_WORK;
    assert_eq!(
        placement_order(&paths, &edge_labels, &mut remaining).unwrap(),
        [7, 5, 0, 2, 8, 3, 1, 4, 6]
    );
    assert_eq!(
        result.node_boxes[2],
        Some(Box {
            x: 1200.0,
            y: 1268.5,
            width: 0.0,
            height: 0.0
        })
    );
    let expected = [
        (Anchor::OutsideTopCenter, 1467.0, 1137.0, 75.0),
        (Anchor::OutsideTopLeft, 1382.0, 1236.0, 63.0),
        (Anchor::OutsideTopCenter, 1069.0, 1310.0, 61.0),
        (Anchor::OutsideTopRight, 1141.0, 1317.0, 99.0),
        (Anchor::OutsideBottomLeft, 1345.0, 1275.0, 83.0),
        (Anchor::OutsideBottomCenter, 1111.0, 1540.0, 72.0),
        (Anchor::OutsideTopCenter, 1331.0, 1427.0, 64.0),
        (Anchor::OutsideTopCenter, 1228.0, 1040.0, 76.0),
        (Anchor::OutsideTopCenter, 1593.0, 1062.0, 72.0),
    ];
    for (index, (anchor, x, y, width)) in expected.into_iter().enumerate() {
        let label = result.edge_labels[index].unwrap();
        assert_eq!(label.anchor, anchor);
        assert_eq!(label.percentage.to_bits(), 0.0_f64.to_bits());
        assert_eq!(
            result.edge_boxes[index],
            Some(Box {
                x,
                y,
                width,
                height: 21.0
            })
        );
    }
}

#[test]
fn caption_order_prioritizes_bends_and_preserves_caller_order_on_equal_lengths() {
    let path = |coordinates: &[(f64, f64)]| {
        coordinates
            .iter()
            .map(|&(x, y)| Point { x, y })
            .collect::<Vec<_>>()
    };
    let paths = [
        path(&[(0.0, 0.0), (3.0, 4.0)]),
        path(&[(0.0, 0.0), (4.0, 0.0), (4.0, 6.0)]),
        path(&[(0.0, 0.0), (0.0, 5.0)]),
        path(&[(0.0, 0.0), (0.0, 1.0)]),
        path(&[(0.0, 0.0), (0.0, 0.5), (0.5, 0.5)]),
    ];
    let caption = Label {
        text: "caption".into(),
        width: 4.0,
        height: 4.0,
    };
    let labels = [
        Some(&caption),
        Some(&caption),
        Some(&caption),
        None,
        Some(&caption),
    ];
    let mut remaining = MAX_WORK;
    assert_eq!(
        placement_order(&paths, &labels, &mut remaining).unwrap(),
        [4, 1, 0, 2]
    );
    let node = Box {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 8.0,
    };
    let result = place_ordinary(&[node, node], &[None, Some(&caption)], &paths, &labels).unwrap();
    assert_eq!(
        result.node_boxes,
        [
            None,
            Some(Box {
                x: 3.0,
                y: 2.0,
                width: 4.0,
                height: 4.0
            })
        ]
    );
    assert!(result.edge_boxes[3].is_none());
    assert!(result.edge_labels[3].is_none());
}

#[test]
fn parallel_label_repair_keeps_completed_placement_when_budget_runs_out() {
    let shapes = [
        Box {
            x: 0.0,
            y: 0.0,
            width: 160.0,
            height: 160.0,
        },
        Box {
            x: 0.0,
            y: 319.0,
            width: 160.0,
            height: 160.0,
        },
    ];
    let paths = [40.0, 80.0, 120.0].map(|x| vec![Point { x, y: 160.0 }, Point { x, y: 319.0 }]);
    let caption = Label {
        text: "four lines".into(),
        width: 75.0,
        height: 69.0,
    };
    let captions = [Some(&caption); 3];
    let mut placement = Placement {
        paths: &paths,
        edge_labels: &captions,
        order: vec![0, 1, 2],
        others: shapes
            .into_iter()
            .map(|body| NodeObstacle {
                body,
                is_container: false,
            })
            .collect(),
        shared: vec![],
        obstacles: vec![],
        candidates: vec![],
    };
    let mut remaining = MAX_WORK;
    let (fixed, collisions) = placement.greedy(&mut remaining).unwrap();
    assert!(!collisions.is_empty());
    let greedy = placement.candidates.clone();
    let mut repair = 10;
    placement.repair(fixed, collisions, &mut repair).unwrap();
    assert_eq!(placement.candidates, greedy);

    let repaired = place_ordinary(&shapes, &[None; 2], &paths, &captions).unwrap();
    let expected = [
        (Anchor::OutsideBottomCenter, -42.0, 205.0),
        (Anchor::OutsideTopLeft, 87.0, 165.0),
        (Anchor::OutsideTopRight, 127.0, 245.0),
    ];
    let mut occupied = shapes.to_vec();
    for (index, (anchor, x, y)) in expected.into_iter().enumerate() {
        let label = repaired.edge_labels[index].unwrap();
        assert_eq!(label.anchor, anchor);
        assert_eq!(label.percentage.to_bits(), 0.0_f64.to_bits());
        let body = repaired.edge_boxes[index].unwrap();
        assert_eq!(
            body,
            Box {
                x,
                y,
                width: 75.0,
                height: 69.0
            }
        );
        assert!(occupied.iter().all(|&other| !overlaps(body, other, 0.0)));
        occupied.push(body);
    }
}
