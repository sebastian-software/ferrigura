//! Port of `packages/mermaid/src/diagrams/agentflow/agentflow-link-style.spec.ts`, the
//! `linkStyle index bounds` group. Test names follow the spec's titles in snake_case.
//!
//! The reference calls the DB's `updateLinkInterpolate` and `updateLink` directly. Those are the
//! `linkStyle` statements of the grammar, which run during the parse, so each check is a statement
//! in the diagram text. An out-of-range index is a rejection of the parse; the message is not
//! compared.

use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::parse_grammar;

const ONE_EDGE: &str = "agentflow-beta TB
  a[\"Alpha\"]
  b[\"Beta\"]
  a --> b";

#[test]
fn rejects_an_out_of_range_index_for_interpolate_with_the_bounds_message() {
    let text = format!("{ONE_EDGE}\n  linkStyle 99 interpolate basis");
    assert!(parse_grammar(&text).is_err());
}

#[test]
fn rejects_an_out_of_range_index_for_style_with_the_same_message() {
    let text = format!("{ONE_EDGE}\n  linkStyle 99 stroke:red");
    assert!(parse_grammar(&text).is_err());
}

#[test]
fn still_applies_interpolate_at_a_valid_index() {
    let text = format!("{ONE_EDGE}\n  linkStyle 0 interpolate basis");
    let diagram = parse_grammar(&text).expect("parses");
    assert_eq!(
        diagram.edges[0].interpolate,
        Some(Value::String("basis".to_owned()))
    );
}

// The reference's `getEdges().defaultInterpolate` is a side property of the edges array, which the
// model does not carry. The default applies to the edges created after the statement, which is
// what the reference reads (`addSingleLink` takes `defaultInterpolate` when it creates an edge), so
// the statement comes before the edge and the edge's interpolate is the observable.
#[test]
fn still_applies_the_default_interpolate() {
    let text = "agentflow-beta TB
  a[\"Alpha\"]
  b[\"Beta\"]
  linkStyle default interpolate basis
  a --> b";
    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(
        diagram.edges[0].interpolate,
        Some(Value::String("basis".to_owned()))
    );
}
