//! The cynefin parser and its DB, through `parse`: the integration specs of
//! `packages/mermaid/src/diagrams/cynefin/cynefin.integration.spec.ts`, and one test per probed quirk
//! of `docs-port/cynefin/NOTES.md` (sections 2.5 and 2.6). The expected values are the reference's,
//! as its parser and DB give them (`tools/port/export-cynefin.ts`), and the differential run
//! compares every input of these tests as well.

use ferrigura_model::cynefin::{CynefinDiagram, CynefinDomain};
use ferrigura_syntax::cynefin::parse;

fn parsed(text: &str) -> CynefinDiagram {
    parse(text).expect("the diagram parses")
}

fn domain<'a>(diagram: &'a CynefinDiagram, name: &str) -> Option<&'a CynefinDomain> {
    diagram.domains.iter().find(|d| d.name == name)
}

fn items(diagram: &CynefinDiagram, name: &str) -> Vec<String> {
    domain(diagram, name)
        .map(|d| d.items.clone())
        .unwrap_or_default()
}

fn names(diagram: &CynefinDiagram) -> Vec<&str> {
    diagram.domains.iter().map(|d| d.name.as_str()).collect()
}

// Integration specs.

#[test]
fn parses_the_domains_and_their_items() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"Emergent practice\"\n    \"Probe-sense-respond\"\n  clear\n    \"Best practice\"\n",
    );
    assert_eq!(names(&diagram), ["complex", "clear"]);
    assert_eq!(
        items(&diagram, "complex"),
        ["Emergent practice", "Probe-sense-respond"]
    );
    assert_eq!(items(&diagram, "clear"), ["Best practice"]);
}

#[test]
fn parses_all_five_domains() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"A\"\n  complicated\n    \"B\"\n  clear\n    \"C\"\n  chaotic\n    \"D\"\n  confusion\n    \"E\"\n",
    );
    assert_eq!(
        names(&diagram),
        ["complex", "complicated", "clear", "chaotic", "confusion"]
    );
}

#[test]
fn parses_an_empty_domain() {
    let diagram = parsed("cynefin-beta\n  complex\n");
    assert!(domain(&diagram, "complex").is_some());
    assert!(items(&diagram, "complex").is_empty());
}

#[test]
fn parses_a_domain_with_several_items_in_order() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"First item\"\n    \"Second item\"\n    \"Third item\"\n",
    );
    assert_eq!(
        items(&diagram, "complex"),
        ["First item", "Second item", "Third item"]
    );
}

#[test]
fn skips_comment_lines_between_the_lines_of_the_diagram() {
    let diagram = parsed(
        "cynefin-beta\n  %% This is a comment\n  complex\n    \"Item\"\n  %% Another comment\n",
    );
    assert_eq!(names(&diagram), ["complex"]);
    assert_eq!(items(&diagram, "complex"), ["Item"]);
}

#[test]
fn parses_a_transition_with_its_label() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"Practice\"\n  complicated\n    \"Analysis\"\n  complex --> complicated : \"Pattern found\"\n",
    );
    assert_eq!(diagram.transitions.len(), 1);
    let transition = &diagram.transitions[0];
    assert_eq!(transition.from, "complex");
    assert_eq!(transition.to, "complicated");
    assert_eq!(transition.label.as_deref(), Some("Pattern found"));
}

#[test]
fn parses_a_transition_without_a_label() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"A\"\n  complicated\n    \"B\"\n  complex --> complicated\n",
    );
    assert_eq!(diagram.transitions.len(), 1);
    assert_eq!(diagram.transitions[0].label, None);
}

#[test]
fn keeps_several_transitions_in_declaration_order() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"A\"\n  complicated\n    \"B\"\n  clear\n    \"C\"\n  complex --> complicated : \"Analyzed\"\n  complicated --> clear : \"Codified\"\n  chaotic --> complex : \"Stabilized\"\n",
    );
    let froms: Vec<&str> = diagram
        .transitions
        .iter()
        .map(|t| t.from.as_str())
        .collect();
    assert_eq!(froms, ["complex", "complicated", "chaotic"]);
}

#[test]
fn parses_the_accessible_title() {
    let diagram = parsed(
        "cynefin-beta\n  accTitle: Cynefin Framework Overview\n  complex\n    \"Emergent\"\n",
    );
    assert_eq!(diagram.acc_title, "Cynefin Framework Overview");
}

#[test]
fn parses_the_accessible_description() {
    let diagram = parsed(
        "cynefin-beta\n  accDescr: A diagram showing the five Cynefin domains\n  complex\n    \"Emergent\"\n",
    );
    assert_eq!(
        diagram.acc_descr,
        "A diagram showing the five Cynefin domains"
    );
}

#[test]
fn parses_the_title() {
    let diagram = parsed("cynefin-beta\n  title My Cynefin Map\n  complex\n    \"Emergent\"\n");
    assert_eq!(diagram.title, "My Cynefin Map");
}

#[test]
fn parses_a_full_diagram_with_all_features() {
    let diagram = parsed(
        "cynefin-beta\n  title Team Practices\n  accTitle: Cynefin for team practices\n  complex\n    \"Retrospectives\"\n    \"Pair programming\"\n  complicated\n    \"Code review\"\n    \"Architecture decisions\"\n  clear\n    \"Deployment checklist\"\n  chaotic\n    \"Incident response\"\n  confusion\n    \"New initiative\"\n  complex --> complicated : \"Pattern emerges\"\n  complicated --> clear : \"Best practice found\"\n  chaotic --> complex : \"Stabilized\"\n  confusion --> chaotic : \"Crisis detected\"\n",
    );
    assert_eq!(diagram.title, "Team Practices");
    assert_eq!(diagram.acc_title, "Cynefin for team practices");
    assert_eq!(diagram.domains.len(), 5);
    assert_eq!(items(&diagram, "complex").len(), 2);
    assert_eq!(items(&diagram, "complicated").len(), 2);
    assert_eq!(items(&diagram, "clear").len(), 1);
    assert_eq!(diagram.transitions.len(), 4);
    let last = &diagram.transitions[3];
    assert_eq!(
        (last.from.as_str(), last.to.as_str(), last.label.as_deref()),
        ("confusion", "chaotic", Some("Crisis detected"))
    );
}

#[test]
fn drops_the_self_loops_and_keeps_the_others() {
    let diagram = parsed(
        "cynefin-beta\n  complex\n    \"A\"\n  complicated\n    \"B\"\n  complex --> complex : \"Self-reflection\"\n  complex --> complicated : \"Pattern found\"\n  complicated --> complicated\n",
    );
    assert_eq!(diagram.transitions.len(), 1);
    assert_eq!(diagram.transitions[0].from, "complex");
    assert_eq!(diagram.transitions[0].to, "complicated");
    assert_eq!(
        diagram.transitions[0].label.as_deref(),
        Some("Pattern found")
    );
}

#[test]
fn gives_no_transitions_when_all_are_self_loops() {
    let diagram = parsed("cynefin-beta\n  complex\n    \"A\"\n  complex --> complex\n");
    assert!(diagram.transitions.is_empty());
}

#[test]
fn every_parse_starts_from_an_empty_state() {
    let first = parsed(
        "cynefin-beta\n  title First Diagram\n  complex\n    \"Alpha\"\n    \"Beta\"\n  complicated\n    \"Gamma\"\n  complex --> complicated : \"Move\"\n",
    );
    assert_eq!(first.domains.len(), 2);
    assert_eq!(first.transitions.len(), 1);
    assert_eq!(first.title, "First Diagram");

    let second = parsed("cynefin-beta\n  title Second Diagram\n  chaotic\n    \"Delta\"\n");
    assert_eq!(names(&second), ["chaotic"]);
    assert!(second.transitions.is_empty());
    assert_eq!(second.title, "Second Diagram");
}

// Quirks of the lexer and the grammar (`NOTES.md` section 2.6).

#[test]
fn a_domain_name_is_matched_as_a_prefix_and_the_rest_is_an_error() {
    // `DOMAIN_NAME` has no word end, so `complexity` lexes as `complex` and then fails at `i`.
    assert!(parse("cynefin-beta\ncomplexity \"A\"\n").is_err());
    assert!(parse("cynefin-beta\nclearly \"A\"\n").is_err());
    assert!(parse("cynefin-beta\ncomplex- \"A\"\n").is_err());
}

#[test]
fn domain_names_are_case_sensitive_and_unknown_words_are_errors() {
    assert!(parse("cynefin-beta\nComplex \"A\"\n").is_err());
    assert!(parse("cynefin-beta\nsimple \"A\"\n").is_err());
    assert!(parse("cynefin-beta\ncomplex --> foo\n").is_err());
}

#[test]
fn the_colon_header_takes_no_word_end_but_the_plain_one_does() {
    // `cynefin-beta:` has no guard, so `cynefin-beta:x` is an error at `x`.
    assert!(parse("cynefin-beta:x\ncomplex \"A\"\n").is_err());
    assert!(parse("cynefin-betaX\ncomplex \"A\"\n").is_err());
    // The parser takes `cynefin-beta%%note` (the detector rejects it, as `detect` tests show).
    assert_eq!(
        names(&parsed("cynefin-beta%%note\ncomplex \"A\"\n")),
        ["complex"]
    );
    assert_eq!(
        names(&parsed("cynefin-beta:\ncomplex \"A\"\n")),
        ["complex"]
    );
}

#[test]
fn the_header_can_share_its_line_with_the_first_block() {
    assert_eq!(names(&parsed("cynefin-beta complex \"A\"\n")), ["complex"]);
    assert_eq!(
        names(&parsed("cynefin-beta:\ncomplex \"A\"\n")),
        ["complex"]
    );
}

#[test]
fn a_second_header_is_a_parse_error() {
    assert!(parse("cynefin-beta\ncynefin-beta\ncomplex \"A\"\n").is_err());
}

#[test]
fn an_item_before_any_domain_is_a_parse_error() {
    assert!(parse("cynefin-beta\n\"A\"\n").is_err());
    assert!(parse("\"A\"\ncynefin-beta\n").is_err());
}

#[test]
fn an_item_cannot_follow_a_title_line_or_a_transition() {
    assert!(parse("cynefin-beta\ncomplex\ntitle x\n\"B\"\n").is_err());
    assert!(parse("cynefin-beta\ncomplex --> clear : \"x\"\n\"A\"\n").is_err());
}

#[test]
fn a_transition_takes_a_colon_and_a_quoted_label_only() {
    assert!(parse("cynefin-beta\ncomplex --> clear : x\n").is_err());
    assert!(parse("cynefin-beta\ncomplex --> clear \"x\"\n").is_err());
    assert!(parse("cynefin-beta\ncomplex --> clear : \"x\" \"y\"\n").is_err());
    assert!(parse("cynefin-beta\ncomplex --> clear --> chaotic\n").is_err());
    assert!(parse("cynefin-beta\ncomplex --> clear : \"x\" complicated\n").is_err());
    assert!(parse("cynefin-beta\ncomplex -->\n").is_err());
}

#[test]
fn the_colon_may_have_a_blank_before_it_or_none_and_either_quote() {
    for text in [
        "cynefin-beta\ncomplex --> complicated : 'x'\n",
        "cynefin-beta\ncomplex --> complicated :\"x\"\n",
        "cynefin-beta\ncomplex --> complicated : \"x\"\n",
    ] {
        let diagram = parsed(text);
        assert_eq!(
            diagram.transitions[0].label.as_deref(),
            Some("x"),
            "{text:?}"
        );
    }
}

#[test]
fn an_empty_transition_label_is_no_label() {
    let diagram = parsed("cynefin-beta\ncomplex --> complicated : \"\"\n");
    assert_eq!(diagram.transitions[0].label, None);
}

#[test]
fn a_transition_to_an_undeclared_domain_is_kept() {
    let diagram = parsed("cynefin-beta\ncomplex --> clear\n");
    assert!(diagram.domains.is_empty());
    assert_eq!(diagram.transitions[0].to, "clear");
}

#[test]
fn a_block_and_a_transition_can_share_a_line() {
    // Only the block is a domain: the transition's `complicated` has no block of its own.
    let diagram = parsed("cynefin-beta\ncomplex \"A\" complicated --> clear\n");
    assert_eq!(names(&diagram), ["complex"]);
    assert_eq!(diagram.transitions[0].from, "complicated");
}

#[test]
fn several_domain_blocks_share_a_line_and_several_items_too() {
    let diagram = parsed("cynefin-beta\ncomplex \"A\" \"B\"\n");
    assert_eq!(items(&diagram, "complex"), ["A", "B"]);
    let diagram = parsed("cynefin-beta\ncomplex complicated\n");
    assert_eq!(names(&diagram), ["complex", "complicated"]);
}

#[test]
fn a_title_can_start_after_a_domain_on_the_same_line() {
    let diagram = parsed("cynefin-beta\ncomplex title x\n");
    assert_eq!(names(&diagram), ["complex"]);
    assert_eq!(diagram.title, "x");
}

#[test]
fn blank_lines_and_comments_may_come_between_a_domain_and_its_items() {
    let diagram = parsed("cynefin-beta\ncomplex\n  \n  %% note\n  \"A\"\n");
    assert_eq!(items(&diagram, "complex"), ["A"]);
}

#[test]
fn a_domain_and_its_items_may_follow_a_transition_on_the_next_line() {
    let diagram = parsed("cynefin-beta\ncomplex \"A\"\nclear --> chaotic\n");
    assert_eq!(names(&diagram), ["complex"]);
    assert_eq!(diagram.transitions[0].to, "chaotic");
}

#[test]
fn an_escape_in_a_string_gives_its_character() {
    let diagram = parsed("cynefin-beta\ncomplex \"a\\\"b\\\\c\\td\"\n");
    assert_eq!(items(&diagram, "complex"), ["a\"b\\c\td"]);
}

#[test]
fn a_newline_escape_and_a_raw_line_break_are_both_line_breaks() {
    let diagram = parsed("cynefin-beta\ncomplex \"a\\nb\"\n");
    assert_eq!(items(&diagram, "complex"), ["a\nb"]);
    let diagram = parsed("cynefin-beta\ncomplex \"a\nb\"\n");
    assert_eq!(items(&diagram, "complex"), ["a\nb"]);
}

#[test]
fn quotes_inside_the_other_quote_and_unknown_escapes() {
    let diagram = parsed("cynefin-beta\ncomplex 'it\\'s' 'say \"hi\"'\n");
    assert_eq!(items(&diagram, "complex"), ["it's", "say \"hi\""]);
    // An unknown escape gives its letter, so `A` is `u0041`.
    let diagram = parsed("cynefin-beta\ncomplex \"a\\qb\\u0041c\\x41d\"\n");
    assert_eq!(items(&diagram, "complex"), ["aqbu0041cx41d"]);
}

#[test]
fn an_unterminated_string_is_a_lexical_error() {
    assert!(parse("cynefin-beta\ncomplex \"abc\n").is_err());
}

#[test]
fn text_outside_the_grammar_is_kept_in_a_string() {
    let diagram = parsed("cynefin-beta\ncomplex \"\u{dc} \u{3b6} \u{2192} x\"\n");
    assert_eq!(items(&diagram, "complex"), ["\u{dc} \u{3b6} \u{2192} x"]);
}

#[test]
fn an_empty_item_is_an_empty_string() {
    let diagram = parsed("cynefin-beta\ncomplex \"\"\n");
    assert_eq!(items(&diagram, "complex"), [""]);
}

#[test]
fn crlf_line_ends_tabs_and_leading_blanks_before_the_header_are_accepted() {
    let diagram = parsed("\n\n  cynefin-beta\r\n\tcomplex\t\"A\"\t\r\n");
    assert_eq!(items(&diagram, "complex"), ["A"]);
}

#[test]
fn a_header_alone_gives_an_empty_diagram() {
    for text in ["cynefin-beta", "cynefin-beta\n", "cynefin-beta:"] {
        let diagram = parsed(text);
        assert!(diagram.domains.is_empty(), "{text:?}");
        assert!(diagram.transitions.is_empty(), "{text:?}");
    }
}

#[test]
fn a_title_comment_is_cut_at_the_comment_marker() {
    let diagram = parsed("cynefin-beta\ntitle a %% comment\n");
    assert_eq!(diagram.title, "a");
    let diagram = parsed("cynefin-beta\ncomplex \"A\" %% note\n");
    assert_eq!(names(&diagram), ["complex"]);
}

#[test]
fn a_title_line_takes_the_rest_of_its_line() {
    let diagram = parsed("cynefin-beta\ntitle x complex \"A\"\n");
    assert_eq!(diagram.title, "x complex \"A\"");
    assert!(diagram.domains.is_empty());
}

#[test]
fn the_title_keyword_needs_a_blank_or_the_end_after_it() {
    assert!(parse("cynefin-beta\ntitleX\n").is_err());
}

#[test]
fn the_last_title_line_wins_and_an_empty_one_is_empty() {
    assert_eq!(
        parsed("cynefin-beta\ntitle First\ntitle Second\n").title,
        "Second"
    );
    assert_eq!(parsed("cynefin-beta\ntitle First\ntitle\n").title, "");
}

#[test]
fn a_title_is_trimmed_and_its_blank_runs_collapse() {
    assert_eq!(
        parsed("cynefin-beta\ntitle   Spaces   here   \n").title,
        "Spaces here"
    );
    assert_eq!(parsed("cynefin-beta\ntitle\n").title, "");
}

#[test]
fn an_accessible_title_needs_its_colon() {
    assert_eq!(parsed("cynefin-beta\naccTitle:\n").acc_title, "");
    assert_eq!(
        parsed("cynefin-beta\naccTitle:   A   &  B  \n").acc_title,
        "A & B"
    );
    assert!(parse("cynefin-beta\naccTitle x\n").is_err());
}

#[test]
fn an_accessible_description_in_one_line_or_in_a_block() {
    assert_eq!(parsed("cynefin-beta\naccDescr: A map\n").acc_descr, "A map");
    let diagram = parsed(
        "cynefin-beta\naccDescr {\n  line one\n    line two\n\n  line three\n}\ncomplex \"A\"\n",
    );
    assert_eq!(diagram.acc_descr, "line one\nline two\nline three");
    assert!(parse("cynefin-beta\naccDescr {\n  never closed\n").is_err());
}

// The entities and the sanitiser of the DB (`NOTES.md` sections 3.4 and 6.12).

#[test]
fn an_entity_placeholder_is_kept_in_an_item_and_in_a_title() {
    // The labels are not decoded, so the placeholder stays; the title is kept as the DB sanitises it.
    let diagram = parsed("cynefin-beta\ncomplex \"a #amp; b\"\ntitle A #amp; B\n");
    assert_eq!(
        items(&diagram, "complex"),
        ["a \u{fb02}\u{b0}amp\u{b6}\u{df} b"]
    );
    assert_eq!(diagram.title, "A \u{fb02}\u{b0}amp\u{b6}\u{df} B");
}

#[test]
fn a_title_with_no_markup_is_unchanged_and_one_with_a_less_than_is_escaped() {
    assert_eq!(parsed("cynefin-beta\ntitle A & B\n").title, "A & B");
    assert_eq!(parsed("cynefin-beta\ntitle a < b\n").title, "a &lt; b");
}

#[test]
fn a_title_with_markup_is_serialised_and_a_script_is_removed() {
    assert_eq!(
        parsed("cynefin-beta\ntitle <b>x</b> & y\n").title,
        "<b>x</b> &amp; y"
    );
    assert_eq!(
        parsed("cynefin-beta\ntitle A<script>alert(1)</script>B\n").title,
        "AB"
    );
    assert_eq!(
        parsed("cynefin-beta\ntitle <img src=x onerror=alert(1)> T\n").title,
        "<img src=\"x\"> T"
    );
}

#[test]
fn an_item_and_a_transition_label_are_not_sanitised() {
    let diagram = parsed("cynefin-beta\ncomplex \"<b>x</b> & y\"\n");
    assert_eq!(items(&diagram, "complex"), ["<b>x</b> & y"]);
}

#[test]
fn the_accessible_texts_go_through_the_same_sanitiser() {
    assert_eq!(parsed("cynefin-beta\naccTitle: A & B\n").acc_title, "A & B");
    assert_eq!(
        parsed("cynefin-beta\naccDescr: a <i>b</i>\n").acc_descr,
        "a <i>b</i>"
    );
}

// The DB calls (`NOTES.md` section 3.2 and 3.4).

#[test]
fn a_second_block_of_the_same_domain_replaces_its_items_and_keeps_its_place() {
    let diagram = parsed("cynefin-beta\ncomplex \"A\"\nclear \"C\"\ncomplex \"B\"\n");
    assert_eq!(names(&diagram), ["complex", "clear"]);
    assert_eq!(items(&diagram, "complex"), ["B"]);
    let diagram = parsed("cynefin-beta\ncomplex \"A\"\ncomplex\n");
    assert!(items(&diagram, "complex").is_empty());
}

#[test]
fn a_front_matter_title_stands_unless_the_body_gives_a_non_empty_title() {
    let front = "---\ntitle: FM\n---\n";
    assert_eq!(
        parsed(&format!("{front}cynefin-beta\ncomplex \"A\"\n")).title,
        "FM"
    );
    assert_eq!(
        parsed(&format!("{front}cynefin-beta\ntitle Body\n")).title,
        "Body"
    );
    assert_eq!(parsed(&format!("{front}cynefin-beta\ntitle\n")).title, "FM");
}

#[test]
fn a_front_matter_title_is_sanitised_as_the_body_title_is() {
    let front = "---\ntitle: a < b\n---\n";
    assert_eq!(parsed(&format!("{front}cynefin-beta\n")).title, "a &lt; b");
}

#[test]
fn the_header_lines_keep_the_last_value_given() {
    let diagram =
        parsed("cynefin-beta\naccDescr: first\naccDescr: second\naccTitle: one\naccTitle: two\n");
    assert_eq!(diagram.acc_descr, "second");
    assert_eq!(diagram.acc_title, "two");
}
