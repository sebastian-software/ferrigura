//! The reasons a layout is not computed.

use std::fmt;

/// Why [`crate::layout`] does not lay a graph out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The graph is not a forest: it has a cycle. cose-base then places the nodes at random
    /// (`positionNodesRandomly`) and grows the trees (`reduceTrees`), which is not ported. Mermaid
    /// never gives the engine such a graph, because a mindmap is a tree.
    NotAForest,
    /// Two or more nodes have no edge. cose-base then groups them under a dummy compound (the
    /// tiling path), which is not ported. A mindmap has one root, so it has at most one such node.
    TilingNotPorted,
    /// Two nodes have the same id, which cytoscape refuses.
    DuplicateNode(String),
    /// An edge names a node that is not in the graph.
    UnknownNode(String),
    /// A width or height that is not a finite number.
    NonFiniteSize(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotAForest => {
                write!(
                    f,
                    "the graph has a cycle; the random layout path is not ported"
                )
            }
            Error::TilingNotPorted => write!(
                f,
                "two or more nodes have no edge; the tiling of zero-degree members is not ported"
            ),
            Error::DuplicateNode(id) => write!(f, "two nodes have the id {id:?}"),
            Error::UnknownNode(id) => write!(f, "an edge names the unknown node {id:?}"),
            Error::NonFiniteSize(id) => write!(f, "node {id:?} has a size that is not finite"),
        }
    }
}

impl std::error::Error for Error {}
