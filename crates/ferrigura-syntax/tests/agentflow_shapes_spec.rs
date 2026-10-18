//! Port of `packages/mermaid/src/diagrams/agentflow/agentflow-shape-catalogue.spec.ts`. Test names
//! follow the spec's titles in snake_case.
//!
//! Agentflow's shape catalogue is the six domain shapes; the icon and image keys stay authorable as
//! ordinary metadata. The reference's `getData` node shape is the layout's, and it is read here as
//! [`render_shape`] (`getTypeFromVertex` then `normaliseNodeShapes`). The reference's
//! `getDiagnostics()` is read as [`diagnostics`], which the DB's store is not ported for.

use std::collections::BTreeMap;

use ferrigura_model::agentflow::{AgentflowDiagram, Vertex};
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse_grammar;
use ferrigura_syntax::agentflow::semantic::{diagnostics, render_shape, semantic_model};

/// Parses a diagram that the reference accepts.
fn parses(text: &str) -> AgentflowDiagram {
    parse_grammar(text).expect("parses")
}

/// The vertex with the id, which the diagram has.
fn vertex<'a>(diagram: &'a AgentflowDiagram, id: &str) -> &'a Vertex {
    diagram
        .vertices
        .iter()
        .find(|v| v.id == id)
        .unwrap_or_else(|| panic!("vertex {id} is in the diagram"))
}

/// The string value of a key, as the test compares it.
fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

#[test]
fn carries_icon_metadata_through_without_selecting_an_icon_shape() {
    let diagram = parses(
        "agentflow-beta TB
  a[\"A\"]@{ icon: \"fa:bell\", form: circle }
  b[\"B\"]
  a --> b",
    );
    let a = vertex(&diagram, "a");

    assert_eq!(render_shape(a), "roundedRect");
    let metadata = a.metadata.as_ref().expect("the vertex has metadata");
    assert_eq!(metadata.get("icon"), Some(&string("fa:bell")));
    assert_eq!(metadata.get("form"), Some(&string("circle")));
    // No shape was authored, so nothing to complain about.
    assert!(diagnostics(&diagram).is_empty());
}

#[test]
fn carries_image_metadata_through_without_selecting_an_image_shape() {
    let diagram = parses(
        "agentflow-beta TB
  a[\"A\"]@{ img: \"https://example.com/x.png\", w: 40, h: 40 }
  b[\"B\"]
  a --> b",
    );
    let a = vertex(&diagram, "a");

    assert_eq!(render_shape(a), "roundedRect");
    let metadata = a.metadata.as_ref().expect("the vertex has metadata");
    assert_eq!(
        metadata.get("img"),
        Some(&string("https://example.com/x.png"))
    );
    assert_eq!(metadata.get("w"), Some(&Value::Number(40.0)));
    assert_eq!(metadata.get("h"), Some(&Value::Number(40.0)));
    assert!(diagnostics(&diagram).is_empty());
}

#[test]
fn keeps_the_label_when_icon_or_image_metadata_is_present() {
    let diagram = parses(
        "agentflow-beta TB
  a[\"Real label\"]@{ icon: \"fa:bell\" }
  b[\"B\"]
  a --> b",
    );
    let model = semantic_model(&diagram);
    let a = model
        .vertices
        .iter()
        .find(|v| v.id == "a")
        .expect("a is in the model");
    // The old icon path blanked the label whenever the metadata carried none.
    assert_eq!(a.label, "Real label");
}

#[test]
fn still_reports_an_explicitly_authored_out_of_catalogue_shape() {
    let diagram = parses(
        "agentflow-beta TB
  a[\"A\"]@{ shape: icon, icon: \"fa:bell\" }
  b[\"B\"]
  a --> b",
    );
    let a = vertex(&diagram, "a");

    assert_eq!(render_shape(a), "roundedRect");
    // The author did write `shape: icon`, so the warning is about their input.
    let ids: Vec<&str> = diagnostics(&diagram).iter().map(|d| d.id).collect();
    assert!(ids.contains(&"SHAPE_UNSUPPORTED"));
}

#[test]
fn strips_presentation_keys_from_the_semantic_model() {
    let diagram = parses(
        "agentflow-beta TB
  a[\"A\"]@{ icon: \"fa:bell\", img: \"x.png\", form: circle, pos: t, w: 40, h: 40, description: \"kept\" }
  b[\"B\"]
  a --> b",
    );

    let model = semantic_model(&diagram);
    let a = model
        .vertices
        .iter()
        .find(|v| v.id == "a")
        .expect("a is in the model");
    let metadata: BTreeMap<String, Value> = a.metadata.clone().unwrap_or_default();
    assert_eq!(metadata.get("description"), Some(&string("kept")));
    // `form` and `pos` are as presentation-only as `icon` and `img`, so they are in the skip set.
    for key in ["icon", "img", "form", "pos", "w", "h"] {
        assert!(!metadata.contains_key(key), "{key} is not in the metadata");
    }
}

#[test]
fn leaves_the_six_domain_shapes_working() {
    let diagram = parses(
        "agentflow-beta TB
  t[\"T\"]@{ shape: task }
  o[\"O\"]@{ shape: tool }
  i[\"I\"]@{ shape: input }
  d[\"D\"]@{ shape: decision }
  r[\"R\"]@{ shape: refdoc }
  c[\"C\"]@{ shape: action }
  t --> o --> i --> d --> r --> c",
    );

    let shapes: BTreeMap<&str, String> = diagram
        .vertices
        .iter()
        .map(|v| (v.id.as_str(), render_shape(v)))
        .collect();
    let expected: BTreeMap<&str, String> = [
        ("t", "roundedRect"),
        ("o", "subroutine"),
        ("i", "lean-right"),
        ("d", "diamond"),
        ("r", "lin-doc"),
        ("c", "hexagon"),
    ]
    .into_iter()
    .map(|(id, shape)| (id, shape.to_owned()))
    .collect();
    assert_eq!(shapes, expected);
    assert!(diagnostics(&diagram).is_empty());
}

// The `[/…/]` short form reaches the DB as `lean_right` (underscore), which used to fall through to
// roundedRect with a SHAPE_UNSUPPORTED warning.
#[test]
fn treats_the_slash_form_as_an_input_same_as_shape_input() {
    let diagram = parses(
        "agentflow-beta TB
flow f[\"F\"]
  a[/Slanted/]
  m[\"Meta\"]@{ shape: input }
  a --> m
end",
    );

    let a = vertex(&diagram, "a");
    let m = vertex(&diagram, "m");
    assert_eq!(render_shape(a), "lean-right");
    assert_eq!(render_shape(a), render_shape(m));

    let model = semantic_model(&diagram);
    let semantic_a = model
        .vertices
        .iter()
        .find(|v| v.id == "a")
        .expect("a is in the model");
    assert_eq!(semantic_a.shape.as_deref(), Some("lean-right"));
    assert_eq!(semantic_a.vertex_kind, "input");

    assert!(diagnostics(&diagram).is_empty());
}
