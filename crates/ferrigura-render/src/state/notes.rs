//! The place of a note in the default rendering: beside its state, at the state's height
//! (`docs/decisions/0023-the-default-rendering.md`). The step is ferrigura's own, run around ELK's
//! layout; the ELK port does not know it.
//!
//! The reference lays a note out as a node of its own, in a group of its own, joined to its state
//! by an edge. ELK gives it a layer of its own, so the note stands below or above its state, and
//! the states of the main flow move off their axis to make room for it. In the default rendering,
//! the notes do not take part in the layout:
//!
//! 1. **Out.** Before the layout, the notes, their groups and their connectors leave the layout
//!    data. A state with notes keeps their room instead, on the note's side only: each note's box
//!    goes to ELK as a label of the state at the note's place, which the state's margins hold
//!    (`Node::beside`). The state keeps its own size, and ELK and the placement put its
//!    transitions on its own sides.
//! 2. **Back.** After ELK's layout, before the result is applied, each note goes into the room
//!    beside its state, at the state's height, with its group and a straight connector. The group
//!    of a note belongs to the state's parent, so a composite state's frame holds the notes of its
//!    states.
//! 3. **Check.** Once applied, a note that touches a node, a transition, a label or a frame, or
//!    that its state's composite does not hold, makes the whole diagram fall back to the
//!    reference's place of its notes.
//!
//! In a layout from top to bottom (or back), `left of` puts the note to the left of its state and
//! `right of` to the right. In a layout from left to right (or back), where the transitions meet
//! the left and the right side, the note keeps its place along the layer, as ELK turns the axes:
//! `left of` above the state and `right of` below it. The notes of one side stand in a column
//! along it, centered together on the state. A composite state is a state here too: its frame's
//! size, which the room of its notes needs before the layout, comes from a layout before
//! (`state::composite_sizes`).

use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::{JsonEdge, JsonEdgeSection, JsonNode, JsonPoint};

use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::elk_build::{ElkState, sub_graphs};
use crate::unified::elk_geometry::find_common_ancestor;
use crate::unified::geom::{BBox, Point};
use crate::unified::room::{
    absolute_boxes, clear_beside, contains, crosses, find_mut, grown, overlaps,
};

/// The distance between a state and its note, which the connector spans.
pub(crate) const NOTE_GAP: f64 = 32.0;

/// The least distance between a note and anything else it does not belong to.
const CLEARANCE: f64 = 4.0;

/// The distance between two notes on one side of a state.
const NOTE_STACK_GAP: f64 = 8.0;

/// The least length over which a note on a side shared with another one stands beside its state,
/// where its connector meets both.
const MIN_OVERLAP: f64 = 8.0;

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
    /// The state's own size: for a composite, the size of its frame in a layout before.
    width: f64,
    height: f64,
    /// Whether the room of its notes is kept: not for a composite whose size is not known yet.
    sized: bool,
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

/// Whether a note stands beside a composite state, whose size its room needs from a layout before.
pub(crate) fn on_composite(data: &LayoutData) -> bool {
    data.edges
        .iter()
        .filter(|edge| is_connector(edge))
        .any(|edge| {
            data.nodes.iter().any(|node| {
                node.is_group
                    && node.shape != "noteGroup"
                    && (node.id == edge.start || node.id == edge.end)
            })
        })
}

/// Takes the notes that can stand beside their state out of the layout data and keeps their room
/// beside their states. `None` where no note can: then the data is unchanged. The states of
/// `kept_hosts` keep their notes where the reference puts them.
///
/// A composite takes the size of its frame from `sizes`, which a layout before gives
/// (`state::composite_sizes`); without `sizes`, the notes of composites leave the layout data without
/// room, for that layout. A note stays where the reference puts it when its state has no size,
/// when its state has an invisible transition to itself, when it would not stand beside its state
/// in a column with the other notes of its side, or when something else than its connector meets
/// it or its group.
pub(crate) fn take_out(
    data: &mut LayoutData,
    kept_hosts: &HashSet<String>,
    sizes: Option<&HashMap<String, (f64, f64)>>,
) -> Option<Notes> {
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
        // The size of the state, or of a composite's frame in a layout before.
        let size = |host: &Node| {
            if !host.is_group {
                return host.width.zip(host.height).map(|size| (size, true));
            }
            match sizes {
                Some(sizes) => sizes.get(&host.id).map(|&size| (size, true)),
                None => Some(((0.0, 0.0), false)),
            }
        };
        let usable = host.filter(|host| {
            is_connector(edge)
                && !kept_hosts.contains(&host.id)
                && size(host).is_some()
                && host.shape != "noteGroup"
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
                // A composite meets its transitions in the direction of its parent.
                let at = if host.is_group {
                    host.parent_id
                        .as_deref()
                        .and_then(|parent| index.get(parent).copied())
                } else {
                    Some(index[host.id.as_str()])
                };
                let direction = at
                    .and_then(|at| crate::unified::pipeline::node_dir(data, at, &by_id))
                    .unwrap_or(data.direction.as_str());
                let ((width, height), sized) = size(host)?;
                hosts.push(Host {
                    id: host.id.clone(),
                    width,
                    height,
                    sized,
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

    // A state with an invisible transition to itself, or with a note that would not stand beside
    // it in the column of its side, keeps its notes where the reference puts them.
    for host in &hosts {
        let looped = data.edges.iter().any(|edge| {
            edge.start == host.id && edge.end == host.id && edge.thickness == "invisible"
        });
        let own = BBox::new(0.0, 0.0, host.width, host.height);
        let crowded = host.sized
            && placed(host, own).iter().any(|bbox| {
                overlap(host, own, *bbox).1 - overlap(host, own, *bbox).0 < MIN_OVERLAP
            });
        if looped || crowded {
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

    for host in hosts.iter().filter(|host| host.sized) {
        let Some(node) = data.nodes.iter_mut().find(|node| node.id == host.id) else {
            continue;
        };
        let own = BBox::new(0.0, 0.0, host.width, host.height);
        node.beside.extend(
            placed(host, own)
                .into_iter()
                .map(|room| (room.x, room.y, room.width, room.height)),
        );
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

impl Notes {
    /// The states with notes beside them, with the sides their notes take, for the self-loops
    /// that stand beside them on the other side.
    pub(crate) fn reserved(&self) -> Vec<crate::unified::loops::Reserved> {
        self.hosts
            .iter()
            .map(|host| crate::unified::loops::Reserved {
                id: host.id.clone(),
                before: host.notes.iter().any(|note| note.side == Side::Before),
                after: host.notes.iter().any(|note| note.side == Side::After),
            })
            .collect()
    }

    /// Puts the notes back into ELK's result and into the layout data: the notes of each state
    /// stand beside it. Where ELK's result lacks a state, the result is that state's id, with the
    /// data unchanged.
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

        // The state's box in the layout.
        let mut own: HashMap<&str, BBox> = HashMap::new();
        for host in &self.hosts {
            let Some((laid, _)) = boxes.get(&host.id) else {
                return Err(host.id.clone());
            };
            own.insert(host.id.as_str(), *laid);
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
                node.beside.clear();
            }
        }
        let (parent_by_id, children_by_id) = sub_graphs(data);
        state.parent_by_id = parent_by_id;
        state.children_by_id = children_by_id;

        for host in &self.hosts {
            let real = own[host.id.as_str()];
            let parent = boxes
                .get(&host.id)
                .map_or_else(String::new, |(_, parent)| parent.clone());
            if let Some(node) = find_mut(graph, &host.id) {
                clear_beside(node);
            }
            let parent_origin = if parent == graph.id {
                Point::default()
            } else {
                origin(Some(&parent))
            };
            let mut groups: Vec<(String, Vec<(String, BBox)>)> = Vec::new();
            let places = placed(host, real);
            for (note, &placed) in host.notes.iter().zip(&places) {
                match groups.iter_mut().find(|(group, _)| *group == note.group) {
                    Some((_, members)) => members.push((note.id.clone(), placed)),
                    None => groups.push((note.group.clone(), vec![(note.id.clone(), placed)])),
                }
                let Some((_, connector)) = self.edges.iter().find(|(_, edge)| edge.id == note.edge)
                else {
                    continue;
                };
                let (from, to) = connector_ends(host, note.side, real, placed);
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

/// Where the notes of a state stand beside it, in the order of its notes: `NOTE_GAP` from their
/// side, the notes of one side in a column along it, `NOTE_STACK_GAP` apart and centered together
/// on the state.
fn placed(host: &Host, real: BBox) -> Vec<BBox> {
    let middle = real.center();
    let along = |note: &Note| if host.across { note.height } else { note.width };
    let mut boxes = vec![BBox::default(); host.notes.len()];
    for side in [Side::Before, Side::After] {
        let members: Vec<usize> = (0..host.notes.len())
            .filter(|&index| host.notes[index].side == side)
            .collect();
        let total = members
            .iter()
            .map(|&index| along(&host.notes[index]))
            .sum::<f64>()
            + NOTE_STACK_GAP * members.len().saturating_sub(1) as f64;
        let mut at = if host.across { middle.y } else { middle.x } - total / 2.0;
        for index in members {
            let note = &host.notes[index];
            let (x, y) = match (host.across, side) {
                (true, Side::After) => (real.right() + NOTE_GAP, at),
                (true, Side::Before) => (real.x - NOTE_GAP - note.width, at),
                (false, Side::After) => (at, real.bottom() + NOTE_GAP),
                (false, Side::Before) => (at, real.y - NOTE_GAP - note.height),
            };
            boxes[index] = BBox::new(x, y, note.width, note.height);
            at += along(note) + NOTE_STACK_GAP;
        }
    }
    boxes
}

/// The stretch along the state's side over which a note stands beside it: from, to.
fn overlap(host: &Host, real: BBox, placed: BBox) -> (f64, f64) {
    if host.across {
        (real.y.max(placed.y), real.bottom().min(placed.bottom()))
    } else {
        (real.x.max(placed.x), real.right().min(placed.right()))
    }
}

/// The ends of a note's connector: on the state's side and on the note's, straight across, in the
/// middle of the stretch over which the note stands beside the state.
fn connector_ends(host: &Host, side: Side, real: BBox, placed: BBox) -> (Point, Point) {
    let (from, to) = overlap(host, real, placed);
    let at = f64::midpoint(from, to);
    match (host.across, side) {
        (true, Side::After) => (Point::new(real.right(), at), Point::new(placed.x, at)),
        (true, Side::Before) => (Point::new(real.x, at), Point::new(placed.right(), at)),
        (false, Side::After) => (Point::new(at, real.bottom()), Point::new(at, placed.y)),
        (false, Side::Before) => (Point::new(at, real.y), Point::new(at, placed.bottom())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(across: bool, sides: &[Side]) -> Host {
        Host {
            id: "S".to_owned(),
            width: 100.0,
            height: 40.0,
            sized: true,
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
    fn a_note_stands_beside_its_state_at_its_height() {
        let real = BBox::new(0.0, 0.0, 100.0, 40.0);
        let right = host(true, &[Side::After]);
        let placed = placed(&right, real)[0];
        assert_eq!(placed, BBox::new(132.0, -10.0, 150.0, 60.0));
        assert_eq!(
            connector_ends(&right, Side::After, real, placed),
            (Point::new(100.0, 20.0), Point::new(132.0, 20.0))
        );
        let left = host(true, &[Side::Before]);
        assert_eq!(
            super::placed(&left, real),
            [BBox::new(-182.0, -10.0, 150.0, 60.0)]
        );
        // From left to right, `right of` is below and `left of` above.
        let below = host(false, &[Side::After]);
        assert_eq!(
            super::placed(&below, real),
            [BBox::new(-25.0, 72.0, 150.0, 60.0)]
        );
        let above = host(false, &[Side::Before]);
        assert_eq!(
            super::placed(&above, real),
            [BBox::new(-25.0, -92.0, 150.0, 60.0)]
        );
    }

    #[test]
    fn two_notes_on_one_side_stand_in_a_column_with_straight_connectors() {
        let real = BBox::new(0.0, 0.0, 100.0, 40.0);
        let mut two = host(true, &[Side::After, Side::After]);
        two.notes[0].height = 30.0;
        two.notes[1].height = 30.0;
        let places = placed(&two, real);
        // 30, 8 and 30, centered on the state's middle at 20.
        assert_eq!(
            places,
            [
                BBox::new(132.0, -14.0, 150.0, 30.0),
                BBox::new(132.0, 24.0, 150.0, 30.0)
            ]
        );
        // Each connector runs across where its note stands beside the state.
        assert_eq!(
            connector_ends(&two, Side::After, real, places[0]),
            (Point::new(100.0, 8.0), Point::new(132.0, 8.0))
        );
        assert_eq!(
            connector_ends(&two, Side::After, real, places[1]),
            (Point::new(100.0, 32.0), Point::new(132.0, 32.0))
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
    fn the_notes_leave_the_layout_and_their_state_keeps_their_room_on_their_side() {
        let mut data = noted();
        let notes = take_out(&mut data, &HashSet::new(), Some(&HashMap::new()))
            .expect("the note can stand beside its state");
        let ids: Vec<&str> = data.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(ids, ["A", "B"]);
        assert_eq!(data.edges.len(), 1);
        // The state keeps its size; the note's room is on its right only.
        assert_eq!(data.nodes[0].width, Some(100.0));
        assert_eq!(data.nodes[0].height, Some(40.0));
        assert_eq!(data.nodes[0].beside, [(100.0 + NOTE_GAP, 0.0, 100.0, 40.0)]);
        assert_eq!(notes.nodes.len(), 2);
        assert_eq!(notes.edges.len(), 1);
    }

    #[test]
    fn a_state_with_a_loop_keeps_its_note_beside_it() {
        let mut data = noted();
        data.edges.push(edge("e2", "A", "A", "transition"));
        let notes = take_out(&mut data, &HashSet::new(), Some(&HashMap::new()))
            .expect("the note beside its state");
        let reserved = notes.reserved();
        assert_eq!(reserved.len(), 1);
        assert_eq!(reserved[0].id, "A");
    }

    #[test]
    fn a_state_with_an_invisible_loop_keeps_its_note_where_the_reference_puts_it() {
        let mut data = noted();
        let mut invisible = edge("e2", "A", "A", "transition");
        invisible.thickness = "invisible".to_owned();
        data.edges.push(invisible);
        let before = data.nodes.len();
        assert!(take_out(&mut data, &HashSet::new(), Some(&HashMap::new())).is_none());
        assert_eq!(data.nodes.len(), before);
        assert!(data.nodes[0].beside.is_empty());
    }

    #[test]
    fn a_composite_takes_its_size_from_a_layout_before() {
        let mut data = noted();
        data.nodes[0].is_group = true;
        data.nodes[0].width = None;
        data.nodes[0].height = None;
        // Without sizes, the note leaves the layout without room.
        let mut probe = data.clone();
        assert!(take_out(&mut probe, &HashSet::new(), None).is_some());
        assert!(probe.nodes[0].beside.is_empty());
        // With the composite's size, it keeps its room beside the frame.
        let sizes = HashMap::from([("A".to_owned(), (300.0, 200.0))]);
        assert!(take_out(&mut data, &HashSet::new(), Some(&sizes)).is_some());
        assert_eq!(data.nodes[0].beside, [(332.0, 80.0, 100.0, 40.0)]);
    }
}
