//! The architecture DB, `packages/mermaid/src/diagrams/architecture/architectureDb.ts`: the state
//! the `populate` calls change, with the reference's checks and messages, and `getDataStructures`
//! that the layout reads. The order of the calls and the quirks are in
//! `docs-port/architecture/PLAN.md`, sections 3 and 4.
//!
//! The reference's maps keep insertion order, and `Map.set` on a key that is there keeps the key's
//! place and takes the new value. `IndexMap::insert` does the same, so the adjacency lists, the
//! group alignments and the spatial maps are indexed maps.

use std::collections::{HashMap, HashSet, VecDeque};

use ferrigura_model::CANONICAL_VERSION;
use ferrigura_model::architecture::{
    AlignDirection, ArchitectureAlignment, ArchitectureDerived, ArchitectureDiagram,
    ArchitectureEdge, ArchitectureGroup, ArchitectureJunction, ArchitectureService, Axis,
    Direction, SpatialPosition,
};
use indexmap::IndexMap;

use super::grammar::{AlignStmt, EdgeStmt, GroupStmt, JunctionStmt, ServiceStmt};
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// A DB error: the message the reference throws, which is kept exactly.
pub(crate) type DbResult = Result<(), String>;

/// What a registered id names. The reference's `registeredIds` is `'node' | 'group'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Group,
    Node,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Node => "node",
        }
    }
}

/// A service or a junction, as the DB stores a node. Its edges are indices into the DB's edges, in
/// the order the edges were added, and a self-loop is listed twice, as `addEdge` pushes it.
#[derive(Debug, Clone)]
struct NodeRecord {
    id: String,
    service: Option<ServiceFields>,
    parent: Option<String>,
    edges: Vec<usize>,
}

/// The fields of a service that the model keeps.
#[derive(Debug, Clone)]
struct ServiceFields {
    icon: Option<String>,
    icon_text: Option<String>,
    title: Option<String>,
}

/// The state of an architecture diagram while its statements are applied.
pub(crate) struct Db {
    title: String,
    acc_title: String,
    acc_descr: String,
    groups: Vec<ArchitectureGroup>,
    registered: HashMap<String, Kind>,
    /// The services, then the junctions: the reference adds every service before any junction, so
    /// its `nodes` map holds them in that order.
    nodes: Vec<NodeRecord>,
    node_index: HashMap<String, usize>,
    edges: Vec<ArchitectureEdge>,
    hints: Vec<ArchitectureAlignment>,
}

impl Db {
    /// A cleared DB, as `clear()` leaves it.
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            groups: Vec::new(),
            registered: HashMap::new(),
            nodes: Vec::new(),
            node_index: HashMap::new(),
            edges: Vec::new(),
            hints: Vec::new(),
        }
    }

    /// `setDiagramTitle`: the text is sanitised.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: the text is sanitised, and its leading whitespace is removed.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: the text is sanitised, and the whitespace after each line break is
    /// removed.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `addGroup`: the id must be free, and the parent, when given, must be an earlier group.
    pub(crate) fn add_group(&mut self, group: &GroupStmt) -> DbResult {
        let id = &group.id;
        if let Some(kind) = self.registered.get(id) {
            return Err(format!(
                "The group id [{id}] is already in use by another {}",
                kind.name()
            ));
        }
        if let Some(parent) = &group.parent {
            if id == parent {
                return Err(format!("The group [{id}] cannot be placed within itself"));
            }
            match self.registered.get(parent) {
                None => {
                    return Err(format!(
                        "The group [{id}]'s parent does not exist. Please make sure the parent is created before this group"
                    ));
                }
                Some(Kind::Node) => {
                    return Err(format!("The group [{id}]'s parent is not a group"));
                }
                Some(Kind::Group) => {}
            }
        }

        self.registered.insert(id.clone(), Kind::Group);
        self.groups.push(ArchitectureGroup {
            id: id.clone(),
            icon: group.icon.clone().unwrap_or_default(),
            title: group.title.clone().unwrap_or_default(),
            parent: group.parent.clone().unwrap_or_default(),
        });
        Ok(())
    }

    /// `addService`: the same checks as `addGroup`, and the parent must be a group.
    pub(crate) fn add_service(&mut self, service: &ServiceStmt) -> DbResult {
        let id = &service.id;
        self.check_free(id, "service")?;
        self.check_parent(id, service.parent.as_deref(), "service")?;
        self.insert_node(
            id,
            Some(ServiceFields {
                icon: service.icon.clone(),
                icon_text: service.icon_text.clone(),
                title: service.title.clone(),
            }),
            service.parent.clone(),
        );
        Ok(())
    }

    /// `addJunction`: the same checks as `addService`, and a junction has no icon or title.
    pub(crate) fn add_junction(&mut self, junction: &JunctionStmt) -> DbResult {
        let id = &junction.id;
        self.check_free(id, "junction")?;
        self.check_parent(id, junction.parent.as_deref(), "junction")?;
        self.insert_node(id, None, junction.parent.clone());
        Ok(())
    }

    /// The id of a service or junction must not be registered yet.
    fn check_free(&self, id: &str, what: &str) -> DbResult {
        match self.registered.get(id) {
            Some(kind) => Err(format!(
                "The {what} id [{id}] is already in use by another {}",
                kind.name()
            )),
            None => Ok(()),
        }
    }

    /// The parent of a service or junction: not itself, registered, and a group.
    fn check_parent(&self, id: &str, parent: Option<&str>, what: &str) -> DbResult {
        let Some(parent) = parent else {
            return Ok(());
        };
        if id == parent {
            return Err(format!("The {what} [{id}] cannot be placed within itself"));
        }
        match self.registered.get(parent) {
            None => Err(format!(
                "The {what} [{id}]'s parent does not exist. Please make sure the parent is created before this {what}"
            )),
            Some(Kind::Node) => Err(format!("The {what} [{id}]'s parent is not a group")),
            Some(Kind::Group) => Ok(()),
        }
    }

    fn insert_node(&mut self, id: &str, service: Option<ServiceFields>, parent: Option<String>) {
        self.registered.insert(id.to_owned(), Kind::Node);
        self.node_index.insert(id.to_owned(), self.nodes.len());
        self.nodes.push(NodeRecord {
            id: id.to_owned(),
            service,
            parent,
            edges: Vec::new(),
        });
    }

    /// `addEdge`: both ends must be registered, and neither may be a group, since the reference
    /// reads the node of each end (`this.nodes.get(id)!.in`), which a group does not have. A
    /// `{group}` mark must cross two groups.
    pub(crate) fn add_edge(&mut self, edge: &EdgeStmt) -> DbResult {
        if !self.registered.contains_key(&edge.lhs_id) {
            return Err(format!(
                "The left-hand id [{}] does not yet exist. Please create the service/group before declaring an edge to it.",
                edge.lhs_id
            ));
        }
        if !self.registered.contains_key(&edge.rhs_id) {
            return Err(format!(
                "The right-hand id [{}] does not yet exist. Please create the service/group before declaring an edge to it.",
                edge.rhs_id
            ));
        }
        // One end is a group: the reference reads `.in` of a node that is not there and throws.
        let (Some(&lhs), Some(&rhs)) = (
            self.node_index.get(&edge.lhs_id),
            self.node_index.get(&edge.rhs_id),
        ) else {
            return Err(format!(
                "The edge between [{}] and [{}] must end at a service or junction, not at a group",
                edge.lhs_id, edge.rhs_id
            ));
        };

        let lhs_parent = self.nodes[lhs].parent.as_deref();
        let rhs_parent = self.nodes[rhs].parent.as_deref();
        let same_group = matches!((lhs_parent, rhs_parent), (Some(a), Some(b)) if a == b);
        if edge.lhs_group && same_group {
            return Err(format!(
                "The left-hand id [{}] is modified to traverse the group boundary, but the edge does not pass through two groups.",
                edge.lhs_id
            ));
        }
        if edge.rhs_group && same_group {
            return Err(format!(
                "The right-hand id [{}] is modified to traverse the group boundary, but the edge does not pass through two groups.",
                edge.rhs_id
            ));
        }

        let index = self.edges.len();
        self.edges.push(ArchitectureEdge {
            lhs_id: edge.lhs_id.clone(),
            lhs_dir: edge.lhs_dir,
            lhs_into: edge.lhs_into,
            lhs_group: edge.lhs_group,
            rhs_id: edge.rhs_id.clone(),
            rhs_dir: edge.rhs_dir,
            rhs_into: edge.rhs_into,
            rhs_group: edge.rhs_group,
            title: edge.title.clone().unwrap_or_default(),
        });
        self.nodes[lhs].edges.push(index);
        self.nodes[rhs].edges.push(index);
        Ok(())
    }

    /// `addLayoutHint`: at least two members, each a registered service or junction, and none
    /// listed twice.
    pub(crate) fn add_layout_hint(&mut self, alignment: &AlignStmt) -> DbResult {
        let direction = direction_name(alignment.direction);
        if alignment.members.len() < 2 {
            return Err(format!(
                "An align directive requires at least two members; got {}",
                alignment.members.len()
            ));
        }
        let mut seen = HashSet::new();
        for id in &alignment.members {
            if self.registered.get(id) != Some(&Kind::Node) {
                return Err(format!(
                    "align {direction} references [{id}], which is not a service or junction"
                ));
            }
            if !seen.insert(id) {
                return Err(format!("align {direction} lists [{id}] more than once"));
            }
        }
        self.hints.push(ArchitectureAlignment {
            direction: alignment.direction,
            members: alignment.members.clone(),
        });
        Ok(())
    }

    /// The parent of a registered node, when it has one.
    fn parent_of(&self, id: &str) -> Option<&str> {
        let index = *self.node_index.get(id)?;
        self.nodes.get(index)?.parent.as_deref()
    }

    /// `getDataStructures`: the group alignments, the adjacency lists and the spatial maps (one per
    /// connected component), derived in the reference's order.
    fn derived(&self) -> ArchitectureDerived {
        let mut group_alignments: IndexMap<String, Axis> = IndexMap::new();
        let mut adjacency: Vec<IndexMap<(Direction, Direction), usize>> = Vec::new();
        for node in &self.nodes {
            let mut direction_map: IndexMap<(Direction, Direction), usize> = IndexMap::new();
            for &edge_index in &node.edges {
                let Some(edge) = self.edges.get(edge_index) else {
                    continue;
                };
                if let (Some(lhs_group), Some(rhs_group)) =
                    (self.parent_of(&edge.lhs_id), self.parent_of(&edge.rhs_id))
                    && lhs_group != rhs_group
                    && let Some(axis) = group_axis(edge.lhs_dir, edge.rhs_dir)
                {
                    group_alignments.insert(group_key(lhs_group, rhs_group), axis);
                }

                // The pair starts at the node's own side, and a pair that repeats a side is no pair.
                let (pair, neighbour) = if edge.lhs_id == node.id {
                    ((edge.lhs_dir, edge.rhs_dir), &edge.rhs_id)
                } else {
                    ((edge.rhs_dir, edge.lhs_dir), &edge.lhs_id)
                };
                if pair.0 == pair.1 {
                    continue;
                }
                if let Some(&neighbour) = self.node_index.get(neighbour) {
                    direction_map.insert(pair, neighbour);
                }
            }
            adjacency.push(direction_map);
        }

        // The breadth-first search of `getDataStructures`: `visited` is set when a node is taken
        // from the queue, `pending` holds the nodes not yet taken, in the order of the nodes.
        let count = self.nodes.len();
        let mut visited = vec![false; count];
        let mut pending = vec![true; count];
        let mut spatial_maps: Vec<Vec<SpatialPosition>> = Vec::new();
        while let Some(first) = pending.iter().position(|&left| left) {
            let mut spatial: IndexMap<usize, (i64, i64)> = IndexMap::new();
            spatial.insert(first, (0, 0));
            let mut queue = VecDeque::from([first]);
            while let Some(current) = queue.pop_front() {
                if let Some(seen) = visited.get_mut(current) {
                    *seen = true;
                }
                if let Some(left) = pending.get_mut(current) {
                    *left = false;
                }
                let Some(&position) = spatial.get(&current) else {
                    continue;
                };
                let Some(adjacent) = adjacency.get(current) else {
                    continue;
                };
                for (&pair, &neighbour) in adjacent.iter() {
                    if visited.get(neighbour) != Some(&true) {
                        spatial.insert(neighbour, shift(position, pair));
                        queue.push_back(neighbour);
                    }
                }
            }
            spatial_maps.push(
                spatial
                    .iter()
                    .filter_map(|(&index, &(x, y))| {
                        Some(SpatialPosition {
                            id: self.nodes.get(index)?.id.clone(),
                            x,
                            y,
                        })
                    })
                    .collect(),
            );
        }

        ArchitectureDerived {
            group_alignments: group_alignments.into_iter().collect(),
            spatial_maps,
        }
    }

    /// The model of the DB state: the header texts, the lists in the DB's order, and the derived
    /// block.
    pub(crate) fn into_model(self) -> ArchitectureDiagram {
        let derived = self.derived();
        let mut services = Vec::new();
        let mut junctions = Vec::new();
        for node in &self.nodes {
            match &node.service {
                Some(fields) => services.push(ArchitectureService {
                    id: node.id.clone(),
                    icon: fields.icon.clone().unwrap_or_default(),
                    icon_text: fields.icon_text.clone().unwrap_or_default(),
                    title: fields.title.clone().unwrap_or_default(),
                    parent: node.parent.clone().unwrap_or_default(),
                }),
                None => junctions.push(ArchitectureJunction {
                    id: node.id.clone(),
                    parent: node.parent.clone().unwrap_or_default(),
                }),
            }
        }
        ArchitectureDiagram {
            version: CANONICAL_VERSION,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            groups: self.groups,
            services,
            junctions,
            edges: self.edges,
            alignments: self.hints,
            derived,
        }
    }
}

/// The name of an alignment direction, as the reference's messages write it.
fn direction_name(direction: AlignDirection) -> &'static str {
    match direction {
        AlignDirection::Row => "row",
        AlignDirection::Column => "column",
    }
}

/// `getArchitectureDirectionAlignment`: the axis of two sides that both lie on one axis, or `None`
/// for a bend, where one side is horizontal and the other vertical.
fn group_axis(lhs: Direction, rhs: Direction) -> Option<Axis> {
    if lhs.is_horizontal() != rhs.is_horizontal() {
        None
    } else if lhs.is_horizontal() {
        Some(Axis::Horizontal)
    } else {
        Some(Axis::Vertical)
    }
}

/// `architectureGroupAlignmentKey`: the two group ids, sorted, each JSON-quoted, joined by `-`.
/// The ids are word characters and dashes, so the quotes need no escapes.
fn group_key(a: &str, b: &str) -> String {
    let (lower, upper) = if a <= b { (a, b) } else { (b, a) };
    format!("\"{lower}\"-\"{upper}\"")
}

/// `shiftPositionByArchitectureDirectionPair`: the grid position of a neighbour, given the side
/// the edge leaves the current node by (`pair.0`) and the side it enters the neighbour by (`pair.1`).
fn shift((x, y): (i64, i64), (lhs, rhs): (Direction, Direction)) -> (i64, i64) {
    let step_x = |side: Direction| if side == Direction::L { -1 } else { 1 };
    let step_y = |side: Direction| if side == Direction::T { 1 } else { -1 };
    if lhs.is_horizontal() {
        if rhs.is_horizontal() {
            (x + step_x(lhs), y)
        } else {
            (x + step_x(lhs), y + step_y(rhs))
        }
    } else if rhs.is_horizontal() {
        (
            x + if rhs == Direction::L { 1 } else { -1 },
            y + step_y(lhs),
        )
    } else {
        (x, y + step_y(lhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bend_is_no_group_alignment() {
        assert_eq!(
            group_axis(Direction::R, Direction::L),
            Some(Axis::Horizontal)
        );
        assert_eq!(group_axis(Direction::T, Direction::B), Some(Axis::Vertical));
        assert_eq!(group_axis(Direction::R, Direction::T), None);
    }

    #[test]
    fn the_group_key_sorts_the_ids() {
        assert_eq!(group_key("b", "a"), "\"a\"-\"b\"");
    }

    #[test]
    fn a_shift_moves_one_step_on_the_side_the_edge_leaves() {
        assert_eq!(shift((0, 0), (Direction::R, Direction::L)), (1, 0));
        assert_eq!(shift((0, 0), (Direction::L, Direction::R)), (-1, 0));
        assert_eq!(shift((0, 0), (Direction::B, Direction::T)), (0, -1));
        assert_eq!(shift((0, 0), (Direction::T, Direction::B)), (0, 1));
        assert_eq!(shift((0, 0), (Direction::R, Direction::T)), (1, 1));
    }

    #[test]
    fn an_alignment_needs_two_members_at_the_db_level() {
        // The grammar always gives two members (`ID ID+`); the DB checks the count for any other
        // caller, as `addLayoutHint` does.
        let alignment = AlignStmt {
            direction: AlignDirection::Row,
            members: vec!["only".to_owned()],
            offset: 0,
        };
        let message = Db::new()
            .add_layout_hint(&alignment)
            .expect_err("one member is rejected");
        assert!(message.contains("at least two members"), "{message}");
    }
}
