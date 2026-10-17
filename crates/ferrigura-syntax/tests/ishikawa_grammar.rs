//! The quirks of the ishikawa grammar and its lexer that the port keeps, one test per row of the
//! table in `docs-port/ishikawa/PLAN.md`, section 2 (I1 to I26). Each expected value is the
//! reference's: the notes record the Jison runs that verified it, and the differential run checks the
//! same inputs against the reference.

use ferrigura_model::ishikawa::{IshikawaDiagram, IshikawaNode};
use ferrigura_syntax::ishikawa::parse;

fn parsed(text: &str) -> IshikawaDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

fn rejected(text: &str) {
    assert!(parse(text).is_err(), "{text:?} should be rejected");
}

/// The tree as text: a node's text, then its children in brackets, separated by spaces.
fn shape(node: &IshikawaNode) -> String {
    if node.children.is_empty() {
        return node.text.clone();
    }
    let children: Vec<String> = node.children.iter().map(shape).collect();
    format!("{}({})", node.text, children.join(" "))
}

fn tree_of(text: &str) -> String {
    let diagram = parsed(text);
    diagram.root.as_ref().map(shape).unwrap_or_default()
}

/// I1: the header alone is a parse error: the grammar needs one statement after it.
#[test]
fn i1_a_bare_header_is_an_error() {
    rejected("ishikawa");
}

/// I2: a header followed by a line break is accepted, with no root. The trailing break of the text
/// and the one the parser adds make a SPACELINE token, which is a statement that calls no DB method.
#[test]
fn i2_a_header_and_a_line_break_have_no_root() {
    for text in ["ishikawa\n", "ishikawa\n\n\n", "ishikawa ", "ishikawa\n  "] {
        let diagram = parsed(text);
        assert!(diagram.root.is_none(), "{text:?} has no root");
        assert_eq!(diagram.title, "", "{text:?} has no title");
    }
}

/// I3: the header's word boundary: `ishikawaEffect` is text, not the header.
#[test]
fn i3_a_header_glued_to_a_word_is_not_the_header() {
    rejected("ishikawaEffect\nCause\n");
}

/// I4: the header line's own text is the first statement, and it is the root.
#[test]
fn i4_the_header_line_text_is_the_root() {
    assert_eq!(tree_of("ishikawa Effect\n  Cause A\n"), "Effect(Cause A)");
}

/// I5: `ishikawa` matches with its `\b` before the hyphen, and `-betafoo` is text.
#[test]
fn i5_a_glued_beta_keyword_takes_the_plain_header() {
    assert_eq!(tree_of("ishikawa-betafoo\nP\n"), "-betafoo(P)");
}

/// I6: blanks after the header and before its line break are a SPACELINE, which the grammar does not
/// accept after the header.
#[test]
fn i6_blanks_after_the_header_are_an_error() {
    rejected("ishikawa  \nP\n  a\n");
}

/// I7: the lexer and the detector are case-insensitive.
#[test]
fn i7_the_header_is_case_insensitive() {
    assert_eq!(tree_of("ISHIKAWA\nP\n  a\n"), "P(a)");
}

/// I8: a second header is an error.
#[test]
fn i8_a_second_header_is_an_error() {
    rejected("ishikawa\nP\nishikawa\nQ\n");
}

/// I9: blank lines and whitespace-only lines before the header are ignored.
#[test]
fn i9_blank_lines_before_the_header_are_ignored() {
    assert_eq!(tree_of("\n\n  \nishikawa\nP\n  a\n"), "P(a)");
    assert_eq!(tree_of("  ishikawa\nP\n  a\n"), "P(a)");
}

/// I10: a `%%` line between causes, or indented, is a comment and is ignored.
#[test]
fn i10_comment_lines_are_ignored() {
    assert_eq!(tree_of("ishikawa\nP\n%% comment\n  a\n"), "P(a)");
    assert_eq!(tree_of("ishikawa\nP\n  %% c\n  a\n"), "P(a)");
}

/// I11: a `%%` inside a text does not start a comment: the text is kept.
#[test]
fn i11_a_percent_pair_inside_a_text_is_text() {
    assert_eq!(tree_of("ishikawa\nP\n  50%% done\n"), "P(50%% done)");
}

/// I12: each tab is one level unit.
#[test]
fn i12_each_tab_is_one_level_unit() {
    assert_eq!(tree_of("ishikawa\nP\n\ta\n\t\tb\n"), "P(a(b))");
}

/// I13: the first cause sets the base level, and a dedent to a level of zero or less is level 1, so
/// both causes are children of the head.
#[test]
fn i13_the_first_cause_sets_the_base_level() {
    assert_eq!(tree_of("ishikawa\nP\n    a\n  b\n"), "P(a b)");
}

/// I14: a deeper line is a child of the cause above it, and a return to the base level is a sibling.
#[test]
fn i14_nested_causes_follow_the_indentation() {
    assert_eq!(tree_of("ishikawa\nP\n  a\n    x\n  b\nQ\n"), "P(a(x) b Q)");
}

/// I15: the first statement after the header is the root, whatever its indentation.
#[test]
fn i15_an_indented_first_statement_is_the_root() {
    assert_eq!(tree_of("ishikawa\n  a\n"), "a");
}

/// I16: `accTitle:`, `accDescr:` and `title` lines are ordinary causes, and the header texts stay empty.
#[test]
fn i16_accessibility_and_title_lines_are_causes() {
    let diagram = parsed("ishikawa\nP\naccTitle: x\naccDescr: y\n");
    assert_eq!(
        tree_of("ishikawa\nP\naccTitle: x\naccDescr: y\n"),
        "P(accTitle: x accDescr: y)"
    );
    assert_eq!(diagram.acc_title, "");
    assert_eq!(diagram.acc_descr, "");
}

/// I17: the markup of a line break is kept as the sanitiser writes it: `<br/>` becomes `<br>`.
#[test]
fn i17_line_break_markup_is_kept_as_written() {
    assert_eq!(tree_of("ishikawa\nP<br>Q\n  a<br/>b\n"), "P<br>Q(a<br>b)");
}

/// I18: a text with `<` goes through DOMPurify, which writes the `&` of the text as `&amp;`.
#[test]
fn i18_a_text_with_markup_is_sanitised() {
    assert_eq!(
        tree_of("ishikawa\nP & <b>x</b>\n  a\n"),
        "P &amp; <b>x</b>(a)"
    );
    assert_eq!(tree_of("ishikawa\nA & B\n"), "A & B");
}

/// I19: a semicolon is ordinary text.
#[test]
fn i19_a_semicolon_is_text() {
    assert_eq!(tree_of("ishikawa\nP;x\n  a;b\n"), "P;x(a;b)");
}

/// I20: CRLF line endings are read as LF.
#[test]
fn i20_crlf_line_endings_are_accepted() {
    assert_eq!(tree_of("ishikawa\r\nP\r\n  a\r\n"), "P(a)");
}

/// I21: a line of only a no-break space is blank, since JavaScript's `\s` takes it.
#[test]
fn i21_a_no_break_space_line_is_blank() {
    assert_eq!(tree_of("ishikawa\nP\n\u{a0}\n  a\n"), "P(a)");
}

/// I22: a cause at the same indentation as the one before it is its child, since the first cause is
/// at level 1 and so is the next one.
#[test]
fn i22_the_same_indentation_gives_a_child() {
    assert_eq!(tree_of("ishikawa\nP\nQ\n"), "P(Q)");
}

/// I23: a blank line between causes is ignored.
#[test]
fn i23_a_blank_line_between_causes_is_ignored() {
    assert_eq!(tree_of("ishikawa\nP\n\n  a\n"), "P(a)");
}

/// I24: a text with no line break at its end is accepted: the end of the text is a stop.
#[test]
fn i24_no_final_line_break_is_accepted() {
    assert_eq!(tree_of("ishikawa\nP\n  a"), "P(a)");
}

/// I25: front matter before the header is removed, and its title is the title until the head sets it.
#[test]
fn i25_front_matter_is_removed_before_the_header() {
    let diagram = parsed("---\ntitle: X\n---\nishikawa\nP\n  a\n");
    assert_eq!(shape(diagram.root.as_ref().expect("a root")), "P(a)");
    assert_eq!(diagram.title, "P");
}

/// I26: a directive before the header is removed.
#[test]
fn i26_a_directive_before_the_header_is_removed() {
    assert_eq!(
        tree_of("%%{init: {\"theme\": \"forest\"}}%%\nishikawa\nP\n  a\n"),
        "P(a)"
    );
}
