//! Placeholders for `packages/mermaid/src/diagrams/swimlanes/lanePalette.spec.ts`. The spec asserts
//! the stylesheet that the swimlane renderer writes (the lane rules of each colour theme's palette,
//! the `!important` border, the handDrawn title fill), so it is the renderer stage's. Each test is
//! kept under its reference name, ignored, and panics if it is run.

mod lane_palette {
    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn paints_both_halves_of_the_lane_in_every_palette_slot_of_every_colour_theme() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }

    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn keeps_the_generic_cluster_palette_rules_away_from_lanes() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }

    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn exempts_palette_lanes_from_the_important_lane_border() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }

    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn never_marks_a_lane_palette_rule_important() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }

    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn emits_no_empty_or_undefined_declaration() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }

    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn emits_the_handdrawn_title_fill_rule_only_where_a_background_palette_exists() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }

    #[test]
    #[ignore = "renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)"]
    fn emits_no_lane_palette_rules_for_the_plain_themes() {
        panic!("renderer stage: the lane palette is the swimlane stylesheet (lanePalette.spec.ts)");
    }
}
