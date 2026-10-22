//! The lexer and grammar quirks of the architecture diagram, pinned on the reference's behaviour
//! (`docs-port/architecture/NOTES.md`, section 2.3, probed with `mermaid.parse()`, and the probes of
//! `tools/port/cases.ts`, which the differential run compares on every input). Each test names the
//! row of the table it covers. The reference's own grammar specs are in `architecture_spec.rs`.
//!
//! The lexer's order is Langium's: keywords first, longest first, then the terminals in the order
//! of the reachable rules (`docs-port/architecture/PLAN.md`, section 2). A word that begins with a
//! side letter or with `title` breaks the ids that begin with it, and so on; the tests below say
//! which.

use ferrigura_syntax::architecture::parse;

const HEADER: &str = "architecture-beta\n";

fn accepts(text: &str) {
    if let Err(error) = parse(text) {
        panic!("expected the diagram to parse, got {error} for {text:?}");
    }
}

fn rejects(text: &str) {
    assert!(parse(text).is_err(), "expected a rejection for {text:?}");
}

/// The header, then two services `a` and `b`, then the given lines.
fn with_services(lines: &str) -> String {
    format!("{HEADER}service a(server)[A]\nservice b(server)[B]\n{lines}")
}

#[test]
fn q1_both_header_keywords_are_accepted() {
    accepts("architecture\nservice a(server)[A]\n");
}

#[test]
fn q2_exact_keyword_ids_are_rejected() {
    rejects(&format!("{HEADER}service L(server)[A]\n"));
    rejects(&format!("{HEADER}service group(server)[A]\n"));
    rejects(&format!("{HEADER}service in(server)[A]\n"));
    rejects(&format!("{HEADER}service row(server)[A]\n"));
    rejects(&format!("{HEADER}service align(server)[A]\n"));
}

#[test]
fn q3_a_keyword_followed_by_word_characters_is_an_id() {
    accepts(&format!(
        "{HEADER}service rowspan(server)[A]\nservice columnar(server)[B]\n"
    ));
}

#[test]
fn q4_an_id_cannot_end_with_a_dash() {
    rejects(&format!("{HEADER}service a-(server)[A]\n"));
}

#[test]
fn q5_ids_may_hold_dashes_underscores_and_start_with_a_digit() {
    accepts(&format!(
        "{HEADER}service a-b_c(server)[A]\nservice 1a(server)[B]\n"
    ));
}

#[test]
fn q6_keywords_are_case_sensitive() {
    rejects(&format!("{HEADER}Service a(server)[A]\n"));
}

#[test]
fn q7_an_icon_text_is_written_without_parentheses() {
    accepts(&format!("{HEADER}service a \"Hello\"[A]\n"));
}

#[test]
fn q8_the_icon_does_not_match_quotes() {
    rejects(&format!("{HEADER}service a(\"Hello\")[A]\n"));
}

#[test]
fn q9_the_icon_has_no_spaces() {
    rejects(&format!("{HEADER}service a( server )[A]\n"));
}

#[test]
fn q10_a_title_needs_one_character() {
    rejects(&format!("{HEADER}service a(server)[]\n"));
}

#[test]
fn q11_titles_may_hold_brackets_quotes_and_commas() {
    accepts(&format!("{HEADER}service a(server)[A (b)]\n"));
    accepts(&format!("{HEADER}service a(server)['A x']\n"));
    accepts(&format!("{HEADER}service a(server)[\"A quoted, title\"]\n"));
}

#[test]
fn q12_colons_are_part_of_the_icon_name() {
    accepts(&format!("{HEADER}service a(logos:aws)[A]\n"));
    accepts(&format!("{HEADER}service a(nope:thing)[A]\n"));
}

#[test]
fn q13_a_title_is_optional() {
    accepts(&format!("{HEADER}service a(server)\n"));
}

#[test]
fn q14_a_comment_may_follow_a_statement() {
    accepts(&format!("{HEADER}service a(server)[A] %% note\n"));
}

#[test]
fn q15_each_statement_needs_its_own_line() {
    rejects(&format!(
        "{HEADER}service a(server)[A] service b(server)[B]\n"
    ));
}

#[test]
fn q16_to_q18_edges_with_the_arrow_forms_and_the_same_side_twice() {
    accepts(&with_services("a:R -- L:b\n"));
    accepts(&with_services("a:R --> L:b\n"));
    accepts(&with_services("a:R <-- L:b\n"));
    accepts(&with_services("a:R <--> L:b\n"));
    accepts(&with_services("a:L <-- R:b\n"));
    // The grammar and the DB do not check that the two sides differ.
    accepts(&with_services("a:R -- R:b\n"));
}

#[test]
fn q19_an_edge_needs_its_ports() {
    rejects(&with_services("a -- b\n"));
}

#[test]
fn q20_directions_are_upper_case_letters_of_the_four_sides() {
    rejects(&with_services("a:X -- L:b\n"));
    rejects(&with_services("a:l -- r:b\n"));
}

#[test]
fn q21_an_edge_label_may_hold_spaces() {
    accepts(&with_services("a:R -[x y]- L:b\n"));
}

#[test]
fn q22_an_empty_edge_label_is_a_lexer_error() {
    rejects(&with_services("a:R -[]- L:b\n"));
}

#[test]
fn q23_three_dashes_are_not_an_edge() {
    rejects(&with_services("a:R --- L:b\n"));
}

#[test]
fn q24_a_group_modifier_across_two_groups_is_accepted() {
    accepts(&format!(
        "{HEADER}group g1(cloud)[G1]\ngroup g2(cloud)[G2]\nservice a(server)[A] in g1\nservice b(server)[B] in g2\na{{group}}:R -- L:b{{group}}\n"
    ));
}

#[test]
fn q25_a_group_modifier_inside_one_group_is_rejected() {
    rejects(&format!(
        "{HEADER}group g(cloud)[G]\nservice a(server)[A] in g\nservice b(server)[B] in g\na{{group}}:R -- L:b\n"
    ));
}

#[test]
fn q26_an_edge_to_a_group_is_rejected() {
    // The reference throws a TypeError here: it reads the node of the group.
    rejects(&format!(
        "{HEADER}group g(cloud)[G]\nservice a(server)[A]\na:R -- L:g\n"
    ));
    rejects(&format!(
        "{HEADER}group g(cloud)[G]\nservice a(server)[A]\ng:R -- L:a\n"
    ));
}

#[test]
fn q27_an_edge_to_an_unknown_id_is_rejected() {
    rejects(&with_services("a:R -- L:zzz\n"));
}

#[test]
fn q28_an_invalid_side_is_rejected() {
    rejects(&with_services("a:X\n"));
}

#[test]
fn q29_a_junction_needs_no_in_clause() {
    accepts(&format!("{HEADER}service a(server)[A]\njunction j\n"));
}

#[test]
fn q30_a_junction_has_no_title() {
    rejects(&format!("{HEADER}service a(server)[A]\njunction j[J]\n"));
}

#[test]
fn q31_and_q32_an_alignment_needs_two_distinct_members() {
    rejects(&with_services("align row a\n"));
    rejects(&with_services("align row a a\n"));
}

#[test]
fn q33_an_alignment_member_must_be_a_service_or_junction() {
    rejects(&format!(
        "{HEADER}group g(cloud)[G]\nservice b(server)[B]\nalign row g b\n"
    ));
}

#[test]
fn q34_a_column_alignment_is_accepted() {
    accepts(&with_services("align column a b\n"));
}

#[test]
fn q35_the_header_lines_are_accepted() {
    accepts(&format!("{HEADER}title My Diagram\n"));
    accepts(&format!("{HEADER}accTitle: Acc\naccDescr: Desc\n"));
    accepts(&format!(
        "{HEADER}accDescr {{\n  line one\n\n  line two\n}}\n"
    ));
}

#[test]
fn q36_a_directive_in_the_body_is_accepted() {
    accepts(&format!(
        "{HEADER}%%{{init: {{\"theme\":\"dark\"}}}}%%\nservice a(server)[A]\n"
    ));
}

#[test]
fn q37_tabs_for_indentation_are_accepted() {
    accepts(&format!("{HEADER}\tservice\ta(server)[A]\n"));
}

#[test]
fn q38_the_text_may_end_without_a_line_break() {
    accepts(&format!("{HEADER}service a(server)[A]"));
}

#[test]
fn a_direction_letter_starts_a_token_before_an_id_is_read() {
    // `L` is the side letter and `x` the id after it: the statement needs an id, so it is rejected.
    rejects(&format!("{HEADER}service Lx(server)[A]\n"));
    rejects(&format!("{HEADER}service Bob(server)[B]\n"));
}

#[test]
fn an_id_that_begins_with_title_breaks_before_the_title_is_read() {
    // `title1` lexes as the `title` line and then `1`.
    rejects(&format!("{HEADER}service title1(server)[A]\n"));
    rejects(&format!("{HEADER}service title(server)[T]\n"));
}

#[test]
fn an_id_that_begins_with_accessibility_keyword_is_an_id_unless_a_colon_follows() {
    accepts(&format!("{HEADER}service accTitlex(server)[A]\n"));
}

/// `architecture` is the header keyword only at the start; after the header it is a name, as in
/// upstream Mermaid, whose grammar knows only `architecture-beta`.
#[test]
fn the_header_keyword_is_a_name_after_the_header() {
    accepts(&format!("{HEADER}service architecture(server)[A]\n"));
    accepts(&format!("{HEADER}service architecture\n"));
    accepts("architecture\nservice architecture\n");
    rejects("architecture(x)\n");
    rejects("architecture-betax\n");
    rejects("architecturex\n");
}

#[test]
fn the_header_may_share_its_line_with_a_statement() {
    accepts("architecture service a(server)[A]\n");
}

#[test]
fn a_front_matter_block_in_the_body_is_hidden_by_the_yaml_terminal() {
    accepts(&format!(
        "{HEADER}service a(server)[A]\n---\nservice b(server)[B]\n---\n"
    ));
}

#[test]
fn an_unclosed_front_matter_block_in_the_body_is_a_lexer_error() {
    rejects(&format!("{HEADER}service a(server)[A]\n---\n"));
}

#[test]
fn an_empty_header_title_is_a_title_with_no_text() {
    let diagram = parse(&format!("{HEADER}title x\ntitle\n")).expect("parses");
    // The last title line wins, even when it is empty.
    assert_eq!(diagram.title, "");
}

#[test]
fn a_string_icon_text_may_span_lines() {
    let diagram = parse(&format!("{HEADER}service a \"multi\nline\"[A]\n")).expect("parses");
    assert_eq!(diagram.services[0].icon_text, "multi\nline");
}

// Probes of the reference's DB order (NOTES 2.3, rows O1 to O7): groups are added before any service
// or junction, so a group may name a parent that is declared later in the text, and services and
// junctions may name groups declared later, but a group may not name a group declared later.

#[test]
fn o1_a_service_may_name_a_group_declared_after_it() {
    accepts(&format!(
        "{HEADER}service a(server)[A] in g\ngroup g(cloud)[G]\n"
    ));
}

#[test]
fn o2_a_group_may_not_name_a_group_declared_after_it() {
    rejects(&format!(
        "{HEADER}group g2(cloud)[G2] in g1\ngroup g1(cloud)[G1]\n"
    ));
}

#[test]
fn o3_a_group_may_name_an_earlier_group() {
    accepts(&format!(
        "{HEADER}group g1(cloud)[G1]\ngroup g2(cloud)[G2] in g1\n"
    ));
}

#[test]
fn o4_a_group_may_not_name_a_service() {
    rejects(&format!(
        "{HEADER}service s(server)[S]\ngroup g(cloud)[G] in s\n"
    ));
}

#[test]
fn o5_an_edge_may_name_a_service_declared_later() {
    accepts(&with_services("a:R -- L:later\nservice later(server)[L]\n"));
}

#[test]
fn o6_an_alignment_may_precede_its_members() {
    accepts(&format!(
        "{HEADER}service a(server)[A]\nalign row a b\nservice b(server)[B]\n"
    ));
}

#[test]
fn o7_an_edge_may_name_a_junction_declared_later() {
    accepts(&format!(
        "{HEADER}junction j\na:R -- L:j\nservice a(server)[A]\n"
    ));
}
