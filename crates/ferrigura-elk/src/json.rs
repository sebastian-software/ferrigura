//! The elkjs JSON graph format, read and written with serde.
//!
//! Shapes and rules follow elkjs 0.9.3: ELK's `JsonImporter` (plugin
//! `org.eclipse.elk.graph.json`) together with the elkjs glue in
//! `JsonAdapter.xtend`. Unknown fields are ignored, so graphs that carry
//! application fields (as Mermaid's do) still load.
//!
//! Option values are kept as JSON values until they are read, because elkjs
//! turns each one into a string first (`stringVal`) and then parses it for the
//! option's type. [`string_val`] reproduces that conversion, including the
//! JavaScript number format, so `40` becomes `"40"` and an integer option parses.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `layoutOptions` (or the older `properties`): option key to JSON value.
///
/// Keys keep their input order. Mermaid's graphs are read in document order
/// by elkjs, so an ordered map is the faithful choice.
pub type LayoutOptions = indexmap::IndexMap<String, Value>;

/// An error while reading a JSON graph, mirroring `JsonImportException`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    message: String,
}

impl JsonError {
    fn new(message: impl Into<String>) -> Self {
        JsonError {
            message: message.into(),
        }
    }

    /// The message elkjs would report.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsonError {}

/// A node (the root or any child). Coordinates and sizes are optional on input.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct JsonNode {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_options: Option<LayoutOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<LayoutOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub individual_spacings: Option<LayoutOptions>,
    pub ports: Vec<JsonPort>,
    pub labels: Vec<JsonLabel>,
    pub children: Vec<JsonNode>,
    pub edges: Vec<JsonEdge>,
}

/// A port of a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct JsonPort {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_options: Option<LayoutOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<LayoutOptions>,
    pub labels: Vec<JsonLabel>,
}

/// A label of a node, port, edge or label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct JsonLabel {
    #[serde(deserialize_with = "deserialize_optional_id")]
    pub id: Option<String>,
    pub text: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_options: Option<LayoutOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<LayoutOptions>,
}

/// An edge. Either `sources`/`targets` (ids of nodes or ports) or the
/// primitive `source`/`target` pair is set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct JsonEdge {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    pub sources: Vec<String>,
    pub targets: Vec<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub source_port: Option<String>,
    pub target_port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_options: Option<LayoutOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<LayoutOptions>,
    pub labels: Vec<JsonLabel>,
    pub sections: Vec<JsonEdgeSection>,
}

/// One section of an edge's route.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct JsonEdgeSection {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    pub start_point: Option<JsonPoint>,
    pub end_point: Option<JsonPoint>,
    pub bend_points: Vec<JsonPoint>,
    pub incoming_shape: Option<String>,
    pub outgoing_shape: Option<String>,
    pub incoming_sections: Vec<String>,
    pub outgoing_sections: Vec<String>,
}

/// A point with coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct JsonPoint {
    pub x: f64,
    pub y: f64,
}

/// `id` may be a string or an integer in elkjs; both are stored as text.
fn deserialize_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    id_text(&value).map_err(serde::de::Error::custom)
}

fn deserialize_optional_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => id_text(&value).map(Some).map_err(serde::de::Error::custom),
    }
}

fn id_text(value: &Value) -> Result<String, JsonError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => match number.as_f64() {
            Some(float) if float.fract() == 0.0 => Ok(js_number_string(float)),
            _ => Err(JsonError::new(format!(
                "Id must be a string or an integer: '{value}'."
            ))),
        },
        _ => Err(JsonError::new(format!(
            "Id must be a string or an integer: '{value}'."
        ))),
    }
}

/// The string elkjs gets from a JSON value: `stringVal` in `JsonAdapter.xtend`.
///
/// Strings are taken as they are, numbers go through the JavaScript number
/// format (`40` becomes `"40"`, `0.5` stays `"0.5"`), and booleans become
/// `"true"` or `"false"`. Arrays, objects and `null` are rejected, as elkjs
/// rejects them with an internal error.
pub fn string_val(value: &Value) -> Result<String, JsonError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => number
            .as_f64()
            .map(js_number_string)
            .ok_or_else(|| JsonError::new("Unsupported number in a layout option.")),
        Value::Bool(flag) => Ok(flag.to_string()),
        other => Err(JsonError::new(format!(
            "Unsupported layout option value: {other}."
        ))),
    }
}

/// `String(x)` for a JavaScript number, for the cases that occur in graphs:
/// integral values print without a fraction, others in shortest round-trip form.
///
/// Values outside `1e-6..1e21` would use exponent notation in JavaScript and
/// are not produced by Mermaid's layouts; they are printed without exponent here.
pub fn js_number_string(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.fract() == 0.0 && value.abs() < 1e21 {
        return format!("{value:.0}");
    }
    format!("{value}")
}

/// Ordered key and string pairs of one `layoutOptions` object, as elkjs reads them.
pub type OptionPairs = Vec<(String, String)>;

/// Turns a `layoutOptions` (or `properties`) object into key and string pairs.
pub fn option_strings(options: &LayoutOptions) -> Result<OptionPairs, JsonError> {
    let mut pairs = Vec::with_capacity(options.len());
    for (key, value) in options {
        pairs.push((key.clone(), string_val(value)?));
    }
    Ok(pairs)
}

impl JsonNode {
    /// The option pairs of this node, from `layoutOptions`, falling back to `properties`.
    pub fn option_pairs(&self) -> Result<OptionPairs, JsonError> {
        option_pairs_of(self.layout_options.as_ref(), self.properties.as_ref())
    }
}

/// Shared rule of `transformProperties`: `layoutOptions`, else `properties`.
pub fn option_pairs_of(
    layout_options: Option<&LayoutOptions>,
    properties: Option<&LayoutOptions>,
) -> Result<OptionPairs, JsonError> {
    match layout_options.or(properties) {
        Some(options) => option_strings(options),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_are_stringified_like_javascript() {
        assert_eq!(string_val(&json!(40)).unwrap(), "40");
        assert_eq!(string_val(&json!(0.5)).unwrap(), "0.5");
        assert_eq!(string_val(&json!(-3)).unwrap(), "-3");
        assert_eq!(string_val(&json!(true)).unwrap(), "true");
        assert_eq!(string_val(&json!("[top=12]")).unwrap(), "[top=12]");
        assert!(string_val(&json!(null)).is_err());
        assert!(string_val(&json!([1])).is_err());
    }

    #[test]
    fn unknown_fields_are_ignored_and_ids_may_be_integers() {
        let node: JsonNode = serde_json::from_value(json!({
            "id": 7,
            "width": 50,
            "$H": 277,
            "shape": "rect",
            "children": [{ "id": "a", "x": 1.5 }]
        }))
        .unwrap();
        assert_eq!(node.id, "7");
        assert_eq!(node.width, Some(50.0));
        assert_eq!(node.children[0].x, Some(1.5));
    }

    #[test]
    fn option_pairs_prefer_layout_options_over_properties() {
        let node: JsonNode = serde_json::from_value(json!({
            "id": "root",
            "layoutOptions": { "spacing.baseValue": 40 },
            "properties": { "spacing.baseValue": 10 }
        }))
        .unwrap();
        assert_eq!(
            node.option_pairs().unwrap(),
            vec![("spacing.baseValue".to_string(), "40".to_string())]
        );
    }
}
