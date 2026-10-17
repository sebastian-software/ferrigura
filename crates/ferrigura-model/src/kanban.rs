//! Canonical model of a kanban diagram. Not ported yet: the model holds the version until the
//! parser of the type is ported (`docs-port/P2-PLAN.md`, the registries).

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed kanban diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
}

impl KanbanDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
        }
    }
}

impl Default for KanbanDiagram {
    fn default() -> Self {
        Self::new()
    }
}
