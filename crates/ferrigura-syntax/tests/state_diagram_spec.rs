//! Port of `packages/mermaid/src/diagrams/state/stateDiagram.spec.js` (30 tests: 29 active and the
//! one `it.skip`, which is ignored). Test names follow the spec's titles in snake_case; a title that
//! repeats after normalising gets `_2`, `_3`, … in source order.
//!
//! Each test parses the spec's text with `parse_grammar`, which the reference's `parser.parse` takes
//! as it is. The DB reads map to the model: `getAccDescription()` is `acc_descr`, `getAccTitle()` is
//! `acc_title` and `getLinks()` is `links`.
//!
//! The click test calls the DB directly in the spec. Here the same state and click come from the
//! diagram text, so its link keeps the quotes the grammar keeps.

use ferrigura_syntax::state::parse_grammar;

#[test]
fn super_simple() {
    let text = r#"
      stateDiagram
        [*] --> State1
        State1 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn simple() {
    let text = r#"stateDiagram

          State1 : this is another string
          [*] --> State1
          State1 --> [*]
      "#;

    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(diagram.acc_descr, "");
}

#[test]
fn simple_with_accdescription_accdescr() {
    let text = r#"stateDiagram

          accDescr: a simple description of the diagram
          State1 : this is another string
          [*] --> State1
          State1 --> [*]
      "#;

    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(diagram.acc_descr, "a simple description of the diagram");
}

#[test]
fn simple_with_multiline_accdescription_accdescr() {
    let text = r#"stateDiagram

          accDescr {
             a simple description of the diagram
            using multiple lines
          }
          State1 : this is another string
          [*] --> State1
          State1 --> [*]
      "#;

    let diagram = parse_grammar(text).expect("parses");
    assert_eq!(
        diagram.acc_descr,
        "a simple description of the diagram\nusing multiple lines"
    );
}

#[test]
fn simple_with_title_accdescr() {
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram
      assemble
      state assemble
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names_1() {
    let text = r#"stateDiagram
      assemble
      state assemble
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names_2() {
    let text = r#"stateDiagram
      assemblies
      state assemblies
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn handle_as_in_state_names_3() {
    let text = r#"stateDiagram
      state "as" as as
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn scale() {
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

        scale 350 width
        [*] --> State1 : This is the description with - in it
        State1 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn shall_handle_descriptions_including_minus_signs() {
    let text = r#"stateDiagram

        scale 350 width
        [*] --> State1 : This is the description +-!
        State1 --> [*]
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_state_statements() {
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

        state "Long state description" as state1
        "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_state_definitions_with_separation_of_id_2() {
    let text = r#"stateDiagram
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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

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
    let text = r#"stateDiagram

        [*] --> Active
        Active --> Inactive

        note left of Active : this is a short<br/>note

        note right of Inactive
          A note can also
          be defined on
          several lines
        end note
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_multiline_notes_with_different_line_breaks() {
    let text = r#"stateDiagram
        State1
        note right of State1
          Line1<br>Line2<br/>Line3<br />Line4<br	/>Line5
        end note
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_floating_notes() {
    let text = r#"stateDiagram
        foo: bar
        note "This is a floating note" as N1
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_floating_notes_2() {
    let text = r#"stateDiagram

        state foo
        note "This is a floating note" as N1
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_handle_notes_for_composite_states() {
    let text = r#"stateDiagram

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

    parse_grammar(text).expect("parses");
}

#[test]
#[ignore = "port: skipped upstream: should handle if statements"]
fn should_handle_if_statements() {
    let text = r#"stateDiagram

      [*] --> "Order Submitted"
      if "Payment Accepted" then
        -->[yes] "Pack products"
        --> "Send parcel"
        -right-> (*)
      else
        ->[no] "Send error message"
        -->[Cancel Order] [*]
      endif
        }

        note right of NotShooting : This is a note on a composite state
      "#;

    parse_grammar(text).expect("parses");
}

#[test]
fn should_store_links_from_click_statements_manually_passed_to_extract() {
    // The spec adds S1 with `addState` and passes a click to `extract`. The same state and click
    // come from the text: the state line, then the click line, which ends with a line break.
    let text = "stateDiagram\n  S1\n  click S1 \"https://example.com\" \"Go to Example\"\n";

    let diagram = parse_grammar(text).expect("parses");

    let link = diagram
        .links
        .iter()
        .find(|link| link.id == "S1")
        .expect("S1 has a link");
    // The spec passes the address and tooltip without quotes. The grammar keeps them as written,
    // and the renderer strips them (`replace(/^"+|"+$/g, '')`), so the DB holds the quoted values.
    assert_eq!(link.url, "\"https://example.com\"");
    assert_eq!(link.tooltip, "\"Go to Example\"");
}
