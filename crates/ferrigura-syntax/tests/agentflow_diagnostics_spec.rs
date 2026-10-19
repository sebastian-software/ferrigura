//! Port of `packages/mermaid/src/diagrams/agentflow/agentflow-diagnostics.spec.ts` and
//! `packages/mermaid/src/diagrams/agentflow/diagnostics.spec.ts`. Test names follow the spec's
//! titles in snake_case; a `it.each` is one test per row.
//!
//! The reference's diagnostics are a store that the DB fills (`emitWarning`, `emitError`,
//! `getDiagnostics`, `clear`), with a `log` line for each one. That store is not ported: the
//! diagnostics are read from the model by `semantic::diagnostics`, and the tests of the store are
//! ignored with that reason. The tests of the shape diagnostics and of the vocabulary are live.
//! Element mappings and positions are not ported (Q-g).

use ferrigura_model::agentflow::AgentflowDiagram;
use ferrigura_syntax::agentflow::parse_grammar;
use ferrigura_syntax::agentflow::semantic::{
    PARSER_EMITTED_DIAGNOSTICS, RESERVED_DIAGNOSTICS, Severity, diagnostics, render_shape,
};

/// Parses a diagram that the reference accepts.
fn parses(text: &str) -> AgentflowDiagram {
    parse_grammar(text).expect("parses")
}

/// The ids of the diagnostics of a diagram, in order.
fn ids_for(text: &str) -> Vec<&'static str> {
    diagnostics(&parses(text)).iter().map(|d| d.id).collect()
}

// The diagnostic vocabulary of `diagnostics.ts` (`AgentflowWarning`), with the ids of the spec.
const VOCABULARY: [&str; 12] = [
    "SHAPE_UNSUPPORTED",
    "SHAPE_REMOVED",
    "EDGE_OPERATOR_UNSUPPORTED",
    "REFERENCE_EDGE_LABEL_REJECTED",
    "CONNECTOR_REF_UNRESOLVED",
    "CONNECTOR_REF_NOT_A_CONNECTOR",
    "METADATA_KEY_MISAPPLIED",
    "DUPLICATE_ID_NODE",
    "RESERVED_SYNTHETIC_ID",
    "CONTAINMENT_VIOLATION",
    "EDGE_SEMANTIC_CONTRADICTION",
    "FLOW_NO_INPUT",
];

/// One diagram per emitted id, chosen to raise exactly that diagnostic.
const REPRODUCTIONS: [(&str, &str); 3] = [
    (
        "SHAPE_UNSUPPORTED",
        "agentflow-beta TB
  a[\"Alpha\"]
  a@{ shape: \"triangle\" }
  b[\"Beta\"]
  a --> b",
    ),
    (
        "SHAPE_REMOVED",
        "agentflow-beta TB
  a[\"Alpha\"]
  a@{ shape: \"cylinder\" }
  b[\"Beta\"]
  a --> b",
    ),
    (
        "CONTAINMENT_VIOLATION",
        "agentflow-beta TB
  flow A[\"Flow A\"]
    a1[\"Task A1\"]
    a1 --> B
  end
  flow B[\"Flow B\"]
    b1[\"Task B1\"]
    b1 --> A
  end",
    ),
];

// DB surface and the diagnostic store: not ported.

#[test]
#[ignore = "DB diagnostic store: emitWarning, emitError and getDiagnostics are not ported; the diagnostics are read from the model (semantic::diagnostics)"]
fn exposes_emit_warning_emit_error_and_get_diagnostics() {
    panic!("DB diagnostic store: emitWarning, emitError and getDiagnostics are not ported");
}

#[test]
#[ignore = "DB diagnostic store: emitWarning is not ported; the diagnostics are read from the model (semantic::diagnostics)"]
fn records_a_structured_diagnostic_with_the_given_id_and_message() {
    panic!("DB diagnostic store: emitWarning is not ported");
}

#[test]
#[ignore = "logging: the log.warn of emitWarning is not ported"]
fn also_writes_to_log_warn_so_console_output_is_preserved() {
    panic!("logging: the log.warn of emitWarning is not ported");
}

#[test]
#[ignore = "element mappings and source positions are not ported (open question Q-g)"]
fn attaches_a_position_from_the_element_mapping_when_a_node_id_is_given() {
    panic!("element mappings and source positions are not ported (open question Q-g)");
}

#[test]
#[ignore = "DB diagnostic store: emitWarning without a context is not ported"]
fn leaves_position_undefined_when_no_node_id_is_given() {
    panic!("DB diagnostic store: emitWarning without a context is not ported");
}

#[test]
#[ignore = "element mappings and source positions are not ported (open question Q-g)"]
fn leaves_position_undefined_when_the_referenced_node_id_has_no_mapping() {
    panic!("element mappings and source positions are not ported (open question Q-g)");
}

#[test]
#[ignore = "DB diagnostic store: emitWarning is not ported; the diagnostics are read from the model (semantic::diagnostics)"]
fn accumulates_multiple_diagnostics_in_order() {
    panic!("DB diagnostic store: emitWarning is not ported");
}

#[test]
#[ignore = "logging: emitError and its log.error are not ported; the severity of a shape is in semantic::diagnostics"]
fn records_a_diagnostic_with_severity_error_and_writes_to_log_error() {
    panic!("logging: emitError and its log.error are not ported");
}

#[test]
#[ignore = "logging: transformData's log.warn fallback without a DB is not ported"]
fn falls_back_to_log_warn_when_no_db_is_passed() {
    panic!("logging: transformData's log.warn fallback without a DB is not ported");
}

#[test]
#[ignore = "DB diagnostic store: clear() drops the diagnostics of the store, which is not ported"]
fn clear_drops_diagnostics() {
    panic!("DB diagnostic store: clear() is not ported");
}

// The shape diagnostics of the layout data.

// The reference passes a layout node `bogus-shape` to `transformData`; the parser of agentflow
// rejects an unknown shape name, so the unsupported shape is written as `triangle`, a name of the
// registry that is not in agentflow's catalogue. The node keeps the default shape. The message is
// the reference's text, which is not compared.
#[test]
fn emits_shape_unsupported_through_emit_warning_when_a_db_is_passed() {
    let diagram = parses(
        "agentflow-beta TB
  n1[\"N1\"]
  n1@{ shape: \"triangle\" }
  n2[\"N2\"]
  n1 --> n2",
    );
    let found = diagnostics(&diagram);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "SHAPE_UNSUPPORTED");
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].node_id, "n1");
    let n1 = diagram
        .vertices
        .iter()
        .find(|v| v.id == "n1")
        .expect("n1 is in the diagram");
    assert_eq!(render_shape(n1), "roundedRect");
}

// The reference passes the layout shapes `roundedRect` and `diamond` to `transformData`. An
// authored shape must be lowercase, so the authored aliases of those two shapes are written: `task`
// resolves to `roundedRect` and `decision` to `diamond`.
#[test]
fn does_not_emit_for_supported_shapes() {
    let diagram = parses(
        "agentflow-beta TB
  n1[\"A\"]
  n1@{ shape: task }
  n2@{ shape: decision }
  n1 --> n2",
    );
    assert!(diagnostics(&diagram).is_empty());
}

// diagnostics.spec: the vocabulary and the reproductions.

#[test]
fn partitions_every_id_into_emitted_or_reserved() {
    let mut all: Vec<&str> = VOCABULARY.to_vec();
    all.sort_unstable();
    all.dedup();

    let mut partitioned: Vec<&str> = PARSER_EMITTED_DIAGNOSTICS
        .iter()
        .chain(RESERVED_DIAGNOSTICS.iter())
        .copied()
        .collect();
    partitioned.sort_unstable();
    partitioned.dedup();
    assert_eq!(partitioned, all);

    for id in PARSER_EMITTED_DIAGNOSTICS {
        assert!(
            !RESERVED_DIAGNOSTICS.contains(&id),
            "{id} is emitted, so it is not reserved"
        );
    }
}

#[test]
fn has_a_reproduction_for_every_id_claimed_as_emitted() {
    let mut reproduced: Vec<&str> = REPRODUCTIONS.iter().map(|(id, _)| *id).collect();
    reproduced.sort_unstable();
    let mut emitted: Vec<&str> = PARSER_EMITTED_DIAGNOSTICS.to_vec();
    emitted.sort_unstable();
    assert_eq!(reproduced, emitted);
}

#[test]
fn actually_emits_shape_unsupported() {
    let (id, source) = REPRODUCTIONS[0];
    assert!(ids_for(source).contains(&id));
}

#[test]
fn actually_emits_shape_removed() {
    let (id, source) = REPRODUCTIONS[1];
    assert!(ids_for(source).contains(&id));
}

#[test]
fn actually_emits_containment_violation() {
    let (id, source) = REPRODUCTIONS[2];
    assert!(ids_for(source).contains(&id));
}

#[test]
fn emits_nothing_for_a_clean_diagram() {
    assert!(
        ids_for(
            "agentflow-beta TB
  a[\"Alpha\"]
  b[\"Beta\"]
  a --> b"
        )
        .is_empty()
    );
}
