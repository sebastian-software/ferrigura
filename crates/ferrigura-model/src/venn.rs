//! Canonical model of a venn diagram: the state of the reference's venn DB after the parse, as
//! `docs-port/venn/NOTES.md` section 4 proposes. The port's rules are in `docs-port/venn/PLAN.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed venn diagram: its title, its subsets in the order the statements gave them, its text
/// nodes and its style entries.
///
/// Texts are the DB's values: the title is sanitised, and the entity placeholders that the DB keeps
/// (`ﬂ°name¶ß`) stay as they are. Set ids are sorted as JavaScript's default sort sorts them. The
/// layout's key order (`Object.keys`) is not applied here: the layout takes the subsets in this
/// order and orders its own keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VennDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title: the grammar's last `title` statement, or the front matter's title when the
    /// grammar has none, sanitised. Empty if neither is given.
    pub title: String,
    /// Every subset in the order it was given, duplicates kept.
    pub subsets: Vec<Subset>,
    /// Every text node in the order it was given.
    pub text_nodes: Vec<TextNode>,
    /// Every style statement in the order it was given.
    pub styles: Vec<StyleEntry>,
}

/// A set (one identifier) or a union (several), with its size and its label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subset {
    /// The set ids, sorted; one id for a set, two or more for a union.
    pub sets: Vec<String>,
    /// The size, read as the reference reads it: the number of the statement, or `10 / n²` when it
    /// has none. A size that is not finite is written as its sentinel string, see `size` below.
    #[serde(with = "size")]
    pub size: f64,
    /// The label, empty when the statement has none.
    pub label: String,
}

/// A text node: a text inside the area of its sets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextNode {
    /// The set ids of the area, sorted.
    pub sets: Vec<String>,
    /// The node's id, as the DB holds it: the identifier, string or number after the sets.
    pub id: String,
    /// The label, empty when the statement has none.
    pub label: String,
}

/// The style properties given for one set, a union or a text node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleEntry {
    /// The ids the style applies to, sorted.
    pub targets: Vec<String>,
    /// The properties, as `[key, value]` pairs in the order the DB holds them: a key given twice
    /// keeps its first place and takes its last value.
    pub styles: Vec<(String, String)>,
}

impl VennDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            subsets: Vec::new(),
            text_nodes: Vec::new(),
            styles: Vec::new(),
        }
    }
}

impl Default for VennDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A size as JSON: a number when it is finite, and the sentinel strings for the other values.
mod size {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(size: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if size.is_finite() {
            serializer.serialize_f64(*size)
        } else if size.is_nan() {
            serializer.serialize_str("NaN")
        } else if *size > 0.0 {
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

    fn subset(size: f64) -> Subset {
        Subset {
            sets: vec!["A".to_owned()],
            size,
            label: String::new(),
        }
    }

    #[test]
    fn a_finite_size_is_a_json_number() {
        let json = serde_json::to_value(subset(-5.0)).expect("serialises");
        assert_eq!(json["size"], serde_json::json!(-5.0));
    }

    #[test]
    fn a_non_finite_size_is_a_sentinel_string() {
        let json = |size| serde_json::to_value(subset(size)).expect("serialises")["size"].clone();
        assert_eq!(json(f64::NAN), serde_json::json!("NaN"));
        assert_eq!(json(f64::INFINITY), serde_json::json!("Infinity"));
        assert_eq!(json(f64::NEG_INFINITY), serde_json::json!("-Infinity"));
    }

    #[test]
    fn the_sentinels_read_back() {
        let read: Subset = serde_json::from_value(serde_json::json!({
            "sets": ["A"], "size": "-Infinity", "label": ""
        }))
        .expect("reads");
        assert_eq!(read.size, f64::NEG_INFINITY);
    }

    #[test]
    fn the_fields_are_camel_case() {
        let json = serde_json::to_value(VennDiagram::new()).expect("serialises");
        assert_eq!(json["version"], serde_json::json!(CANONICAL_VERSION));
        assert!(json.get("textNodes").is_some());
        assert!(json.get("styles").is_some());
    }
}
