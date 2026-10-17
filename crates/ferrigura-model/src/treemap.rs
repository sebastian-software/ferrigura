//! Canonical model of a treemap diagram: the state of the treemap DB after the parse, with the
//! hierarchy that `buildHierarchy` gives. The layout, the value formatting and the configuration
//! are the renderer's, so they are not in the model. See `docs-port/treemap/PLAN.md`.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed treemap diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The title: the grammar's `title` when it is non-empty, else the front matter's, sanitised.
    pub title: String,
    /// The `accTitle`, sanitised and without leading blanks; `""` when none was given.
    pub acc_title: String,
    /// The `accDescr`, sanitised; `""` when none was given.
    pub acc_descr: String,
    /// The class definitions, in the order their names first appear.
    pub classes: Vec<TreemapClass>,
    /// The root of the hierarchy: its name is `""`, and its children are the top-level nodes.
    pub tree: TreemapNode,
}

/// A class of the DB: its style pieces, and the pieces that are label styles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapClass {
    /// The class name, as written after `classDef`.
    pub id: String,
    /// The style pieces, in order. A piece may be empty.
    pub styles: Vec<String>,
    /// The pieces that are label styles (`color`, `font-size`, ...), in order.
    pub text_styles: Vec<String>,
}

/// A node of the hierarchy. A section has `children` (empty when it has no rows); a leaf has no
/// `children` and has a `value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapNode {
    /// The name, as the string literal gives it (no escapes are resolved).
    pub name: String,
    /// The children of a section, in order. Absent for a leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreemapNode>>,
    /// The value of a leaf. A value that is not finite is written as its sentinel string.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "value::serialize",
        deserialize_with = "value::deserialize"
    )]
    pub value: Option<f64>,
    /// The class named after `:::`, when the row gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_selector: Option<String>,
    /// The styles of that class, when the DB defines it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css_compiled_styles: Option<Vec<String>>,
}

impl TreemapDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            classes: Vec::new(),
            tree: TreemapNode::root(),
        }
    }
}

impl TreemapNode {
    /// The root of a hierarchy: no name and no children yet.
    #[must_use]
    pub fn root() -> Self {
        Self {
            name: String::new(),
            children: Some(Vec::new()),
            value: None,
            class_selector: None,
            css_compiled_styles: None,
        }
    }
}

impl Default for TreemapDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A value as JSON: the number when it is finite, else the strings `"NaN"`, `"Infinity"` and
/// `"-Infinity"`, see `value.rs`.
mod value {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        value: &Option<f64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(number) if number.is_finite() => serializer.serialize_f64(*number),
            Some(number) if number.is_nan() => serializer.serialize_str("NaN"),
            Some(number) if *number > 0.0 => serializer.serialize_str("Infinity"),
            Some(_) => serializer.serialize_str("-Infinity"),
            None => serializer.serialize_none(),
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Number(f64),
        Sentinel(String),
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<f64>, D::Error> {
        match Option::<Written>::deserialize(deserializer)? {
            None => Ok(None),
            Some(Written::Number(number)) => Ok(Some(number)),
            Some(Written::Sentinel(text)) => match text.as_str() {
                "NaN" => Ok(Some(f64::NAN)),
                "Infinity" => Ok(Some(f64::INFINITY)),
                "-Infinity" => Ok(Some(f64::NEG_INFINITY)),
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

    #[test]
    fn writes_a_non_finite_value_as_its_sentinel() {
        let leaf = TreemapNode {
            value: Some(f64::NAN),
            ..TreemapNode::root()
        };
        let json = serde_json::to_value(&leaf).expect("serialises");
        assert_eq!(json["value"], serde_json::json!("NaN"));
        let leaf = TreemapNode {
            value: Some(f64::NEG_INFINITY),
            ..TreemapNode::root()
        };
        let json = serde_json::to_value(&leaf).expect("serialises");
        assert_eq!(json["value"], serde_json::json!("-Infinity"));
    }

    #[test]
    fn leaves_out_absent_children_and_values() {
        let leaf = TreemapNode {
            name: "A".to_owned(),
            children: None,
            value: Some(1.0),
            ..TreemapNode::root()
        };
        let json = serde_json::to_value(&leaf).expect("serialises");
        assert!(json.get("children").is_none());
        assert_eq!(json["value"], serde_json::json!(1.0));
    }
}
