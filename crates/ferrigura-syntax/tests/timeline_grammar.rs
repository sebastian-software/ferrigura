//! The quirks of the timeline that the port keeps, one test per row of the table in
//! `docs-port/timeline/PLAN.md`, section 6 (Q1 to Q21). Each expected value is the reference's:
//! the notes record the Jison and Chromium runs that verified it. The differential run checks the
//! same inputs against the reference.

use ferrigura_model::timeline::{TimelineDiagram, TimelinePeriod};
use ferrigura_syntax::timeline::parse;
use ferrigura_syntax::{DiagramType, detect};

fn parsed(text: &str) -> TimelineDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

fn rejected(text: &str) {
    assert!(parse(text).is_err(), "{text:?} should be rejected");
}

fn period(section: &str, name: &str, events: &[&str]) -> TimelinePeriod {
    TimelinePeriod {
        section: section.to_owned(),
        period: name.to_owned(),
        events: events.iter().map(|event| (*event).to_owned()).collect(),
    }
}

#[test]
fn should_not_detect_a_capitalised_timeline() {
    assert_eq!(detect("Timeline\nsection A"), None);
    assert_eq!(detect("timeline\nsection A"), Some(DiagramType::Timeline));
    assert_eq!(detect("timelineX"), Some(DiagramType::Timeline));
}

#[test]
fn should_ignore_the_front_matter_title() {
    let diagram = parsed("---\ntitle: From front matter\n---\ntimeline\nsection A");
    assert_eq!(diagram.title, "");
}

#[test]
fn should_let_the_last_title_win_even_when_it_is_blank() {
    let diagram = parsed("timeline\ntitle From grammar\ntitle  ");
    assert_eq!(diagram.title, " ");
}

#[test]
fn should_keep_blanks_in_titles_sections_periods_and_events() {
    let diagram = parsed("timeline\ntitle  x  \nsection A \n  p  : e  ");
    assert_eq!(diagram.title, " x  ");
    assert_eq!(diagram.sections, ["A "]);
    assert_eq!(diagram.periods, [period("A ", "p  ", &["e  "])]);
}

#[test]
fn should_read_title_with_one_trailing_blank_as_a_period() {
    let diagram = parsed("timeline\ntitle \n");
    assert_eq!(diagram.title, "");
    assert_eq!(diagram.periods, [period("", "title ", &[])]);
}

#[test]
fn should_match_the_timeline_keyword_only_at_a_word_boundary() {
    rejected("timelineX");
    let diagram = parsed("timeline TDx");
    assert_eq!(diagram.direction, "LR");
    assert_eq!(diagram.periods, [period("", "TDx", &[])]);
}

#[test]
fn should_need_a_blank_after_the_colon_of_an_event() {
    let diagram = parsed("timeline\n2004 : a:b");
    assert_eq!(diagram.periods, [period("", "2004 ", &["a:b"])]);
    rejected("timeline\n2004:x");
}

#[test]
fn should_attach_an_event_to_the_last_period_created() {
    let diagram = parsed("timeline\nsection A\n  t1\n  section B\n  : e");
    assert_eq!(diagram.periods, [period("A", "t1", &["e"])]);
    assert_eq!(diagram.sections, ["A", "B"]);
}

#[test]
fn should_take_the_next_line_as_the_value_of_acc_title() {
    assert_eq!(parsed("timeline\naccTitle:\nfoo").acc_title, "foo");
    let diagram = parsed("timeline\naccTitle:\n  2004");
    assert_eq!(diagram.acc_title, "2004");
    assert!(diagram.periods.is_empty());
}

#[test]
fn should_reject_an_acc_title_or_acc_descr_that_ends_the_input() {
    rejected("timeline\naccTitle:");
    rejected("timeline\naccDescr: ");
}

#[test]
fn should_start_a_comment_with_hash_only_at_the_start_of_a_token() {
    assert_eq!(parsed("timeline\ntitle #x#").title, "#x#");
    assert_eq!(parsed("timeline\nsection A #x").sections, ["A #x"]);
    assert_eq!(
        parsed("timeline\n2004 # x").periods,
        [period("", "2004 ", &[])]
    );
    assert_eq!(
        parsed("timeline\n2004 : e # x").periods,
        [period("", "2004 ", &["e # x"])]
    );
}

#[test]
fn should_drop_a_character_before_a_percent_comment() {
    assert!(parsed("timeline\nx%%c").periods.is_empty());
    assert_eq!(
        parsed("timeline\nabc%%c").periods,
        [period("", "abc%%c", &[])]
    );
    assert!(parsed("timeline\n%abc").periods.is_empty());
}

#[test]
fn should_keep_comment_marks_inside_an_accessibility_block_as_text() {
    let diagram = parsed("timeline\naccDescr {\n  a # b\n  c %% d\n}");
    assert_eq!(diagram.acc_descr, "a # b\nc %% d");
}

#[test]
fn should_reject_an_unclosed_acc_descr_block_and_accept_an_empty_one() {
    rejected("timeline\naccDescr {\n a");
    let diagram = parsed("timeline\naccDescr {}");
    assert_eq!(diagram.acc_descr, "");
}

#[test]
fn should_remove_the_indentation_of_every_line_of_an_acc_descr_block() {
    let diagram = parsed("timeline\naccDescr {\n    one\n        two\n}");
    assert_eq!(diagram.acc_descr, "one\ntwo");
}

#[test]
fn should_keep_a_repeated_section_name_with_the_periods_of_each_occurrence() {
    let diagram = parsed("timeline\nsection A\n  t1\nsection A\n  t2");
    assert_eq!(diagram.sections, ["A", "A"]);
    assert_eq!(
        diagram.periods,
        [period("A", "t1", &[]), period("A", "t2", &[])]
    );
}

#[test]
fn should_ignore_blank_lines_and_blanks_between_statements() {
    let diagram = parsed("timeline\n\n   section A  \n\n\n  t1 \n\n  : e\n\n");
    assert_eq!(diagram.sections, ["A  "]);
    assert_eq!(diagram.periods, [period("A  ", "t1 ", &["e"])]);
}

#[test]
fn should_reject_an_event_with_no_period_before_it() {
    rejected("timeline\n: e");
}

#[test]
fn should_need_no_line_break_between_statements() {
    let diagram = parsed("timeline section A");
    assert_eq!(diagram.sections, ["A"]);
}

#[test]
fn should_reject_a_second_header() {
    rejected("timeline\ntimeline TD");
    rejected("timeline\ntimeline LR");
}

#[test]
fn should_encode_entities_before_the_parser_sees_them() {
    let diagram = parsed("timeline\nsection #quot;A#quot;\n  t: #9829;");
    assert_eq!(diagram.sections, ["ﬂ°quot¶ßAﬂ°quot¶ß"]);
    assert_eq!(diagram.periods[0].events, ["ﬂ°°9829¶ß"]);
}

#[test]
fn should_sanitise_the_title_only_when_it_has_markup() {
    assert_eq!(parsed("timeline\ntitle AT&T").title, "AT&T");
    assert_eq!(
        parsed("timeline\ntitle AT&T <b>bold</b>").title,
        "AT&amp;T <b>bold</b>"
    );
}

#[test]
fn should_not_collapse_blanks_in_header_texts() {
    let diagram = parsed("timeline\naccTitle:   a   b  ");
    assert_eq!(diagram.acc_title, "a   b");
}
