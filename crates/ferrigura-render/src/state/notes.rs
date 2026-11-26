//! The place of a note in the design mode: beside its state, at the state's height
//! (`docs/decisions/0023-design-mode.md`). The step is ferrigura's own, run around ELK's layout;
//! the ELK port does not know it.
//!
//! The reference lays a note out as a node of its own, in a group of its own, joined to its state
//! by an edge. ELK gives it a layer of its own, so the note stands below or above its state, and
//! the states of the main flow move off their axis to make room for it. In the design mode, the
//! notes do not take part in the layout:
//!
//! 1. **Out.** Before the layout, the notes, their groups and their connectors leave the layout
//!    data. A state with notes takes their room instead: its box grows along its layer by the
//!    note's room on both sides, the gap and the note's size, so that the state stays in the
//!    middle of its box, where ELK and the placement put its transitions. Along the flow, the box
//!    grows to the note's size where the note is larger.
//! 2. **Back.** After ELK's layout, before the result is applied, the state takes its own size in
//!    the middle of its box, the ends of its transitions move along their last part onto its
//!    outline, and each note goes into the room beside the state, at the state's height, with its
//!    group and a straight connector. The group of a note belongs to the state's parent, so a
//!    composite state's frame holds the notes of its states.
//! 3. **Check.** Once applied, a note that touches a node, a transition, a label or a frame, or
//!    that its state's composite does not hold, makes the whole diagram fall back to the
//!    reference's place of its notes.
//!
//! In a layout from top to bottom (or back), `left of` puts the note to the left of its state and
//! `right of` to the right. In a layout from left to right (or back), where the transitions meet
//! the left and the right side, the note keeps its place along the layer, as ELK turns the axes:
//! `left of` above the state and `right of` below it.

use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::{JsonEdge, JsonEdgeSection, JsonNode, JsonPoint};

use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::elk_build::{ElkState, sub_graphs};
use crate::unified::elk_geometry::find_common_ancestor;
use crate::unified::geom::{BBox, Point};

/// The distance between a state and its note, which the connector spans.
pub(crate) const NOTE_GAP: f64 = 32.0;

/// The least distance between a note and anything else it does not belong to.
const CLEARANCE: f64 = 4.0;

/// How far a point may be off a side to count as on it.
const TOLERANCE: f64 = 0.5;

/// The side of its state a note stands on, along the state's layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// Left, or above in a layout from left to right: `left of`.
    Before,
    /// Right, or below in a layout from left to right: `right of`.
    After,
}

/// A note that stands beside its state.
#[derive(Debug, Clone)]
struct Note {
    id: String,
    /// Its group (`<state>----parent`).
    group: String,
    /// Its connector.
    edge: String,
    side: Side,
    width: f64,
    height: f64,
}

/// A state with notes beside it.
#[derive(Debug, Clone)]
struct Host {
    id: String,
    /// The state's own size, which its box in the layout exceeds.
    width: f64,
    height: f64,
    /// Whether its layer runs along x: a layout from top to bottom or back.
    across: bool,
    notes: Vec<Note>,
}

/// The notes taken out of the layout data, and what puts them back.
#[derive(Debug, Clone, Default)]
pub(crate) struct Notes {
    hosts: Vec<Host>,
    /// The nodes taken out, at their positions in the layout data.
    nodes: Vec<(usize, Node)>,
    /// The connectors taken out, at their positions.
    edges: Vec<(usize, Edge)>,
}

/// Whether an edge is a note's connector.
fn is_connector(edge: &Edge) -> bool {
    edge.classes
        .split_whitespace()
        .any(|class| class == "note-edge")
}

/// Takes the notes that can stand beside their state out of the layout data and grows their
/// states by their room. `None` where no note can: then the data is unchanged. The states of
/// `kept_hosts` keep their notes where the reference puts them.
///
/// A note stays where the reference puts it when its state is a composite or has no size, when
/// its state has a transition to itself, which ELK would route through the note's room, when
/// another note of the state stands on the same side, or when something else than its connector
/// meets it or its group.
pub(crate) fn take_out(data: &mut LayoutData, kept_hosts: &HashSet<String>) -> Option<Notes> {
    let groups: HashSet<&str> = data
        .nodes
        .iter()
        .filter(|node| node.is_group && node.shape == "noteGroup")
        .map(|node| node.id.as_str())
        .collect();
    if groups.is_empty() {
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
    // The groups with a note that cannot move, which stay with all their notes.
    let mut kept: HashSet<&str> = HashSet::new();
    for note in data
        .nodes
        .iter()
        .filter(|node| !node.is_group && node.shape == "note")
    {
        let Some(group) = note
            .parent_id
            .as_deref()
            .filter(|parent| groups.contains(parent))
        else {
            continue;
        };
        let mut connectors = data
            .edges
            .iter()
            .filter(|edge| edge.start == note.id || edge.end == note.id);
        let (Some(edge), None) = (connectors.next(), connectors.next()) else {
            kept.insert(group);
            continue;
        };
        let host_id = if edge.start == note.id {
            &edge.end
        } else {
            &edge.start
        };
        let host = by_id.get(host_id.as_str()).copied();
        let usable = host.filter(|host| {
            is_connector(edge)
                && !kept_hosts.contains(&host.id)
                && !host.is_group
                && host.width.is_some()
                && host.height.is_some()
                && !host
                    .parent_id
                    .as_deref()
                    .is_some_and(|parent| groups.contains(parent))
        });
        let (Some(host), Some(width), Some(height)) = (
            usable,
            note.width.filter(|width| *width > 0.0),
            note.height.filter(|height| *height > 0.0),
        ) else {
            kept.insert(group);
            continue;
        };
        let side = if note.position.as_deref() == Some("left of") {
            Side::Before
        } else {
            Side::After
        };
        let entry = match hosts.iter().position(|entry| entry.id == host.id) {
            Some(position) => &mut hosts[position],
            None => {
                let direction =
                    crate::unified::pipeline::node_dir(data, index[host.id.as_str()], &by_id)
                        .unwrap_or(data.direction.as_str());
                hosts.push(Host {
                    id: host.id.clone(),
                    width: host.width.unwrap_or(0.0),
                    height: host.height.unwrap_or(0.0),
                    across: !matches!(direction, "LR" | "RL"),
                    notes: Vec::new(),
                });
                hosts.last_mut()?
            }
        };
        entry.notes.push(Note {
            id: note.id.clone(),
            group: group.to_owned(),
            edge: edge.id.clone(),
            side,
            width,
            height,
        });
    }

    // A state with a transition to itself, with two notes on one side, or narrower than the bundle
    // of its transitions keeps its notes where the reference puts them.
    for host in &hosts {
        let looped = data
            .edges
            .iter()
            .any(|edge| edge.start == host.id && edge.end == host.id);
        let ends = |at_start: bool| {
            data.edges
                .iter()
                .filter(|edge| edge.start != edge.end && !is_connector(edge))
                .filter(|edge| {
                    if at_start {
                        edge.start == host.id
                    } else {
                        edge.end == host.id
                    }
                })
                .count()
        };
        let bundle = (ends(true).max(ends(false)).max(1) - 1) as f64 * crate::design::PORT_PITCH;
        let side = if host.across { host.width } else { host.height };
        let narrow = bundle > side;
        let crowded = [Side::Before, Side::After]
            .iter()
            .any(|&side| host.notes.iter().filter(|note| note.side == side).count() > 1);
        if looped || crowded || narrow {
            kept.extend(host.notes.iter().map(|note| note.group.as_str()));
        }
    }
    let kept: HashSet<String> = kept.into_iter().map(str::to_owned).collect();
    hosts.retain(|host| host.notes.iter().all(|note| !kept.contains(&note.group)));
    // A group holds the notes of one state only.
    hosts.retain(|host| {
        host.notes.iter().all(|note| {
            data.nodes
                .iter()
                .filter(|node| node.parent_id.as_deref() == Some(note.group.as_str()))
                .all(|member| host.notes.iter().any(|other| other.id == member.id))
        })
    });
    if hosts.is_empty() {
        return None;
    }

    let moved_nodes: HashSet<String> = hosts
        .iter()
        .flat_map(|host| host.notes.iter())
        .flat_map(|note| [note.id.clone(), note.group.clone()])
        .collect();
    let moved_edges: HashSet<String> = hosts
        .iter()
        .flat_map(|host| host.notes.iter())
        .map(|note| note.edge.clone())
        .collect();
    // Nothing but a connector meets a note or its group.
    if data.edges.iter().any(|edge| {
        !moved_edges.contains(&edge.id)
            && (moved_nodes.contains(&edge.start) || moved_nodes.contains(&edge.end))
    }) {
        return None;
    }

    for host in &hosts {
        let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) else {
            continue;
        };
        let (width, height) = room(host);
        node.width = Some(width);
        node.height = Some(height);
    }
    let mut notes = Notes {
        hosts,
        ..Notes::default()
    };
    for (position, node) in std::mem::take(&mut data.nodes).into_iter().enumerate() {
        if moved_nodes.contains(&node.id) {
            notes.nodes.push((position, node));
        } else {
            data.nodes.push(node);
        }
    }
    for (position, edge) in std::mem::take(&mut data.edges).into_iter().enumerate() {
        if moved_edges.contains(&edge.id) {
            notes.edges.push((position, edge));
        } else {
            data.edges.push(edge);
        }
    }
    Some(notes)
}

/// The size of a state's box in the layout: its own size, with the room of its notes on both
/// sides along its layer, and at least the size of its notes along the flow.
fn room(host: &Host) -> (f64, f64) {
    let (mut reach, mut depth) = (0.0_f64, 0.0_f64);
    for note in &host.notes {
        let (along, flow) = if host.across {
            (note.width, note.height)
        } else {
            (note.height, note.width)
        };
        reach = reach.max(NOTE_GAP + along);
        depth = depth.max(flow);
    }
    // The box grows by whole units on each side, so that the middle of its side, where ELK and
    // the placement put the ports on whole units, is where it is for the state alone.
    let reach = reach.ceil();
    let grow = |size: f64| size + 2.0 * ((depth - size) / 2.0).max(0.0).ceil();
    if host.across {
        (host.width + 2.0 * reach, grow(host.height))
    } else {
        (grow(host.width), host.height + 2.0 * reach)
    }
}

impl Notes {
    /// Puts the notes back into ELK's result and into the layout data: each state takes its own
    /// size in the middle of its box, the ends of its transitions move onto its outline, and its
    /// notes stand beside it. Where an end of a transition does not leave a state's box straight
    /// from a side, the result is that state's id, with the data unchanged.
    pub(crate) fn put_back(
        &self,
        graph: &mut JsonNode,
        data: &mut LayoutData,
        state: &mut ElkState,
    ) -> Result<(), String> {
        let mut boxes: HashMap<String, (BBox, String)> = HashMap::new();
        absolute_boxes(graph, Point::default(), &mut boxes);
        let origin = |id: Option<&String>| {
            id.and_then(|id| boxes.get(id))
                .map_or_else(Point::default, |(bbox, _)| Point::new(bbox.x, bbox.y))
        };

        // The state's box in the layout and its own box in the middle of it.
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

        // The ends of the transitions, on the outline of their state.
        let mut ends: Vec<(usize, bool, JsonPoint)> = Vec::new();
        for (position, edge) in graph.edges.iter().enumerate() {
            let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
                continue;
            };
            let Some(section) = edge.sections.first() else {
                continue;
            };
            let offset = origin(find_common_ancestor(source, target, &state.parent_by_id).as_ref());
            let absolute = |point: Option<JsonPoint>| {
                point.map(|point| Point::new(point.x + offset.x, point.y + offset.y))
            };
            let start = absolute(section.start_point);
            let end = absolute(section.end_point);
            let first_bend = absolute(section.bend_points.first().copied()).or(end);
            let last_bend = absolute(section.bend_points.last().copied()).or(start);
            for (at_start, id, point, next) in [
                (true, source, start, first_bend),
                (false, target, end, last_bend),
            ] {
                let Some(&(laid, real)) = own.get(id.as_str()) else {
                    continue;
                };
                let (Some(point), Some(next)) = (point, next) else {
                    return Err(id.clone());
                };
                let Some(moved) = onto_outline(point, next, laid, real) else {
                    return Err(id.clone());
                };
                ends.push((
                    position,
                    at_start,
                    JsonPoint {
                        x: moved.x - offset.x,
                        y: moved.y - offset.y,
                    },
                ));
            }
        }
        for (position, at_start, point) in ends {
            let section = &mut graph.edges[position].sections[0];
            if at_start {
                section.start_point = Some(point);
            } else {
                section.end_point = Some(point);
            }
        }

        // The layout data with the notes, their groups and their connectors, each group in the
        // parent of its state.
        let parents: HashMap<String, Option<String>> = self
            .hosts
            .iter()
            .flat_map(|host| {
                let parent = data
                    .nodes
                    .iter()
                    .find(|node| node.id == host.id)
                    .and_then(|node| node.parent_id.clone());
                host.notes
                    .iter()
                    .map(move |note| (note.group.clone(), parent.clone()))
            })
            .collect();
        for (position, node) in &self.nodes {
            let mut node = node.clone();
            if let Some(parent) = parents.get(&node.id) {
                node.parent_id.clone_from(parent);
            }
            data.nodes.insert((*position).min(data.nodes.len()), node);
        }
        for (position, edge) in &self.edges {
            data.edges
                .insert((*position).min(data.edges.len()), edge.clone());
        }
        for host in &self.hosts {
            if let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) {
                node.width = Some(host.width);
                node.height = Some(host.height);
            }
        }
        let (parent_by_id, children_by_id) = sub_graphs(data);
        state.parent_by_id = parent_by_id;
        state.children_by_id = children_by_id;

        for host in &self.hosts {
            let (_, real) = own[host.id.as_str()];
            let parent = boxes
                .get(&host.id)
                .map_or_else(String::new, |(_, parent)| parent.clone());
            if let Some(node) = find_mut(graph, &host.id) {
                node.x =
                    Some(node.x.unwrap_or(0.0) + (node.width.unwrap_or(0.0) - real.width) / 2.0);
                node.y =
                    Some(node.y.unwrap_or(0.0) + (node.height.unwrap_or(0.0) - real.height) / 2.0);
                node.width = Some(real.width);
                node.height = Some(real.height);
            }
            let parent_origin = if parent == graph.id {
                Point::default()
            } else {
                origin(Some(&parent))
            };
            let mut groups: Vec<(String, Vec<(String, BBox)>)> = Vec::new();
            for note in &host.notes {
                let placed = beside(host, note, real);
                match groups.iter_mut().find(|(group, _)| *group == note.group) {
                    Some((_, members)) => members.push((note.id.clone(), placed)),
                    None => groups.push((note.group.clone(), vec![(note.id.clone(), placed)])),
                }
                let Some((_, connector)) = self.edges.iter().find(|(_, edge)| edge.id == note.edge)
                else {
                    continue;
                };
                let (from, to) = connector_ends(host, note, real, placed);
                let (from, to) = if connector.start == host.id {
                    (from, to)
                } else {
                    (to, from)
                };
                let offset = origin(
                    find_common_ancestor(&connector.start, &connector.end, &state.parent_by_id)
                        .as_ref(),
                );
                let relative = |point: Point| JsonPoint {
                    x: point.x - offset.x,
                    y: point.y - offset.y,
                };
                graph.edges.push(JsonEdge {
                    id: connector.id.clone(),
                    sources: vec![connector.start.clone()],
                    targets: vec![connector.end.clone()],
                    sections: vec![JsonEdgeSection {
                        id: format!("{}_s0", connector.id),
                        start_point: Some(relative(from)),
                        end_point: Some(relative(to)),
                        ..JsonEdgeSection::default()
                    }],
                    ..JsonEdge::default()
                });
            }
            for (group, members) in groups {
                let frame = members
                    .iter()
                    .map(|(_, placed)| *placed)
                    .reduce(BBox::union)
                    .unwrap_or_default();
                let children = members
                    .into_iter()
                    .map(|(id, placed)| JsonNode {
                        id,
                        x: Some(placed.x - frame.x),
                        y: Some(placed.y - frame.y),
                        width: Some(placed.width),
                        height: Some(placed.height),
                        ..JsonNode::default()
                    })
                    .collect();
                let node = JsonNode {
                    id: group,
                    x: Some(frame.x - parent_origin.x),
                    y: Some(frame.y - parent_origin.y),
                    width: Some(frame.width),
                    height: Some(frame.height),
                    children,
                    ..JsonNode::default()
                };
                if parent == graph.id {
                    graph.children.push(node);
                } else if let Some(container) = find_mut(graph, &parent) {
                    container.children.push(node);
                }
            }
        }
        Ok(())
    }

    /// The first state with a note that does not stand clear of everything it does not belong to,
    /// or not inside the frames of its state's composites, in the applied layout; `None` where
    /// every note does.
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
            // The composites that hold the state.
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
            for note in &host.notes {
                let Some(node) = by_id.get(note.id.as_str()) else {
                    return Some(host.id.clone());
                };
                let paper = frame(node);
                let room = grown(paper, CLEARANCE);
                for other in &data.nodes {
                    if other.id == note.id || other.id == note.group {
                        continue;
                    }
                    let bbox = frame(other);
                    // A note's group draws nothing; its notes count as nodes.
                    let fine = if !other.is_group {
                        !overlaps(bbox, room)
                    } else if other.shape == "noteGroup" {
                        true
                    } else if holders.contains(other.id.as_str()) {
                        contains(bbox, room)
                    } else {
                        !overlaps(bbox, room)
                    };
                    if !fine {
                        return Some(host.id.clone());
                    }
                }
                for edge in data.edges.iter().filter(|edge| edge.id != note.edge) {
                    if edge
                        .points
                        .windows(2)
                        .any(|pair| crosses(pair[0], pair[1], room))
                    {
                        return Some(host.id.clone());
                    }
                    if let (false, Some(x), Some(y), Some(width), Some(height)) = (
                        edge.label.is_empty(),
                        edge.x,
                        edge.y,
                        edge.width,
                        edge.height,
                    ) && overlaps(
                        BBox::new(x - width / 2.0, y - height / 2.0, width, height),
                        room,
                    ) {
                        return Some(host.id.clone());
                    }
                }
            }
        }
        None
    }
}

/// Where a note stands beside its state: `NOTE_GAP` from its side, centered on the state across.
fn beside(host: &Host, note: &Note, real: BBox) -> BBox {
    let middle = real.center();
    let (x, y) = match (host.across, note.side) {
        (true, Side::After) => (real.right() + NOTE_GAP, middle.y - note.height / 2.0),
        (true, Side::Before) => (real.x - NOTE_GAP - note.width, middle.y - note.height / 2.0),
        (false, Side::After) => (middle.x - note.width / 2.0, real.bottom() + NOTE_GAP),
        (false, Side::Before) => (middle.x - note.width / 2.0, real.y - NOTE_GAP - note.height),
    };
    BBox::new(x, y, note.width, note.height)
}

/// The ends of a note's connector: on the state's side and on the note's, through their middles.
fn connector_ends(host: &Host, note: &Note, real: BBox, placed: BBox) -> (Point, Point) {
    let middle = real.center();
    match (host.across, note.side) {
        (true, Side::After) => (
            Point::new(real.right(), middle.y),
            Point::new(placed.x, middle.y),
        ),
        (true, Side::Before) => (
            Point::new(real.x, middle.y),
            Point::new(placed.right(), middle.y),
        ),
        (false, Side::After) => (
            Point::new(middle.x, real.bottom()),
            Point::new(middle.x, placed.y),
        ),
        (false, Side::Before) => (
            Point::new(middle.x, real.y),
            Point::new(middle.x, placed.bottom()),
        ),
    }
}

/// Moves the end of a transition from the side of a state's box in the layout onto the state's
/// own outline, along the transition's last part, which must leave the side straight. `None`
/// where the end is not on a side of the box, leaves it at a slant, or misses the state.
fn onto_outline(point: Point, next: Point, laid: BBox, real: BBox) -> Option<Point> {
    let near = |a: f64, b: f64| (a - b).abs() <= TOLERANCE;
    let within =
        |value: f64, from: f64, to: f64| value >= from - TOLERANCE && value <= to + TOLERANCE;
    if near(point.y, laid.y) || near(point.y, laid.bottom()) {
        if !near(point.x, next.x) || !within(point.x, real.x, real.right()) {
            return None;
        }
        let y = if near(point.y, laid.y) {
            real.y
        } else {
            real.bottom()
        };
        return Some(Point::new(point.x, y));
    }
    if near(point.x, laid.x) || near(point.x, laid.right()) {
        if !near(point.y, next.y) || !within(point.y, real.y, real.bottom()) {
            return None;
        }
        let x = if near(point.x, laid.x) {
            real.x
        } else {
            real.right()
        };
        return Some(Point::new(x, point.y));
    }
    None
}

/// The absolute box of every node of ELK's result, with the id of its parent.
fn absolute_boxes(node: &JsonNode, origin: Point, boxes: &mut HashMap<String, (BBox, String)>) {
    for child in &node.children {
        let at = Point::new(
            origin.x + child.x.unwrap_or(0.0),
            origin.y + child.y.unwrap_or(0.0),
        );
        boxes.insert(
            child.id.clone(),
            (
                BBox::new(
                    at.x,
                    at.y,
                    child.width.unwrap_or(0.0),
                    child.height.unwrap_or(0.0),
                ),
                node.id.clone(),
            ),
        );
        absolute_boxes(child, at, boxes);
    }
}

/// The node of ELK's result with the id, at any depth below `node`.
fn find_mut<'a>(node: &'a mut JsonNode, id: &str) -> Option<&'a mut JsonNode> {
    for child in &mut node.children {
        if child.id == id {
            return Some(child);
        }
        if let Some(found) = find_mut(child, id) {
            return Some(found);
        }
    }
    None
}

/// The box grown by `by` on every side.
fn grown(bbox: BBox, by: f64) -> BBox {
    BBox::new(
        bbox.x - by,
        bbox.y - by,
        bbox.width + 2.0 * by,
        bbox.height + 2.0 * by,
    )
}

/// Whether two boxes share more than an edge.
fn overlaps(a: BBox, b: BBox) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Whether `outer` holds all of `inner`.
fn contains(outer: BBox, inner: BBox) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}

/// Whether the segment from `a` to `b` passes through the inside of the box.
fn crosses(a: Point, b: Point, bbox: BBox) -> bool {
    // Liang-Barsky clipping of the segment against the box.
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.x - bbox.x),
        (dx, bbox.right() - a.x),
        (-dy, a.y - bbox.y),
        (dy, bbox.bottom() - a.y),
    ] {
        if p == 0.0 {
            if q <= 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            low = low.max(t);
        } else {
            high = high.min(t);
        }
        if low >= high {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(across: bool, sides: &[Side]) -> Host {
        Host {
            id: "S".to_owned(),
            width: 100.0,
            height: 40.0,
            across,
            notes: sides
                .iter()
                .enumerate()
                .map(|(index, &side)| Note {
                    id: format!("S----note-{index}"),
                    group: "S----parent".to_owned(),
                    edge: format!("e{index}"),
                    side,
                    width: 150.0,
                    height: 60.0,
                })
                .collect(),
        }
    }

    #[test]
    fn a_state_takes_the_room_of_its_note_on_both_sides_of_its_layer() {
        // From top to bottom: the layer runs along x, the note is taller than the state.
        assert_eq!(
            room(&host(true, &[Side::After])),
            (100.0 + 2.0 * 182.0, 60.0)
        );
        // From left to right: the layer runs along y, the note is wider than the state.
        assert_eq!(
            room(&host(false, &[Side::Before])),
            (150.0, 40.0 + 2.0 * 92.0)
        );
        // Two notes take the room of the larger one.
        assert_eq!(
            room(&host(true, &[Side::Before, Side::After])),
            (100.0 + 2.0 * 182.0, 60.0)
        );
    }

    #[test]
    fn a_note_stands_beside_its_state_at_its_height() {
        let real = BBox::new(0.0, 0.0, 100.0, 40.0);
        let right = host(true, &[Side::After]);
        let placed = beside(&right, &right.notes[0], real);
        assert_eq!(placed, BBox::new(132.0, -10.0, 150.0, 60.0));
        assert_eq!(
            connector_ends(&right, &right.notes[0], real, placed),
            (Point::new(100.0, 20.0), Point::new(132.0, 20.0))
        );
        let left = host(true, &[Side::Before]);
        assert_eq!(
            beside(&left, &left.notes[0], real),
            BBox::new(-182.0, -10.0, 150.0, 60.0)
        );
        // From left to right, `right of` is below and `left of` above.
        let below = host(false, &[Side::After]);
        assert_eq!(
            beside(&below, &below.notes[0], real),
            BBox::new(-25.0, 72.0, 150.0, 60.0)
        );
        let above = host(false, &[Side::Before]);
        assert_eq!(
            beside(&above, &above.notes[0], real),
            BBox::new(-25.0, -92.0, 150.0, 60.0)
        );
    }

    #[test]
    fn the_end_of_a_transition_moves_along_its_last_part_onto_the_state() {
        let laid = BBox::new(0.0, 0.0, 300.0, 60.0);
        let real = BBox::new(100.0, 10.0, 100.0, 40.0);
        // Straight down out of the bottom of the box.
        assert_eq!(
            onto_outline(
                Point::new(150.0, 60.0),
                Point::new(150.0, 100.0),
                laid,
                real
            ),
            Some(Point::new(150.0, 50.0))
        );
        // Straight up into the top.
        assert_eq!(
            onto_outline(Point::new(140.0, 0.0), Point::new(140.0, -30.0), laid, real),
            Some(Point::new(140.0, 10.0))
        );
        // At a slant, or beside the state, it cannot.
        assert_eq!(
            onto_outline(
                Point::new(150.0, 60.0),
                Point::new(170.0, 100.0),
                laid,
                real
            ),
            None
        );
        assert_eq!(
            onto_outline(Point::new(50.0, 60.0), Point::new(50.0, 100.0), laid, real),
            None
        );
        // Not on a side of the box.
        assert_eq!(
            onto_outline(
                Point::new(150.0, 30.0),
                Point::new(150.0, 100.0),
                laid,
                real
            ),
            None
        );
    }

    #[test]
    fn a_segment_crosses_a_box_only_through_its_inside() {
        let bbox = BBox::new(0.0, 0.0, 10.0, 10.0);
        assert!(crosses(Point::new(-5.0, 5.0), Point::new(15.0, 5.0), bbox));
        assert!(crosses(Point::new(5.0, 5.0), Point::new(5.0, 6.0), bbox));
        assert!(!crosses(
            Point::new(-5.0, 12.0),
            Point::new(15.0, 12.0),
            bbox
        ));
        assert!(!crosses(
            Point::new(10.0, -5.0),
            Point::new(10.0, 15.0),
            bbox
        ));
        assert!(!crosses(
            Point::new(-5.0, -5.0),
            Point::new(-1.0, 15.0),
            bbox
        ));
    }

    fn node(id: &str, shape: &str, parent: Option<&str>, group: bool) -> Node {
        Node {
            id: id.to_owned(),
            shape: shape.to_owned(),
            parent_id: parent.map(str::to_owned),
            is_group: group,
            width: Some(100.0),
            height: Some(40.0),
            position: (shape == "note").then(|| "right of".to_owned()),
            ..Node::default()
        }
    }

    fn edge(id: &str, start: &str, end: &str, classes: &str) -> Edge {
        Edge {
            id: id.to_owned(),
            start: start.to_owned(),
            end: end.to_owned(),
            classes: classes.to_owned(),
            ..Edge::default()
        }
    }

    fn noted() -> LayoutData {
        LayoutData {
            direction: "TB".to_owned(),
            nodes: vec![
                node("A", "rect", None, false),
                node("A----parent", "noteGroup", None, true),
                node("A----note-1", "note", Some("A----parent"), false),
                node("B", "rect", None, false),
            ],
            edges: vec![
                edge("e1", "A", "B", "transition"),
                edge("A-A----note-1", "A", "A----note-1", "transition note-edge"),
            ],
            ..LayoutData::default()
        }
    }

    #[test]
    fn the_notes_leave_the_layout_and_their_state_takes_their_room() {
        let mut data = noted();
        let notes =
            take_out(&mut data, &HashSet::new()).expect("the note can stand beside its state");
        let ids: Vec<&str> = data.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(ids, ["A", "B"]);
        assert_eq!(data.edges.len(), 1);
        assert_eq!(data.nodes[0].width, Some(100.0 + 2.0 * (NOTE_GAP + 100.0)));
        assert_eq!(data.nodes[0].height, Some(40.0));
        assert_eq!(notes.nodes.len(), 2);
        assert_eq!(notes.edges.len(), 1);
    }

    #[test]
    fn a_state_that_loops_keeps_its_note_where_the_reference_puts_it() {
        let mut data = noted();
        data.edges.push(edge("e2", "A", "A", "transition"));
        let before = data.nodes.len();
        assert!(take_out(&mut data, &HashSet::new()).is_none());
        assert_eq!(data.nodes.len(), before);
        assert_eq!(data.nodes[0].width, Some(100.0));
    }
}
