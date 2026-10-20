//! Canonical model of a sankey diagram: the DB state after the parse (`sankeyDB.ts`), with the
//! field list of `docs-port/sankey/PLAN.md`, section 6. The layout is the renderer's, and is not in
//! the model.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed sankey diagram: its header texts, its nodes in the order the DB created them, and its
/// links in the order they were given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SankeyDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The front matter's `title`, sanitised, or empty. The grammar has no title statement, and
    /// the sankey never draws the title, but the DB holds it, so the model does too.
    pub title: String,
    /// The DB's `accTitle`. The grammar has no statement for it, so it is always empty.
    pub acc_title: String,
    /// The DB's `accDescr`. The grammar has no statement for it, so it is always empty.
    pub acc_descr: String,
    /// The nodes, each named once, in the order the DB created them: the source of a record before
    /// its target, in order of first appearance.
    pub nodes: Vec<SankeyNode>,
    /// Every link in input order. Duplicates are kept.
    pub links: Vec<SankeyLink>,
}

/// A node of a sankey diagram. Its id is the name as the DB sanitises it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SankeyNode {
    /// The sanitised name.
    pub id: String,
}

/// A link of a sankey diagram, from a source node to a target node, both named by their ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SankeyLink {
    /// The id of the node the link starts at.
    pub source: String,
    /// The id of the node the link ends at.
    pub target: String,
    /// The value, read as `parseFloat` reads the field. A value that is not finite is written as
    /// its sentinel string, see `value`.
    #[serde(with = "value")]
    pub value: f64,
}

impl SankeyDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            nodes: Vec::new(),
            links: Vec::new(),
        }
    }
}

impl Default for SankeyDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A value as JSON: a number when it is finite, and the sentinel strings for the other values, as
/// the other models write them (`docs-port/canonical-model.md`, Numbers).
mod value {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if value.is_finite() {
            serializer.serialize_f64(*value)
        } else if value.is_nan() {
            serializer.serialize_str("NaN")
        } else if *value > 0.0 {
            serializer.serialize_str("Infinity")
        } else {
            serializer.serialize_str("-Infinity")
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Number(f64),
        Sentinel(String),
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        match Written::deserialize(deserializer)? {
            Written::Number(number) => Ok(number),
            Written::Sentinel(text) => match text.as_str() {
                "NaN" => Ok(f64::NAN),
                "Infinity" => Ok(f64::INFINITY),
                "-Infinity" => Ok(f64::NEG_INFINITY),
                other => Err(serde::de::Error::custom(format!(
                    "expected a number or NaN, Infinity or -Infinity, found {other:?}"
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(value: f64) -> SankeyLink {
        SankeyLink {
            source: "a".to_owned(),
            target: "b".to_owned(),
            value,
        }
    }

    #[test]
    fn a_finite_value_is_a_number() {
        let json = serde_json::to_value(link(1.5)).expect("serialises");
        assert_eq!(json["value"], serde_json::json!(1.5));
    }

    #[test]
    fn a_value_that_is_not_finite_is_its_sentinel_string() {
        let json = |value| serde_json::to_value(link(value)).expect("serialises")["value"].clone();
        assert_eq!(json(f64::NAN), serde_json::json!("NaN"));
        assert_eq!(json(f64::INFINITY), serde_json::json!("Infinity"));
        assert_eq!(json(f64::NEG_INFINITY), serde_json::json!("-Infinity"));
    }

    #[test]
    fn the_sentinel_strings_read_back() {
        let read: SankeyLink = serde_json::from_value(serde_json::json!({
            "source": "a", "target": "b", "value": "-Infinity"
        }))
        .expect("reads");
        assert_eq!(read.value, f64::NEG_INFINITY);
    }

    #[test]
    fn the_header_texts_are_camel_case_and_empty_by_default() {
        let json = serde_json::to_value(SankeyDiagram::new()).expect("serialises");
        assert_eq!(json["version"], serde_json::json!(1));
        assert_eq!(json["title"], serde_json::json!(""));
        assert_eq!(json["accTitle"], serde_json::json!(""));
        assert_eq!(json["accDescr"], serde_json::json!(""));
        assert_eq!(json["nodes"], serde_json::json!([]));
        assert_eq!(json["links"], serde_json::json!([]));
    }
}
