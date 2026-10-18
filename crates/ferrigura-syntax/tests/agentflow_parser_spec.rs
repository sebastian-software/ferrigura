//! Port of `packages/mermaid/src/diagrams/agentflow/parser/agentflow.spec.ts` (v0.8.1). Test names
//! follow the spec's titles in snake_case, with the punctuation of a title left out.
//!
//! The reference reads the model through the DB getters, and through `getData()` for four checks.
//! Where `getData()` only lists the vertices and the edges that the DB holds, the test reads the
//! model's `vertices` and `edges`, as the kanban port does. The checks that only `getData()` makes
//! (the group flag of a container, the redirect of a collapsed container's edges) are the
//! renderer's and are not ported. `isToolDefinition` and `getSemanticModel().vertexKind` are read
//! through [`vertex_kind`].

use std::collections::BTreeMap;

use ferrigura_model::agentflow::{AgentflowDiagram, SubGraph, Vertex};
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse;
use ferrigura_syntax::agentflow::semantic::vertex_kind;

/// Parses a diagram that the reference accepts.
fn parses(text: &str) -> AgentflowDiagram {
    parse(text).expect("parses")
}

/// The vertex with the given id.
fn vertex<'a>(diagram: &'a AgentflowDiagram, id: &str) -> Option<&'a Vertex> {
    diagram.vertices.iter().find(|vertex| vertex.id == id)
}

/// The container with the given id.
fn sub_graph<'a>(diagram: &'a AgentflowDiagram, id: &str) -> Option<&'a SubGraph> {
    diagram
        .sub_graphs
        .iter()
        .find(|sub_graph| sub_graph.id == id)
}

/// A string-valued metadata entry, or `None` when the entry is absent or not a string.
fn string_entry<'a>(metadata: &'a Option<BTreeMap<String, Value>>, key: &str) -> Option<&'a str> {
    metadata.as_ref()?.get(key)?.as_str()
}

#[test]
fn parses_an_empty_diagram_with_just_the_keyword_newline() {
    // The reference's `getData().nodes` is the vertices of the DB, here.
    let diagram = parses(
        r#"agentflow-beta TB
"#,
    );
    assert!(diagram.vertices.is_empty());
    assert!(diagram.edges.is_empty());
}

#[test]
fn parses_two_nodes_and_an_edge() {
    let diagram = parses(
        r#"agentflow-beta TB
  a["Alpha"]
  b["Beta"]
  a --> b"#,
    );
    assert_eq!(diagram.vertices.len(), 2);
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].start, "a");
    assert_eq!(diagram.edges[0].end, "b");
}

#[test]
fn captures_node_labels() {
    let diagram = parses(
        r#"agentflow-beta TB
  research["Research"]
  research --> write"#,
    );
    let research = vertex(&diagram, "research").expect("vertex research");
    assert_eq!(research.text, "Research");
}

#[test]
fn parses_lr_direction() {
    let diagram = parses(
        r#"agentflow-beta LR
  a --> b"#,
    );
    assert_eq!(diagram.direction, "LR");
}

#[test]
fn renders_a_self_loop_edge_a_to_a() {
    // The reference reads the edges of `getData()`, which are the DB's when no container collapses.
    let diagram = parses(
        r#"agentflow-beta TB
  a["Alpha"]
  a --> a"#,
    );
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].start, "a");
    assert_eq!(diagram.edges[0].end, "a");
}

#[test]
fn keeps_a_self_loop_alongside_a_normal_edge() {
    let diagram = parses(
        r#"agentflow-beta TB
  a["Alpha"]
  b["Beta"]
  a --> b
  b --> b"#,
    );
    assert_eq!(diagram.edges.len(), 2);
    assert!(
        diagram
            .edges
            .iter()
            .any(|edge| edge.start == "b" && edge.end == "b")
    );
}

#[test]
#[ignore = "renderer: getData's redirect of a collapsed container's edges (issue #53) is not ported"]
fn still_drops_collapse_induced_self_loops_issue_53_invariant() {
    panic!("renderer: getData's redirect of a collapsed container's edges is not ported");
}

#[test]
fn parses_a_flow_with_a_quoted_label() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow pipeline["Pipeline"]
    a --> b
  end"#,
    );
    assert_eq!(diagram.sub_graphs.len(), 1);
    assert_eq!(diagram.sub_graphs[0].id, "pipeline");
    assert_eq!(diagram.sub_graphs[0].title, "Pipeline");
    assert_eq!(diagram.sub_graphs[0].kind, "flow");
}

#[test]
fn parses_a_flow_without_a_label() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow pipeline
    a --> b
  end"#,
    );
    assert_eq!(diagram.sub_graphs.len(), 1);
    assert_eq!(diagram.sub_graphs[0].id, "pipeline");
}

#[test]
fn parses_nested_flows() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow outer["Outer"]
    flow inner["Inner"]
      a --> b
    end
  end"#,
    );
    assert!(diagram.sub_graphs.len() >= 2);
    assert!(sub_graph(&diagram, "inner").is_some());
    assert!(sub_graph(&diagram, "outer").is_some());
}

#[test]
#[ignore = "renderer: the group flag (isGroup) is getData's layout data, which is not ported"]
fn exposes_flow_as_the_container_type_in_layout_data() {
    panic!("renderer: the group flag (isGroup) is getData's layout data, which is not ported");
}

#[test]
fn parses_arrow_as_arrow_point_sequence() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b"#,
    );
    assert_eq!(diagram.edges.len(), 1);
    let edge = &diagram.edges[0];
    assert_eq!(edge.arrow, "arrow_point");
    assert_eq!(edge.stroke, "normal");
    assert_eq!(edge.edge_semantic.as_deref(), Some("sequence"));
}

#[test]
fn parses_x_as_arrow_cross_failure() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --x b"#,
    );
    assert_eq!(diagram.edges.len(), 1);
    let edge = &diagram.edges[0];
    assert_eq!(edge.arrow, "arrow_cross");
    assert_eq!(edge.edge_semantic.as_deref(), Some("failure"));
}

#[test]
fn parses_a_dotted_reference_edge() {
    let diagram = parses(
        r#"agentflow-beta TB
  a -.- b"#,
    );
    assert_eq!(diagram.edges.len(), 1);
    let edge = &diagram.edges[0];
    assert_eq!(edge.stroke, "dotted");
    assert_eq!(edge.edge_semantic.as_deref(), Some("reference"));
}

#[test]
fn accepts_labels_on_arrow() {
    let diagram = parses(
        r#"agentflow-beta TB
  a -- yes --> b"#,
    );
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].text, "yes");
}

#[test]
fn accepts_shape_tool() {
    let diagram = parses(
        r#"agentflow-beta TB
  s["Search"]
  s@{ shape: tool, returns: "Result" }"#,
    );
    let search = vertex(&diagram, "s").expect("vertex s");
    assert_eq!(vertex_kind(search), "tool");
}

#[test]
fn accepts_shape_action() {
    let diagram = parses(
        r#"agentflow-beta TB
  a["Action"]
  a@{ shape: action }"#,
    );
    let a = vertex(&diagram, "a").expect("vertex a");
    assert_eq!(vertex_kind(a), "action");
}

#[test]
fn accepts_shape_input() {
    let diagram = parses(
        r#"agentflow-beta TB
  i["x"]
  i@{ shape: input, value: "literal" }"#,
    );
    let i = vertex(&diagram, "i").expect("vertex i");
    assert_eq!(vertex_kind(i), "input");
}

#[test]
fn accepts_shape_refdoc() {
    let diagram = parses(
        r#"agentflow-beta TB
  r["Guide"]
  r@{ shape: refdoc }"#,
    );
    let r = vertex(&diagram, "r").expect("vertex r");
    assert_eq!(vertex_kind(r), "refdoc");
}

#[test]
fn accepts_shape_decision_and_inline_id_diamond_syntax() {
    let diagram = parses(
        r#"agentflow-beta TB
  d{"is_valid?"}"#,
    );
    let d = vertex(&diagram, "d").expect("vertex d");
    assert_eq!(vertex_kind(d), "decision");
}

#[test]
fn parses_a_connector_declaration() {
    let diagram = parses(
        r#"agentflow-beta TB
  connector github["GitHub"]
  github@{ protocol: "mcp", transport: "stdio" }"#,
    );
    assert_eq!(diagram.connectors.len(), 1);
    assert_eq!(diagram.connectors[0].id, "github");
    assert_eq!(
        string_entry(&diagram.connectors[0].metadata, "protocol"),
        Some("mcp")
    );
}

#[test]
fn multiple_connectors_are_tracked_separately() {
    let diagram = parses(
        r#"agentflow-beta TB
  connector github["GitHub"]
  connector slack["Slack"]"#,
    );
    let mut ids: Vec<&str> = diagram
        .connectors
        .iter()
        .map(|connector| connector.id.as_str())
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, ["github", "slack"]);
}

#[test]
fn parses_inline_metadata_on_a_labeled_connector_declaration() {
    let diagram = parses(
        r#"agentflow-beta TB
  connector github["GitHub"]@{ protocol: "mcp", transport: "stdio" }"#,
    );
    assert_eq!(diagram.connectors.len(), 1);
    let github = &diagram.connectors[0];
    assert_eq!(github.id, "github");
    assert_eq!(github.text, "GitHub");
    assert_eq!(string_entry(&github.metadata, "protocol"), Some("mcp"));
    assert_eq!(string_entry(&github.metadata, "transport"), Some("stdio"));
}

#[test]
fn parses_inline_metadata_on_a_bare_connector_declaration() {
    let diagram = parses(
        r#"agentflow-beta TB
  connector github@{ protocol: "mcp" }"#,
    );
    assert_eq!(diagram.connectors.len(), 1);
    assert_eq!(diagram.connectors[0].id, "github");
    assert_eq!(
        string_entry(&diagram.connectors[0].metadata, "protocol"),
        Some("mcp")
    );
}

#[test]
fn merges_inline_metadata_with_a_later_standalone_attachment() {
    let diagram = parses(
        r#"agentflow-beta TB
  connector github["GitHub"]@{ protocol: "mcp" }
  github@{ transport: "stdio" }"#,
    );
    assert_eq!(diagram.connectors.len(), 1);
    let github = &diagram.connectors[0];
    assert_eq!(string_entry(&github.metadata, "protocol"), Some("mcp"));
    assert_eq!(string_entry(&github.metadata, "transport"), Some("stdio"));
}

#[test]
fn parses_multi_line_inline_metadata_on_a_connector_declaration() {
    let diagram = parses(
        r#"agentflow-beta TB
  connector github["GitHub"]@{
    protocol: "mcp"
    transport: "stdio"
  }
  a --> b"#,
    );
    assert_eq!(diagram.connectors.len(), 1);
    let github = &diagram.connectors[0];
    assert_eq!(github.id, "github");
    assert_eq!(string_entry(&github.metadata, "protocol"), Some("mcp"));
    assert_eq!(string_entry(&github.metadata, "transport"), Some("stdio"));
}

#[test]
fn expands_a_arrow_b_and_c_into_two_edges() {
    let diagram = parses(
        r#"agentflow-beta TB
  a --> b & c"#,
    );
    assert_eq!(diagram.edges.len(), 2);
    let mut ends: Vec<&str> = diagram.edges.iter().map(|edge| edge.end.as_str()).collect();
    ends.sort_unstable();
    assert_eq!(ends, ["b", "c"]);
}

#[test]
fn parses_flat_metadata_on_a_vertex() {
    let diagram = parses(
        r#"agentflow-beta TB
  a["Alpha"]
  a@{ description: "alpha node", instruction: "be careful" }"#,
    );
    let a = vertex(&diagram, "a").expect("vertex a");
    assert_eq!(string_entry(&a.metadata, "description"), Some("alpha node"));
    assert_eq!(string_entry(&a.metadata, "instruction"), Some("be careful"));
}

#[test]
fn merges_multiple_metadata_statements_on_the_same_vertex() {
    let diagram = parses(
        r#"agentflow-beta TB
  a["A"]
  a@{ description: "first" }
  a@{ instruction: "second" }"#,
    );
    let a = vertex(&diagram, "a").expect("vertex a");
    assert_eq!(string_entry(&a.metadata, "description"), Some("first"));
    assert_eq!(string_entry(&a.metadata, "instruction"), Some("second"));
}

#[test]
fn parses_metadata_on_a_flow_container() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow pipeline["P"]
    a --> b
  end
  pipeline@{ model: "claude-opus-4-6", memory: "shared" }"#,
    );
    let pipeline = sub_graph(&diagram, "pipeline").expect("container pipeline");
    assert_eq!(
        string_entry(&pipeline.metadata, "model"),
        Some("claude-opus-4-6")
    );
    assert_eq!(string_entry(&pipeline.metadata, "memory"), Some("shared"));
}

#[test]
fn marks_a_subgraph_as_collapsed_via_view_collapsed() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow pipeline["P"]
    a --> b
  end
  pipeline@{ view: "collapsed" }"#,
    );
    let pipeline = sub_graph(&diagram, "pipeline").expect("container pipeline");
    assert_eq!(string_entry(&pipeline.metadata, "view"), Some("collapsed"));
}
