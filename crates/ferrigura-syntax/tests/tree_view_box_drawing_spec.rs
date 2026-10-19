//! Port of `packages/mermaid/src/diagrams/treeView/boxDrawingPreprocessor.spec.ts`: the detection of
//! the box-drawing format, the rewriting of the lines, the line map, and the two errors, whose
//! wording the spec asserts. Test names follow the reference's `it` titles in snake_case; the
//! reference's `describe` groups are comments.
//!
//! `remapErrorLines` rewrites the line numbers inside a message. Here the errors carry a position,
//! so the same mapping is tested on the line of the error ([`BoxDrawing::remap_error`]), and the
//! message-rewriting cases are ignored (`docs-port/decisions/0002`).

use std::collections::BTreeMap;

use ferrigura_syntax::ParseError;
use ferrigura_syntax::tree_view::{
    BoxDrawing, is_box_drawing_format, parse, preprocess_box_drawing,
};

fn lines(input: &str) -> Vec<String> {
    let result = preprocess_box_drawing(input).expect("the input preprocesses");
    result.text.split('\n').map(str::to_owned).collect()
}

fn preprocess_error(input: &str) -> ParseError {
    preprocess_box_drawing(input).expect_err("the input is rejected")
}

fn joined(parts: &[&str]) -> String {
    parts.join("\n")
}

mod is_box_drawing_format {
    use super::*;

    #[test]
    fn should_detect_standard_box_drawing_characters() {
        assert!(is_box_drawing_format(&["├── file.txt", "└── other.txt"]));
    }

    #[test]
    fn should_detect_heavy_box_drawing_characters() {
        assert!(is_box_drawing_format(&["┣━━ file.txt", "┗━━ other.txt"]));
    }

    #[test]
    fn should_detect_vertical_continuation_character() {
        assert!(is_box_drawing_format(&["│   └── file.txt"]));
    }

    #[test]
    fn should_return_false_for_indent_only_lines() {
        assert!(!is_box_drawing_format(&[
            "    file.txt",
            "        nested.txt"
        ]));
    }

    #[test]
    fn should_return_false_for_empty_lines() {
        assert!(!is_box_drawing_format(&["", "   "]));
    }

    #[test]
    fn should_return_false_for_plain_text() {
        assert!(!is_box_drawing_format(&["root/", "file.txt"]));
    }
}

mod preprocess_box_drawing {
    use super::*;

    mod indent_format_passthrough {
        use super::*;

        #[test]
        fn should_return_indent_based_input_unchanged() {
            let input = "treeView-beta\nroot/\n    src/\n        index.js";
            let result = preprocess_box_drawing(input).expect("preprocesses");
            assert_eq!(result.text, input);
            assert!(result.line_map.is_empty());
        }

        #[test]
        fn should_return_input_without_keyword_unchanged() {
            let input = "not-a-keyword\nsome text";
            let result = preprocess_box_drawing(input).expect("preprocesses");
            assert_eq!(result.text, input);
            assert!(result.line_map.is_empty());
        }
    }

    mod standard_tree_output {
        use super::*;

        #[test]
        fn should_convert_a_simple_tree() {
            let input = joined(&[
                "treeView-beta",
                "root/",
                "├── src/",
                "│   └── index.js",
                "└── README.md",
            ]);
            let out = lines(&input);
            assert_eq!(out[0], "treeView-beta");
            assert_eq!(out[1], "root/");
            assert_eq!(out[2], "    src/");
            assert_eq!(out[3], "        index.js");
            assert_eq!(out[4], "    README.md");
        }

        #[test]
        fn should_handle_a_deeply_nested_tree() {
            let input = joined(&[
                "treeView-beta",
                "root/",
                "├── a/",
                "│   ├── b/",
                "│   │   ├── c/",
                "│   │   │   └── deep.txt",
                "│   │   └── d.txt",
                "│   └── e.txt",
                "└── f.txt",
            ]);
            let out = lines(&input);
            assert_eq!(out[2], "    a/");
            assert_eq!(out[3], "        b/");
            assert_eq!(out[4], "            c/");
            assert_eq!(out[5], "                deep.txt");
            assert_eq!(out[6], "            d.txt");
            assert_eq!(out[7], "        e.txt");
            assert_eq!(out[8], "    f.txt");
        }

        #[test]
        fn should_handle_a_flat_list_all_at_depth_1() {
            let input = joined(&[
                "treeView-beta",
                "root/",
                "├── file1.txt",
                "├── file2.txt",
                "└── file3.txt",
            ]);
            let out = lines(&input);
            assert_eq!(out[1], "root/");
            assert_eq!(out[2], "    file1.txt");
            assert_eq!(out[3], "    file2.txt");
            assert_eq!(out[4], "    file3.txt");
        }

        #[test]
        fn should_handle_no_root_line_all_box_drawing() {
            let input = joined(&["treeView-beta", "├── file1.txt", "└── file2.txt"]);
            let out = lines(&input);
            assert_eq!(out[0], "treeView-beta");
            assert_eq!(out[1], "    file1.txt");
            assert_eq!(out[2], "    file2.txt");
        }
    }

    mod space_continuation_after_last_child {
        use super::*;

        #[test]
        fn should_handle_space_continuation_correctly() {
            let input = joined(&[
                "treeView-beta",
                "root/",
                "├── a/",
                "│   └── b.txt",
                "└── c/",
                "    └── d.txt",
            ]);
            let out = lines(&input);
            // `    └── d.txt`: the branch is at column 4 and the segment width is 4, so depth 2.
            assert_eq!(out[1], "root/");
            assert_eq!(out[2], "    a/");
            assert_eq!(out[3], "        b.txt");
            assert_eq!(out[4], "    c/");
            assert_eq!(out[5], "        d.txt");
        }
    }

    mod flexible_prefix_widths {
        use super::*;

        #[test]
        fn should_handle_extra_dash() {
            let out = lines(&joined(&["treeView-beta", "root/", "├─── file.txt"]));
            assert_eq!(out[2], "    file.txt");
        }

        #[test]
        fn should_handle_no_trailing_space() {
            let out = lines(&joined(&["treeView-beta", "root/", "├──file.txt"]));
            assert_eq!(out[2], "    file.txt");
        }

        #[test]
        fn should_handle_many_dashes() {
            let out = lines(&joined(&["treeView-beta", "root/", "├───── file.txt"]));
            assert_eq!(out[2], "    file.txt");
        }

        #[test]
        fn should_handle_minimal_dash() {
            let out = lines(&joined(&["treeView-beta", "root/", "└─ file.txt"]));
            assert_eq!(out[2], "    file.txt");
        }
    }

    mod heavy_unicode_variants {
        use super::*;

        #[test]
        fn should_handle_heavy_box_drawing_characters() {
            let input = joined(&[
                "treeView-beta",
                "root/",
                "┣━━ src/",
                "┃   ┗━━ index.js",
                "┗━━ README.md",
            ]);
            let out = lines(&input);
            assert_eq!(out[0], "treeView-beta");
            assert_eq!(out[1], "root/");
            assert_eq!(out[2], "    src/");
            assert_eq!(out[3], "        index.js");
            assert_eq!(out[4], "    README.md");
        }

        #[test]
        fn should_handle_heavy_variants_with_extra_dashes() {
            let out = lines(&joined(&["treeView-beta", "root/", "┣━━━ file.txt"]));
            assert_eq!(out[2], "    file.txt");
        }
    }

    mod annotations {
        use super::*;

        #[test]
        fn should_preserve_class_annotation() {
            let out = lines(&joined(&["treeView-beta", "├── file.txt :::highlight"]));
            assert_eq!(out[1], "    file.txt :::highlight");
        }

        #[test]
        fn should_preserve_icon_annotation() {
            let out = lines(&joined(&["treeView-beta", "├── data.bin icon(database)"]));
            assert_eq!(out[1], "    data.bin icon(database)");
        }

        #[test]
        fn should_preserve_description_annotation() {
            let out = lines(&joined(&["treeView-beta", "├── index.js ## entry point"]));
            assert_eq!(out[1], "    index.js ## entry point");
        }

        #[test]
        fn should_preserve_all_annotations_combined() {
            let out = lines(&joined(&[
                "treeView-beta",
                "├── App.tsx :::highlight icon(react) ## main component",
            ]));
            assert_eq!(
                out[1],
                "    App.tsx :::highlight icon(react) ## main component"
            );
        }
    }

    mod comments_and_blank_lines {
        use super::*;

        #[test]
        fn should_preserve_comments() {
            let input = joined(&[
                "treeView-beta",
                "%% a comment",
                "├── file.txt",
                "%% another comment",
                "└── other.txt",
            ]);
            let out = lines(&input);
            assert_eq!(out[1], "%% a comment");
            assert_eq!(out[2], "    file.txt");
            assert_eq!(out[3], "%% another comment");
            assert_eq!(out[4], "    other.txt");
        }

        #[test]
        fn should_preserve_blank_lines() {
            let input = joined(&["treeView-beta", "", "├── file.txt", "", "└── other.txt"]);
            let out = lines(&input);
            assert_eq!(out[1], "");
            assert_eq!(out[2], "    file.txt");
            assert_eq!(out[3], "");
            assert_eq!(out[4], "    other.txt");
        }
    }

    mod metadata_lines {
        use super::*;

        #[test]
        fn should_pass_through_title_line() {
            let input = joined(&[
                "treeView-beta",
                "title My Tree",
                "├── file.txt",
                "└── other.txt",
            ]);
            let out = lines(&input);
            assert_eq!(out[1], "title My Tree");
            assert_eq!(out[2], "    file.txt");
        }

        #[test]
        fn should_pass_through_acc_title_line() {
            let input = joined(&[
                "treeView-beta",
                "accTitle: Accessible Title",
                "├── file.txt",
            ]);
            let out = lines(&input);
            assert_eq!(out[1], "accTitle: Accessible Title");
            assert_eq!(out[2], "    file.txt");
        }

        #[test]
        fn should_pass_through_acc_descr_line() {
            let input = joined(&["treeView-beta", "accDescr: A description", "├── file.txt"]);
            let out = lines(&input);
            assert_eq!(out[1], "accDescr: A description");
            assert_eq!(out[2], "    file.txt");
        }
    }

    mod decoration_only_lines {
        use super::*;

        #[test]
        fn should_skip_lines_with_only_vertical_bars() {
            let input = joined(&["treeView-beta", "root/", "├── a.txt", "│", "└── b.txt"]);
            let out = lines(&input);
            // The │-only line is dropped.
            assert_eq!(out.len(), 4);
            assert_eq!(out[0], "treeView-beta");
            assert_eq!(out[1], "root/");
            assert_eq!(out[2], "    a.txt");
            assert_eq!(out[3], "    b.txt");
        }
    }

    mod line_mapping {
        use super::*;

        #[test]
        fn should_map_output_lines_to_original_line_numbers() {
            let input = joined(&[
                "treeView-beta",
                "root/",
                "├── src/",
                "│   └── index.js",
                "└── README.md",
            ]);
            let result = preprocess_box_drawing(&input).expect("preprocesses");
            for line in 1..=5 {
                assert_eq!(
                    result.line_map.get(&line),
                    Some(&line),
                    "output line {line}"
                );
            }
        }

        #[test]
        fn should_handle_skipped_decoration_lines_in_mapping() {
            let input = joined(&["treeView-beta", "├── a.txt", "│", "└── b.txt"]);
            let result = preprocess_box_drawing(&input).expect("preprocesses");
            assert_eq!(result.line_map.get(&1), Some(&1));
            assert_eq!(result.line_map.get(&2), Some(&2));
            // Line 3 of the output is line 4 of the input: line 3 is decoration, and is dropped.
            assert_eq!(result.line_map.get(&3), Some(&4));
        }
    }

    mod error_cases {
        use super::*;

        #[test]
        fn should_throw_on_empty_content_after_prefix() {
            let error = preprocess_error(&joined(&["treeView-beta", "├── "]));
            assert_eq!(
                error.message,
                "Line 2: Empty node — expected a filename or directory name after the box-drawing prefix"
            );
            assert_eq!(error.line, 2);
        }

        #[test]
        fn should_normalize_tabs_to_spaces() {
            let out = lines(&joined(&["treeView-beta", "├──\tfile.txt"]));
            assert_eq!(out[1], "    file.txt");
        }

        #[test]
        fn should_handle_tab_indented_box_drawing_lines() {
            // A tab expands to four spaces, so `\t├──` puts the branch at column 4: depth 2.
            let out = lines(&joined(&[
                "treeView-beta",
                "├── src/",
                "\t├── index.ts",
                "\t└── utils.ts",
            ]));
            assert_eq!(out[1], "    src/");
            assert_eq!(out[2], "        index.ts");
            assert_eq!(out[3], "        utils.ts");
        }

        #[test]
        fn should_throw_on_indented_line_without_box_chars_in_box_mode() {
            let error = preprocess_error(&joined(&[
                "treeView-beta",
                "├── src/",
                "    index.js",
                "└── README.md",
            ]));
            assert!(
                error
                    .message
                    .starts_with("Line 3: Unexpected indentation without box-drawing characters"),
                "{}",
                error.message
            );
            assert!(
                error.message.ends_with(
                    "In box-drawing format, use ├── or └── prefixes for indented nodes."
                )
            );
        }

        #[test]
        fn should_include_line_number_in_empty_content_error() {
            let error =
                preprocess_error(&joined(&["treeView-beta", "root/", "├── src/", "│   └── "]));
            assert!(error.message.starts_with("Line 4:"), "{}", error.message);
        }
    }

    mod real_world_tree_command_output {
        use super::*;

        #[test]
        fn should_handle_typical_linux_tree_output() {
            let input = joined(&[
                "treeView-beta",
                "my-project/",
                "├── src/",
                "│   ├── components/",
                "│   │   └── MyComponent.js",
                "│   ├── index.css",
                "│   └── index.js",
                "├── package.json",
                "└── README.md",
            ]);
            let out = lines(&input);
            assert_eq!(out[0], "treeView-beta");
            assert_eq!(out[1], "my-project/");
            assert_eq!(out[2], "    src/");
            assert_eq!(out[3], "        components/");
            assert_eq!(out[4], "            MyComponent.js");
            assert_eq!(out[5], "        index.css");
            assert_eq!(out[6], "        index.js");
            assert_eq!(out[7], "    package.json");
            assert_eq!(out[8], "    README.md");
        }

        #[test]
        fn should_handle_tree_output_with_annotations() {
            let input = joined(&[
                "treeView-beta",
                "my-project/",
                "├── src/ :::highlight",
                "│   ├── App.tsx :::highlight icon(react) ## main component",
                "│   └── index.ts ## entry point",
                "├── package.json",
                "└── README.md ## project docs",
            ]);
            let out = lines(&input);
            assert_eq!(out[2], "    src/ :::highlight");
            assert_eq!(
                out[3],
                "        App.tsx :::highlight icon(react) ## main component"
            );
            assert_eq!(out[4], "        index.ts ## entry point");
            assert_eq!(out[5], "    package.json");
            assert_eq!(out[6], "    README.md ## project docs");
        }
    }

    /// Port-specific: the column of a branch is counted in UTF-16 units, as the reference's `index`
    /// is. An emoji (two units) before a branch moves it to depth 2 in a file with segment width 4.
    mod utf16_columns {
        use super::*;

        #[test]
        fn should_count_an_astral_character_as_two_columns() {
            // The segment width is 4 (the first branch past column 0 is at column 4). `🙂` is two
            // UTF-16 units, so the branch sits at column 2: round(2 / 4) + 1 = 2.
            let input = joined(&[
                "treeView-beta",
                "root/",
                "├── a/",
                "│   └── b.txt",
                "🙂└── c",
            ]);
            let out = lines(&input);
            assert_eq!(out[4], "        c");
        }
    }
}

mod remap_error_lines {
    use super::*;

    fn map(pairs: &[(usize, usize)]) -> BoxDrawing {
        BoxDrawing {
            text: String::new(),
            line_map: pairs.iter().copied().collect::<BTreeMap<_, _>>(),
        }
    }

    fn error_at(line: usize) -> ParseError {
        ParseError {
            line,
            column: 1,
            message: "bad syntax".to_owned(),
        }
    }

    #[test]
    fn should_remap_line_numbers_found_in_the_map() {
        let result = map(&[(1, 1), (2, 3), (3, 5)]);
        assert_eq!(result.remap_error(error_at(2)).line, 3);
    }

    #[test]
    fn should_leave_unmapped_line_numbers_unchanged() {
        let result = map(&[(1, 1)]);
        assert_eq!(result.remap_error(error_at(99)).line, 99);
    }

    #[test]
    fn should_handle_multiple_line_references() {
        let result = map(&[(2, 4), (5, 10)]);
        assert_eq!(result.remap_error(error_at(2)).line, 4);
        assert_eq!(result.remap_error(error_at(5)).line, 10);
    }

    #[test]
    #[ignore = "port: a ParseError carries a position, not a message that names a line, so the message is not rewritten (docs-port/decisions/0002)"]
    fn should_be_case_insensitive() {}

    #[test]
    fn a_rewritten_text_is_remapped_through_parse() {
        // The grammar's error on the rewritten text is reported at the original line: the lexer
        // error of `└── b icon(a.b)` (output line 4) is on line 4 of the input.
        let error = parse("treeView-beta\n.\n├── a :::\n└── b icon(a.b)").expect_err("rejected");
        assert_eq!(error.line, 4);
    }

    #[test]
    fn a_text_without_box_drawing_keeps_its_positions() {
        let error = parse("treeView-beta\na\ntitle X").expect_err("rejected");
        assert_eq!(error.line, 3);
    }
}
