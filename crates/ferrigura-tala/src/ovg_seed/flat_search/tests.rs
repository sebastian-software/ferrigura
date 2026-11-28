use super::*;

#[derive(serde::Deserialize)]
struct Positioned {
    nodes: Vec<NodeGeometry>,
    edges: Vec<[usize; 2]>,
}

#[test]
fn matches_pinned_go_first_observatory_routes_without_captured_transition_costs() {
    let Positioned { nodes, edges } =
        serde_json::from_str(include_str!("../search/observatory.json")).unwrap();
    let graph = super::super::rectangular_linked_visibility(&nodes, &edges).unwrap();
    let weights = Weights::from_graph(&nodes, &edges).unwrap();
    assert_eq!(
        weights,
        Weights {
            turn: 247.5,
            crossing: 218.97216,
            non_center_port: 84.89249999999998
        }
    );
    let labels = vec![None; nodes.len()];
    let search = FlatSearch::new(&graph, &nodes, &labels, weights).unwrap();
    let mut work = usize::MAX;
    type Case = ([usize; 2], f64, &'static [(f64, f64)]);
    let cases: [Case; 2] = [
        // Default and ShortestToLongest both start with control -> antenna.
        (
            [0, 1],
            536.3924999999999,
            &[
                (1324.0, 1176.0),
                (1324.0, 1210.0),
                (1324.0, 1237.0),
                (1362.0, 1237.0),
                (1386.0, 1237.0),
                (1410.0, 1237.0),
                (1448.0, 1237.0),
                (1496.0, 1237.0),
                (1572.0, 1220.0),
            ],
        ),
        // LongestToShortest starts with antenna -> sampler.
        (
            [1, 4],
            576.5,
            &[
                (1572.0, 1220.0),
                (1496.0, 1220.0),
                (1448.0, 1220.0),
                (1200.0, 1220.0),
                (1200.0, 1248.0),
                (1200.0, 1268.5),
            ],
        ),
    ];
    for (owners, expected_cost, expected_path) in cases {
        let route = search
            .find_first(&mut SearchScratch::default(), owners, &mut work)
            .unwrap()
            .unwrap();
        assert_eq!(route.cost.to_bits(), expected_cost.to_bits(), "{owners:?}");
        assert_eq!(
            route
                .vertices
                .iter()
                .map(|&index| {
                    let point = graph.vertices[index].point;
                    (point.x, point.y)
                })
                .collect::<Vec<_>>(),
            expected_path,
            "{owners:?}",
        );
    }
}

#[test]
fn segment_box_distinguishes_corner_contacts_inside_links() {
    let bounds = Rectangle {
        x: 10.0,
        y: 10.0,
        width: 10.0,
        height: 10.0,
    };
    assert!(bounds.intersects(Point { x: 0.0, y: 10.0 }, Point { x: 30.0, y: 10.0 }));
    assert!(!bounds.intersects(Point { x: 0.0, y: 20.0 }, Point { x: 20.0, y: 0.0 }));
    assert!(bounds.intersects(Point { x: 0.0, y: 20.0 }, Point { x: 10.0, y: 10.0 }));
}

#[test]
fn shared_directions_allow_either_valid_side_and_tunnels_remain_unrestricted() {
    let merged = PortOwner {
        node: 0,
        sides: 1 | 2,
        center: true,
    };
    let unrestricted = PortOwner {
        node: 0,
        sides: 0,
        center: false,
    };
    let port = Point { x: 10.0, y: 10.0 };
    assert!(port_faces(&merged, port, Point { x: 10.0, y: 5.0 }, false));
    assert!(port_faces(&merged, port, Point { x: 5.0, y: 10.0 }, false));
    assert!(!port_faces(
        &merged,
        port,
        Point { x: 15.0, y: 10.0 },
        false
    ));
    assert!(port_faces(&merged, port, Point { x: 15.0, y: 10.0 }, true));
    assert!(port_faces(&unrestricted, port, port, false));
}

#[derive(serde::Deserialize)]
struct FlavorFixture {
    name: String,
    routes: Vec<RouteFixture>,
}

#[derive(serde::Deserialize)]
struct RouteFixture {
    owners: [usize; 2],
    policy: String,
    cost: f64,
    points: Vec<[f64; 2]>,
}

#[test]
fn matches_every_pinned_go_raw_observatory_route() {
    let Positioned { nodes, edges } =
        serde_json::from_str(include_str!("../search/observatory.json")).unwrap();
    let graph = super::super::rectangular_linked_visibility(&nodes, &edges).unwrap();
    let weights = Weights::from_graph(&nodes, &edges).unwrap();
    let labels = vec![None; nodes.len()];
    let search = FlatSearch::new(&graph, &nodes, &labels, weights).unwrap();
    let flavors: Vec<FlavorFixture> =
        serde_json::from_str(include_str!("observatory-routes.json")).unwrap();
    let mut work = usize::MAX;
    let node_ids = [
        3749328226, 1739408958, 1831990113, 3347641530, 3327259844, 3835906992, 1421265581,
    ];
    let edge_for = |owners| OrdinaryEdge {
        owners,
        arrowheads: [String::from("none"), String::from("triangle")],
        appearance: Appearance::default(),
        invisible: false,
    };
    let edges: Vec<_> = edges.into_iter().map(edge_for).collect();
    let mut weighted = 0;
    let mut slingshot = 0;
    for fixture in flavors {
        let flavor = match fixture.name.as_str() {
            "Default" => Flavor::Default,
            "ShortestToLongest" => Flavor::ShortestToLongest,
            "LongestToShortest" => Flavor::LongestToShortest,
            name => panic!("unknown reference flavor {name}"),
        };
        let native = search
            .route_flavor(flavor, &node_ids, &edges, &mut work)
            .unwrap()
            .unwrap();
        assert_eq!(native.routes.len(), fixture.routes.len());
        for (choice, expected) in native.routes.into_iter().zip(fixture.routes) {
            let owners = edges[choice.edge].owners;
            assert_eq!(owners, expected.owners, "{}", fixture.name);
            let strategy = match choice.strategy {
                RouteStrategy::Slingshot => {
                    slingshot += 1;
                    "slingshot"
                }
                RouteStrategy::Quick => "quick",
                RouteStrategy::Fallback => "line",
                RouteStrategy::Weighted => {
                    weighted += 1;
                    "search"
                }
            };
            assert_eq!(
                strategy, expected.policy,
                "{} {:?} route policy",
                fixture.name, owners
            );
            let path: Vec<_> = choice
                .route
                .vertices
                .iter()
                .map(|&vertex| {
                    let point = graph.vertices[vertex].point;
                    [point.x, point.y]
                })
                .collect();
            assert_eq!(path, expected.points, "{} {:?} path", fixture.name, owners);
            assert_eq!(
                choice.route.cost.to_bits(),
                expected.cost.to_bits(),
                "{} {:?} raw cost",
                fixture.name,
                owners
            );
        }
    }
    assert_eq!((weighted, slingshot), (24, 3));

    let selected = search
        .select_flavor(&node_ids, &edges, &mut work)
        .unwrap()
        .unwrap();
    assert_eq!(selected.flavor, Flavor::LongestToShortest);
    assert_eq!(selected.cost.to_bits(), 3_815.364_66_f64.to_bits());
    let expected: &[([usize; 2], &[[f64; 2]])] = &[
        (
            [1, 4],
            &[[1496.0, 1220.0], [1200.0, 1220.0], [1200.0, 1248.0]],
        ),
        (
            [4, 5],
            &[[1124.0, 1269.0], [1076.0, 1269.0], [1076.0, 1434.0]],
        ),
        (
            [3, 1],
            &[[1524.0, 1021.0], [1572.0, 1021.0], [1572.0, 1186.0]],
        ),
        ([2, 6], &[[1324.0, 1378.0], [1324.0, 1496.0]]),
        (
            [5, 2],
            &[[1114.0, 1434.0], [1114.0, 1361.0], [1248.0, 1361.0]],
        ),
        (
            [0, 3],
            &[[1324.0, 1142.0], [1324.0, 1021.0], [1372.0, 1021.0]],
        ),
        ([2, 0], &[[1324.0, 1310.0], [1324.0, 1210.0]]),
        (
            [5, 6],
            &[[1076.0, 1502.0], [1076.0, 1530.0], [1248.0, 1530.0]],
        ),
        (
            [0, 1],
            &[[1400.0, 1159.0], [1572.0, 1159.0], [1572.0, 1186.0]],
        ),
    ];
    let before = search.route_points_in_input_order(&selected).unwrap();
    let mut after = before.clone();
    swap_ports(&nodes, &[None; 7], &edges, &mut after, &mut work).unwrap();
    assert_eq!(
        after, before,
        "Go leaves all Observatory routes unchanged at port swaps"
    );
    search
        .straight_edges_fallback(&edges, &mut after, &mut work)
        .unwrap();
    assert_eq!(
        after, before,
        "Go leaves all Observatory routes unchanged at straight fallback"
    );
    let materialized = search
        .materialize_selected_routes(&selected, &edges, &mut work)
        .unwrap();
    assert_eq!(materialized.len(), expected.len());
    for (route, (owners, vertices)) in materialized.into_iter().zip(expected) {
        assert_eq!(&edges[route.edge].owners, owners);
        let points: Vec<_> = route
            .points
            .into_iter()
            .map(|point| [point.x, point.y])
            .collect();
        assert_eq!(&points, vertices, "{owners:?}");
    }
}

fn attempt_flight(rectangles: &[(f64, f64, f64, f64)]) -> Option<(Vec<Point>, f64)> {
    let nodes: Vec<_> = rectangles
        .iter()
        .enumerate()
        .map(|(i, &(x, y, width, height))| NodeGeometry {
            id: i.to_string(),
            x,
            y,
            width,
            height,
        })
        .collect();
    let edges = [[0, 1]];
    let edge = OrdinaryEdge {
        owners: [0, 1],
        arrowheads: Default::default(),
        appearance: Appearance::default(),
        invisible: false,
    };
    let graph = super::super::rectangular_linked_visibility(&nodes, &edges).unwrap();
    let weights = Weights::from_graph(&nodes, &edges).unwrap();
    let labels = vec![None; nodes.len()];
    let search = FlatSearch::new(&graph, &nodes, &labels, weights).unwrap();
    let mut work = usize::MAX;
    let mut history = RouteHistory::new(&graph, &mut work).unwrap();
    let node_ids = [1, 2, 3, 4, 5];
    search
        .slingshot(
            &mut history,
            &edge,
            &node_ids[..nodes.len()],
            std::slice::from_ref(&edge),
            &mut work,
        )
        .unwrap()
        .map(|route| {
            let path = route
                .vertices
                .into_iter()
                .map(|index| graph.vertices[index].point)
                .collect();
            (path, route.cost)
        })
}

#[test]
fn slingshot() {
    let rectangles = [
        (0.0, 0.0, 40.0, 40.0),
        (100.0, 100.0, 40.0, 40.0),
        (80.0, 0.0, 10.0, 40.0),
        (0.0, 80.0, 10.0, 40.0),
        (0.0, 20.0, 1000.0, 10.0),
    ];
    let run = |count| attempt_flight(&rectangles[..count]).map_or(f64::INFINITY, |(_, cost)| cost);
    let isolated = run(2);
    let one_side_blocked = run(3);
    let both_sides_blocked = run(4);
    assert_eq!(isolated.to_bits(), 150.606_601_717_798_2_f64.to_bits());
    assert_eq!(one_side_blocked, isolated);
    assert!(both_sides_blocked > isolated);
    assert!(both_sides_blocked > one_side_blocked);
    // The pinned Go fixture returns no route here, satisfying its
    // original greater-than assertions.
    assert_eq!(both_sides_blocked, f64::INFINITY);
    assert_eq!(run(5), f64::INFINITY);
}

#[test]
fn matches_the_pinned_go_s_shaped_flight() {
    let rectangles = [
        (0.0, 0.0, 40.0, 40.0),
        (240.0, 150.0, 40.0, 40.0),
        (160.0, 0.0, 33.0, 48.0),
        (80.0, 40.0, 39.0, 39.0),
        (170.0, 140.0, 25.0, 66.0),
    ];
    let (path, cost) = attempt_flight(&rectangles).unwrap();
    let expected = [
        (20.0, 20.0),
        (30.0, 40.0),
        (30.0, 69.0),
        (30.0, 95.0),
        (140.0, 95.0),
        (250.0, 95.0),
        (250.0, 150.0),
        (260.0, 170.0),
    ];
    assert_eq!(path, expected.map(|(x, y)| Point { x, y }));
    assert_eq!(cost.to_bits(), 387.063_561_052_566_6_f64.to_bits());
}
