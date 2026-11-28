use super::*;
use crate::ovg_seed::{LinkedVisibility, Vertex};

fn rectangle(x: f64, y: f64) -> NodeGeometry {
    NodeGeometry {
        id: String::new(),
        x,
        y,
        width: 20.0,
        height: 20.0,
    }
}

fn graph(points: &[(f64, f64)], links: &[[usize; 2]]) -> LinkedVisibility {
    LinkedVisibility {
        vertices: points
            .iter()
            .map(|&(x, y)| Vertex {
                point: Point { x, y },
                owners: Vec::new(),
            })
            .collect(),
        ports_by_node: Vec::new(),
        centers_by_node: Vec::new(),
        links: links.to_vec(),
        tunnel_link_count: 0,
        near_port_owners: vec![Vec::new(); points.len()],
    }
}

#[test]
fn ideal_turn_axes_owns_route_turn_policy() {
    let source = rectangle(100.0, 100.0);
    let target = rectangle(0.0, 0.0);
    assert_eq!(
        ideal_turn_axes(&source, &target),
        [TurnAxis::X(60.0), TurnAxis::Y(60.0)]
    );
    assert!(ideal_turn_axes(&source, &rectangle(105.0, 105.0)).is_empty());
}

#[test]
fn pops_the_vertical_source_before_the_horizontal_source() {
    let graph = graph(&[(0.0, 0.0), (0.0, 10.0)], &[[0, 1]]);
    let adjacency = VisibilityAdjacency::new(&graph).unwrap();
    let turns = TurnPolicy {
        axes: &[],
        cost: 10.0,
        even_arranged_source_cluster: false,
    };
    let mut scratch = SearchScratch::default();
    let mut transitions = Vec::new();
    let mut work = usize::MAX;
    let found = scratch
        .find(&adjacency, [0, 1], &turns, &mut work, |transition, _| {
            transitions.push(transition.from_horizontal);
            Ok(Some(1.0))
        })
        .unwrap()
        .unwrap();
    assert_eq!(transitions, [false, true]);
    assert_eq!(
        found,
        SearchRoute {
            vertices: vec![0, 1],
            cost: 1.0
        }
    );
}

#[test]
fn reuses_search_state_after_failure() {
    let graph = graph(&[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0)], &[[0, 1], [1, 2]]);
    let adjacency = VisibilityAdjacency::new(&graph).unwrap();
    let turns = TurnPolicy {
        axes: &[],
        cost: 10.0,
        even_arranged_source_cluster: false,
    };
    let mut scratch = SearchScratch::default();
    let cost = |_: Transition, _: &mut usize| Ok(Some(1.0));
    let mut limited_work = 25;
    assert_eq!(
        scratch.find(&adjacency, [0, 2], &turns, &mut limited_work, cost),
        Err(Error::WorkLimit)
    );
    let mut work = usize::MAX;
    assert_eq!(
        scratch.find(&adjacency, [2, 0], &turns, &mut work, cost),
        Ok(Some(SearchRoute {
            vertices: vec![2, 1, 0],
            cost: 2.0
        }))
    );
    assert_eq!(
        scratch.find(&adjacency, [0, 2], &turns, &mut work, |_, _| Ok(None)),
        Ok(None)
    );
}

type Coordinate = (f64, f64, bool);
type RecordedStep = (
    Coordinate,
    Coordinate,
    Option<Coordinate>,
    bool,
    Option<f64>,
);

#[derive(serde::Deserialize)]
struct SearchFixture {
    name: String,
    nodes: Vec<NodeGeometry>,
    edges: Vec<[usize; 2]>,
    turn: f64,
    cost: f64,
    path: Vec<Coordinate>,
    steps: Vec<RecordedStep>,
}

fn replay(fixture: &SearchFixture) {
    let linked =
        crate::ovg_seed::rectangular_linked_visibility(&fixture.nodes, &fixture.edges).unwrap();
    let [source, target] =
        [fixture.path[0], *fixture.path.last().unwrap()].map(|(x, y, center)| {
            assert!(center);
            linked
                .centers_by_node
                .iter()
                .position(|&index| {
                    let point = linked.vertices[index].point;
                    (point.x, point.y) == (x, y)
                })
                .unwrap()
        });
    let endpoints = [
        linked.centers_by_node[source],
        linked.centers_by_node[target],
    ];
    let adjacency = VisibilityAdjacency::new(&linked).unwrap();
    let axes = ideal_turn_axes(&fixture.nodes[source], &fixture.nodes[target]);
    let turns = TurnPolicy {
        axes: &axes,
        cost: fixture.turn,
        even_arranged_source_cluster: false,
    };
    let mut work = usize::MAX;
    let mut steps = fixture.steps.iter().enumerate();
    let graph_centers = linked
        .centers_by_node
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let coordinate = |index: usize| {
        let point = adjacency.vertex(index).point;
        (point.x, point.y, graph_centers.contains(&index))
    };
    let route = SearchScratch::default()
        .find(&adjacency, endpoints, &turns, &mut work, |transition, _| {
            let (step, &(from, to, previous, horizontal, cost)) = steps
                .next()
                .unwrap_or_else(|| panic!("{}: extra native transition", fixture.name));
            let actual = (
                coordinate(transition.from),
                coordinate(transition.to),
                transition.previous.map(coordinate),
                transition.from_horizontal,
            );
            assert_eq!(
                actual,
                (from, to, previous, horizontal),
                "{}: transition {step}",
                fixture.name
            );
            Ok(cost)
        })
        .unwrap()
        .unwrap();
    assert!(
        steps.next().is_none(),
        "{}: missing native transitions",
        fixture.name
    );
    assert_eq!(
        route.cost.to_bits(),
        fixture.cost.to_bits(),
        "{}",
        fixture.name
    );
    assert_eq!(
        route
            .vertices
            .into_iter()
            .map(coordinate)
            .collect::<Vec<_>>(),
        fixture.path,
        "{}",
        fixture.name
    );
}

#[test]
fn matches_pinned_go_observatory_search() {
    let fixture: SearchFixture = serde_json::from_str(include_str!("observatory.json")).unwrap();
    replay(&fixture);
}
