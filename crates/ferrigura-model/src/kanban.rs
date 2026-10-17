//! Canonical model of a kanban diagram: the state of the kanban DB after the parse.
//!
//! The model follows the DB of `kanbanDb.ts`, not `getData`: every node the DB holds, in the order
//! the grammar added it, with the fields of its DB object. A section is a node with
//! `isSection: true`; the DB keeps sections and items in one list, and a section is the same object
//! as its entry in that list. The shape of a node is not in the model: the DB never stores one (its
//! only shape check leaves the shape out), and the renderer picks the section or item shape from
//! `isSection`. The width and padding of the DB come from the configuration and are not read by the
//! renderer's data, so they are left out too. See `docs-port/kanban/PLAN.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;
use crate::value::Value;

/// A parsed kanban diagram: its nodes, sections and items alike, in the order the DB holds them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Every node in the order it was added. The sections are the nodes with `is_section`.
    pub nodes: Vec<KanbanNode>,
}

/// One node of the DB: a section, or an item of the section it belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanNode {
    /// The sanitised id, with the blanks and comments the lexer kept. An id that sanitises to
    /// nothing is `kbn` and a counter (`kanbanDb.ts:110`).
    pub id: String,
    /// The indentation of the node: the number of blanks in front of it, zero without any.
    pub level: u32,
    /// The label. A text label is sanitised; a label from the shape data is the value the YAML
    /// gives (a number, a boolean, an array or an object, as the configuration value keeps them).
    #[serde(serialize_with = "sentinels::serialize")]
    pub label: Value,
    /// The id of the section the node belongs to. Absent for a section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Whether the node is a section, a node the DB pushed to its `sections` list.
    pub is_section: bool,
    /// The icon, sanitised when it comes from `::icon(…)`, a string of the YAML otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The assignee, as the YAML value's string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned: Option<String>,
    /// The external ticket, as the YAML value's string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<String>,
    /// The priority, the YAML value as the DB holds it (a string, a number or a boolean).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "sentinels::present"
    )]
    pub priority: Option<Value>,
    /// The classes of `:::…`, sanitised. Empty when the sanitising removed everything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css_classes: Option<String>,
}

/// A value of the shape data as JSON. `serde_json` writes a non-finite number as `null`, so the
/// model writes it as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`, the differential's
/// convention (`value.rs`, `examples/config.rs`). Read back, such a string is a string: a text
/// label that reads `Infinity` cannot be told apart from the number, as in the differential.
mod sentinels {
    use serde::ser::{SerializeMap, SerializeSeq};
    use serde::{Serialize, Serializer};

    use crate::value::Value;

    pub(super) fn serialize<S: Serializer>(
        value: &Value,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(flag) => serializer.serialize_bool(*flag),
            Value::Number(number) if number.is_finite() => serializer.serialize_f64(*number),
            Value::Number(number) => serializer.serialize_str(if number.is_nan() {
                "NaN"
            } else if *number > 0.0 {
                "Infinity"
            } else {
                "-Infinity"
            }),
            Value::String(text) => serializer.serialize_str(text),
            Value::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&Nested(item))?;
                }
                seq.end()
            }
            Value::Object(map) => {
                let mut entries = serializer.serialize_map(Some(map.len()))?;
                for (key, item) in map {
                    entries.serialize_entry(key, &Nested(item))?;
                }
                entries.end()
            }
        }
    }

    /// The same, for an optional value that is written only when it is set.
    pub(super) fn present<S: Serializer>(
        value: &Option<Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => serialize(value, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// A value nested in an array or an object, which takes the same rule.
    struct Nested<'a>(&'a Value);

    impl Serialize for Nested<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serialize(self.0, serializer)
        }
    }
}

impl KanbanDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            nodes: Vec::new(),
        }
    }
}

impl Default for KanbanDiagram {
    fn default() -> Self {
        Self::new()
    }
}
