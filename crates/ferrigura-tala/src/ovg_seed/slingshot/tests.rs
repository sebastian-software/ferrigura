use super::*;
use crate::ovg_seed::PortOwner;

fn rectangle(x: f64, y: f64, width: f64, height: f64) -> NodeGeometry {
    NodeGeometry {
        id: String::new(),
        x,
        y,
        width,
        height,
    }
}

#[test]
fn prefer_launching_vertically() {
    let a = rectangle(10.0, 10.0, 5.0, 5.0);
    let b = rectangle(20.0, 20.0, 5.0, 5.0);
    let diagonal = Orientation::between(&a, &b).diagonal().unwrap();
    assert_eq!(diagonal, Diagonal::TopLeft);
    assert_eq!(
        diagonal.prefer_launching_vertically(1, 2, [], []),
        (true, false)
    );
    let c = rectangle(10.0, 20.0, 5.0, 5.0);
    assert_eq!(
        diagonal.prefer_launching_vertically(1, 2, [Orientation::between(&c, &a)], []),
        (false, true)
    );
    let c = rectangle(20.0, 10.0, 5.0, 5.0);
    assert_eq!(
        diagonal.prefer_launching_vertically(1, 2, [Orientation::between(&c, &a)], []),
        (true, true)
    );
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
fn fill_path() {
    let graph = graph(
        &[
            (12.0, 12.0),
            (12.0, 14.0),
            (12.0, 16.0),
            (10.0, 16.0),
            (20.0, 22.0),
            (22.0, 22.0),
            (12.0, 22.0),
        ],
        &[[0, 1], [1, 2], [1, 3], [2, 6], [6, 4], [4, 5]],
    );
    let adjacency = VisibilityAdjacency::new(&graph).unwrap();
    let mut work = 6;
    let actual = adjacency.fill_path(1, 6, &mut work).unwrap();
    assert_eq!(actual, Some(vec![1, 2]));
    assert_eq!(work, 0);
    assert_eq!(adjacency.fill_path(1, 6, &mut 5), Err(Error::WorkLimit));
    let mut work = usize::MAX;
    assert_eq!(adjacency.fill_path(6, 1, &mut work), Ok(Some(vec![6, 2])));
    assert_eq!(adjacency.fill_path(1, 4, &mut work), Ok(None));
    assert_eq!(adjacency.fill_path(6, 6, &mut work), Ok(None));
}

#[test]
fn flight_direction_respects_port_owner_and_order() {
    let mut graph = graph(&[(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)], &[]);
    graph.ports_by_node = vec![vec![2, 0, 2, 1], vec![2, 1]];
    graph.vertices[0].owners = vec![PortOwner {
        node: 0,
        sides: 1,
        center: false,
    }];
    graph.vertices[1].owners = vec![
        PortOwner {
            node: 0,
            sides: 4,
            center: false,
        },
        PortOwner {
            node: 1,
            sides: 2,
            center: false,
        },
    ];
    graph.vertices[2].owners = vec![
        PortOwner {
            node: 0,
            sides: 4,
            center: false,
        },
        PortOwner {
            node: 1,
            sides: 8,
            center: false,
        },
    ];
    let adjacency = VisibilityAdjacency::new(&graph).unwrap();
    let flight = Diagonal::TopLeft.flight(true);
    assert_eq!(flight.launch_ports(&adjacency, 0), [2, 1]);
    assert_eq!(flight.landing_ports(&adjacency, 1), [1]);
    assert_eq!(
        Diagonal::TopRight.flight(true).landing_ports(&adjacency, 1),
        [2]
    );
    assert!(flight.on_flight_plan(adjacency.vertex(0).point, adjacency.vertex(1).point));
    let target = rectangle(0.0, 1.0, 1.0, 1.0);
    assert!(flight.undershot(adjacency.vertex(0).point, &target));
    assert!(!flight.undershot(adjacency.vertex(1).point, &target));
    assert!(flight.overshot(adjacency.vertex(2).point, &target, false));
    assert!(!flight.overshot(adjacency.vertex(2).point, &target, true));
}
