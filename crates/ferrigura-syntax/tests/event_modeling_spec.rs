//! Port of `packages/mermaid/src/diagrams/eventmodeling/eventmodeling.spec.ts`. Test names follow
//! the spec's titles in snake_case.
//!
//! The reference's `not.toThrow()` becomes `parse(…).is_ok()`, and its DB checks read the model.
//! The reference's `getState()` throws after a failed parse; the port has no state to read after
//! an error, so that check becomes the error itself. The spec's `getBBox` stub has no counterpart:
//! the model holds no sizes.

use ferrigura_model::event_modeling::{EventModelingDiagram, Relation};
use ferrigura_syntax::event_modeling::parse;

fn parses(text: &str) -> EventModelingDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

#[test]
fn should_handle_a_simple_eventmodeling_definition() {
    let text = "eventmodeling
    tf 01 ui UI
    tf 02 cmd RunAction
    tf 03 evt ActionExecuted";
    assert!(parse(text).is_ok());
}

#[test]
fn should_handle_data_inline_in_frames() {
    let text = "eventmodeling
    tf 01 cmd AddItem { productId: 7 }
    tf 02 evt ItemAdded { productId: 7 }";
    let model = parses(text);
    assert_eq!(
        model.frames[0].data_inline.as_deref(),
        Some("{ productId: 7 }")
    );
    assert_eq!(
        model.frames[1].data_inline.as_deref(),
        Some("{ productId: 7 }")
    );
}

#[test]
fn should_handle_data_block_references() {
    let text = "eventmodeling
    tf 01 cmd AddItem
    tf 02 evt ItemAdded [[ItemAddedData]]

data ItemAddedData
{
  productId: 7
}";
    let model = parses(text);
    assert_eq!(
        model.frames[1].data_reference.as_deref(),
        Some("ItemAddedData")
    );
    assert_eq!(model.data_entities[0].name, "ItemAddedData");
}

#[test]
fn should_handle_qualified_names() {
    let text = "eventmodeling
    tf 01 ui CartUI
    tf 02 cmd Inventory.AddItem
    tf 03 evt Inventory.ItemAdded";
    let model = parses(text);
    assert_eq!(model.frames[1].namespace.as_deref(), Some("Inventory"));
    assert_eq!(model.frames[1].entity_name, "AddItem");
}

#[test]
fn should_handle_multiple_source_frames() {
    let text = "eventmodeling
    tf 01 ui CartUI
    tf 02 cmd AddItem
    tf 03 cmd RemoveItem
    tf 04 evt ItemChanged ->> 02 ->> 03";
    let model = parses(text);
    assert_eq!(model.frames[3].source_frames, ["02", "03"]);
}

#[test]
fn should_handle_reset_frames() {
    let text = "eventmodeling
    rf 01 ui CartUI
    rf 02 cmd AddItem
    rf 03 evt ItemAdded";
    assert!(parse(text).is_ok());
}

#[test]
fn should_reject_a_frame_id_reused_by_a_reset_frame() {
    assert!(
        parse(
            "eventmodeling
      tf 09 ui PreviousDiagram"
        )
        .is_ok()
    );

    let error = parse(
        "eventmodeling
        tf 01 ui UI
        tf 02 cmd Command
        rf 01 evt Event",
    )
    .expect_err("a reused frame id should be rejected");
    assert_eq!(error.line, 4);
    assert!(error.message.contains("\"01\""), "{}", error.message);
}

#[test]
fn should_report_the_original_line_number_when_comments_precede_a_duplicate_frame() {
    let error = ferrigura_syntax::parse(
        "eventmodeling
        tf 01 ui UI
        %% This comment occupies a source line.
        rf 01 evt Event",
    )
    .expect_err("a reused frame id should be rejected");
    assert_eq!(error.line, 4);
}

#[test]
fn should_keep_explicit_sources_on_a_reset_frame_without_inferring_the_previous_frame() {
    let model = parses(
        "eventmodeling
      tf 01 ui UI
      tf 02 cmd Command
      tf 03 evt Event
      rf 04 rmo ReadModel ->> 03
      rf 05 ui OtherUI",
    );
    // Frames are numbered from 0 in the text: `03` is 2, `04` is 3, `05` is 4.
    let relation = |source_frame, target_frame| Relation {
        source_frame,
        target_frame,
    };
    assert!(model.relations.contains(&relation(2, 3)));
    assert!(!model.relations.contains(&relation(3, 4)));
}

#[test]
fn should_handle_all_entity_types() {
    let text = "eventmodeling
    tf 01 ui UI
    tf 02 ui UI2
    tf 03 cmd Command
    tf 04 command Command2
    tf 05 evt Event
    tf 06 event Event2
    tf 07 pcr Processor
    tf 08 processor Processor2
    tf 09 rmo ReadModel
    tf 10 readmodel ReadModel2";
    assert!(parse(text).is_ok());
}
