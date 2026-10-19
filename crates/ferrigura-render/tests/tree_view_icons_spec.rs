//! Port of `packages/mermaid/src/diagrams/treeView/icons.spec.ts`, its 23 cases under the reference's
//! `describe` groups, with the reference's titles in snake_case.
//!
//! The reference's functions return the qualified name of the icon (`mermaid-treeview:folder`,
//! `logos:react`), and the tests read those names. ferrigura draws the icon, so each case reads the
//! icon the row draws: the built-in folder or file, the unknown icon for a name that does not resolve
//! (ferrigura registers no pack, and the reference draws the same unknown icon for a pack it cannot
//! load), or no icon. A qualified name that the unknown icon hides (`logos:react` against
//! `mermaid-treeview:react`) is not told apart: both draw the unknown icon, as the reference does.

#[path = "tree_view/support.rs"]
mod support;

use ferrigura_model::config::TreeViewDiagramConfig;
use ferrigura_model::value::Value;
use serde_json::json;
use support::{IconKind, draw_section, parse, rows};

/// The configuration of the icon tests: `showIcons`, the default pack, and the two maps.
fn config(
    show_icons: bool,
    pack: &str,
    filenames: Value,
    extensions: Value,
) -> TreeViewDiagramConfig {
    TreeViewDiagramConfig {
        show_icons,
        default_icon_pack: pack.to_owned(),
        filename_icons: Some(filenames),
        extension_icons: Some(extensions),
        ..TreeViewDiagramConfig::default()
    }
}

/// The configuration with no pack and no maps.
fn plain(show_icons: bool) -> TreeViewDiagramConfig {
    config(show_icons, "", Value::empty_object(), Value::empty_object())
}

/// The icon of the node that the line gives, drawn under the root: `None` when it has no icon.
fn icon_of(section: &TreeViewDiagramConfig, line: &str) -> Option<IconKind> {
    let text = format!("treeView-beta\n  {line}");
    let svg = parse(&draw_section(&text, section).expect("the tree view renders"));
    rows(&svg)[1].icon
}

fn map(value: serde_json::Value) -> Value {
    Value::from(value)
}

mod icons {
    mod treeview_icons_pack {
        use super::super::*;

        #[test]
        fn uses_the_mermaid_treeview_prefix() {
            // The built-in pack is the one `mermaid-treeview:` names: its folder and file are drawn.
            assert_eq!(
                icon_of(&plain(false), "a icon(mermaid-treeview:folder)"),
                Some(IconKind::Folder)
            );
            assert_eq!(
                icon_of(&plain(false), "a icon(mermaid-treeview:file)"),
                Some(IconKind::File)
            );
        }

        #[test]
        fn contains_exactly_the_built_in_file_and_folder_icons() {
            assert_eq!(
                icon_of(&plain(false), "a icon(folder)"),
                Some(IconKind::Folder)
            );
            assert_eq!(icon_of(&plain(false), "a icon(file)"), Some(IconKind::File));
            assert_eq!(
                icon_of(&plain(false), "a icon(bell)"),
                Some(IconKind::Unknown)
            );
        }

        #[test]
        fn every_icon_has_a_non_empty_body_that_inherits_color_via_current_color() {
            // Both built-in icons are drawn with `currentColor`, so the icon group's colour reaches them.
            for line in ["a icon(folder)", "a icon(file)"] {
                let text = format!("treeView-beta\n  {line}");
                let svg = draw_section(&text, &plain(false)).expect("the tree view renders");
                assert!(svg.contains(r#"fill="currentColor""#), "{line}: {svg}");
            }
        }
    }

    mod detect_icon {
        use super::super::*;

        #[test]
        fn has_no_built_in_mapping_returns_undefined_without_configured_maps() {
            // Without a map a file gets the built-in file icon, and no name is detected.
            assert_eq!(icon_of(&plain(true), "utils.ts"), Some(IconKind::File));
            assert_eq!(icon_of(&plain(true), "Dockerfile"), Some(IconKind::File));
            assert_eq!(icon_of(&plain(true), "package.json"), Some(IconKind::File));
        }

        #[test]
        fn matches_exact_filenames_from_filename_icons() {
            let section = config(
                true,
                "",
                map(json!({"Dockerfile": "docker"})),
                Value::empty_object(),
            );
            assert_eq!(icon_of(&section, "Dockerfile"), Some(IconKind::Unknown));
            assert_eq!(icon_of(&section, "Other"), Some(IconKind::File));
        }

        #[test]
        fn matches_extensions_from_extension_icons_with_or_without_the_leading_dot() {
            let dotted = config(
                true,
                "",
                Value::empty_object(),
                map(json!({".ts": "typescript"})),
            );
            assert_eq!(icon_of(&dotted, "utils.ts"), Some(IconKind::Unknown));
            let bare = config(
                true,
                "",
                Value::empty_object(),
                map(json!({"ts": "typescript"})),
            );
            assert_eq!(icon_of(&bare, "utils.ts"), Some(IconKind::Unknown));
        }

        #[test]
        fn matches_extensions_case_insensitively() {
            let section = config(
                true,
                "",
                Value::empty_object(),
                map(json!({".ts": "folder"})),
            );
            assert_eq!(icon_of(&section, "APP.TS"), Some(IconKind::Folder));
        }

        #[test]
        fn filename_matches_beat_extension_matches() {
            // The extension would hide the icon; the filename's folder wins.
            let section = config(
                true,
                "",
                map(json!({"tsconfig.json": "folder"})),
                map(json!({".json": "none"})),
            );
            assert_eq!(icon_of(&section, "tsconfig.json"), Some(IconKind::Folder));
        }

        #[test]
        fn uses_the_last_extension_for_multi_dot_names() {
            // The last extension is `.ts`; `.spec.ts` is not one the map names.
            let section = config(
                true,
                "",
                Value::empty_object(),
                map(json!({".ts": "folder"})),
            );
            assert_eq!(
                icon_of(&section, "component.spec.ts"),
                Some(IconKind::Folder)
            );
        }

        #[test]
        fn does_not_extension_match_dotfiles() {
            // `.bashrc` has no extension: the map's `bashrc` is not read, and the file icon is drawn.
            let section = config(
                true,
                "",
                Value::empty_object(),
                map(json!({"bashrc": "folder"})),
            );
            assert_eq!(icon_of(&section, ".bashrc"), Some(IconKind::File));
        }
    }

    mod get_node_icon {
        use super::super::*;

        #[test]
        fn returns_undefined_for_none_regardless_of_config() {
            assert_eq!(icon_of(&plain(false), "a.ts icon(none)"), None);
            let section = config(true, "logos", Value::empty_object(), Value::empty_object());
            assert_eq!(icon_of(&section, "src/ icon(none)"), None);
        }

        #[test]
        fn returns_prefixed_explicit_icons_as_is_regardless_of_show_icons() {
            assert_eq!(
                icon_of(&plain(false), "a.ts icon(logos:react)"),
                Some(IconKind::Unknown)
            );
            assert_eq!(
                icon_of(&plain(true), "a.ts icon(logos:react)"),
                Some(IconKind::Unknown)
            );
        }

        #[test]
        fn qualifies_built_in_names_with_the_built_in_pack_even_when_default_icon_pack_is_set() {
            let section = config(false, "logos", Value::empty_object(), Value::empty_object());
            assert_eq!(icon_of(&section, "a.ts icon(file)"), Some(IconKind::File));
            assert_eq!(
                icon_of(&plain(false), "a.ts icon(folder)"),
                Some(IconKind::Folder)
            );
        }

        #[test]
        fn qualifies_unprefixed_explicit_icons_with_the_default_icon_pack() {
            // `logos:react` is not a name ferrigura has: the unknown icon, as for any other pack.
            let section = config(false, "logos", Value::empty_object(), Value::empty_object());
            assert_eq!(
                icon_of(&section, "a.ts icon(react)"),
                Some(IconKind::Unknown)
            );
        }

        #[test]
        fn qualifies_unprefixed_explicit_icons_with_the_built_in_pack_when_no_default_icon_pack_is_set()
         {
            // Resolves to the unknown-icon fallback at fetch time.
            assert_eq!(
                icon_of(&plain(false), "a.ts icon(react)"),
                Some(IconKind::Unknown)
            );
        }

        #[test]
        fn returns_undefined_without_an_explicit_icon_when_show_icons_is_off() {
            assert_eq!(icon_of(&plain(false), "utils.ts"), None);
            let section = config(false, "logos", Value::empty_object(), Value::empty_object());
            assert_eq!(icon_of(&section, "src/"), None);
        }

        #[test]
        fn uses_the_built_in_icons_by_node_type_when_show_icons_is_on() {
            assert_eq!(icon_of(&plain(true), "utils.ts"), Some(IconKind::File));
            assert_eq!(icon_of(&plain(true), "src/"), Some(IconKind::Folder));
            // The default pack only affects explicit icon() references, not the defaults.
            let section = config(true, "logos", Value::empty_object(), Value::empty_object());
            assert_eq!(icon_of(&section, "utils.ts"), Some(IconKind::File));
        }
    }

    mod configured_detection_maps {
        use super::super::*;

        #[test]
        fn picks_file_icons_from_extension_icons_when_show_icons_is_on() {
            let section = config(
                true,
                "material-icon-theme",
                Value::empty_object(),
                map(json!({".ts": "typescript"})),
            );
            assert_eq!(icon_of(&section, "utils.ts"), Some(IconKind::Unknown));
        }

        #[test]
        fn uses_prefixed_map_values_as_is_even_without_a_default_icon_pack() {
            let section = config(
                true,
                "",
                Value::empty_object(),
                map(json!({".ts": "logos:typescript-icon"})),
            );
            assert_eq!(icon_of(&section, "utils.ts"), Some(IconKind::Unknown));
        }

        #[test]
        fn ignores_the_maps_when_show_icons_is_off() {
            let section = config(
                false,
                "",
                Value::empty_object(),
                map(json!({".ts": "logos:typescript-icon"})),
            );
            assert_eq!(icon_of(&section, "utils.ts"), None);
        }

        #[test]
        fn hides_the_icon_for_files_mapped_to_none() {
            let section = config(
                true,
                "",
                Value::empty_object(),
                map(json!({".txt": "none"})),
            );
            assert_eq!(icon_of(&section, "notes.txt"), None);
        }

        #[test]
        fn allows_map_values_to_reference_the_built_in_icons() {
            let section = config(
                true,
                "material-icon-theme",
                map(json!({"notes.txt": "folder"})),
                Value::empty_object(),
            );
            assert_eq!(icon_of(&section, "notes.txt"), Some(IconKind::Folder));
        }

        #[test]
        fn falls_back_to_the_built_in_file_icon_when_nothing_matches() {
            let section = config(
                true,
                "",
                Value::empty_object(),
                map(json!({".ts": "typescript"})),
            );
            assert_eq!(icon_of(&section, "data.xyz"), Some(IconKind::File));
        }
    }
}
