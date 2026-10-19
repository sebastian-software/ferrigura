//! The lexer and the grammar of `wardley-beta`, one test per quirk L1 to L23 of
//! `docs-port/wardley/NOTES.md`, section 2.5. Each input is the probe the notes ran through the
//! reference's own parser: a rejection on both sides is agreement, so only acceptance is checked
//! here, and the reference's message is not.

use ferrigura_model::wardley::{Flow, NodeClass, Strategy, WardleyDiagram};
use ferrigura_syntax::wardley::parse;

fn accepts(text: &str) -> bool {
    parse(text).is_ok()
}

fn parsed(text: &str) -> WardleyDiagram {
    parse(text).expect("the map parses")
}

#[test]
fn l1_a_name_that_begins_with_the_strategy_build_buy_outsource_market_is_split() {
    assert!(!accepts("wardley-beta\ncomponent marketing [0.5, 0.5]\n"));
    assert!(!accepts("wardley-beta\ncomponent buyer [0.5, 0.5]\n"));
    assert!(!accepts("wardley-beta\ncomponent builder [0.5, 0.5]\n"));
}

#[test]
fn l2_a_name_that_begins_with_a_keyword_is_split() {
    assert!(!accepts("wardley-beta\ncomponent notes [0.5, 0.5]\n"));
    assert!(!accepts(
        "wardley-beta\ncomponent evolutionary [0.5, 0.5]\n"
    ));
    assert!(!accepts("wardley-beta\ncomponent componentA [0.5, 0.5]\n"));
    // `title` alone is the title keyword, not a name.
    assert!(!accepts("wardley-beta\ncomponent title [0.5, 0.5]\n"));
}

#[test]
fn l3_a_name_that_begins_with_a_digit_is_rejected() {
    assert!(!accepts("wardley-beta\ncomponent 2fast [0.5, 0.5]\n"));
}

#[test]
fn l4_the_header_keyword_is_case_sensitive_so_wardley_beta_capitalised_is_rejected() {
    assert!(!accepts("Wardley-Beta\ncomponent A [0.5, 0.5]\n"));
}

#[test]
fn l5_the_header_has_no_word_end() {
    assert!(!accepts("wardley-betax\ncomponent A [0.5, 0.5]\n"));
}

#[test]
fn l6_a_name_that_begins_with_title_without_a_blank_is_a_name() {
    let diagram = parsed("wardley-beta\ncomponent titlebar [0.2, 0.3]\n");
    assert_eq!(diagram.nodes[0].label, "titlebar");
}

#[test]
fn l7_keywords_are_case_sensitive() {
    let diagram = parsed("wardley-beta\ncomponent Size [0.2, 0.3]\n");
    assert_eq!(diagram.nodes[0].label, "Size");
}

#[test]
fn l8_a_trailing_hyphen_is_part_of_a_name() {
    let diagram = parsed("wardley-beta\ncomponent foo- [0.2, 0.3]\n");
    assert_eq!(diagram.nodes[0].label, "foo-");
}

#[test]
fn l9_there_is_no_dashed_arrow_of_the_form_dot_dash_dot_or_a_double_dash() {
    assert!(!accepts(
        "wardley-beta\ncomponent A [0.1, 0.1]\ncomponent B [0.2, 0.2]\nA .-. B\n"
    ));
    assert!(!accepts(
        "wardley-beta\ncomponent A [0.1, 0.1]\ncomponent B [0.2, 0.2]\nA -- B\n"
    ));
}

#[test]
fn l10_a_link_label_needs_one_character_after_the_semicolon() {
    assert!(!accepts(
        "wardley-beta\ncomponent A [0.1, 0.1]\ncomponent B [0.2, 0.2]\nA +> B ;\n"
    ));
}

#[test]
fn l11_component_coordinates_are_decimals() {
    assert!(!accepts("wardley-beta\ncomponent A [0, 0.5]\n"));
    assert!(!accepts("wardley-beta\ncomponent A [1, 0.5]\n"));
}

#[test]
fn l12_the_size_is_two_integers() {
    assert!(!accepts(
        "wardley-beta\nsize [1200.5, 900]\ncomponent A [0.5, 0.5]\n"
    ));
}

#[test]
fn l13_evolution_boundaries_are_decimals() {
    assert!(!accepts("wardley-beta\nevolution A@1 -> B\n"));
    // `@1.0` is a decimal, and is 1.
    let diagram = parsed("wardley-beta\nevolution A@1.0 -> B\n");
    assert_eq!(diagram.axes.stage_boundaries.expect("boundaries")[0].0, 1.0);
}

#[test]
fn l14_an_evolution_has_at_least_two_stages() {
    assert!(accepts("wardley-beta\nevolution A -> B\n"));
    assert!(!accepts("wardley-beta\nevolution A\n"));
}

#[test]
fn l15_the_inertia_mark_comes_after_the_label_and_the_decorator() {
    assert!(!accepts(
        "wardley-beta\ncomponent A [0.5,0.5] (inertia) (build)\n"
    ));
    assert!(accepts(
        "wardley-beta\ncomponent A [0.5, 0.5] (build) (inertia)\n"
    ));
    assert!(!accepts(
        "wardley-beta\ncomponent A [0.5, 0.5] (build) label [1, 2]\n"
    ));
}

#[test]
fn l16_annotation_coordinates_take_integers_and_decimals() {
    assert!(accepts("wardley-beta\nannotations [1, 0]\n"));
    assert!(accepts("wardley-beta\nannotation 1,[1, 0] \"x\"\n"));
}

#[test]
fn l17_the_last_title_wins() {
    let diagram = parsed("wardley-beta\ntitle One\ntitle Two\n");
    assert_eq!(diagram.title, "Two");
}

#[test]
fn l18_a_blank_title_leaves_the_title_empty_as_the_last_line_gives_it() {
    let diagram = parsed("wardley-beta\ntitle \n");
    assert_eq!(diagram.title, "");
    let diagram = parsed("wardley-beta\ntitle One\ntitle \n");
    assert_eq!(diagram.title, "");
}

#[test]
fn l19_strings_resolve_their_escapes_and_accept_either_quote() {
    let diagram =
        parsed("wardley-beta\ncomponent \"A\\\"B\" [0.1, 0.2]\ncomponent 'Single' [0.1, 0.2]\n");
    assert_eq!(diagram.nodes[0].label, "A\"B");
    assert_eq!(diagram.nodes[1].label, "Single");
}

#[test]
fn l20_a_title_ends_at_a_comment_marker() {
    let diagram = parsed("wardley-beta\ntitle 50%% done\n");
    assert_eq!(diagram.title, "50");
}

#[test]
fn l21_a_statement_may_follow_the_header_on_the_same_line() {
    let diagram = parsed("wardley-beta component A [0.2, 0.4]\n");
    assert_eq!(diagram.nodes[0].label, "A");
    assert!(accepts("wardley-beta title Inline\n"));
}

#[test]
fn l22_crlf_tabs_and_no_final_newline_are_accepted() {
    assert!(accepts("wardley-beta\r\ncomponent\tA [0.1, 0.2]\r\nA -> A"));
}

#[test]
fn l23_the_accessibility_texts_are_read_by_the_common_converter() {
    let diagram = parsed("wardley-beta\naccTitle: My title\ncomponent A [0.1, 0.2]\n");
    assert_eq!(diagram.acc_title, "My title");
    let diagram =
        parsed("wardley-beta\naccDescr {\n  line one\n  line two\n}\ncomponent A [0.1, 0.2]\n");
    assert_eq!(diagram.acc_descr, "line one\nline two");
}

#[test]
fn the_strategies_and_flows_are_the_reference_words() {
    let diagram = parsed(
        "wardley-beta\ncomponent A [0.1, 0.2] (market)\ncomponent B [0.1, 0.2] (outsource)\nA +'x'<> B\nA +< B\nA +> B\n",
    );
    assert_eq!(diagram.nodes[0].source_strategy, Some(Strategy::Market));
    assert_eq!(diagram.nodes[1].source_strategy, Some(Strategy::Outsource));
    assert_eq!(diagram.nodes[0].class_name, Some(NodeClass::Component));
    assert_eq!(diagram.links[0].flow, Some(Flow::Bidirectional));
    assert_eq!(diagram.links[1].flow, Some(Flow::Backward));
    assert_eq!(diagram.links[2].flow, Some(Flow::Forward));
}

#[test]
fn a_double_dash_arrow_and_the_dashed_arrow_are_links() {
    let diagram = parsed(
        "wardley-beta\ncomponent A [0.1, 0.2]\ncomponent B [0.1, 0.2]\nA -.-> B\nA --> B\nA > B\n",
    );
    assert!(diagram.links[0].dashed);
    assert!(!diagram.links[1].dashed);
    assert!(!diagram.links[2].dashed);
}

#[test]
fn a_pipeline_component_takes_no_decorator_and_no_inertia() {
    assert!(!accepts(
        "wardley-beta\ncomponent K [0.5, 0.5]\npipeline K {\n  component X [0.3] (build)\n}\n"
    ));
    assert!(!accepts(
        "wardley-beta\ncomponent K [0.5, 0.5]\npipeline K {\n  component X [0.3] inertia\n}\n"
    ));
}

#[test]
fn a_pipeline_needs_a_line_break_and_a_component() {
    assert!(!accepts(
        "wardley-beta\ncomponent K [0.5, 0.5]\npipeline K { component X [0.3]\n}\n"
    ));
    assert!(!accepts(
        "wardley-beta\ncomponent K [0.5, 0.5]\npipeline K {\n}\n"
    ));
    assert!(accepts(
        "wardley-beta\ncomponent K [0.5, 0.5]\npipeline K {\n\n  component X [0.3]\n\n}\n"
    ));
}

#[test]
fn two_statements_on_one_line_are_rejected() {
    assert!(!accepts(
        "wardley-beta\ncomponent A [0.5, 0.5] component B [0.5, 0.5]\n"
    ));
}

#[test]
fn a_comment_marker_in_the_middle_of_a_line_is_a_comment() {
    assert!(accepts("wardley-beta\ncomponent A [0.5, 0.5] %% comment\n"));
}
