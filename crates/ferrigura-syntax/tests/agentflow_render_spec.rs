//! The reference's renderer and mermaid API tests, ignored with their reasons. Each reference `it` is
//! one test with a panicking body, so the count of what is not ported stays visible.
//!
//! The renderer's colour slots, its container colour indices, its `draw` and the render path of the
//! mermaid API are the renderer's (`colorSlots.ts`, `renderer.ts`, `mermaidAPI`). The source
//! positions and the element mappings are not ported (open question Q-g). The diagnostic lifecycle
//! reads the diagnostics after `getData()`; the diagnostics themselves are live in
//! `agentflow_diagnostics_spec.rs`.
//!
//! Sources: `agentflow-diagnostics-lifecycle.spec.ts` (4), `agentflow-render-positions.spec.ts` (4),
//! `agentflow-max-text-size.spec.ts` (5), `colorSlots.spec.ts` (14), `containerOrder.spec.ts` (3),
//! `renderer.spec.ts` (1).

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

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn tags_a_node_with_the_kind_the_db_reports() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn gives_two_nodes_of_one_kind_the_same_class_wherever_they_sit() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn keeps_any_classes_the_node_already_carried() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn numbers_containers_from_the_order_it_is_given_above_the_kind_slots() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn never_folds_a_container_back_onto_a_kind_slot() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn stamps_a_slot_the_stylesheet_actually_names_on_a_palette_shorter_than_the_kinds() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the colour slots of colorSlots.ts are the renderer's and are not ported"]
fn agrees_with_the_stylesheet_at_every_palette_length() {
    panic!("renderer: the colour slots of colorSlots.ts are not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn emits_a_rule_for_every_kind_at_that_kind_slot_colour() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn paints_containers_from_the_slots_above_the_kind_range() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn names_both_forms_of_a_container_so_a_collapsed_one_is_not_left_grey() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn uses_no_container_slot_below_the_kind_range() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn strokes_without_filling_when_the_theme_carries_no_background_palette() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn emits_nothing_for_a_theme_that_carries_no_palette() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

#[test]
#[ignore = "renderer: the stylesheet of styles.ts is the renderer's and is not ported"]
fn rejects_a_look_that_would_break_out_of_the_selector() {
    panic!("renderer: the stylesheet of styles.ts is not ported");
}

// containerOrder.spec.ts: the colour indices of getData

#[test]
#[ignore = "renderer: the container colour indices of getData come with the renderer"]
fn gives_the_first_declared_container_the_first_container_slot() {
    panic!("renderer: the container colour indices of getData come with the renderer");
}

#[test]
#[ignore = "renderer: the container colour indices of getData come with the renderer"]
fn numbers_a_parent_before_the_container_nested_inside_it() {
    panic!("renderer: the container colour indices of getData come with the renderer");
}

#[test]
#[ignore = "renderer: the container colour indices of getData come with the renderer"]
fn keeps_a_collapsed_container_in_the_sequence() {
    panic!("renderer: the container colour indices of getData come with the renderer");
}

// renderer.spec.ts

#[test]
#[ignore = "renderer: draw(), the renderer's entry point, is not ported"]
fn calls_set_diagram_id_with_the_svg_element_id() {
    panic!("renderer: draw(), the renderer's entry point, is not ported");
}
