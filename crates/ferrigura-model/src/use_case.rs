//! Canonical model of a use case diagram: the state of the use case DB after a successful parse,
//! and the graph AST the DB keeps with it (`usecaseAst.ts`).
//!
//! The model follows the DB of `usecaseDb.ts` with the names in camelCase. The arrays are the DB's
//! maps in their declaration order (`reorderElements`, `usecaseModelBuilder.ts` 599-618). The
//! texts are the DB's values: labels are raw (the sanitising of the labels happens in `getData`,
//! the renderer's input, and is not in the model), the header texts are sanitised by the common
//! DB's setters. The entity placeholders (`ﬂ°…¶ß`) stay as the DB keeps them. Left out: the
//! symbols and the counters (the lengths of the lists), the configuration, and the render input.
//! See `docs-port/usecase/PLAN.md` and `docs-port/canonical-model.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, Serializer};

use crate::CANONICAL_VERSION;
use crate::value::Value;

/// A parsed use case diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UseCaseDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The title of the DB. Always `""`: `Diagram.fromText` sets the front matter title, then the
    /// parser clears the DB, which resets it (`commonDb.clear`).
    pub title: String,
    /// The direction of the layout. `TD` is stored as `TB`; the last direction statement wins.
    pub direction: Direction,
    /// The accessibility title, sanitised and trimmed as the DB does.
    pub acc_title: String,
    /// The accessibility description, sanitised as the DB does.
    pub acc_description: String,
    /// The actors, in declaration order.
    pub actors: Vec<Actor>,
    /// The use cases, in declaration order.
    pub use_cases: Vec<UseCase>,
    /// The system boundaries, in declaration order.
    pub system_boundaries: Vec<SystemBoundary>,
    /// The relationships, in the order the DB stores them.
    pub relationships: Vec<Relationship>,
    /// The notes, numbered `note-N` in declaration order.
    pub notes: Vec<Note>,
    /// The JSON tables, in declaration order.
    pub json_nodes: Vec<JsonNode>,
    /// The class definitions, last write wins per id.
    pub class_defs: Vec<ClassDef>,
    /// The graph AST of the parse (`usecaseAst.ts`).
    pub ast: GraphAst,
}

impl UseCaseDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            direction: Direction::Lr,
            acc_title: String::new(),
            acc_description: String::new(),
            actors: Vec::new(),
            use_cases: Vec::new(),
            system_boundaries: Vec::new(),
            relationships: Vec::new(),
            notes: Vec::new(),
            json_nodes: Vec::new(),
            class_defs: Vec::new(),
            ast: GraphAst::empty(),
        }
    }
}

impl Default for UseCaseDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// The layout direction: `TB`, `BT`, `RL` or `LR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Top to bottom (`TD` and `TB` in the text).
    #[serde(rename = "TB")]
    Tb,
    /// Bottom to top.
    #[serde(rename = "BT")]
    Bt,
    /// Right to left.
    #[serde(rename = "RL")]
    Rl,
    /// Left to right, the default.
    #[serde(rename = "LR")]
    Lr,
}

/// Whether a label is plain text or Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelType {
    /// Unquoted or double-quoted text.
    Text,
    /// A Markdown string, written between a quote and a backquote.
    Markdown,
}

/// The glyph of an actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    /// The stick figure (`normal`).
    Normal,
    /// The hollow glyph.
    Hollow,
    /// The awesome glyph.
    Awesome,
    /// An icon, when the actor has an `icon` metadata.
    Icon,
}

/// The shape of a use case: an ellipse, or a rectangle for `[Label]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UseCaseShape {
    /// The ellipse, the default.
    Ellipse,
    /// A rectangle, from the bracket form `[Label]` of an entity.
    Rect,
}

/// The geometry of a system boundary: a rectangle, or a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoundaryType {
    /// A plain rectangle, the default.
    Rect,
    /// A package, from the `type: package` metadata.
    Package,
}

/// The kind of a relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipType {
    /// A plain association (one of the arrow forms).
    Association,
    /// `..>:include`.
    Include,
    /// `..>:extend`.
    Extend,
    /// `--|>`.
    Generalization,
}

/// The animation of an edge, from its metadata: `animation: "fast"` or `"slow"`. `animate: true`
/// alone sets `animate` and leaves this absent (the renderer reads the absent value as `fast`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Animation {
    /// `animation: "fast"`.
    Fast,
    /// `animation: "slow"`.
    Slow,
}

/// An actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    /// The id, as written or generated from a quoted label.
    pub id: String,
    /// The label, raw.
    pub label: String,
    /// The label type.
    pub label_type: LabelType,
    /// The glyph.
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    /// The icon of the `icon` metadata, when it is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Whether the `business` metadata is set.
    pub business: bool,
    /// The stereotype, raw. Absent when the actor has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
    /// The id of the boundary that contains the actor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The classes, in the order they were assigned.
    pub classes: Vec<String>,
    /// The styles, in the order they were assigned.
    pub styles: Vec<String>,
}

/// A use case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UseCase {
    /// The id.
    pub id: String,
    /// The label, raw.
    pub label: String,
    /// The label type.
    pub label_type: LabelType,
    /// The shape.
    pub shape: UseCaseShape,
    /// Whether the `business` metadata is set.
    pub business: bool,
    /// The stereotype, raw. Absent when the use case has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
    /// The id of the boundary that contains the use case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The classes.
    pub classes: Vec<String>,
    /// The styles.
    pub styles: Vec<String>,
}

/// A system boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemBoundary {
    /// The id.
    pub id: String,
    /// The label, raw.
    pub label: String,
    /// The label type.
    pub label_type: LabelType,
    /// The geometry.
    #[serde(rename = "type")]
    pub boundary_type: BoundaryType,
    /// The ids of the members, in declaration order (`refreshBoundaries`).
    pub members: Vec<String>,
    /// The classes.
    pub classes: Vec<String>,
    /// The styles.
    pub styles: Vec<String>,
}

/// A relationship. Its arrow type is the DB's number (`ARROW_TYPE`): 0 solid, 1 back, 2 line,
/// 3 circle, 4 cross, 5 reversed circle, 6 reversed cross.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    /// The id: the explicit `id@`, or `edge-N` in source order for an anonymous edge.
    pub id: String,
    /// Whether the id was written by the user.
    pub explicit_id: bool,
    /// The id of the source element.
    pub source: String,
    /// The id of the target element.
    pub target: String,
    /// The kind.
    #[serde(rename = "type")]
    pub relationship_type: RelationshipType,
    /// The arrow type, 0 to 6.
    pub arrow_type: u8,
    /// The label, raw. Absent when the edge has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The label type. Absent when the edge has no label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_type: Option<LabelType>,
    /// The minimum length of the edge, the number of dashes minus one.
    pub minlen: u32,
    /// The classes.
    pub classes: Vec<String>,
    /// The styles.
    pub styles: Vec<String>,
    /// Whether the edge is animated.
    pub animate: bool,
    /// The animation, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<Animation>,
}

/// A note on an actor or a use case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// The id, `note-N`.
    pub id: String,
    /// The id of the actor or use case the note is for.
    pub target: String,
    /// The label, raw.
    pub label: String,
    /// The label type.
    pub label_type: LabelType,
}

/// A JSON table: the object of a `json <id> @{ … }` statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonNode {
    /// The id.
    pub id: String,
    /// The object, as `JSON.parse` reads it: non-finite numbers are the sentinel strings.
    #[serde(serialize_with = "sentinel_value")]
    pub value: Value,
    /// For each object of the value, keyed by its JSON pointer (`""` is the root), the keys in
    /// the order of the text. A repeated key keeps its first position.
    pub property_order: BTreeMap<String, Vec<String>>,
    /// The classes.
    pub classes: Vec<String>,
    /// The styles.
    pub styles: Vec<String>,
    /// The display rows of the table (`flattenJsonRows`, `usecaseDb.ts` 149-211), sanitised cell
    /// by cell as `getData` gives them. This is the only place the JavaScript formatting of the
    /// numbers and the key order of the rows show.
    pub rows: Vec<JsonRow>,
}

/// Writes a value with the sentinel strings of its non-finite numbers, as the canonical JSON does.
fn sentinel_value<S: Serializer>(value: &Value, serializer: S) -> Result<S::Ok, S::Error> {
    sentinel_json(value).serialize(serializer)
}

/// The JSON of a value, with each non-finite number as its sentinel string.
fn sentinel_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(flag) => serde_json::Value::Bool(*flag),
        Value::Number(number) => serde_json::Number::from_f64(*number).map_or_else(
            || {
                let text = if number.is_nan() {
                    "NaN"
                } else if *number > 0.0 {
                    "Infinity"
                } else {
                    "-Infinity"
                };
                serde_json::Value::String(text.to_owned())
            },
            serde_json::Value::Number,
        ),
        Value::String(text) => serde_json::Value::String(text.clone()),
        Value::Array(items) => serde_json::Value::Array(items.iter().map(sentinel_json).collect()),
        Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), sentinel_json(item)))
                .collect(),
        ),
    }
}

/// One row of a JSON table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRow {
    /// The visual key cell. A scalar array's rows after the first have an empty key.
    pub key: String,
    /// The key for assistive technology.
    pub accessible_key: String,
    /// The value cell, as `String()` writes a scalar.
    pub value: String,
}

/// A class definition: `classDef <id> <styles>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDef {
    /// The class name.
    pub id: String,
    /// The styles.
    pub styles: Vec<String>,
}

/// A span of the prepared text: the offsets of the first and the end character, in UTF-16 code
/// units as JavaScript counts them (the reference's `[start, end]`).
pub type Span = [usize; 2];

/// The graph AST of the DB (`GraphAST` in `usecaseTypes.ts`, built by `usecaseAst.ts`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphAst {
    /// Always 1.
    pub version: u32,
    /// Always `usecase`.
    pub diagram_type: String,
    /// The prepared text the AST was read from.
    pub source: String,
    /// The header of the diagram.
    pub header: AstHeader,
    /// The accessibility title, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    /// The accessibility description, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    /// The nodes by id.
    pub nodes: BTreeMap<String, GraphNode>,
    /// The edges, in order.
    pub edges: Vec<GraphEdge>,
    /// The groups (boundaries) by id.
    pub groups: BTreeMap<String, GraphGroup>,
    /// The class definitions by name.
    pub class_defs: BTreeMap<String, AstClassDef>,
    /// The statements, one per line of the text.
    pub statements: Vec<GraphStatement>,
}

impl GraphAst {
    /// The AST of an empty diagram.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            version: 1,
            diagram_type: "usecase".to_owned(),
            source: String::new(),
            header: AstHeader {
                keyword: "usecase".to_owned(),
                direction: Direction::Lr,
                span: [0, 0],
            },
            acc_title: None,
            acc_descr: None,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            groups: BTreeMap::new(),
            class_defs: BTreeMap::new(),
            statements: Vec::new(),
        }
    }
}

/// The header of the AST: the keyword, the direction (never `TD`) and the span of the keyword.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AstHeader {
    /// Always `usecase`.
    pub keyword: String,
    /// The direction.
    pub direction: Direction,
    /// The span of the header keyword.
    pub span: Span,
}

/// A node of the AST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// The label, when the node has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The shape name, when the node has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// The classes, when the node has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The styles, when the node has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    /// The attributes the AST adds (`usecaseAst.ts`), when there are any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<BTreeMap<String, Value>>,
}

/// An edge of the AST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// The id of the edge.
    pub id: String,
    /// The id of the source.
    pub source: String,
    /// The id of the target.
    pub target: String,
    /// The label, when the edge has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The classes, when the edge has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The styles, when the edge has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    /// The attributes the AST adds, when there are any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<BTreeMap<String, Value>>,
}

/// A group of the AST: a system boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphGroup {
    /// The title, when the group has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The parent group, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The members, in order.
    pub nodes: Vec<String>,
    /// The direction, when the group has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// The classes, when the group has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The styles, when the group has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    /// The attributes the AST adds, when there are any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<BTreeMap<String, Value>>,
}

/// A class definition of the AST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AstClassDef {
    /// The styles.
    pub styles: Vec<String>,
}

/// A metadata property of a statement: its key and the spans of the key and the value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataOccurrence {
    /// The key.
    pub key: String,
    /// The span of the property.
    pub span: Span,
    /// The span of the key.
    pub key_span: Span,
    /// The span of the value.
    pub value_span: Span,
}

/// A node occurrence in a statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeOccurrence {
    /// The id.
    pub id: String,
    /// The span of the occurrence.
    pub span: Span,
    /// The span of the id.
    pub id_span: Span,
    /// The span of the label, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_span: Option<Span>,
    /// Whether the occurrence declares the node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defines: Option<bool>,
    /// The span of the stereotype, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype_span: Option<Span>,
    /// The metadata, when there is any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<MetadataOccurrence>>,
    /// The spans of the classes, when there are any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_spans: Option<Vec<Span>>,
}

/// An edge occurrence in a statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeOccurrence {
    /// The id of the edge.
    pub id: String,
    /// The span of the occurrence.
    pub span: Span,
    /// The span of the label, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_span: Option<Span>,
    /// The span of the explicit id, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_span: Option<Span>,
    /// The metadata, when there is any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<MetadataOccurrence>>,
    /// The spans of the classes, when there are any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_spans: Option<Vec<Span>>,
}

/// The kind of a statement of the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatementKind {
    /// A node statement (actors, use cases, boundary members).
    Node,
    /// An edge statement.
    Edge,
    /// A system boundary.
    Group,
    /// A note.
    Note,
    /// A JSON table.
    Json,
    /// A metadata assignment to a node.
    Metadata,
    /// A metadata assignment to an edge.
    EdgeMetadata,
    /// A class definition.
    ClassDef,
    /// A class assignment.
    ClassAssign,
    /// A style assignment.
    Style,
    /// The direction statement.
    Direction,
    /// The accessibility title.
    AccTitle,
    /// The accessibility description.
    AccDescr,
    /// A comment line.
    Comment,
    /// A blank line.
    Blank,
}

/// A statement of the AST. The fields that a statement kind does not use are absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStatement {
    /// The kind.
    pub kind: StatementKind,
    /// The span of the statement.
    pub span: Span,
    /// The node occurrences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<NodeOccurrence>>,
    /// The edge occurrences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<EdgeOccurrence>>,
    /// The group (boundary) id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The span of the id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_span: Option<Span>,
    /// The span of the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_span: Option<Span>,
    /// The span of the closing `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_span: Option<Span>,
    /// The id a statement refers to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    /// The span of the reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_span: Option<Span>,
    /// The statements in a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<GraphStatement>>,
    /// The metadata, when there is any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<MetadataOccurrence>>,
    /// The span of the stereotype, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype_span: Option<Span>,
    /// The spans of the classes, when there are any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_spans: Option<Vec<Span>>,
}
