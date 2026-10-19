//! Port of three specs of `packages/mermaid/src/diagrams/agentflow/parser/`:
//! `agentflow-tool-validators.spec.ts` (the shape-based tool model, v0.8.1),
//! `agentflow-blank-line-fold.spec.ts` (issue #56, the blank line after an `@{ }` block) and
//! `agentflow-flowchart-parity.spec.ts` (the lexer of agentflow agrees with the flowchart's). Test
//! names follow the spec's titles in snake_case. The `it.each` of the parity spec is one test per row.
//!
//! The tool and the parity specs call the reference's raw Jison parsers (`agentflow.parser.parse`,
//! `flow.parser.parse`), so they parse with the `parse_grammar` of each diagram type. The
//! blank-line spec calls the wrapper (`agentflow.parse`), whose brace-blank rule it tests, so it
//! parses with `parse`.
//!
//! The tool checks read `isToolDefinition`, `getTools()` and `getSemanticModel()`. The first two are
//! read through the semantic module's `vertex_kind`, whose tool names are the reference's
//! `SUBROUTINE_ALIASES`. The position checks of the blank-line spec are source positions and element
//! mappings, which are not ported (open question Q-g). The DB side of each of them is checked: the
//! vertices and the edges, and the line of a parse error, which the parser reports itself.

use std::collections::BTreeMap;

use ferrigura_model::agentflow::{AgentflowDiagram, Vertex};
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::semantic::{semantic_metadata, vertex_kind};
use ferrigura_syntax::agentflow::{parse, parse_grammar};
use ferrigura_syntax::flowchart;

/// Parses with the grammar and the DB alone, as the reference's raw Jison parser does.
fn parses_grammar(text: &str) -> AgentflowDiagram {
    parse_grammar(text).expect("parses")
}

/// Parses the diagram text with the wrapper's steps, as the reference's `agentflow.parse` does.
fn parses(text: &str) -> AgentflowDiagram {
    parse(text).expect("parses")
}

/// The vertex with the given id. A missing vertex fails the test.
fn vertex<'a>(diagram: &'a AgentflowDiagram, id: &str) -> &'a Vertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex `{id}`"))
}

/// `isToolDefinition`: the authored shape of the vertex is a tool name. `vertex_kind` answers
/// `"tool"` for exactly those names.
fn is_tool_definition(vertex: &Vertex) -> bool {
    vertex_kind(vertex) == "tool"
}

/// A metadata entry, or `None` when the record has no metadata or no such key.
fn entry<'a>(metadata: &'a Option<BTreeMap<String, Value>>, key: &str) -> Option<&'a Value> {
    metadata.as_ref()?.get(key)
}

/// A text value.
fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

// Spec for the shape-based tool model: a tool definition is a node whose shape is `subroutine`, or
// one of its aliases. The tool model has no capability metadata of its own.

#[test]
fn recognises_shape_subroutine_canonical_as_a_tool_definition() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  search["search"]
  search@{ shape: subroutine, returns: "Result" }"#,
    );
    assert!(is_tool_definition(vertex(&diagram, "search")));
}

#[test]
fn recognises_the_alias_tool_as_a_tool_definition() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  search["search"]
  search@{ shape: tool }"#,
    );
    assert!(is_tool_definition(vertex(&diagram, "search")));
}

#[test]
fn recognises_the_alias_subprocess_as_a_tool_definition() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  search["search"]
  search@{ shape: subprocess }"#,
    );
    assert!(is_tool_definition(vertex(&diagram, "search")));
}

#[test]
fn does_not_recognise_a_default_shape_node_as_a_tool() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  plain["plain node"]"#,
    );
    assert!(!is_tool_definition(vertex(&diagram, "plain")));
}

#[test]
fn does_not_recognise_an_input_or_action_node_as_a_tool() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  i["i"]
  i@{ shape: input }
  h["h"]
  h@{ shape: action }"#,
    );
    assert!(!is_tool_definition(vertex(&diagram, "i")));
    assert!(!is_tool_definition(vertex(&diagram, "h")));
}

#[test]
fn get_tools_returns_all_tool_shaped_vertices() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  a["a"]
  a@{ shape: subroutine }
  b["b"]
  b@{ shape: tool }
  c["c"]
  d["d"]
  d@{ shape: framed-rectangle }"#,
    );
    let mut tools: Vec<&str> = diagram
        .vertices
        .iter()
        .filter(|vertex| is_tool_definition(vertex))
        .map(|vertex| vertex.id.as_str())
        .collect();
    tools.sort_unstable();
    assert_eq!(tools, ["a", "b", "d"]);
}

#[test]
fn tool_nodes_get_vertex_kind_tool_in_the_semantic_model() {
    // The semantic model reports the kind and the metadata without the presentation keys.
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  search["search"]
  search@{ shape: tool, returns: "Result" }"#,
    );
    let search = vertex(&diagram, "search");
    assert_eq!(vertex_kind(search), "tool");
    let metadata = semantic_metadata(search.metadata.as_ref().expect("the vertex has metadata"));
    assert_eq!(metadata.get("returns"), Some(&text("Result")));
}

#[test]
fn alias_shaped_tools_also_surface_vertex_kind_tool() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  a["a"]
  a@{ shape: subprocess }
  b["b"]
  b@{ shape: framed-rectangle }"#,
    );
    assert_eq!(vertex_kind(vertex(&diagram, "a")), "tool");
    assert_eq!(vertex_kind(vertex(&diagram, "b")), "tool");
}

#[test]
fn retry_and_cache_survive_on_a_tool() {
    let diagram = parses_grammar(
        r#"agentflow-beta TB
  search["search"]
  search@{ shape: tool, retry: 3, cache: "24h", returns: "Result" }"#,
    );
    let search = vertex(&diagram, "search");
    assert_eq!(entry(&search.metadata, "retry"), Some(&Value::Number(3.0)));
    assert_eq!(entry(&search.metadata, "cache"), Some(&text("24h")));
    assert_eq!(entry(&search.metadata, "returns"), Some(&text("Result")));
}

// Spec for issue #56: a blank line after an `@{ }` block must not fold the source positions. The
// positions are element mappings and are not ported (Q-g). The vertices and edges are checked, and
// the line of a parse error, which the parser reports as a position of its own.

#[test]
fn a_blank_line_directly_after_an_inline_block_does_not_shift_the_next_node() {
    let diagram = parses(
        r#"agentflow-beta
flow f["F"]
  a["A"]@{ shape: rounded }

  b["B"]
end"#,
    );
    assert_eq!(vertex(&diagram, "b").text, "B");
}

#[test]
fn the_fold_accumulates_two_blank_lines_after_block_would_drift_by_two() {
    let diagram = parses(
        r#"agentflow-beta
flow f["F"]
  a["A"]@{ shape: rounded }


  b["B"]
end"#,
    );
    assert_eq!(vertex(&diagram, "b").text, "B");
}

#[test]
fn a_comment_line_after_a_block_is_already_correct_and_stays_correct() {
    let diagram = parses(
        r#"agentflow-beta
flow f["F"]
  a["A"]@{ shape: rounded }
  %% a note
  b["B"]
end"#,
    );
    assert_eq!(vertex(&diagram, "b").text, "B");
}

#[test]
fn an_edge_after_a_block_plus_blank_block_keeps_its_source_line() {
    // The edge's position is an element mapping. The edge itself is checked.
    let diagram = parses(
        r#"agentflow-beta TB
  a["A"]@{ shape: rounded }

  c["C"]
  a --> c"#,
    );
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].start, "a");
    assert_eq!(diagram.edges[0].end, "c");
}

#[test]
fn a_thrown_parse_error_reports_the_source_line_not_a_folded_line() {
    // Two `@{ }` blocks, each followed by a blank line, sit above the malformed line. The stray `"]`
    // makes `c` a syntax error on source line 6. A folded blank would move the error up.
    let error = parse(
        r#"agentflow-beta TB
  a["A"]@{ shape: rounded }

  b["B"]@{ shape: rounded }

  c["C"]"]
"#,
    )
    .expect_err("the malformed line is a syntax error");
    assert_eq!(error.line, 6);
}

#[test]
fn still_strips_trailing_whitespace_after_brace_so_the_statement_reduces() {
    // The trailing blanks after the `}` are removed, and the block reduces. The position of `b` is
    // not ported. The two vertices are checked.
    let diagram = parses("agentflow-beta TB\n  a[\"A\"]@{ shape: rounded }   \n\n  b[\"B\"]");
    assert_eq!(vertex(&diagram, "a").id, "a");
    assert_eq!(vertex(&diagram, "b").id, "b");
}

// Spec for the lexer parity: agentflow must accept exactly what flowchart accepts on the syntax both
// grammars share. The comparison runs both Rust parsers on the same body.

/// Whether agentflow accepts the body (in the `agentflow-beta TB` diagram of the spec).
fn agentflow_accepts(body: &str) -> bool {
    parse_grammar(&format!("agentflow-beta TB\n{body}\n")).is_ok()
}

/// Whether flowchart accepts the body (in the `flowchart TB` diagram of the spec). The flowchart's
/// `parse_grammar` keeps the wrapper's closing-brace rule, which changes none of these bodies: each
/// `}` in them is followed by a line break.
fn flowchart_accepts(body: &str) -> bool {
    flowchart::parse_grammar(&format!("flowchart TB\n{body}\n")).is_ok()
}

/// The parity check of one body: both grammars agree on whether to accept it.
fn assert_agrees(body: &str) {
    assert_eq!(
        agentflow_accepts(body),
        flowchart_accepts(body),
        "the grammars disagree on {body:?}"
    );
}

#[test]
fn agrees_on_whether_to_accept_shape_data_with_a_quoted_label() {
    assert_agrees("  A@{ label: \"hello\" }");
}

#[test]
fn agrees_on_whether_to_accept_shape_data_with_a_shape() {
    assert_agrees("  A@{ shape: rect }");
}

#[test]
fn agrees_on_whether_to_accept_shape_data_with_a_caret() {
    assert_agrees("  A@{ label: a^b }");
}

#[test]
fn agrees_on_whether_to_accept_multi_line_shape_data() {
    assert_agrees("  A@{\n    label: \"multi\"\n  }");
}

#[test]
fn agrees_on_whether_to_accept_bracket_labels_joined_by_an_arrow() {
    assert_agrees("  A[\"Label\"] --> B[\"Other\"]");
}

#[test]
fn agrees_on_whether_to_accept_an_arrow_with_text() {
    assert_agrees("  A -- text --> B");
}

#[test]
fn agrees_on_whether_to_accept_a_bracket_label_with_parentheses() {
    assert_agrees("  A[\"a (paren) b\"]");
}

#[test]
fn rejects_a_caret_in_shape_data_exactly_like_flowchart_does() {
    // Agentflow once accepted `A@{ label: a^b }`, which flowchart rejects. Both reject it now.
    let body = "  A@{ label: a^b }";
    assert!(!agentflow_accepts(body));
    assert!(!flowchart_accepts(body));
}
