use super::*;

fn node(x: f64, y: f64, size: f64) -> NodeGeometry {
    NodeGeometry {
        id: String::new(),
        x,
        y,
        width: size,
        height: size,
    }
}

#[test]
fn sort_edges_top_down_left_right() {
    let nodes = [
        node(0.0, 0.0, 1.0),
        node(100.0, 0.0, 1.0),
        node(0.0, 100.0, 1.0),
        node(100.0, 100.0, 1.0),
        node(100.0, 200.0, 1.0),
        node(0.0, 200.0, 1.0),
    ];
    let edges = [[1, 2], [0, 3], [4, 3], [0, 2], [1, 3], [5, 2]];
    let mut work = usize::MAX;
    assert_eq!(
        order_routes(
            Flavor::TopDownLeftRight,
            &nodes,
            &edges,
            &HashSet::new(),
            &mut work
        )
        .unwrap(),
        [3, 1, 0, 4, 5, 2],
    );
}

#[test]
fn sort_edges_preserves_distance_ties() {
    let nodes = [node(0.0, 0.0, 10.0), node(100.0, 0.0, 10.0)];
    let edges = [[0, 1], [0, 1]];
    for flavor in [Flavor::ShortestToLongest, Flavor::LongestToShortest] {
        let mut work = usize::MAX;
        assert_eq!(
            order_routes(flavor, &nodes, &edges, &HashSet::new(), &mut work).unwrap(),
            [0, 1],
        );
    }
}

#[test]
fn sort_edges_preserves_cluster_ties() {
    let nodes = [
        node(0.0, 0.0, 10.0),
        node(100.0, 0.0, 10.0),
        node(200.0, 0.0, 10.0),
    ];
    let edges = [[1, 2], [0, 2], [0, 1]];
    let mut work = usize::MAX;
    assert_eq!(
        order_routes(
            Flavor::Default,
            &nodes,
            &edges,
            &HashSet::from([0]),
            &mut work
        )
        .unwrap(),
        [1, 2, 0],
    );
}

#[test]
fn sort_edges_preserves_nan_distance_tie() {
    let nodes = [
        node(f64::NAN, 0.0, 10.0),
        node(0.0, 0.0, 10.0),
        node(100.0, 0.0, 10.0),
    ];
    let edges = [[0, 2], [1, 2]];
    for flavor in [Flavor::ShortestToLongest, Flavor::LongestToShortest] {
        let mut work = usize::MAX;
        assert_eq!(
            order_routes(flavor, &nodes, &edges, &HashSet::new(), &mut work).unwrap(),
            [0, 1],
        );
    }
}

#[test]
fn matches_pinned_go_quick_routes_and_the_weighted_routes_that_follow() {
    use crate::ovg_seed::flat_search::Weights;
    use crate::ovg_seed::rectangular_linked_visibility;

    let nodes = [
        node(0.0, 0.0, 40.0),
        node(70.0, 0.0, 40.0),
        node(0.0, 145.0, 40.0),
    ];
    let pairs = [[0, 1], [0, 1], [0, 2], [1, 2]];
    let edges = pairs.map(|owners| OrdinaryEdge {
        owners,
        arrowheads: [String::from("none"), String::from("triangle")],
        appearance: Appearance::default(),
        invisible: false,
    });
    let graph = rectangular_linked_visibility(&nodes, &pairs).unwrap();
    let weights = Weights::from_graph(&nodes, &pairs).unwrap();
    let search = FlatSearch::new(&graph, &nodes, &[None, None, None], weights).unwrap();
    let mut work = usize::MAX;
    let chosen = search
        .route_flavor(Flavor::Default, &[1, 2, 3], &edges, &mut work)
        .unwrap()
        .unwrap();
    let expected: &[&[[f64; 2]]] = &[
        &[[20.0, 20.0], [40.0, 20.0], [70.0, 20.0], [90.0, 20.0]],
        &[[20.0, 20.0], [40.0, 10.0], [70.0, 10.0], [90.0, 20.0]],
        &[[20.0, 20.0], [20.0, 40.0], [20.0, 145.0], [20.0, 165.0]],
        &[
            [90.0, 20.0],
            [80.0, 40.0],
            [80.0, 92.5],
            [80.0, 140.0],
            [80.0, 155.0],
            [40.0, 155.0],
            [20.0, 165.0],
        ],
    ];
    assert_eq!(chosen.routes.len(), expected.len());
    assert_eq!(chosen.cost.to_bits(), 416.056_989_549_952_16_f64.to_bits());
    assert_eq!(
        chosen
            .routes
            .iter()
            .map(|route| route.strategy)
            .collect::<Vec<_>>(),
        [
            RouteStrategy::Quick,
            RouteStrategy::Quick,
            RouteStrategy::Weighted,
            RouteStrategy::Slingshot
        ]
    );
    for (route, points) in chosen.routes.iter().zip(expected) {
        let path: Vec<_> = search
            .raw_route_points(&chosen, &route.route)
            .unwrap()
            .into_iter()
            .map(|point| [point.x, point.y])
            .collect();
        assert_eq!(&path, points);
    }
}

#[test]
fn matches_pinned_go_line_fallback_on_a_disconnected_visibility_graph() {
    use crate::ovg_seed::flat_search::Weights;
    use crate::ovg_seed::rectangular_linked_visibility;

    let pairs = [[0, 1]];
    let edges = pairs.map(|owners| OrdinaryEdge {
        owners,
        arrowheads: Default::default(),
        appearance: Appearance::default(),
        invisible: false,
    });
    let cases = [
        (
            [200.0, 0.0],
            160.0_f64,
            [[20.0, 20.0], [40.0, 20.0], [200.0, 20.0], [220.0, 20.0]],
        ),
        (
            [21.0, 15.0],
            25.806_975_801_127_88_f64,
            [[20.0, 20.0], [40.0, 34.0], [41.0, 35.0], [41.0, 35.0]],
        ),
    ];
    for ([x, y], cost, expected) in cases {
        let nodes = [node(0.0, 0.0, 40.0), node(x, y, 40.0)];
        let mut graph = rectangular_linked_visibility(&nodes, &pairs).unwrap();
        // The reference test removes adjacency after positioning ports.
        graph.links.clear();
        graph.tunnel_link_count = 0;
        let weights = Weights::from_graph(&nodes, &pairs).unwrap();
        let search = FlatSearch::new(&graph, &nodes, &[None, None], weights).unwrap();
        let mut work = usize::MAX;
        let chosen = search
            .route_flavor(Flavor::Default, &[1, 2], &edges, &mut work)
            .unwrap()
            .unwrap();
        assert_eq!(chosen.cost.to_bits(), cost.to_bits());
        let route = &chosen.routes[0];
        assert_eq!(route.strategy, RouteStrategy::Fallback);
        let path: Vec<_> = search
            .raw_route_points(&chosen, &route.route)
            .unwrap()
            .into_iter()
            .map(|point| [point.x, point.y])
            .collect();
        assert_eq!(path, expected);
        let points: Vec<_> = search.route_points_in_input_order(&chosen).unwrap()[0]
            .iter()
            .map(|point| [point.x, point.y])
            .collect();
        assert_eq!(points, expected[1..3]);
    }
}
