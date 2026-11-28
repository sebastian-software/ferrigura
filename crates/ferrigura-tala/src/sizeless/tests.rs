// SPDX-License-Identifier: MPL-2.0
// The placement point and swap tests come from
// internal/placement/sizeless_optimizer_test.go in the pinned D2 source.

use serde::Deserialize;

use super::*;
use crate::NodeGeometry;

const OCCUPIED: [Cell; 5] = [(1, 0), (0, 1), (1, 2), (2, 1), (0, 0)];

fn occupied(cells: &[Cell]) -> HashMap<Cell, usize> {
    cells
        .iter()
        .copied()
        .enumerate()
        .map(|(i, cell)| (cell, i))
        .collect()
}

#[test]
fn finds_closest_unoccupied_distance() {
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let mut nodes = occupied(&OCCUPIED);
    assert_eq!(nearest_distance((2, 2), &nodes, &mut work), Ok(0));
    assert_eq!(nearest_distance((1, 2), &nodes, &mut work), Ok(1));
    nodes.remove(&(0, 0));
    nodes.insert((1, 1), 4);
    assert_eq!(nearest_distance((1, 1), &nodes, &mut work), Ok(2));
}

#[test]
fn generates_unoccupied_placement_points() {
    let mut work = Work(MAX_OPTIMIZATION_WORK);
    let nodes = occupied(&OCCUPIED);
    let distance = nearest_distance((2, 2), &nodes, &mut work).unwrap();
    assert_eq!(
        placement_points((2, 2), distance, &nodes, &mut work).unwrap(),
        [(3, 2), (2, 2), (2, 3)],
    );
}

#[test]
fn finds_optimizable_swaps_in_go_order() {
    let nodes = occupied(&OCCUPIED);
    let mut movable = [true; 5];
    for (index, expected) in [&[4][..], &[4], &[], &[], &[1, 0]].into_iter().enumerate() {
        assert_eq!(swap_candidates(OCCUPIED[index], &movable, &nodes), expected);
    }
    movable[0] = false;
    assert_eq!(swap_candidates(OCCUPIED[4], &movable, &nodes), [1]);
}

#[derive(Deserialize)]
struct InitializerFixture {
    graph: Graph,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObservatoryFixture {
    edge_labels: Vec<EdgeLabel>,
    seeds: Vec<ExpectedPlacement>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EdgeLabel {
    id: String,
    label: String,
    label_width: f64,
    label_height: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedPlacement {
    seed: i64,
    first_pass: Grid,
    full_phase: Grid,
    sized_transition: Vec<NodeGeometry>,
}

#[test]
fn seeded_observatory_placement_matches_the_go_stages() {
    let source: Vec<InitializerFixture> =
        serde_json::from_str(include_str!("../initialize/fixtures.json")).unwrap();
    let mut graph = source
        .into_iter()
        .find(|fixture| fixture.name == "Cyclic observatory services")
        .unwrap()
        .graph;
    let fixture: ObservatoryFixture =
        serde_json::from_str(include_str!("observatory.json")).unwrap();
    assert_eq!(graph.edges.len(), fixture.edge_labels.len());
    for (edge, label) in graph.edges.iter_mut().zip(fixture.edge_labels) {
        assert_eq!(edge.id, label.id);
        edge.label = label.label;
        edge.label_width = label.label_width;
        edge.label_height = label.label_height;
    }
    for expected in fixture.seeds {
        let seed = expected.seed;
        assert_eq!(
            first_pass(&graph, seed),
            Ok(expected.first_pass),
            "first pass, seed {seed}"
        );
        assert_eq!(
            optimize_phase(&graph, seed),
            Ok(expected.full_phase),
            "phase, seed {seed}"
        );
        assert_eq!(
            transition_to_sized(&graph, seed),
            Ok(expected.sized_transition),
            "sized transition, seed {seed}"
        );
    }
}
