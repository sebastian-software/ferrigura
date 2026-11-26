//! The inheritance tree of the design mode (`docs/decisions/0023-design-mode.md`): the
//! generalizations that meet a class on one side share one triangle in the middle of that side and
//! one line into it, and branch on a bus to their children, as a person draws an inheritance tree.
//!
//! ELK gives every edge a port of its own, so two children of one parent would meet it at two
//! triangles side by side. After the layout, this step takes the edges with a triangle at the same
//! side of the same class, of the same line pattern (an inheritance and a realization are not
//! merged), and moves their ends to the middle of that side. Each route runs on past its last turn
//! to the bus, the line where the edge that turns closest to the class turns, along the bus to the
//! middle, and into the class. Where one child stands straight below its parent, its line takes the
//! place of the middle, so that the bus meets it without a step. A side where another edge meets the class near its middle, a route
//! that is not orthogonal, and an end with a cardinality or an edge with a label are left alone.
//! The step runs on the applied layout, in the renderer, not in the ELK port.

use std::collections::BTreeMap;

use crate::unified::data::{Edge, Node};
use crate::unified::geom::Point;

/// How close two coordinates must be to count as one.
const EPSILON: f64 = 0.5;

/// The half width of the room that another edge's end keeps around the middle of the side: an
/// end there would run on the merged line.
const CLEARANCE: f64 = 10.0;

/// Merges the generalizations that meet one side of a class into one triangle and one line.
pub fn merge_generalizations(edges: &mut [Edge], nodes: &[Node]) {
    // The ends with a triangle, by their class, the side they meet and their pattern.
    let mut groups: BTreeMap<(String, Side, String), Vec<(usize, bool)>> = BTreeMap::new();
    for (index, edge) in edges.iter().enumerate() {
        if !edge.label.is_empty() || edge.points.len() < 2 || edge.start == edge.end {
            continue;
        }
        for at_start in [true, false] {
            let arrow = if at_start {
                &edge.arrow_type_start
            } else {
                &edge.arrow_type_end
            };
            if arrow != "extension" || has_terminal(edge, at_start) {
                continue;
            }
            let node = if at_start { &edge.start } else { &edge.end };
            let Some(side) = side_of(&toward(edge, at_start)) else {
                continue;
            };
            groups
                .entry((node.clone(), side, edge.pattern.clone()))
                .or_default()
                .push((index, at_start));
        }
    }
    for ((node_id, side, _), members) in groups {
        if members.len() < 2 {
            continue;
        }
        let Some(node) = nodes.iter().find(|node| node.id == node_id) else {
            continue;
        };
        let (Some(x), Some(y)) = (node.x, node.y) else {
            continue;
        };
        let routes: Vec<Vec<Point>> = members
            .iter()
            .map(|&(index, at_start)| toward(&edges[index], at_start))
            .collect();
        if !routes.iter().all(|route| orthogonal(route)) {
            continue;
        }
        // The middle of the side, or the line of the one child that stands straight below its
        // parent, which the bus then meets without a step.
        let straight: Vec<f64> = routes
            .iter()
            .filter(|route| run_start(route, side) == 0)
            .filter_map(|route| route.last().map(|end| side.across(*end)))
            .collect();
        let middle = match straight.as_slice() {
            [axis] => *axis,
            _ => {
                if side.vertical() {
                    x
                } else {
                    y
                }
            }
        };
        // Another edge that meets the side near its middle would run on the merged line.
        let crowded = edges.iter().enumerate().any(|(index, edge)| {
            [true, false].into_iter().any(|at_start| {
                let id = if at_start { &edge.start } else { &edge.end };
                if *id != node_id || members.contains(&(index, at_start)) {
                    return false;
                }
                let points = toward(edge, at_start);
                side_of(&points) == Some(side)
                    && points
                        .last()
                        .is_some_and(|end| (side.across(*end) - middle).abs() < CLEARANCE)
            })
        });
        if crowded {
            continue;
        }
        // The bus runs where the edge that turns closest to the class turns.
        let Some(bus) = routes
            .iter()
            .filter_map(|route| last_turn(route, side))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(along, _)| along)
        else {
            continue;
        };
        for (&(index, at_start), route) in members.iter().zip(routes) {
            let merged = merge(&route, side, bus, middle);
            let points = if at_start {
                merged.into_iter().rev().collect()
            } else {
                merged
            };
            edges[index].points = points;
        }
    }
}

/// Whether the end of `edge` carries a cardinality.
fn has_terminal(edge: &Edge, at_start: bool) -> bool {
    let text = if at_start {
        &edge.start_label_right
    } else {
        &edge.end_label_left
    };
    text.as_deref().is_some_and(|text| !text.is_empty())
}

/// The route of `edge` toward the end `at_start` names: reversed for the start.
fn toward(edge: &Edge, at_start: bool) -> Vec<Point> {
    if at_start {
        edge.points.iter().rev().copied().collect()
    } else {
        edge.points.clone()
    }
}

/// The side of a class a route meets at its last point, from the direction of its last segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    /// Whether the side is the top or the bottom, which an edge meets running vertically.
    fn vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// The coordinate of a point along the side.
    fn across(self, point: Point) -> f64 {
        if self.vertical() { point.x } else { point.y }
    }

    /// The coordinate of a point across the side, toward or away from the class.
    fn along(self, point: Point) -> f64 {
        if self.vertical() { point.y } else { point.x }
    }

    /// A point from its coordinates along the side and across it.
    fn point(self, across: f64, along: f64) -> Point {
        if self.vertical() {
            Point::new(across, along)
        } else {
            Point::new(along, across)
        }
    }
}

/// The side a route meets, when its last segment runs straight into it.
fn side_of(route: &[Point]) -> Option<Side> {
    let [.., before, end] = route else {
        return None;
    };
    if (before.x - end.x).abs() < EPSILON && (before.y - end.y).abs() >= EPSILON {
        Some(if before.y < end.y {
            Side::Top
        } else {
            Side::Bottom
        })
    } else if (before.y - end.y).abs() < EPSILON && (before.x - end.x).abs() >= EPSILON {
        Some(if before.x < end.x {
            Side::Left
        } else {
            Side::Right
        })
    } else {
        None
    }
}

/// The index where the straight run into the class starts: the last point of the route that is not
/// on the line of the last segment, plus one.
fn run_start(route: &[Point], side: Side) -> usize {
    let end = route[route.len() - 1];
    let mut start = route.len() - 1;
    while start > 0 && (side.across(route[start - 1]) - side.across(end)).abs() < EPSILON {
        start -= 1;
    }
    start
}

/// Where the route turns into its run into the class, and how far that is from the class: `None`
/// for a straight route.
fn last_turn(route: &[Point], side: Side) -> Option<(f64, f64)> {
    let start = run_start(route, side);
    if start == 0 {
        return None;
    }
    let end = route[route.len() - 1];
    let along = side.along(route[start]);
    Some((along, (along - side.along(end)).abs()))
}

/// The route with its end moved to `middle` of the side: the part before its last turn runs on to
/// the bus, along it to the middle, and into the class. A straight route turns onto the bus.
fn merge(route: &[Point], side: Side, bus: f64, middle: f64) -> Vec<Point> {
    let end = route[route.len() - 1];
    if (side.across(end) - middle).abs() < EPSILON {
        return route.to_vec();
    }
    let start = run_start(route, side);
    let mut out: Vec<Point> = route[..start.max(1)].to_vec();
    let last = out[out.len() - 1];
    out.push(side.point(side.across(last), bus));
    out.push(side.point(middle, bus));
    out.push(side.point(middle, side.along(end)));
    simplify(out)
}

/// Whether the route is orthogonal: every segment runs along or across the side.
fn orthogonal(route: &[Point]) -> bool {
    route.windows(2).all(|pair| {
        (pair[0].x - pair[1].x).abs() < EPSILON || (pair[0].y - pair[1].y).abs() < EPSILON
    })
}

/// The route without repeated points and without points in the middle of a straight part.
fn simplify(route: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(route.len());
    for point in route {
        if out.last().is_some_and(|last| {
            (last.x - point.x).abs() < EPSILON && (last.y - point.y).abs() < EPSILON
        }) {
            continue;
        }
        if let [.., a, b] = out.as_slice() {
            let straight = ((a.x - b.x).abs() < EPSILON && (b.x - point.x).abs() < EPSILON)
                || ((a.y - b.y).abs() < EPSILON && (b.y - point.y).abs() < EPSILON);
            if straight {
                out.pop();
            }
        }
        out.push(point);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(start: &str, end: &str, points: &[(f64, f64)]) -> Edge {
        Edge {
            start: start.to_owned(),
            end: end.to_owned(),
            arrow_type_start: "extension".to_owned(),
            arrow_type_end: "none".to_owned(),
            pattern: "solid".to_owned(),
            points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            ..Edge::default()
        }
    }

    fn node(id: &str, x: f64, y: f64) -> Node {
        Node {
            id: id.to_owned(),
            x: Some(x),
            y: Some(y),
            width: Some(100.0),
            height: Some(40.0),
            ..Node::default()
        }
    }

    #[test]
    fn two_children_share_one_triangle_in_the_middle_of_their_parent() {
        // The parent's bottom is at y = 20; its children are below, left and right.
        let mut edges = vec![
            edge(
                "P",
                "A",
                &[(-10.0, 20.0), (-10.0, 50.0), (-80.0, 50.0), (-80.0, 100.0)],
            ),
            edge(
                "P",
                "B",
                &[(10.0, 20.0), (10.0, 60.0), (80.0, 60.0), (80.0, 100.0)],
            ),
        ];
        let nodes = [
            node("P", 0.0, 0.0),
            node("A", -80.0, 120.0),
            node("B", 80.0, 120.0),
        ];
        merge_generalizations(&mut edges, &nodes);
        let points = |edge: &Edge| -> Vec<(f64, f64)> {
            edge.points.iter().map(|point| (point.x, point.y)).collect()
        };
        assert_eq!(
            points(&edges[0]),
            [(0.0, 20.0), (0.0, 50.0), (-80.0, 50.0), (-80.0, 100.0)]
        );
        assert_eq!(
            points(&edges[1]),
            [(0.0, 20.0), (0.0, 50.0), (80.0, 50.0), (80.0, 100.0)]
        );
    }

    #[test]
    fn an_edge_in_the_middle_of_the_side_keeps_the_side_apart() {
        let mut edges = vec![
            edge(
                "P",
                "A",
                &[(-20.0, 20.0), (-20.0, 50.0), (-80.0, 50.0), (-80.0, 100.0)],
            ),
            edge(
                "P",
                "B",
                &[(20.0, 20.0), (20.0, 50.0), (80.0, 50.0), (80.0, 100.0)],
            ),
            Edge {
                arrow_type_start: "none".to_owned(),
                ..edge("P", "C", &[(0.0, 20.0), (0.0, 100.0)])
            },
        ];
        let before = edges[0].points.clone();
        let nodes = [node("P", 0.0, 0.0)];
        merge_generalizations(&mut edges, &nodes);
        assert_eq!(edges[0].points, before);
    }
}
