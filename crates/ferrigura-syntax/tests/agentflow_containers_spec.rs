//! Port of five container specs of `packages/mermaid/src/diagrams/agentflow/parser/`:
//! `agentflow-global.spec.ts` (issue #80), `agentflow-containment-cycle.spec.ts`,
//! `agentflow-flow-self-reference.spec.ts` (issue #70), `agentflow-flow-metadata.spec.ts`
//! (issue #63) and `agentflow-subgraph-span.spec.ts` (issue #59). Test names follow the spec's
//! titles in snake_case, with the punctuation of a title left out.
//!
//! The specs call the reference's raw Jison parser (`agentflow.parser.parse`), so the tests parse
//! with `parse_grammar`: the grammar and the DB alone, with no trailing line break added.
//!
//! The reference reads the DB through its getters and through `getData()`. The checks on values the
//! DB holds (the members of a container, the vertices, the edges and their metadata, a title, the
//! semantic diagnostics) read the model. The values that only `getData()` computes are left out:
//! the parent id of a node, the group flag, the collapsed shape, and the diagnostic that `getData()`
//! raises (`CONTAINMENT_VIOLATION`). Each test that leaves one out says so in a comment. The
//! element mappings and the source positions are not ported (open question Q-g).

use std::collections::BTreeMap;

use ferrigura_model::agentflow::{AgentflowDiagram, SubGraph, Vertex};
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse_grammar;
use ferrigura_syntax::agentflow::semantic::diagnostics;

/// Parses with the grammar and the DB alone, as the reference's raw Jison parser does.
fn parses(text: &str) -> AgentflowDiagram {
    parse_grammar(text).expect("parses")
}

/// The container with the given id. A missing container fails the test.
fn flow<'a>(diagram: &'a AgentflowDiagram, id: &str) -> &'a SubGraph {
    diagram
        .sub_graphs
        .iter()
        .find(|sub_graph| sub_graph.id == id)
        .unwrap_or_else(|| panic!("no container `{id}`"))
}

/// The vertex with the given id. A missing vertex fails the test.
fn vertex<'a>(diagram: &'a AgentflowDiagram, id: &str) -> &'a Vertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex `{id}`"))
}

/// Whether the container lists the id as a member.
fn has_member(sub_graph: &SubGraph, id: &str) -> bool {
    sub_graph.nodes.iter().any(|node| node == id)
}

/// The ids of the containers that list the id as a member, in order.
fn containers_of<'a>(diagram: &'a AgentflowDiagram, id: &str) -> Vec<&'a str> {
    diagram
        .sub_graphs
        .iter()
        .filter(|sub_graph| has_member(sub_graph, id))
        .map(|sub_graph| sub_graph.id.as_str())
        .collect()
}

/// Whether an edge runs from `start` to `end`.
fn has_edge(diagram: &AgentflowDiagram, start: &str, end: &str) -> bool {
    diagram
        .edges
        .iter()
        .any(|edge| edge.start == start && edge.end == end)
}

/// A metadata entry, or `None` when the record has no metadata or no such key.
fn entry<'a>(metadata: &'a Option<BTreeMap<String, Value>>, key: &str) -> Option<&'a Value> {
    metadata.as_ref()?.get(key)
}

/// A text value.
fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

/// The mutual containment of two flows. The edge `a1 --> B` inside `flow A` folds `B` into `A`, and
/// the mirror edge folds `A` into `B`. The arguments are the metadata of the two flow headers.
fn mutual(view_a: &str, view_b: &str) -> String {
    format!(
        "agentflow-beta TB\n  flow A[\"Flow A\"]{view_a}\n    a1[\"Task A1\"]\n    a1 --> B\n  end\n  flow B[\"Flow B\"]{view_b}\n    b1[\"Task B1\"]\n    b1 --> A\n  end"
    )
}

/// The `global` block of the issue #80 spec: `A` is global, and the flow holds `B` and `C`.
const GLOBAL_SOURCE: &str = r#"agentflow-beta

global
  A
end

flow
 A --> B --> C
end"#;

// Spec for issue #80: the `global ... end` block. Nodes declared in it keep no container, even when
// a `flow` block names them afterwards.

#[test]
fn excludes_a_global_node_from_the_flow_membership() {
    let diagram = parses(GLOBAL_SOURCE);
    assert_eq!(diagram.sub_graphs.len(), 1);
    let sub_graph = &diagram.sub_graphs[0];
    assert!(!has_member(sub_graph, "A"));
    assert!(has_member(sub_graph, "B"));
    assert!(has_member(sub_graph, "C"));
}

#[test]
fn emits_the_global_node_without_a_parent_id_while_flow_members_keep_theirs() {
    // The reference reads the parent id from `getData()`, which takes it from the member lists.
    // The DB side is checked here: no container lists `A`, and the flow lists `B` and `C`.
    let diagram = parses(GLOBAL_SOURCE);
    let flow_id = diagram.sub_graphs[0].id.as_str();
    assert!(containers_of(&diagram, "A").is_empty());
    assert_eq!(containers_of(&diagram, "B"), [flow_id]);
    assert_eq!(containers_of(&diagram, "C"), [flow_id]);
}

#[test]
fn keeps_the_edges_from_the_flow_intact() {
    // The edge list of `getData()` is the DB's edge list, so the edges are read from the model.
    let diagram = parses(GLOBAL_SOURCE);
    assert!(has_edge(&diagram, "A", "B"));
    assert!(has_edge(&diagram, "B", "C"));
}

#[test]
fn emits_no_container_node_for_the_global_block_itself() {
    // `getData()` marks the containers with `isGroup`, which is the renderer's flag. The DB side is
    // that the global block is no container: the flow is the only one.
    let diagram = parses(GLOBAL_SOURCE);
    assert_eq!(diagram.sub_graphs.len(), 1);
    assert!(has_member(&diagram.sub_graphs[0], "B"));
}

#[test]
fn is_order_independent_a_global_block_after_the_flow_still_releases_the_node() {
    let diagram = parses(
        r#"agentflow-beta

flow pipeline["Pipeline"]
  A --> B
end

global
  A
end"#,
    );
    let pipeline = flow(&diagram, "pipeline");
    assert!(!has_member(pipeline, "A"));
    assert!(has_member(pipeline, "B"));
}

#[test]
fn allows_edges_inside_the_global_block_both_endpoints_become_global() {
    // The parent ids of `getData()` are left out. The members and the edges are checked.
    let diagram = parses(
        r#"agentflow-beta

global
  A --> X
end

flow f["Flow"]
  A --> B
  X --> B
end"#,
    );
    assert_eq!(flow(&diagram, "f").nodes, ["B"]);
    assert!(has_edge(&diagram, "A", "X"));
}

#[test]
fn acts_as_an_escape_hatch_when_nested_inside_a_flow() {
    // The parent ids of `getData()` are left out. The members are checked.
    let diagram = parses(
        r#"agentflow-beta

flow f["Flow"]
  global
    A
  end
  A --> B
end"#,
    );
    assert_eq!(flow(&diagram, "f").nodes, ["B"]);
}

#[test]
fn supports_labels_and_shapes_on_nodes_declared_in_the_global_block() {
    // The label is the vertex's text and the shape is the vertex's shape. The parent id of
    // `getData()` is left out, and the containers are checked: none lists `A` or `D`.
    let diagram = parses(
        r#"agentflow-beta

global
  A["Shared input"]
  D{"Decision"}
end

flow f["Flow"]
  A --> B --> D
end"#,
    );
    assert_eq!(vertex(&diagram, "A").text, "Shared input");
    assert!(containers_of(&diagram, "A").is_empty());
    assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("diamond"));
    assert!(containers_of(&diagram, "D").is_empty());
}

#[test]
fn merges_multiple_global_blocks() {
    let diagram = parses(
        r#"agentflow-beta

global
  A
end

global
  B
end

flow f["Flow"]
  A --> B --> C
end"#,
    );
    assert_eq!(flow(&diagram, "f").nodes, ["C"]);
}

#[test]
fn keeps_global_usable_inside_bracketed_labels() {
    let diagram = parses(
        r#"agentflow-beta

flow f["global settings"]
  a["a global thing"] --> b
end"#,
    );
    assert_eq!(diagram.sub_graphs[0].title, "global settings");
    assert_eq!(vertex(&diagram, "a").text, "a global thing");
}

// Regression spec for mutual containment between two `flow` containers. The DB keeps the cycle in
// the member lists. The parent choice of `getData()` (which of the two wins) is the renderer's.

#[test]
#[ignore = "layout data (getData) comes with the renderer"]
fn does_not_emit_a_parent_chain_that_closes_a_cycle() {
    panic!("not ported: the parent chain is the layout data of getData");
}

#[test]
fn keeps_the_first_containment_and_drops_only_the_one_closing_the_loop() {
    // The reference's parent of `A` and `B` is `getData()`'s. The DB holds the cycle: `A` lists `B`
    // and `B` lists `A`. The tasks keep their own containers.
    let diagram = parses(&mutual("", ""));
    assert!(has_member(flow(&diagram, "A"), "B"));
    assert!(has_member(flow(&diagram, "B"), "A"));
    assert_eq!(containers_of(&diagram, "a1"), ["A"]);
    assert_eq!(containers_of(&diagram, "b1"), ["B"]);
}

#[test]
#[ignore = "layout data (getData) comes with the renderer"]
fn reports_the_dropped_nesting_as_a_containment_violation() {
    panic!("not ported: CONTAINMENT_VIOLATION is raised by getData");
}

#[test]
fn one_collapsed_does_not_recurse_forever() {
    // The recursion guard of `getData()` is the renderer's. The parse accepts the text, and the
    // semantic reading of the model, whose walk over the hidden members has a guard too, returns.
    let diagram = parses(&mutual("@{ view: collapsed }", ""));
    let _ = diagnostics(&diagram);
}

#[test]
fn one_collapsed_still_draws_the_collapsed_container_it_was_asked_to_draw() {
    // The collapsed shape and the parent id are `getData()`'s. The DB holds the container with its
    // `view` metadata.
    let diagram = parses(&mutual("@{ view: collapsed }", ""));
    assert_eq!(
        entry(&flow(&diagram, "A").metadata, "view"),
        Some(&text("collapsed"))
    );
}

#[test]
fn both_collapsed_does_not_recurse_forever() {
    let diagram = parses(&mutual("@{ view: collapsed }", "@{ view: collapsed }"));
    let _ = diagnostics(&diagram);
}

#[test]
fn both_collapsed_draws_at_least_one_of_the_two_containers_rather_than_blanking() {
    // `getData()` draws at least one container. The DB keeps both of them.
    let diagram = parses(&mutual("@{ view: collapsed }", "@{ view: collapsed }"));
    let ids: Vec<&str> = diagram
        .sub_graphs
        .iter()
        .map(|sub_graph| sub_graph.id.as_str())
        .collect();
    assert_eq!(ids, ["A", "B"]);
}

#[test]
fn three_container_cycle_terminates() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow A["A"]@{ view: collapsed }
    a1["a1"]
    a1 --> B
  end
  flow B["B"]
    b1["b1"]
    b1 --> C
  end
  flow C["C"]
    c1["c1"]
    c1 --> A
  end"#,
    );
    let _ = diagnostics(&diagram);
}

// Regression spec for issue #70: a task in a `flow ... end` block that draws an edge to the flow's
// own id must not make the flow a member of its own container.

#[test]
fn does_not_make_the_flow_a_member_of_its_own_subgraph() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow myFlow["My Flow"]
    a["Task A"]
    a --> myFlow
  end"#,
    );
    let sub_graph = flow(&diagram, "myFlow");
    assert!(has_member(sub_graph, "a"));
    assert!(!has_member(sub_graph, "myFlow"));
}

#[test]
fn emits_the_flow_node_without_a_self_parent_id() {
    // The parent id of `getData()` is left out. The flow is a container that does not list itself.
    let diagram = parses(
        r#"agentflow-beta TB
  flow myFlow["My Flow"]
    a["Task A"]
    a --> myFlow
  end"#,
    );
    assert!(!has_member(flow(&diagram, "myFlow"), "myFlow"));
}

#[test]
fn still_emits_the_task_inside_the_flow_and_the_edge_to_the_flow() {
    // The parent id of the task is `getData()`'s. The members and the edge are checked.
    let diagram = parses(
        r#"agentflow-beta TB
  flow myFlow["My Flow"]
    a["Task A"]
    a --> myFlow
  end"#,
    );
    assert!(has_member(flow(&diagram, "myFlow"), "a"));
    assert!(has_edge(&diagram, "a", "myFlow"));
}

#[test]
fn a_nested_flow_referencing_its_own_id_degrades_the_same_way() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow outer["Outer"]
    flow inner["Inner"]
      b["Task B"]
      b --> inner
    end
  end"#,
    );
    assert!(!has_member(flow(&diagram, "inner"), "inner"));
    // `inner` keeps its real parent, `outer`, which lists it. Only the self-membership is dropped.
    assert!(has_member(flow(&diagram, "outer"), "inner"));
}

#[test]
fn self_reference_at_the_middle_of_a_three_level_nesting_keeps_the_chain_intact() {
    // The parent chain (leafFlow in mid, mid in top, c in leafFlow, m in mid) is `getData()`'s. The
    // member lists it reads from are checked.
    let diagram = parses(
        r#"agentflow-beta TB
  flow top["Top"]
    flow mid["Mid"]
      flow leafFlow["Leaf"]
        c["Task C"]
      end
      m["Task M"]
      m --> mid
    end
  end"#,
    );
    assert!(!has_member(flow(&diagram, "mid"), "mid"));
    assert!(has_member(flow(&diagram, "top"), "mid"));
    assert!(has_member(flow(&diagram, "mid"), "leafFlow"));
    assert!(has_member(flow(&diagram, "mid"), "m"));
    assert!(has_member(flow(&diagram, "leafFlow"), "c"));
    assert!(containers_of(&diagram, "top").is_empty());
}

#[test]
fn self_references_at_two_nesting_levels_at_once_are_both_dropped() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow outer["Outer"]
    flow inner["Inner"]
      b["Task B"]
      b --> inner
    end
    a["Task A"]
    a --> outer
  end"#,
    );
    assert!(!has_member(flow(&diagram, "outer"), "outer"));
    assert!(!has_member(flow(&diagram, "inner"), "inner"));
    assert!(has_member(flow(&diagram, "outer"), "inner"));
    assert!(has_edge(&diagram, "b", "inner"));
    assert!(has_edge(&diagram, "a", "outer"));
}

#[test]
fn a_normal_flow_membership_is_untouched() {
    // The parent ids of `getData()` are left out. The members are checked.
    let diagram = parses(
        r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end"#,
    );
    let sub_graph = flow(&diagram, "f");
    assert!(has_member(sub_graph, "a"));
    assert!(has_member(sub_graph, "b"));
}

// Regression spec for issue #63: inline `@{ ... }` metadata on a flow header. The metadata goes to
// the container, as the standalone `<id>@{ ... }` attachment does, and it is carried verbatim.

#[test]
fn attaches_multi_line_inline_metadata_to_the_flow_issue_example() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow article_generation@{
         model: "claude-opus-4"
       }
        A --> B
  end"#,
    );
    let sub_graph = flow(&diagram, "article_generation");
    assert_eq!(
        entry(&sub_graph.metadata, "model"),
        Some(&text("claude-opus-4"))
    );
    // The flow body is still parsed.
    assert!(has_member(sub_graph, "A"));
    assert!(has_member(sub_graph, "B"));
    assert_eq!(diagram.edges.len(), 1);
}

#[test]
fn attaches_single_line_inline_metadata_to_the_flow() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow f@{ model: "claude-opus-4", memory: "shared" }
    a --> b
  end"#,
    );
    let sub_graph = flow(&diagram, "f");
    assert_eq!(
        entry(&sub_graph.metadata, "model"),
        Some(&text("claude-opus-4"))
    );
    assert_eq!(entry(&sub_graph.metadata, "memory"), Some(&text("shared")));
}

#[test]
fn supports_the_labelled_inline_form_and_keeps_the_title() {
    let diagram = parses(
        r#"agentflow-beta TB
  flow f["Article Generation"]@{ model: "claude-opus-4" }
    a --> b
  end"#,
    );
    let sub_graph = flow(&diagram, "f");
    assert_eq!(sub_graph.title, "Article Generation");
    assert_eq!(
        entry(&sub_graph.metadata, "model"),
        Some(&text("claude-opus-4"))
    );
}

#[test]
fn carries_inline_metadata_verbatim_without_validating_it_issue_64() {
    // The parser carries every key as written. The applicability of the keys is the semantic
    // module's concern, and this diagram raises no diagnostic.
    let diagram = parses(
        r#"agentflow-beta TB
  flow f@{ model: "m", memory: "shared", execution: "parallel", custom: 1 }
    a --> b
  end"#,
    );
    let sub_graph = flow(&diagram, "f");
    assert_eq!(entry(&sub_graph.metadata, "model"), Some(&text("m")));
    assert_eq!(entry(&sub_graph.metadata, "memory"), Some(&text("shared")));
    assert_eq!(
        entry(&sub_graph.metadata, "execution"),
        Some(&text("parallel"))
    );
    assert_eq!(
        entry(&sub_graph.metadata, "custom"),
        Some(&Value::Number(1.0))
    );
    assert!(diagnostics(&diagram).is_empty());
}

// Regression spec for issue #59: the span of a sibling flow must not overlap its neighbour. The
// spans are source positions and are not ported. The membership of a nested flow is checked.

#[test]
#[ignore = "element mappings and source positions are not ported (open question Q-g)"]
fn two_sibling_flows_do_not_overlap_on_the_boundary_line() {
    panic!("not ported: the spans of the element mappings are source positions");
}

#[test]
fn a_nested_inner_flow_ends_at_its_own_end_not_the_parent_content() {
    // The positions are not ported. The members are: the inner flow holds `a` and `b`, and the
    // parent's `c --> d` is not in it.
    let diagram = parses(
        r#"agentflow-beta TB
  flow outer["Outer"]
    flow inner["Inner"]
      a --> b
    end
    c --> d
  end"#,
    );
    let inner = flow(&diagram, "inner");
    assert!(has_member(inner, "a"));
    assert!(has_member(inner, "b"));
    assert!(!has_member(inner, "c"));
    assert!(!has_member(inner, "d"));
}

#[test]
fn a_flow_whose_end_is_the_last_line_no_trailing_newline_still_parses_and_maps() {
    // The position of `f` is not ported. The parse and the container and vertices are checked.
    let diagram = parses(
        r#"agentflow-beta TB
  flow f["F"]
    a --> b
  end"#,
    );
    assert_eq!(flow(&diagram, "f").id, "f");
    assert_eq!(vertex(&diagram, "a").id, "a");
    assert_eq!(vertex(&diagram, "b").id, "b");
}

#[test]
#[ignore = "element mappings and source positions are not ported (open question Q-g)"]
fn prefers_the_tighter_span_when_a_nested_container_shares_the_point() {
    panic!("not ported: getElementAtPosition is the editor's API (Q-g)");
}

#[test]
#[ignore = "element mappings and source positions are not ported (open question Q-g)"]
fn still_resolves_the_innermost_element_when_a_label_is_wider_than_1000_columns() {
    panic!("not ported: getElementAtPosition is the editor's API (Q-g)");
}
