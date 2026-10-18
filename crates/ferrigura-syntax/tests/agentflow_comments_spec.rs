//! Port of `packages/mermaid/src/diagrams/agentflow/parser/agentflow-comments.spec.ts`. Test names
//! follow the spec's titles in snake_case, with the punctuation of a title left out.
//!
//! The reference's `not.toThrow()` becomes `parse(…)` succeeding, which [`parses`] checks. Its
//! `getVertices()` is the model's `vertices`. The init directives (`%%{init: …}%%`) are not tested
//! here, as in the reference: they are handled before the grammar sees the text.

use ferrigura_model::agentflow::{AgentflowDiagram, Vertex};
use ferrigura_syntax::agentflow::parse;

/// Parses a diagram that the reference accepts.
fn parses(text: &str) -> AgentflowDiagram {
    parse(text).expect("parses")
}

/// The vertex with the given id.
fn vertex<'a>(diagram: &'a AgentflowDiagram, id: &str) -> Option<&'a Vertex> {
    diagram.vertices.iter().find(|vertex| vertex.id == id)
}

#[test]
fn parses_a_comment_before_the_agentflow_keyword() {
    let diagram = parses(
        r#"%% leading comment
agentflow-beta TB
  a --> b"#,
    );
    assert!(vertex(&diagram, "a").is_some());
    assert!(vertex(&diagram, "b").is_some());
}

#[test]
fn parses_multiple_comments_before_the_header() {
    let diagram = parses(
        r#"%% first
%% second
%% third
agentflow-beta TB
  a --> b"#,
    );
    assert!(vertex(&diagram, "a").is_some());
}

#[test]
fn parses_a_comment_immediately_after_the_agentflow_keyword() {
    let diagram = parses(
        r#"agentflow-beta TB
%% trailing header comment
  a --> b"#,
    );
    assert_eq!(diagram.vertices.len(), 2);
}

#[test]
fn parses_a_comment_between_two_vertex_statements() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b
%% mid comment
  b --> c"#,
    );
    assert_eq!(diagram.vertices.len(), 3);
}

#[test]
fn parses_back_to_back_comments_between_statements() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b
%% first mid comment
%% second mid comment
  b --> c"#,
    );
    assert_eq!(diagram.vertices.len(), 3);
}

#[test]
fn parses_a_trailing_comment_at_end_of_diagram() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b
%% trailing comment"#,
    );
    assert_eq!(diagram.vertices.len(), 2);
}

#[test]
fn parses_comment_with_no_space_between_and_the_text() {
    // Without the COMMENT rule ahead of NODE_STRING, `%%foo` lexes as an identifier run and fails.
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b
%%no-space comment
  b --> c"#,
    );
    assert_eq!(diagram.vertices.len(), 3);
}

#[test]
fn parses_empty_comment_line() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b
%%
  b --> c"#,
    );
    assert_eq!(diagram.vertices.len(), 3);
}

#[test]
fn strips_comment_inside_a_square_label_text_state() {
    // The label has to span a line break for the comment to sit inside it.
    let diagram = parses(
        r#"agentflow-beta TB
  a[Plain %% not part of the label
label]
  b --> a"#,
    );
    let text = &vertex(&diagram, "a").expect("vertex a").text;
    assert!(!text.contains("%%"));
    assert!(!text.contains("not part of the label"));
    assert!(text.contains("Plain"));
    assert!(text.contains("label"));
}

#[test]
fn strips_comment_inside_an_ellipse_label_ellipse_text_state() {
    // `(-text-)` puts the lexer in the ellipse-text state, where an inline comment is skipped.
    let diagram = parses(
        r#"agentflow-beta TB
  a(-ellipse %% not part of the label
label-)
  b --> a"#,
    );
    let text = &vertex(&diagram, "a").expect("vertex a").text;
    assert!(!text.contains("%%"));
    assert!(!text.contains("not part of the label"));
    assert!(text.contains("ellipse"));
}

#[test]
fn strips_comment_inside_a_trapezoid_label_trap_text_state() {
    let diagram = parses(
        r#"agentflow-beta TB
  a[/trap %% not part of the label
label/]
  b --> a"#,
    );
    let text = &vertex(&diagram, "a").expect("vertex a").text;
    assert!(!text.contains("%%"));
    assert!(!text.contains("not part of the label"));
    assert!(text.contains("trap"));
}

#[test]
fn parses_comments_inside_and_around_a_flow_container() {
    let diagram = parses(
        r#"agentflow-beta TB
%% before flow
flow f1["Flow One"]
%% inside flow
  a --> b
end
%% after flow"#,
    );
    assert_eq!(diagram.vertices.len(), 2);
}

#[test]
fn parses_comments_around_connector_declarations() {
    let diagram = parses(
        r#"agentflow-beta TB
%% before connector
connector github["GitHub"]
%% after connector
  a --> b"#,
    );
    // Three: the connector and the two implicit endpoints of the edge.
    assert_eq!(diagram.vertices.len(), 3);
}
