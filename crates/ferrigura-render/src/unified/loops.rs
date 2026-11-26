//! The self-loops of the design mode: one loop on a free side of its node, around the middle of
//! the side, with its label beside it (`docs/decisions/0023-design-mode.md`). The step is
//! ferrigura's own, run around ELK's layout; the ELK port does not know it.
//!
//! ELK routes a self-loop as a small hook near a corner of its node and places its label on the far
//! side of the hook, where it floats away from the line. In the design mode the loops do not take
//! part in the layout:
//!
//! 1. **Out.** Before the layout, the loops leave the layout data. Their node takes their room
//!    instead: its box grows along its layer by the room of a loop and its label on both sides, so
//!    that the node stays in the middle of its box, where ELK and the placement put its other
//!    edges, and along the flow where the label or the cardinalities need more than the node's
//!    size.
//! 2. **Back.** After ELK's layout, before the result is applied, the node takes its own size in
//!    the middle of its box, the ends of its other edges move along their last part onto its
//!    outline, and each loop goes into the room beside it: out of the side [`PORT_PITCH`] apart
//!    around the middle, [`LOOP_REACH`] out and back, with its label [`LOOP_LABEL_GAP`] beside
//!    it, centered on the loop. The result is applied as ELK's own route would be, so the ends
//!    are clipped at the outline of the node's shape and leave room for their markers.
//! 3. **Check.** Once applied, a loop, its label or a cardinality that touches a node, an edge, a
//!    label or a frame, or that the node's groups do not hold, makes the diagram be laid out again
//!    with that node's loops on the sides of the other direction, and then with its loops left to
//!    ELK. The same holds where an end of another edge does not leave the node's box straight.
//!
//! In a layout from top to bottom (or back), the edges meet a node at the top and the bottom, so
//! the first loop stands on the right and a second one on the left, each with its label on its far
//! side. In a layout from left to right (or back), the first loop stands on the top and a second
//! one at the bottom, each with its label at its right. A side that a state's note takes is not
//! free ([`Reserved`]). A node with more loops than free sides, an invisible loop, or a side too
//! short for the ends of a loop, and a group, keep their loops where ELK puts them.

use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::{JsonEdge, JsonEdgeSection, JsonLabel, JsonNode, JsonPoint};

use crate::design::{CHIP_MARGIN, LOOP_LABEL_GAP, LOOP_REACH, PORT_PITCH};
use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::elk_build::ElkState;
use crate::unified::elk_geometry::find_common_ancestor;
use crate::unified::geom::{BBox, Point};
use crate::unified::room::{
    absolute_boxes, contains, crosses, ends_onto_outlines, find_mut, grown, overlaps,
};

/// The least distance between a loop, its label or a cardinality and anything else.
const CLEARANCE: f64 = 4.0;

/// The least room between an end of a loop and a corner of its side.
const END_ROOM: f64 = 4.0;

/// The least distance between the ends of a loop. A node whose side is shorter than twice this
/// keeps its loops where ELK puts them.
const MIN_PITCH: f64 = 8.0;

/// The room a cardinality takes beside the end of a loop, besides its own size: the half width
/// of the widest marker and the gap of `terminal::place_beside_ends`.
const TERMINAL_ROOM: f64 = 12.0;

/// A node whose box another step of the design mode has grown already, for something that stands
/// beside it: a state with its notes. Its loops take the sides that are left.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reserved {
    pub(crate) id: String,
    /// The node's own size.
    pub(crate) width: f64,
    pub(crate) height: f64,
    /// Whether the side before the node along its layer is taken: the left, or the top in a
    /// layout from left to right.
    pub(crate) before: bool,
    /// Whether the side after it is taken: the right, or the bottom.
    pub(crate) after: bool,
}

/// The side of its node a loop stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Right,
    Left,
    Top,
    Bottom,
}

impl Side {
    /// The sides a node's loops take, in order: the sides its other edges do not meet.
    fn of(across: bool) -> [Self; 2] {
        if across {
            [Self::Right, Self::Left]
        } else {
            [Self::Top, Self::Bottom]
        }
    }

    /// Whether the side comes before the node along its layer: the left, or the top.
    fn is_before(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }

    /// The unit vector out of the node.
    fn out(self) -> Point {
        match self {
            Self::Right => Point::new(1.0, 0.0),
            Self::Left => Point::new(-1.0, 0.0),
            Self::Top => Point::new(0.0, -1.0),
            Self::Bottom => Point::new(0.0, 1.0),
        }
    }

    /// The unit vector along the side, from the start of a loop to its end.
    fn along(self) -> Point {
        match self {
            Self::Right | Self::Left => Point::new(0.0, 1.0),
            Self::Top | Self::Bottom => Point::new(1.0, 0.0),
        }
    }

    /// The middle of the side of a box.
    fn middle(self, bbox: BBox) -> Point {
        let centre = bbox.center();
        match self {
            Self::Right => Point::new(bbox.right(), centre.y),
            Self::Left => Point::new(bbox.x, centre.y),
            Self::Top => Point::new(centre.x, bbox.y),
            Self::Bottom => Point::new(centre.x, bbox.bottom()),
        }
    }
}

/// A loop beside its node.
#[derive(Debug, Clone, PartialEq)]
struct Loop {
    edge: String,
    side: Side,
    /// The distance between its ends: [`PORT_PITCH`], or half the side of a node too small for it.
    pitch: f64,
    /// The size of its label's box, the chip, when it has a label.
    chip: Option<(f64, f64)>,
    /// The largest size of a cardinality at its start and at its end.
    start_terminal: Option<(f64, f64)>,
    end_terminal: Option<(f64, f64)>,
}

impl Loop {
    /// How far the label stands right of the end of a loop on the top or the bottom: past the
    /// cardinality at the end, which stands right of it.
    fn label_shift(&self) -> f64 {
        self.end_terminal
            .map_or(0.0, |(width, _)| width + TERMINAL_ROOM)
    }

    /// The room the loop and its label take out of the side, and half the room they take along it.
    fn room(&self) -> (f64, f64) {
        let terminals = [self.start_terminal, self.end_terminal]
            .into_iter()
            .flatten();
        match self.side {
            Side::Right | Side::Left => {
                let reach = LOOP_REACH
                    + self
                        .chip
                        .map_or(0.0, |(width, _)| LOOP_LABEL_GAP + width + CHIP_MARGIN);
                let half = terminals
                    .map(|(_, height)| self.pitch / 2.0 + TERMINAL_ROOM + height)
                    .fold(self.pitch / 2.0 + END_ROOM, f64::max)
                    .max(self.chip.map_or(0.0, |(_, height)| height / 2.0));
                (reach, half)
            }
            Side::Top | Side::Bottom => {
                let reach = terminals
                    .clone()
                    .map(|(_, height)| LOOP_LABEL_GAP + height)
                    .fold(LOOP_REACH, f64::max)
                    .max(
                        self.chip
                            .map_or(0.0, |(_, height)| LOOP_REACH / 2.0 + height / 2.0),
                    );
                let half = terminals
                    .map(|(width, _)| self.pitch / 2.0 + TERMINAL_ROOM + width)
                    .fold(self.pitch / 2.0 + END_ROOM, f64::max)
                    .max(self.chip.map_or(0.0, |(width, _)| {
                        self.pitch / 2.0 + self.label_shift() + LOOP_LABEL_GAP + width + CHIP_MARGIN
                    }));
                (reach, half)
            }
        }
    }

    /// The route of the loop beside the node's own box: out of the side, along it and back in.
    fn route(&self, real: BBox) -> [Point; 4] {
        let (middle, out, along) = (self.side.middle(real), self.side.out(), self.side.along());
        let at = |step: f64, reach: f64| {
            Point::new(
                middle.x + along.x * step + out.x * reach,
                middle.y + along.y * step + out.y * reach,
            )
        };
        let half = self.pitch / 2.0;
        [
            at(-half, 0.0),
            at(-half, LOOP_REACH),
            at(half, LOOP_REACH),
            at(half, 0.0),
        ]
    }

    /// The center of the label's box beside the loop: past the loop on the right and the left,
    /// at the right of it on the top and the bottom, centered on the loop.
    fn label_centre(&self, real: BBox, chip_width: f64) -> Point {
        let middle = self.side.middle(real);
        let out = self.side.out();
        match self.side {
            Side::Right | Side::Left => {
                let distance = LOOP_REACH + LOOP_LABEL_GAP + chip_width / 2.0;
                Point::new(middle.x + out.x * distance, middle.y)
            }
            Side::Top | Side::Bottom => Point::new(
                middle.x
                    + self.pitch / 2.0
                    + self.label_shift()
                    + LOOP_LABEL_GAP
                    + chip_width / 2.0,
                middle.y + out.y * LOOP_REACH / 2.0,
            ),
        }
    }
}

/// A node with loops beside it.
#[derive(Debug, Clone)]
struct Host {
    id: String,
    /// The node's own size, which its box in the layout exceeds.
    width: f64,
    height: f64,
    /// Whether its layer runs along x: a layout from top to bottom or back.
    across: bool,
    loops: Vec<Loop>,
}

impl Host {
    /// The size of the node's box in the layout: its own size, with the room of its loops on both
    /// sides along its layer, and along the flow as much as the loops need.
    fn room(&self) -> (f64, f64) {
        let (mut reach, mut half) = (0.0_f64, 0.0_f64);
        for entry in &self.loops {
            let (out, along) = entry.room();
            reach = reach.max(out);
            half = half.max(along);
        }
        // The box grows by whole units on each side, so that the middle of its side, where ELK and
        // the placement put the ports on whole units, is where it is for the node alone.
        let reach = reach.ceil();
        let grow = |size: f64| size + 2.0 * (half - size / 2.0).max(0.0).ceil();
        if self.across {
            (self.width + 2.0 * reach, grow(self.height))
        } else {
            (grow(self.width), self.height + 2.0 * reach)
        }
    }
}

/// The loops taken out of the layout data, and what puts them back.
#[derive(Debug, Clone, Default)]
pub(crate) struct Loops {
    hosts: Vec<Host>,
    /// The loops taken out, at their positions in the layout data.
    edges: Vec<(usize, Edge)>,
}

/// The box of an edge's label as the design mode draws it, its chip, when it has a label.
fn chip_of(edge: &Edge) -> Option<(f64, f64)> {
    if edge.label.is_empty() {
        return None;
    }
    let (width, height) = (edge.width?, edge.height?);
    (width > 0.0 && height > 0.0).then_some(((width - 2.0 * CHIP_MARGIN).max(0.0), height))
}

/// The largest size of the cardinalities at one end of an edge.
fn terminal_at(edge: &Edge, at_start: bool) -> Option<(f64, f64)> {
    edge.terminals
        .iter()
        .filter(|terminal| terminal.key.at_start() == at_start)
        .map(|terminal| (terminal.width, terminal.height))
        .reduce(|a, b| (a.0.max(b.0), a.1.max(b.1)))
}

/// The boxes of an edge's label and its cardinalities, as they are drawn.
fn label_boxes(edge: &Edge) -> Vec<BBox> {
    let chip = chip_of(edge).and_then(|(width, height)| {
        Some(BBox::new(
            edge.x? - width / 2.0,
            edge.y? - height / 2.0,
            width,
            height,
        ))
    });
    chip.into_iter()
        .chain(edge.terminals.iter().filter_map(|terminal| {
            let centre = terminal.centre?;
            Some(BBox::new(
                centre.x - terminal.width / 2.0,
                centre.y - terminal.height / 2.0,
                terminal.width,
                terminal.height,
            ))
        }))
        .collect()
}

/// The part of `room` outside the side of `node` that a loop on `side` stands on.
fn outside(room: BBox, node: BBox, side: Side) -> BBox {
    let (mut low, mut high) = (
        Point::new(room.x, room.y),
        Point::new(room.right(), room.bottom()),
    );
    match side {
        Side::Right => low.x = low.x.max(node.right()),
        Side::Left => high.x = high.x.min(node.x),
        Side::Top => high.y = high.y.min(node.y),
        Side::Bottom => low.y = low.y.max(node.bottom()),
    }
    BBox::new(
        low.x,
        low.y,
        (high.x - low.x).max(0.0),
        (high.y - low.y).max(0.0),
    )
}

/// Takes the loops that can stand beside their node out of the layout data and grows their nodes
/// by their room. `None` where no loop can: then the data is unchanged. The nodes of `turned` take
/// the sides of the other direction: the edges of a node in a group with a direction of its own
/// may meet it in the direction of the group's parent. The nodes of `kept_hosts`
/// keep their loops where ELK puts them; the nodes of `reserved` keep their sides taken.
pub(crate) fn take_out(
    data: &mut LayoutData,
    kept_hosts: &HashSet<String>,
    turned: &HashSet<String>,
    reserved: &[Reserved],
) -> Option<Loops> {
    if !data.edges.iter().any(|edge| edge.start == edge.end) {
        return None;
    }
    let by_id: HashMap<&str, &Node> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let index: HashMap<&str, usize> = data
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.as_str(), position))
        .collect();

    let mut hosts: Vec<Host> = Vec::new();
    let mut unusable: HashSet<&str> = HashSet::new();
    for edge in data.edges.iter().filter(|edge| edge.start == edge.end) {
        let Some(node) = by_id.get(edge.start.as_str()).copied() else {
            continue;
        };
        let taken = reserved.iter().find(|entry| entry.id == node.id);
        let (Some(width), Some(height)) = (
            taken
                .map_or(node.width, |entry| Some(entry.width))
                .filter(|width| *width > 0.0),
            taken
                .map_or(node.height, |entry| Some(entry.height))
                .filter(|height| *height > 0.0),
        ) else {
            unusable.insert(node.id.as_str());
            continue;
        };
        if node.is_group || edge.thickness == "invisible" || kept_hosts.contains(&node.id) {
            unusable.insert(node.id.as_str());
            continue;
        }
        let host = match hosts.iter().position(|host| host.id == node.id) {
            Some(position) => &mut hosts[position],
            None => {
                let direction =
                    crate::unified::pipeline::node_dir(data, index[node.id.as_str()], &by_id)
                        .unwrap_or(data.direction.as_str());
                hosts.push(Host {
                    id: node.id.clone(),
                    width,
                    height,
                    across: matches!(direction, "LR" | "RL") == turned.contains(&node.id),
                    loops: Vec::new(),
                });
                hosts.last_mut()?
            }
        };
        let sides: Vec<Side> = Side::of(host.across)
            .into_iter()
            .filter(|side| {
                !taken.is_some_and(|entry| {
                    if side.is_before() {
                        entry.before
                    } else {
                        entry.after
                    }
                })
            })
            .collect();
        let Some(&side) = sides.get(host.loops.len()) else {
            unusable.insert(node.id.as_str());
            continue;
        };
        let length = if host.across { host.height } else { host.width };
        host.loops.push(Loop {
            edge: edge.id.clone(),
            side,
            pitch: PORT_PITCH.min(length / 2.0),
            chip: chip_of(edge),
            start_terminal: terminal_at(edge, true),
            end_terminal: terminal_at(edge, false),
        });
    }
    hosts.retain(|host| {
        let side = if host.across { host.height } else { host.width };
        !unusable.contains(host.id.as_str()) && side >= 2.0 * MIN_PITCH
    });
    if hosts.is_empty() {
        return None;
    }

    for host in &hosts {
        let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) else {
            continue;
        };
        // A box grown for something else beside the node holds the loops' room as well.
        let (width, height) = host.room();
        node.width = Some(node.width.unwrap_or(0.0).max(width));
        node.height = Some(node.height.unwrap_or(0.0).max(height));
    }
    let moved: HashSet<String> = hosts
        .iter()
        .flat_map(|host| host.loops.iter().map(|entry| entry.edge.clone()))
        .collect();
    let mut loops = Loops {
        hosts,
        ..Loops::default()
    };
    for (position, edge) in std::mem::take(&mut data.edges).into_iter().enumerate() {
        if moved.contains(&edge.id) {
            loops.edges.push((position, edge));
        } else {
            data.edges.push(edge);
        }
    }
    Some(loops)
}

impl Loops {
    /// Puts the loops back into ELK's result and into the layout data: each node takes its own
    /// size in the middle of its box, the ends of its other edges move onto its outline, and its
    /// loops stand beside it with their labels. Where an end of another edge does not leave a
    /// node's box straight from a side, the result is that node's id.
    pub(crate) fn put_back(
        &self,
        graph: &mut JsonNode,
        data: &mut LayoutData,
        state: &ElkState,
    ) -> Result<(), String> {
        let mut boxes: HashMap<String, (BBox, String)> = HashMap::new();
        absolute_boxes(graph, Point::default(), &mut boxes);

        // The node's box in the layout and its own box in the middle of it.
        let mut own: HashMap<&str, (BBox, BBox)> = HashMap::new();
        for host in &self.hosts {
            let Some((laid, _)) = boxes.get(&host.id) else {
                return Err(host.id.clone());
            };
            let real = BBox::new(
                laid.x + (laid.width - host.width) / 2.0,
                laid.y + (laid.height - host.height) / 2.0,
                host.width,
                host.height,
            );
            own.insert(host.id.as_str(), (*laid, real));
        }
        ends_onto_outlines(graph, &boxes, &own, &state.parent_by_id)?;

        for (position, edge) in &self.edges {
            data.edges
                .insert((*position).min(data.edges.len()), edge.clone());
        }
        for host in &self.hosts {
            if let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) {
                node.width = Some(host.width);
                node.height = Some(host.height);
            }
            let (_, real) = own[host.id.as_str()];
            if let Some(node) = find_mut(graph, &host.id) {
                node.x =
                    Some(node.x.unwrap_or(0.0) + (node.width.unwrap_or(0.0) - real.width) / 2.0);
                node.y =
                    Some(node.y.unwrap_or(0.0) + (node.height.unwrap_or(0.0) - real.height) / 2.0);
                node.width = Some(real.width);
                node.height = Some(real.height);
            }
            // The sections of a loop are relative to its node's parent, as ELK gives them.
            let offset = find_common_ancestor(&host.id, &host.id, &state.parent_by_id)
                .and_then(|parent| boxes.get(&parent))
                .map_or_else(Point::default, |(bbox, _)| Point::new(bbox.x, bbox.y));
            let relative = |point: Point| JsonPoint {
                x: point.x - offset.x,
                y: point.y - offset.y,
            };
            for entry in &host.loops {
                let Some((_, edge)) = self.edges.iter().find(|(_, edge)| edge.id == entry.edge)
                else {
                    continue;
                };
                let [start, first, second, end] = entry.route(real);
                // The label's box in the layout around the center of its chip, or an empty label
                // at the middle of the loop.
                let (width, height) = match entry.chip {
                    Some(_) => (edge.width.unwrap_or(0.0), edge.height.unwrap_or(0.0)),
                    None => (0.0, 0.0),
                };
                let centre = match entry.chip {
                    Some((chip_width, _)) => entry.label_centre(real, chip_width),
                    None => Point::new((first.x + second.x) / 2.0, (first.y + second.y) / 2.0),
                };
                let corner = relative(Point::new(centre.x - width / 2.0, centre.y - height / 2.0));
                graph.edges.push(JsonEdge {
                    id: edge.id.clone(),
                    sources: vec![host.id.clone()],
                    targets: vec![host.id.clone()],
                    labels: vec![JsonLabel {
                        text: Some(edge.label.clone()),
                        x: Some(corner.x),
                        y: Some(corner.y),
                        width: Some(width),
                        height: Some(height),
                        ..JsonLabel::default()
                    }],
                    sections: vec![JsonEdgeSection {
                        id: format!("{}_s0", edge.id),
                        start_point: Some(relative(start)),
                        end_point: Some(relative(end)),
                        bend_points: vec![relative(first), relative(second)],
                        ..JsonEdgeSection::default()
                    }],
                    ..JsonEdge::default()
                });
            }
        }
        Ok(())
    }

    /// The first node with a loop that does not stand clear of everything it does not belong to,
    /// or not inside the frames of its node's groups, in the applied layout; `None` where every
    /// loop does.
    pub(crate) fn blocked(&self, data: &LayoutData) -> Option<String> {
        let by_id: HashMap<&str, &Node> = data
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let frame = |node: &Node| {
            let (width, height) = (node.width.unwrap_or(0.0), node.height.unwrap_or(0.0));
            BBox::new(
                node.x.unwrap_or(0.0) - width / 2.0,
                node.y.unwrap_or(0.0) - height / 2.0,
                width,
                height,
            )
        };
        for host in &self.hosts {
            // The groups that hold the node.
            let mut holders = HashSet::new();
            let mut parent = by_id
                .get(host.id.as_str())
                .and_then(|node| node.parent_id.as_deref());
            while let Some(id) = parent {
                if !holders.insert(id) {
                    break;
                }
                parent = by_id.get(id).and_then(|node| node.parent_id.as_deref());
            }
            for entry in &host.loops {
                let Some(edge) = data.edges.iter().find(|edge| edge.id == entry.edge) else {
                    return Some(host.id.clone());
                };
                let Some(path) = BBox::from_points(edge.points.iter().copied()) else {
                    return Some(host.id.clone());
                };
                let own_labels: Vec<BBox> = label_boxes(edge);
                // The loop's own labels stand apart and off its node.
                let node_box = by_id.get(host.id.as_str()).map(|node| frame(node));
                for (position, label) in own_labels.iter().enumerate() {
                    if node_box.is_some_and(|bbox| overlaps(bbox, *label))
                        || own_labels[position + 1..]
                            .iter()
                            .any(|other| overlaps(*other, *label))
                    {
                        return Some(host.id.clone());
                    }
                }
                // The loop's room ends at the side of its node: the edges that meet the node
                // elsewhere may come close to the loop's ends inside the node's box.
                let path = node_box.map_or_else(
                    || grown(path, CLEARANCE),
                    |bbox| outside(grown(path, CLEARANCE), bbox, entry.side),
                );
                let items: Vec<BBox> = std::iter::once(path)
                    .chain(own_labels.iter().map(|bbox| grown(*bbox, CLEARANCE)))
                    .collect();
                for room in &items {
                    for other in &data.nodes {
                        if other.id == host.id {
                            continue;
                        }
                        let bbox = frame(other);
                        // A note's group draws nothing; its notes count as nodes.
                        let fine = if !other.is_group {
                            !overlaps(bbox, *room)
                        } else if other.shape == "noteGroup" {
                            true
                        } else if holders.contains(other.id.as_str()) {
                            contains(bbox, *room)
                        } else {
                            !overlaps(bbox, *room)
                        };
                        if !fine {
                            return Some(host.id.clone());
                        }
                    }
                    for other in data.edges.iter().filter(|other| other.id != edge.id) {
                        if other
                            .points
                            .windows(2)
                            .any(|pair| crosses(pair[0], pair[1], *room))
                            || label_boxes(other)
                                .into_iter()
                                .any(|bbox| overlaps(bbox, *room))
                        {
                            return Some(host.id.clone());
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looped(side: Side, chip: Option<(f64, f64)>) -> Loop {
        Loop {
            edge: "e".to_owned(),
            side,
            pitch: PORT_PITCH,
            chip,
            start_terminal: None,
            end_terminal: None,
        }
    }

    #[test]
    fn a_loop_on_the_right_leaves_and_enters_around_the_middle_of_the_side() {
        let real = BBox::new(0.0, 0.0, 120.0, 44.0);
        let route = looped(Side::Right, None).route(real);
        assert_eq!(
            route,
            [
                Point::new(120.0, 12.0),
                Point::new(152.0, 12.0),
                Point::new(152.0, 32.0),
                Point::new(120.0, 32.0),
            ]
        );
    }

    #[test]
    fn a_loop_on_the_top_runs_from_left_to_right() {
        let real = BBox::new(0.0, 0.0, 120.0, 44.0);
        let route = looped(Side::Top, None).route(real);
        assert_eq!(
            route,
            [
                Point::new(50.0, 0.0),
                Point::new(50.0, -32.0),
                Point::new(70.0, -32.0),
                Point::new(70.0, 0.0),
            ]
        );
    }

    #[test]
    fn the_label_stands_beside_the_loop_centered_on_it() {
        let real = BBox::new(0.0, 0.0, 120.0, 44.0);
        let right = looped(Side::Right, Some((40.0, 24.0)));
        assert_eq!(right.label_centre(real, 40.0), Point::new(176.0, 22.0));
        let left = looped(Side::Left, Some((40.0, 24.0)));
        assert_eq!(left.label_centre(real, 40.0), Point::new(-56.0, 22.0));
        let top = looped(Side::Top, Some((40.0, 24.0)));
        assert_eq!(top.label_centre(real, 40.0), Point::new(94.0, -16.0));
    }

    #[test]
    fn a_node_takes_the_room_of_its_loop_on_both_sides_of_its_layer() {
        let host = Host {
            id: "a".to_owned(),
            width: 120.0,
            height: 44.0,
            across: true,
            loops: vec![looped(Side::Right, Some((40.0, 24.0)))],
        };
        // 32 for the loop, 4 of gap, the chip of 40 and its margin of 6.
        assert_eq!(host.room(), (120.0 + 2.0 * 82.0, 44.0));
        let tall = Host {
            loops: vec![looped(Side::Right, Some((40.0, 60.0)))],
            ..host
        };
        assert_eq!(tall.room(), (284.0, 60.0));
    }

    fn node(id: &str, width: f64, height: f64) -> Node {
        Node {
            id: id.to_owned(),
            width: Some(width),
            height: Some(height),
            ..Node::default()
        }
    }

    fn edge(id: &str, start: &str, end: &str) -> Edge {
        Edge {
            id: id.to_owned(),
            start: start.to_owned(),
            end: end.to_owned(),
            ..Edge::default()
        }
    }

    #[test]
    fn the_loops_leave_the_layout_and_their_node_takes_their_room() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0), node("b", 120.0, 44.0)],
            edges: vec![edge("l", "a", "a"), edge("e", "a", "b")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let loops = take_out(&mut data, &HashSet::new(), &HashSet::new(), &[])
            .expect("a loop beside its node");
        assert_eq!(data.edges.len(), 1);
        assert_eq!(data.edges[0].id, "e");
        assert_eq!(data.nodes[0].width, Some(120.0 + 2.0 * 32.0));
        assert_eq!(loops.edges.len(), 1);
        assert_eq!(loops.hosts[0].loops[0].side, Side::Right);
    }

    #[test]
    fn a_second_loop_takes_the_other_free_side_and_a_third_leaves_them_to_elk() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: vec![edge("l1", "a", "a"), edge("l2", "a", "a")],
            direction: "LR".to_owned(),
            ..LayoutData::default()
        };
        let loops = take_out(&mut data, &HashSet::new(), &HashSet::new(), &[])
            .expect("two loops beside their node");
        let sides: Vec<Side> = loops.hosts[0].loops.iter().map(|l| l.side).collect();
        assert_eq!(sides, vec![Side::Top, Side::Bottom]);

        let mut three = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: vec![
                edge("l1", "a", "a"),
                edge("l2", "a", "a"),
                edge("l3", "a", "a"),
            ],
            direction: "LR".to_owned(),
            ..LayoutData::default()
        };
        assert!(take_out(&mut three, &HashSet::new(), &HashSet::new(), &[]).is_none());
        assert_eq!(three.edges.len(), 3);
    }

    #[test]
    fn a_loop_takes_the_side_a_note_leaves_free_in_the_box_the_note_grew() {
        // The note's step grew the state from 120 by 100 on both sides.
        let mut data = LayoutData {
            nodes: vec![node("a", 320.0, 44.0)],
            edges: vec![edge("l", "a", "a")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let reserved = [Reserved {
            id: "a".to_owned(),
            width: 120.0,
            height: 44.0,
            before: false,
            after: true,
        }];
        let loops =
            take_out(&mut data, &HashSet::new(), &HashSet::new(), &reserved).expect("a loop");
        assert_eq!(loops.hosts[0].loops[0].side, Side::Left);
        assert_eq!(loops.hosts[0].width, 120.0);
        // The larger room of the two holds both.
        assert_eq!(data.nodes[0].width, Some(320.0));
    }

    #[test]
    fn a_turned_node_takes_the_sides_of_the_other_direction() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: vec![edge("l", "a", "a")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let turned = HashSet::from(["a".to_owned()]);
        let loops = take_out(&mut data, &HashSet::new(), &turned, &[]).expect("a loop");
        assert_eq!(loops.hosts[0].loops[0].side, Side::Top);
    }

    #[test]
    fn the_room_of_a_loop_ends_at_the_side_of_its_node() {
        let node = BBox::new(0.0, 0.0, 100.0, 40.0);
        let room = BBox::new(90.0, 6.0, 46.0, 28.0);
        assert_eq!(
            outside(room, node, Side::Right),
            BBox::new(100.0, 6.0, 36.0, 28.0)
        );
        assert_eq!(
            outside(BBox::new(40.0, -36.0, 28.0, 40.0), node, Side::Top),
            BBox::new(40.0, -36.0, 28.0, 36.0)
        );
    }

    #[test]
    fn a_kept_or_too_small_node_keeps_its_loop_where_elk_puts_it() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0), node("dot", 14.0, 14.0)],
            edges: vec![edge("l", "a", "a"), edge("m", "dot", "dot")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let kept = HashSet::from(["a".to_owned()]);
        assert!(take_out(&mut data, &kept, &HashSet::new(), &[]).is_none());
        assert_eq!(data.edges.len(), 2);
    }
}
