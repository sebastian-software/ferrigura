//! The semantic reading of an agentflow diagram: the diagnostics that the layout data raises, and
//! the vertex kind, the metadata and the projection (`semantic_model`) that `getSemanticModel`
//! reports. The conformance fixtures assert these (`agentflow/conformance/runner.ts`), and the
//! diagnostics are raised only when the layout data is read, not by the parse
//! (`agentflowDb.ts:1908`, `normaliseNodeShapes`).
//!
//! The collapsed containers' hidden members and the layout's own nodes are read from the model:
//! a vertex that is a container, or hidden by a collapsed container, raises nothing.

use std::collections::{BTreeMap, HashMap, HashSet};

use ferrigura_model::agentflow::{AgentflowDiagram, Vertex};
use ferrigura_model::value::Value;

use super::shapes::{ShapeIssue, TOOL_SHAPES, layout_shape, normalise_shape, resolve_shape_alias};

/// The metadata keys that the semantic model leaves out: presentation only (`SEMANTIC_METADATA_SKIP_KEYS`).
const SEMANTIC_SKIP_KEYS: [&str; 11] = [
    "shape",
    "view",
    "icon",
    "img",
    "form",
    "pos",
    "w",
    "h",
    "class",
    "style",
    "labelType",
];

/// The diagnostic ids that this reading raises (`PARSER_EMITTED_DIAGNOSTICS` of `diagnostics.ts`).
pub const PARSER_EMITTED_DIAGNOSTICS: [&str; 3] = [
    "SHAPE_UNSUPPORTED",
    "SHAPE_REMOVED",
    "CONTAINMENT_VIOLATION",
];

/// The diagnostic ids of the shared vocabulary that the semantics module emits, and this reading
/// never raises (`RESERVED_DIAGNOSTICS`).
pub const RESERVED_DIAGNOSTICS: [&str; 9] = [
    "EDGE_OPERATOR_UNSUPPORTED",
    "REFERENCE_EDGE_LABEL_REJECTED",
    "CONNECTOR_REF_UNRESOLVED",
    "CONNECTOR_REF_NOT_A_CONNECTOR",
    "METADATA_KEY_MISAPPLIED",
    "DUPLICATE_ID_NODE",
    "RESERVED_SYNTHETIC_ID",
    "EDGE_SEMANTIC_CONTRADICTION",
    "FLOW_NO_INPUT",
];

/// The severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// `SHAPE_REMOVED`, and the reference's error tier.
    Error,
    /// `SHAPE_UNSUPPORTED` and `CONTAINMENT_VIOLATION`.
    Warning,
}

/// A diagnostic of the layout data: its id, its severity and the node it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The id: one of [`PARSER_EMITTED_DIAGNOSTICS`].
    pub id: &'static str,
    /// The severity of the id.
    pub severity: Severity,
    /// The node the diagnostic is about.
    pub node_id: String,
}

/// The diagnostics of the layout data, in the order the reference raises them: the containment
/// violations of the parent assignment first, then the shapes of the vertices in order. A removed
/// shape is an error and an unsupported one a warning; the node keeps the default shape either way.
#[must_use]
pub fn diagnostics(diagram: &AgentflowDiagram) -> Vec<Diagnostic> {
    let hidden = hidden_ids(diagram);
    let containers: HashSet<&str> = diagram.sub_graphs.iter().map(|sg| sg.id.as_str()).collect();
    let mut out = containment_violations(diagram, &hidden);
    for vertex in &diagram.vertices {
        if hidden.contains(&vertex.id) || containers.contains(vertex.id.as_str()) {
            continue;
        }
        let shape = layout_shape(vertex.shape.as_deref());
        let (_, issue) = normalise_shape(Some(&shape));
        let (id, severity) = match issue {
            None => continue,
            Some(ShapeIssue::Removed) => ("SHAPE_REMOVED", Severity::Error),
            Some(ShapeIssue::Unsupported) => ("SHAPE_UNSUPPORTED", Severity::Warning),
        };
        out.push(Diagnostic {
            id,
            severity,
            node_id: vertex.id.clone(),
        });
    }
    out
}

/// The members that a collapsed container hides, and the members of those (`getData`,
/// `collectDescendants`). A container that a collapsed one already hides is not a root.
fn hidden_ids(diagram: &AgentflowDiagram) -> HashSet<String> {
    let mut hidden = HashSet::new();
    for sub_graph in &diagram.sub_graphs {
        let collapsed = sub_graph
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("view"))
            .is_some_and(|view| matches!(view, Value::String(text) if text == "collapsed"));
        if collapsed && !hidden.contains(&sub_graph.id) {
            let mut seen = HashSet::new();
            collect_descendants(
                diagram,
                &sub_graph.id,
                &sub_graph.id,
                &mut hidden,
                &mut seen,
            );
        }
    }
    hidden
}

fn collect_descendants(
    diagram: &AgentflowDiagram,
    sub_graph_id: &str,
    ancestor: &str,
    hidden: &mut HashSet<String>,
    seen: &mut HashSet<String>,
) {
    if !seen.insert(sub_graph_id.to_owned()) {
        return;
    }
    let Some(sub_graph) = diagram.sub_graphs.iter().find(|sg| sg.id == sub_graph_id) else {
        return;
    };
    for child in &sub_graph.nodes {
        if child == ancestor || seen.contains(child) {
            continue;
        }
        hidden.insert(child.clone());
        collect_descendants(diagram, child, ancestor, hidden, seen);
    }
}

/// The `CONTAINMENT_VIOLATION` warnings of `getData`'s parent assignment (`agentflowDb.ts`): the
/// containers are walked last to first, and a child whose assignment would close a containment
/// cycle is reported and not assigned. A hidden container is skipped, as the reference skips it.
fn containment_violations(diagram: &AgentflowDiagram, hidden: &HashSet<String>) -> Vec<Diagnostic> {
    let mut parent_of: HashMap<&str, &str> = HashMap::new();
    let mut out = Vec::new();
    for sub_graph in diagram.sub_graphs.iter().rev() {
        if hidden.contains(&sub_graph.id) {
            continue;
        }
        for child in &sub_graph.nodes {
            if closes_cycle(&parent_of, child, &sub_graph.id) {
                out.push(Diagnostic {
                    id: "CONTAINMENT_VIOLATION",
                    severity: Severity::Warning,
                    node_id: child.clone(),
                });
                continue;
            }
            parent_of.insert(child.as_str(), sub_graph.id.as_str());
        }
    }
    out
}

/// `wouldCloseContainmentCycle(child, parent)`: whether the chain of parents from `parent` reaches
/// `child`. The walk stops at a repeated id, as the reference's `seen` set does.
fn closes_cycle(parent_of: &HashMap<&str, &str>, child: &str, parent: &str) -> bool {
    let mut seen = HashSet::new();
    let mut current = Some(parent);
    while let Some(id) = current {
        if seen.contains(id) {
            break;
        }
        if id == child {
            return true;
        }
        seen.insert(id);
        current = parent_of.get(id).copied();
    }
    false
}

/// `deriveVertexKind`: the kind a vertex has for the semantic model. A tool is a vertex whose
/// authored type is one of the tool names (`isToolDefinition` reads the type, not the alias).
#[must_use]
pub fn vertex_kind(vertex: &Vertex) -> &'static str {
    if let Some(vertex_type) = vertex.shape.as_deref()
        && TOOL_SHAPES.contains(&vertex_type)
    {
        return "tool";
    }
    let resolved = vertex.shape.as_deref().map(resolve_shape_alias);
    match resolved.as_deref() {
        Some("hexagon" | "hex") => "action",
        Some("lean-right" | "lean_right") => "input",
        Some("lin-doc" | "lined-document") => "refdoc",
        Some("diamond") => "decision",
        _ => "task",
    }
}

/// The metadata that the semantic model keeps: every key but the presentation ones.
#[must_use]
pub fn semantic_metadata(metadata: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    metadata
        .iter()
        .filter(|(key, _)| !SEMANTIC_SKIP_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// The shape the layout draws a vertex with: `getTypeFromVertex`, then `normaliseNodeShapes`, so a
/// missing, removed or unsupported shape is the default. The issue that the normalisation raises is
/// reported by [`diagnostics`].
#[must_use]
pub fn render_shape(vertex: &Vertex) -> String {
    normalise_shape(Some(&layout_shape(vertex.shape.as_deref()))).0
}

/// A vertex of the semantic model (`SemanticVertex`).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticVertex {
    /// The id.
    pub id: String,
    /// The label: the vertex's text, which is its id when it has no label.
    pub label: String,
    /// The shape after alias resolution, absent for a bare id.
    pub shape: Option<String>,
    /// The kind of the vertex (`deriveVertexKind`).
    pub vertex_kind: &'static str,
    /// The metadata without the presentation keys; absent when none is left.
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// An edge of the semantic model (`SemanticEdge`).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticEdge {
    /// The edge id.
    pub id: String,
    /// The start id.
    pub start: String,
    /// The end id.
    pub end: String,
    /// The label; absent when the edge has none.
    pub label: Option<String>,
    /// The arrow, which the reference calls the edge's `type`.
    pub arrow: String,
    /// `normal` or `dotted`.
    pub stroke: String,
    /// The edge length.
    pub length: u32,
    /// The meaning of the arrow: `sequence`, `reference` or `failure`.
    pub edge_semantic: Option<String>,
    /// The metadata of the edge, all of it (the reference does not filter an edge's metadata).
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A container of the semantic model (`SemanticSubGraph`).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticSubGraph {
    /// The id.
    pub id: String,
    /// The members, in the order the body gave them.
    pub nodes: Vec<String>,
    /// The container's type, which is `flow`.
    pub kind: String,
    /// The title.
    pub title: String,
    /// The container's own direction, from its `direction` statement.
    pub direction: Option<String>,
    /// The metadata without the presentation keys; absent when none is left.
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A connector of the semantic model (`SemanticConnector`).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticConnector {
    /// The id.
    pub id: String,
    /// The title: the text, when it differs from the id.
    pub title: Option<String>,
    /// The metadata without the presentation keys; absent when none is left.
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// The semantic model of a diagram (`getSemanticModel`): what the diagram means, without the
/// styling, the classes, the view and the element mappings.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticModel {
    /// The vertices, in insertion order. A container's id and a connector are not here.
    pub vertices: Vec<SemanticVertex>,
    /// The edges, in creation order.
    pub edges: Vec<SemanticEdge>,
    /// The containers, in completion order.
    pub sub_graphs: Vec<SemanticSubGraph>,
    /// The connectors, in insertion order.
    pub connectors: Vec<SemanticConnector>,
    /// The diagnostics of the layout data (see [`diagnostics`]).
    pub diagnostics: Vec<Diagnostic>,
    /// The direction of the diagram; always present for a parsed diagram.
    pub direction: Option<String>,
}

/// `getSemanticModel`: the presentation-free projection of a diagram, following
/// `agentflowDb.ts` (`getSemanticModel`). The diagnostics are those of [`diagnostics`].
#[must_use]
pub fn semantic_model(diagram: &AgentflowDiagram) -> SemanticModel {
    let sub_graph_ids: HashSet<&str> = diagram.sub_graphs.iter().map(|sg| sg.id.as_str()).collect();
    let mut vertices = Vec::new();
    let mut connectors = Vec::new();
    for vertex in &diagram.vertices {
        if sub_graph_ids.contains(vertex.id.as_str()) {
            continue;
        }
        if vertex.is_connector == Some(true) {
            connectors.push(SemanticConnector {
                id: vertex.id.clone(),
                title: (vertex.text != vertex.id).then(|| vertex.text.clone()),
                metadata: presentation_free(vertex.metadata.as_ref()),
            });
            continue;
        }
        vertices.push(SemanticVertex {
            id: vertex.id.clone(),
            label: vertex.text.clone(),
            shape: vertex.shape.as_deref().map(resolve_shape_alias),
            vertex_kind: vertex_kind(vertex),
            metadata: presentation_free(vertex.metadata.as_ref()),
        });
    }
    let edges = diagram
        .edges
        .iter()
        .map(|edge| SemanticEdge {
            id: edge.id.clone(),
            start: edge.start.clone(),
            end: edge.end.clone(),
            label: (!edge.text.is_empty()).then(|| edge.text.clone()),
            arrow: edge.arrow.clone(),
            stroke: edge.stroke.clone(),
            length: edge.length,
            edge_semantic: edge.edge_semantic.clone(),
            metadata: edge
                .metadata
                .as_ref()
                .filter(|metadata| !metadata.is_empty())
                .cloned(),
        })
        .collect();
    let sub_graphs = diagram
        .sub_graphs
        .iter()
        .map(|sg| SemanticSubGraph {
            id: sg.id.clone(),
            nodes: sg.nodes.clone(),
            kind: sg.kind.clone(),
            title: sg.title.clone(),
            direction: sg.dir.clone(),
            metadata: presentation_free(sg.metadata.as_ref()),
        })
        .collect();
    SemanticModel {
        vertices,
        edges,
        sub_graphs,
        connectors,
        diagnostics: diagnostics(diagram),
        direction: Some(diagram.direction.clone()),
    }
}

/// The metadata without the presentation keys, or `None` when nothing is left.
fn presentation_free(
    metadata: Option<&BTreeMap<String, Value>>,
) -> Option<BTreeMap<String, Value>> {
    metadata
        .map(semantic_metadata)
        .filter(|metadata| !metadata.is_empty())
}
