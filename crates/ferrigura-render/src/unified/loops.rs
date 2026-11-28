//! The self-loops of the default rendering: one loop on a free side of its node, around the middle
//! of the side, with its label beside it (`docs/decisions/0023-the-default-rendering.md`). The step
//! is ferrigura's own, run around ELK's layout; the ELK port does not know it.
//!
//! ELK routes a self-loop as a small hook near a corner of its node and places its label on the far
//! side of the hook, where it floats away from the line. In the default rendering the loops do not
//! take part in the layout:
//!
//! 1. **Out.** Before the layout, the loops leave the layout data. Their node keeps their room
//!    instead, on the loop's side only: the box of a loop with its label and its cardinalities
//!    goes to ELK as a label of the node at the loop's place, which the node's margins hold
//!    (`Node::beside`). The node keeps its own size, and ELK and the placement put its other
//!    edges on its own sides.
//! 2. **Back.** After ELK's layout, before the result is applied, each loop goes into the room
//!    beside its node: out of the side [`PORT_PITCH`] apart
//!    around the middle, [`LOOP_REACH`] out and back, with its label [`LOOP_LABEL_GAP`] beside
//!    it, centered on the loop. The result is applied as ELK's own route would be, so the ends
//!    are clipped at the outline of the node's shape and leave room for their markers.
//! 3. **Check.** Once applied, a loop, its label or a cardinality that touches a node, an edge, a
//!    label or a frame, or that the node's groups do not hold, makes the diagram be laid out again
//!    with that node's loops on the sides of the other direction, and then with its loops left to
//!    ELK.
//!
//! In a layout from top to bottom (or back), the edges meet a node at the top and the bottom, so
//! the first loop stands on the right and a second one on the left, each with its label on its far
//! side. In a layout from left to right (or back), the first loop stands on the top and a second
//! one at the bottom, each with its label at its right. A side that a state's note takes is not
//! free ([`Reserved`]). A decision's loop stays off a side tip that a branch would leave from
//! ([`Loops::wanted`], [`Avoided`]): it takes the other tip, or the side that faces the next layer
//! where the branches need both. More loops than free sides share them, two on a side one after the
//! other along it; where a state's notes take both sides, one loop stands at the end of the side
//! its edges enter ([`assign`]). A group's loop stands on its frame, whose size a layout before
//! gives. A node with more than two loops on a free side, an invisible loop, or a side too short for
//! the ends of a loop keeps its loops where ELK puts them.

use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::{JsonEdge, JsonEdgeSection, JsonLabel, JsonNode, JsonPoint};

use crate::design::{CHIP_MARGIN, LOOP_LABEL_GAP, LOOP_REACH, PORT_PITCH};
use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::elk_build::ElkState;
use crate::unified::elk_geometry::find_common_ancestor;
use crate::unified::geom::{BBox, Point};
use crate::unified::room::{
    absolute_boxes, clear_beside, contains, crosses, find_mut, grown, overlaps,
};
use crate::unified::shapes;

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

/// A node beside which another step of the default rendering keeps room for something of its own: a
/// state with its notes. Its loops take the sides that are left.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reserved {
    pub(crate) id: String,
    /// Whether the side before the node along its layer is taken: the left, or the top in a
    /// layout from left to right.
    pub(crate) before: bool,
    /// Whether the side after it is taken: the right, or the bottom.
    pub(crate) after: bool,
}

/// The side of its node a loop stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
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

    /// The side of a node that faces the next layer in a layout of the direction: the side its
    /// edges leave from.
    fn facing_flow(direction: &str) -> Self {
        match direction {
            "BT" => Self::Top,
            "LR" => Self::Right,
            "RL" => Self::Left,
            _ => Self::Bottom,
        }
    }

    /// Whether a loop on the side stands at a side tip of a decision in a layout whose next layer
    /// the side `flow` faces: on a side across the flow.
    fn is_tip(self, flow: Self) -> bool {
        matches!(
            (self, flow),
            (Self::Left | Self::Right, Self::Top | Self::Bottom)
                | (Self::Top | Self::Bottom, Self::Left | Self::Right)
        )
    }

    /// The length of the side on a box of the size.
    fn length(self, width: f64, height: f64) -> f64 {
        match self {
            Self::Right | Self::Left => height,
            Self::Top | Self::Bottom => width,
        }
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

/// Where the label of a loop on the top or the bottom stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelAt {
    /// At the right of the loop, halfway out.
    After,
    /// At the left of the loop, halfway out: the first of two loops on one side.
    Before,
    /// Past the loop, centered on it: a loop at the end of a side.
    Past,
}

/// A loop beside its node.
#[derive(Debug, Clone, PartialEq)]
struct Loop {
    edge: String,
    side: Side,
    /// The distance between its ends: [`PORT_PITCH`], or half the side of a node too small for it,
    /// or a quarter of it where two loops share the side.
    pitch: f64,
    /// Where its middle stands along the side, from the side's middle.
    offset: f64,
    /// Where its label's middle stands along a side on the right or the left, from the side's
    /// middle: its own middle, or apart from the label of a loop beside it.
    label_offset: f64,
    /// Where its label stands on the top or the bottom; on the right and the left, it stands past
    /// the loop.
    label: LabelAt,
    /// Whether its room reaches as far along the side on both sides of the side's middle, where
    /// the side runs along the flow.
    centered: bool,
    /// The size of its label's box, the chip, when it has a label.
    chip: Option<(f64, f64)>,
    /// The largest size of a cardinality at its start and at its end.
    start_terminal: Option<(f64, f64)>,
    end_terminal: Option<(f64, f64)>,
    /// How far it stands out: [`LOOP_REACH`], or an ER relation's room for its markers.
    reach: f64,
}

impl Loop {
    /// A loop on a side, around the side's middle; [`assign`] sets its side and place.
    fn new(edge: &Edge) -> Self {
        Self {
            edge: edge.id.clone(),
            side: Side::Right,
            pitch: PORT_PITCH,
            offset: 0.0,
            label_offset: 0.0,
            label: LabelAt::After,
            centered: true,
            chip: chip_of(edge),
            start_terminal: terminal_at(edge, true),
            end_terminal: terminal_at(edge, false),
            reach: LOOP_REACH,
        }
    }

    /// How far the label stands beside the end of a loop on the top or the bottom: past the
    /// cardinality at that end, which stands beside it.
    fn label_shift(&self) -> f64 {
        let terminal = if self.label == LabelAt::Before {
            self.start_terminal
        } else {
            self.end_terminal
        };
        terminal.map_or(0.0, |(width, _)| width + TERMINAL_ROOM)
    }

    /// The room the loop and its label take out of the side, and where it starts and ends along
    /// the side, from the side's middle.
    fn extent(&self) -> (f64, f64, f64) {
        let terminals = [self.start_terminal, self.end_terminal]
            .into_iter()
            .flatten();
        let half = self.pitch / 2.0;
        match self.side {
            Side::Right | Side::Left => {
                let reach = self.reach
                    + self
                        .chip
                        .map_or(0.0, |(width, _)| LOOP_LABEL_GAP + width + CHIP_MARGIN);
                let ends = terminals
                    .map(|(_, height)| half + TERMINAL_ROOM + height)
                    .fold(half + END_ROOM, f64::max);
                let (mut low, mut high) = (self.offset - ends, self.offset + ends);
                if let Some((_, height)) = self.chip {
                    low = low.min(self.label_offset - height / 2.0);
                    high = high.max(self.label_offset + height / 2.0);
                }
                (reach, low, high)
            }
            Side::Top | Side::Bottom => {
                let mut reach = terminals
                    .clone()
                    .map(|(_, height)| LOOP_LABEL_GAP + height)
                    .fold(self.reach, f64::max);
                let ends = terminals
                    .map(|(width, _)| half + TERMINAL_ROOM + width)
                    .fold(half + END_ROOM, f64::max);
                let Some((width, height)) = self.chip else {
                    return (reach, self.offset - ends, self.offset + ends);
                };
                let beside = half + self.label_shift() + LOOP_LABEL_GAP + width + CHIP_MARGIN;
                match self.label {
                    LabelAt::After => (
                        reach.max(self.reach / 2.0 + height / 2.0),
                        self.offset - ends,
                        self.offset + ends.max(beside),
                    ),
                    LabelAt::Before => (
                        reach.max(self.reach / 2.0 + height / 2.0),
                        self.offset - ends.max(beside),
                        self.offset + ends,
                    ),
                    LabelAt::Past => {
                        reach = reach.max(self.reach + LOOP_LABEL_GAP + height);
                        let half_label = width / 2.0 + CHIP_MARGIN;
                        (
                            reach,
                            self.offset - ends.max(half_label),
                            self.offset + ends.max(half_label),
                        )
                    }
                }
            }
        }
    }

    /// The room of the loop, its label and its cardinalities beside a node of the given size, from
    /// the node's top left corner: out of its side, and along the side as far as they reach, on
    /// both sides of the side's middle where the loop is centered.
    fn room_box(&self, width: f64, height: f64) -> BBox {
        let (reach, mut low, mut high) = self.extent();
        if self.centered {
            high = high.max(-low);
            low = -high;
        }
        match self.side {
            Side::Right => BBox::new(width, height / 2.0 + low, reach, high - low),
            Side::Left => BBox::new(-reach, height / 2.0 + low, reach, high - low),
            Side::Top => BBox::new(width / 2.0 + low, -reach, high - low, reach),
            Side::Bottom => BBox::new(width / 2.0 + low, height, high - low, reach),
        }
    }

    /// The route of the loop beside the node's own box: out of the side, along it and back in.
    fn route(&self, real: BBox) -> [Point; 4] {
        let (middle, out, along) = (self.side.middle(real), self.side.out(), self.side.along());
        let at = |step: f64, reach: f64| {
            Point::new(
                middle.x + along.x * (self.offset + step) + out.x * reach,
                middle.y + along.y * (self.offset + step) + out.y * reach,
            )
        };
        let half = self.pitch / 2.0;
        [
            at(-half, 0.0),
            at(-half, self.reach),
            at(half, self.reach),
            at(half, 0.0),
        ]
    }

    /// The center of the label's box beside the loop: past the loop on the right and the left,
    /// beside it or past it on the top and the bottom, centered on the loop.
    fn label_centre(&self, real: BBox, (chip_width, chip_height): (f64, f64)) -> Point {
        let middle = self.side.middle(real);
        let out = self.side.out();
        match self.side {
            Side::Right | Side::Left => {
                let distance = self.reach + LOOP_LABEL_GAP + chip_width / 2.0;
                Point::new(middle.x + out.x * distance, middle.y + self.label_offset)
            }
            Side::Top | Side::Bottom => {
                let beside =
                    self.pitch / 2.0 + self.label_shift() + LOOP_LABEL_GAP + chip_width / 2.0;
                match self.label {
                    LabelAt::Past => Point::new(
                        middle.x + self.offset,
                        middle.y + out.y * (self.reach + LOOP_LABEL_GAP + chip_height / 2.0),
                    ),
                    LabelAt::Before | LabelAt::After => {
                        let sign = if self.label == LabelAt::Before {
                            -1.0
                        } else {
                            1.0
                        };
                        Point::new(
                            middle.x + self.offset + sign * beside,
                            middle.y + out.y * self.reach / 2.0,
                        )
                    }
                }
            }
        }
    }
}

/// The distance between the labels of two loops that share a side on the right or the left.
const LABEL_STACK_GAP: f64 = 4.0;

/// The side opposite a side.
fn opposite(side: Side) -> Side {
    match side {
        Side::Right => Side::Left,
        Side::Left => Side::Right,
        Side::Top => Side::Bottom,
        Side::Bottom => Side::Top,
    }
}

/// Gives the loops of a node their sides and places, from the sides they may take in order: one
/// loop around the middle of each side, and where there are more loops than sides, two on a side,
/// one after the other along it, the first sides first. Where no side is free, one loop stands on
/// the side `entry`, at the end of it. `false` where the loops do not fit so.
fn assign(
    host: &mut Host,
    sides: &[Side],
    entry: Option<Side>,
    centered: impl Fn(Side) -> bool,
) -> bool {
    let count = host.loops.len();
    let (width, height) = (host.width, host.height);
    if sides.is_empty() {
        let Some(side) = entry.filter(|_| count == 1) else {
            return false;
        };
        let length = side.length(width, height);
        let entry_loop = &mut host.loops[0];
        entry_loop.side = side;
        entry_loop.pitch = PORT_PITCH.min(length / 4.0);
        entry_loop.offset = length / 2.0 - END_ROOM - entry_loop.pitch / 2.0;
        entry_loop.label_offset = entry_loop.offset;
        entry_loop.label = LabelAt::Past;
        entry_loop.centered = centered(side);
        return true;
    }
    if count > 2 * sides.len() {
        return false;
    }
    let mut next = 0;
    for (index, &side) in sides.iter().enumerate() {
        let shared = count / sides.len() + usize::from(index < count % sides.len());
        let length = side.length(width, height);
        let members = next..next + shared;
        next += shared;
        if shared == 1 {
            let only = &mut host.loops[members.start];
            only.side = side;
            only.pitch = PORT_PITCH.min(length / 2.0);
            only.centered = centered(side);
        } else if shared == 2 {
            // One after the other along the side, a quarter of it from its middle, their labels
            // apart past them on the right and the left, at their outer ends on the top and the
            // bottom.
            let (first, second) = (members.start, members.start + 1);
            let quarter = length / 4.0;
            let heights = [first, second]
                .map(|index| host.loops[index].chip.map_or(0.0, |(_, height)| height));
            let apart = ((heights[0] + heights[1]) / 2.0 + LABEL_STACK_GAP).max(2.0 * quarter);
            for (index, sign) in [(first, -1.0), (second, 1.0)] {
                let entry = &mut host.loops[index];
                entry.side = side;
                entry.pitch = PORT_PITCH.min(quarter);
                entry.offset = sign * quarter;
                entry.label_offset = sign * apart / 2.0;
                if sign < 0.0 {
                    entry.label = LabelAt::Before;
                }
                entry.centered = centered(side);
            }
        }
    }
    true
}

/// A node with loops beside it.
#[derive(Debug, Clone)]
struct Host {
    id: String,
    /// The node's own size.
    width: f64,
    height: f64,
    /// Whether its layer runs along x: a layout from top to bottom or back.
    across: bool,
    /// The side that faces the next layer in the node's direction.
    flow: Side,
    loops: Vec<Loop>,
}

/// The sides that the loops of decisions stay off, because a branch would leave from the side tip
/// there ([`Loops::wanted`]); see [`take_out`].
#[derive(Debug, Clone, Default)]
pub(crate) struct Avoided {
    sides: HashMap<String, Vec<Side>>,
    /// The decisions whose loops went back to their first sides, since the sides left did not
    /// hold them.
    settled: HashSet<String>,
}

impl Avoided {
    fn of(&self, id: &str) -> &[Side] {
        self.sides.get(id).map_or(&[], Vec::as_slice)
    }

    /// Adds sides for a decision's loops to stay off; whether any is new.
    pub(crate) fn add(&mut self, id: String, sides: Vec<Side>) -> bool {
        if self.settled.contains(&id) {
            return false;
        }
        let entry = self.sides.entry(id).or_default();
        let before = entry.len();
        for side in sides {
            if !entry.contains(&side) {
                entry.push(side);
            }
        }
        entry.len() > before
    }

    /// Gives a decision whose loops did not fit on the sides left its first sides back; whether
    /// it had sides to stay off.
    pub(crate) fn settle(&mut self, id: &str) -> bool {
        if self.sides.remove(id).is_some_and(|sides| !sides.is_empty()) {
            self.settled.insert(id.to_owned());
            return true;
        }
        false
    }
}

/// The loops taken out of the layout data, and what puts them back.
#[derive(Debug, Clone, Default)]
pub(crate) struct Loops {
    hosts: Vec<Host>,
    /// The loops taken out, at their positions in the layout data.
    edges: Vec<(usize, Edge)>,
}

/// The box of an edge's label as the default rendering draws it, its chip, when it has a label.
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

/// Takes the loops that can stand beside their node out of the layout data and keeps their room
/// beside their nodes. `None` where no loop can: then the data is unchanged. The nodes of `turned` take
/// the sides of the other direction: the edges of a node in a group with a direction of its own
/// may meet it in the direction of the group's parent. The nodes of `kept_hosts`
/// keep their loops where ELK puts them; the nodes of `reserved` keep their sides taken. A
/// decision stays off the sides `avoided` names, and takes the side that faces the next layer when
/// it stays off one.
pub(crate) fn take_out(
    data: &mut LayoutData,
    kept_hosts: &HashSet<String>,
    turned: &HashSet<String>,
    avoided: &Avoided,
    reserved: &[Reserved],
    sizes: &HashMap<String, (f64, f64)>,
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

    // An ER relation's cardinality markers stand on the straight parts of its loop.
    let reach = if data.diagram_type == super::markers::ER {
        crate::er::MARKER_ROOM
    } else {
        LOOP_REACH
    };
    let mut hosts: Vec<Host> = Vec::new();
    let mut unusable: HashSet<&str> = HashSet::new();
    for edge in data.edges.iter().filter(|edge| edge.start == edge.end) {
        let Some(node) = by_id.get(edge.start.as_str()).copied() else {
            continue;
        };
        // A group takes the size of its frame from a layout before.
        let size = if node.is_group {
            sizes.get(&node.id).copied()
        } else {
            node.width.zip(node.height)
        };
        let Some((width, height)) = size.filter(|(width, height)| *width > 0.0 && *height > 0.0)
        else {
            unusable.insert(node.id.as_str());
            continue;
        };
        if edge.thickness == "invisible" || kept_hosts.contains(&node.id) {
            unusable.insert(node.id.as_str());
            continue;
        }
        let host = match hosts.iter().position(|host| host.id == node.id) {
            Some(position) => &mut hosts[position],
            None => {
                // A group meets its edges in the direction of its parent.
                let at = if node.is_group {
                    node.parent_id
                        .as_deref()
                        .and_then(|parent| index.get(parent).copied())
                } else {
                    Some(index[node.id.as_str()])
                };
                let direction = at
                    .and_then(|at| crate::unified::pipeline::node_dir(data, at, &by_id))
                    .unwrap_or(data.direction.as_str());
                hosts.push(Host {
                    id: node.id.clone(),
                    width,
                    height,
                    across: matches!(direction, "LR" | "RL") == turned.contains(&node.id),
                    flow: Side::facing_flow(direction),
                    loops: Vec::new(),
                });
                hosts.last_mut()?
            }
        };
        host.loops.push(Loop {
            reach,
            ..Loop::new(edge)
        });
    }
    hosts.retain(|host| !unusable.contains(host.id.as_str()));
    let decisions: HashSet<String> = hosts
        .iter()
        .filter(|host| {
            by_id
                .get(host.id.as_str())
                .is_some_and(|node| shapes::is_decision(node) || shapes::is_choice(node))
        })
        .map(|host| host.id.clone())
        .collect();
    hosts.retain_mut(|host| {
        let taken = reserved.iter().find(|entry| entry.id == host.id);
        let is_turned = turned.contains(&host.id);
        // A decision whose side tips its branches need takes the side that faces the next layer.
        let avoid = avoided.of(&host.id);
        let mut order = Side::of(host.across).to_vec();
        if !avoid.is_empty() && !is_turned {
            order.push(host.flow);
        }
        let sides: Vec<Side> = order
            .into_iter()
            .filter(|side| {
                !avoid.contains(side)
                    && !taken.is_some_and(|entry| {
                        if side.is_before() {
                            entry.before
                        } else {
                            entry.after
                        }
                    })
            })
            .collect();
        // Where notes take both free sides, the loop stands on the side its edges enter, at its
        // end; not on a decision, whose tip that is.
        let entry = (!is_turned && !decisions.contains(&host.id)).then(|| opposite(host.flow));
        let flow_along_y = matches!(host.flow, Side::Top | Side::Bottom);
        let centered =
            |side: Side| is_turned || matches!(side, Side::Right | Side::Left) == flow_along_y;
        assign(host, &sides, entry, centered)
            && host.loops.iter().all(|entry| entry.pitch >= MIN_PITCH)
    });
    if hosts.is_empty() {
        return None;
    }

    for host in &hosts {
        let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) else {
            continue;
        };
        node.beside.extend(host.loops.iter().map(|entry| {
            let room = entry.room_box(host.width, host.height);
            (room.x, room.y, room.width, room.height)
        }));
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
    /// Puts the loops back into ELK's result and into the layout data: the loops of each node
    /// stand beside it with their labels. Where ELK's result lacks a node, the result is that
    /// node's id.
    pub(crate) fn put_back(
        &self,
        graph: &mut JsonNode,
        data: &mut LayoutData,
        state: &ElkState,
    ) -> Result<(), String> {
        let mut boxes: HashMap<String, (BBox, String)> = HashMap::new();
        absolute_boxes(graph, Point::default(), &mut boxes);

        // The node's box in the layout.
        let mut own: HashMap<&str, BBox> = HashMap::new();
        for host in &self.hosts {
            let Some((laid, _)) = boxes.get(&host.id) else {
                return Err(host.id.clone());
            };
            own.insert(host.id.as_str(), *laid);
        }

        for (position, edge) in &self.edges {
            data.edges
                .insert((*position).min(data.edges.len()), edge.clone());
        }
        for host in &self.hosts {
            if let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) {
                node.beside.clear();
            }
            let real = own[host.id.as_str()];
            if let Some(node) = find_mut(graph, &host.id) {
                clear_beside(node);
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
                    Some(chip) => entry.label_centre(real, chip),
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

    /// A decision with a loop at a side tip that one of its branches would leave from
    /// (`flowchart::turns`), in the applied layout, with the side tips its branches need: its
    /// loops are to stay off them. `None` where no loop stands in a branch's way.
    pub(crate) fn wanted(&self, data: &LayoutData) -> Option<(String, Vec<Side>)> {
        for host in &self.hosts {
            let Some(node) = data.nodes.iter().find(|node| node.id == host.id) else {
                continue;
            };
            if !(shapes::is_decision(node) || shapes::is_choice(node)) {
                continue;
            }
            let (before, after) = crate::flowchart::turns::wanted_tips(data, &host.id);
            let wanted = |side: Side| {
                side.is_tip(host.flow) && if side.is_before() { before } else { after }
            };
            if host.loops.iter().any(|entry| wanted(entry.side)) {
                let tips = Side::of(!matches!(host.flow, Side::Left | Side::Right));
                return Some((
                    host.id.clone(),
                    tips.into_iter().filter(|side| wanted(*side)).collect(),
                ));
            }
        }
        None
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
            side,
            chip,
            ..Loop::new(&edge("e", "a", "a"))
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
        assert_eq!(
            right.label_centre(real, (40.0, 24.0)),
            Point::new(176.0, 22.0)
        );
        let left = looped(Side::Left, Some((40.0, 24.0)));
        assert_eq!(
            left.label_centre(real, (40.0, 24.0)),
            Point::new(-56.0, 22.0)
        );
        let top = looped(Side::Top, Some((40.0, 24.0)));
        assert_eq!(
            top.label_centre(real, (40.0, 24.0)),
            Point::new(94.0, -16.0)
        );
    }

    #[test]
    fn a_node_keeps_the_room_of_its_loop_on_the_loops_side_only() {
        // 32 for the loop, 4 of gap, the chip of 40 and its margin of 6; along the side the ends
        // 20 apart with 4 of room each, or the chip's height.
        let right = looped(Side::Right, Some((40.0, 24.0)));
        assert_eq!(
            right.room_box(120.0, 44.0),
            BBox::new(120.0, 8.0, 82.0, 28.0)
        );
        let tall = looped(Side::Right, Some((40.0, 60.0)));
        assert_eq!(
            tall.room_box(120.0, 44.0),
            BBox::new(120.0, -8.0, 82.0, 60.0)
        );
        let left = looped(Side::Left, None);
        assert_eq!(
            left.room_box(120.0, 44.0),
            BBox::new(-32.0, 8.0, 32.0, 28.0)
        );
        let top = looped(Side::Top, None);
        assert_eq!(
            top.room_box(120.0, 44.0),
            BBox::new(46.0, -32.0, 28.0, 32.0)
        );
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
    fn the_loops_leave_the_layout_and_their_node_keeps_their_room() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0), node("b", 120.0, 44.0)],
            edges: vec![edge("l", "a", "a"), edge("e", "a", "b")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let loops = take_out(
            &mut data,
            &HashSet::new(),
            &HashSet::new(),
            &Avoided::default(),
            &[],
            &HashMap::new(),
        )
        .expect("a loop beside its node");
        assert_eq!(data.edges.len(), 1);
        assert_eq!(data.edges[0].id, "e");
        assert_eq!(data.nodes[0].width, Some(120.0));
        assert_eq!(data.nodes[0].beside, [(120.0, 8.0, 32.0, 28.0)]);
        assert_eq!(loops.edges.len(), 1);
        assert_eq!(loops.hosts[0].loops[0].side, Side::Right);
    }

    #[test]
    fn a_second_loop_takes_the_other_free_side_and_more_share_the_sides() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: vec![edge("l1", "a", "a"), edge("l2", "a", "a")],
            direction: "LR".to_owned(),
            ..LayoutData::default()
        };
        let none = (HashSet::new(), Avoided::default(), HashMap::new());
        let loops = take_out(&mut data, &none.0, &none.0, &none.1, &[], &none.2)
            .expect("two loops beside their node");
        let sides: Vec<Side> = loops.hosts[0].loops.iter().map(|l| l.side).collect();
        assert_eq!(sides, vec![Side::Top, Side::Bottom]);

        // A third loop shares the first side: one after the other along it, a quarter of the side
        // from its middle, their ends a quarter of the side apart.
        let mut three = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: (1..=3)
                .map(|index| edge(&format!("l{index}"), "a", "a"))
                .collect(),
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let loops = take_out(&mut three, &none.0, &none.0, &none.1, &[], &none.2)
            .expect("three loops beside their node");
        let placed: Vec<(Side, f64, f64)> = loops.hosts[0]
            .loops
            .iter()
            .map(|entry| (entry.side, entry.offset, entry.pitch))
            .collect();
        assert_eq!(
            placed,
            [
                (Side::Right, -11.0, 11.0),
                (Side::Right, 11.0, 11.0),
                (Side::Left, 0.0, 20.0)
            ]
        );

        // More than two on each free side are left to ELK.
        let mut five = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: (1..=5)
                .map(|index| edge(&format!("l{index}"), "a", "a"))
                .collect(),
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        assert!(take_out(&mut five, &none.0, &none.0, &none.1, &[], &none.2).is_none());
        assert_eq!(five.edges.len(), 5);
    }

    #[test]
    fn where_notes_take_both_sides_a_loop_stands_at_the_end_of_the_entry_side() {
        let mut data = LayoutData {
            nodes: vec![node("a", 120.0, 44.0)],
            edges: vec![edge("l", "a", "a")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let reserved = [Reserved {
            id: "a".to_owned(),
            before: true,
            after: true,
        }];
        let none = (HashSet::new(), Avoided::default(), HashMap::new());
        let loops = take_out(&mut data, &none.0, &none.0, &none.1, &reserved, &none.2)
            .expect("a loop on the top");
        let entry = &loops.hosts[0].loops[0];
        // On the top, its end 4 from the right corner, its label past it.
        assert_eq!(
            (entry.side, entry.offset, entry.label),
            (Side::Top, 46.0, LabelAt::Past)
        );
        let real = BBox::new(0.0, 0.0, 120.0, 44.0);
        assert_eq!(
            entry.route(real),
            [
                Point::new(96.0, 0.0),
                Point::new(96.0, -32.0),
                Point::new(116.0, -32.0),
                Point::new(116.0, 0.0),
            ]
        );
    }

    #[test]
    fn a_loop_takes_the_side_a_note_leaves_free() {
        // The note's step keeps the room of a note of 68 on the right.
        let mut a = node("a", 120.0, 44.0);
        a.beside.push((152.0, 0.0, 68.0, 44.0));
        let mut data = LayoutData {
            nodes: vec![a],
            edges: vec![edge("l", "a", "a")],
            direction: "TB".to_owned(),
            ..LayoutData::default()
        };
        let reserved = [Reserved {
            id: "a".to_owned(),
            before: false,
            after: true,
        }];
        let loops = take_out(
            &mut data,
            &HashSet::new(),
            &HashSet::new(),
            &Avoided::default(),
            &reserved,
            &HashMap::new(),
        )
        .expect("a loop");
        assert_eq!(loops.hosts[0].loops[0].side, Side::Left);
        assert_eq!(loops.hosts[0].width, 120.0);
        // Both rooms stand beside the state.
        assert_eq!(
            data.nodes[0].beside,
            [(152.0, 0.0, 68.0, 44.0), (-32.0, 8.0, 32.0, 28.0)]
        );
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
        let loops = take_out(
            &mut data,
            &HashSet::new(),
            &turned,
            &Avoided::default(),
            &[],
            &HashMap::new(),
        )
        .expect("a loop");
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
        assert!(
            take_out(
                &mut data,
                &kept,
                &HashSet::new(),
                &Avoided::default(),
                &[],
                &HashMap::new()
            )
            .is_none()
        );
        assert_eq!(data.edges.len(), 2);
    }
}
