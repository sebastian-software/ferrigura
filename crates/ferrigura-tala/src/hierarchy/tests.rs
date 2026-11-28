// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Deserialize;

use super::layout;
use crate::{Error, Graph, Layout};

#[derive(Deserialize)]
struct Fixture {
    name: String,
    graph: Graph,
    expected: Layout,
}

fn fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("fixtures.json")).expect("pinned Go hierarchy fixtures")
}

#[test]
fn measured_container_geometry_matches_pinned_go_tala() {
    for Fixture {
        name,
        graph,
        expected,
    } in fixtures()
    {
        assert_eq!(crate::layout(&graph), Ok(expected), "{name}");
    }
}

#[test]
fn rejects_topology_and_labels_outside_the_container_slice() {
    let mut fixtures = fixtures();
    let mut graph = fixtures.remove(0).graph;
    graph.nodes[0].label_height = 50.25;
    assert_eq!(crate::layout(&graph), Err(Error::UnsupportedGraph));

    graph.nodes[0].label_height = 14.0;
    graph.nodes[0].parent_id = Some(String::from("outer"));
    assert!(layout(&graph).is_none());

    graph.nodes[0].parent_id = None;
    graph.nodes[1].parent_id = Some(String::from("missing"));
    assert!(layout(&graph).is_none());

    graph.nodes[1].parent_id = None;
    assert!(layout(&graph).is_none());

    let mut graph = fixtures
        .into_iter()
        .find(|fixture| fixture.name == "directed pair uses implicit innermost direction")
        .unwrap()
        .graph;
    graph.edges[0].target = String::from("outer");
    assert_eq!(crate::layout(&graph), Err(Error::UnsupportedGraph));
    graph.edges[0].target = String::from("sink");
    graph.edges[0].label = String::from("label clearance affects this route");
    assert_eq!(crate::layout(&graph), Err(Error::UnsupportedGraph));
}

fn boundary_fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("boundary-fixtures.json"))
        .expect("pinned Go boundary fixtures")
}

#[test]
fn crosses_one_container_boundary_and_attaches_to_its_frame() {
    for Fixture {
        name,
        graph,
        expected,
    } in boundary_fixtures()
    {
        assert_eq!(crate::layout(&graph), Ok(expected), "{name}");
    }
}

#[test]
fn rejects_unported_cross_boundary_geometry() {
    let mut graph = boundary_fixtures().remove(0).graph;
    graph.direction = None;
    assert_eq!(crate::layout(&graph), Err(Error::UnsupportedGraph));

    graph.direction = Some(String::from("TB"));
    graph.nodes[0].direction = Some(String::from("LR"));
    assert_eq!(crate::layout(&graph), Err(Error::UnsupportedGraph));

    graph.nodes[0].direction = None;
    graph.nodes[1].width += 1.0;
    assert_eq!(crate::layout(&graph), Err(Error::UnsupportedGraph));
}
