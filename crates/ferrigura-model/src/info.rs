//! Canonical model of an info diagram.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed info diagram. The info diagram draws only the Mermaid version, which is not part of
/// the input, so the model keeps the header texts the grammar accepts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title from `title`, or from the front matter when the text has none.
    pub title: String,
    /// Accessible title from `accTitle`, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, empty if absent.
    pub acc_descr: String,
}

impl InfoDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
        }
    }
}

impl Default for InfoDiagram {
    fn default() -> Self {
        Self::new()
    }
}
