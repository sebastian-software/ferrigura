//! Canonical model of a timeline diagram.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed timeline: its direction, its header texts, its sections in the order they were given,
/// and its periods in creation order with their events.
///
/// Texts are the encoded text the reference's DB holds, as `docs-port/canonical-model.md` (Timeline)
/// describes; the header texts are sanitised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// `"LR"`, the default, or `"TD"`.
    pub direction: String,
    /// The grammar's last `title` line, even when it is blank; the front matter title does not reach
    /// it.
    pub title: String,
    /// Accessible title from `accTitle`, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, empty if absent.
    pub acc_descr: String,
    /// The section names in the order they were given. A name given twice appears twice.
    pub sections: Vec<String>,
    /// The periods in the order they were created.
    pub periods: Vec<TimelinePeriod>,
}

/// One period of a timeline, with the events given after it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePeriod {
    /// The section that was current when the period was created, `""` before the first section.
    pub section: String,
    /// The period text, untrimmed, with its trailing blanks.
    pub period: String,
    /// The events of the period in order, untrimmed.
    pub events: Vec<String>,
}

impl TimelineDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and direction `LR`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            direction: String::from("LR"),
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            sections: Vec::new(),
            periods: Vec::new(),
        }
    }
}

impl Default for TimelineDiagram {
    fn default() -> Self {
        Self::new()
    }
}
