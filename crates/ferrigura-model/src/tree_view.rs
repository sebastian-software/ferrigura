//! Canonical model of a tree view diagram: the DB state after the parse, as
//! `docs-port/tree-view/NOTES.md` section 4 proposes and `docs-port/tree-view/PLAN.md` fixes.
//!
//! The DB always holds a root, the synthetic `/` at level -1 with id 0, which the renderer draws as
//! a directory row. The user's nodes are its descendants. The node's `level` is the number of
//! characters of indentation the grammar gave it (or the depth of a box-drawing line), and the
//! `icon` keeps the three states of the DB: `""` for none, `none` for a suppressed icon, and the
//! name as written otherwise.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed tree view: the title and the accessibility texts, the number of nodes the DB counted,
/// and the root of the tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeViewDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title, sanitised: the grammar's `title` when it is not empty, else the front
    /// matter's, else `""`.
    pub title: String,
    /// The accessible title, sanitised, without its leading blanks.
    pub acc_title: String,
    /// The accessible description, sanitised, with the blanks after each line break removed.
    pub acc_descr: String,
    /// The DB's `getCount()`: the number of nodes plus one, because the counter starts at 1 and the
    /// root takes no number of its own.
    pub count: u32,
    /// The synthetic root `/` and the tree below it.
    pub root: TreeViewNode,
}

/// A node of the tree, as the DB stores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeViewNode {
    /// The DB's counter at the time the node was added: 1 for the first node, and 0 for the root.
    pub id: u32,
    /// The indentation level. The root has -1, a node at the top has 0 and so on.
    pub level: i64,
    /// The name, with the trailing `/` of a directory removed. Entity placeholders are kept.
    pub name: String,
    /// Whether the node is a file or a directory (a name that ended in `/`).
    pub node_type: NodeType,
    /// The icon: `""` when the input gave none, `none` when it was suppressed, else the name.
    pub icon: String,
    /// The class of the `:::` annotation, the last one given; `""` when there is none.
    pub css_class: String,
    /// The description, sanitised; `""` when there is none.
    pub description: String,
    /// The children, in the order they were added.
    pub children: Vec<TreeViewNode>,
}

/// The kind of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// A name without a trailing `/`.
    File,
    /// A name with a trailing `/`, and the root.
    Directory,
}

impl TreeViewNode {
    /// The synthetic root `/` of an empty DB: id 0, level -1, a directory with no children.
    #[must_use]
    pub fn root() -> Self {
        Self {
            id: 0,
            level: -1,
            name: "/".to_owned(),
            node_type: NodeType::Directory,
            icon: String::new(),
            css_class: String::new(),
            description: String::new(),
            children: Vec::new(),
        }
    }
}

impl TreeViewDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and the root alone.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            count: 1,
            root: TreeViewNode::root(),
        }
    }
}

impl Default for TreeViewDiagram {
    fn default() -> Self {
        Self::new()
    }
}
