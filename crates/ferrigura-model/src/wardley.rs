//! Canonical model of a wardley map: the state of the wardley DB after the parse
//! (`wardleyDb.ts`, `getWardleyData`). The field list and the rules are in
//! `docs-port/wardley/PLAN.md`, section 4, and the conventions in `docs-port/canonical-model.md`.
//!
//! The model keeps the DB's `x` (the evolution, a percentage) and `y` (the visibility, a
//! percentage) as the DB holds them, not the order of the statements.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::CANONICAL_VERSION;

/// A parsed wardley map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title: the grammar's last non-empty `title`, else the front matter's.
    pub title: String,
    /// The sanitised `accTitle`, with its leading whitespace removed. `""` when absent.
    pub acc_title: String,
    /// The sanitised `accDescr`, with the whitespace after each line break removed. `""` when
    /// absent.
    pub acc_descr: String,
    /// The `size` statement, `null` when the text gives none.
    pub size: Option<Size>,
    /// The evolution axis, `null` fields when the text gives no `evolution`.
    pub axes: Axes,
    /// The nodes, in the DB's order: the first insertion of each id.
    pub nodes: Vec<Node>,
    /// The links, in text order.
    pub links: Vec<Link>,
    /// The trends (`evolve`), one per node, in the order the node first got one.
    pub trends: Vec<Trend>,
    /// The pipelines, one per parent, in the order the parent first got one.
    pub pipelines: Vec<Pipeline>,
    /// The `annotation` statements, in text order.
    pub annotations: Vec<Annotation>,
    /// The notes, in text order.
    pub notes: Vec<Note>,
    /// The accelerators, in text order.
    pub accelerators: Vec<Mark>,
    /// The deaccelerators, in text order.
    pub deaccelerators: Vec<Mark>,
    /// The position of the annotations box, from the first `annotations` statement.
    pub annotations_box: Option<Point>,
}

impl WardleyDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            size: None,
            axes: Axes::default(),
            nodes: Vec::new(),
            links: Vec::new(),
            trends: Vec::new(),
            pipelines: Vec::new(),
            annotations: Vec::new(),
            notes: Vec::new(),
            accelerators: Vec::new(),
            deaccelerators: Vec::new(),
            annotations_box: None,
        }
    }
}

impl Default for WardleyDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// The `size` statement: the width and the height as the text gives them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// The width.
    pub width: Number,
    /// The height.
    pub height: Number,
}

/// The evolution axis. Both fields are `null` until an `evolution` statement sets them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Axes {
    /// The stage names, `Name` or `Name / Second`, trimmed.
    pub stages: Option<Vec<String>>,
    /// The `@` boundaries that the stages give, in order, as the decimals the text gives.
    pub stage_boundaries: Option<Vec<Number>>,
}

/// The class of a node, as the DB names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeClass {
    /// An anchor.
    #[serde(rename = "anchor")]
    Anchor,
    /// A component.
    #[serde(rename = "component")]
    Component,
    /// A component of a pipeline.
    #[serde(rename = "pipeline-component")]
    PipelineComponent,
}

/// The sourcing strategy of a component: its decorator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// `build`.
    Build,
    /// `buy`.
    Buy,
    /// `outsource`.
    Outsource,
    /// `market`.
    Market,
}

/// The direction of a link's flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flow {
    /// `+>` or the arrow's `>`.
    Forward,
    /// `+<` or the arrow's `<`.
    Backward,
    /// `+<>`, or an arrow with `<>`.
    Bidirectional,
}

/// A node of the map. The DB adds anchors, components and pipeline components; the
/// optional fields are `null` when the DB holds no value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// The id: the name, or `Parent_Name` for a pipeline component.
    pub id: String,
    /// The label: the name as written.
    pub label: String,
    /// The evolution, as a percentage of the plot.
    pub x: f64,
    /// The visibility, as a percentage of the plot.
    pub y: f64,
    /// The class of the node.
    pub class_name: Option<NodeClass>,
    /// The horizontal label offset, signed.
    pub label_offset_x: Option<Number>,
    /// The vertical label offset, signed.
    pub label_offset_y: Option<Number>,
    /// Whether the component carries the `inertia` mark. `null` for anchors and pipeline
    /// components, whose DB calls give no inertia.
    pub inertia: Option<bool>,
    /// The decorator of the component.
    pub source_strategy: Option<Strategy>,
    /// Set when the node is a component of a pipeline.
    pub in_pipeline: Option<bool>,
    /// Set when the node is the parent of a pipeline.
    pub is_pipeline_parent: Option<bool>,
}

/// A link between two names. The ends are the ids the DB resolves them to, or the names as
/// written when no node has them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// The source end.
    pub source: String,
    /// The target end.
    pub target: String,
    /// Whether the arrow is `-.->`.
    pub dashed: bool,
    /// The label: the flow label of the arrow, else the `;` label.
    pub label: Option<String>,
    /// The flow of the link.
    pub flow: Option<Flow>,
}

/// An `evolve` statement: the target of a node's evolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    /// The id of the node.
    pub node_id: String,
    /// The target evolution, as a percentage.
    pub target_x: f64,
    /// The visibility of the node, as a percentage.
    pub target_y: f64,
}

/// A pipeline: its parent and the ids of its components, in text order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    /// The id of the parent.
    pub node_id: String,
    /// The ids of the components of the last pipeline block of this parent.
    pub component_ids: Vec<String>,
}

/// An `annotation` statement: its number, its position and its text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    /// The number of the annotation.
    pub number: Number,
    /// The position, as one point.
    pub coordinates: Vec<Point>,
    /// The text, `""` when it is empty.
    pub text: String,
}

/// A note: its text and its position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// The text, with the entity placeholders kept.
    pub text: String,
    /// The evolution, as a percentage.
    pub x: f64,
    /// The visibility, as a percentage.
    pub y: f64,
}

/// An accelerator or a deaccelerator: its name and its position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    /// The name, as written.
    pub name: String,
    /// The evolution, as a percentage.
    pub x: f64,
    /// The visibility, as a percentage.
    pub y: f64,
}

/// A position on the plot, as percentages.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// The evolution.
    pub x: f64,
    /// The visibility.
    pub y: f64,
}

/// A number of the model that JSON cannot always write. A finite number is a JSON number. A
/// non-finite one (only an integer of more than 308 digits, read as `Infinity`) is the string
/// `"NaN"`, `"Infinity"` or `"-Infinity"`, as `canonical-model.md` ("Numbers") says.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number(pub f64);

impl Serialize for Number {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = self.0;
        if value.is_finite() {
            serializer.serialize_f64(value)
        } else if value.is_nan() {
            serializer.serialize_str("NaN")
        } else if value > 0.0 {
            serializer.serialize_str("Infinity")
        } else {
            serializer.serialize_str("-Infinity")
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Number(f64),
            Text(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Number(value) => Ok(Self(value)),
            Raw::Text(text) => match text.as_str() {
                "NaN" => Ok(Self(f64::NAN)),
                "Infinity" => Ok(Self(f64::INFINITY)),
                "-Infinity" => Ok(Self(f64::NEG_INFINITY)),
                other => Err(de::Error::custom(format_args!(
                    "expected a number or NaN, Infinity or -Infinity, found {other:?}"
                ))),
            },
        }
    }
}
