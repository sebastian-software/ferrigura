//! Canonical model of a cynefin diagram: the state of the cynefin DB after the parse
//! (`cynefinDb.ts`), with the header texts and the two lists the renderer reads.
//!
//! The field list and the exclusions are in `docs-port/cynefin/NOTES.md`, section 4, and the
//! conventions in `docs-port/canonical-model.md`, Cynefin. The configuration is not in the model.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed cynefin diagram.
///
/// The header texts are the DB's values after the sanitiser, with `""` when unset. The domains
/// are in the order of their first appearance, and a name appears once. The items and labels are
/// the strings the DB holds, with the entity placeholders kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CynefinDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title, `""` when there is none.
    pub title: String,
    /// The accessible title, `""` when there is none.
    pub acc_title: String,
    /// The accessible description, `""` when there is none.
    pub acc_descr: String,
    /// The domains that have a block, in order of first appearance.
    pub domains: Vec<CynefinDomain>,
    /// The transitions, without self-loops, in declaration order.
    pub transitions: Vec<CynefinTransition>,
}

/// A domain with its block: `complex`, `complicated`, `clear`, `chaotic` or `confusion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CynefinDomain {
    /// The domain's name.
    pub name: String,
    /// The item labels of the block, in declaration order.
    pub items: Vec<String>,
}

/// An arrow between two domains. `label` is `null` when the transition has no label or an empty
/// one, as the DB stores `undefined` for both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CynefinTransition {
    /// The domain the arrow starts at.
    pub from: String,
    /// The domain the arrow points to.
    pub to: String,
    /// The label, when there is one.
    pub label: Option<String>,
}

impl CynefinDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            domains: Vec::new(),
            transitions: Vec::new(),
        }
    }
}

impl Default for CynefinDiagram {
    fn default() -> Self {
        Self::new()
    }
}
