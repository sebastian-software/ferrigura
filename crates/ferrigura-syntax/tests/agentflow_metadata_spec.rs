//! Ports of five reference specs on the `@{ … }` metadata block. Each one is a module here:
//!
//! - `agentflow-metadata-no-validation.spec.ts` (issue #64) is `no_validation`.
//! - `agentflow-metadata-trailing-comma.spec.ts` is `trailing_commas`.
//! - `agentflow-empty-metadata.spec.ts` (issue #83) is `empty_block`.
//! - `agentflow-metadata-prototype-keys.spec.ts` is `prototype_keys`.
//! - `agentflow-yaml-error-position.spec.ts` (issue #56 part 2) is `yaml_error_position`.
//!
//! Test names are the `it` titles in snake_case. The reference's `toThrow()` becomes a rejection
//! (`parse(…).is_err()`), and its message regexes are not compared. The DB reads are the model's.
//! `getDiagnostics()` is [`diagnostics`]: it raises the shape diagnostics only, which the reference
//! raises from its layout read too, and the parser raises none of the removed semantic ids.
//! `getData()` is called in the reference only as a trigger, and it is not ported.
//! The source positions of errors and of element mappings are not ported (open question Q-g).

use std::collections::BTreeMap;

use ferrigura_model::agentflow::{AgentflowDiagram, Edge, SubGraph, Vertex};
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::semantic::diagnostics;
use ferrigura_syntax::agentflow::{parse, parse_grammar};

/// Parses a diagram that the reference accepts.
fn parses(text: &str) -> AgentflowDiagram {
    parse(text).expect("parses")
}

/// Asserts that the reference's rejection holds: the text does not parse.
fn rejects(text: &str) {
    assert!(parse(text).is_err(), "expected the parse to fail");
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

/// The edge with the given id.
fn edge<'a>(diagram: &'a AgentflowDiagram, id: &str) -> Option<&'a Edge> {
    diagram.edges.iter().find(|edge| edge.id == id)
}

/// A string-valued metadata entry, or `None` when the entry is absent or not a string.
fn string_entry<'a>(metadata: &'a Option<BTreeMap<String, Value>>, key: &str) -> Option<&'a str> {
    metadata.as_ref()?.get(key)?.as_str()
}

/// A number-valued metadata entry, or `None` when the entry is absent or not a number.
fn number_entry(metadata: &Option<BTreeMap<String, Value>>, key: &str) -> Option<f64> {
    metadata.as_ref()?.get(key)?.as_f64()
}

/// The strings of an array-valued metadata entry, in order.
fn string_array<'a>(
    metadata: &'a Option<BTreeMap<String, Value>>,
    key: &str,
) -> Option<Vec<&'a str>> {
    metadata
        .as_ref()?
        .get(key)?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect()
}

/// The metadata of a vertex, or an empty map when it has none (the reference's `?? {}`).
fn metadata_of(vertex: &Vertex) -> BTreeMap<String, Value> {
    vertex.metadata.clone().unwrap_or_default()
}

/// The diagnostics that the reference's `getDiagnostics()` would hold: none.
fn no_diagnostics(diagram: &AgentflowDiagram) {
    let found = diagnostics(diagram);
    assert!(found.is_empty(), "expected no diagnostics, got {found:?}");
}

mod no_validation {
    use super::*;

    #[test]
    fn keeps_an_unknown_metadata_key_on_a_vertex_without_warning() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]
  a@{ myExtension: 42 }"#,
        );
        let a = vertex(&diagram, "a").expect("vertex a");
        assert_eq!(number_entry(&a.metadata, "myExtension"), Some(42.0));
        no_diagnostics(&diagram);
    }

    #[test]
    fn keeps_an_unknown_metadata_key_on_a_flow_without_warning() {
        let diagram = parses(
            r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end
  f@{ totallyCustomKey: "opaque" }"#,
        );
        let f = sub_graph(&diagram, "f").expect("container f");
        assert_eq!(
            string_entry(&f.metadata, "totallyCustomKey"),
            Some("opaque")
        );
        no_diagnostics(&diagram);
    }

    #[test]
    fn keeps_a_misapplied_known_key_execution_on_a_flow_and_does_not_warn() {
        // `execution` is task-only under the old table; the parser no longer checks that.
        let diagram = parses(
            r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end
  f@{ execution: "parallel" }"#,
        );
        let f = sub_graph(&diagram, "f").expect("container f");
        assert_eq!(string_entry(&f.metadata, "execution"), Some("parallel"));
        no_diagnostics(&diagram);
    }

    #[test]
    fn carries_arbitrary_edge_metadata_not_just_instruction() {
        let diagram = parses(
            r#"agentflow-beta TB
  a e1@--> b
  e1@{ instruction: "hand off", weight: 5 }"#,
        );
        let e1 = edge(&diagram, "e1").expect("edge e1");
        assert_eq!(string_entry(&e1.metadata, "instruction"), Some("hand off"));
        assert_eq!(number_entry(&e1.metadata, "weight"), Some(5.0));
        no_diagnostics(&diagram);
    }

    #[test]
    fn carries_prompt_as_an_ordinary_unknown_key_no_alias_no_rename_no_warning() {
        // `prompt` is not an alias of `instruction`: it is carried as it is.
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]
  a@{ prompt: "do the thing" }"#,
        );
        let a = vertex(&diagram, "a").expect("vertex a");
        assert_eq!(string_entry(&a.metadata, "prompt"), Some("do the thing"));
        assert_eq!(string_entry(&a.metadata, "instruction"), None);
        no_diagnostics(&diagram);
    }

    #[test]
    fn does_not_resolve_validate_connector_ref() {
        // The reference spec calls its raw Jison parser on text with no trailing line break, so
        // `call` at the end of the text is an identifier; `parse` would add the line break.
        let diagram = parse_grammar(
            r#"agentflow-beta TB
  call["call"]@{ shape: action, connectorRef: "nonexistent.op" }
  input["in"]@{ shape: input }
  input --> call"#,
        )
        .expect("parses");
        let call = vertex(&diagram, "call").expect("vertex call");
        assert_eq!(
            string_entry(&call.metadata, "connectorRef"),
            Some("nonexistent.op")
        );
        no_diagnostics(&diagram);
    }

    #[test]
    fn does_not_warn_on_a_flow_with_no_input_node() {
        let diagram = parses(
            r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end"#,
        );
        no_diagnostics(&diagram);
    }

    #[test]
    fn does_not_warn_on_a_duplicate_id() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["First"]
  a["Second"]
  a --> b"#,
        );
        no_diagnostics(&diagram);
    }

    #[test]
    fn does_not_warn_on_declaring_the_reserved_connectors_id() {
        let diagram = parses(
            r#"agentflow-beta TB
  connectors["My group"]
  connectors --> b"#,
        );
        no_diagnostics(&diagram);
    }

    #[test]
    fn still_rejects_an_invalid_shape_name_unknown_shape_error() {
        rejects(
            r#"agentflow-beta TB
  a["A"]
  a@{ shape: NotAShape }"#,
        );
    }

    #[test]
    fn still_records_view_collapsed_on_a_flow() {
        let diagram = parses(
            r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end
  f@{ view: "collapsed" }"#,
        );
        let f = sub_graph(&diagram, "f").expect("container f");
        assert_eq!(string_entry(&f.metadata, "view"), Some("collapsed"));
    }
}

mod trailing_commas {
    use super::*;

    #[test]
    fn inline_connector_every_line_comma_terminated() {
        let diagram = parses(
            r#"agentflow-beta TB
  connector github["GitHub"]@{
    protocol: "mcp",
    transport: "stdio",
  }"#,
        );
        let github = &diagram.connectors[0];
        assert_eq!(string_entry(&github.metadata, "protocol"), Some("mcp"));
        assert_eq!(string_entry(&github.metadata, "transport"), Some("stdio"));
    }

    #[test]
    fn inline_connector_mixed_lines_with_and_without_commas() {
        let diagram = parses(
            r#"agentflow-beta TB
  connector github["GitHub"]@{
    protocol: "mcp",
    endpoint: "https://api.github.com"
    token_required: true,
  }"#,
        );
        let github = &diagram.connectors[0];
        assert_eq!(string_entry(&github.metadata, "protocol"), Some("mcp"));
        assert_eq!(
            string_entry(&github.metadata, "endpoint"),
            Some("https://api.github.com")
        );
        assert_eq!(
            github
                .metadata
                .as_ref()
                .and_then(|m| m.get("token_required")),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn standalone_vertex_attachment_with_trailing_commas() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["Alpha"]
  a@{
    description: "alpha node",
    instruction: "be careful",
  }"#,
        );
        let a = vertex(&diagram, "a").expect("vertex a");
        assert_eq!(string_entry(&a.metadata, "description"), Some("alpha node"));
        assert_eq!(string_entry(&a.metadata, "instruction"), Some("be careful"));
    }

    #[test]
    fn flow_header_inline_metadata_with_trailing_commas() {
        let diagram = parses(
            r#"agentflow-beta TB
  flow f["Flow"]@{
    model: "claude-opus-4",
    memory: "shared",
  }
    a --> b
  end"#,
        );
        let f = sub_graph(&diagram, "f").expect("container f");
        assert_eq!(string_entry(&f.metadata, "model"), Some("claude-opus-4"));
        assert_eq!(string_entry(&f.metadata, "memory"), Some("shared"));
    }

    #[test]
    fn strips_a_comma_sitting_before_a_line_trailing_comment() {
        let diagram = parses(
            r#"agentflow-beta TB
  connector github["GitHub"]@{
    protocol: "mcp", # main protocol
    transport: "stdio"
  }"#,
        );
        let github = &diagram.connectors[0];
        assert_eq!(string_entry(&github.metadata, "protocol"), Some("mcp"));
        assert_eq!(string_entry(&github.metadata, "transport"), Some("stdio"));
    }

    #[test]
    fn keeps_commas_inside_block_scalar_content() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["Alpha"]
  a@{
    instruction: |
      Do a thing,
      then stop
    protocol: "mcp",
  }"#,
        );
        let a = vertex(&diagram, "a").expect("vertex a");
        assert_eq!(
            string_entry(&a.metadata, "instruction"),
            Some("Do a thing,\nthen stop\n")
        );
        assert_eq!(string_entry(&a.metadata, "protocol"), Some("mcp"));
    }

    #[test]
    fn keeps_separator_commas_in_flow_sequences_spanning_lines() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["Alpha"]
  a@{
    tools: ["read",
      "write"],
    protocol: "mcp"
  }"#,
        );
        let a = vertex(&diagram, "a").expect("vertex a");
        assert_eq!(
            string_array(&a.metadata, "tools"),
            Some(vec!["read", "write"])
        );
        assert_eq!(string_entry(&a.metadata, "protocol"), Some("mcp"));
    }

    #[test]
    fn keeps_a_comma_ending_a_line_inside_a_single_quoted_scalar() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["Alpha"]
  a@{
    note: 'first,
      second',
    protocol: "mcp"
  }"#,
        );
        let a = vertex(&diagram, "a").expect("vertex a");
        // YAML folds the quoted scalar's newline into a space.
        assert_eq!(string_entry(&a.metadata, "note"), Some("first, second"));
        assert_eq!(string_entry(&a.metadata, "protocol"), Some("mcp"));
    }

    #[test]
    fn parses_a_full_diagram_around_a_trailing_comma_connector_declaration() {
        let diagram = parses(
            r#"agentflow-beta LR
  connector github["GitHub API"]@{
    protocol: "http",
    endpoint: "https://api.github.com",
    token_required: true,
  }

  query["query"]
  query@{ shape: input }
  fetch_issues["fetch_issues"]
  fetch_issues@{ shape: tool, connectorRef: "github.list_issues" }
  query --> fetch_issues"#,
        );
        let github = &diagram.connectors[0];
        assert_eq!(string_entry(&github.metadata, "protocol"), Some("http"));
        assert_eq!(
            string_entry(&github.metadata, "endpoint"),
            Some("https://api.github.com")
        );
        assert_eq!(
            github
                .metadata
                .as_ref()
                .and_then(|m| m.get("token_required")),
            Some(&Value::Bool(true))
        );
        let fetch = vertex(&diagram, "fetch_issues").expect("vertex fetch_issues");
        assert_eq!(
            string_entry(&fetch.metadata, "connectorRef"),
            Some("github.list_issues")
        );
        assert_eq!(diagram.edges.len(), 1);
    }

    #[test]
    fn single_line_form_still_tolerates_a_trailing_comma() {
        let diagram = parses(
            r#"agentflow-beta TB
  connector github["GitHub"]@{ protocol: "mcp", }"#,
        );
        assert_eq!(
            string_entry(&diagram.connectors[0].metadata, "protocol"),
            Some("mcp")
        );
    }

    #[test]
    fn invalid_yaml_that_commas_cannot_fix_still_throws() {
        rejects(
            r#"agentflow-beta TB
  a["Alpha"]
  a@{
    protocol: *undefined_alias,
  }"#,
        );
    }
}

mod empty_block {
    use super::*;

    #[test]
    fn multi_line_empty_block_parses_without_crashing_the_reported_repro() {
        let diagram = parses(
            r#"agentflow-beta TB
  newtask["Example"]@{
}"#,
        );
        let newtask = vertex(&diagram, "newtask").expect("vertex newtask");
        assert_eq!(newtask.text, "Example");
    }

    #[test]
    fn whitespace_only_multi_line_block_parses_without_crashing() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]@{

}"#,
        );
        assert!(vertex(&diagram, "a").is_some());
    }

    #[test]
    fn whitespace_only_single_line_block_parses_without_crashing() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]@{   }"#,
        );
        assert!(vertex(&diagram, "a").is_some());
    }

    #[test]
    fn multi_line_empty_block_behaves_identically_to_single_line() {
        let single = parses(
            r#"agentflow-beta TB
  a["A"]@{}"#,
        );
        let multi = parses(
            r#"agentflow-beta TB
  a["A"]@{
}"#,
        );
        let multi_a = vertex(&multi, "a").expect("vertex a");
        let single_a = vertex(&single, "a").expect("vertex a");
        assert_eq!(multi_a.metadata, single_a.metadata);
        assert_eq!(multi_a.shape, single_a.shape);
    }

    #[test]
    fn empty_multi_line_block_on_a_flow_parses_and_matches_single_line_behaviour() {
        let single = parses(
            r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end
  f@{}"#,
        );
        let multi = parses(
            r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end
  f@{
}"#,
        );
        let multi_f = sub_graph(&multi, "f").expect("container f");
        let single_f = sub_graph(&single, "f").expect("container f");
        assert_eq!(multi_f.metadata, single_f.metadata);
        assert_eq!(vertex(&multi, "f"), vertex(&single, "f"));
    }

    #[test]
    fn a_genuinely_malformed_block_still_throws_a_positioned_yaml_error_not_a_typeerror() {
        // The reference also checks the message's `(line:column)`, which is a source position and is
        // not ported. The rejection is the part kept here.
        rejects(
            r#"agentflow-beta TB
  a["A"]@{
:
}"#,
        );
    }

    #[test]
    fn a_non_string_shape_value_is_rejected_as_an_unknown_shape_not_a_typeerror() {
        rejects(
            r#"agentflow-beta TB
  a["A"]@{ shape: 123 }"#,
        );
    }

    #[test]
    fn a_valid_metadata_block_is_unaffected() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]@{ shape: task }"#,
        );
        // `task` is an alias resolved to the canonical shape id.
        let a = vertex(&diagram, "a").expect("vertex a");
        assert_eq!(a.shape.as_deref(), Some("roundedRect"));
    }
}

mod prototype_keys {
    use super::*;

    #[test]
    fn drops_an_authored_proto_key_from_vertex_metadata() {
        // The reference also checks that `Object.prototype` is not polluted. A Rust map has no
        // prototype, so only the key's removal is ported.
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]
  a@{
    __proto__:
      polluted: true
  }"#,
        );
        let metadata = metadata_of(vertex(&diagram, "a").expect("vertex a"));
        assert!(!metadata.contains_key("__proto__"));
    }

    #[test]
    fn drops_constructor_and_prototype_keys_too() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]
  a@{ constructor: "x", prototype: "y", description: "kept" }"#,
        );
        let metadata = metadata_of(vertex(&diagram, "a").expect("vertex a"));
        let keys: Vec<&str> = metadata.keys().map(String::as_str).collect();
        assert_eq!(keys, ["description"]);
        assert_eq!(
            metadata.get("description").and_then(Value::as_str),
            Some("kept")
        );
    }

    #[test]
    fn strips_nested_prototype_keys_while_keeping_the_surrounding_structure() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]
  a@{
    params:
      - name: first
        __proto__:
          polluted: true
    description: "kept"
  }"#,
        );
        let metadata = metadata_of(vertex(&diagram, "a").expect("vertex a"));
        assert_eq!(
            metadata.get("description").and_then(Value::as_str),
            Some("kept")
        );
        let params = metadata
            .get("params")
            .and_then(Value::as_array)
            .expect("params is an array");
        assert_eq!(params.len(), 1);
        let first = params[0].as_object().expect("params[0] is an object");
        assert_eq!(first.get("name").and_then(Value::as_str), Some("first"));
        assert!(!first.contains_key("__proto__"));
    }

    #[test]
    fn leaves_ordinary_metadata_untouched() {
        let diagram = parses(
            r#"agentflow-beta TB
  a["A"]
  a@{ shape: rounded, description: "hello", myExtension: 42 }"#,
        );
        let metadata = metadata_of(vertex(&diagram, "a").expect("vertex a"));
        assert_eq!(
            metadata.get("description").and_then(Value::as_str),
            Some("hello")
        );
        assert_eq!(
            metadata.get("myExtension").and_then(Value::as_f64),
            Some(42.0)
        );
    }
}

mod yaml_error_position {
    use super::*;

    #[test]
    #[ignore = "source positions are not ported (open question Q-g)"]
    fn single_line_block_maps_block_2_16_to_the_source_line_of_the_block() {
        panic!("source positions of YAML errors are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "source positions are not ported (open question Q-g)"]
    fn multi_line_block_maps_the_body_line_into_source_space_and_keeps_the_tab_caret() {
        panic!("source positions of YAML errors are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "source positions are not ported (open question Q-g)"]
    fn multi_line_block_error_on_the_line_itself_maps_with_the_content_column_offset() {
        panic!("source positions of YAML errors are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "source positions are not ported (open question Q-g)"]
    fn composes_with_the_blank_line_fold_fix_a_block_below_a_keeps_its_source_line() {
        panic!("source positions of YAML errors are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "source positions are not ported (open question Q-g)"]
    fn applies_the_frontmatter_line_offset_on_top_of_the_block_position() {
        panic!("source positions of YAML errors are not ported (open question Q-g)");
    }

    #[test]
    fn leaves_the_message_of_a_non_yaml_error_alone_but_still_locates_it() {
        // The reference checks that the message is not rewritten and that the block's position is
        // carried. The message is not compared, and the position is not ported: the rejection is.
        rejects(
            r#"agentflow-beta TB
  a["A"]@{ shape: Rounded }"#,
        );
    }

    #[test]
    #[ignore = "source positions are not ported (open question Q-g)"]
    fn applies_the_frontmatter_offset_to_a_shape_error_too() {
        panic!("source positions of YAML errors are not ported (open question Q-g)");
    }

    #[test]
    fn a_well_formed_block_parses_without_throwing() {
        parses(
            r#"agentflow-beta TB
  a["A"]@{ shape: rounded }

  b["B"]"#,
        );
    }
}
