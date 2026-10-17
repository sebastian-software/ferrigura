//! Port of `packages/parser/tests/eventmodeling.test.ts`: the syntax tree of the event modeling
//! grammar, and the validator registration. Test names follow the reference's `it` titles in
//! snake_case. The reference's `describe` groups are comments.
//!
//! The reference reads Langium's AST through its `parse()` and asserts that the lexer reports no
//! errors. Here [`parse_ast`] returns the syntax tree, or the first lexer or grammar error, so the
//! reference's `assert(lexerErrors.length === 0)` becomes `parse_ast(...).expect(...)`. The
//! reference's `result.value.frames.length` becomes `ast.frames.len()`, and
//! `frame.$type === 'EmResetFrame'` becomes `frame.kind == FrameKind::Reset`.
//!
//! The registration test is ignored: Langium's validation registry is not in the Rust port, and
//! `parse` does not run the source checks (`docs-port/eventmodeling/NOTES.md`, section 2.5). The
//! checks are tested on their own in `event_modeling_validator.rs`.

use ferrigura_syntax::event_modeling::ast::{EventModel, FrameKind};
use ferrigura_syntax::event_modeling::parse_ast;
use ferrigura_syntax::event_modeling::validator::check_source_frame_types;

fn parsed(text: &str) -> EventModel {
    parse_ast(text).expect("the event modeling text parses")
}

#[test]
fn should_parse_complex_model() {
    let ast = parsed(
        r#"eventmodeling
tf 01 cmd UpdateCartCommand
tf 02 evt CartUpdatedEvent ->> 01 `jsobj`{ a: b }
tf 03 rmo CartItemsReadModel ->> 02 [[CartItemsReadModel03]]
tf 04 evt ProductDescriptionUpdatedEvent ->> 01 `jsobj`{ a: { c: d } }
tf 05 evt ProductTitleUpdatedEvent ->> 01 { "a": { "c": true } }
tf 06 evt ProductCountIncrementedEvent ->> 01 `json`" { "a": { "c": true } } "

data CartItemsReadModel03 {
  { a: b }
}

data NotAssignedData02 `jsobj` {
  { a: {
    d: true
  }}
}

data AnotherNotAssignedData06 {
  a: 'abc'
}

note 02 `md` {
    # head 1
    this is markdown note
}

note 05 {
  This is whatever <b>you</b> want
  On multiple lines
}

gwt 01
  given
    evt CartUpdatedEvent
    evt CartUpdatedEvent
  when
    evt ProductDescriptionUpdatedEvent
    evt ProductTitleUpdatedEvent
  then
    evt ProductTitleUpdatedEvent


gwt 03
  given
    evt CartUpdatedEvent
    evt ProductTitleUpdatedEvent
  then
    evt ProductTitleUpdatedEvent
    evt CartUpdatedEvent
"#,
    );
    assert_eq!(ast.frames.len(), 6);
    assert_eq!(ast.data_entities.len(), 3);
    assert_eq!(ast.note_entities.len(), 2);
    assert_eq!(ast.gwt_entities.len(), 2);
}

#[test]
fn should_parse_simple_model_with_full_syntax() {
    let ast = parsed("eventmodeling\ntimeframe 01 event Start\n\n  ");
    assert_eq!(ast.frames.len(), 1);
    let frame = &ast.frames[0];
    assert_eq!(frame.name, "01");
    assert_eq!(frame.model_entity_type, "event");
    assert_eq!(frame.entity_identifier, "Start");
}

#[test]
fn should_parse_qualified_names_in_model() {
    let ast = parsed(
        r#"eventmodeling

timeframe 02 ui UI
tf 01 evt Product.PriceChanged
tf 03 evt Cart.ItemAdded

  "#,
    );
    assert_eq!(ast.frames.len(), 3);
    let frame = &ast.frames[1];
    assert_eq!(frame.name, "01");
    assert_eq!(frame.model_entity_type, "evt");
    assert_eq!(frame.entity_identifier, "Product.PriceChanged");
}

#[test]
fn should_parse_both_types_of_frames_in_model() {
    let ast = parsed(
        r#"eventmodeling

tf 02 ui UI
resetframe 01 evt Product.PriceChanged
tf 03 evt Cart.ItemAdded

  "#,
    );
    assert_eq!(ast.frames.len(), 3);
    let frame = &ast.frames[1];
    assert_eq!(frame.kind, FrameKind::Reset);
    assert_eq!(frame.name, "01");
    assert_eq!(frame.model_entity_type, "evt");
    assert_eq!(frame.entity_identifier, "Product.PriceChanged");
}

#[test]
#[ignore = "port: Langium's validation registry is not in the Rust port, and parse does not call the checks (docs-port/eventmodeling/NOTES.md, section 2.5)"]
fn registers_checks_against_the_validation_registry() {
    // The reference counts the checks it registers with the registry. The port has no registry:
    // the check is a function, so this asserts what the check reports for a rule it enforces.
    assert!(!check_source_frame_types("evt", &["rmo"]).is_empty());
}

#[test]
fn validation_is_not_run_by_parse() {
    // An evt fed by a ui breaks the source rule, and the reference's parse() does not run the
    // check either, so the text parses.
    assert!(parse_ast("eventmodeling\ntf 01 ui A\ntf 02 evt B ->> 01").is_ok());
}

#[test]
fn should_parse_multiple_source_frames_model() {
    let ast = parsed(
        r#"eventmodeling
tf 01 evt Start
tf 02 evt End
rf 03 readmodel ReadModel01 ->> 01 ->> 02 { a: true }
rf 04 rmo ReadModel02 ->> 01 ->> 02
  "#,
    );
    assert_eq!(ast.frames.len(), 4);

    let frame = &ast.frames[2];
    assert_eq!(frame.name, "03");
    assert_eq!(frame.model_entity_type, "readmodel");
    assert_eq!(frame.source_frames.len(), 2);

    let frame = &ast.frames[3];
    assert_eq!(frame.name, "04");
    assert_eq!(frame.model_entity_type, "rmo");
    assert_eq!(frame.source_frames.len(), 2);
}
