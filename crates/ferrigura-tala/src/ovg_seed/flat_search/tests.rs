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
    for flavor in flavors {
        let mut history = RouteHistory::new(&graph, &mut work).unwrap();
        let mut scratch = SearchScratch::default();
        for fixture in flavor.routes {
            let edge = edge_for(fixture.owners);
            let path: Vec<_> = fixture
                .points
                .iter()
                .enumerate()
                .map(|(index, &[x, y])| {
                    if index == 0 {
                        return graph.centers_by_node[edge.owners[0]];
                    } else if index == fixture.points.len() - 1 {
                        return graph.centers_by_node[edge.owners[1]];
                    }
                    graph
                        .vertices
                        .iter()
                        .enumerate()
                        .find_map(|(index, vertex)| {
                            (vertex.point == Point { x, y }
                                && !graph.centers_by_node.contains(&index))
                            .then_some(index)
                        })
                        .unwrap()
                })
                .collect();
            let expected = SearchRoute {
                cost: fixture.cost,
                vertices: path,
            };
            let shortcut = search
                .slingshot(&mut history, &edge, &node_ids, &edges, &mut work)
                .unwrap();
            let (policy, found) = if let Some(shortcut) = shortcut {
                slingshot += 1;
                ("slingshot", shortcut)
            } else {
                weighted += 1;
                (
                    "search",
                    search
                        .find(&mut scratch, &mut history, &edge, &mut work)
                        .unwrap()
                        .unwrap(),
                )
            };
            assert_eq!(
                policy, fixture.policy,
                "{} {:?} route policy",
                flavor.name, edge.owners
            );
            assert_eq!(
                found.cost.to_bits(),
                expected.cost.to_bits(),
                "{} {:?} cost: {} vs {}",
                flavor.name,
                edge.owners,
                found.cost,
                expected.cost
            );
            assert_eq!(
                found.vertices, expected.vertices,
                "{} {:?} path",
                flavor.name, edge.owners
            );
            history.accept(edge, found, &mut work).unwrap();
        }
    }
    assert_eq!((weighted, slingshot), (24, 3));
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
