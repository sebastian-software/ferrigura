//! The quirks of the venn grammar and DB that the port reproduces, each with its probe, as
//! `docs-port/venn/PLAN.md` lists them. The inputs are the ones the differential run checks against
//! the reference (`tools/port/cases.ts`), so each expectation is the reference's own output.

use ferrigura_model::venn::{StyleEntry, Subset, TextNode, VennDiagram};
use ferrigura_syntax::venn::parse;

fn accepts(text: &str) -> VennDiagram {
    parse(text).expect("the reference accepts this text")
}

fn rejects(text: &str) {
    assert!(parse(text).is_err(), "the reference rejects {text:?}");
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|id| (*id).to_owned()).collect()
}

fn subset(sets: &[&str], size: f64) -> Subset {
    Subset {
        sets: ids(sets),
        size,
        label: String::new(),
    }
}

fn node(sets: &[&str], id: &str) -> TextNode {
    TextNode {
        sets: ids(sets),
        id: id.to_owned(),
        label: String::new(),
    }
}

/// V1: a `set` or `union` turns the indent mode on, and a line that begins with a non-blank
/// character turns it off (rule 19), so an indented `text` belongs to the last `set` or `union`
/// only while nothing else starts a line.
#[test]
fn v1_an_indented_text_joins_the_last_set_or_union() {
    let diagram = accepts("venn-beta\nset A\n  text \"x\"");
    assert_eq!(diagram.text_nodes, [node(&["A"], "x")]);
}

#[test]
fn v1_a_style_line_turns_the_indent_mode_off() {
    rejects("venn-beta\nset A\nstyle A fill:red\n  text \"x\"");
}

#[test]
fn v1_a_column_zero_text_line_turns_the_indent_mode_off() {
    rejects("venn-beta\nset A\ntext A \"x\"\n  text \"y\"");
}

/// V1, with the comment rules: a comment line leaves the indent mode as it is, and the newline
/// before it is eaten with the comment (rule 8), so the next line still begins in `bol`.
#[test]
fn v1_a_comment_line_leaves_the_indent_mode_on() {
    let diagram = accepts("venn-beta\nset A\n%% c\n  text \"x\"");
    assert_eq!(diagram.text_nodes, [node(&["A"], "x")]);
}

#[test]
fn v1_a_comment_glued_to_a_character_takes_the_character_and_keeps_the_indent() {
    let diagram = accepts("venn-beta\nset A\nx%% c\n  text \"y\"");
    assert_eq!(diagram.text_nodes, [node(&["A"], "y")]);
}

#[test]
fn v1_a_directive_at_the_start_of_a_line_turns_the_indent_mode_off() {
    rejects("venn-beta\nset A\n%%{ bad\n  text \"x\"");
}

/// V1: an indented word that starts with `text` but not `text\b` is no indented text, so the blanks
/// turn the indent mode off, and the word is an identifier at the start of a statement.
#[test]
fn v1_an_indented_textual_word_is_an_identifier_and_an_error() {
    rejects("venn-beta\nset A\n  textual \"x\"");
}

/// V2: a comment glued to a token takes the token's last character (`[^\}]\%\%[^\n]*`), so
/// `A%%` is no identifier, while a comment after a blank is harmless.
#[test]
fn v2_a_comment_glued_to_an_identifier_is_a_parse_error() {
    rejects("venn-beta\nset A%% c");
}

#[test]
fn v2_a_comment_after_a_blank_is_harmless() {
    let diagram = accepts("venn-beta\nset A %% c");
    assert_eq!(diagram.subsets, [subset(&["A"], 10.0)]);
}

/// V3: statements need no separator.
#[test]
fn v3_two_statements_on_one_line() {
    let diagram = accepts("venn-beta set A set B");
    assert_eq!(
        diagram.subsets,
        [subset(&["A"], 10.0), subset(&["B"], 10.0)]
    );
}

/// V4: a title stops at `#` or `;`, which is then a lexical error, and keeps its trailing blanks.
#[test]
fn v4_a_hash_ends_the_title_and_is_an_error() {
    rejects("venn-beta\ntitle foo#");
}

#[test]
fn v4_a_semicolon_ends_the_title_and_is_an_error() {
    rejects("venn-beta\ntitle foo;");
}

#[test]
fn v4_a_title_keeps_its_comment_text_and_its_trailing_blanks() {
    assert_eq!(accepts("venn-beta\ntitle foo %% c").title, "foo %% c");
    assert_eq!(accepts("venn-beta\ntitle T   ").title, "T   ");
}

/// V5: a union needs two identifiers, and each must be a `set` given before it.
#[test]
fn v5_a_union_needs_known_sets() {
    rejects("venn-beta\nunion A,B\nset A\nset B");
    rejects("venn-beta\nset A\nunion A,C");
}

#[test]
fn v5_a_union_of_one_is_an_error() {
    rejects("venn-beta\nunion A");
}

/// V6: a second `set` of the same id is a second subset, and a union may repeat an id.
#[test]
fn v6_duplicate_statements_are_kept() {
    let diagram = accepts("venn-beta\nset A\nset A\nset B\nset C\nunion A,A");
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 10.0),
            subset(&["A"], 10.0),
            subset(&["B"], 10.0),
            subset(&["C"], 10.0),
            subset(&["A", "A"], 2.5),
        ]
    );
}

/// V7: a text node's sets are sorted, as a union's are.
#[test]
fn v7_text_sets_are_sorted() {
    let diagram = accepts("venn-beta\nset A\nset B\nunion A,B\n  text \"x\"\ntext B,A \"y\"");
    assert_eq!(
        diagram.text_nodes,
        [node(&["A", "B"], "x"), node(&["A", "B"], "y")]
    );
}

/// V8: the size of a statement without one is `10 / n²`, for the `n` identifiers as written.
#[test]
fn v8_the_default_size_is_ten_over_n_squared() {
    let diagram = accepts("venn-beta\nset A\nset B\nset C\nunion A,B,C");
    assert_eq!(diagram.subsets[3].size, 10.0 / 9.0);
    assert_eq!(diagram.subsets[0].size, 10.0);
}

/// V9: quotes are removed, and a label or id is trimmed before the quotes are looked at.
#[test]
fn v9_quotes_are_removed_and_labels_are_trimmed() {
    let diagram = accepts("venn-beta\nset A[\"  x  \"]\nset B[ x ]\nset C[\"\"]\nset D[ ]");
    let labels: Vec<&str> = diagram.subsets.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["x", "x", "", ""]);
}

/// V10 (Q7): a negative size is kept, not rejected; the renderer draws it as `NaN`.
#[test]
fn v10_a_negative_size_is_kept() {
    let diagram = accepts("venn-beta\nset A:-5");
    assert_eq!(diagram.subsets, [subset(&["A"], -5.0)]);
}

#[test]
fn v10_a_zero_size_is_kept() {
    let diagram = accepts("venn-beta\nset A:0\nset B\nunion A,B");
    assert_eq!(diagram.subsets[0].size, 0.0);
}

/// V11: the keywords `venn-beta`, `set`, `union`, `text` and `style` need a word boundary after
/// them (Jison appends `\b` to a pattern that ends in a word character), so `setA` is an identifier.
#[test]
fn v11_a_set_keyword_glued_to_an_identifier_is_an_identifier() {
    rejects("venn-beta\nsetA");
    rejects("venn-beta\nsettings");
    rejects("venn-beta\nset A\nset B\nunionA,B");
}

#[test]
fn v11_a_style_keyword_glued_to_an_identifier_is_an_identifier() {
    rejects("venn-beta\nset A\nstyleA fill:red");
}

#[test]
fn v11_a_set_followed_by_a_word_that_starts_with_text_is_one_identifier() {
    let diagram = accepts("venn-beta\nset textbook");
    assert_eq!(diagram.subsets, [subset(&["textbook"], 10.0)]);
}

/// V12: the ids of a subset are sorted as JavaScript's default sort sorts them, by code units, so
/// `10` comes before `2`; the subsets keep the order they were given in.
#[test]
fn v12_numeric_ids_sort_as_strings_within_a_subset() {
    let diagram = accepts("venn-beta\nset \"2\"\nset \"10\"\nunion \"10\",\"2\"");
    assert_eq!(diagram.subsets[0].sets, ids(&["2"]));
    assert_eq!(diagram.subsets[1].sets, ids(&["10"]));
    assert_eq!(diagram.subsets[2].sets, ids(&["10", "2"]));
}

/// V13: a text id that is a number takes no bracket label, as the grammar has it.
#[test]
fn v13_a_numeric_text_id_takes_no_label() {
    rejects("venn-beta\nset A\ntext A 12[\"x\"]");
    rejects("venn-beta\nset A\n  text 12[\"x\"]");
}

/// V14: the styles are an object in the reference, so a key given twice keeps its first place and
/// takes its last value, and the key `__proto__` is dropped.
#[test]
fn v14_a_repeated_style_key_keeps_its_place_and_takes_the_last_value() {
    let diagram = accepts("venn-beta\nset A\nstyle A fill:red,color:x,fill:blue");
    assert_eq!(
        diagram.styles,
        [StyleEntry {
            targets: ids(&["A"]),
            styles: vec![
                ("fill".to_owned(), "blue".to_owned()),
                ("color".to_owned(), "x".to_owned()),
            ],
        }]
    );
}

#[test]
fn v14_the_proto_key_is_dropped() {
    let diagram = accepts("venn-beta\nset A\nstyle A __proto__:red,toString:x");
    assert_eq!(
        diagram.styles,
        [StyleEntry {
            targets: ids(&["A"]),
            styles: vec![("toString".to_owned(), "x".to_owned())],
        }]
    );
}

/// V15: a character outside the BMP is no token, and a comment does not start after it.
#[test]
fn v15_a_non_bmp_character_before_a_comment_is_an_error() {
    rejects("venn-beta\n\u{1F600}%% c\nset A");
}

/// V16: the parser reads the keywords without regard to case, though the detector does not.
#[test]
fn v16_the_keywords_are_case_insensitive() {
    let diagram = accepts("VENN-BETA\nSET A\nSET B\nUNION A,B");
    assert_eq!(diagram.subsets.len(), 3);
}

#[test]
fn v16_venn_beta_glued_to_a_word_is_an_error() {
    rejects("venn-betax\nset A");
}

/// V17: a label is the DB's string: its markup is raw, and an entity placeholder stays.
#[test]
fn v17_a_label_keeps_its_markup_and_its_placeholders() {
    let diagram = accepts("venn-beta\nset A[\"<b>x</b> &amp; y\"]\nset B[\"#quot;\"]");
    assert_eq!(diagram.subsets[0].label, "<b>x</b> &amp; y");
    assert_eq!(diagram.subsets[1].label, "ﬂ°quot¶ß");
}

/// V18: the title is sanitised, and a script goes with its content.
#[test]
fn v18_the_title_is_sanitised() {
    assert_eq!(
        accepts("venn-beta\ntitle <b>x</b> & y  ").title,
        "<b>x</b> &amp; y  "
    );
    assert_eq!(
        accepts("venn-beta\ntitle <script>alert(1)</script>").title,
        ""
    );
    assert_eq!(accepts("venn-beta\ntitle AT&T").title, "AT&T");
}

/// V19: a title given later replaces an earlier one, and a front matter title is the default.
#[test]
fn v19_the_grammar_title_replaces_the_front_matter_title() {
    assert_eq!(
        accepts("---\ntitle: fm title\n---\nvenn-beta\ntitle grammar\nset A").title,
        "grammar"
    );
    assert_eq!(
        accepts("---\ntitle: fm title\n---\nvenn-beta\nset A").title,
        "fm title"
    );
}

/// V20: a bare `venn-beta` is accepted with no subsets (the renderer is the one that fails).
#[test]
fn v20_a_bare_header_is_accepted() {
    let diagram = accepts("venn-beta");
    assert!(diagram.subsets.is_empty());
    assert!(diagram.text_nodes.is_empty());
    assert!(diagram.styles.is_empty());
}

/// V21: a non-breaking space is not a blank for the lexer, so it begins no statement.
#[test]
fn v21_a_non_breaking_space_is_not_a_blank() {
    rejects("venn-beta\n\u{A0}set A");
}
