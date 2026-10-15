//! Canonical model of a pie diagram.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed pie diagram: its header texts and its slices in the order they were first given.
///
/// Texts are the source text after the grammar's value rules, unsanitised and unescaped; the
/// conventions are in `docs-port/canonical-model.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PieDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title: the grammar's `title` when it is non-empty, else the front matter's; empty if
    /// neither is given. See `docs-port/canonical-model.md`, Title precedence.
    pub title: String,
    /// Accessible title from `accTitle`, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, empty if absent.
    pub acc_descr: String,
    /// Whether the header asked for the values to be shown next to the legend (`showData`).
    pub show_data: bool,
    /// The slices in source order. A label given twice keeps its first value and position.
    pub sections: Vec<PieSection>,
}

/// One slice of a pie: a label and its value, which is never negative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PieSection {
    /// The label as written between the quotes, with escapes resolved.
    pub label: String,
    /// The slice value. Negative values are rejected by the parser; `-0` is kept as written.
    pub value: f64,
}

impl PieDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            show_data: false,
            sections: Vec::new(),
        }
    }
}

impl Default for PieDiagram {
    fn default() -> Self {
        Self::new()
    }
}
