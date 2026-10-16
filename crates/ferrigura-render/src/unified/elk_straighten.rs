//! Straightening the port-to-channel step at the ends of ELK's routes: a port of
//! `straightenTerminalJogs`, `straightenEdgeTerminals` and `projectLabelOntoStraightenedRun` in
//! `packages/mermaid/src/rendering-util/layout-algorithms/elk/render.ts` (1594 to 1930 of that file).
//! See `docs-port/flowchart/render-notes/elk-apply.md` section 3.10.
//!
//! Only the flowchart's main labels are carried along; a terminal label never exists here.

use super::data::Edge;
use super::elk_geometry::{js_hypot, js_sign};
use super::geom::{CenterBox, Point};

/// Largest port-to-channel jog worth collapsing (`TERMINAL_JOG_MAX`).
const TERMINAL_JOG_MAX: f64 = 16.0;

/// How far from the node the step may sit and still count as the connector (`TERMINAL_RUN_MAX`).
const TERMINAL_RUN_MAX: f64 = 30.0;

/// Tolerance for deciding whether a segment is axis-aligned and whether a step is a step at all
/// (`JOG_EPS`). Much smaller than [`super::elk_geometry::EPS`], so that sub-pixel steps count.
const JOG_EPS: f64 = 0.01;

/// A run displaced by `straightenFront`: its points before and after, each as the pair of its
/// first and last point in the polyline's order (`StraightenedRun`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StraightenedRun {
    /// The run before straightening, first and last point.
    pub old: (Point, Point),
    /// The run after straightening, first and last point.
    pub new: (Point, Point),
}

/// A label's centre and size, as the straightening reads it (`labelBox`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Label {
    centre: Point,
    width: f64,
    height: f64,
}

/// Axis of an axis-aligned segment: horizontal, vertical, or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

/// `axisOf` in `render.ts`.
fn axis_of(a: Point, b: Point) -> Option<Axis> {
    let dx = (b.x - a.x).abs();
    let dy = (b.y - a.y).abs();
    if dx > JOG_EPS && dy <= JOG_EPS {
        return Some(Axis::Horizontal);
    }
    if dy > JOG_EPS && dx <= JOG_EPS {
        return Some(Axis::Vertical);
    }
    None
}

/// Straightens the staircase at the front of `points`, or returns `None` when it does not apply
/// (`straightenFront`). The port stays where it is: the channel moves onto the port's row.
fn straighten_front(points: &[Point]) -> Option<(Vec<Point>, StraightenedRun)> {
    if points.len() < 5 {
        return None;
    }
    let (p0, p1, p2, p3) = (points[0], points[1], points[2], points[3]);
    let axis = axis_of(p0, p1)?;
    let perpendicular = match axis {
        Axis::Horizontal => Axis::Vertical,
        Axis::Vertical => Axis::Horizontal,
    };
    if axis_of(p2, p3) != Some(axis) || axis_of(p1, p2) != Some(perpendicular) {
        return None;
    }
    // The step has to sit next to the node to be the port-to-channel connector.
    if js_hypot(p1.x - p0.x, p1.y - p0.y) > TERMINAL_RUN_MAX {
        return None;
    }
    let jog = match axis {
        Axis::Horizontal => (p2.y - p1.y).abs(),
        Axis::Vertical => (p2.x - p1.x).abs(),
    };
    if !(JOG_EPS..=TERMINAL_JOG_MAX).contains(&jog) {
        return None;
    }
    // The route keeps travelling the same way after the step, or the step is a real turn.
    let forward = match axis {
        Axis::Horizontal => js_sign(p1.x - p0.x) == js_sign(p3.x - p2.x),
        Axis::Vertical => js_sign(p1.y - p0.y) == js_sign(p3.y - p2.y),
    };
    if !forward {
        return None;
    }
    // The whole run moves, not only its first segment, or a diagonal is left where the halves meet.
    let mut last = 3;
    while last + 1 < points.len() && axis_of(points[last], points[last + 1]) == Some(axis) {
        last += 1;
    }
    // The far terminal must not be inside the run: moving it would drag the other port.
    if last == points.len() - 1 {
        return None;
    }

    let mut moved = points.to_vec();
    for (i, point) in points.iter().enumerate().take(last + 1).skip(2) {
        moved[i] = match axis {
            Axis::Horizontal => Point::new(point.x, p0.y),
            Axis::Vertical => Point::new(p0.x, point.y),
        };
    }
    let run = StraightenedRun {
        old: (points[2], points[last]),
        new: (moved[2], moved[last]),
    };
    // p1 and p2 are now collinear with p0 and the rest of the run.
    moved.drain(1..3);
    Some((moved, run))
}

/// Straightens the port-to-channel step at both ends of a route, and reports the runs that moved
/// (`straightenTerminalJogsWithRuns`). No runs means the route is unchanged.
fn straighten_terminal_jogs_with_runs(points: &[Point]) -> (Vec<Point>, Vec<StraightenedRun>) {
    let mut runs = Vec::new();
    let mut pts = points.to_vec();

    if let Some((front, run)) = straighten_front(&pts) {
        pts = front;
        runs.push(run);
    }

    let mut reversed = pts.clone();
    reversed.reverse();
    if let Some((back, run)) = straighten_front(&reversed) {
        let mut back = back;
        back.reverse();
        pts = back;
        runs.push(run);
    }

    (pts, runs)
}

/// Straightens the port-to-channel staircase at either end of a clipped route, leaving both ports
/// where they are (`straightenTerminalJogs`). Returns the input unchanged when nothing applies.
#[must_use]
pub fn straighten_terminal_jogs(points: &[Point]) -> Vec<Point> {
    straighten_terminal_jogs_with_runs(points).0
}

/// Whether two axis-aligned segments cross at a point interior to both (`segmentsCrossStrict`).
fn segments_cross_strict(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    let side = |o: Point, p: Point, q: Point| (p.x - o.x) * (q.y - o.y) - (p.y - o.y) * (q.x - o.x);
    let d1 = side(b1, b2, a1);
    let d2 = side(b1, b2, a2);
    let d3 = side(a1, a2, b1);
    let d4 = side(a1, a2, b2);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// How many times one polyline crosses another (`crossingCount`).
fn crossing_count(a: &[Point], b: &[Point]) -> usize {
    let mut n = 0;
    for i in 0..a.len().saturating_sub(1) {
        for j in 0..b.len().saturating_sub(1) {
            if segments_cross_strict(a[i], a[i + 1], b[j], b[j + 1]) {
                n += 1;
            }
        }
    }
    n
}

/// Where the projection of `p` onto the line through `a` and `b` falls, clamped to the segment
/// (`paramOnSegment`).
fn param_on_segment(p: Point, a: Point, b: Point) -> f64 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len_sq = abx * abx + aby * aby;
    if len_sq == 0.0 {
        return 0.0;
    }
    (((p.x - a.x) * abx + (p.y - a.y) * aby) / len_sq).clamp(0.0, 1.0)
}

/// The point at parameter `t` along `a` to `b`, with `t` 0 at `a` and 1 at `b` (`pointAtParam`).
fn point_at_param(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
}

/// Projects a label that sits on a straightened run onto the run's new position
/// (`projectLabelOntoStraightenedRun`). Only the runs that moved are searched, so an unchanged
/// segment that happens to be nearer to the label is never chosen.
#[must_use]
pub fn project_label_onto_straightened_run(
    label: Point,
    runs: &[StraightenedRun],
) -> Option<Point> {
    let mut best: Option<(f64, Point)> = None;
    for run in runs {
        let t = param_on_segment(label, run.old.0, run.old.1);
        let old_point = point_at_param(run.old.0, run.old.1, t);
        let dx = old_point.x - label.x;
        let dy = old_point.y - label.y;
        let dist_sq = dx * dx + dy * dy;
        if best.is_none_or(|(best_dist, _)| dist_sq < best_dist) {
            best = Some((dist_sq, point_at_param(run.new.0, run.new.1, t)));
        }
    }
    best.map(|(_, point)| point)
}

/// Whether a segment from `a` to `b` passes through the box (Liang-Barsky clip, `segmentHitsBox`).
fn segment_hits_box(a: Point, b: Point, bounds: CenterBox) -> bool {
    let (x1, y1) = (
        bounds.x - bounds.width / 2.0,
        bounds.y - bounds.height / 2.0,
    );
    let (x2, y2) = (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    for (p, q) in [
        (-dx, a.x - x1),
        (dx, x2 - a.x),
        (-dy, a.y - y1),
        (dy, y2 - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else if p < 0.0 {
            t0 = t0.max(q / p);
        } else {
            t1 = t1.min(q / p);
        }
    }
    t0 < t1
}

/// The box of a label, centred on its centre (`labelBox`).
fn label_box(label: Label) -> CenterBox {
    CenterBox::new(label.centre.x, label.centre.y, label.width, label.height)
}

/// The label of an edge as the straightening sees it: the main label, when the edge has one with a
/// position and a size (`labels[index]`).
fn label_of(edge: &Edge) -> Option<Label> {
    let (x, y) = (edge.x?, edge.y?);
    let width = edge.width.filter(|w| *w != 0.0 && !w.is_nan())?;
    let height = edge.height.filter(|h| *h != 0.0 && !h.is_nan())?;
    if edge.label.is_empty() {
        return None;
    }
    Some(Label {
        centre: Point::new(x, y),
        width,
        height,
    })
}

/// Whether a route runs through a label's box (`covers`).
fn covers(route: &[Point], label: Option<Label>) -> bool {
    let Some(label) = label else {
        return false;
    };
    let box_ = label_box(label);
    route
        .windows(2)
        .any(|segment| segment_hits_box(segment[0], segment[1], box_))
}

/// Straightens the port-to-channel step on every edge that has one, where that does not buy a
/// crossing or a label overlap (`straightenEdgeTerminals`).
///
/// The edges are decided in order, and each accepted change is seen by the edges after it. Returns
/// the index and the moved runs of each edge whose points changed, so the caller can carry the
/// edge's main label along with the run it sat on. The edges' points are updated in place.
pub fn straighten_edge_terminals(edges: &mut [Edge]) -> Vec<(usize, Vec<StraightenedRun>)> {
    let mut routes: Vec<Vec<Point>> = edges.iter().map(|edge| edge.points.clone()).collect();
    let mut labels: Vec<Option<Label>> = edges.iter().map(label_of).collect();
    let mut changed = Vec::new();

    for index in 0..edges.len() {
        let original = routes[index].clone();
        if original.len() < 5 {
            continue;
        }
        let (candidate, runs) = straighten_terminal_jogs_with_runs(&original);
        if runs.is_empty() {
            continue;
        }

        // Where the label lands if this goes ahead: the caller carries it along.
        let label = labels[index];
        let candidate_label =
            label.map(
                |label| match project_label_onto_straightened_run(label.centre, &runs) {
                    Some(centre) => Label { centre, ..label },
                    None => label,
                },
            );

        let mut before = 0;
        let mut after = 0;
        // Net counts, as for crossings: label-over-edge pairs in either direction.
        let mut covered_before = 0;
        let mut covered_after = 0;
        for other in 0..routes.len() {
            if other == index || routes[other].len() < 2 {
                continue;
            }
            before += crossing_count(&original, &routes[other]);
            after += crossing_count(&candidate, &routes[other]);
            covered_before += usize::from(covers(&routes[other], label))
                + usize::from(covers(&original, labels[other]));
            covered_after += usize::from(covers(&routes[other], candidate_label))
                + usize::from(covers(&candidate, labels[other]));
        }
        if after > before || covered_after > covered_before {
            continue;
        }

        edges[index].points.clone_from(&candidate);
        routes[index] = candidate;
        labels[index] = candidate_label;
        changed.push((index, runs));
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn edge(id: &str, points: Vec<Point>) -> Edge {
        Edge {
            id: id.to_string(),
            points,
            ..Edge::default()
        }
    }

    // geometry.spec.ts: "geometry helpers" > "straightenTerminalJogs". Every case keeps both ports
    // where they are.

    #[test]
    fn moves_the_channel_onto_the_port_row_leaving_the_port_alone() {
        // geometry.spec.ts: "moves the channel onto the port row, leaving the port alone"
        let pts = [
            p(193.0, 116.25),
            p(218.0, 116.25),
            p(218.0, 119.5),
            p(400.0, 119.5),
            p(400.0, 300.0),
        ];
        assert_eq!(
            straighten_terminal_jogs(&pts),
            vec![p(193.0, 116.25), p(400.0, 116.25), p(400.0, 300.0)]
        );
    }

    #[test]
    fn moves_a_whole_multi_segment_run_not_just_its_first_leg() {
        // geometry.spec.ts: "moves a whole multi-segment run, not just its first leg"
        let pts = [
            p(193.0, 116.25),
            p(218.0, 116.25),
            p(218.0, 119.5),
            p(300.0, 119.5),
            p(400.0, 119.5),
            p(400.0, 300.0),
        ];
        assert_eq!(
            straighten_terminal_jogs(&pts),
            vec![
                p(193.0, 116.25),
                p(300.0, 116.25),
                p(400.0, 116.25),
                p(400.0, 300.0)
            ]
        );
    }

    #[test]
    fn handles_a_sub_pixel_step() {
        // geometry.spec.ts: "handles a sub-pixel step"
        let pts = [
            p(193.0, 116.5),
            p(218.0, 116.5),
            p(218.0, 117.358),
            p(400.0, 117.358),
            p(400.0, 300.0),
        ];
        assert_eq!(
            straighten_terminal_jogs(&pts),
            vec![p(193.0, 116.5), p(400.0, 116.5), p(400.0, 300.0)]
        );
    }

    #[test]
    fn refuses_when_the_run_ends_at_the_far_port() {
        // geometry.spec.ts: "refuses when the run ends at the far port"
        let pts = vec![
            p(193.0, 116.25),
            p(218.0, 116.25),
            p(218.0, 119.5),
            p(300.0, 119.5),
            p(400.0, 119.5),
        ];
        assert_eq!(straighten_terminal_jogs(&pts), pts);
    }

    #[test]
    fn leaves_a_step_too_large_to_be_a_port_connector() {
        // geometry.spec.ts: "leaves a step too large to be a port connector"
        let pts = vec![
            p(193.0, 100.0),
            p(218.0, 100.0),
            p(218.0, 140.0),
            p(400.0, 140.0),
            p(400.0, 300.0),
        ];
        assert_eq!(straighten_terminal_jogs(&pts), pts);
    }

    #[test]
    fn leaves_a_step_that_turns_too_far_from_the_node() {
        // geometry.spec.ts: "leaves a step that turns too far from the node"
        let pts = vec![
            p(193.0, 116.25),
            p(313.0, 116.25),
            p(313.0, 119.5),
            p(415.0, 119.5),
            p(415.0, 300.0),
        ];
        assert_eq!(straighten_terminal_jogs(&pts), pts);
    }

    #[test]
    fn leaves_a_step_that_reverses_direction() {
        // geometry.spec.ts: "leaves a step that reverses direction"
        let pts = vec![
            p(193.0, 116.0),
            p(218.0, 116.0),
            p(218.0, 119.0),
            p(100.0, 119.0),
            p(100.0, 300.0),
        ];
        assert_eq!(straighten_terminal_jogs(&pts), pts);
    }

    #[test]
    fn leaves_a_route_with_nothing_to_collapse() {
        // geometry.spec.ts: "leaves a route with nothing to collapse"
        let pts = vec![p(193.0, 120.0), p(315.0, 120.0)];
        assert_eq!(straighten_terminal_jogs(&pts), pts);
    }

    // render.spec.ts: "straightenEdgeTerminals label projection (#8292)"

    fn jog_route() -> Vec<Point> {
        vec![
            p(193.0, 116.25),
            p(218.0, 116.25),
            p(218.0, 119.5),
            p(300.0, 119.5),
            p(400.0, 119.5),
            p(400.0, 300.0),
        ]
    }

    #[test]
    fn straightens_the_edge_and_reports_the_run_that_moved() {
        // render.spec.ts: "straightens the edge and reports the run that moved"
        let mut edges = vec![Edge {
            x: Some(300.0),
            y: Some(119.5),
            ..edge("e1", jog_route())
        }];
        let changed = straighten_edge_terminals(&mut edges);

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].0, 0);
        assert_eq!(
            changed[0].1,
            vec![StraightenedRun {
                old: (p(218.0, 119.5), p(400.0, 119.5)),
                new: (p(218.0, 116.25), p(400.0, 116.25)),
            }]
        );
        assert_eq!(
            edges[0].points,
            vec![
                p(193.0, 116.25),
                p(300.0, 116.25),
                p(400.0, 116.25),
                p(400.0, 300.0)
            ]
        );
    }

    #[test]
    fn projects_a_label_sitting_on_the_moved_run_onto_the_new_route() {
        // render.spec.ts: "projects a label sitting on the moved run onto the new route"
        let mut edges = vec![edge("e1", jog_route())];
        let changed = straighten_edge_terminals(&mut edges);
        let runs = &changed[0].1;

        assert_eq!(
            project_label_onto_straightened_run(p(300.0, 119.5), runs),
            Some(p(300.0, 116.25))
        );
    }

    #[test]
    fn projects_onto_the_moved_run_even_when_an_unchanged_segment_sits_nearer() {
        // render.spec.ts: "projects onto the moved run even when an unchanged segment sits nearer to
        // the old label position". The return leg at y=118 is nearer to the label than the run's new
        // position, so a whole-route search would pick it.
        let mut route = jog_route();
        route.extend([p(350.0, 300.0), p(250.0, 118.0), p(350.0, 118.0)]);
        let mut edges = vec![Edge {
            x: Some(300.0),
            y: Some(119.5),
            ..edge("e1", route)
        }];
        let changed = straighten_edge_terminals(&mut edges);

        assert_eq!(
            edges[0].points[edges[0].points.len() - 2..],
            [p(250.0, 118.0), p(350.0, 118.0)]
        );
        assert_eq!(
            project_label_onto_straightened_run(p(300.0, 119.5), &changed[0].1),
            Some(p(300.0, 116.25))
        );
    }

    #[test]
    fn leaves_an_edge_with_no_jog_and_its_label_untouched() {
        // render.spec.ts: "leaves an edge with no jog, and its label, untouched"
        let straight = vec![p(0.0, 0.0), p(100.0, 0.0)];
        let mut edges = vec![Edge {
            x: Some(50.0),
            y: Some(0.0),
            ..edge("e2", straight.clone())
        }];
        assert!(straighten_edge_terminals(&mut edges).is_empty());
        assert_eq!(edges[0].points, straight);
    }

    // render.spec.ts: "straightenEdgeTerminals keeps labels off other edges (#8368)"

    /// Independent of the implementation's clip: every route here is orthogonal.
    fn segment_hits_bounding_box(a: Point, b: Point, bounds: CenterBox) -> bool {
        let (x1, y1) = (
            bounds.x - bounds.width / 2.0,
            bounds.y - bounds.height / 2.0,
        );
        let (x2, y2) = (
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        a.x.max(b.x) >= x1 && a.x.min(b.x) <= x2 && a.y.max(b.y) >= y1 && a.y.min(b.y) <= y2
    }

    /// The ids of the edges whose route passes through `labelled`'s label box.
    fn edges_under_label(edges: &[Edge], labelled: usize) -> Vec<String> {
        let l = &edges[labelled];
        let (x, y) = (l.x.unwrap_or(0.0), l.y.unwrap_or(0.0));
        let bounds = CenterBox::new(x, y, l.width.unwrap_or(0.0), l.height.unwrap_or(0.0));
        edges
            .iter()
            .enumerate()
            .filter(|(i, other)| {
                *i != labelled
                    && other
                        .points
                        .windows(2)
                        .any(|w| segment_hits_bounding_box(w[0], w[1], bounds))
            })
            .map(|(_, other)| other.id.clone())
            .collect()
    }

    /// What `applyElkEdgeLayout` does: straighten, then carry each label along.
    fn straighten_and_project(edges: &mut [Edge]) {
        for (index, runs) in straighten_edge_terminals(edges) {
            let edge = &mut edges[index];
            let (Some(x), Some(y)) = (edge.x, edge.y) else {
                continue;
            };
            if let Some(projected) = project_label_onto_straightened_run(p(x, y), &runs) {
                edge.x = Some(projected.x);
                edge.y = Some(projected.y);
            }
        }
    }

    fn forward_route() -> Vec<Point> {
        vec![
            p(74.34375, 54.61979166666667),
            p(94.34375, 54.61979166666667),
            p(94.34375, 42.203125),
            p(142.8125, 42.203125),
            p(191.28125, 42.203125),
            p(191.28125, 45.96875),
            p(211.28125, 45.96875),
            p(223.26566632092, 45.96875),
        ]
    }

    fn back_route() -> Vec<Point> {
        vec![
            p(223.26566632092, 69.9375),
            p(211.28125, 69.9375),
            p(191.28125, 69.9375),
            p(191.28125, 73.703125),
            p(94.34375, 73.703125),
            p(94.34375, 61.28645833333333),
            p(74.34375, 61.28645833333333),
        ]
    }

    fn forward_edge() -> Edge {
        edge("L_A_B_0", forward_route())
    }

    fn back_edge() -> Edge {
        Edge {
            label: "changes".to_string(),
            x: Some(142.8125),
            y: Some(73.203125),
            width: Some(56.9375),
            height: Some(20.0),
            ..edge("L_B_A_0", back_route())
        }
    }

    #[test]
    fn does_not_straighten_a_labelled_edge_onto_a_route_its_label_would_then_cover() {
        // render.spec.ts: "does not straighten a labelled edge onto a route its label would then cover"
        let mut edges = vec![forward_edge(), back_edge()];
        straighten_and_project(&mut edges);

        assert!(edges_under_label(&edges, 1).is_empty());
        assert_ne!(edges[0].points, forward_route());
        assert_eq!(edges[1].points, back_route());
    }

    #[test]
    fn does_not_straighten_an_edge_through_a_label_placed_before_it() {
        // render.spec.ts: "does not straighten an edge through a label placed before it"
        let mut edges = vec![back_edge(), forward_edge()];
        straighten_and_project(&mut edges);

        assert!(edges_under_label(&edges, 0).is_empty());
        assert_ne!(edges[0].points, back_route());
        assert_eq!(edges[1].points, forward_route());
    }

    #[test]
    #[ignore = "port: runs runElkLayoutCore (build and ELK layout)"]
    fn centres_a_real_edge_label_on_its_route_after_a_real_elk_layout_straightens_it() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore (build and ELK layout)"]
    fn keeps_the_edge_label_of_the_issue_diagram_clear_of_the_parallel_edge_after_a_real_elk_layout()
     {
    }
}
