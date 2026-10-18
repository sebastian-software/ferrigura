//! Port of two edge specs of `packages/mermaid/src/diagrams/agentflow/parser/`:
//! `agentflow-edge-semantic.spec.ts` (the `edgeSemantic` field, v0.8.1) and
//! `agentflow-edge-instruction.spec.ts` (issue #62, the edge `instruction` metadata). Test names
//! follow the spec's titles in snake_case. An operator in a title is written as a word: `-->` is
//! the normal arrow, `-.-` the dotted link, `--x` the cross arrow, and `---->` the longer normal
//! arrow.
//!
//! Both specs call the reference's raw Jison parser (`agentflow.parser.parse`), so the tests parse
//! with `parse_grammar`: the grammar and the DB alone, with no trailing line break added.
//!
//! The reference reads the edges through `getEdges()`, `getData()` and `getSemanticModel()`. The
//! model's `edges` hold the same values: the semantic model copies each edge and its metadata, and
//! the `getData()` edge list is the DB's list. The checks on them read the model's `edges`.

use ferrigura_model::agentflow::Edge;
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse_grammar;

/// The single edge of the diagram `a <operator> b`, where `a` and `b` are the two vertices.
fn single_edge(operator: &str) -> Edge {
    let text = format!("agentflow-beta TB\n  a[\"A\"]\n  b[\"B\"]\n  a {operator} b");
    let diagram = parse_grammar(&text).expect("parses");
    assert_eq!(diagram.edges.len(), 1);
    diagram
        .edges
        .into_iter()
        .next()
        .expect("the diagram has one edge")
}

// Spec for the `edgeSemantic` field: every edge operator carries the meaning of the arrow. The
// table is `-->` sequence, `-.-` reference and `--x` failure. The other fields are unchanged.

#[test]
fn normal_arrow_maps_to_sequence() {
    assert_eq!(
        single_edge("-->").edge_semantic.as_deref(),
        Some("sequence")
    );
}

#[test]
fn dotted_link_maps_to_reference() {
    assert_eq!(
        single_edge("-.-").edge_semantic.as_deref(),
        Some("reference")
    );
}

#[test]
fn cross_arrow_maps_to_failure() {
    assert_eq!(single_edge("--x").edge_semantic.as_deref(), Some("failure"));
}

#[test]
fn longer_normal_arrow_is_still_sequence() {
    assert_eq!(
        single_edge("---->").edge_semantic.as_deref(),
        Some("sequence")
    );
}

#[test]
fn normal_arrow_keeps_type_arrow_point_and_stroke_normal() {
    let edge = single_edge("-->");
    assert_eq!(edge.arrow, "arrow_point");
    assert_eq!(edge.stroke, "normal");
}

#[test]
fn dotted_link_keeps_stroke_dotted() {
    assert_eq!(single_edge("-.-").stroke, "dotted");
}

#[test]
fn cross_arrow_keeps_type_arrow_cross_and_stroke_normal() {
    let edge = single_edge("--x");
    assert_eq!(edge.arrow, "arrow_cross");
    assert_eq!(edge.stroke, "normal");
}

#[test]
fn surfaces_edge_semantic_in_the_semantic_model_edges() {
    // The semantic model copies the edges of the DB, with their `edgeSemantic`, in order.
    let diagram = parse_grammar(
        r#"agentflow-beta TB
  a["A"]
  b["B"]
  c["C"]
  a --> b
  b --x c"#,
    )
    .expect("parses");
    assert_eq!(diagram.edges.len(), 2);
    assert_eq!(diagram.edges[0].edge_semantic.as_deref(), Some("sequence"));
    assert_eq!(diagram.edges[1].edge_semantic.as_deref(), Some("failure"));
}

// Spec for issue #62: the `instruction` metadata of an edge (section 5.3) is carried from the parse
// to the edges, to the layout edge list and to the semantic model.

/// The instruction that the spec attaches to the edge `e1`.
const INSTRUCTION: &str = "Pass the distilled question and any relevant context to the next step";

/// The diagram of the spec: `a e1@--> b`, and the metadata of `e1` on the next line.
fn instruction_diagram() -> String {
    format!(
        "agentflow-beta TB\n  a[\"A\"]\n  b[\"B\"]\n  a e1@--> b\n  e1@{{ instruction: \"{INSTRUCTION}\" }}"
    )
}

/// The edge with the given id. A missing edge fails the test.
fn edge_by_id<'a>(edges: &'a [Edge], id: &str) -> &'a Edge {
    edges
        .iter()
        .find(|edge| edge.id == id)
        .unwrap_or_else(|| panic!("no edge `{id}`"))
}

#[test]
fn surfaces_instruction_on_the_raw_edge_get_edges() {
    let diagram = parse_grammar(&instruction_diagram()).expect("parses");
    let edge = edge_by_id(&diagram.edges, "e1");
    assert_eq!(
        edge.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("instruction")),
        Some(&Value::String(INSTRUCTION.to_owned()))
    );
}

#[test]
fn carries_instruction_into_the_ir_get_data_issue_62() {
    // The edge list of `getData()` is the DB's edge list, so the check reads the model's edge.
    let diagram = parse_grammar(&instruction_diagram()).expect("parses");
    let edge = edge_by_id(&diagram.edges, "e1");
    assert_eq!(
        edge.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("instruction")),
        Some(&Value::String(INSTRUCTION.to_owned()))
    );
}

#[test]
fn carries_instruction_into_the_semantic_model() {
    // The semantic model keeps the edge metadata, and the model's edge is the same record.
    let diagram = parse_grammar(&instruction_diagram()).expect("parses");
    let edge = edge_by_id(&diagram.edges, "e1");
    assert_eq!(
        edge.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("instruction")),
        Some(&Value::String(INSTRUCTION.to_owned()))
    );
}

#[test]
fn leaves_metadata_undefined_for_edges_without_it() {
    let diagram =
        parse_grammar("agentflow-beta TB\n  a[\"A\"]\n  b[\"B\"]\n  a --> b").expect("parses");
    assert!(diagram.edges[0].metadata.is_none());
}
