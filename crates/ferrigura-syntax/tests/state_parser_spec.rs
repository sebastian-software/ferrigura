//! Port of `packages/mermaid/src/diagrams/state/parser/state-parser.spec.js`. Test names follow the
//! spec's titles in snake_case; the `it.each` tests keep the substituted value in the title. Each
//! test parses the text with [`parse_grammar`], as the spec calls the reference parser, and reads
//! the model.
//!
//! `getStates().get(id)` is [`find_state`], and `not.toBeUndefined()` is `is_some`. A composite's
//! `doc[i]` is [`doc_state`], and a state's `description` is [`description_text`].

use ferrigura_model::state::{Description, StateDiagram, StateEntry, StateStmt, Statement};
use ferrigura_syntax::state::parse_grammar;

/// The message of the grammar for a state name of more than one word.
const SINGLE_WORD_ERROR: &str = "Error: State name must be a single word.";

/// The state with the given id, if the diagram has one.
fn find_state<'a>(diagram: &'a StateDiagram, id: &str) -> Option<&'a StateEntry> {
    diagram.states.iter().find(|state| state.id == id)
}

/// The state with the given id. Panics with the id when there is none.
fn state<'a>(diagram: &'a StateDiagram, id: &str) -> &'a StateEntry {
    find_state(diagram, id).unwrap_or_else(|| panic!("no state {id}"))
}

/// The descriptions of a state, joined with a space as the spec joins them.
fn descriptions(diagram: &StateDiagram, id: &str) -> String {
    state(diagram, id).descriptions.join(" ")
}

/// The state statement at `index` of a composite's document. Panics when it is not a state.
fn doc_state(entry: &StateEntry, index: usize) -> &StateStmt {
    match entry.doc.as_ref().and_then(|doc| doc.get(index)) {
        Some(Statement::State(stmt)) => stmt,
        other => panic!("doc[{index}] of {} is not a state: {other:?}", entry.id),
    }
}

/// The text of a statement's description, when it has a single one.
fn description_text(stmt: &StateStmt) -> Option<&str> {
    match &stmt.description {
        Some(Description::Text(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// The text of the note of a state, when the state has one.
fn note_text<'a>(diagram: &'a StateDiagram, id: &str) -> Option<&'a str> {
    find_state(diagram, id)
        .and_then(|state| state.note.as_ref())
        .map(|note| note.text.as_str())
}

/// Parses the text, which the spec expects to parse.
fn parse_ok(text: &str) -> StateDiagram {
    parse_grammar(text).expect("parses")
}

/// A state named `prop` is in the diagram after `[*] --> prop` and `prop --> [*]`.
fn allows_state_name(prop: &str) {
    let diagram = parse_ok(&format!(
        "\nstateDiagram-v2\n[*] --> {prop}\n{prop} --> [*]"
    ));
    assert!(find_state(&diagram, prop).is_some());
}

#[test]
fn should_only_accept_1_word() {
    let diagram = parse_ok("stateDiagram-v2\n        state valid { X }");
    assert!(find_state(&diagram, "valid").is_some());
}

#[test]
fn should_throw_error_with_2_words() {
    let error =
        parse_grammar("stateDiagram-v2\n        state invalid syntax { Y }").expect_err("fails");
    assert!(
        error.message.contains(SINGLE_WORD_ERROR),
        "{}",
        error.message
    );
}

#[test]
fn should_also_throw_with_more_than_2_words() {
    let error =
        parse_grammar("stateDiagram-v2\n        state invalid syntax with more than 2 words { Z }")
            .expect_err("fails");
    assert!(
        error.message.contains(SINGLE_WORD_ERROR),
        "{}",
        error.message
    );
}

#[test]
fn stateid_as_some_name() {
    let diagram = parse_ok("stateDiagram-v2\n        state \"Small State 1\" as namedState1");
    assert!(find_state(&diagram, "namedState1").is_some());
    assert_eq!(descriptions(&diagram, "namedState1"), "Small State 1");
}

#[test]
fn space_before_and_after_the_colon() {
    let diagram = parse_ok("stateDiagram-v2\n        namedState1 : Small State 1");
    assert!(find_state(&diagram, "namedState1").is_some());
    assert_eq!(descriptions(&diagram, "namedState1"), "Small State 1");
}

#[test]
fn no_spaces_before_and_after_the_colon() {
    let diagram = parse_ok("stateDiagram-v2\n        namedState1:Small State 1");
    assert!(find_state(&diagram, "namedState1").is_some());
    assert_eq!(descriptions(&diagram, "namedState1"), "Small State 1");
}

#[test]
fn assemble_assemblies_state_assemble_state_assemblies() {
    let diagram = parse_ok(
        r#"stateDiagram-v2
      assemble
      assemblies
      state assemble
      state assemblies
      "#,
    );
    assert!(find_state(&diagram, "assemble").is_some());
    assert!(find_state(&diagram, "assemblies").is_some());
}

#[test]
fn state_as_as_as() {
    let diagram = parse_ok(
        r#"stateDiagram-v2
      state "as" as as
      "#,
    );
    assert!(find_state(&diagram, "as").is_some());
    assert_eq!(descriptions(&diagram, "as"), "as");
}

#[test]
fn state_group_name_as_stateidentifier() {
    let diagram = parse_ok(
        r#"stateDiagram-v2
        state "Small State 1" as namedState1
        %% Notice that this is named "Big State 1" with an "as"
        state "Big State 1" as bigState1 {
            bigState1InternalState
        }
        namedState1 --> bigState1: should point to \nBig State 1 container

        state "Small State 2" as namedState2
        %% Notice that bigState2 does not have a name; no "as"
        state bigState2 {
            bigState2InternalState
        }
        namedState2 --> bigState2: should point to \nbigState2 container"#,
    );

    assert!(find_state(&diagram, "namedState1").is_some());
    assert!(find_state(&diagram, "bigState1").is_some());
    assert_eq!(
        doc_state(state(&diagram, "bigState1"), 0).id,
        "bigState1InternalState"
    );
    assert!(find_state(&diagram, "namedState2").is_some());
    assert!(find_state(&diagram, "bigState2").is_some());
    assert_eq!(
        doc_state(state(&diagram, "bigState2"), 0).id,
        "bigState2InternalState"
    );
    assert_eq!(diagram.relations[0].id1, "namedState1");
    assert_eq!(diagram.relations[0].id2, "bigState1");
    assert_eq!(diagram.relations[1].id1, "namedState2");
    assert_eq!(diagram.relations[1].id2, "bigState2");
}

#[test]
fn group_has_a_state_with_stateid_as_state_name_and_state2id_another_state_name() {
    let diagram = parse_ok(
        r#"stateDiagram-v2
        state "Big State 1" as bigState1 {
            state "inner state 1" as inner1
            inner2: inner state 2
            inner1 --> inner2
        }"#,
    );

    let big = state(&diagram, "bigState1");
    assert_eq!(doc_state(big, 0).id, "inner1");
    assert_eq!(description_text(doc_state(big, 0)), Some("inner state 1"));
    assert_eq!(doc_state(big, 1).id, "inner2");
    assert_eq!(description_text(doc_state(big, 1)), Some("inner state 2"));
}

#[test]
fn should_allow_a_colon_inside_transition_text() {
    let diagram =
        parse_ok("stateDiagram-v2\n        locked --> pending : recoverable error (ex: timeout)");

    assert_eq!(diagram.relations.len(), 1);
    assert_eq!(diagram.relations[0].id1, "locked");
    assert_eq!(diagram.relations[0].id2, "pending");
    assert_eq!(
        diagram.relations[0].relation_title,
        "recoverable error (ex: timeout)"
    );
}

#[test]
fn should_allow_multiple_colons_inside_transition_text() {
    let diagram = parse_ok("stateDiagram-v2\n        A --> B : info: key: value");

    assert_eq!(diagram.relations.len(), 1);
    assert_eq!(diagram.relations[0].relation_title, "info: key: value");
}

#[test]
fn should_allow_a_colon_inside_state_description_text() {
    let diagram = parse_ok("stateDiagram-v2\n        myState : status: active");

    assert!(find_state(&diagram, "myState").is_some());
    assert_eq!(descriptions(&diagram, "myState"), "status: active");
}

#[test]
fn should_allow_proto_as_a_state_name() {
    allows_state_name("__proto__");
}

#[test]
fn should_allow_constructor_as_a_state_name() {
    allows_state_name("constructor");
}

#[test]
fn should_not_terminate_a_note_when_send_note_appears_within_the_note_text() {
    let diagram = parse_ok(
        r#"stateDiagram-v2
      State1
      note right of State1
        this sentence contains send note inside the note text
      end note
      State1 --> State2"#,
    );

    assert_eq!(diagram.relations.len(), 1);
    assert_eq!(diagram.relations[0].id1, "State1");
    assert_eq!(diagram.relations[0].id2, "State2");
    let text = note_text(&diagram, "State1").expect("State1 has a note");
    assert!(text.contains("send note inside the note text"), "{text}");
}

#[test]
fn should_not_treat_end_note_as_a_closing_keyword_when_it_is_part_of_the_note_text() {
    let diagram = parse_ok(
        r#"stateDiagram-v2
      State1
      note right of State1
        this sentence contains end note as part of the note text
      end note
      State1 --> State2"#,
    );

    assert_eq!(diagram.relations.len(), 1);
    assert_eq!(diagram.relations[0].id1, "State1");
    assert_eq!(diagram.relations[0].id2, "State2");
    let text = note_text(&diagram, "State1").expect("State1 has a note");
    assert!(text.contains("end note as part of the note text"), "{text}");
}
