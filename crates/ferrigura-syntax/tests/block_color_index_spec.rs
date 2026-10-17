//! Port of packages/mermaid/src/diagrams/block/blockColorIndex.spec.ts. Its eight tests check the
//! palette slots of the composites: the `colorIndex` the DB gives them, the palette rules of the
//! style sheet, and the stamping of `data-color-id`. The slots are read by the renderer and the
//! rules are the renderer's styles (`docs-port/block/PLAN.md`, section 1), so the tests are ignored
//! here and move to the renderer's tests with them. The `colorIndex` values are in the model, and
//! the differential run compares them.

#[test]
#[ignore = "port: colour slots are the renderer stage's (stamped as data-color-id, styled by the palette rules)"]
fn numbers_composites_in_declaration_order() {
    todo!("the renderer stage ports the colour slots")
}

#[test]
#[ignore = "port: colour slots are the renderer stage's (stamped as data-color-id, styled by the palette rules)"]
fn gives_no_slot_to_a_simple_block() {
    todo!("the renderer stage ports the colour slots")
}

#[test]
#[ignore = "port: colour slots are the renderer stage's (stamped as data-color-id, styled by the palette rules)"]
fn numbers_an_outer_composite_before_one_nested_inside_it() {
    todo!("the renderer stage ports the colour slots")
}

#[test]
#[ignore = "port: colour slots are the renderer stage's (stamped as data-color-id, styled by the palette rules)"]
fn runs_one_counter_across_the_whole_diagram() {
    todo!("the renderer stage ports the colour slots")
}

#[test]
#[ignore = "port: palette rules are the renderer's styles (styles.ts genColor)"]
fn emits_one_composite_rule_per_palette_entry_under_a_colour_theme() {
    todo!("the renderer stage ports the palette rules")
}

#[test]
#[ignore = "port: palette rules are the renderer's styles (styles.ts genColor)"]
fn leaves_the_simple_shapes_to_the_flat_theme_colour() {
    todo!("the renderer stage ports the palette rules")
}

#[test]
#[ignore = "port: palette rules are the renderer's styles (styles.ts genColor)"]
fn emits_nothing_for_a_theme_that_carries_no_palette() {
    todo!("the renderer stage ports the palette rules")
}

#[test]
#[ignore = "port: palette rules are the renderer's styles (styles.ts genColor)"]
fn rejects_a_look_that_would_break_out_of_the_selector() {
    todo!("the renderer stage ports the palette rules")
}
