//! Render-level tests of the sequence diagram, as ignored stubs with the reference's names. They
//! check the drawn output (bounds, SVG, CSS, theme colours) of the renderer, which is ported in
//! milestone M6-R. Each stub names its reference file and line; the title says what it checks.
//!
//! The files are `sequenceDiagram.spec.js` (the bounds block `:1783-1916`, the APA and mirror
//! blocks `:1917-2396`), `svgDraw.spec.js`, `sequenceRenderer.spec.js`, `actorBandModel.spec.ts`,
//! `actorSizing.spec.ts`, `lifelineStart.spec.ts`, `rectSectionFill.spec.ts`,
//! `noteFontWeight.spec.ts` and `palettePicking.spec.ts`. The `it.each` tables of the reference are
//! one stub per row where the row is named in the title, and one stub otherwise.

mod sequence_diagram_bounds {
    //! `sequenceDiagram.spec.js:1783-1916`.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_a_simple_bound_call() {
        // Reference: sequenceDiagram.spec.js:1812. Bounds of a bound call, no messages.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_an_expanding_bound() {
        // Reference: sequenceDiagram.spec.js:1821.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_inserts_within_the_bound_without_changing_the_outer_bounds() {
        // Reference: sequenceDiagram.spec.js:1831.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_a_loop_without_expanding_the_area() {
        // Reference: sequenceDiagram.spec.js:1842.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_multiple_loops_without_expanding_the_bounds() {
        // Reference: sequenceDiagram.spec.js:1863.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_a_loop_that_expands_the_area() {
        // Reference: sequenceDiagram.spec.js:1894.
    }
}

mod sequence_diagram_apa_rendering {
    //! `sequenceDiagram.spec.js:1917-2335`, the APA rendering block.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn it_should_handle_one_actor_when_text_placement_is_tspan() {
        // Reference: sequenceDiagram.spec.js:1957, the `textPlacement` table, row `tspan`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn it_should_handle_one_actor_when_text_placement_is_fo() {
        // Reference: sequenceDiagram.spec.js:1957, row `fo`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn it_should_handle_one_actor_when_text_placement_is_old() {
        // Reference: sequenceDiagram.spec.js:1957, row `old`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn it_should_handle_one_actor_when_text_placement_is_undefined() {
        // Reference: sequenceDiagram.spec.js:1957, row `undefined`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_same_actor_with_different_whitespace_properly() {
        // Reference: sequenceDiagram.spec.js:1974.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_and_a_centered_note() {
        // Reference: sequenceDiagram.spec.js:1985.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_and_a_note_to_the_left() {
        // Reference: sequenceDiagram.spec.js:2003.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_and_a_note_to_the_right() {
        // Reference: sequenceDiagram.spec.js:2019.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_two_actors() {
        // Reference: sequenceDiagram.spec.js:2035.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_two_actors_in_a_box() {
        // Reference: sequenceDiagram.spec.js:2048.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_two_actors_with_init_directive() {
        // Reference: sequenceDiagram.spec.js:2066.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_two_actors_with_init_directive_with_multiline_directive() {
        // Reference: sequenceDiagram.spec.js:2083.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_two_actors_and_two_centered_shared_notes() {
        // Reference: sequenceDiagram.spec.js:2104.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_and_two_messages() {
        // Reference: sequenceDiagram.spec.js:2121.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_notes_to_the_right() {
        // Reference: sequenceDiagram.spec.js:2136.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_notes_to_the_left() {
        // Reference: sequenceDiagram.spec.js:2155.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_inline() {
        // Reference: sequenceDiagram.spec.js:2172.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_directive() {
        // Reference: sequenceDiagram.spec.js:2191.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_and_the_init_directive_sets_the_theme_to_dark()
     {
        // Reference: sequenceDiagram.spec.js:2215.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_actors_notes_to_the_left_with_text_wrapped_and_the_init_directive_sets_the_theme_to_dark_and_font_family_to_menlo_font_size_to_18_and_font_weight_to_800()
     {
        // Reference: sequenceDiagram.spec.js:2240.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_two_loops() {
        // Reference: sequenceDiagram.spec.js:2266.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_draw_background_rect() {
        // Reference: sequenceDiagram.spec.js:2283.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_increment_the_sequence_number_with_a_decimal_in_the_hundredths_place() {
        // Reference: sequenceDiagram.spec.js:2301. The visibility of the numbers is set by draw.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_increment_the_sequence_number_with_a_decimal_in_the_tenths_place() {
        // Reference: sequenceDiagram.spec.js:2318.
    }
}

mod sequence_diagram_mirror_rendering {
    //! `sequenceDiagram.spec.js:2336-2375`, the mirror block (its parse-level test is in
    //! `sequence_parse_spec.rs`).

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_when_text_placement_is_tspan() {
        // Reference: sequenceDiagram.spec.js:2370, row `tspan`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_when_text_placement_is_fo() {
        // Reference: sequenceDiagram.spec.js:2370, row `fo`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_when_text_placement_is_old() {
        // Reference: sequenceDiagram.spec.js:2370, row `old`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_handle_one_actor_when_text_placement_is_undefined() {
        // Reference: sequenceDiagram.spec.js:2370, row `undefined`.
    }
}

mod svg_draw {
    //! `svgDraw.spec.js`: the SVG helpers of the renderer.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_read_config_at_render_time_for_each_rect() {
        // Reference: svgDraw.spec.js:3.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_read_config_at_render_time_for_each_rect_2() {
        // Reference: svgDraw.spec.js:42.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_append_a_rectangle() {
        // Reference: svgDraw.spec.js:71.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_not_add_the_class_attribute_if_a_class_is_not_provided() {
        // Reference: svgDraw.spec.js:97.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_append_a_single_element() {
        // Reference: svgDraw.spec.js:117.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_append_a_multiple_elements() {
        // Reference: svgDraw.spec.js:141.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn anchor_left_positions_text_at_x_text_margin() {
        // Reference: svgDraw.spec.js:172.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn anchor_center_positions_text_at_x_width_2() {
        // Reference: svgDraw.spec.js:187.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn anchor_right_positions_text_at_x_width_text_margin() {
        // Reference: svgDraw.spec.js:202.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_work_with_numeral_font_sizes() {
        // Reference: svgDraw.spec.js:218.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_use_section_title_class_for_section_titles_so_custom_loop_text_css_does_not_hide_them()
     {
        // Reference: svgDraw.spec.js:294.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn should_append_a_rect_before_the_previous_element_within_a_given_bound() {
        // Reference: svgDraw.spec.js:332.
    }
}

mod sequence_renderer {
    //! `sequenceRenderer.spec.js:46-96`.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn passes_min_startx_stopx_and_abs_stopx_startx_to_draw_text_when_startx_stopx() {
        // Reference: sequenceRenderer.spec.js:60.
    }
}

mod actor_band_model {
    //! `actorBandModel.spec.ts`: the actor bands, pure in the reference but drawn with the pipeline.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn starts_and_ends_every_lifeline_on_the_two_datum_lines() {
        // Reference: actorBandModel.spec.ts:139.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn puts_every_outside_label_on_one_baseline_clear_of_the_lifeline_header_and_footer() {
        // Reference: actorBandModel.spec.ts:148.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn keeps_a_multiline_label_on_the_datum_growing_away_from_it() {
        // Reference: actorBandModel.spec.ts:162.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn renders_every_glyph_inside_the_bands_transforms_included() {
        // Reference: actorBandModel.spec.ts:187.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn reports_one_uniform_actor_height_back_into_the_pipeline() {
        // Reference: actorBandModel.spec.ts:220.
    }
}

mod stick_figure_actor_sizing {
    //! `actorSizing.spec.ts`.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn puts_its_label_where_a_database_puts_its_own_on_classic() {
        // Reference: actorSizing.spec.ts:98, row `classic`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn puts_its_label_where_a_database_puts_its_own_on_neo() {
        // Reference: actorSizing.spec.ts:98, row `neo`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn leaves_the_classic_figure_at_the_size_it_has_always_been() {
        // Reference: actorSizing.spec.ts:108.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn draws_the_figure_smaller_on_neo_than_on_classic_at_the_glyph_band_size() {
        // Reference: actorSizing.spec.ts:118.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn fills_the_shared_glyph_band_on_neo() {
        // Reference: actorSizing.spec.ts:130.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn anchors_the_label_at_the_datum_on_neo() {
        // Reference: actorSizing.spec.ts:139.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn reports_a_height_that_is_not_shrunk_by_the_look() {
        // Reference: actorSizing.spec.ts:150.
    }
}

mod lifeline_start {
    //! `lifelineStart.spec.ts`.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn starts_every_icon_and_label_below_shape_at_the_same_place_on_neo() {
        // Reference: lifelineStart.spec.ts:86.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn leaves_the_same_gap_between_label_and_lifeline_for_all_of_them() {
        // Reference: lifelineStart.spec.ts:98.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn starts_at_the_box_bottom_for_shapes_whose_label_sits_inside_the_box() {
        // Reference: lifelineStart.spec.ts:109.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn ends_the_round_icons_at_the_same_height_on_neo() {
        // Reference: lifelineStart.spec.ts:117.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn leaves_the_classic_icon_centres_alone() {
        // Reference: lifelineStart.spec.ts:131.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn leaves_every_classic_lifeline_exactly_where_it_was() {
        // Reference: lifelineStart.spec.ts:141.
    }
}

mod sequence_rect_section_fill {
    //! `rectSectionFill.spec.ts`: the theme colours (`it.each` over the themes, one stub each).

    #[test]
    #[ignore = "port: render, M6-R"]
    fn is_distinguishable_from_the_background_on_the_theme() {
        // Reference: rectSectionFill.spec.ts:24, one row per theme.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn stays_distinguishable_when_the_background_is_overridden_to_black_on_the_theme() {
        // Reference: rectSectionFill.spec.ts:30, one row per theme.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn is_still_overridable_through_theme_variables() {
        // Reference: rectSectionFill.spec.ts:44.
    }
}

mod sequence_note_font_weight {
    //! `noteFontWeight.spec.ts`: the stylesheet of the note text.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn does_not_force_a_weight_on_note_text_under_the_theme() {
        // Reference: noteFontWeight.spec.ts:28, one row per theme.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn leaves_the_theme_variable_alone_because_git_depends_on_it() {
        // Reference: noteFontWeight.spec.ts:36.
    }
}

mod palette_picking {
    //! `palettePicking.spec.ts`: the actor colour palette of the renderer.

    #[test]
    #[ignore = "port: render, M6-R"]
    fn cycles_within_the_palette_it_was_given() {
        // Reference: palettePicking.spec.ts:26.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn returns_undefined_for_an_empty_palette_rather_than_a_substitute_colour() {
        // Reference: palettePicking.spec.ts:37.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn never_runs_off_the_end_of_a_short_palette() {
        // Reference: palettePicking.spec.ts:43.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn is_independent_of_any_other_palette_length() {
        // Reference: palettePicking.spec.ts:50.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn redux_color() {
        // Reference: palettePicking.spec.ts:66, row `redux-color`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn redux_dark_color() {
        // Reference: palettePicking.spec.ts:66, row `redux-dark-color`.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn redux_dark_color_still_yields_no_fill_so_the_stylesheet_keeps_deciding() {
        // Reference: palettePicking.spec.ts:82.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn has_no_raw_palette_indexing_left() {
        // Reference: palettePicking.spec.ts:101.
    }

    #[test]
    #[ignore = "port: render, M6-R"]
    fn routes_every_actor_colour_through_palette_color_each_to_its_own_property() {
        // Reference: palettePicking.spec.ts:108.
    }
}
