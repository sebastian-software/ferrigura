//! Canonical model of a packet diagram: the DB state after the parse, as `docs-port/packet/NOTES.md`
//! section 4 proposes. The conventions are in `docs-port/canonical-model.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed packet diagram: its header texts and its rows of blocks, top to bottom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title: the grammar's `title` when it is non-empty, else the front matter's; empty if
    /// neither is given. Sanitised, as the DB holds it.
    pub title: String,
    /// Accessible title from `accTitle`, sanitised and without leading blanks, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, sanitised, empty if absent.
    pub acc_descr: String,
    /// The rows of the diagram, each a list of blocks from the left. A row is never empty. The
    /// rows depend on `bitsPerRow` of the effective configuration.
    pub rows: Vec<Vec<PacketBlock>>,
}

/// A block of a row: an inclusive range of absolute bit numbers and its label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketBlock {
    /// The first bit of the block, absolute.
    #[serde(with = "number")]
    pub start: f64,
    /// The last bit of the block, absolute and inclusive.
    #[serde(with = "number")]
    pub end: f64,
    /// The DB's `bits`. A piece of a block split at a row boundary carries one less than its
    /// width, as `parser.ts` gives it (`docs-port/packet/PLAN.md`, quirk 1).
    #[serde(with = "number")]
    pub bits: f64,
    /// The label as the DB holds it: the string of the grammar, with entities as placeholders.
    pub label: String,
}

impl PacketDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            rows: Vec::new(),
        }
    }
}

impl Default for PacketDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A bit number as JSON: a number when it is finite, and the sentinel strings for the other values
/// (`docs-port/canonical-model.md`).
mod number {
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

    fn block(start: f64, end: f64, bits: f64) -> PacketBlock {
        PacketBlock {
            start,
            end,
            bits,
            label: "x".to_owned(),
        }
    }

    #[test]
    fn non_finite_bits_are_sentinel_strings() {
        let value = serde_json::to_value(block(0.0, f64::INFINITY, f64::NEG_INFINITY)).unwrap();
        assert_eq!(value["end"], serde_json::json!("Infinity"));
        assert_eq!(value["bits"], serde_json::json!("-Infinity"));
        let back: PacketBlock = serde_json::from_value(value).unwrap();
        assert_eq!(back.end, f64::INFINITY);
    }

    #[test]
    fn rows_use_camel_case_keys() {
        let mut diagram = PacketDiagram::new();
        diagram.acc_title = "t".to_owned();
        diagram.rows = vec![vec![block(0.0, 10.0, 11.0)]];
        let value = serde_json::to_value(&diagram).unwrap();
        assert_eq!(value["accTitle"], serde_json::json!("t"));
        assert_eq!(value["rows"][0][0]["bits"], serde_json::json!(11.0));
    }
}
