//! Port of `packages/mermaid/src/diagrams/agentflow/conformance/conformance.spec.ts` and
//! `packages/mermaid/src/diagrams/agentflow/conformance/runner.spec.ts`.
//!
//! The eight fixture pairs of `conformance/fixtures/` are copied to `tests/fixtures/agentflow/` and
//! loaded with `include_str!`. Each pair is one spec here, with its assertions written out: the
//! outcome (`valid`, `warning`, `error`, `parse-error`), the expected diagnostics (id and node) and
//! the semantic assertions (vertex kind, metadata and edge semantic). The reference's runner
//! (`conformance/runner.ts`) is not ported; its behaviour is the assertions of each spec. The
//! `.expected.json` of each pair is parsed and its outcome is pinned to the one the spec asserts.
//!
//! The reference's `parseErrorContains` pins the parser's message, which is not compared: the
//! parse-error spec asserts that the parse is rejected.
//!
//! The runner's own tests (`runner.spec.ts`) test the reference's helpers, not the parser; they are
//! ignored, with the open question Q-f.

use ferrigura_model::agentflow::AgentflowDiagram;
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse_grammar;
use ferrigura_syntax::agentflow::semantic::{
    Diagnostic, SemanticEdge, SemanticModel, SemanticVertex, Severity, semantic_model,
};

const COMPLETE_EXAMPLE_MMD: &str =
    include_str!("fixtures/agentflow/complete-example-agentflow.mmd");
const COMPLETE_EXAMPLE_JSON: &str =
    include_str!("fixtures/agentflow/complete-example-agentflow.expected.json");
const ERROR_SHAPE_REMOVED_MMD: &str =
    include_str!("fixtures/agentflow/error-shape-removed-agentflow.mmd");
const ERROR_SHAPE_REMOVED_JSON: &str =
    include_str!("fixtures/agentflow/error-shape-removed-agentflow.expected.json");
const PARSE_ERROR_UNTERMINATED_CONTAINER_MMD: &str =
    include_str!("fixtures/agentflow/parse-error-unterminated-container-agentflow.mmd");
const PARSE_ERROR_UNTERMINATED_CONTAINER_JSON: &str =
    include_str!("fixtures/agentflow/parse-error-unterminated-container-agentflow.expected.json");
const PATTERN_CONNECTOR_DOTTED_FORM_MMD: &str =
    include_str!("fixtures/agentflow/pattern-connector-dotted-form-agentflow.mmd");
const PATTERN_CONNECTOR_DOTTED_FORM_JSON: &str =
    include_str!("fixtures/agentflow/pattern-connector-dotted-form-agentflow.expected.json");
const PATTERN_PARALLEL_MMD: &str =
    include_str!("fixtures/agentflow/pattern-parallel-agentflow.mmd");
const PATTERN_PARALLEL_JSON: &str =
    include_str!("fixtures/agentflow/pattern-parallel-agentflow.expected.json");
const PATTERN_REFERENCE_DOC_MMD: &str =
    include_str!("fixtures/agentflow/pattern-reference-doc-agentflow.mmd");
const PATTERN_REFERENCE_DOC_JSON: &str =
    include_str!("fixtures/agentflow/pattern-reference-doc-agentflow.expected.json");
const SMOKE_VALID_MMD: &str = include_str!("fixtures/agentflow/smoke-valid-agentflow.mmd");
const SMOKE_VALID_JSON: &str =
    include_str!("fixtures/agentflow/smoke-valid-agentflow.expected.json");
const WARNING_SHAPE_UNSUPPORTED_MMD: &str =
    include_str!("fixtures/agentflow/warning-shape-unsupported-agentflow.mmd");
const WARNING_SHAPE_UNSUPPORTED_JSON: &str =
    include_str!("fixtures/agentflow/warning-shape-unsupported-agentflow.expected.json");

/// The eight pairs: the name, the source and the expectation of each.
const FIXTURES: [(&str, &str, &str); 8] = [
    (
        "complete-example-agentflow",
        COMPLETE_EXAMPLE_MMD,
        COMPLETE_EXAMPLE_JSON,
    ),
    (
        "error-shape-removed-agentflow",
        ERROR_SHAPE_REMOVED_MMD,
        ERROR_SHAPE_REMOVED_JSON,
    ),
    (
        "parse-error-unterminated-container-agentflow",
        PARSE_ERROR_UNTERMINATED_CONTAINER_MMD,
        PARSE_ERROR_UNTERMINATED_CONTAINER_JSON,
    ),
    (
        "pattern-connector-dotted-form-agentflow",
        PATTERN_CONNECTOR_DOTTED_FORM_MMD,
        PATTERN_CONNECTOR_DOTTED_FORM_JSON,
    ),
    (
        "pattern-parallel-agentflow",
        PATTERN_PARALLEL_MMD,
        PATTERN_PARALLEL_JSON,
    ),
    (
        "pattern-reference-doc-agentflow",
        PATTERN_REFERENCE_DOC_MMD,
        PATTERN_REFERENCE_DOC_JSON,
    ),
    ("smoke-valid-agentflow", SMOKE_VALID_MMD, SMOKE_VALID_JSON),
    (
        "warning-shape-unsupported-agentflow",
        WARNING_SHAPE_UNSUPPORTED_MMD,
        WARNING_SHAPE_UNSUPPORTED_JSON,
    ),
];

/// The expectation of a pair, as its JSON declares it.
fn declared(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("the expectation is JSON")
}

/// The outcome of a parsed diagram, classified from its diagnostics: an error if any is an error,
/// else a warning if any is a warning, else valid.
fn outcome_of(diagnostics: &[Diagnostic]) -> &'static str {
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        "error"
    } else if diagnostics.iter().any(|d| d.severity == Severity::Warning) {
        "warning"
    } else {
        "valid"
    }
}

/// The diagnostics match the expected (id, node) pairs in both directions: each expected pair is
/// one of the actual diagnostics, and each actual one is one of the expected pairs.
fn assert_diagnostics(actual: &[Diagnostic], expected: &[(&str, &str)]) {
    for (id, node) in expected {
        assert!(
            actual.iter().any(|d| d.id == *id && d.node_id == *node),
            "expected diagnostic {id} on {node}, got {actual:?}"
        );
    }
    for diagnostic in actual {
        assert!(
            expected
                .iter()
                .any(|(id, node)| diagnostic.id == *id && diagnostic.node_id == *node),
            "unexpected diagnostic {} on {}",
            diagnostic.id,
            diagnostic.node_id
        );
    }
}

/// The vertex with the id, which the model has.
fn vertex<'a>(model: &'a SemanticModel, id: &str) -> &'a SemanticVertex {
    model
        .vertices
        .iter()
        .find(|v| v.id == id)
        .unwrap_or_else(|| panic!("vertex {id} is in the semantic model"))
}

/// The first edge with the start and the end, which the model has.
fn edge<'a>(model: &'a SemanticModel, start: &str, end: &str) -> &'a SemanticEdge {
    model
        .edges
        .iter()
        .find(|e| e.start == start && e.end == end)
        .unwrap_or_else(|| panic!("edge {start} -> {end} is in the semantic model"))
}

fn assert_vertex_kind(model: &SemanticModel, id: &str, kind: &str) {
    assert_eq!(vertex(model, id).vertex_kind, kind, "vertexKind of {id}");
}

fn assert_edge_semantic(model: &SemanticModel, start: &str, end: &str, semantic: &str) {
    assert_eq!(
        edge(model, start, end).edge_semantic.as_deref(),
        Some(semantic),
        "edgeSemantic of {start} -> {end}"
    );
}

/// The model of a source that parses.
fn model_of(source: &str) -> SemanticModel {
    let diagram: AgentflowDiagram = parse_grammar(source).expect("parses");
    semantic_model(&diagram)
}

#[test]
fn has_discovered_at_least_one_fixture() {
    // The reference discovers the pairs in the directory. Here they are the eight pairs included
    // above, and each one declares its expectation.
    assert!(!FIXTURES.is_empty());
    for (name, source, json) in FIXTURES {
        assert!(!source.is_empty(), "{name} has its source");
        assert!(declared(json).is_object(), "{name} declares an expectation");
    }
}

#[test]
fn complete_example_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(COMPLETE_EXAMPLE_JSON);
    assert_eq!(expected["outcome"], "valid");

    let model = model_of(COMPLETE_EXAMPLE_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "valid");
    assert_diagnostics(&model.diagnostics, &[]);

    assert_vertex_kind(&model, "city", "input");
    assert_vertex_kind(&model, "research_loc", "tool");
    assert_vertex_kind(&model, "write_copy", "tool");
    assert_vertex_kind(&model, "translate_sv", "tool");
    assert_vertex_kind(&model, "gen_html", "tool");
    assert_vertex_kind(&model, "style_guide", "refdoc");
    assert_edge_semantic(&model, "gen_html", "style_guide", "reference");
}

#[test]
fn error_shape_removed_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(ERROR_SHAPE_REMOVED_JSON);
    assert_eq!(expected["outcome"], "error");

    let model = model_of(ERROR_SHAPE_REMOVED_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "error");
    assert_diagnostics(&model.diagnostics, &[("SHAPE_REMOVED", "store")]);
}

#[test]
fn parse_error_unterminated_container_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(PARSE_ERROR_UNTERMINATED_CONTAINER_JSON);
    assert_eq!(expected["outcome"], "parse-error");

    // The reference pins the parser's message with `parseErrorContains`; the message is not
    // compared, so only the rejection is asserted.
    assert!(parse_grammar(PARSE_ERROR_UNTERMINATED_CONTAINER_MMD).is_err());
}

#[test]
fn pattern_connector_dotted_form_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(PATTERN_CONNECTOR_DOTTED_FORM_JSON);
    assert_eq!(expected["outcome"], "valid");

    let model = model_of(PATTERN_CONNECTOR_DOTTED_FORM_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "valid");
    assert_diagnostics(&model.diagnostics, &[]);

    assert_vertex_kind(&model, "payload", "input");
    let create_issue = vertex(&model, "create_issue");
    assert_eq!(create_issue.vertex_kind, "tool");
    let connector_ref = create_issue
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("connectorRef"));
    assert_eq!(
        connector_ref,
        Some(&Value::String("github.create_issue".to_owned()))
    );
    assert_edge_semantic(&model, "payload", "create_issue", "sequence");
}

#[test]
fn pattern_parallel_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(PATTERN_PARALLEL_JSON);
    assert_eq!(expected["outcome"], "valid");

    let model = model_of(PATTERN_PARALLEL_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "valid");
    assert_diagnostics(&model.diagnostics, &[]);

    assert_vertex_kind(&model, "query", "input");
    vertex(&model, "search");
    vertex(&model, "analyze");
    vertex(&model, "validate");
    assert_edge_semantic(&model, "query", "orchestrator", "sequence");
    assert_edge_semantic(&model, "orchestrator", "search", "sequence");
    assert_edge_semantic(&model, "orchestrator", "analyze", "sequence");
    assert_edge_semantic(&model, "orchestrator", "validate", "sequence");
}

#[test]
fn pattern_reference_doc_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(PATTERN_REFERENCE_DOC_JSON);
    assert_eq!(expected["outcome"], "valid");

    let model = model_of(PATTERN_REFERENCE_DOC_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "valid");
    assert_diagnostics(&model.diagnostics, &[]);

    assert_vertex_kind(&model, "query", "input");
    assert_vertex_kind(&model, "style_guide", "refdoc");
    assert_edge_semantic(&model, "query", "gen_html", "sequence");
    assert_edge_semantic(&model, "gen_html", "style_guide", "reference");
}

#[test]
fn smoke_valid_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(SMOKE_VALID_JSON);
    assert_eq!(expected["outcome"], "valid");

    let model = model_of(SMOKE_VALID_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "valid");
    assert_diagnostics(&model.diagnostics, &[]);

    assert_vertex_kind(&model, "input", "input");
    vertex(&model, "a");
    vertex(&model, "b");
    assert_edge_semantic(&model, "input", "a", "sequence");
    assert_edge_semantic(&model, "a", "b", "sequence");
}

#[test]
fn warning_shape_unsupported_matches_its_expected_outcome_and_diagnostics() {
    let expected = declared(WARNING_SHAPE_UNSUPPORTED_JSON);
    assert_eq!(expected["outcome"], "warning");

    let model = model_of(WARNING_SHAPE_UNSUPPORTED_MMD);
    assert_eq!(outcome_of(&model.diagnostics), "warning");
    assert_diagnostics(&model.diagnostics, &[("SHAPE_UNSUPPORTED", "x")]);
}

// runner.spec.ts: the reference's conformance runner, not the parser.

#[test]
#[ignore = "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"]
fn fails_when_semantic_assertions_are_declared_but_no_semantic_model_exists() {
    panic!(
        "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"
    );
}

#[test]
#[ignore = "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"]
fn fails_on_a_diagnostic_the_fixture_did_not_declare() {
    panic!(
        "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"
    );
}

#[test]
#[ignore = "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"]
fn allows_extra_diagnostics_when_the_fixture_opts_in() {
    panic!(
        "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"
    );
}

#[test]
#[ignore = "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"]
fn fails_when_the_parse_error_does_not_match_parse_error_contains() {
    panic!(
        "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"
    );
}

#[test]
#[ignore = "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"]
fn produces_a_semantic_model_for_a_diagram_that_parses() {
    panic!(
        "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"
    );
}

#[test]
#[ignore = "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"]
fn reports_parse_error_and_no_semantic_model_when_the_parser_throws() {
    panic!(
        "tests the reference's conformance runner, not the parser (open question Q-f: the 8 pairs are ported as specs)"
    );
}
