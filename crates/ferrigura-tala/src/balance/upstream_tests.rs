// SPDX-License-Identifier: MPL-2.0
// D2 TALA b463ea5, internal/routing/edge_postprocess_test.go.

use super::balance_routes;
use crate::{Edge, EdgeGeometry, Graph, Layout, Node, NodeGeometry, Point};

type Coordinates = (f64, f64);

struct Route<'a> {
    source: usize,
    target: usize,
    before: &'a [Coordinates],
    after: &'a [Coordinates],
}

fn route<'a>(
    source: usize,
    target: usize,
    before: &'a [Coordinates],
    after: &'a [Coordinates],
) -> Route<'a> {
    Route {
        source,
        target,
        before,
        after,
    }
}

const VERTICAL_PAIR: [[f64; 4]; 2] = [[0.0, 0.0, 1000.0, 1000.0], [0.0, 2000.0, 1000.0, 1000.0]];

fn check(boxes: &[[f64; 4]], routes: &[Route<'_>]) {
    let ids = (0..boxes.len())
        .map(|index| index.to_string())
        .collect::<Vec<_>>();
    let nodes = boxes
        .iter()
        .zip(&ids)
        .map(|(&[_, _, width, height], id)| Node {
            id: id.clone(),
            parent_id: None,
            direction: None,
            is_group: false,
            shape: "rectangle".into(),
            width,
            height,
            label: String::new(),
            label_width: 0.0,
            label_height: 0.0,
        })
        .collect();
    let edges = routes
        .iter()
        .enumerate()
        .map(|(index, route)| Edge {
            id: index.to_string(),
            source: ids[route.source].clone(),
            target: ids[route.target].clone(),
            source_arrow: false,
            target_arrow: false,
            label: String::new(),
            label_width: 0.0,
            label_height: 0.0,
        })
        .collect();
    let graph = Graph {
        direction: None,
        nodes,
        edges,
    };
    let placed = boxes
        .iter()
        .zip(&ids)
        .map(|(&[x, y, width, height], id)| NodeGeometry {
            id: id.clone(),
            x,
            y,
            width,
            height,
        })
        .collect::<Vec<_>>();
    let geometry = |coordinates: for<'a> fn(&Route<'a>) -> &'a [Coordinates]| {
        routes
            .iter()
            .enumerate()
            .map(|(index, route)| EdgeGeometry {
                id: index.to_string(),
                points: coordinates(route)
                    .iter()
                    .map(|&(x, y)| Point { x, y })
                    .collect(),
                label: None,
            })
            .collect()
    };
    let mut layout = Layout {
        nodes: placed.clone(),
        edges: geometry(|route| route.before),
    };
    balance_routes(&graph, &mut layout).unwrap();
    assert_eq!(layout.nodes, placed);
    assert_eq!(layout.edges, geometry(|route| route.after));
}

#[test]
fn simple_single() {
    check(
        &VERTICAL_PAIR,
        &[route(
            0,
            1,
            &[(1.0, 1000.0), (1.0, 2000.0)],
            &[(500.0, 1000.0), (500.0, 2000.0)],
        )],
    );
}

#[test]
fn simple_double() {
    check(
        &VERTICAL_PAIR,
        &[
            route(
                0,
                1,
                &[(1.0, 1000.0), (1.0, 2000.0)],
                &[(333.0, 1000.0), (333.0, 2000.0)],
            ),
            route(
                0,
                1,
                &[(3.0, 1000.0), (3.0, 2000.0)],
                &[(666.0, 1000.0), (666.0, 2000.0)],
            ),
        ],
    );
}

#[test]
fn simple_double_opposite_directions() {
    check(
        &VERTICAL_PAIR,
        &[
            route(
                0,
                1,
                &[(1.0, 1000.0), (1.0, 2000.0)],
                &[(333.0, 1000.0), (333.0, 2000.0)],
            ),
            route(
                1,
                0,
                &[(3.0, 2000.0), (3.0, 1000.0)],
                &[(666.0, 2000.0), (666.0, 1000.0)],
            ),
        ],
    );
}

#[test]
fn multiple_nodes() {
    check(
        &[
            [0.0, 0.0, 200.0, 1000.0],
            [500.0, 0.0, 200.0, 1000.0],
            [0.0, 2000.0, 1000.0, 1000.0],
        ],
        &[
            route(
                0,
                2,
                &[(1.0, 1000.0), (1.0, 2000.0)],
                &[(100.0, 1000.0), (100.0, 2000.0)],
            ),
            route(
                1,
                2,
                &[(501.0, 1000.0), (501.0, 2000.0)],
                &[(600.0, 1000.0), (600.0, 2000.0)],
            ),
        ],
    );
}

#[test]
fn single_bendy() {
    check(
        &VERTICAL_PAIR,
        &[route(
            0,
            1,
            &[
                (1.0, 1000.0),
                (1.0, 1300.0),
                (300.0, 1300.0),
                (300.0, 2000.0),
            ],
            &[(500.0, 1000.0), (500.0, 1300.0), (500.0, 2000.0)],
        )],
    );
}

#[test]
fn back_to_back() {
    check(
        &[
            [0.0, 0.0, 1000.0, 1000.0],
            [0.0, 2000.0, 1000.0, 1000.0],
            [2000.0, 0.0, 1000.0, 1000.0],
            [2000.0, 2000.0, 1000.0, 1000.0],
        ],
        &[
            route(
                0,
                1,
                &[
                    (1000.0, 500.0),
                    (1450.0, 500.0),
                    (1450.0, 2400.0),
                    (1000.0, 2400.0),
                ],
                &[
                    (1000.0, 500.0),
                    (1333.0, 500.0),
                    (1333.0, 2500.0),
                    (1000.0, 2500.0),
                ],
            ),
            route(
                2,
                3,
                &[
                    (2000.0, 500.0),
                    (1500.0, 500.0),
                    (1500.0, 2500.0),
                    (2000.0, 2500.0),
                ],
                &[
                    (2000.0, 500.0),
                    (1666.0, 500.0),
                    (1666.0, 2500.0),
                    (2000.0, 2500.0),
                ],
            ),
        ],
    );
}

#[test]
fn multiple_ranges() {
    check(
        &[
            [0.0, 0.0, 1000.0, 1000.0],
            [0.0, 6000.0, 1000.0, 1000.0],
            [0.0, 3000.0, 1000.0, 1000.0],
            [0.0, 10000.0, 1000.0, 1000.0],
            [1000.0, 7500.0, 1000.0, 1000.0],
            [5000.0, 0.0, 1000.0, 10000.0],
        ],
        &[
            route(
                0,
                1,
                &[
                    (1000.0, 500.0),
                    (1600.0, 500.0),
                    (1600.0, 6200.0),
                    (1000.0, 6200.0),
                ],
                &[
                    (1000.0, 500.0),
                    (2250.0, 500.0),
                    (2250.0, 6500.0),
                    (1000.0, 6500.0),
                ],
            ),
            route(
                2,
                3,
                &[
                    (1000.0, 3500.0),
                    (2640.0, 3500.0),
                    (2640.0, 10500.0),
                    (1000.0, 10500.0),
                ],
                &[
                    (1000.0, 3500.0),
                    (3500.0, 3500.0),
                    (3500.0, 10500.0),
                    (1000.0, 10500.0),
                ],
            ),
        ],
    );
}

#[test]
fn shared_routes() {
    check(
        &VERTICAL_PAIR,
        &[
            route(
                0,
                1,
                &[(1.0, 1000.0), (1.0, 2000.0)],
                &[(333.0, 1000.0), (333.0, 2000.0)],
            ),
            route(
                0,
                1,
                &[(1.0, 1000.0), (1.0, 2000.0)],
                &[(333.0, 1000.0), (333.0, 2000.0)],
            ),
            route(
                1,
                0,
                &[(3.0, 2000.0), (3.0, 1000.0)],
                &[(666.0, 2000.0), (666.0, 1000.0)],
            ),
        ],
    );
}

#[test]
fn shared_routes_different_ranges() {
    check(
        &[
            [4000.0, 0.0, 3000.0, 1000.0],
            [4500.0, 4000.0, 1000.0, 1000.0],
            [0.0, 2000.0, 1000.0, 1000.0],
        ],
        &[
            route(
                0,
                1,
                &[(4510.0, 1000.0), (4510.0, 4000.0)],
                &[(5000.0, 1000.0), (5000.0, 4000.0)],
            ),
            route(
                0,
                2,
                &[(4510.0, 1000.0), (4510.0, 2500.0), (1000.0, 2500.0)],
                &[(5000.0, 1000.0), (5000.0, 2500.0), (1000.0, 2500.0)],
            ),
        ],
    );
}
