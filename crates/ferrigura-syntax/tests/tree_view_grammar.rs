//! The lexer, grammar and DB quirks of the tree view, pinned on the reference's behaviour
//! (`docs-port/tree-view/NOTES.md`, section 2.7, and section 6.11 for the root row). Each test names
//! the row of the table it covers, and uses the probe's input; the differential run
//! (`pnpm port:diff treeView`) compares every probe in `tools/port/cases.ts` with the reference.
//!
//! The reference's own grammar tests are in `packages/parser/tests/treeView.test.ts`, which is not
//! counted for the port (`docs-port/STATUS.md`); the quirks here are the ones the notes probed.

use ferrigura_model::tree_view::{NodeType, TreeViewDiagram, TreeViewNode};
use ferrigura_syntax::tree_view::parse;

fn parsed(text: &str) -> TreeViewDiagram {
    match parse(text) {
        Ok(diagram) => diagram,
        Err(error) => panic!("expected the diagram to parse, got {error} for {text:?}"),
    }
}

fn rejects(text: &str) {
    assert!(parse(text).is_err(), "expected a rejection for {text:?}");
}

/// The root's children, with the root of the diagram.
fn top(text: &str) -> Vec<TreeViewNode> {
    parsed(text).root.children
}

/// The node names of a list, in order.
fn names(nodes: &[TreeViewNode]) -> Vec<&str> {
    nodes.iter().map(|n| n.name.as_str()).collect()
}

mod the_root_row {
    use super::*;

    #[test]
    fn root_row_is_always_present_and_is_a_directory_named_slash() {
        // Q1 of section 8.3: the DB's root is the synthetic `/`, drawn by the renderer.
        let diagram = parsed("treeView-beta\na");
        assert_eq!(diagram.root.id, 0);
        assert_eq!(diagram.root.level, -1);
        assert_eq!(diagram.root.name, "/");
        assert_eq!(diagram.root.node_type, NodeType::Directory);
        assert_eq!(names(&diagram.root.children), ["a"]);
    }

    #[test]
    fn an_empty_diagram_has_the_root_alone() {
        // `empty-tree`, `empty-tree-nl` and `only-comment`: the count is 1, the root has no child.
        for text in ["treeView-beta", "treeView-beta\n", "treeView-beta\n%% only"] {
            let diagram = parsed(text);
            assert!(diagram.root.children.is_empty(), "{text:?}");
            assert_eq!(diagram.count, 1, "{text:?}");
        }
    }

    #[test]
    fn count_is_the_number_of_nodes_plus_one() {
        // `getCount()` starts at 1 because the root takes no number, and each node takes the next.
        assert_eq!(parsed("treeView-beta\na\n    b\nc").count, 4);
    }
}

mod q1_title_prefixed_names {
    use super::*;

    #[test]
    fn q1_titlepage_at_the_top_is_a_node_named_page_md() {
        let diagram = parsed("treeView-beta\ntitlepage.md");
        assert_eq!(names(&diagram.root.children), ["page.md"]);
        assert_eq!(diagram.title, "");
    }

    #[test]
    fn q1b_title_dot_is_a_node_named_dot_txt() {
        assert_eq!(names(&top("treeView-beta\ntitle.txt")), [".txt"]);
    }

    #[test]
    fn q1c_an_indented_title_prefixed_name_after_a_node_is_rejected() {
        rejects("treeView-beta\nsrc/\n    titles.ts");
        rejects("treeView-beta\nsrc/\n    title");
    }

    #[test]
    fn q1d_a_title_line_after_a_node_is_rejected() {
        rejects("treeView-beta\na\ntitle X");
    }

    #[test]
    fn q1e_a_title_prefixed_name_after_a_title_line_keeps_the_title_empty() {
        // The last TITLE token is `title` alone, so the header's title is empty and the title stays
        // as the grammar left it (`title-prefix-after-title`).
        let diagram = parsed("treeView-beta\ntitle T\ntitles.md");
        assert_eq!(names(&diagram.root.children), ["s.md"]);
        assert_eq!(diagram.title, "");
    }

    #[test]
    fn q1e_with_front_matter_the_front_matter_title_stays() {
        let diagram = parsed("---\ntitle: FM\n---\ntreeView-beta\ntitles.md");
        assert_eq!(names(&diagram.root.children), ["s.md"]);
        assert_eq!(diagram.title, "FM");
    }
}

mod q2_keyword_line {
    use super::*;

    #[test]
    fn q2_keyword_with_trailing_spaces_is_rejected() {
        rejects("treeView-beta  \na");
    }

    #[test]
    fn q2_keyword_followed_by_text_is_rejected() {
        rejects("treeView-beta foo\na");
    }

    #[test]
    fn q2_keyword_followed_by_a_line_comment_is_rejected() {
        rejects("treeView-beta %% c\na");
    }

    #[test]
    fn q2_keyword_glued_to_a_word_is_rejected() {
        rejects("treeView-betaX\na");
    }

    #[test]
    fn q2_no_keyword_is_rejected() {
        rejects("a\nb");
    }
}

mod q3_keyword_as_a_name {
    use super::*;

    #[test]
    fn q3_an_indented_keyword_is_rejected() {
        rejects("treeView-beta\na\n    treeView-beta");
    }
}

mod q4_blank_lines_are_indentation {
    use super::*;

    #[test]
    fn q4_a_line_of_spaces_gives_the_next_node_its_indentation() {
        // `INDENTATION` is not hidden: the next node takes the spaces, so `b` is a child at level 3.
        let root = parsed("treeView-beta\na\n   \nb").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(names(&root.children[0].children), ["b"]);
        assert_eq!(root.children[0].children[0].level, 3);
    }

    #[test]
    fn q4b_a_line_of_one_tab_gives_the_next_node_level_one() {
        let root = parsed("treeView-beta\na\n\t\nb").root;
        assert_eq!(root.children[0].children[0].name, "b");
        assert_eq!(root.children[0].children[0].level, 1);
    }

    #[test]
    fn q4_a_space_line_before_the_first_node_indents_it() {
        let root = parsed("treeView-beta\n   \na").root;
        assert_eq!(root.children[0].name, "a");
        assert_eq!(root.children[0].level, 3);
    }

    #[test]
    fn q4c_a_space_line_before_a_title_is_rejected() {
        rejects("treeView-beta\n  \ntitle T\na");
    }
}

mod q5_comments {
    use super::*;

    #[test]
    fn q5_an_indented_percent_pair_alone_is_rejected() {
        rejects("treeView-beta\na\n    %%\n    b");
    }

    #[test]
    fn q5b_an_unindented_percent_pair_alone_is_a_comment_between_siblings() {
        let root = parsed("treeView-beta\na\n%%\nb").root;
        assert_eq!(names(&root.children), ["a", "b"]);
        assert_eq!(root.children[0].level, 0);
        assert_eq!(root.children[1].level, 0);
    }

    #[test]
    fn q5c_an_indented_comment_line_is_removed_by_the_core() {
        // `cleanupComments` removes `    %% note`, so `b` is a child of `a` (level 4).
        let root = parsed("treeView-beta\na\n    %% note\n    b").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(names(&root.children[0].children), ["b"]);
        assert_eq!(root.children[0].children[0].level, 4);
    }

    #[test]
    fn q5_consecutive_spaces_in_names_without_emoji_are_kept() {
        // The reference's consecutive-space probe is in an excluded fixture, which holds an emoji;
        // this is the same rule without one (`Q-two-spaces`, section 6.3).
        let root = parsed("treeView-beta\nA   B  C\n    Name  with  runs.md").root;
        assert_eq!(root.children[0].name, "A   B  C");
        assert_eq!(root.children[0].children[0].name, "Name  with  runs.md");
    }

    #[test]
    fn q6_a_percent_pair_inside_a_line_is_part_of_the_name() {
        let root = parsed("treeView-beta\na %% note\nb").root;
        assert_eq!(names(&root.children), ["a %% note", "b"]);
    }

    #[test]
    fn q6_a_trailing_percent_pair_is_part_of_the_name() {
        assert_eq!(names(&top("treeView-beta\na %%")), ["a %%"]);
    }

    #[test]
    fn blank_lines_are_harmless() {
        let root = parsed("treeView-beta\n\na\n\n\n    b\n").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(names(&root.children[0].children), ["b"]);
    }
}

mod q20_line_separators {
    use super::*;

    #[test]
    fn q20_a_line_separator_starts_a_comment_too() {
        // The reference's `cleanupComments` is `^\s*%%…` with the multi-line flag, and `^` matches
        // after U+2028 as well as after a line feed. The comment and its line feed are removed, so
        // `b` continues the name `a` (`docs-port/tree-view/PLAN.md`, section 4).
        let root = parsed("treeView-beta\na\u{2028}%% note\nb").root;
        assert_eq!(names(&root.children), ["a\u{2028}b"]);
    }
}

mod q7_trailing_spaces {
    use super::*;

    #[test]
    fn q7_trailing_spaces_after_a_quoted_name_before_an_indented_line_are_rejected() {
        rejects("treeView-beta\n\"a\"  \n    b");
    }

    #[test]
    fn q7_trailing_spaces_after_a_quoted_name_at_the_end_are_rejected() {
        rejects("treeView-beta\n\"a\"  ");
    }

    #[test]
    fn q7_trailing_spaces_after_a_bare_name_are_not_part_of_it() {
        let root = parsed("treeView-beta\na  \n    b  ").root;
        assert_eq!(root.children[0].name, "a");
        assert_eq!(root.children[0].children[0].name, "b");
        assert_eq!(root.children[0].children[0].level, 4);
        assert_eq!(names(&top("treeView-beta\na   ")), ["a"]);
    }
}

mod q8_quoted_names {
    use super::*;

    #[test]
    fn q8_a_quote_inside_a_quoted_name_ends_it() {
        // `"a"b"` is the quoted name `a` and the bare name `b"`.
        let root = parsed("treeView-beta\n\"a\"b\"").root;
        assert_eq!(names(&root.children), ["a", "b\""]);
    }

    #[test]
    fn q8b_an_unterminated_quote_is_rejected() {
        rejects("treeView-beta\n\"abc");
    }

    #[test]
    fn q8c_an_empty_quoted_name_is_a_file_named_empty() {
        let n = &top("treeView-beta\n\"\"")[0];
        assert_eq!(n.name, "");
        assert_eq!(n.node_type, NodeType::File);
    }

    #[test]
    fn q8d_a_quoted_name_with_a_trailing_slash_is_a_directory() {
        let n = &top("treeView-beta\n\"src/\"")[0];
        assert_eq!(n.name, "src");
        assert_eq!(n.node_type, NodeType::Directory);
    }

    #[test]
    fn q8_a_quoted_name_may_span_lines() {
        // `QUOTED_NAME` is `"[^"]*"`, which takes line breaks.
        assert_eq!(names(&top("treeView-beta\n\"a\nb\"")), ["a\nb"]);
    }
}

mod q9_slashes {
    use super::*;

    #[test]
    fn q9_a_bare_slash_is_a_directory_with_an_empty_name() {
        let n = &top("treeView-beta\n/")[0];
        assert_eq!(n.name, "");
        assert_eq!(n.node_type, NodeType::Directory);
    }

    #[test]
    fn q9b_only_one_trailing_slash_is_removed() {
        let n = &top("treeView-beta\na//")[0];
        assert_eq!(n.name, "a/");
        assert_eq!(n.node_type, NodeType::Directory);
    }

    #[test]
    fn q9_a_directory_with_trailing_blanks_loses_its_slash_only() {
        let n = &top("treeView-beta\na/   ")[0];
        assert_eq!(n.name, "a");
        assert_eq!(n.node_type, NodeType::Directory);
    }
}

mod q10_annotations {
    use super::*;

    #[test]
    fn q10_a_class_needs_a_letter_or_underscore_first() {
        assert_eq!(names(&top("treeView-beta\nj :::1cls")), ["j :::1cls"]);
        assert_eq!(names(&top("treeView-beta\nk :::")), ["k :::"]);
    }

    #[test]
    fn q10b_a_single_valued_field_given_twice_keeps_the_last() {
        let class = &top("treeView-beta\nf :::a :::b")[0];
        assert_eq!(class.css_class, "b");
        let icon = &top("treeView-beta\nf icon(x) icon(y)")[0];
        assert_eq!(icon.icon, "y");
    }

    #[test]
    fn q10c_a_description_takes_the_rest_of_the_line() {
        assert_eq!(
            top("treeView-beta\ne ## desc :::cls")[0].description,
            "desc :::cls"
        );
        assert_eq!(top("treeView-beta\na ## b ## c")[0].description, "b ## c");
    }

    #[test]
    fn q10_a_class_with_a_hyphen_is_kept_whole() {
        assert_eq!(top("treeView-beta\na :::my-cls1")[0].css_class, "my-cls1");
    }

    #[test]
    fn q10_a_directory_takes_a_class_and_an_icon() {
        let n = &top("treeView-beta\nsrc/ :::hl icon(folder)")[0];
        assert_eq!(n.name, "src");
        assert_eq!(n.node_type, NodeType::Directory);
        assert_eq!(n.css_class, "hl");
        assert_eq!(n.icon, "folder");
    }

    #[test]
    fn q10_a_class_with_a_blank_run_after_it_is_a_second_node_under_the_first() {
        // ` extra` after the class is an indentation and a name, so it is a child (level 1).
        let root = parsed("treeView-beta\na :::cls extra").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(root.children[0].css_class, "cls");
        assert_eq!(names(&root.children[0].children), ["extra"]);
        assert_eq!(root.children[0].children[0].level, 1);
    }
}

mod q11_annotations_at_the_start {
    use super::*;

    #[test]
    fn q11_a_line_starting_with_a_class_is_rejected() {
        rejects("treeView-beta\n:::x");
    }

    #[test]
    fn q11_a_line_starting_with_an_icon_is_rejected() {
        rejects("treeView-beta\nicon(x)");
    }

    #[test]
    fn q11_a_line_starting_with_a_description_is_rejected() {
        rejects("treeView-beta\n## x");
    }

    #[test]
    fn q11b_an_icon_with_a_dot_is_rejected() {
        rejects("treeView-beta\ng icon(a.b)");
    }

    #[test]
    fn q11b_an_icon_with_three_parts_is_rejected() {
        rejects("treeView-beta\nl icon(a:b:c)");
    }

    #[test]
    fn q11c_an_empty_icon_and_the_none_icon_both_suppress() {
        assert_eq!(top("treeView-beta\nh icon()")[0].icon, "none");
        assert_eq!(top("treeView-beta\ni icon(none)")[0].icon, "none");
    }
}

mod q12_entities {
    use super::*;

    #[test]
    fn q12_an_entity_in_a_name_is_kept_as_its_placeholder() {
        // `encodeEntities` runs before the parser, and the DB keeps the placeholder.
        assert_eq!(top("treeView-beta\nA #amp; B")[0].name, "A ﬂ°amp¶ß B");
    }

    #[test]
    fn q12_an_entity_in_a_description_is_kept_as_its_placeholder_too() {
        assert_eq!(
            top("treeView-beta\na ## x #amp; y")[0].description,
            "x ﬂ°amp¶ß y"
        );
    }
}

mod q13_sanitising {
    use super::*;

    #[test]
    fn q13_the_title_is_sanitised() {
        assert_eq!(
            parsed("treeView-beta\ntitle A & <b>B</b>\na").title,
            "A &amp; <b>B</b>"
        );
    }

    #[test]
    fn q13_the_accessible_title_is_sanitised() {
        assert_eq!(
            parsed("treeView-beta\naccTitle: A <i>x</i>\na").acc_title,
            "A <i>x</i>"
        );
    }

    #[test]
    fn q13b_a_description_without_markup_is_kept_as_it_is() {
        assert_eq!(top("treeView-beta\na ## a & b")[0].description, "a & b");
        assert_eq!(
            top("treeView-beta\na ## a &amp; b")[0].description,
            "a &amp; b"
        );
    }

    #[test]
    fn q13b_a_description_with_a_tag_keeps_the_tag_as_markup() {
        assert_eq!(
            top("treeView-beta\na ## <b>x</b>")[0].description,
            "<b>x</b>"
        );
    }

    #[test]
    fn q13c_a_name_is_not_sanitised() {
        assert_eq!(top("treeView-beta\na <b>x</b>")[0].name, "a <b>x</b>");
    }
}

mod q14_front_matter_title {
    use super::*;

    #[test]
    fn q14_a_title_line_replaces_the_front_matter_title() {
        assert_eq!(
            parsed("---\ntitle: FM\n---\ntreeView-beta\ntitle Line\na").title,
            "Line"
        );
    }

    #[test]
    fn q14b_an_empty_title_line_keeps_the_front_matter_title() {
        assert_eq!(
            parsed("---\ntitle: FM\n---\ntreeView-beta\ntitle\na").title,
            "FM"
        );
    }

    #[test]
    fn the_last_title_line_wins_and_an_empty_last_one_wins_too() {
        assert_eq!(parsed("treeView-beta\ntitle A\ntitle B\na").title, "B");
        assert_eq!(parsed("treeView-beta\ntitle\na").title, "");
        assert_eq!(parsed("treeView-beta\ntitle My Tree\na").title, "My Tree");
    }

    #[test]
    fn the_front_matter_config_does_not_reach_the_model() {
        // The `config` block is the configuration's (rowIndent and the others are renderer keys).
        let root =
            parsed("---\nconfig:\n  treeView:\n    rowIndent: 20\n---\ntreeView-beta\na").root;
        assert_eq!(names(&root.children), ["a"]);
    }
}

mod q15_levels {
    use super::*;

    #[test]
    fn q15_a_child_then_a_root_item() {
        let root = parsed("treeView-beta\na\n    b\nc").root;
        assert_eq!(names(&root.children), ["a", "c"]);
        assert_eq!(names(&root.children[0].children), ["b"]);
    }

    #[test]
    fn q15a_a_deeper_node_then_a_middle_one_pops_the_deeper_ones() {
        let root = parsed("treeView-beta\na\n    b\n        c\n    d").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(names(&root.children[0].children), ["b", "d"]);
        assert_eq!(names(&root.children[0].children[0].children), ["c"]);
    }

    #[test]
    fn q15b_uneven_indentation_makes_a_child_of_the_nearest_smaller_level() {
        // Levels 0, 2, 1: `c` (level 1) pops `b` (level 2), and `a` (level 0) stays, so `c` is a
        // child of `a`.
        let root = parsed("treeView-beta\na\n  b\n c").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(names(&root.children[0].children), ["b", "c"]);
    }

    #[test]
    fn q15c_the_first_node_indented_is_still_a_root_item_at_its_own_level() {
        let root = parsed("treeView-beta\n        a\n    b\nc").root;
        assert_eq!(names(&root.children), ["a", "b", "c"]);
        assert_eq!(root.children[0].level, 8);
    }

    #[test]
    fn q15d_a_tab_counts_as_one_character() {
        let root = parsed("treeView-beta\n\ta\n    b").root;
        assert_eq!(names(&root.children), ["a"]);
        assert_eq!(names(&root.children[0].children), ["b"]);
        assert_eq!(root.children[0].level, 1);
    }

    #[test]
    fn crlf_line_breaks_are_read_as_line_breaks() {
        let root = parsed("treeView-beta\r\na\r\n    b\r\n").root;
        assert_eq!(names(&root.children[0].children), ["b"]);
    }
}

mod q16_box_drawing_branch_in_a_name {
    use super::*;

    #[test]
    fn q16_a_branch_character_inside_a_name_drops_the_text_before_it() {
        // `    b ├ c` is read as a branch at column 6: the text before the branch is lost.
        let root = parsed("treeView-beta\nsrc\n    b ├ c").root;
        assert_eq!(names(&root.children), ["src"]);
        assert_eq!(names(&root.children[0].children), ["c"]);
        assert_eq!(root.children[0].children[0].level, 8);
    }
}

mod q18_title_in_a_box_drawing_tree {
    use super::*;

    #[test]
    fn q18_a_title_line_between_the_keyword_and_a_box_tree_is_read() {
        let diagram = parsed("treeView-beta\ntitle T\n.\n└── a");
        assert_eq!(diagram.title, "T");
        assert_eq!(names(&diagram.root.children), ["."]);
        assert_eq!(names(&diagram.root.children[0].children), ["a"]);
    }
}

mod q19_names_with_runs_of_spaces {
    use super::*;

    #[test]
    fn q19_a_name_keeps_both_runs_of_spaces_and_its_unicode() {
        let root = parsed("treeView-beta\n  Ärger 🙂.md\n    ÉÜ  double  space").root;
        assert_eq!(root.children[0].name, "Ärger 🙂.md");
        assert_eq!(root.children[0].level, 2);
        assert_eq!(root.children[0].children[0].name, "ÉÜ  double  space");
    }
}

mod headers {
    use super::*;

    #[test]
    fn accessible_title_and_description_are_read() {
        let diagram = parsed("treeView-beta\naccTitle: T1\naccDescr: D1\na");
        assert_eq!(diagram.acc_title, "T1");
        assert_eq!(diagram.acc_descr, "D1");
    }

    #[test]
    fn a_block_description_loses_the_indentation_of_each_line() {
        let diagram = parsed("treeView-beta\naccDescr {\n  line one\n  line two\n}\na");
        assert_eq!(diagram.acc_descr, "line one\nline two");
    }

    #[test]
    fn accessibility_words_without_a_colon_are_names() {
        assert_eq!(names(&top("treeView-beta\naccTitles.md")), ["accTitles.md"]);
        assert_eq!(
            names(&top("treeView-beta\naccDescribed.md")),
            ["accDescribed.md"]
        );
    }
}
