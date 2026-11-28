//! The side exits of a decision in the default rendering
//! (`docs/decisions/0023-the-default-rendering.md`): a branch to a node beside the decision leaves
//! from the decision's side tip and turns once, where the layout's route leaves from the tip facing
//! the flow and turns twice. A state diagram's choice is a decision here.
//!
//! ELK routes every edge out of a node from the node's side towards the next layer. A branch to a
//! node that stands beside the decision's axis then runs down, across and down again: a Z with two
//! bends. A person drawing the chart takes the branch out of the side tip, across, and down into
//! the target: an L with one bend. This step rewrites such a route after the layout has been
//! applied. It is ferrigura's own and runs only in the default rendering; the layout itself stays
//! ELK's.
//!
//! The step works in the coordinates of the flow: `along` grows with the flow, `across` is the
//! axis of the layers. In a chart from top to bottom these are `y` and `x`; the other directions
//! turn or mirror them.

use std::collections::{HashMap, HashSet};

use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::geom::Point;
use crate::unified::shapes;

/// The room a route needs for one turn: the corner of the default rendering cuts
/// `EDGE_RADIUS / sin 45°` of each part at a right angle, and takes at most half of a part
/// (`unified::path::rounded_corner`), so a part of twice the cut turns on the full radius.
const TURN_ROOM: f64 = crate::design::TURN_ROOM;

/// The least length of the part from the side tip to the turn. It is shorter than
/// [`TURN_ROOM`]: two branches to nodes in a row beside a decision as wide as a node have about
/// 24 there, and a turn on 20 still has a radius of 7, which reads as a curve.
const SIDE_RUN: f64 = 20.0;

/// The length of the arrowhead of the default rendering, which the last part of a route keeps free
/// beside its turn.
const ARROW_ROOM: f64 = 10.0;

/// The room a new route keeps from a node or a label it passes.
const CLEARANCE: f64 = 4.0;

/// The tolerance of a comparison of coordinates.
const EPSILON: f64 = 0.5;

/// How far the middle of a label may be from the part of the route it sits on: the layout centers
/// a label on its edge to about a unit.
const LABEL_TOLERANCE: f64 = 2.0;

/// A box in the coordinates of the flow.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Span {
    across: (f64, f64),
    along: (f64, f64),
}

impl Span {
    fn grown(self, by: f64) -> Self {
        Self {
            across: (self.across.0 - by, self.across.1 + by),
            along: (self.along.0 - by, self.along.1 + by),
        }
    }

    fn middle(self) -> (f64, f64) {
        (
            (self.across.0 + self.across.1) / 2.0,
            (self.along.0 + self.along.1) / 2.0,
        )
    }

    /// Whether an axis-parallel part from `a` to `b` enters the box.
    fn meets(self, a: (f64, f64), b: (f64, f64)) -> bool {
        a.0.max(b.0) > self.across.0
            && a.0.min(b.0) < self.across.1
            && a.1.max(b.1) > self.along.0
            && a.1.min(b.1) < self.along.1
    }

    fn holds(self, point: (f64, f64)) -> bool {
        (self.across.0..=self.across.1).contains(&point.0)
            && (self.along.0..=self.along.1).contains(&point.1)
    }
}

/// The direction of a chart, as the turn of its coordinates into those of the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Down,
    Up,
    Right,
    Left,
}

impl Flow {
    fn of(direction: &str) -> Option<Self> {
        match direction {
            "TB" | "TD" | "" => Some(Self::Down),
            "BT" => Some(Self::Up),
            "LR" => Some(Self::Right),
            "RL" => Some(Self::Left),
            _ => None,
        }
    }

    /// A point in the coordinates of the flow: across, then along.
    fn to_flow(self, point: Point) -> (f64, f64) {
        match self {
            Self::Down => (point.x, point.y),
            Self::Up => (point.x, -point.y),
            Self::Right => (point.y, point.x),
            Self::Left => (point.y, -point.x),
        }
    }

    /// A point of the flow back in the chart's coordinates.
    fn to_chart(self, (across, along): (f64, f64)) -> Point {
        match self {
            Self::Down => Point::new(across, along),
            Self::Up => Point::new(across, -along),
            Self::Right => Point::new(along, across),
            Self::Left => Point::new(-along, across),
        }
    }

    /// A box of the chart, given by its center and its size, in the coordinates of the flow.
    fn span(self, centre: Point, width: f64, height: f64) -> Span {
        let (across, along) = self.to_flow(centre);
        let (half_across, half_along) = match self {
            Self::Down | Self::Up => (width / 2.0, height / 2.0),
            Self::Right | Self::Left => (height / 2.0, width / 2.0),
        };
        Span {
            across: (across - half_across, across + half_across),
            along: (along - half_along, along + half_along),
        }
    }
}

/// The box of a node in the coordinates of the flow, once the layout has placed it. A group's
/// frame is at least as wide as its title and the padding, as the frame is painted.
fn node_span(flow: Flow, node: &Node) -> Option<Span> {
    let (x, y, mut width, height) = (node.x?, node.y?, node.width?, node.height?);
    if node.is_group
        && let Some((title, _)) = node.label_bbox
    {
        width = width.max(title + node.padding);
    }
    Some(flow.span(Point::new(x, y), width, height))
}

/// The box of an edge's label in the coordinates of the flow, as the layout reserved it.
fn label_span(flow: Flow, edge: &Edge) -> Option<Span> {
    if edge.label.is_empty() {
        return None;
    }
    Some(flow.span(
        Point::new(edge.x?, edge.y?),
        edge.width.unwrap_or(0.0),
        edge.height.unwrap_or(0.0),
    ))
}

/// The groups a node stands in, the nearest first.
fn ancestors<'a>(id: &str, by_id: &HashMap<&str, &'a Node>) -> Vec<&'a Node> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut current = by_id.get(id).and_then(|node| node.parent_id.as_deref());
    while let Some(parent) = current {
        if !seen.insert(parent) {
            break;
        }
        let Some(group) = by_id.get(parent) else {
            break;
        };
        chain.push(*group);
        current = group.parent_id.as_deref();
    }
    chain
}

/// Whether two parts of routes meet: cross, touch or overlap.
fn parts_meet(a: Point, b: Point, c: Point, d: Point) -> bool {
    let side = |p: Point, q: Point, r: Point| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let within = |p: Point, q: Point, r: Point| {
        (p.x.min(q.x) - 1e-9..=p.x.max(q.x) + 1e-9).contains(&r.x)
            && (p.y.min(q.y) - 1e-9..=p.y.max(q.y) + 1e-9).contains(&r.y)
    };
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1.abs() < 1e-9 && within(c, d, a))
        || (d2.abs() < 1e-9 && within(c, d, b))
        || (d3.abs() < 1e-9 && within(a, b, c))
        || (d4.abs() < 1e-9 && within(a, b, d))
}

/// How many parts of the other edges' routes a route meets.
fn crossings(route: &[Point], edges: &[Edge], own: usize) -> usize {
    let mut count = 0;
    for (index, edge) in edges.iter().enumerate() {
        if index == own {
            continue;
        }
        for other in edge.points.windows(2) {
            for part in route.windows(2) {
                if parts_meet(part[0], part[1], other[0], other[1]) {
                    count += 1;
                }
            }
        }
    }
    count
}

/// A branch that could leave from a side tip of its decision.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    edge: usize,
    /// The new route, in the chart's coordinates: the tip, the turn, the entry.
    route: [Point; 3],
    /// How far the target's entry is from the decision's axis.
    distance: f64,
}

/// Rewrites the route of each branch of a decision that stands beside the decision's axis, in the
/// next layer, so that it leaves from the side tip and turns once. See the module documentation and
/// `docs/decisions/0023-the-default-rendering.md` for the rule and the cases that keep their route.
pub(crate) fn side_exits(data: &mut LayoutData) {
    let Some(flow) = Flow::of(&data.direction) else {
        return;
    };
    let by_id: HashMap<&str, &Node> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    // Each tip of each decision, with the branches that could take it.
    let mut tips: Vec<((usize, bool), Vec<Candidate>)> = Vec::new();
    for (index, edge) in data.edges.iter().enumerate() {
        let Some(candidate) = candidate(flow, data, &by_id, index, edge) else {
            continue;
        };
        let Some(decision) = data.nodes.iter().position(|node| node.id == edge.start) else {
            continue;
        };
        let (decision_across, _) = flow.to_flow(Point::new(
            data.nodes[decision].x.unwrap_or(0.0),
            data.nodes[decision].y.unwrap_or(0.0),
        ));
        let key = (
            decision,
            flow.to_flow(candidate.route[0]).0 > decision_across,
        );
        // A tip where a self-loop of the decision stands is taken: the loop reaches past it. A
        // loop on the side that faces the flow stays between the tips.
        let half = node_span(flow, &data.nodes[decision])
            .map_or(0.0, |span| (span.across.1 - span.across.0) / 2.0);
        let looped = data.edges.iter().any(|other| {
            other.start == edge.start
                && other.end == edge.start
                && other.points.iter().any(|&point| {
                    let across = flow.to_flow(point).0;
                    (across - decision_across).abs() > half + EPSILON
                        && (across > decision_across) == key.1
                })
        });
        if looped {
            continue;
        }
        match tips.iter_mut().find(|(tip, _)| *tip == key) {
            Some((_, list)) => list.push(candidate),
            None => tips.push((key, vec![candidate])),
        }
    }
    // The branch whose target is nearest takes the tip; when its new route meets more edges than
    // its old one, the next one tries.
    for (_, mut candidates) in tips {
        candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.edge.cmp(&b.edge)));
        for candidate in candidates {
            let before = crossings(
                &data.edges[candidate.edge].points,
                &data.edges,
                candidate.edge,
            );
            let after = crossings(&candidate.route, &data.edges, candidate.edge);
            if after <= before {
                data.edges[candidate.edge].points = candidate.route.to_vec();
                break;
            }
        }
    }
}

/// Which side tips of a decision a branch would leave from, were no self-loop of the decision there:
/// the tip at the lesser coordinate across the flow (the left from top to bottom, the top from left
/// to right) and the one at the greater. A branch that has left from a tip already counts.
pub(crate) fn wanted_tips(data: &LayoutData, decision: &str) -> (bool, bool) {
    let Some(flow) = Flow::of(&data.direction) else {
        return (false, false);
    };
    let Some(node) = data.nodes.iter().find(|node| node.id == decision) else {
        return (false, false);
    };
    let (Some(x), Some(y)) = (node.x, node.y) else {
        return (false, false);
    };
    let (decision_across, _) = flow.to_flow(Point::new(x, y));
    // The decision's own loops and their labels leave the chart, as if they stood elsewhere.
    let mut free = data.clone();
    free.edges
        .retain(|edge| !(edge.start == decision && edge.end == decision));
    let by_id: HashMap<&str, &Node> = free
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut wanted = (false, false);
    for (index, edge) in free.edges.iter().enumerate() {
        if edge.start != decision {
            continue;
        }
        if let Some(candidate) = candidate(flow, &free, &by_id, index, edge) {
            if flow.to_flow(candidate.route[0]).0 > decision_across {
                wanted.1 = true;
            } else {
                wanted.0 = true;
            }
        }
    }
    wanted
}

/// The new route of an edge, when it is a branch that can leave from a side tip of its decision:
/// see [`side_exits`]. `None` keeps the edge's route.
fn candidate(
    flow: Flow,
    data: &LayoutData,
    by_id: &HashMap<&str, &Node>,
    index: usize,
    edge: &Edge,
) -> Option<Candidate> {
    if edge.start == edge.end || edge.thickness == "invisible" || !edge.terminals.is_empty() {
        return None;
    }
    let decision = *by_id.get(edge.start.as_str())?;
    let target = *by_id.get(edge.end.as_str())?;
    let decides = shapes::is_decision(decision) || shapes::is_choice(decision);
    if !decides || target.is_group || decision.parent_id != target.parent_id {
        return None;
    }
    // A group with a direction of its own lays its members out in another flow.
    let chain = ancestors(&decision.id, by_id);
    if chain.iter().any(|group| {
        group
            .dir
            .as_deref()
            .is_some_and(|dir| Flow::of(dir) != Some(flow))
    }) {
        return None;
    }
    let own = node_span(flow, decision)?;
    let goal = node_span(flow, target)?;
    // The target stands beyond the decision along the flow, and no node stands between them: it
    // is in the next layer.
    if goal.along.0 <= own.along.1 + EPSILON {
        return None;
    }
    let between = data.nodes.iter().any(|node| {
        !node.is_group
            && node.id != decision.id
            && node.id != target.id
            && node_span(flow, node).is_some_and(|span| {
                span.along.0 > own.along.1 - EPSILON && span.along.1 < goal.along.0 + EPSILON
            })
    });
    if between {
        return None;
    }

    // The old route enters the target along the flow, on the side that faces the decision.
    let points: Vec<(f64, f64)> = edge.points.iter().map(|p| flow.to_flow(*p)).collect();
    if points.len() < 2 || points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return None;
    }
    let entry = points[points.len() - 1];
    let before_entry = points[points.len() - 2];
    if (entry.0 - before_entry.0).abs() > EPSILON
        || before_entry.1 >= entry.1
        || (entry.1 - goal.along.0).abs() > EPSILON
    {
        return None;
    }
    // The old route stays between the decision's middle and the target: it passes no layer.
    let (middle_across, middle_along) = own.middle();
    if points
        .iter()
        .any(|p| p.1 < middle_along - EPSILON || p.1 > entry.1 + EPSILON)
    {
        return None;
    }

    // The entry lies beyond a side tip, with room for the turn.
    let half = (own.across.1 - own.across.0) / 2.0;
    let side = if entry.0 >= middle_across { 1.0 } else { -1.0 };
    let tip = (middle_across + side * half, middle_along);
    if side * (entry.0 - tip.0) < SIDE_RUN || entry.1 - tip.1 < TURN_ROOM + ARROW_ROOM {
        return None;
    }
    let turn = (entry.0, tip.1);
    let route = [tip, turn, entry];

    // The label stays on the new route: on its last part, clear of the turn.
    if let Some(label) = label_span(flow, edge) {
        let (label_across, _) = label.middle();
        if (label_across - entry.0).abs() > LABEL_TOLERANCE
            || label.along.0 < turn.1 + TURN_ROOM / 2.0
            || label.along.1 > entry.1 + EPSILON
        {
            return None;
        }
    }

    // The new route passes no other node and no other edge's label.
    let parts = [(tip, turn), (turn, entry)];
    let blocked = |span: Span| {
        let span = span.grown(CLEARANCE);
        parts.iter().any(|(a, b)| span.meets(*a, *b))
    };
    for node in &data.nodes {
        if node.is_group || node.id == decision.id || node.id == target.id {
            continue;
        }
        if node_span(flow, node).is_some_and(blocked) {
            return None;
        }
    }
    for (other, edge) in data.edges.iter().enumerate() {
        if other != index && label_span(flow, edge).is_some_and(blocked) {
            return None;
        }
    }
    // It stays inside the groups of its ends and out of every other group.
    for group in data.nodes.iter().filter(|node| node.is_group) {
        let Some(frame) = node_span(flow, group) else {
            continue;
        };
        if chain.iter().any(|member_of| member_of.id == group.id) {
            if !route
                .iter()
                .all(|point| frame.grown(-CLEARANCE).holds(*point))
            {
                return None;
            }
        } else if parts
            .iter()
            .any(|(a, b)| frame.grown(CLEARANCE).meets(*a, *b))
        {
            return None;
        }
    }

    Some(Candidate {
        edge: index,
        route: route.map(|point| flow.to_chart(point)),
        distance: (entry.0 - middle_across).abs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOWS: [(&str, Flow); 5] = [
        ("TB", Flow::Down),
        ("TD", Flow::Down),
        ("BT", Flow::Up),
        ("LR", Flow::Right),
        ("RL", Flow::Left),
    ];

    /// A chart written in the coordinates of the flow and stored in those of a direction.
    struct Chart {
        flow: Flow,
        data: LayoutData,
    }

    impl Chart {
        fn new(direction: &str, flow: Flow) -> Self {
            Self {
                flow,
                data: LayoutData {
                    direction: direction.to_owned(),
                    ..LayoutData::default()
                },
            }
        }

        fn size(&self, (across, along): (f64, f64)) -> (f64, f64) {
            match self.flow {
                Flow::Down | Flow::Up => (across, along),
                Flow::Right | Flow::Left => (along, across),
            }
        }

        /// A node with its middle at `at` and its size across and along the flow.
        fn node(&mut self, id: &str, shape: &str, at: (f64, f64), size: (f64, f64)) -> &mut Node {
            let centre = self.flow.to_chart(at);
            let (width, height) = self.size(size);
            self.data.nodes.push(Node {
                id: id.to_owned(),
                shape: shape.to_owned(),
                x: Some(centre.x),
                y: Some(centre.y),
                width: Some(width),
                height: Some(height),
                ..Node::default()
            });
            self.data.nodes.last_mut().expect("the node")
        }

        fn group(&mut self, id: &str, at: (f64, f64), size: (f64, f64)) {
            self.node(id, "rect", at, size).is_group = true;
        }

        fn edge(&mut self, start: &str, end: &str, route: &[(f64, f64)]) {
            let points = route.iter().map(|p| self.flow.to_chart(*p)).collect();
            self.data.edges.push(Edge {
                id: format!("L_{start}_{end}_{}", self.data.edges.len()),
                start: start.to_owned(),
                end: end.to_owned(),
                points,
                ..Edge::default()
            });
        }

        fn label(&mut self, edge: usize, at: (f64, f64), size: (f64, f64)) {
            let centre = self.flow.to_chart(at);
            let (width, height) = self.size(size);
            let edge = &mut self.data.edges[edge];
            edge.label = "yes".to_owned();
            edge.x = Some(centre.x);
            edge.y = Some(centre.y);
            edge.width = Some(width);
            edge.height = Some(height);
        }

        /// The routes of the edges after the step, in the coordinates of the flow.
        fn routes(&mut self) -> Vec<Vec<(f64, f64)>> {
            side_exits(&mut self.data);
            self.data
                .edges
                .iter()
                .map(|edge| {
                    edge.points
                        .iter()
                        .map(|p| {
                            let (across, along) = self.flow.to_flow(*p);
                            // No negative zero from a mirrored axis.
                            (across + 0.0, along + 0.0)
                        })
                        .collect()
                })
                .collect()
        }

        fn route(&mut self) -> Vec<(f64, f64)> {
            self.routes().swap_remove(0)
        }
    }

    /// The route of the branch to `T` that the layout leaves: a Z.
    const Z: [(f64, f64); 4] = [(190.0, 130.0), (190.0, 180.0), (60.0, 180.0), (60.0, 230.0)];

    /// The new route of the branch to `T`: out of the left tip, across, and into `T` where the
    /// old route entered it.
    const L: [(f64, f64); 3] = [(125.0, 100.0), (60.0, 100.0), (60.0, 230.0)];

    /// A decision `D` 150 across and 60 along, at 200 across and 100 along, and a node `T` in the
    /// next layer, at 60 across, with its near side at 230.
    fn chart(direction: &str, flow: Flow) -> Chart {
        let mut chart = Chart::new(direction, flow);
        chart.node("D", "diamond", (200.0, 100.0), (150.0, 60.0));
        chart.node("T", "rect", (60.0, 250.0), (100.0, 40.0));
        chart.edge("D", "T", &Z);
        chart
    }

    fn down() -> Chart {
        chart("TB", Flow::Down)
    }

    #[test]
    fn a_branch_beside_the_decision_leaves_from_the_side_tip_in_every_direction() {
        for (direction, flow) in FLOWS {
            assert_eq!(chart(direction, flow).route(), L.to_vec(), "{direction}");
        }
    }

    #[test]
    fn a_labeled_branch_turns_only_when_its_label_sits_on_the_last_part() {
        let mut chart = down();
        chart.label(0, (61.0, 205.0), (40.0, 17.0));
        assert_eq!(chart.route(), L.to_vec());
        // On the part across, the label would be off the new route.
        let mut chart = down();
        chart.label(0, (120.0, 180.0), (40.0, 17.0));
        assert_eq!(chart.route(), Z.to_vec());
        // Too near the turn, its gap would cut the corner.
        let mut chart = down();
        chart.label(0, (60.0, 110.0), (40.0, 17.0));
        assert_eq!(chart.route(), Z.to_vec());
    }

    #[test]
    fn a_branch_keeps_its_route_when_the_target_is_not_beside_it_in_the_next_layer() {
        // The entry is not beyond the tip with room for the turn.
        let mut chart = Chart::new("TB", Flow::Down);
        chart.node("D", "diamond", (200.0, 100.0), (150.0, 60.0));
        chart.node("T", "rect", (110.0, 250.0), (100.0, 40.0));
        let near = [
            (190.0, 130.0),
            (190.0, 180.0),
            (110.0, 180.0),
            (110.0, 230.0),
        ];
        chart.edge("D", "T", &near);
        assert_eq!(chart.route(), near.to_vec());
        // A node stands between them: the target is not in the next layer.
        let mut chart = down();
        chart.node("M", "rect", (400.0, 180.0), (100.0, 20.0));
        assert_eq!(chart.route(), Z.to_vec());
        // The target stands before the decision.
        let mut chart = Chart::new("TB", Flow::Down);
        chart.node("D", "diamond", (200.0, 300.0), (150.0, 60.0));
        chart.node("T", "rect", (60.0, 150.0), (100.0, 40.0));
        let back = [(190.0, 270.0), (190.0, 220.0), (60.0, 220.0), (60.0, 170.0)];
        chart.edge("D", "T", &back);
        assert_eq!(chart.route(), back.to_vec());
    }

    #[test]
    fn a_branch_keeps_its_route_when_it_does_not_enter_along_the_flow() {
        let mut chart = Chart::new("TB", Flow::Down);
        chart.node("D", "diamond", (200.0, 100.0), (150.0, 60.0));
        chart.node("T", "rect", (60.0, 250.0), (100.0, 40.0));
        let side = [(190.0, 130.0), (190.0, 250.0), (110.0, 250.0)];
        chart.edge("D", "T", &side);
        assert_eq!(chart.route(), side.to_vec());
        // Nor when it passes behind the decision's middle.
        let mut chart = Chart::new("TB", Flow::Down);
        chart.node("D", "diamond", (200.0, 100.0), (150.0, 60.0));
        chart.node("T", "rect", (60.0, 250.0), (100.0, 40.0));
        let around = [
            (275.0, 100.0),
            (300.0, 100.0),
            (300.0, 40.0),
            (60.0, 40.0),
            (60.0, 230.0),
        ];
        chart.edge("D", "T", &around);
        assert_eq!(chart.route(), around.to_vec());
    }

    #[test]
    fn only_a_visible_branch_of_a_decision_to_a_node_turns() {
        let mut chart = Chart::new("TB", Flow::Down);
        chart.node("D", "rect", (200.0, 100.0), (150.0, 60.0));
        chart.node("T", "rect", (60.0, 250.0), (100.0, 40.0));
        chart.edge("D", "T", &Z);
        assert_eq!(chart.route(), Z.to_vec());
        let mut chart = down();
        chart.data.edges[0].thickness = "invisible".to_owned();
        assert_eq!(chart.route(), Z.to_vec());
        let mut chart = down();
        chart.data.nodes[1].is_group = true;
        assert_eq!(chart.route(), Z.to_vec());
    }

    #[test]
    fn a_branch_keeps_its_route_when_the_new_one_would_pass_a_node_or_a_label() {
        let mut chart = down();
        chart.node("N", "rect", (95.0, 100.0), (20.0, 20.0));
        assert_eq!(chart.route(), Z.to_vec());
        let mut chart = down();
        chart.node("A", "rect", (300.0, 250.0), (60.0, 40.0));
        chart.edge(
            "D",
            "A",
            &[
                (210.0, 130.0),
                (210.0, 180.0),
                (300.0, 180.0),
                (300.0, 230.0),
            ],
        );
        chart.label(1, (80.0, 100.0), (30.0, 17.0));
        assert_eq!(chart.route(), Z.to_vec());
    }

    #[test]
    fn a_branch_stays_in_its_group_and_out_of_the_others() {
        // Both ends in one group: the new route stays inside it.
        let mut chart = down();
        chart.group("G", (150.0, 170.0), (300.0, 260.0));
        for node in &mut chart.data.nodes[..2] {
            node.parent_id = Some("G".to_owned());
        }
        assert_eq!(chart.route(), L.to_vec());
        // A group of neither end on the way.
        let mut chart = down();
        chart.group("G", (90.0, 120.0), (40.0, 40.0));
        assert_eq!(chart.route(), Z.to_vec());
        // The ends in different groups.
        let mut chart = down();
        chart.group("G", (60.0, 250.0), (140.0, 80.0));
        chart.data.nodes[1].parent_id = Some("G".to_owned());
        assert_eq!(chart.route(), Z.to_vec());
        // A group with a direction of its own.
        let mut chart = down();
        chart.group("G", (150.0, 170.0), (300.0, 260.0));
        chart.data.nodes[2].dir = Some("LR".to_owned());
        for node in &mut chart.data.nodes[..2] {
            node.parent_id = Some("G".to_owned());
        }
        assert_eq!(chart.route(), Z.to_vec());
    }

    /// A decision with two branches to the left: `F` farther, `N` nearer.
    fn two_on_one_side(far: &[(f64, f64)]) -> Chart {
        let mut chart = Chart::new("TB", Flow::Down);
        chart.node("D", "diamond", (400.0, 100.0), (150.0, 60.0));
        chart.node("F", "rect", (60.0, 250.0), (100.0, 40.0));
        chart.node("N", "rect", (200.0, 250.0), (100.0, 40.0));
        chart.edge("D", "F", far);
        chart.edge("D", "N", &NEAR);
        chart
    }

    const NEAR: [(f64, f64); 4] = [
        (390.0, 130.0),
        (390.0, 190.0),
        (200.0, 190.0),
        (200.0, 230.0),
    ];

    #[test]
    fn one_branch_takes_a_tip_the_nearest_unless_it_would_cross_more_edges() {
        // The nearer one's new route would cross the farther one's old route, which its old route
        // does not: the farther one takes the tip, the nearer one keeps its route.
        let far = [(380.0, 130.0), (380.0, 170.0), (60.0, 170.0), (60.0, 230.0)];
        let routes = two_on_one_side(&far).routes();
        assert_eq!(
            routes[0],
            vec![(325.0, 100.0), (60.0, 100.0), (60.0, 230.0)]
        );
        assert_eq!(routes[1], NEAR.to_vec());
        // Where the old routes cross already, the nearer one takes it.
        let far = [(380.0, 130.0), (380.0, 210.0), (60.0, 210.0), (60.0, 230.0)];
        let routes = two_on_one_side(&far).routes();
        assert_eq!(routes[0], far.to_vec());
        assert_eq!(
            routes[1],
            vec![(325.0, 100.0), (200.0, 100.0), (200.0, 230.0)]
        );
    }

    #[test]
    fn a_self_loop_takes_the_tip_it_stands_at_but_not_one_beside_the_flow() {
        // A loop at the left tip, out of the decision's side and back.
        let mut at_tip = down();
        at_tip.edge(
            "D",
            "D",
            &[(125.0, 90.0), (93.0, 90.0), (93.0, 110.0), (125.0, 110.0)],
        );
        assert_eq!(at_tip.route(), Z.to_vec());
        // The branch would take the tip were the loop not there.
        assert_eq!(wanted_tips(&at_tip.data, "D"), (true, false));
        // A loop below the decision, on the side that faces the flow, leaves both tips free.
        for (direction, flow) in FLOWS {
            let mut below = chart(direction, flow);
            below.edge(
                "D",
                "D",
                &[
                    (190.0, 130.0),
                    (190.0, 162.0),
                    (210.0, 162.0),
                    (210.0, 130.0),
                ],
            );
            assert_eq!(below.route(), L.to_vec(), "{direction}");
            assert_eq!(wanted_tips(&below.data, "D"), (true, false), "{direction}");
        }
    }
}
