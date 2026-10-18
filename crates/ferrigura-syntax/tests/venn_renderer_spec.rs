//! The specs of the venn renderer and its palette, `vennRenderer.spec.ts` and `vennPalette.spec.ts`.
//! They test the drawing and the layout data, which are the renderer stage's (`docs-port/venn/PLAN.md`,
//! section on the renderer), so each one is an ignored test that says why. The palette spec's
//! `describe.each` over the themes runs its tests once per theme; each title is listed once here.

const REASON: &str =
    "port: the renderer and the layout data of the venn diagram are the renderer stage's";

#[test]
#[ignore = "port: the renderer and the layout data of the venn diagram are the renderer stage's"]
fn renders_a_title_when_provided() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the renderer and the layout data of the venn diagram are the renderer stage's"]
fn renders_text_nodes_with_custom_color_via_style_data() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the renderer and the layout data of the venn diagram are the renderer stage's"]
fn applies_theme_colors_to_circles() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the renderer and the layout data of the venn diagram are the renderer stage's"]
fn user_override_colors_take_priority_over_theme_via_style_data() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the renderer and the layout data of the venn diagram are the renderer stage's"]
fn computes_contrasting_text_color_for_dark_backgrounds() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the renderer and the layout data of the venn diagram are the renderer stage's"]
fn renders_debug_layout_helpers_when_enabled() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: ensurePairwiseSubsets is the layout data of the renderer, ported with the renderer stage"]
fn ensure_pairwise_subsets_returns_the_same_reference_for_empty_array() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: ensurePairwiseSubsets is the layout data of the renderer, ported with the renderer stage"]
fn ensure_pairwise_subsets_returns_the_same_reference_when_no_3_set_unions_exist() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: ensurePairwiseSubsets is the layout data of the renderer, ported with the renderer stage"]
fn ensure_pairwise_subsets_adds_pairwise_subsets_for_a_3_set_union() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: ensurePairwiseSubsets is the layout data of the renderer, ported with the renderer stage"]
fn ensure_pairwise_subsets_handles_partial_pairwise_coverage_adds_only_missing_pairs() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: ensurePairwiseSubsets is the layout data of the renderer, ported with the renderer stage"]
fn ensure_pairwise_subsets_handles_sets_out_of_alphabetical_order() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: ensurePairwiseSubsets is the layout data of the renderer, ported with the renderer stage"]
fn ensure_pairwise_subsets_falls_back_to_default_size_when_individual_set_sizes_are_unknown() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn accounts_for_every_registered_theme() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn every_theme_with_a_palette_defines_every_slot_the_renderer_reads() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn every_theme_without_a_palette_defines_none_and_renders_flat_by_design() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn the_redux_palettes_take_the_theme_categorical_palette() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn the_redux_palettes_give_every_slot_a_distinct_colour() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn paints_each_circle_its_own_colour_under_a_palette_theme() {
    panic!("{REASON}");
}

#[test]
#[ignore = "port: the theme palette is read by the renderer, ported with the renderer stage"]
fn falls_back_to_one_colour_not_to_undefined_without_a_palette() {
    panic!("{REASON}");
}
