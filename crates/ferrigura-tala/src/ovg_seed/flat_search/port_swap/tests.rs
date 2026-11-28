use super::*;
use crate::ovg_seed::flat_search::Appearance;

fn node(x: f64, y: f64, width: f64, height: f64) -> NodeGeometry {
    NodeGeometry {
        id: String::new(),
        x,
        y,
        width,
        height,
    }
}

fn edge(owners: [usize; 2], arrows: [&str; 2]) -> OrdinaryEdge {
    OrdinaryEdge {
        owners,
        arrowheads: arrows.map(String::from),
        appearance: Appearance::default(),
        invisible: false,
    }
}

fn points(coords: &[[f64; 2]]) -> Vec<Point> {
    coords.iter().map(|&[x, y]| Point { x, y }).collect()
}

fn swap_at(
    nodes: &[NodeGeometry],
    parents: &[Option<usize>],
    edges: &[OrdinaryEdge],
    paths: &mut [Vec<Point>],
    owner: usize,
) -> bool {
    let context = PortSwap {
        nodes,
        edges,
        parents,
    };
    let mut work = usize::MAX;
    let mut groups = PortGroups::at(edges, paths, owner, &mut work).unwrap();
    groups
        .reorder_same_side(&context, owner, paths, &mut work)
        .unwrap()
}

#[test]
fn swap_edge_ports() {
    let nodes = [
        node(0.0, 150.0, 200.0, 100.0),
        node(200.0, 0.0, 50.0, 50.0),
        node(100.0, 175.0, 50.0, 50.0),
    ];
    let edges = [edge([2, 1], ["", ""]), edge([1, 2], ["", ""])];
    let mut routes = [
        points(&[[138.0, 175.0], [138.0, 25.0], [200.0, 25.0]]),
        points(&[[225.0, 50.0], [225.0, 75.0], [113.0, 75.0], [113.0, 175.0]]),
    ];
    assert!(swap_at(
        &nodes,
        &[None, None, Some(0)],
        &edges,
        &mut routes,
        2
    ));
    let expected = [
        points(&[[113.0, 175.0], [113.0, 25.0], [200.0, 25.0]]),
        points(&[[225.0, 50.0], [225.0, 75.0], [138.0, 75.0], [138.0, 175.0]]),
    ];
    assert_eq!(routes, expected);
}

#[test]
fn invalid_swap_node_overlap() {
    let nodes = [
        node(50.0, 0.0, 100.0, 50.0),
        node(200.0, 0.0, 50.0, 50.0),
        node(0.0, 150.0, 200.0, 100.0),
    ];
    let edges = [
        edge([2, 1], ["", ""]),
        edge([1, 2], ["", ""]),
        edge([2, 0], ["", ""]),
    ];
    let before = [
        points(&[[175.0, 150.0], [175.0, 25.0], [200.0, 25.0]]),
        points(&[[225.0, 50.0], [225.0, 75.0], [75.0, 75.0], [75.0, 150.0]]),
        points(&[[125.0, 150.0], [125.0, 50.0]]),
    ];
    let mut routes = before.clone();
    assert!(!swap_at(&nodes, &[None; 3], &edges, &mut routes, 2));
    assert_eq!(routes, before);
}

#[test]
fn invalid_swap_arrowhead() {
    let nodes = [
        node(10.0, 0.0, 50.0, 50.0),
        node(200.0, 0.0, 50.0, 50.0),
        node(0.0, 150.0, 200.0, 100.0),
    ];
    let edges = [
        edge([2, 1], ["none", "triangle"]),
        edge([1, 2], ["none", "triangle"]),
        edge([2, 0], ["triangle", "triangle"]),
    ];
    let before = [
        points(&[[175.0, 150.0], [175.0, 25.0], [200.0, 25.0]]),
        points(&[[225.0, 50.0], [225.0, 75.0], [125.0, 75.0], [125.0, 150.0]]),
        points(&[[125.0, 150.0], [125.0, 25.0], [60.0, 25.0]]),
    ];
    let mut routes = before.clone();
    assert!(!swap_at(&nodes, &[None; 3], &edges, &mut routes, 2));
    assert_eq!(routes, before);
}

#[test]
fn matches_the_pinned_go_adjacent_port_swap_and_straight_refinement() {
    let nodes = [
        node(0.0, 0.0, 40.0, 40.0),
        node(160.0, -48.0, 45.0, 100.0),
        node(60.0, -100.0, 40.0, 40.0),
    ];
    let edges = [edge([0, 1], ["", ""]), edge([0, 2], ["", ""])];
    let original = [
        points(&[[20.0, 0.0], [20.0, -40.0], [160.0, -40.0]]),
        points(&[[40.0, 20.0], [80.0, 20.0], [80.0, -60.0]]),
    ];
    let expected = [
        points(&[[40.0, 5.0], [160.0, 5.0]]),
        points(&[[20.0, 0.0], [20.0, -42.5], [80.0, -42.5], [80.0, -60.0]]),
    ];
    let start = 10_000;
    let mut work = start;
    let mut routes = original.clone();
    swap_ports(&nodes, &[None; 3], &edges, &mut routes, &mut work).unwrap();
    assert_eq!(routes, expected);
    let consumed = start - work;
    let mut reduced = consumed - 1;
    let mut routes = original.clone();
    assert_eq!(
        swap_ports(&nodes, &[None; 3], &edges, &mut routes, &mut reduced),
        Err(Error::WorkLimit)
    );
    assert_eq!(routes, original);
}
