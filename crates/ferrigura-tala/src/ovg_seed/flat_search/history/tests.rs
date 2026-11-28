use super::*;
use crate::ovg_seed::Vertex;

fn graph(points: &[[f64; 2]]) -> LinkedVisibility {
    LinkedVisibility {
        vertices: points
            .iter()
            .map(|&[x, y]| Vertex {
                point: Point { x, y },
                owners: Vec::new(),
            })
            .collect(),
        ports_by_node: Vec::new(),
        centers_by_node: vec![0, points.len() - 1],
        links: Vec::new(),
        tunnel_link_count: 0,
        near_port_owners: vec![Vec::new(); points.len()],
    }
}

fn edge() -> OrdinaryEdge {
    OrdinaryEdge {
        owners: [0, 1],
        arrowheads: [String::new(), String::from("triangle")],
        appearance: Appearance::default(),
        invisible: false,
    }
}

fn route(vertices: &[usize]) -> SearchRoute {
    SearchRoute {
        vertices: vertices.to_vec(),
        cost: 0.0,
    }
}

#[test]
fn route_opposing_colinear() {
    let graph = graph(&[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [200.0, 100.0]]);
    let mut route = AcceptedRoute {
        edge: edge(),
        vertices: vec![0, 1, 2, 3],
        directions: None,
    };
    let cases = [
        ([0.0, 0.0], [100.0, 0.0], [false, true]),
        ([100.0, 0.0], [100.0, 100.0], [false, true]),
        ([190.0, 0.0], [100.0, 0.0], [false, false]),
        ([90.0, 0.0], [120.0, 0.0], [false, true]),
        ([101.0, 0.0], [200.0, 0.0], [false, false]),
        ([0.0, 0.0], [100.0, 100.0], [false, false]),
        ([120.0, 100.0], [300.0, 100.0], [false, false]),
    ];
    let mut work = usize::MAX;
    for ([x1, y1], [x2, y2], expected) in cases {
        let a = Point { x: x1, y: y1 };
        let b = Point { x: x2, y: y2 };
        for ((from, to), expected) in [(a, b), (b, a)].into_iter().zip(expected) {
            assert_eq!(
                route.opposes(&graph, Segment { from, to }, &mut work),
                Ok(expected)
            );
        }
    }
}

#[test]
fn add_route_preserves_order_at_points_and_deduplicates_indexed_segments() {
    let graph = graph(&[
        [5.0, 5.0],
        [5.0, 10.0],
        [15.0, 10.0],
        [15.0, 15.0],
        [20.0, 15.0],
    ]);
    let mut work = usize::MAX;
    let mut history = RouteHistory::new(&graph, &mut work).unwrap();
    assert_eq!(
        history.accept(edge(), route(&[0, 1, 2, 3, 4]), &mut 11),
        Err(Error::WorkLimit)
    );
    assert!(history.routes.is_empty());
    assert!(history.segments.is_empty());
    assert!(history.points.is_empty());
    assert_eq!(
        history.accept(edge(), route(&[0, 1, 1, 4]), &mut work),
        Err(Error::InvalidGraph)
    );
    for _ in 0..2 {
        history
            .accept(edge(), route(&[0, 1, 2, 3, 4]), &mut work)
            .unwrap();
    }
    assert_eq!(history.routes.len(), 2);
    assert_eq!(history.segments, [[1, 2], [2, 3]].into_iter().collect());
    assert_eq!(history.lines.len(), 2);
    assert_eq!(
        history
            .lines
            .each_ref()
            .map(|lines| lines[0].segments.len()),
        [1, 1]
    );
    for index in 0..5 {
        assert_eq!(history.routes_at(index), &[0, 1]);
    }
}
