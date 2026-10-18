//! Port of `packages/mermaid/src/diagrams/treeView/icons.spec.ts`, as ignored tests. The icon
//! resolution (`treeViewIcons`, `detectIcon` and `getNodeIcon`) runs when the tree is drawn, with the
//! configuration of `showIcons`, `defaultIconPack`, `filenameIcons` and `extensionIcons`, so it is
//! the renderer stage's (`docs-port/tree-view/PLAN.md`, section 6). The model keeps the icon the
//! grammar gave, as the DB does, and the parser tests check that value (`tree_view_parser_spec.rs`).
//!
//! Each `it` of the reference is listed under its `describe` group, with its title in snake_case.

mod icons {
    mod treeview_icons_pack {
        #[test]
        #[ignore = "port: the built-in icon pack is the renderer's (docs-port/tree-view/PLAN.md, section 6)"]
        fn uses_the_mermaid_treeview_prefix() {}

        #[test]
        #[ignore = "port: the built-in icon pack is the renderer's (docs-port/tree-view/PLAN.md, section 6)"]
        fn contains_exactly_the_built_in_file_and_folder_icons() {}

        #[test]
        #[ignore = "port: the built-in icon pack is the renderer's (docs-port/tree-view/PLAN.md, section 6)"]
        fn every_icon_has_a_non_empty_body_that_inherits_color_via_current_color() {}
    }

    mod detect_icon {
        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn has_no_built_in_mapping_returns_undefined_without_configured_maps() {}

        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn matches_exact_filenames_from_filename_icons() {}

        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn matches_extensions_from_extension_icons_with_or_without_the_leading_dot() {}

        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn matches_extensions_case_insensitively() {}

        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn filename_matches_beat_extension_matches() {}

        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn uses_the_last_extension_for_multi_dot_names() {}

        #[test]
        #[ignore = "port: icon detection runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn does_not_extension_match_dotfiles() {}
    }

    mod get_node_icon {
        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn returns_undefined_for_none_regardless_of_config() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn returns_prefixed_explicit_icons_as_is_regardless_of_show_icons() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn qualifies_built_in_names_with_the_built_in_pack_even_when_default_icon_pack_is_set() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn qualifies_unprefixed_explicit_icons_with_the_default_icon_pack() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn qualifies_unprefixed_explicit_icons_with_the_built_in_pack_when_no_default_icon_pack() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn returns_undefined_without_an_explicit_icon_when_show_icons_is_off() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn uses_the_built_in_icons_by_node_type_when_show_icons_is_on() {}
    }

    mod configured_detection_maps {
        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn picks_file_icons_from_extension_icons_when_show_icons_is_on() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn uses_prefixed_map_values_as_is_even_without_a_default_icon_pack() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn ignores_the_maps_when_show_icons_is_off() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn hides_the_icon_for_files_mapped_to_none() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn allows_map_values_to_reference_the_built_in_icons() {}

        #[test]
        #[ignore = "port: icon resolution runs at render time (docs-port/tree-view/PLAN.md, section 6)"]
        fn falls_back_to_the_built_in_file_icon_when_nothing_matches() {}
    }
}
