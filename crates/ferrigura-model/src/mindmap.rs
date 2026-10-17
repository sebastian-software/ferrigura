//! Canonical model of a mindmap diagram: the tree of nodes the DB holds after the parse.
//!
//! The DB keeps every node with its id, its text and shape, its padding and width, its icon and
//! class, and its children. The layout data (`getData`) is the renderer stage and is not here; see
//! `docs-port/mindmap/PLAN.md`, sections 3 and 4.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;
use crate::value::Value;

/// A parsed mindmap: the root of the tree, or none when the text has no node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MindmapDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The first node given, which is the root of the tree. `None` when there is no node.
    pub root: Option<MindmapNode>,
}

/// A node of the mindmap, as the DB stores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MindmapNode {
    /// The DB's counter: the position of the node in the order the parser added the nodes.
    pub id: u32,
    /// The id of the node (the text before its shape), sanitised.
    pub node_id: String,
    /// The text of the node, sanitised, with the entity placeholders of the DB.
    pub descr: String,
    /// The shape: `0` default, `1` rounded rectangle, `2` rectangle, `3` circle, `4` cloud, `5`
    /// bang and `6` hexagon.
    #[serde(rename = "type")]
    pub node_type: u8,
    /// The indentation of the node relative to the root, in columns.
    pub level: u32,
    /// The width the config gave the node: `mindmap.maxNodeWidth`, 200 by default. The DB keeps the
    /// configuration's value as it is, so a width that is not a number stays what it was given.
    pub width: Value,
    /// The padding of the node: `mindmap.padding`, 10 by default, doubled for the rounded
    /// rectangle, the rectangle and the hexagon. The doubling is JavaScript's `*= 2`, so a value
    /// that is not a number becomes one; the other shapes keep the value as it was given.
    pub padding: Value,
    /// The icon set with `::icon(…)`, the last one given; empty when there is none.
    pub icon: String,
    /// The classes set with `:::`, the last ones given, with their blanks; empty when there is none.
    pub class: String,
    /// The children, in the order they were added.
    pub children: Vec<MindmapNode>,
}

impl MindmapDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            root: None,
        }
    }
}

impl Default for MindmapDiagram {
    fn default() -> Self {
        Self::new()
    }
}
