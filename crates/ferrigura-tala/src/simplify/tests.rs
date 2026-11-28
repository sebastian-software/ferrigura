// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/routing/{edge_simplify_safety,post_route_atomicity}_test.go.

use super::*;
use crate::{Edge, EdgeGeometry, Node};

fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn node(id: &str, x: f64, y: f64, width: f64, height: f64) -> NodeGeometry {
    NodeGeometry {
        id: id.into(),
        x,
        y,
        width,
        height,
    }
}

fn edge(id: &str, source: &str, target: &str) -> Edge {
    Edge {
        id: id.into(),
        source: source.into(),
        target: target.into(),
        source_arrow: false,
        target_arrow: true,
        label: String::new(),
        label_width: 0.0,
        label_height: 0.0,
    }
}

fn cases(obstruction: &str, transpose: bool) -> (Graph, Layout) {
    let mut boxes = vec![
        node("a", 0.0, 0.0, 40.0, 40.0),
        node("b", 240.0, 50.0, 40.0, 40.0),
    ];
    let mut edges = vec![edge("e", "a", "b")];
    let mut routes = vec![EdgeGeometry {
        id: "e".into(),
        points: [
            (40.0, 20.0),
            (80.0, 20.0),
            (80.0, 120.0),
            (200.0, 120.0),
            (200.0, 70.0),
            (240.0, 70.0),
        ]
        .map(|(x, y)| point(x, y))
        .to_vec(),
        label: None,
    }];
    match obstruction {
        "node" => boxes.push(node("obstacle", 190.0, 35.0, 20.0, 20.0)),
        "edge" => {
            boxes.push(node("c", 140.0, 40.0, 10.0, 10.0));
            boxes.push(node("d", 210.0, 40.0, 10.0, 10.0));
            edges.push(edge("other", "c", "d"));
            routes.push(EdgeGeometry {
                id: "other".into(),
                points: vec![point(150.0, 45.0), point(210.0, 45.0)],
                label: None,
            });
        }
        "target" => {
            boxes[1].x = 180.0;
            boxes[1].y = 30.0;
            routes[0].points.pop();
        }
        "clear" => {}
        _ => panic!("unknown fixture"),
    }
    if transpose {
        for node in &mut boxes {
            std::mem::swap(&mut node.x, &mut node.y);
            std::mem::swap(&mut node.width, &mut node.height);
        }
        for point in routes.iter_mut().flat_map(|edge| &mut edge.points) {
            std::mem::swap(&mut point.x, &mut point.y);
        }
    }
    routed_graph(boxes, edges, routes)
}

fn routed_graph(
    boxes: Vec<NodeGeometry>,
    edges: Vec<Edge>,
    routes: Vec<EdgeGeometry>,
) -> (Graph, Layout) {
    let nodes = boxes
        .iter()
        .map(|node| Node {
            id: node.id.clone(),
            parent_id: None,
            direction: None,
            is_group: false,
            shape: "rectangle".into(),
            width: node.width,
            height: node.height,
            label: String::new(),
            label_width: 0.0,
            label_height: 0.0,
        })
        .collect();
    (
        Graph {
            direction: None,
            nodes,
            edges,
        },
        Layout {
            nodes: boxes,
            edges: routes,
        },
    )
}

#[test]
fn both_replacement_legs_and_the_target_approach_follow_the_reference() {
    for transpose in [false, true] {
        for obstruction in ["node", "edge", "target", "clear"] {
            let (graph, mut layout) = cases(obstruction, transpose);
            let before = layout.edges[0].points.clone();
            simplify_routes(&graph, &mut layout).unwrap();
            let expected = if obstruction == "clear" {
                let turn = if transpose {
                    point(20.0, 200.0)
                } else {
                    point(200.0, 20.0)
                };
                vec![before[0], turn, before[4], before[5]]
            } else {
                before
            };
            assert_eq!(
                layout.edges[0].points, expected,
                "{obstruction}, transpose {transpose}"
            );
        }
    }
}

#[test]
fn rejects_an_obstacle_that_contains_the_new_bend() {
    let path = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (20.0, 10.0),
        (20.0, 0.0),
    ]
    .map(|(x, y)| point(x, y))
    .to_vec();
    let (graph, mut layout) = routed_graph(
        vec![
            node("a", 1_000.0, 1_000.0, 10.0, 10.0),
            node("b", 2_000.0, 2_000.0, 10.0, 10.0),
            node("obstacle", 16.0, -4.0, 8.0, 8.0),
        ],
        vec![edge("e", "a", "b")],
        vec![EdgeGeometry {
            id: "e".into(),
            points: path.clone(),
            label: None,
        }],
    );
    simplify_routes(&graph, &mut layout).unwrap();
    assert_eq!(layout.edges[0].points, path);
}

#[test]
fn exhausting_the_budget_keeps_every_route_untouched() {
    let (graph, mut layout) = cases("edge", false);
    let before = layout.edges[0].points.clone();
    layout.edges[1].points = vec![point(150.0, 45.0); MAX_WORK];
    assert_eq!(simplify_routes(&graph, &mut layout), Err(Error::WorkLimit));
    assert_eq!(layout.edges[0].points, before);
    assert_eq!(layout.edges[1].points.len(), MAX_WORK);
}
