use super::*;
use crate::NodeGeometry;
use crate::ovg_seed::flat_search::{Appearance, Flavor, RouteChoice, RouteStrategy, Weights};
use crate::ovg_seed::{rectangular_linked_visibility, search::SearchRoute};

fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn matches_pinned_go_selected_parallel_routes_with_opposite_directions() {
    let nodes = [(0.0, 0.0), (300.0, 0.0), (0.0, 300.0)].map(|(x, y)| NodeGeometry {
        id: String::new(),
        x,
        y,
        width: 100.0,
        height: 100.0,
    });
    let pairs = [[0, 1], [1, 0], [0, 2]];
    let edges = pairs.map(|owners| OrdinaryEdge {
        owners,
        arrowheads: Default::default(),
        appearance: Appearance::default(),
        invisible: false,
    });
    let graph = rectangular_linked_visibility(&nodes, &pairs).unwrap();
    let weights = Weights::from_graph(&nodes, &pairs).unwrap();
    let search = FlatSearch::new(&graph, &nodes, &[None; 3], weights).unwrap();

    let raw: [&[(f64, f64)]; 3] = [
        &[(50.0, 50.0), (100.0, 75.0), (300.0, 75.0), (350.0, 50.0)],
        &[(350.0, 50.0), (300.0, 25.0), (100.0, 25.0), (50.0, 50.0)],
        &[
            (50.0, 50.0),
            (100.0, 75.0),
            (125.0, 75.0),
            (125.0, 320.0),
            (100.0, 320.0),
            (50.0, 350.0),
        ],
    ];
    for shared in [false, true] {
        let mut chosen = FlavorRoutes {
            flavor: Flavor::Default,
            cost: 0.0,
            routes: Vec::new(),
            detached: Vec::new(),
        };
        let length = if shared { 3 } else { 2 };
        for (edge, coords) in raw[..length].iter().enumerate() {
            let first = graph.vertices.len() + chosen.detached.len();
            chosen
                .detached
                .extend(coords.iter().map(|&(x, y)| point(x, y)));
            chosen.routes.push(RouteChoice {
                edge,
                strategy: RouteStrategy::Weighted,
                route: SearchRoute {
                    vertices: (first..first + coords.len()).collect(),
                    cost: 0.0,
                },
            });
        }
        let mut work = usize::MAX;
        let actual = search
            .materialize_selected_routes(&chosen, &edges[..length], &mut work)
            .unwrap();
        let expected = if shared {
            vec![
                MaterializedRoute {
                    edge: 0,
                    points: vec![point(100.0, 75.0), point(300.0, 75.0)],
                },
                MaterializedRoute {
                    edge: 1,
                    points: vec![point(300.0, 25.0), point(100.0, 25.0)],
                },
                MaterializedRoute {
                    edge: 2,
                    points: vec![
                        point(100.0, 75.0),
                        point(125.0, 75.0),
                        point(125.0, 320.0),
                        point(100.0, 320.0),
                    ],
                },
            ]
        } else {
            vec![
                MaterializedRoute {
                    edge: 1,
                    points: vec![point(300.0, 75.0), point(100.0, 75.0)],
                },
                MaterializedRoute {
                    edge: 0,
                    points: vec![point(100.0, 25.0), point(300.0, 25.0)],
                },
            ]
        };
        assert_eq!(actual, expected);
    }
}
