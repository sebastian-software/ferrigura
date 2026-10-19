//! The reference's mermaid API tests that are not ported, ignored with their reasons. Each reference `it` is
//! one test with a panicking body, so the count of what is not ported stays visible.
//!
//! The renderer's colour slots, its stylesheet, its container colour indices and its `draw` are ported to
//! `crates/ferrigura-render/tests/agentflow_colour_slots_spec.rs` and `agentflow_render_spec.rs`. The source
//! positions and the element mappings are not ported (open question Q-g). The diagnostic lifecycle reads the
//! diagnostics after `getData()`; the diagnostics themselves are live in `agentflow_diagnostics_spec.rs`.
//!
//! Sources: `agentflow-diagnostics-lifecycle.spec.ts` (4), `agentflow-render-positions.spec.ts` (4),
//! `agentflow-max-text-size.spec.ts` (5).

// agentflow-diagnostics-lifecycle.spec.ts

#[test]
#[ignore = "mermaid API: the diagnostics after getData(), the layout read that the renderer drives, are not ported"]
fn reports_an_unsupported_shape_after_get_data_without_rendering() {
    panic!(
        "mermaid API: the diagnostics after getData(), the layout read that the renderer drives, are not ported"
    );
}

#[test]
#[ignore = "mermaid API: the diagnostics after getData(), the layout read that the renderer drives, are not ported"]
fn reports_a_removed_shape_as_an_error_after_get_data() {
    panic!(
        "mermaid API: the diagnostics after getData(), the layout read that the renderer drives, are not ported"
    );
}

#[test]
#[ignore = "mermaid API: repeated getData() calls on the store of the DB are not ported; the diagnostics are a pure reading of the model"]
fn does_not_accumulate_duplicates_when_get_data_is_called_more_than_once() {
    panic!("mermaid API: repeated getData() calls on the store of the DB are not ported");
}

#[test]
#[ignore = "mermaid API: the position of a diagnostic is an element mapping, not ported (open question Q-g)"]
fn reports_the_position_of_the_offending_node_when_one_was_captured() {
    panic!(
        "mermaid API: the position of a diagnostic is an element mapping, not ported (open question Q-g)"
    );
}

// agentflow-render-positions.spec.ts

#[test]
#[ignore = "mermaid API: source positions on the render path are not ported (open question Q-g)"]
fn reports_original_source_positions_through_mermaid_api_render() {
    panic!("mermaid API: source positions on the render path are not ported (open question Q-g)");
}

#[test]
#[ignore = "mermaid API: source positions on the parse path are not ported (open question Q-g)"]
fn reports_the_same_positions_through_mermaid_api_parse() {
    panic!("mermaid API: source positions on the parse path are not ported (open question Q-g)");
}

#[test]
#[ignore = "mermaid API: source positions on the render and parse paths are not ported (open question Q-g)"]
fn parse_and_render_agree_on_the_reported_position() {
    panic!(
        "mermaid API: source positions on the render and parse paths are not ported (open question Q-g)"
    );
}

#[test]
#[ignore = "mermaid API: the diagram registry and its source-position hooks are not ported"]
fn is_opt_in_no_other_registered_diagram_consumes_the_source_position_hooks() {
    panic!("mermaid API: the diagram registry and its source-position hooks are not ported");
}

// agentflow-max-text-size.spec.ts

#[test]
#[ignore = "mermaid API: the maxTextSize fallback on the render path is not ported"]
fn does_not_hand_the_parser_a_comment_padded_source_larger_than_the_cap() {
    panic!("mermaid API: the maxTextSize fallback on the render path is not ported");
}

#[test]
#[ignore = "mermaid API: comment-stripped positions on the render path are not ported (open question Q-g)"]
fn reports_comment_stripped_positions_once_the_cap_forces_the_fallback() {
    panic!(
        "mermaid API: comment-stripped positions on the render path are not ported (open question Q-g)"
    );
}

#[test]
#[ignore = "mermaid API: comment-accurate positions on the render path are not ported (open question Q-g)"]
fn keeps_comment_accurate_positions_when_the_source_is_under_the_cap() {
    panic!(
        "mermaid API: comment-accurate positions on the render path are not ported (open question Q-g)"
    );
}

#[test]
#[ignore = "mermaid API: the maxTextSize fallback on the parse path is not ported (open question Q-g for its positions)"]
fn caps_the_parse_path_too_not_only_render() {
    panic!(
        "mermaid API: the maxTextSize fallback on the parse path is not ported (open question Q-g for its positions)"
    );
}

#[test]
#[ignore = "mermaid API: the log warnings of the render path for a flowchart are not ported"]
fn says_nothing_for_a_comment_heavy_diagram_that_never_parses_comments() {
    panic!("mermaid API: the log warnings of the render path for a flowchart are not ported");
}

// colorSlots.spec.ts: the colour slots of the renderer

// containerOrder.spec.ts: the colour indices of getData

// renderer.spec.ts
