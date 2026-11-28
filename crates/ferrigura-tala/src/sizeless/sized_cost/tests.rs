// SPDX-License-Identifier: MPL-2.0
// Ported from pinned D2 TALA: internal/placementcost/flow_continuity_test.go.

use super::{EdgeCost, IncidentEdge};
use crate::{NodeGeometry, Point};

fn bodies(points: &[Point]) -> Vec<NodeGeometry> {
    std::iter::once(Point { x: 0.0, y: 0.0 })
        .chain(points.iter().copied())
        .enumerate()
        .map(|(index, center)| NodeGeometry {
            id: index.to_string(),
            x: center.x - 20.0,
            y: center.y - 20.0,
            width: 40.0,
            height: 40.0,
        })
        .collect()
}

fn edge(neighbor: usize, incoming: bool) -> IncidentEdge {
    IncidentEdge {
        neighbor,
        directed: true,
        outgoing: !incoming,
        labeled: false,
    }
}

fn fixture(n: usize, incoming: &[bool]) -> EdgeCost {
    let mut adjacency = vec![Vec::new(); n];
    adjacency[0] = incoming
        .iter()
        .enumerate()
        .map(|(index, &incoming)| edge(index + 1, incoming))
        .collect();
    EdgeCost {
        adjacency,
        direction: None,
        cell_size: 40.0,
        turn_cost: 1.0,
    }
}

#[test]
fn flow_branches_reward_angular_room_for_fan_in_and_fan_out() {
    let mut crowded = None;
    for incoming in [false, true] {
        let cost = fixture(3, &[incoming, incoming]);
        let mut positions = bodies(&[Point { x: 0.0, y: 160.0 }, Point { x: 40.0, y: 160.0 }]);
        let value = cost.flow_continuity(0, &positions);
        if let Some(previous) = crowded {
            assert_eq!(value, previous);
        }
        assert!(value > 0.0);
        crowded = Some(value);
        (positions[2].x, positions[2].y) = (140.0, -20.0);
        assert_eq!(cost.flow_continuity(0, &positions), 0.0);
    }
}

#[test]
fn flow_reciprocal_pair_has_both_roles() {
    let positions = bodies(&[Point { x: 0.0, y: 160.0 }, Point { x: 96.0, y: 128.0 }]);
    let mut cost = fixture(3, &[true, false]);
    cost.adjacency[0].extend([edge(1, false), edge(2, true)]);
    assert!((cost.flow_continuity(0, &positions) - (1.8 + 0.6)).abs() < 1e-8);
    for _ in 0..2 {
        cost.adjacency[0].extend([edge(1, true), edge(2, false)]);
    }
    assert!((cost.flow_continuity(0, &positions) - (1.8 + 0.6)).abs() < 1e-8);
}

#[test]
fn flow_never_pairs_a_neighbor_with_itself() {
    let positions = bodies(&[Point { x: 0.0, y: 160.0 }]);
    let mut cost = fixture(2, &[true]);
    for index in 1..8 {
        cost.adjacency[0].push(edge(1, index % 2 == 0));
        assert_eq!(cost.flow_continuity(0, &positions), 0.0);
    }
}

#[test]
fn flow_parallel_edges_do_not_reweight_branches() {
    let points = [
        Point { x: 0.0, y: 160.0 },
        Point { x: 96.0, y: 128.0 },
        Point { x: -160.0, y: 0.0 },
    ];
    let positions = bodies(&points);
    let mut cost = fixture(positions.len(), &[false; 3]);
    for index in 0..6 {
        assert!((cost.flow_continuity(0, &positions) - 0.2).abs() < 1e-8);
        if index < 5 {
            cost.adjacency[0].push(edge(1, false));
        }
    }
}
