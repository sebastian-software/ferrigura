//! Canonical model of an ishikawa diagram: the DB state after the parse, as `ishikawaDb.ts` holds
//! it, with the rules of `docs-port/canonical-model.md` and `docs-port/ishikawa/PLAN.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed ishikawa diagram: the title, the accessibility texts, and the tree of the first
/// statement, which is the head of the diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IshikawaDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title. The first statement sets it to its own text, and the front matter title
    /// stands when no statement was read. Sanitised, empty if neither is given.
    pub title: String,
    /// Accessible title. The grammar has no rule that sets it, so it is always empty.
    pub acc_title: String,
    /// Accessible description. The grammar has no rule that sets it, so it is always empty.
    pub acc_descr: String,
    /// The head and its causes, or `None` when no statement was read (a header followed by a line
    /// break, `docs-port/ishikawa/PLAN.md`, quirk I2).
    pub root: Option<IshikawaNode>,
}

/// A node of the diagram: a head, a cause or a sub-cause, with its children in the order given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IshikawaNode {
    /// The sanitised text of the node, with the entity placeholders the DB keeps.
    pub text: String,
    /// The children, in the order of the source.
    pub children: Vec<IshikawaNode>,
}

impl IshikawaDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            root: None,
        }
    }
}

impl Default for IshikawaDiagram {
    fn default() -> Self {
        Self::new()
    }
}
