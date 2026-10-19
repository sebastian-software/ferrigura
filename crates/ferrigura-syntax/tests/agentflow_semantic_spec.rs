//! Port of `packages/mermaid/src/diagrams/agentflow/agentflow-semantic-model.spec.ts`, the
//! `getSemanticModel()` tests (v0.8.1). Test names follow the spec's titles in snake_case.
//!
//! The reference's presentation-only contract (styling, classes, the view, icons, images and the
//! element-mapping positions do not change the semantic export) is checked on [`semantic_model`].
//! The reference's `emitWarning` in the last test is replaced by a diagram that raises the same
//! diagnostic: the DB's diagnostic store is not ported, and the diagnostics are read from the model.
//! The position of that diagnostic is not asserted (element mappings are not ported, Q-g).

use std::collections::BTreeMap;

use ferrigura_model::agentflow::AgentflowDiagram;
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse_grammar;
use ferrigura_syntax::agentflow::semantic::{
    SemanticConnector, SemanticEdge, SemanticModel, SemanticSubGraph, SemanticVertex,
    semantic_model,
};

/// Parses a diagram that the reference accepts.
fn parses(text: &str) -> AgentflowDiagram {
    parse_grammar(text).expect("parses")
}

/// The string value of a key, as the test compares it.
fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

/// The vertex with the id, which the model has.
fn vertex<'a>(model: &'a SemanticModel, id: &str) -> &'a SemanticVertex {
    model
        .vertices
        .iter()
        .find(|v| v.id == id)
        .unwrap_or_else(|| panic!("vertex {id} is in the model"))
}

/// The container with the id, which the model has.
fn sub_graph<'a>(model: &'a SemanticModel, id: &str) -> &'a SemanticSubGraph {
    model
        .sub_graphs
        .iter()
        .find(|sg| sg.id == id)
        .unwrap_or_else(|| panic!("container {id} is in the model"))
}

/// The connector with the id, which the model has.
fn connector<'a>(model: &'a SemanticModel, id: &str) -> &'a SemanticConnector {
    model
        .connectors
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("connector {id} is in the model"))
}

/// The first edge with the start and the end, which the model has.
fn edge<'a>(model: &'a SemanticModel, start: &str, end: &str) -> &'a SemanticEdge {
    model
        .edges
        .iter()
        .find(|e| e.start == start && e.end == end)
        .unwrap_or_else(|| panic!("edge {start} -> {end} is in the model"))
}

/// The metadata of a record, empty when the record has none.
fn metadata_of(metadata: &Option<BTreeMap<String, Value>>) -> BTreeMap<String, Value> {
    metadata.clone().unwrap_or_default()
}

// DB surface

#[test]
fn exposes_get_semantic_model() {
    // The reference checks that the method is a function. The Rust function is the projection; a
    // call on a two-vertex diagram returns its vertices.
    let model = semantic_model(&parses("agentflow-beta TB\n  a --> b"));
    assert_eq!(model.vertices.len(), 2);
}

#[test]
fn returns_a_model_with_vertices_edges_sub_graphs_connectors_diagnostics() {
    let model = semantic_model(&parses("agentflow-beta TB\n  a --> b"));
    assert_eq!(model.vertices.len(), 2);
    assert_eq!(model.edges.len(), 1);
    assert!(model.sub_graphs.is_empty());
    assert!(model.connectors.is_empty());
    assert!(model.diagnostics.is_empty());
}

// styled-vs-unstyled equivalence

#[test]
fn produces_identical_models_for_styled_and_unstyled_variants_of_the_same_diagram() {
    let unstyled = parses("agentflow-beta TB\n  a --> b");
    let styled = parses(
        "agentflow-beta TB
  classDef highlight fill:#f9f,stroke:#333,stroke-width:2px
  a:::highlight --> b
  style a fill:#0f0
  linkStyle 0 stroke:red,stroke-width:3px",
    );
    assert_eq!(semantic_model(&styled), semantic_model(&unstyled));
}

// collapsed-vs-expanded equivalence

#[test]
fn view_collapsed_does_not_leak_into_the_vertex_edge_subgraph_semantics() {
    let expanded = parses(
        "agentflow-beta TB
  flow pipeline[\"Pipeline\"]
    a --> b
  end",
    );
    let collapsed = parses(
        "agentflow-beta TB
  flow pipeline[\"Pipeline\"]
    a --> b
  end
  pipeline@{ view: \"collapsed\" }",
    );
    // The diagnostics carry the source positions of the two inputs, which differ, so the
    // comparison is of the semantic surface only.
    let surface = |model: SemanticModel| {
        (
            model.vertices,
            model.edges,
            model.sub_graphs,
            model.connectors,
            model.direction,
        )
    };
    assert_eq!(
        surface(semantic_model(&collapsed)),
        surface(semantic_model(&expanded))
    );
}

// presentation fields stripped

#[test]
fn never_includes_styles_classes_icon_img_w_h_view_on_vertices() {
    let diagram = parses(
        "agentflow-beta TB
  a[\"Alpha\"]
  a@{ icon: \"star\", img: \"http://example/a.png\", w: 100, h: 50, view: \"collapsed\" }
  a:::highlight
  a --> b",
    );
    let model = semantic_model(&diagram);
    let a = vertex(&model, "a");
    let metadata = metadata_of(&a.metadata);
    for key in ["icon", "img", "w", "h", "view", "class", "style"] {
        assert!(!metadata.contains_key(key), "{key} is not in the metadata");
    }
}

#[test]
fn does_not_expose_element_mappings_classes_or_link_styles_on_the_root_model() {
    // The reference checks the absence of these keys at runtime. The Rust model has no such
    // fields: the exhaustive destructuring below fails to compile if one is added, and the
    // bindings are read so that the test asserts the diagram's content.
    let model = semantic_model(&parses("agentflow-beta TB\n  a --> b"));
    let SemanticModel {
        vertices,
        edges,
        sub_graphs,
        connectors,
        diagnostics,
        direction,
    } = model;
    assert_eq!(vertices.len(), 2);
    assert_eq!(edges.len(), 1);
    assert!(sub_graphs.is_empty());
    assert!(connectors.is_empty());
    assert!(diagnostics.is_empty());
    assert_eq!(direction.as_deref(), Some("TB"));
}

// semantic fields preserved

#[test]
fn keeps_domain_metadata_on_flow_containers_model_memory() {
    let diagram = parses(
        "agentflow-beta TB
  flow researcher[\"Researcher\"]
    a --> b
  end
  researcher@{ model: \"claude-opus-4-6\", memory: \"shared\" }",
    );
    let model = semantic_model(&diagram);
    let researcher = sub_graph(&model, "researcher");
    let metadata = metadata_of(&researcher.metadata);
    assert_eq!(metadata.get("model"), Some(&string("claude-opus-4-6")));
    assert_eq!(metadata.get("memory"), Some(&string("shared")));
    assert_eq!(researcher.kind, "flow");
}

#[test]
fn surfaces_declared_connectors_in_the_connectors_projection() {
    let diagram = parses(
        "agentflow-beta TB
  connector github[\"GitHub\"]
  github@{ protocol: \"mcp\", transport: \"stdio\" }
  connector slack[\"Slack\"]
  slack@{ protocol: \"http\" }",
    );
    let model = semantic_model(&diagram);
    let mut ids: Vec<&str> = model.connectors.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, ["github", "slack"]);
    let github = metadata_of(&connector(&model, "github").metadata);
    assert_eq!(github.get("protocol"), Some(&string("mcp")));
    assert_eq!(github.get("transport"), Some(&string("stdio")));
}

#[test]
fn keeps_value_on_input_vertices() {
    let diagram = parses(
        "agentflow-beta TB
  file_path[\"file_path\"]
  file_path@{ shape: input, value: \"src/HelloWorld.java\" }",
    );
    let model = semantic_model(&diagram);
    let file_path = vertex(&model, "file_path");
    let metadata = metadata_of(&file_path.metadata);
    assert_eq!(metadata.get("value"), Some(&string("src/HelloWorld.java")));
    assert_eq!(file_path.vertex_kind, "input");
}

#[test]
fn keeps_edge_type_stroke_label() {
    let diagram = parses(
        "agentflow-beta TB
  a -- yes --> b
  a --x c
  refdoc1[\"RefDoc\"]
  refdoc1@{ shape: refdoc }
  a -.- refdoc1",
    );
    let model = semantic_model(&diagram);
    let labelled = edge(&model, "a", "b");
    assert_eq!(labelled.label.as_deref(), Some("yes"));
    let failure = model
        .edges
        .iter()
        .find(|e| e.end == "c")
        .expect("the failure edge is in the model");
    assert_eq!(failure.edge_semantic.as_deref(), Some("failure"));
    let reference = model
        .edges
        .iter()
        .find(|e| e.end == "refdoc1")
        .expect("the reference edge is in the model");
    assert_eq!(reference.stroke, "dotted");
}

// diagnostics flow through

#[test]
fn includes_diagnostics_emitted_during_the_render_pipeline() {
    // The reference emits a warning on `a` through `emitWarning` and reads the model's
    // diagnostics. Here the unsupported shape of `a` is the warning. The reference's position
    // (`startLine` 2) is an element mapping, which is not ported (Q-g), so it is not asserted.
    let diagram = parses(
        "agentflow-beta TB
  a[\"Alpha\"]
  a@{ shape: \"triangle\" }
  b[\"Beta\"]
  a --> b",
    );
    let model = semantic_model(&diagram);
    assert!(!model.diagnostics.is_empty());
    let shape = model
        .diagnostics
        .iter()
        .find(|d| d.id == "SHAPE_UNSUPPORTED")
        .expect("the unsupported shape is reported");
    assert_eq!(shape.node_id, "a");
}
