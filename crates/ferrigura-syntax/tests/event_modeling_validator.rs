//! Port of the 25 validator tests in `packages/parser/tests/eventmodeling.test.ts`: the `it`s under
//! `describe('Validation')` that call `EventModelingValidator.checkSourceFrameTypes`. Test names
//! follow the reference's `it` titles in snake_case.
//!
//! The reference's `collectErrors(frame, sources)` builds a frame whose sources are the given
//! frames, and collects the messages. The rule reads only the entity type of each frame, so the
//! port passes the types: [`check_source_frame_types`] returns one message per source that the
//! target may not take, in the order of the sources. The reference's `toHaveLength(n)` becomes a
//! length check, and `toContain` becomes a substring check on the message.

use ferrigura_syntax::event_modeling::validator::check_source_frame_types;

#[test]
fn should_allow_evt_sourced_from_cmd() {
    let errors = check_source_frame_types("evt", &["cmd"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_event_sourced_from_command() {
    let errors = check_source_frame_types("event", &["command"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_reject_evt_sourced_from_rmo() {
    let errors = check_source_frame_types("evt", &["rmo"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("event"), "{errors:?}");
    assert!(errors[0].contains("command"), "{errors:?}");
}

#[test]
fn should_reject_evt_sourced_from_pcr() {
    let errors = check_source_frame_types("evt", &["pcr"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn should_allow_cmd_sourced_from_ui() {
    let errors = check_source_frame_types("cmd", &["ui"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_command_sourced_from_ui() {
    let errors = check_source_frame_types("command", &["ui"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_cmd_sourced_from_pcr() {
    let errors = check_source_frame_types("cmd", &["pcr"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_command_sourced_from_processor() {
    let errors = check_source_frame_types("command", &["processor"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_reject_cmd_sourced_from_cmd() {
    let errors = check_source_frame_types("cmd", &["cmd"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("command"), "{errors:?}");
    assert!(errors[0].contains("ui or processor"), "{errors:?}");
}

#[test]
fn should_reject_cmd_sourced_from_evt() {
    let errors = check_source_frame_types("cmd", &["evt"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("command"), "{errors:?}");
    assert!(errors[0].contains("ui or processor"), "{errors:?}");
}

#[test]
fn should_reject_ui_sourced_from_evt() {
    let errors = check_source_frame_types("ui", &["evt"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn should_allow_ui_sourced_from_rmo() {
    let errors = check_source_frame_types("ui", &["rmo"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_ui_sourced_from_readmodel() {
    let errors = check_source_frame_types("ui", &["readmodel"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_reject_ui_sourced_from_cmd() {
    let errors = check_source_frame_types("ui", &["cmd"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("ui"), "{errors:?}");
    assert!(errors[0].contains("read model"), "{errors:?}");
}

#[test]
fn should_reject_cmd_sourced_from_rmo() {
    let errors = check_source_frame_types("cmd", &["rmo"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("command"), "{errors:?}");
    assert!(errors[0].contains("ui or processor"), "{errors:?}");
}

#[test]
fn should_allow_rmo_sourced_from_evt() {
    let errors = check_source_frame_types("rmo", &["evt"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_readmodel_sourced_from_event() {
    let errors = check_source_frame_types("readmodel", &["event"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_pcr_sourced_from_rmo() {
    let errors = check_source_frame_types("pcr", &["rmo"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_allow_processor_sourced_from_readmodel() {
    let errors = check_source_frame_types("processor", &["readmodel"]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn should_reject_rmo_sourced_directly_from_pcr() {
    let errors = check_source_frame_types("rmo", &["pcr"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("read model"), "{errors:?}");
    assert!(errors[0].contains("event"), "{errors:?}");
}

#[test]
fn should_reject_pcr_sourced_directly_from_evt() {
    let errors = check_source_frame_types("pcr", &["evt"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("processor"), "{errors:?}");
    assert!(errors[0].contains("read model"), "{errors:?}");
}

#[test]
fn should_reject_pcr_sourced_from_cmd() {
    let errors = check_source_frame_types("pcr", &["cmd"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn should_reject_rmo_sourced_from_ui() {
    let errors = check_source_frame_types("rmo", &["ui"]);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn should_report_an_error_for_each_invalid_source() {
    let errors = check_source_frame_types("pcr", &["evt", "cmd"]);
    assert_eq!(errors.len(), 2, "{errors:?}");
}

#[test]
fn should_not_validate_frames_with_no_sources() {
    let errors = check_source_frame_types("rmo", &[]);
    assert!(errors.is_empty(), "{errors:?}");
}
