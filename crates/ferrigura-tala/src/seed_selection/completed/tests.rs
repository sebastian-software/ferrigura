use std::cell::Cell;

use super::*;

const POINTS: [Point; 2] = [Point { x: 0.0, y: 20.0 }, Point { x: 100.0, y: 50.0 }];
pub(super) const NODES: [CompletedNode; 2] = [
    CompletedNode {
        id: 41,
        top_left: Some(Point { x: -20.0, y: 20.0 }),
        size: Dimensions {
            width: 20.0,
            height: 20.0,
        },
        label_size: None,
    },
    CompletedNode {
        id: 79,
        top_left: Some(Point { x: 100.0, y: 50.0 }),
        size: Dimensions {
            width: 20.0,
            height: 20.0,
        },
        label_size: None,
    },
];
pub(super) const EDGE: CompletedEdge<'static> = CompletedEdge {
    id: 118,
    source: Some(0),
    target: Some(1),
    points: &POINTS,
    label_percentage: 0.5,
    label_size: None,
    source_arrowhead_label_size: None,
    target_arrowhead_label_size: None,
};
const AUTHORED_EDGE: AuthoredEdge = AuthoredEdge {
    id: 118,
    source: 41,
    target: 79,
};
const AUTHORED: AuthoredTopology<'static> = AuthoredTopology {
    node_ids: &[41, 79],
    edges: &[AUTHORED_EDGE],
};

#[test]
fn evaluate_seed_rejects_invalid_completed_graphs() {
    let y_infinite = [
        Point {
            x: 0.0,
            y: f64::INFINITY,
        },
        POINTS[1],
    ];
    let mut nodes = NODES;
    let mut edge = EDGE;
    assert_eq!(validate(&nodes, &[edge]), Ok(()));

    nodes[0].top_left.as_mut().unwrap().x = f64::NAN;
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::InvalidNodeGeometry {
            id: 41,
            field: "node x"
        }
    );
    nodes[0] = NODES[0];
    nodes[0].size.width = 0.0;
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::InvalidNodeGeometry {
            id: 41,
            field: "node width"
        }
    );
    nodes[0] = NODES[0];

    edge.points = &[];
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::IncompleteRoute(118)
    );
    edge.points = &y_infinite;
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::InvalidRoutePoint {
            id: 118,
            index: 0,
            axis: 'y'
        }
    );
    edge = EDGE;
    edge.label_percentage = f64::NAN;
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::InvalidEdgeGeometry {
            id: 118,
            field: "label percentage"
        }
    );
    edge = EDGE;
    edge.source = None;
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::ForeignEndpoint {
            id: 118,
            field: "source"
        }
    );
    edge = EDGE;

    nodes[1].id = nodes[0].id;
    assert_eq!(
        validate(&nodes, &[edge]).unwrap_err(),
        ValidationError::DuplicateNode(41)
    );
}

#[test]
fn evaluate_seed_cancellation_precedes_validation() {
    let edges = [CompletedEdge {
        points: &[],
        ..EDGE
    }];
    let graph = CompletedGraph {
        nodes: &NODES,
        edges: &edges,
    };
    assert_eq!(graph.validate(&|| true), Err(ValidationError::Cancelled));
    assert_eq!(
        graph.validate(&|| false),
        Err(ValidationError::IncompleteRoute(118))
    );
}

#[test]
fn completed_geometry_checks_source_ranges_without_changing_coordinates() {
    let mut nodes = NODES;
    nodes[0].top_left = Some(Point {
        x: -MAX_RESULT_COORDINATE,
        y: MAX_RESULT_COORDINATE,
    });
    nodes[0].size = Dimensions {
        width: MAX_RESULT_COORDINATE,
        height: MAX_RESULT_COORDINATE,
    };
    nodes[0].label_size = Some(Dimensions {
        width: 0.0,
        height: -0.0,
    });
    let edge = CompletedEdge {
        points: &[
            Point {
                x: MAX_RESULT_COORDINATE,
                y: 0.0,
            },
            Point { x: 0.0, y: 0.0 },
        ],
        label_percentage: 1.0,
        ..EDGE
    };
    assert_eq!(validate(&nodes, &[edge]), Ok(()));
    assert_eq!(nodes[0].top_left.unwrap().y, MAX_RESULT_COORDINATE);
    assert_eq!(
        validate(&nodes, &[edge, edge]),
        Err(ValidationError::DuplicateEdge(118))
    );
    nodes[0].top_left = None;
    assert_eq!(
        validate(&nodes, &[edge]),
        Err(ValidationError::MissingNodePosition(41))
    );
    let edge = CompletedEdge {
        source_arrowhead_label_size: Some(Dimensions {
            width: 0.0,
            height: -0.01,
        }),
        ..EDGE
    };
    assert_eq!(
        validate(&NODES, &[edge]),
        Err(ValidationError::InvalidEdgeGeometry {
            id: 118,
            field: "source arrowhead label height"
        })
    );
}

#[test]
fn checks_cancellation_again_while_validating_long_routes() {
    let points = [POINTS[0]; 257];
    let edge = CompletedEdge {
        points: &points,
        ..EDGE
    };
    let count = Cell::new(0);
    let cancellation = || {
        count.set(count.get() + 1);
        count.get() == 6
    };
    assert_eq!(
        CompletedGraph {
            nodes: &NODES,
            edges: &[edge]
        }
        .validate(&cancellation),
        Err(ValidationError::Cancelled)
    );
}

#[test]
fn evaluate_seed_rejects_changed_topology() {
    let expected = AUTHORED_EDGE;
    let edges = [CompletedEdge {
        source: EDGE.target,
        ..EDGE
    }];
    let graph = CompletedGraph {
        nodes: &NODES,
        edges: &edges,
    };
    let actual = AuthoredEdge {
        source: expected.target,
        ..expected
    };
    assert_eq!(
        graph.validate_topology(&AUTHORED, HashSet::new(), &|| false),
        Err(ValidationError::ChangedEndpoints { expected, actual })
    );
}

#[test]
fn topology_tracks_absent_sequence_transitions_and_stable_node_ids() {
    let missing_edge = AuthoredEdge {
        id: 119,
        ..AUTHORED_EDGE
    };
    let edges = [AUTHORED_EDGE, missing_edge];
    let expected = AuthoredTopology {
        edges: &edges,
        ..AUTHORED
    };
    let nodes = [NODES[1], NODES[0]];
    let edges = [CompletedEdge {
        source: Some(1),
        target: Some(0),
        ..EDGE
    }];
    let graph = CompletedGraph {
        nodes: &nodes,
        edges: &edges,
    };

    assert_eq!(
        graph.validate_topology(&expected, HashSet::new(), &|| false),
        Err(ValidationError::MissingEdge(119))
    );
    assert_eq!(
        graph.validate_topology(&expected, HashSet::from([118, 119]), &|| false),
        Ok(HashSet::from([119]))
    );
    let changed = AuthoredTopology {
        node_ids: &[41, 96],
        ..expected
    };
    assert_eq!(
        graph.validate_topology(&changed, HashSet::new(), &|| false),
        Err(ValidationError::UnexpectedNode(79))
    );
    let unknown = [AuthoredEdge {
        id: 121,
        ..AUTHORED_EDGE
    }];
    let changed = AuthoredTopology {
        edges: &unknown,
        ..expected
    };
    assert_eq!(
        graph.validate_topology(&changed, HashSet::from([121]), &|| false),
        Err(ValidationError::UnexpectedEdge(118))
    );
}

fn validate(nodes: &[CompletedNode], edges: &[CompletedEdge<'_>]) -> Result<(), ValidationError> {
    CompletedGraph { nodes, edges }.validate(&|| false)
}
