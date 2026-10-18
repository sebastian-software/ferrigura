//! Canonical model of an agentflow diagram: the state of the reference's `AgentFlowDB` after the
//! parse (`docs-port/agentflow/NOTES.md`, section 4.1). It is what the DB holds, not the layout
//! data of `getData`, which the renderer's model will carry.
//!
//! Optional fields are left out of the JSON when the reference's value is `undefined`, as
//! `JSON.stringify` leaves them out. The DOM ids and the vertex counter are not in the model: the
//! reference derives them from a counter.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, Serializer};

use crate::CANONICAL_VERSION;
use crate::value::Value;

/// A parsed agentflow diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentflowDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title from the front matter (`setDiagramTitle`), sanitised.
    pub title: String,
    /// The accessibility title (`setAccTitle`): sanitised, with the leading blanks removed.
    pub acc_title: String,
    /// The accessibility description (`setAccDescription`).
    pub acc_descr: String,
    /// The direction of the header, as the DB keeps it (`BR` is kept as written).
    pub direction: String,
    /// The vertices, in the order the DB created them. Connectors are vertices too.
    pub vertices: Vec<Vertex>,
    /// The edges, in creation order.
    pub edges: Vec<Edge>,
    /// The containers (`flow` blocks), in completion order: a repeated id merges into its first.
    pub sub_graphs: Vec<SubGraph>,
    /// The connector records, in creation order. Each one is also in `vertices`.
    pub connectors: Vec<Vertex>,
    /// The class definitions, in creation order.
    pub classes: Vec<ClassDef>,
    /// The tooltips, by id.
    pub tooltips: BTreeMap<String, String>,
}

impl AgentflowDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: "TB".to_owned(),
            vertices: Vec::new(),
            edges: Vec::new(),
            sub_graphs: Vec::new(),
            connectors: Vec::new(),
            classes: Vec::new(),
            tooltips: BTreeMap::new(),
        }
    }
}

impl Default for AgentflowDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A node of the diagram: a bracket or bare id, or a connector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    /// The id as written.
    pub id: String,
    /// The label, sanitised. A vertex without a label keeps its id.
    pub text: String,
    /// `text`, `string` or `markdown`, as the label was written.
    pub label_type: String,
    /// The shape: the authored name after alias resolution, or the bracket's shape. Absent for a
    /// bare id.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// The style strings of `style` statements.
    pub styles: Vec<String>,
    /// The classes given with `:::` and `class`, and `clickable` after a click.
    pub classes: Vec<String>,
    /// The props of the `[| field: value | text]` form. Absent for a connector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<BTreeMap<String, String>>,
    /// The metadata of the `@{ … }` block, with the prototype keys removed.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_metadata"
    )]
    pub metadata: Option<BTreeMap<String, Value>>,
    /// The link of a click, sanitised. Absent when the URL is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// The target of a click link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
    /// `true` for a connector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_connector: Option<bool>,
}

/// An edge: an arrow between two ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// The edge id: the author's, or `L_<start>_<end>_<n>`.
    pub id: String,
    /// Whether the author gave the id.
    pub is_user_defined_id: bool,
    /// The start id.
    pub start: String,
    /// The end id.
    pub end: String,
    /// The arrow: `arrow_point`, `arrow_cross` or `arrow_open`.
    #[serde(rename = "type")]
    pub arrow: String,
    /// `normal` or `dotted`.
    pub stroke: String,
    /// The edge length, at most 10: the number of dashes less one, or the number of dots.
    pub length: u32,
    /// The label, sanitised. Empty when there is none.
    pub text: String,
    /// `text`, `string` or `markdown`.
    pub label_type: String,
    /// The classes of the edge.
    pub classes: Vec<String>,
    /// The style strings of a `linkStyle` statement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Vec<String>>,
    /// The interpolation of a `linkStyle` statement, or of the metadata's `curve`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_value"
    )]
    pub interpolate: Option<Value>,
    /// The metadata's `animate`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_value"
    )]
    pub animate: Option<Value>,
    /// The metadata's `animation`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_value"
    )]
    pub animation: Option<Value>,
    /// The meaning of the arrow: `sequence`, `reference` or `failure`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_semantic: Option<String>,
    /// The metadata of the edge's `@{ … }` block.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_metadata"
    )]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A container: a `flow` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubGraph {
    /// The id: the author's, or `subGraph<n>`.
    pub id: String,
    /// The members, in the order the body gave them.
    pub nodes: Vec<String>,
    /// The title, sanitised and trimmed.
    pub title: String,
    /// `text`, `string`, `markdown`, or `markdown` when there is no title.
    pub label_type: String,
    /// The container's own direction: the body's `direction` statement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// The container's type: `flow`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The classes of the container.
    pub classes: Vec<String>,
    /// The metadata of a `@{ … }` block on the container id.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_metadata"
    )]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A class definition: `classDef`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDef {
    /// The class name.
    pub id: String,
    /// The style strings.
    pub styles: Vec<String>,
    /// The styles that set a colour, with `fill` renamed `bgFill`.
    pub text_styles: Vec<String>,
}

/// A metadata value as the differential writes it: a number that is not finite is its sentinel
/// string, as `JSON.stringify` would write `null` for it (`docs-port/canonical-model.md`).
struct Sentinel<'a>(&'a Value);

impl Serialize for Sentinel<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Number(number) if !number.is_finite() => {
                let text = if number.is_nan() {
                    "NaN"
                } else if *number > 0.0 {
                    "Infinity"
                } else {
                    "-Infinity"
                };
                serializer.serialize_str(text)
            }
            Value::Array(items) => serializer.collect_seq(items.iter().map(Sentinel)),
            Value::Object(map) => {
                serializer.collect_map(map.iter().map(|(key, value)| (key, Sentinel(value))))
            }
            other => other.serialize(serializer),
        }
    }
}

/// The `metadata` of a record: its entries, with the sentinels.
fn serialize_metadata<S: Serializer>(
    metadata: &Option<BTreeMap<String, Value>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match metadata {
        Some(map) => serializer.collect_map(map.iter().map(|(key, value)| (key, Sentinel(value)))),
        None => serializer.serialize_none(),
    }
}

/// A value of a record that may be absent, with the sentinels.
fn serialize_optional_value<S: Serializer>(
    value: &Option<Value>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => Sentinel(value).serialize(serializer),
        None => serializer.serialize_none(),
    }
}
