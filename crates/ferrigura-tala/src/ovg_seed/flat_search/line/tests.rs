use super::*;
use crate::ovg_seed::flat_search::{Appearance, Weights};
use crate::ovg_seed::rectangular_linked_visibility;

fn node(x: f64, y: f64, width: f64, height: f64) -> NodeGeometry {
    NodeGeometry {
        id: String::new(),
        x,
        y,
        width,
        height,
    }
}

fn edge(owners: [usize; 2], directed: bool) -> OrdinaryEdge {
    OrdinaryEdge {
        owners,
        arrowheads: [
            String::new(),
            if directed {
                String::from("triangle")
            } else {
                String::new()
            },
        ],
        appearance: Appearance::default(),
        invisible: false,
    }
}

fn solve(
    nodes: &[NodeGeometry],
    candidate: &OrdinaryEdge,
    existing: &[ExistingLine<'_>],
) -> Option<StraightRoute> {
    let mut pairs = vec![candidate.owners];
    pairs.extend(existing.iter().map(|line| line.edge.owners));
    let graph = rectangular_linked_visibility(nodes, &pairs).unwrap();
    let weights = Weights::from_graph(nodes, &pairs).unwrap();
    let labels = vec![None; nodes.len()];
    let mut work = usize::MAX;
    FlatSearch::new(&graph, nodes, &labels, weights)
        .unwrap()
        .straight_line(candidate, existing, &mut work)
        .unwrap()
}

#[test]
fn route_line_returns_ports() {
    let nodes = [node(0.0, 0.0, 40.0, 40.0), node(200.0, 0.0, 40.0, 40.0)];
    let route = solve(&nodes, &edge([0, 1], false), &[]).unwrap();
    assert_eq!(
        route.ports,
        [Point { x: 40.0, y: 20.0 }, Point { x: 200.0, y: 20.0 }]
    );
    assert_eq!(route.cost, 160.0);
}

#[test]
fn route_line_ignores_non_overlapping_edges_when_checking_shared_routes() {
    let nodes = [
        node(0.0, 0.0, 100.0, 100.0),
        node(300.0, 0.0, 100.0, 100.0),
        node(200.0, 300.0, 100.0, 100.0),
        node(0.0, 500.0, 100.0, 100.0),
        node(300.0, 500.0, 100.0, 100.0),
    ];
    let candidate = edge([0, 1], true);
    let shared = edge([0, 2], true);
    let unrelated = edge([3, 4], false);
    let existing = [
        ExistingLine {
            edge: &shared,
            points: &[
                Point { x: 100.0, y: 50.0 },
                Point { x: 250.0, y: 50.0 },
                Point { x: 250.0, y: 300.0 },
            ],
        },
        ExistingLine {
            edge: &unrelated,
            points: &[Point { x: 100.0, y: 550.0 }, Point { x: 300.0, y: 550.0 }],
        },
    ];
    let route = solve(&nodes, &candidate, &existing).unwrap();
    assert_eq!(
        route.ports,
        [Point { x: 100.0, y: 50.0 }, Point { x: 300.0, y: 50.0 }]
    );
    assert_eq!(route.cost, 200.0);
}

#[test]
fn directly_facing_ports_can_all_be_obstructed() {
    let nodes = [
        node(0.0, 0.0, 40.0, 40.0),
        node(70.0, 0.0, 40.0, 40.0),
        node(45.0, -20.0, 20.0, 80.0),
    ];
    assert!(solve(&nodes, &edge([0, 1], false), &[]).is_none());
}

#[test]
fn straight_edges_fallback_commits_successful_replacement_and_publishes_atomically() {
    let nodes = [
        node(0.0, 0.0, 40.0, 40.0),
        node(200.0, 0.0, 40.0, 40.0),
        node(0.0, 300.0, 40.0, 40.0),
        node(200.0, 300.0, 40.0, 40.0),
    ];
    let pairs = [[0, 1], [2, 3]];
    let edges = pairs.map(|pair| edge(pair, false));
    let graph = rectangular_linked_visibility(&nodes, &pairs).unwrap();
    let weights = Weights::from_graph(&nodes, &pairs).unwrap();
    let search = FlatSearch::new(&graph, &nodes, &[None; 4], weights).unwrap();
    let original = [0.0, 300.0].map(|y| {
        vec![
            Point {
                x: 40.0,
                y: y + 20.0,
            },
            Point {
                x: 40.0,
                y: y + 100.0,
            },
            Point {
                x: 200.0,
                y: y + 100.0,
            },
            Point {
                x: 200.0,
                y: y + 20.0,
            },
        ]
    });
    let mut paths = original.clone();
    let start = 100_000;
    let mut work = start;
    search
        .straight_edges_fallback(&edges, &mut paths, &mut work)
        .unwrap();
    for (path, old) in paths.iter().zip(&original) {
        assert_eq!(path.as_slice(), [old[0], old[3]]);
    }
    let consumed = start - work;
    assert!(consumed > 0);
    for mut limit in [0, consumed / 2, consumed - 1] {
        let mut paths = original.clone();
        assert_eq!(
            search.straight_edges_fallback(&edges, &mut paths, &mut limit),
            Err(Error::WorkLimit)
        );
        assert_eq!(paths, original);
    }
}

#[test]
fn straight_edge_fallback_preserves_candidate_rejection() {
    let nodes = [node(0.0, 0.0, 40.0, 40.0), node(0.0, 0.0, 40.0, 40.0)];
    let pairs = [[0, 1]];
    let edges = pairs.map(|pair| edge(pair, false));
    let graph = rectangular_linked_visibility(&nodes, &pairs).unwrap();
    let weights = Weights::from_graph(&nodes, &pairs).unwrap();
    let search = FlatSearch::new(&graph, &nodes, &[None; 2], weights).unwrap();
    let original = [vec![Point { x: 0.0, y: 20.0 }, Point { x: 40.0, y: 20.0 }]];
    let mut paths = original.clone();
    let mut work = usize::MAX;
    search
        .straight_edges_fallback(&edges, &mut paths, &mut work)
        .unwrap();
    assert_eq!(paths, original);
}
