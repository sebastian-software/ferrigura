// SPDX-License-Identifier: MPL-2.0
// These cases come from internal/placement/sized_optimizer_test.go at the D2 pin.

use super::*;

fn node(id: usize, x: f64, y: f64, size: f64) -> NodeGeometry {
    NodeGeometry {
        id: id.to_string(),
        x,
        y,
        width: size,
        height: size,
    }
}

fn cross() -> Vec<NodeGeometry> {
    [
        (100.0, 0.0),
        (0.0, 100.0),
        (200.0, 100.0),
        (100.0, 200.0),
        (0.0, 0.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (x, y))| node(id + 1, x, y, 50.0))
    .collect()
}

#[test]
fn median_point_uses_the_neighbors_measured_centers() {
    let nodes = cross();
    let median = neighbor_median(
        &nodes[4],
        &nodes,
        &mut [0, 1, 2, 3],
        50.0,
        0.0,
        &mut GoRand::new(1),
    );
    assert_eq!(median, Point { x: 125.0, y: 125.0 });
}

#[test]
fn visits_placements_touching_the_offset_point() {
    let nodes = [node(1, 0.0, 0.0, 10.0)];
    let search = CandidateSearch {
        bodies: &nodes,
        index: 0,
        cell: 2.0,
        gaps: &[Spacing::default()],
    };
    let mut candidates = Vec::new();
    search
        .around(5, 5, |point| {
            candidates.push(point);
            Ok(false)
        })
        .unwrap();
    for at in [0.0, 2.0, 4.0, 6.0, 8.0, 10.0] {
        for (x, y) in [(0.0, at), (10.0, at), (at, 0.0), (at, 10.0)] {
            assert!(candidates.contains(&Point { x, y }));
        }
    }
    assert_eq!(candidates.len(), 36);
}

fn cross_search<'a>(nodes: &'a [NodeGeometry], gaps: &'a [Spacing]) -> CandidateSearch<'a> {
    CandidateSearch {
        bodies: nodes,
        index: 4,
        cell: 50.0,
        gaps: &gaps[..nodes.len()],
    }
}

#[test]
fn finds_the_closest_unoccupied_distance_with_connected_spacing() {
    let mut nodes = cross();
    let mut gaps = [Spacing::default(); 6];
    for spacing in &mut gaps[..4] {
        spacing.edge_count = 1;
    }
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let search = cross_search(&nodes, &gaps);
    for (x, y, distance) in [(0.0, 0.0, 0), (100.0, 100.0, 4), (100.0, 200.0, 3)] {
        assert_eq!(
            search.nearest_distance(Point { x, y }, &mut work),
            Ok(distance)
        );
    }
    nodes.push(node(6, 100.0, 100.0, 50.0));
    let search = cross_search(&nodes, &gaps);
    assert_eq!(
        search.nearest_distance(Point { x: 100.0, y: 100.0 }, &mut work),
        Ok(4)
    );
}

#[test]
fn finds_the_closest_unoccupied_distance_without_connections() {
    let mut nodes = cross();
    let gaps = [Spacing::default(); 6];
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let search = cross_search(&nodes, &gaps);
    for (x, y, distance) in [(0.0, 0.0, 0), (100.0, 100.0, 0), (100.0, 200.0, 1)] {
        assert_eq!(
            search.nearest_distance(Point { x, y }, &mut work),
            Ok(distance)
        );
    }
    nodes.push(node(6, 100.0, 100.0, 50.0));
    let search = cross_search(&nodes, &gaps);
    assert_eq!(
        search.nearest_distance(Point { x: 100.0, y: 100.0 }, &mut work),
        Ok(3)
    );
}

#[test]
fn sized_placement_candidates_land_on_the_grid() {
    let nodes = cross();
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let points = cross_search(&nodes, &[Spacing::default(); 5])
        .placement_points(Point { x: 100.0, y: 100.0 }, 1, &mut work)
        .unwrap();
    assert_eq!(points.len(), 24);
    assert!(
        points
            .iter()
            .all(|point| point.x % 50.0 == 0.0 && point.y % 50.0 == 0.0)
    );
}

fn positioned_optimizer(
    positions: &[(f64, f64)],
    size: f64,
    edges: &[(usize, usize)],
    cell: f64,
) -> SizedOptimizer {
    let nodes = positions
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| node(i, x, y, size))
        .collect::<Vec<_>>();
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "nodes": nodes.iter().map(|n| serde_json::json!({
            "id": n.id, "isGroup": false, "shape": "rect", "width": n.width, "height": n.height,
            "label": "", "labelWidth": 0, "labelHeight": 0
        })).collect::<Vec<_>>(),
        "edges": edges.iter().enumerate().map(|(i, (source, target))| serde_json::json!({
            "id": i.to_string(), "source": source.to_string(), "target": target.to_string(),
            "sourceArrow": false, "targetArrow": false, "label": "", "labelWidth": 0, "labelHeight": 0
        })).collect::<Vec<_>>()
    })).unwrap();
    let mut optimizer = SizedOptimizer::new(&graph, 1).unwrap();
    optimizer.state.phase.rng = GoRand::new(1);
    optimizer.state.nodes = nodes;
    optimizer.scorer = EdgeCost::new(&graph, &optimizer.state.nodes, cell).unwrap();
    optimizer.cell = cell;
    optimizer
}

#[test]
fn best_swap_candidate_respects_adjacent_spacing() {
    let positions = [
        (100.0, 0.0),
        (0.0, 100.0),
        (250.0, 100.0),
        (100.0, 200.0),
        (350.0, 200.0),
    ];
    let edges = [(0, 4), (1, 4), (2, 4), (3, 4)];
    let mut optimizer = positioned_optimizer(&positions, 50.0, &edges, 50.0);
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    assert_eq!(optimizer.best_swap(4, &mut work), Ok(None));
    let mut expanded = positions;
    expanded[4] = (400.0, 100.0);
    let mut optimizer = positioned_optimizer(&expanded, 50.0, &edges, 100.0);
    assert_eq!(optimizer.best_swap(4, &mut work), Ok(Some(2)));
}

#[test]
fn transpose_rejects_the_second_neighbor_when_diagonal() {
    let positions = [(0.0, 0.0), (30.0, 0.0), (30.0, 30.0), (0.0, 30.0)];
    let edges = [(0, 1), (0, 2), (1, 3)];
    let mut optimizer = positioned_optimizer(&positions, 10.0, &edges, 10.0);
    let original = optimizer.state.nodes.clone();
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let relative = super::super::sized_cost::Bounds::from(&original[0])
        .direction_relative_to(super::super::sized_cost::Bounds::from(&original[2]));
    assert!(relative.is_some_and(|direction| direction.abs() % 2 == 1));
    assert_eq!(optimizer.transpose(0, &mut work), Ok(false));
    assert_eq!(optimizer.state.nodes, original);
}
