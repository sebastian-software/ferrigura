//! The grammar of `radar.langium` and its lexer, as `docs-port/radar/NOTES.md` section 2.5 probes
//! them through the reference's parser. Each text is one probe: a text that the reference accepts
//! is accepted, and one it rejects is rejected, with the position and message ferrigura's own.

use ferrigura_syntax::radar::parse;

fn accepted(text: &str) {
    if let Err(error) = parse(text) {
        panic!("{text:?} should be accepted: {error}");
    }
}

fn rejected(text: &str) {
    assert!(parse(text).is_err(), "{text:?} should be rejected");
}

mod header {
    use super::*;

    #[test]
    fn the_header_takes_its_colon_or_none() {
        for text in [
            "radar-beta",
            "radar-beta:",
            "radar-beta :",
            "radar-beta\n",
            "radar-beta\naxis A\ncurve c{1}",
        ] {
            accepted(text);
        }
    }

    #[test]
    fn a_comment_or_a_statement_may_follow_the_header_on_its_line() {
        accepted("radar-beta %% note\naxis A\ncurve c{1}");
        accepted("radar-beta%% note\naxis A\ncurve c{1}");
        accepted("radar-beta axis A curve c{1}");
    }

    #[test]
    fn leading_blanks_and_comments_before_the_header_are_skipped() {
        accepted("  radar-beta\naxis A\ncurve c{1}");
        accepted("%% c\nradar-beta\naxis A\ncurve c{1}");
        accepted("\n\nradar-beta\n\n\naxis A\n\n");
    }

    #[test]
    fn crlf_and_a_lone_cr_are_line_breaks() {
        accepted("radar-beta\r\naxis A\r\ncurve c{1}\r\n");
        accepted("radar-beta\raxis A");
    }

    #[test]
    fn a_word_that_starts_with_the_header_is_not_the_header() {
        rejected("radar-beta{\naxis A");
        rejected("radar-betaX\naxis A");
        rejected("pie\naxis A");
        rejected("");
    }
}

mod statements {
    use super::*;

    #[test]
    fn axes_are_comma_separated_with_or_without_blanks() {
        accepted("radar-beta\naxis A, B, C");
        accepted("radar-beta\naxis A,B,C");
        accepted("radar-beta\n\taxis\tA,\tB");
    }

    #[test]
    fn a_duplicate_axis_and_a_duplicate_option_are_accepted() {
        accepted("radar-beta\naxis A\naxis A");
        accepted("radar-beta\nticks 3\nticks 4");
    }

    #[test]
    fn statements_need_no_separator() {
        accepted("radar-beta\naxis A,B curve c1{1,2} curve c2{3,4}");
        accepted("radar-beta\nticks 3 max 5");
        accepted("radar-beta\nshowLegend true, ticks 3");
    }

    #[test]
    fn an_axis_list_needs_a_comma_and_an_id_after_it() {
        rejected("radar-beta\naxis A B");
        rejected("radar-beta\naxis A,");
        rejected("radar-beta\naxis A, B,\n");
        rejected("radar-beta\naxis");
        rejected("radar-beta\naxis A \"label\"");
        rejected("radar-beta\naxis A\n[\"x\"]");
        rejected("radar-beta\naxis A,\nB");
    }

    #[test]
    fn a_curve_needs_its_brace_on_the_same_line() {
        rejected("radar-beta\ncurve c\n{1,2}");
    }

    #[test]
    fn an_option_needs_its_value_on_the_same_line() {
        rejected("radar-beta\nshowLegend\ntrue");
        rejected("radar-beta\nticks 5,\nmax 3");
    }

    #[test]
    fn a_label_is_a_quoted_string_in_brackets_on_the_same_line() {
        accepted("radar-beta\naxis A[\"Label A\"]");
        accepted("radar-beta\naxis A [\"Label A\"]");
        accepted("radar-beta\naxis A['x y']");
        accepted("radar-beta\naxis A[\"a\\\"b\"]");
        accepted("radar-beta\naxis A[\"a\\tb\"]");
        accepted("radar-beta\ncurve c [\"Label\"] {1,2}");
    }
}

mod names {
    use super::*;

    #[test]
    fn a_name_that_only_begins_with_a_keyword_is_an_id() {
        accepted("radar-beta\naxis accTitle");
        accepted("radar-beta\naxis accDescr");
        accepted("radar-beta\naxis showLegend2");
        accepted("radar-beta\naxis curvex, ticksy");
        accepted("radar-beta\ncurve radar-beta{1}");
    }

    #[test]
    fn an_id_has_word_characters_and_inner_dashes() {
        accepted("radar-beta\naxis a-1");
        accepted("radar-beta\naxis _x_1");
    }

    #[test]
    fn the_header_word_is_a_name_only_where_no_blank_follows_it() {
        // `axis radar-beta` is the header keyword, and `curve radar-beta{1}` is an id.
        rejected("radar-beta\naxis radar-beta");
        accepted("radar-beta\naxis A\ncurve radar-beta{1}");
        accepted("radar-beta\naxis A\ncurve curvex{1}");
        accepted("radar-beta\naxis A title");
    }

    #[test]
    fn a_keyword_or_a_terminal_alone_is_not_a_name() {
        rejected("radar-beta\naxis max");
        rejected("radar-beta\naxis min");
        rejected("radar-beta\naxis circle");
        rejected("radar-beta\naxis true");
        rejected("radar-beta\naxis axis");
        rejected("radar-beta\naxis curve");
        rejected("radar-beta\ncurve ticks{1}");
        rejected("radar-beta\ncurve axis{1}");
    }

    #[test]
    fn a_terminal_takes_a_prefix_of_a_word() {
        rejected("radar-beta\naxis trueX");
        rejected("radar-beta\naxis falseAlarm");
        rejected("radar-beta\naxis title1");
        rejected("radar-beta\naxis 1A");
        rejected("radar-beta\naxis 12");
    }

    #[test]
    fn an_id_cannot_end_in_a_dash_or_hold_a_dot_or_a_non_ascii_letter() {
        rejected("radar-beta\naxis A-");
        rejected("radar-beta\naxis a.b");
        rejected("radar-beta\naxis Äx");
    }
}

mod entries {
    use super::*;

    #[test]
    fn numbers_may_be_split_over_lines_after_a_comma() {
        accepted("radar-beta\ncurve c{1,2,3}");
        accepted("radar-beta\ncurve c { 1, 2 }");
        accepted("radar-beta\ncurve c{1 ,2}");
        accepted("radar-beta\ncurve c{1,\n2}");
        accepted("radar-beta\ncurve c{\n1,\n2\n}");
    }

    #[test]
    fn a_number_is_an_unsigned_decimal_with_an_optional_fraction() {
        accepted("radar-beta\ncurve c{123456789012345678901234567890}");
        accepted("radar-beta\ncurve c{0.0001}");
        rejected("radar-beta\ncurve c{-1}");
        rejected("radar-beta\ncurve c{.5}");
        rejected("radar-beta\ncurve c{1.}");
        rejected("radar-beta\ncurve c{1.5.2}");
        rejected("radar-beta\ncurve c{01}");
        rejected("radar-beta\ncurve c{1e3}");
    }

    #[test]
    fn an_entry_list_is_not_empty_and_has_no_trailing_comma() {
        rejected("radar-beta\ncurve c{}");
        rejected("radar-beta\ncurve c{1,}");
        rejected("radar-beta\ncurve c{1 2}");
        rejected("radar-beta\ncurve c{1\n,2}");
    }

    #[test]
    fn detailed_and_number_entries_are_not_mixed() {
        rejected("radar-beta\naxis A, B\ncurve c{A 1, 2}");
        rejected("radar-beta\naxis A, B\ncurve c{1, B 2}");
    }

    #[test]
    fn a_detailed_entry_takes_an_optional_colon() {
        accepted("radar-beta\naxis A, B, C\ncurve c{ C 3, A 1, B 2 }");
        accepted("radar-beta\naxis A\ncurve c{A: 1}");
    }
}

mod options {
    use super::*;

    #[test]
    fn the_options_take_their_values() {
        accepted("radar-beta\nshowLegend true");
        accepted("radar-beta\nshowLegend false");
        accepted("radar-beta\nticks 5");
        accepted("radar-beta\nticks 2.5");
        accepted("radar-beta\nticks 0");
        accepted("radar-beta\ngraticule polygon");
        accepted("radar-beta\ngraticule circle");
    }

    #[test]
    fn a_wrong_value_is_rejected() {
        rejected("radar-beta\nshowLegend 1");
        rejected("radar-beta\nshowLegend True");
        rejected("radar-beta\nmin -1");
        rejected("radar-beta\ngraticule square");
        rejected("radar-beta\ngraticule polygonal");
    }
}

mod comments_and_blocks {
    use super::*;

    #[test]
    fn a_directive_or_a_comment_in_the_body_is_hidden() {
        accepted("radar-beta\n%%{init: {\"theme\":\"base\"}}%%\naxis A\ncurve c{1}");
        accepted("radar-beta\naxis A\n%% c\ncurve c{1}");
        accepted("radar-beta\ntitle T %% c\naxis A");
    }

    #[test]
    fn a_yaml_like_block_in_the_body_is_hidden() {
        accepted("radar-beta\n---\nx\n---\naxis A");
    }

    #[test]
    fn an_unclosed_dash_block_and_a_dot_are_lexer_errors() {
        rejected("radar-beta\n---\naxis A");
        rejected("radar-beta\naxis A.b");
    }
}
