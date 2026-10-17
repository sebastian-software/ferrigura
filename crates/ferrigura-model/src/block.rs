//! Canonical model of a block diagram: the DB state after the parse, as `docs-port/block/PLAN.md`
//! describes it. The layout of the blocks is the renderer's, and is not in the model.
//!
//! A field the reference leaves `undefined` is absent from the JSON (`docs-port/canonical-model.md`).
//! The one exception is the quirk of a chained edge (`A --> B --> C`), whose first block has no id
//! and no type; that block is written as an empty object, as the reference's DB holds it.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed block diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The columns of the diagram, from the last `columns` statement at its top level: `-1` when
    /// none is given or it is `auto`. Absent when the DB's `root` entry has no columns, which
    /// happens only when the text names a block `root` in a class list with blanks around it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<f64>,
    /// The top-level blocks in the order they were declared, composites with their children.
    pub blocks: Vec<Block>,
    /// Every edge in the order it was declared. Ids carry the count of their key, so `1-A-B` and
    /// `2-A-B` are two edges of the same pair.
    pub edges: Vec<Edge>,
    /// The `classDef` entries in the order they were first declared.
    pub class_defs: Vec<ClassDef>,
}

/// A block of the tree: a node, a space cell, or a composite with its children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// The id. Ids the reference generates at random are the port's deterministic ids, which the
    /// differential replaces by placeholders (`docs-port/block/PLAN.md`, section 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The shape or kind: `na` for a plain id, `square`, `round`, …, `block_arrow`, `composite`
    /// or `space`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub block_type: Option<String>,
    /// The label, sanitised, with the entity placeholders the DB keeps. A space cell's label is
    /// its generated id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The number of cells a space statement takes (`space:N`), on each cell of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// The span of a block (`A:2`), as the grammar gives it. Absent on the nodes of an edge, whose
    /// action copies only the id, label, type and directions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_in_columns: Option<f64>,
    /// The columns of a composite, from its own `columns` statement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<f64>,
    /// The directions of a block arrow, as written: `x` and `y` are kept, and blanks written after
    /// a direction are kept too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directions: Option<Vec<String>>,
    /// The classes applied with `class`, in order, duplicates kept. A `class` statement without
    /// styles applies the empty class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The styles of a `style` statement, split at each comma and not trimmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    /// The palette slot of a composite: its index among the composites of the parse, in
    /// declaration order, outer composites first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_index: Option<u32>,
    /// The blocks inside a composite or the root's blocks; a space cell has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Block>>,
}

/// An edge between two blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// The count of the key among the edges of the parse, then `-`, then the key (`start-end`).
    pub id: String,
    /// The start of the edge. Absent on the edge of a chained link (`A --> B --> C`), whose start
    /// is the `undefined` block of the reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// The end of the edge.
    pub end: String,
    /// The label of the link, sanitised; empty when there is none.
    pub label: String,
    /// `thick` for `==`, `normal` otherwise.
    pub thickness: String,
    /// `dotted` for `-.`, `solid` otherwise.
    pub pattern: String,
    /// The directions of a block arrow at the end, when the end is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directions: Option<Vec<String>>,
    /// `arrow_point`, `arrow_cross`, `arrow_circle`, or empty when the link has no end arrow.
    pub arrow_type_end: String,
    /// `arrow_point`, `arrow_cross`, `arrow_circle`, or `arrow_open` when no mark starts the link.
    pub arrow_type_start: String,
}

/// A `classDef` entry: its styles as written, and the text styles that the reference derives from
/// them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDef {
    /// The class name.
    pub id: String,
    /// Each style of the definition, split at `,`, with the trailing `;` and the blanks removed.
    pub styles: Vec<String>,
    /// The text style of each style that mentions `color`: `fill` for `color`, `bgFill` for
    /// `fill` first.
    pub text_styles: Vec<String>,
}

impl BlockDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            columns: Some(-1.0),
            blocks: Vec::new(),
            edges: Vec::new(),
            class_defs: Vec::new(),
        }
    }
}

impl Default for BlockDiagram {
    fn default() -> Self {
        Self::new()
    }
}
