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
