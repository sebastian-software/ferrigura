//! The quirks of the treemap lexer and grammar that the port reproduces, each as a direct assertion
//! on the model or on a rejection. The list and its reasons are in `docs-port/treemap/PLAN.md`
//! (sections 2 and 3), and `docs-port/treemap/NOTES.md` section 2.8. The differential run
//! (`pnpm port:diff treemap`) checks the same behaviour against the reference.

use ferrigura_model::treemap::{TreemapDiagram, TreemapNode};
use ferrigura_syntax::treemap::parse;

fn accepts(input: &str) -> TreemapDiagram {
    parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"))
}

fn rejects(input: &str) {
    assert!(parse(input).is_err(), "{input:?} should be rejected");
}

fn roots(diagram: &TreemapDiagram) -> &[TreemapNode] {
    diagram.tree.children.as_deref().unwrap_or_default()
}

fn children(node: &TreemapNode) -> &[TreemapNode] {
    node.children.as_deref().unwrap_or_default()
}

#[test]
fn the_keyword_is_followed_by_no_word_so_treemapx_is_rejected() {
    rejects("treemapX\n\"A\": 1");
    rejects("treemap\nA: 1");
}

#[test]
fn a_name_is_quoted_and_may_be_empty() {
    let diagram = accepts("treemap\n\"\"");
    assert_eq!(roots(&diagram)[0].name, "");
    let diagram = accepts("treemap\n'single quoted': 3");
    assert_eq!(roots(&diagram)[0].name, "single quoted");
}

#[test]
fn values_keep_digits_commas_dots_and_underscores_as_parse_float_reads_them() {
    let value = |text: &str| roots(&accepts(text))[0].value;
    assert_eq!(value("treemap\n\"A\": 1,2,3"), Some(123.0));
    assert_eq!(value("treemap\n\"A\": 1_000"), Some(1.0));
    assert_eq!(value("treemap\n\"A\": 1..2"), Some(1.0));
    assert_eq!(value("treemap\n\"A\": .5"), Some(0.5));
    assert_eq!(value("treemap\n\"A\": 007.50"), Some(7.5));
    assert_eq!(value("treemap\n\"A\": 1,000.5"), Some(1000.5));
}

#[test]
fn a_value_of_only_separators_is_nan_and_its_text_is_the_sentinel() {
    let leaf = roots(&accepts("treemap\n\"A\": ._."))[0].clone();
    assert!(leaf.value.is_some_and(f64::is_nan));
    rejects("treemap\n\"A\": _");
}

#[test]
fn numbers_with_signs_exponents_or_no_separator_are_rejected() {
    for text in [
        "treemap\n\"A\": -5",
        "treemap\n\"A\": 1e3",
        "treemap\n\"A\": 1 2",
        "treemap\n\"A\":",
        "treemap\n\"A\"  1",
        "treemap\n\"A\" :::cc",
        "treemap\n\"A\": 1:::a:::b",
    ] {
        rejects(text);
    }
}

#[test]
fn a_comma_is_a_separator_and_a_number_may_run_into_the_next_name() {
    let diagram = accepts("treemap\n\"A\",1");
    assert_eq!(roots(&diagram)[0].value, Some(1.0));
    let diagram = accepts("treemap\n\"A\": 1,\"B\": 2");
    assert_eq!(roots(&diagram).len(), 2);
}

#[test]
fn a_trailing_blank_is_the_indentation_of_the_next_row() {
    // `S` is a section at level 0; `B` takes the trailing blank of `A`'s line as its indentation, so
    // it is a child of `S` and not a root.
    let diagram = accepts("treemap\n\"S\"\n  \"A\": 1 \n\"B\": 2");
    let section = &roots(&diagram)[0];
    let names: Vec<&str> = children(section)
        .iter()
        .map(|node| node.name.as_str())
        .collect();
    assert_eq!(names, ["A", "B"]);
}

#[test]
fn two_indentation_tokens_in_a_row_are_rejected() {
    rejects("treemap\n\"A\": 1  \n  \"B\": 2");
}

#[test]
fn a_trailing_blank_at_the_end_of_the_text_is_rejected() {
    rejects("treemap\n\"A\" : 1 \n");
}

#[test]
fn a_comment_line_removes_its_blanks() {
    // The comment line's blanks are removed with it, so `A` is at level 0 and not under `S`.
    let diagram = accepts("treemap\n\"S\"\n  %% c\n\"A\": 1");
    let names: Vec<&str> = roots(&diagram)
        .iter()
        .map(|node| node.name.as_str())
        .collect();
    assert_eq!(names, ["S", "A"]);
}

#[test]
fn tabs_count_as_one_character_of_indentation() {
    let diagram = accepts("treemap\n\t\"S\"\n\t\t\"A\": 1");
    assert_eq!(children(&roots(&diagram)[0]).len(), 1);
}

#[test]
fn a_bare_class_def_swallows_the_next_line_as_its_style_text() {
    let diagram = accepts("treemap\nclassDef cc\n\"A\":::cc");
    assert!(roots(&diagram).is_empty(), "A is part of the style text");
    assert_eq!(diagram.classes[0].styles, ["\"A\":::cc"]);
}

#[test]
fn a_class_def_name_needs_two_characters() {
    rejects("treemap\nclassDef c fill:red");
    rejects("treemap\nclassDef a-b fill:red");
    rejects("treemap\nclassDef 1a fill:red");
    let diagram = accepts("treemap\nclassDef _x fill:red\n\"A\":::_x");
    assert_eq!(diagram.classes[0].id, "_x");
}

#[test]
fn a_class_def_ends_at_a_semicolon_and_keeps_its_commas() {
    let diagram = accepts("treemap\nclassDef cc fill:red;\n\"A\":::cc");
    assert_eq!(diagram.classes[0].styles, ["fill:red"]);
    let diagram = accepts("treemap\nclassDef cc fill:red, stroke:blue\n\"A\":::cc");
    assert_eq!(diagram.classes[0].styles, ["fill:red", " stroke:blue"]);
}

#[test]
fn a_repeated_class_def_accumulates_its_styles() {
    let diagram = accepts("treemap\nclassDef cc fill:red\nclassDef cc stroke:blue\n\"A\":::cc");
    assert_eq!(diagram.classes.len(), 1);
    assert_eq!(diagram.classes[0].styles, ["fill:red", "stroke:blue"]);
}

#[test]
fn a_class_defined_after_its_use_styles_the_item() {
    let diagram = accepts("treemap\n\"A\":::cc\nclassDef cc fill:red");
    let node = &roots(&diagram)[0];
    assert_eq!(node.css_compiled_styles, Some(vec!["fill:red".to_owned()]));
}

#[test]
fn an_unknown_class_gives_no_styles() {
    let diagram = accepts("treemap\n\"A\":::nope");
    let node = &roots(&diagram)[0];
    assert_eq!(node.class_selector.as_deref(), Some("nope"));
    assert_eq!(node.css_compiled_styles, None);
}

#[test]
fn a_class_selector_may_be_a_word_that_starts_like_a_keyword() {
    let diagram = accepts("treemap\n\"A\":::true");
    assert_eq!(roots(&diagram)[0].class_selector.as_deref(), Some("true"));
    rejects("treemap\n\"A\":::treemap");
    rejects("treemap\n\"A\":::title");
}

#[test]
fn names_are_not_sanitised() {
    let diagram = accepts("treemap\n\"<b>C</b>\": 1");
    assert_eq!(roots(&diagram)[0].name, "<b>C</b>");
}

#[test]
fn two_roots_are_accepted() {
    let diagram = accepts("treemap\n\"Root1\"\n\"Root2\"");
    assert_eq!(roots(&diagram).len(), 2);
}

#[test]
fn the_last_title_wins_and_a_title_may_sit_inside_a_row() {
    let diagram = accepts("treemap\ntitle A\ntitle B");
    assert_eq!(diagram.title, "B");
    let diagram = accepts("treemap\n\"A\" title T\n\"B\": 1");
    assert_eq!(diagram.title, "T");
    assert_eq!(roots(&diagram).len(), 2);
}
