// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Deserialize;

use super::{Grid, initialize};
use crate::{Error, Graph};

#[derive(Deserialize)]
struct Fixture {
    name: String,
    graph: Graph,
    expected: Grid,
}

fn fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("fixtures.json")).expect("pinned Go initialization trace")
}

#[test]
fn ordinary_initializer_matches_pinned_cyclic_and_local_components() {
    // Each fixture traces seed 1 immediately after initializeNodes in
    // node_placement.go from a measured graph in tools/tala-pilot/captures.
    for Fixture {
        name,
        graph,
        expected,
    } in fixtures()
    {
        assert_eq!(initialize(&graph), Ok(expected), "{name}");
    }
}

#[test]
fn rejects_unprocessed_containers_and_disconnected_components() {
    let mut graph = fixtures().remove(0).graph;
    graph.nodes[0].is_group = true;
    assert_eq!(initialize(&graph), Err(Error::UnsupportedGraph));
    graph.nodes[0].is_group = false;
    graph.edges.clear();
    assert_eq!(initialize(&graph), Err(Error::UnsupportedGraph));
}

#[test]
fn implicit_direction_prefers_lower_right_only_for_directed_edges() {
    let mut edge = super::Neighbor {
        node: 1,
        authored_from_here: true,
        directed: true,
        incoming: false,
    };
    let cost = |edge, target| super::edge_cost(edge, (0, 0), target, None);
    assert!(cost(edge, (0, 1)) < cost(edge, (-1, 0)));
    edge.directed = false;
    assert_eq!(cost(edge, (0, 1)), cost(edge, (-1, 0)));
}
