use super::*;
use crate::ovg_seed::Vertex;

fn segment(from: [f64; 2], to: [f64; 2]) -> Segment {
    Segment {
        from: Point {
            x: from[0],
            y: from[1],
        },
        to: Point { x: to[0], y: to[1] },
    }
}

fn graph(segments: &[Segment]) -> LinkedVisibility {
    let mut graph = LinkedVisibility {
        vertices: Vec::new(),
        links: Vec::new(),
        centers_by_node: Vec::new(),
        ports_by_node: Vec::new(),
        tunnel_link_count: 0,
        near_port_owners: Vec::new(),
    };
    for &segment in segments {
        let start = graph.vertices.len();
        graph.links.push([start, start + 1]);
        graph
            .vertices
            .extend([segment.from, segment.to].map(|point| Vertex {
                point,
                owners: Vec::new(),
            }));
    }
    graph
}

fn expected_intersection(first: Segment, second: Segment) -> bool {
    let (Some(a), Some(b)) = (first.direction(), second.direction()) else {
        return false;
    };
    if first.shares_point(second) {
        return false;
    }
    let (line, start, end) = first.ordered(a);
    let (other_line, other_start, other_end) = second.ordered(b);
    if a == b {
        line == other_line && start < other_end && other_start < end
    } else {
        start <= other_line && other_line <= end && other_start <= line && line <= other_end
    }
}

#[test]
fn crossing_index_matches_route_segments() {
    let links: Vec<_> = (0..12)
        .flat_map(|index| {
            let axis = f64::from(index) + 0.5;
            [
                segment([0.0, axis], [12.0, axis]),
                segment([axis, 12.0], [axis, 0.0]),
                segment([axis, axis], [axis, axis + 3.0]),
            ]
        })
        .collect();
    let routes = [
        segment([2.5, 6.5], [8.5, 6.5]),
        segment([5.5, 12.0], [5.5, 4.5]),
        segment([2.5, 3.5], [2.5, 5.5]),
        segment([12.0, 8.5], [6.5, 8.5]),
        segment([0.0, 0.0], [12.0, 12.0]),
    ];
    let mut work = usize::MAX;
    let index = Grid::new(&graph(&links), &mut work).unwrap();
    let mut crossed = vec![false; links.len()];
    for (i, &route) in routes.iter().enumerate() {
        index
            .visit_crossed(route, &mut work, |link| crossed[link] = true)
            .unwrap();
        for (index, &link) in links.iter().enumerate() {
            let want = routes[..=i]
                .iter()
                .any(|&route| expected_intersection(route, link));
            assert_eq!(crossed[index], want, "route {i}, link {index}");
        }
    }
}

#[test]
fn ranges_include_close_axis_and_shared_endpoint_but_not_float_past_proximity() {
    let just_past = 5.75_f64.next_up();
    let links = [
        segment([0.0, 0.75], [10.0, 0.75]),
        segment([10.0, 5.75], [20.0, 5.75]),
        segment([0.0, just_past], [10.0, just_past]),
        segment([5.0, 0.0], [5.0, 5.0]),
    ];
    let mut work = usize::MAX;
    let index = Grid::new(&graph(&links), &mut work).unwrap();
    let mut nearby = Vec::new();
    index
        .visit_nearby(links[0], &mut work, |link, same_line| {
            nearby.push((link, same_line))
        })
        .unwrap();
    assert_eq!(nearby, [(0, true), (1, false)]);
    let mut crosses = Vec::new();
    index
        .visit_crossed(links[0], &mut work, |link| crosses.push(link))
        .unwrap();
    assert_eq!(crosses, [3]);
}
