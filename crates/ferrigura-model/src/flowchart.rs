//! Canonical model of a flowchart diagram, as the flowchart DB of `flowDb.ts` holds it after
//! parsing, less what the input does not contain or only rendering needs (see
//! `docs-port/flowchart/PLAN.md`, sections 8 and 9).
//!
//! Field names are the DB's names in camelCase. A field the DB leaves `undefined` is `None`
//! (serialised as `null`); a list the DB starts empty is `[]`. Lists keep the DB's order: vertices
//! in creation order, edges in creation order, subgraphs in closing order.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CANONICAL_VERSION;

/// The kind of a label: plain text, a quoted string, or a Markdown string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelType {
    /// Unquoted text.
    #[default]
    Text,
    /// A double-quoted string.
    String,
    /// A Markdown string, written between backticks inside quotes.
    Markdown,
}

/// A parsed flowchart: header texts, direction, and the DB's vertices, edges, subgraphs, class
/// definitions and tooltips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowchartDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Title from the front matter; the flowchart grammar has no title statement.
    pub title: String,
    /// Accessible title from `accTitle`, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, empty if absent.
    pub acc_descr: String,
    /// Direction as the DB keeps it: `TB`, `BT`, `LR`, `RL`, or `BR` as written.
    pub direction: String,
    /// Vertices in the order the DB creates them.
    pub vertices: Vec<FlowVertex>,
    /// Edges in the order the DB creates them.
    pub edges: Vec<FlowEdge>,
    /// Subgraphs in the order they close, a repeated id merged into its first position.
    pub subgraphs: Vec<FlowSubgraph>,
    /// Class definitions in the order they are first named.
    pub classes: Vec<FlowClass>,
    /// Tooltips in the order they are first set, keyed by id; the id need not name a vertex.
    pub tooltips: Vec<FlowTooltip>,
    /// Styles of `linkStyle default`, `None` if none were given.
    pub default_style: Option<Vec<String>>,
    /// Interpolation of `linkStyle default interpolate`, `None` if none was given.
    pub default_interpolate: Option<String>,
}

/// A vertex (node) of the flowchart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowVertex {
    /// The id as written.
    pub id: String,
    /// The label text, the id when no label is given.
    pub text: String,
    /// Kind of the label.
    pub label_type: LabelType,
    /// Shape name as written in the source (`square`, `round`, `db`, …), `None` if none was given.
    pub shape: Option<String>,
    /// Styles from `style` statements, in order.
    pub styles: Vec<String>,
    /// Classes applied to the vertex, in order; `clickable` is added by interactions.
    pub classes: Vec<String>,
    /// Link from `click … href`, trimmed; `None` if none or if empty.
    pub link: Option<String>,
    /// Target of the link, `None` if none was given.
    pub link_target: Option<String>,
    /// Properties from `[|key:value|text]`, merged across statements.
    pub props: BTreeMap<String, String>,
    /// Icon name from the shape data.
    pub icon: Option<String>,
    /// Icon form from the shape data.
    pub form: Option<String>,
    /// Position from the shape data.
    pub pos: Option<String>,
    /// Image from the shape data.
    pub img: Option<String>,
    /// Constraint from the shape data.
    pub constraint: Option<String>,
    /// Asset width from the shape data (`w`).
    pub asset_width: Option<f64>,
    /// Asset height from the shape data (`h`).
    pub asset_height: Option<f64>,
}

/// An edge between two vertices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEdge {
    /// The id: the user's `e1@` id, or the generated `L_<start>_<end>_<n>`.
    pub id: String,
    /// Whether the id was given by the user.
    pub is_user_defined_id: bool,
    /// Id of the start vertex (or subgraph).
    pub start: String,
    /// Id of the end vertex (or subgraph).
    pub end: String,
    /// The DB's arrow type: `arrow_open`, `arrow_point`, `arrow_cross`, `arrow_circle`, the
    /// `double_` forms, or `INVALID` for a link whose two ends do not agree.
    #[serde(rename = "type")]
    pub edge_type: String,
    /// `normal`, `thick`, `dotted`, `invisible`, or `INVALID`.
    pub stroke: String,
    /// Length of the link, at most 10; `None` for an `INVALID` link.
    pub length: Option<u32>,
    /// The label text, empty if none.
    pub text: String,
    /// Kind of the label.
    pub label_type: LabelType,
    /// Classes applied to the edge.
    pub classes: Vec<String>,
    /// Styles from `linkStyle`, `None` if none were given.
    pub style: Option<Vec<String>>,
    /// Interpolation: the default at creation, or the one set later; `None` if none.
    pub interpolate: Option<String>,
    /// `animate` from the edge's shape data.
    pub animate: Option<bool>,
    /// `animation` from the edge's shape data.
    pub animation: Option<String>,
}

/// A subgraph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowSubgraph {
    /// The id, given or generated as `subGraph<n>`.
    pub id: String,
    /// The title, trimmed.
    pub title: String,
    /// Kind of the title.
    pub label_type: LabelType,
    /// Members in the DB's order: vertices and nested subgraph ids.
    pub nodes: Vec<String>,
    /// Direction given by a `direction` statement inside the subgraph, `None` if none.
    pub dir: Option<String>,
    /// Classes applied to the subgraph.
    pub classes: Vec<String>,
    /// Metadata from `@{ … }` on the subgraph id, with keys as written.
    pub metadata: BTreeMap<String, Value>,
}

/// A class definition from `classDef`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowClass {
    /// The class name.
    pub id: String,
    /// The styles, split as the DB splits them (not trimmed).
    pub styles: Vec<String>,
}

/// A tooltip set by `click`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowTooltip {
    /// The id the tooltip is set for.
    pub id: String,
    /// The tooltip text, trimmed as the DB stores it.
    pub tooltip: String,
}

impl FlowchartDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and direction `TB`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: String::from("TB"),
            vertices: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            classes: Vec::new(),
            tooltips: Vec::new(),
            default_style: None,
            default_interpolate: None,
        }
    }
}

impl Default for FlowchartDiagram {
    fn default() -> Self {
        Self::new()
    }
}
