//! Canonical model of an architecture diagram: the state of the architecture DB after the parse
//! (`architectureDb.ts`), with the derived block that the layout depends on.
//!
//! The field list and the exclusions are in `docs-port/architecture/PLAN.md`, section 3, and the
//! conventions in `docs-port/canonical-model.md`, Architecture.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A side of an edge's endpoint: `L`, `R`, `T` or `B`, as the arrow's port letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// Left.
    L,
    /// Right.
    R,
    /// Top.
    T,
    /// Bottom.
    B,
}

impl Direction {
    /// Whether the side is on the horizontal axis (`L` or `R`).
    #[must_use]
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::L | Self::R)
    }
}

/// The direction of a `align` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlignDirection {
    /// `align row`.
    Row,
    /// `align column`.
    Column,
}

/// The alignment that two groups have, as the edges between them give it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    /// The groups sit side by side.
    Horizontal,
    /// The groups sit one above the other.
    Vertical,
}

/// A parsed architecture diagram: its header texts, its groups, services, junctions, edges and
/// alignments in the order the DB holds them, and the derived block.
///
/// Texts are the DB's values: the header texts are sanitised, the others are the grammar's values
/// as written. Groups are in text order; services and junctions are separate lists, each in text
/// order, and the DB adds them all after the groups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title: the grammar's `title` when it is non-empty, else the front matter's, sanitised.
    pub title: String,
    /// Accessible title from `accTitle`, sanitised and without leading whitespace.
    pub acc_title: String,
    /// Accessible description from `accDescr`, sanitised.
    pub acc_descr: String,
    /// The groups in text order.
    pub groups: Vec<ArchitectureGroup>,
    /// The services in text order.
    pub services: Vec<ArchitectureService>,
    /// The junctions in text order.
    pub junctions: Vec<ArchitectureJunction>,
    /// The edges in text order.
    pub edges: Vec<ArchitectureEdge>,
    /// The `align` statements in text order.
    pub alignments: Vec<ArchitectureAlignment>,
    /// What `getDataStructures` derives from the edges: the group alignments and the spatial maps.
    pub derived: ArchitectureDerived,
}

/// A group: an id, an icon name, a title and the group it sits in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureGroup {
    /// The id.
    pub id: String,
    /// The icon name, without its parentheses; empty if none.
    pub icon: String,
    /// The title, without its brackets; empty if none.
    pub title: String,
    /// The id of the enclosing group; empty if the group is at the top.
    #[serde(rename = "in")]
    pub parent: String,
}

/// A service: an id, an icon name or an icon text, a title and the group it sits in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureService {
    /// The id.
    pub id: String,
    /// The icon name, without its parentheses; empty if the service has none or an icon text.
    pub icon: String,
    /// The icon text (a quoted string after the id), unquoted; empty if none.
    pub icon_text: String,
    /// The title, without its brackets; empty if none.
    pub title: String,
    /// The id of the enclosing group; empty if the service is at the top.
    #[serde(rename = "in")]
    pub parent: String,
}

/// A junction: an id and the group it sits in. A junction has no icon and no title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureJunction {
    /// The id.
    pub id: String,
    /// The id of the enclosing group; empty if the junction is at the top.
    #[serde(rename = "in")]
    pub parent: String,
}

/// An edge between two services or junctions, with the side of each end, the arrow heads, the
/// `{group}` modifiers and the label. The arrow heads and the modifiers are booleans: the grammar
/// keeps only whether the token was there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureEdge {
    /// The id of the left-hand end.
    pub lhs_id: String,
    /// The side of the left-hand end.
    pub lhs_dir: Direction,
    /// Whether the left-hand end has an arrow head (`<` or `>` before the line).
    pub lhs_into: bool,
    /// Whether the left-hand id is followed by `{group}`.
    pub lhs_group: bool,
    /// The id of the right-hand end.
    pub rhs_id: String,
    /// The side of the right-hand end.
    pub rhs_dir: Direction,
    /// Whether the right-hand end has an arrow head (`<` or `>` after the line).
    pub rhs_into: bool,
    /// Whether the right-hand id is followed by `{group}`.
    pub rhs_group: bool,
    /// The label between the dashes, unbracketed; empty if none.
    pub title: String,
}

/// An `align row` or `align column` statement and its members in the order given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureAlignment {
    /// The direction of the alignment.
    pub direction: AlignDirection,
    /// The members: service and junction ids.
    pub members: Vec<String>,
}

/// The derived block: `getDataStructures` of the DB, memoised there and read by the layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureDerived {
    /// The alignment of each pair of groups, keyed by `"A"-"B"` (the two ids JSON-quoted, sorted),
    /// in the order the DB first set each key. A pair set again keeps its place and takes the new
    /// alignment.
    pub group_alignments: Vec<(String, Axis)>,
    /// One spatial map per connected component of the services and junctions, in the order the
    /// breadth-first search finds them. Each map lists its nodes in the order the map holds them.
    pub spatial_maps: Vec<Vec<SpatialPosition>>,
}

/// A node's grid position in a spatial map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialPosition {
    /// The id of the service or junction.
    pub id: String,
    /// The column; the start of the search is at column 0.
    pub x: i64,
    /// The row; the start of the search is at row 0.
    pub y: i64,
}

impl ArchitectureDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            groups: Vec::new(),
            services: Vec::new(),
            junctions: Vec::new(),
            edges: Vec::new(),
            alignments: Vec::new(),
            derived: ArchitectureDerived {
                group_alignments: Vec::new(),
                spatial_maps: Vec::new(),
            },
        }
    }
}

impl Default for ArchitectureDiagram {
    fn default() -> Self {
        Self::new()
    }
}
