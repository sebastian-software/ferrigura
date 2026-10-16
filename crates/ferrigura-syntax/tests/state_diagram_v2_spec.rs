//! Port of `packages/mermaid/src/diagrams/state/stateDiagram-v2.spec.js` (31 tests). Test names follow
//! the spec's titles in snake_case; a title that repeats after normalising gets `_2`, `_3`, … in
//! source order. The arrow-direction test is commented out in the spec, so it is not ported.
//!
//! Each test parses the spec's text with `parse_grammar`, which the reference's `parser.parse` takes
//! as it is. The DB reads map to the model:
//! - `getState(id)` and `getStates().get(id)` are the state entry with that id;
//! - the inline snapshots of the notes are the notes' `position` and `text`, compared as a whole;
//! - `getRelations()` is `relations`, `getAccTitle()` is `acc_title`, `getAccDescription()` is
//!   `acc_descr` and `getDirection()` is `direction`.
//!
//! No expectation reads `getData()`, so no test is ignored.

use ferrigura_model::state::{Note, Relation, StateDiagram, StateEntry, Statement};
use ferrigura_syntax::state::parse_grammar;

/// `DEFAULT_DIAGRAM_DIRECTION` of `stateCommon.ts`.
const DEFAULT_DIAGRAM_DIRECTION: &str = "TB";

/// The state entry with the given id. Panics with the id when there is none.
fn state<'a>(diagram: &'a StateDiagram, id: &str) -> &'a StateEntry {
    diagram
        .states
        .iter()
        .find(|state| state.id == id)
        .unwrap_or_else(|| panic!("no state {id}"))
}

/// The note of the state with the given id. Panics when there is none.
fn note<'a>(diagram: &'a StateDiagram, id: &str) -> &'a Note {
    state(diagram, id)
        .note
        .as_ref()
        .unwrap_or_else(|| panic!("no note on {id}"))
}

/// The relation from `id1` to `id2`. Panics when there is none.
fn relation<'a>(diagram: &'a StateDiagram, id1: &str, id2: &str) -> &'a Relation {
    diagram
        .relations
        .iter()
        .find(|relation| relation.id1 == id1 && relation.id2 == id2)
        .unwrap_or_else(|| panic!("no relation from {id1} to {id2}"))
}

#[test]
fn super_simple() {
    let text = r#"
      stateDiagram-v2
      [*] --> State1
      State1 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn simple() {
    let text = r#"stateDiagram-v2

      State1 : this is another string
      [*] --> State1
      State1 --> [*]
      "#;

    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(diagram.acc_descr, "");
}

#[test]
fn simple_with_accdescription() {
    let text = r#"stateDiagram-v2

      accDescr: a simple description of the diagram
      State1 : this is another string
      [*] --> State1
      State1 --> [*]
      "#;

    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(diagram.acc_descr, "a simple description of the diagram");
}

#[test]
fn simple_with_title() {
    let text = r#"stateDiagram-v2

      accTitle: a simple title of the diagram
      State1 : this is another string
      [*] --> State1
      State1 --> [*]
      "#;

    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(diagram.acc_title, "a simple title of the diagram");
}

#[test]
fn should_handle_relation_definitions() {
    let text = r#"stateDiagram-v2

      [*] --> State1
      State1 --> [*]
      State1 : this is a string
      State1 : this is another string

      State1 --> State2
      State2 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn hide_empty_description() {
    let text = r#"stateDiagram-v2

      hide empty description
      [*] --> State1
      State1 --> [*]
      State1 : this is a string
      State1 : this is another string

      State1 --> State2
      State2 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names() {
    let text = r#"stateDiagram-v2
      assemble
      state assemble
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names_1() {
    let text = r#"stateDiagram-v2
      assemble
      state assemble
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names_2() {
    let text = r#"stateDiagram-v2
      assemblies
      state assemblies
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names_3() {
    let text = r#"stateDiagram-v2
      state "as" as as
      "#;

    parse_grammar(text).expect("parses");
}

// The spec's `describe('relationship labels')`.
#[test]
fn simple_states_with_labels() {
    let text = r#"
          stateDiagram-v2
          [*] --> State1
          State1 --> State2 : Transition 1
          State1 --> State3 : Transition 2
          State1 --> State4 : Transition 3
          State1 --> [*]
        "#;

    let diagram = parse_grammar(text).expect("parses");

    assert_eq!(
        relation(&diagram, "State1", "State2").relation_title,
        "Transition 1"
    );
    assert_eq!(
        relation(&diagram, "State1", "State3").relation_title,
        "Transition 2"
    );
    assert_eq!(
        relation(&diagram, "State1", "State4").relation_title,
        "Transition 3"
    );
}

#[test]
fn scale() {
    let text = r#"stateDiagram-v2

      scale 350 width
      [*] --> State1
      State1 --> [*]
      State1 : this is a string with - in it
      State1 : this is another string

      State1 --> State2
      State2 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn description_after_second_state() {
    let text = r#"stateDiagram-v2

      scale 350 width
      [*] --> State1 : This is the description with - in it
      State1 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn shall_handle_descriptions_including_minus_signs() {
    let text = r#"stateDiagram-v2

      scale 350 width
      [*] --> State1 : This is the description +-!
      State1 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_state_statements() {
    let text = r#"stateDiagram-v2

      state Configuring {
        [*] --> NewValueSelection
        NewValueSelection --> NewValuePreview : EvNewValue
        NewValuePreview --> NewValueSelection : EvNewValueRejected
        NewValuePreview --> NewValueSelection : EvNewValueSaved1
      }
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_recursive_state_definitions() {
    let text = r#"stateDiagram-v2

      state Configuring {
        [*] --> NewValueSelection
        NewValueSelection --> NewValuePreview : EvNewValue
        NewValuePreview --> NewValueSelection : EvNewValueRejected
        NewValuePreview --> NewValueSelection : EvNewValueSaved

        state NewValuePreview {
          State1 --> State2
        }
      }
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_multiple_recursive_state_definitions() {
    let text = r#"stateDiagram-v2

      scale 350 width
      [*] --> NotShooting

      state NotShooting {
        [*] --> Idle
        Idle --> Configuring : EvConfig
        Configuring --> Idle : EvConfig
      }

      state Configuring {
        [*] --> NewValueSelection
        NewValueSelection --> NewValuePreview : EvNewValue
        NewValuePreview --> NewValueSelection : EvNewValueRejected
        NewValuePreview --> NewValueSelection : EvNewValueSaved

        state NewValuePreview {
          State1 --> State2
        }
      }
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_state_definitions_with_separation_of_id() {
    let text = r#"stateDiagram-v2

      state "Long state description" as state1
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_state_definitions_with_separation_of_id_2() {
    let text = r#"stateDiagram-v2
      state "Not Shooting State" as NotShooting {
        state "Idle mode" as Idle
        state "Configuring mode" as Configuring
        [*] --> Idle
        Idle --> Configuring : EvConfig
        Configuring --> Idle : EvConfig
      }
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_state_definition_with_quotes() {
    // The spec's `\n` inside the quoted name is a line break in the text.
    let text = r#"stateDiagram-v2

      scale 600 width

      [*] --> State1
      State1 --> State2 : Succeeded
      State1 --> [*] : Aborted
      State2 --> State3 : Succeeded
      State2 --> [*] : Aborted
      state State3 {
        state "Accumulate Enough Data
Long State Name" as long1
        long1 : Just a test
        [*] --> long1
        long1 --> long1 : New Data
        long1 --> ProcessData : Enough Data
      }
      State3 --> State3 : Failed
      State3 --> [*] : Succeeded / Save Result
      State3 --> [*] : Aborted
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_fork_statements() {
    let text = r#"stateDiagram-v2

      state fork_state <<fork>>
      [*] --> fork_state
      fork_state --> State2
      fork_state --> State3

      state join_state <<join>>
      State2 --> join_state
      State3 --> join_state
      join_state --> State4
      State4 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_concurrent_state() {
    let text = r#"stateDiagram-v2

      [*] --> Active

      state Active {
        [*] --> NumLockOff
        NumLockOff --> NumLockOn : EvNumLockPressed
        NumLockOn --> NumLockOff : EvNumLockPressed
        --
        [*] --> CapsLockOff
        CapsLockOff --> CapsLockOn : EvCapsLockPressed
        CapsLockOn --> CapsLockOff : EvCapsLockPressed
        --
        [*] --> ScrollLockOff
        ScrollLockOff --> ScrollLockOn : EvCapsLockPressed
        ScrollLockOn --> ScrollLockOff : EvCapsLockPressed
      }
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_concurrent_state_2() {
    let text = r#"stateDiagram-v2

      [*] --> Active

      state Active {
        [*] --> NumLockOff
        --
        [*] --> CapsLockOff
        --
        [*] --> ScrollLockOff
      }
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_note_statements() {
    let text = r#"stateDiagram-v2

      [*] --> Active
      Active --> Inactive

      note left of Active : this is a short<br/>note

      note right of Inactive
      A note can also
      be defined on
      several lines
      end note
      "#;

    let diagram = parse_grammar(text).expect("parses");

    assert_eq!(
        note(&diagram, "Active"),
        &Note {
            position: "left of".to_owned(),
            text: "this is a short<br>note".to_owned(),
        }
    );
    assert_eq!(
        note(&diagram, "Inactive"),
        &Note {
            position: "right of".to_owned(),
            text: "A note can also\n      be defined on\n      several lines".to_owned(),
        }
    );
}

#[test]
fn should_handle_multiline_notes_with_different_line_breaks() {
    let text = r#"stateDiagram-v2
      State1
      note right of State1
      Line1<br>Line2<br/>Line3<br />Line4<br	/>Line5
      end note
      "#;

    let diagram = parse_grammar(text).expect("parses");

    assert_eq!(
        note(&diagram, "State1"),
        &Note {
            position: "right of".to_owned(),
            text: "Line1<br>Line2<br>Line3<br>Line4<br>Line5".to_owned(),
        }
    );
}

#[test]
fn should_handle_floating_notes() {
    let text = r#"stateDiagram-v2
      foo: bar
      note "This is a floating note" as N1
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_floating_notes_2() {
    let text = r#"stateDiagram-v2
      state foo
      note "This is a floating note" as N1
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_notes_for_composite_nested_states() {
    let text = r#"stateDiagram-v2
      [*] --> NotShooting

      state "Not Shooting State" as NotShooting {
        state "Idle mode" as Idle
        state "Configuring mode" as Configuring
        [*] --> Idle
        Idle --> Configuring : EvConfig
        Configuring --> Idle : EvConfig
      }

      note right of NotShooting : This is a note on a composite state
      "#;

    let diagram = parse_grammar(text).expect("parses");

    assert_eq!(
        note(&diagram, "NotShooting"),
        &Note {
            position: "right of".to_owned(),
            text: "This is a note on a composite state".to_owned(),
        }
    );
}

#[test]
fn a_composite_state_should_be_able_to_link_to_itself() {
    let text = r#"
          stateDiagram-v2
            state Active {
              Idle
            }
            Inactive --> Idle: ACT
            Active --> Active: LOG
        "#;

    let diagram = parse_grammar(text).expect("parses");

    let doc = state(&diagram, "Active")
        .doc
        .as_ref()
        .expect("Active has a document");
    match &doc[0] {
        Statement::State(stmt) => assert_eq!(stmt.id, "Idle"),
        other => panic!("expected a state statement, got {other:?}"),
    }

    assert_eq!(relation(&diagram, "Inactive", "Idle").relation_title, "ACT");
    assert_eq!(relation(&diagram, "Active", "Active").relation_title, "LOG");
}

#[test]
fn should_check_default_diagram_direction() {
    let text = r#"
        stateDiagram
          [*] --> Still
          Still --> [*]
        "#;

    let diagram = parse_grammar(text).expect("parses");

    // checking default direction if no direction is specified
    assert_eq!(diagram.direction, DEFAULT_DIAGRAM_DIRECTION);
}

#[test]
fn retrieve_the_diagram_direction_correctly() {
    let text = r#"
        stateDiagram
          direction LR
          [*] --> Still
          Still --> [*]
        "#;

    let diagram = parse_grammar(text).expect("parses");

    assert_eq!(diagram.direction, "LR");
}
