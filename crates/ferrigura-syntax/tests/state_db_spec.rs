//! Port of `packages/mermaid/src/diagrams/state/stateDb.spec.js`. Test names follow the spec's titles
//! in snake_case.
//!
//! The spec calls the DB object directly, and the port has no public DB API. Each DB call is
//! written as the state diagram text that makes it, and the outcome is read from the model through
//! [`parse_grammar`]. The grammar drops one leading `:` of a description itself (`trimColon`) and
//! `addDescription` drops another, so a description that the spec passes as `:x` is written `: :x`
//! in the text.

use ferrigura_model::state::{StateDiagram, StateEntry};
use ferrigura_syntax::state::parse_grammar;

/// Parses the text, which the spec expects to parse.
fn parse_ok(text: &str) -> StateDiagram {
    parse_grammar(text).expect("parses")
}

/// The state with the given id. Panics with the id when there is none.
fn state<'a>(diagram: &'a StateDiagram, id: &str) -> &'a StateEntry {
    diagram
        .states
        .iter()
        .find(|state| state.id == id)
        .unwrap_or_else(|| panic!("no state {id}"))
}

#[test]
fn is_added_to_the_list_of_style_classes() {
    let diagram =
        parse_ok("stateDiagram-v2\nclassDef newStyleClass font-weight:bold, border:blue;");

    let class = diagram
        .classes
        .iter()
        .find(|class| class.id == "newStyleClass")
        .expect("the class is added");
    assert_eq!(class.id, "newStyleClass");
    assert_eq!(class.styles, ["font-weight:bold", "border:blue"]);
}

#[test]
fn removes_only_the_first_leading() {
    // `state1 : x` gives `x`; `state1 : : x` gives `x`, as `trimColon` and `addDescription` each
    // drop one `:`; `state1 : : :x` gives `:x`.
    let diagram = parse_ok(
        "stateDiagram-v2\nstate1\nstate1 : rest of the description\nstate1 : : rest of the description\nstate1 : : :rest of the description",
    );

    let descriptions = &state(&diagram, "state1").descriptions;
    assert_eq!(descriptions[0], "rest of the description");
    assert_eq!(descriptions[1], "rest of the description");
    assert_eq!(descriptions[2], ":rest of the description");
}

#[test]
fn adds_each_description_to_the_array_of_descriptions() {
    let diagram = parse_ok(
        "stateDiagram-v2\nstate1\nstate1 : description 0\nstate1 : description 1\nstate1 : description 2",
    );

    assert_eq!(
        state(&diagram, "state1").descriptions,
        ["description 0", "description 1", "description 2"]
    );
}

#[test]
fn sanitizes_on_the_description() {
    let diagram = parse_ok(
        "stateDiagram-v2\nstate1\nstate1 : desc outside the script <script>the description</script>",
    );

    assert_eq!(
        state(&diagram, "state1").descriptions[0],
        "desc outside the script "
    );
}

#[test]
fn adds_the_description_to_the_state_with_the_given_id() {
    let diagram = parse_ok("stateDiagram-v2\nstate1\nstate1 : the description");

    assert_eq!(state(&diagram, "state1").descriptions[0], "the description");
}

#[test]
#[ignore = "port: calls the DB directly; no public DB API, the parse tests cover it"]
fn should_have_functions_used_in_flow_jison_as_own_property() {
    // stateDb.spec.js checks that the JISON's helper functions are own properties of the JS
    // object. The port has no such object, so there is nothing to check.
}
