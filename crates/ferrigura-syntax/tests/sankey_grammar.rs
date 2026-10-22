//! The grammar and DB quirks of the sankey diagram, one test per quirk of `docs-port/sankey/PLAN.md`,
//! section 7 (S1 to S15). Each result is the reference's, as `docs-port/sankey/NOTES.md` probed it
//! (the probe numbers are in the test names' comments) and as the differential run checks it.

use ferrigura_model::sankey::SankeyDiagram;
use ferrigura_syntax::sankey::parse;

fn parses(text: &str) -> SankeyDiagram {
    parse(text).expect("parses")
}

fn rejects(text: &str) {
    assert!(parse(text).is_err(), "the reference rejects {text:?}");
}

fn node_ids(diagram: &SankeyDiagram) -> Vec<&str> {
    diagram.nodes.iter().map(|node| node.id.as_str()).collect()
}

fn links(diagram: &SankeyDiagram) -> Vec<(&str, &str, f64)> {
    diagram
        .links
        .iter()
        .map(|link| (link.source.as_str(), link.target.as_str(), link.value))
        .collect()
}

// S1: the header is the keyword alone on its line (P04, P02, P03, P45).

#[test]
fn a_header_with_text_on_its_line_is_rejected() {
    rejects("sankey a,b,1");
    rejects("sankey   \na,b,1");
    rejects("sankey,a,b");
    rejects("sankey\ta,b,1");
}

#[test]
fn a_word_glued_to_the_keyword_is_rejected() {
    rejects("sankeyfoo\na,b,1");
}

/// The lexer's `<INITIAL>"sankey-beta"` rule, which upstream keeps before `"sankey"`, in any case.
#[test]
fn the_beta_keyword_is_taken_too() {
    assert_eq!(links(&parses("sankey-beta\na,b,1")), [("a", "b", 1.0)]);
    assert_eq!(links(&parses("Sankey-Beta\na,b,1")), [("a", "b", 1.0)]);
}

#[test]
fn the_keyword_is_taken_in_any_case_by_the_parser() {
    // The detector is case-sensitive; the parser is not (`cases.ts`, `undetectedCases`).
    assert_eq!(links(&parses("SANKEY\na,b,1")), [("a", "b", 1.0)]);
    assert_eq!(links(&parses("Sankey\na,b,1")), [("a", "b", 1.0)]);
}

// S2: at least one record; blank lines go; a line of blanks between records does not (P05-P07, P48,
// P50, P56).

#[test]
fn a_header_without_a_record_is_rejected() {
    rejects("sankey");
    rejects("sankey\n");
    rejects("sankey\n\n\n");
}

#[test]
fn blank_lines_are_removed_at_the_start_and_the_end() {
    let diagram = parses("sankey\n\n\na,b,1\n\n\nb,c,2\n");
    assert_eq!(links(&diagram), [("a", "b", 1.0), ("b", "c", 2.0)]);
    assert_eq!(node_ids(&diagram), ["a", "b", "c"]);
}

#[test]
fn a_line_of_blanks_between_records_is_rejected() {
    rejects("sankey\n  \n a,b,1");
    rejects("sankey\na,b,1\n   \nc,d,2");
    rejects("sankey\na,b,1\n\t\nc,d,2");
    rejects("sankey\na,b,1\t\nc,d,2");
}

#[test]
fn blanks_at_the_very_end_are_removed() {
    assert_eq!(links(&parses("sankey\na,b,1   ")), [("a", "b", 1.0)]);
    assert_eq!(links(&parses("sankey\na,b,1\t")), [("a", "b", 1.0)]);
}

#[test]
fn a_blank_at_the_start_of_a_record_is_trimmed_from_its_name() {
    let diagram = parses("sankey\n   a , b , 1");
    assert_eq!(links(&diagram), [("a", "b", 1.0)]);
}

// S3: CRLF reads as LF (P25).

#[test]
fn crlf_reads_as_lf() {
    let diagram = parses("sankey\na,b,1\r\nb,c,2\r\n");
    assert_eq!(links(&diagram), [("a", "b", 1.0), ("b", "c", 2.0)]);
}

// S4: a record has exactly three fields (P20, P21, P57).

#[test]
fn a_record_has_exactly_three_fields() {
    rejects("sankey\na,b");
    rejects("sankey\na");
    rejects("sankey\na,b,1,2");
    rejects("sankey\na,b,1,");
    rejects("sankey\na,b,1\nc,d");
}

// S5: an empty field is an error, but a field of blanks is an empty name (P13-P15, P58, P59; the
// blank name and value are the port's own probes, checked by the differential run).

#[test]
fn an_empty_field_is_rejected() {
    rejects("sankey\n,b,1");
    rejects("sankey\na,b,");
    rejects("sankey\n,,1");
    rejects("sankey\na,b,1\n,,\n");
}

#[test]
fn a_field_of_blanks_is_the_empty_name() {
    let diagram = parses("sankey\n ,b,1");
    assert_eq!(node_ids(&diagram), ["", "b"]);
    assert_eq!(links(&diagram), [("", "b", 1.0)]);
}

#[test]
fn a_quoted_field_of_blanks_is_the_empty_name() {
    let diagram = parses("sankey\n\"  \",b,1");
    assert_eq!(node_ids(&diagram), ["", "b"]);
}

#[test]
fn a_value_of_blanks_in_the_middle_is_not_a_number() {
    let diagram = parses("sankey\na,b, \nc,d,1");
    let value = diagram.links[0].value;
    assert!(value.is_nan(), "the value is {value}");
    assert_eq!(diagram.links[1].value, 1.0);
}

// S6: quoted fields. `""` is one quote, a line break is kept in the name, and only the closing quote
// may follow a quoted field (P10-P13, P37, P46, P47, P53, P54).

#[test]
fn a_quoted_field_keeps_its_commas_and_its_quotes_as_written() {
    assert_eq!(node_ids(&parses("sankey\n\"a,x\",b,1")), ["a,x", "b"]);
    assert_eq!(
        node_ids(&parses("sankey\n\"a \"\"q\"\" b\",c,1")),
        ["a \"q\" b", "c"]
    );
    assert_eq!(node_ids(&parses("sankey\n\"a\"\"\",c,1")), ["a\"", "c"]);
}

#[test]
fn a_quoted_field_keeps_its_line_breaks_and_collapses_runs_of_them() {
    assert_eq!(node_ids(&parses("sankey\n\"a\nb\",c,1")), ["a\nb", "c"]);
    assert_eq!(node_ids(&parses("sankey\n\"a\n\n\nb\",c,1")), ["a\nb", "c"]);
}

#[test]
fn a_quoted_field_is_trimmed_before_its_quotes_are_resolved() {
    assert_eq!(node_ids(&parses("sankey\n\" a \",b,1")), ["a", "b"]);
}

#[test]
fn a_quoted_field_takes_nothing_after_its_closing_quote() {
    rejects("sankey\n\"a\" ,b,1");
    rejects("sankey\n\"a\"b,c,1");
}

#[test]
fn a_quoted_field_must_close() {
    rejects("sankey\n\"a,b,1");
    rejects("sankey\n\"\"\",b,1");
    rejects("sankey\n\"\"\"\"\",b,1");
}

#[test]
fn an_empty_quoted_field_is_rejected() {
    rejects("sankey\n\"\",b,1");
}

#[test]
fn a_quote_in_the_middle_of_an_unquoted_field_is_rejected() {
    rejects("sankey\na\"b,c,1");
}

// S7: text outside printable ASCII is an error: non-ASCII anywhere, a tab in the middle of a field
// (P22-P24). A tab at the very end is removed with the other trailing blanks.

#[test]
fn non_ascii_text_is_rejected_quoted_or_not() {
    rejects("sankey\nK\u{e4}se,b,1");
    rejects("sankey\n\"K\u{e4}se\",b,1");
    rejects("sankey\n\u{c4},b,1");
}

#[test]
fn a_tab_inside_a_field_is_rejected() {
    rejects("sankey\na\tb,c,1");
    rejects("sankey\n\ta,b,1");
}

// S8: an entity is a placeholder that is not ASCII, so an entity in a name is an error; `&quot;`
// without `#` is text (E01-E09).

#[test]
fn an_entity_in_a_name_is_rejected() {
    rejects("sankey\na#quot;x,b,1");
    rejects("sankey\na#9829;x,b,1");
    rejects("sankey\na#unknown;x,b,1");
    rejects("sankey\n\"a#quot;x\",b,1");
}

#[test]
fn a_named_reference_without_the_hash_is_text() {
    let diagram = parses("sankey\na&quot;x,b,1");
    assert_eq!(node_ids(&diagram), ["a&quot;x", "b"]);
}

// S9: the value is `parseFloat` of the field, and a value that is not finite is its sentinel (P16,
// P17-P19, P27, P39-P41, P51, P61-P63).

fn value_of(field: &str) -> f64 {
    let diagram = parses(&format!("sankey\na,b,{field}"));
    diagram.links[0].value
}

#[test]
fn a_value_is_read_as_parse_float_reads_it() {
    assert_eq!(value_of("10abc"), 10.0);
    assert_eq!(value_of("1e2"), 100.0);
    assert_eq!(value_of("1e"), 1.0);
    assert_eq!(value_of("  5  "), 5.0);
    assert_eq!(value_of("0x10"), 0.0);
    assert_eq!(value_of(".5"), 0.5);
    assert_eq!(value_of("+3"), 3.0);
    assert_eq!(value_of("1_000"), 1.0);
    assert_eq!(value_of("1 %% trailing"), 1.0);
    assert_eq!(value_of("1;"), 1.0);
    assert_eq!(value_of("1#x"), 1.0);
    assert_eq!(value_of("0"), 0.0);
    assert_eq!(value_of("-3"), -3.0);
}

#[test]
fn a_value_that_is_not_a_number_is_nan() {
    assert!(value_of("x").is_nan());
    assert!(value_of("NaN").is_nan());
    assert!(value_of("-").is_nan());
    assert!(value_of("\"x\"").is_nan());
}

#[test]
fn a_value_beyond_the_double_range_is_infinite() {
    assert_eq!(value_of("Infinity"), f64::INFINITY);
    assert_eq!(value_of("-Infinity"), f64::NEG_INFINITY);
    assert_eq!(value_of("2e308"), f64::INFINITY);
}

#[test]
fn a_quoted_value_is_read_as_its_text() {
    assert_eq!(value_of("\"1,2\""), 1.0);
}

// S10: names are sanitised as the DB sanitises them (P33-P35, D05-D11), and nodes are keyed by the
// sanitised name.

#[test]
fn a_name_with_markup_is_sanitised() {
    let diagram = parses("sankey\n<script>x</script>n,B,1");
    assert_eq!(node_ids(&diagram), ["n", "B"]);
    let diagram = parses("sankey\n<img src=x onerror=alert(1)>n,B,1");
    assert_eq!(node_ids(&diagram), ["<img src=\"x\">n", "B"]);
    let diagram = parses("sankey\n<i>it</i>,B,1");
    assert_eq!(node_ids(&diagram), ["<i>it</i>", "B"]);
    let diagram = parses("sankey\na<br>b,B,1");
    assert_eq!(node_ids(&diagram), ["a<br>b", "B"]);
}

#[test]
fn an_open_tag_swallows_the_rest_of_the_name() {
    assert_eq!(node_ids(&parses("sankey\na<b,B,1")), ["a", "B"]);
    assert_eq!(node_ids(&parses("sankey\na<,B,1")), ["a&lt;", "B"]);
}

#[test]
fn a_name_without_an_open_angle_bracket_is_kept_as_written() {
    assert_eq!(node_ids(&parses("sankey\na&b,B,1")), ["a&b", "B"]);
    assert_eq!(node_ids(&parses("sankey\na=b,B,1")), ["a=b", "B"]);
    assert_eq!(
        node_ids(&parses("sankey\n&lt;x&gt;,B,1")),
        ["&lt;x&gt;", "B"]
    );
}

#[test]
fn names_that_sanitise_alike_are_one_node() {
    let diagram = parses("sankey\na<b,c,1\na,c,2");
    assert_eq!(node_ids(&diagram), ["a", "c"]);
    assert_eq!(links(&diagram), [("a", "c", 1.0), ("a", "c", 2.0)]);
}

#[test]
fn names_keep_their_punctuation() {
    let diagram = parses("sankey\nA&B,x'y/z-w,1");
    assert_eq!(node_ids(&diagram), ["A&B", "x'y/z-w"]);
}

// S11: duplicate links are kept, and a cycle parses; the layout rejects a cycle, not the parser (P28,
// P29, P30, P60).

#[test]
fn duplicate_links_are_kept() {
    let diagram = parses("sankey\na,b,1\na,b,2");
    assert_eq!(node_ids(&diagram), ["a", "b"]);
    assert_eq!(links(&diagram), [("a", "b", 1.0), ("a", "b", 2.0)]);
}

#[test]
fn a_cycle_parses() {
    assert_eq!(links(&parses("sankey\na,a,1")), [("a", "a", 1.0)]);
    assert_eq!(links(&parses("sankey\na,b,1\nb,a,1")).len(), 2);
    assert_eq!(links(&parses("sankey\na,b,1\nb,c,1\nc,a,1")).len(), 3);
}

// S12: prototype names are ordinary ids in the DB (P31, P32, Q03).

#[test]
fn prototype_names_are_ordinary_ids() {
    let diagram = parses("sankey\nconstructor,b,1\n__proto__,b,2");
    assert_eq!(node_ids(&diagram), ["constructor", "b", "__proto__"]);
    assert_eq!(
        links(&diagram),
        [("constructor", "b", 1.0), ("__proto__", "b", 2.0)]
    );
    let diagram = parses("sankey\nhasOwnProperty,B,1");
    assert_eq!(node_ids(&diagram), ["hasOwnProperty", "B"]);
}

// S13: the front matter title is the DB title: sanitised, and entities stay as written (V09).

#[test]
fn the_front_matter_title_is_the_db_title() {
    assert_eq!(
        parses("---\ntitle: Front title\n---\nsankey\na,b,1").title,
        "Front title"
    );
}

#[test]
fn the_front_matter_title_is_sanitised() {
    assert_eq!(
        parses("---\ntitle: <b>x</b>\n---\nsankey\na,b,1").title,
        "<b>x</b>"
    );
    assert_eq!(parses("---\ntitle: a<b\n---\nsankey\na,b,1").title, "a");
}

#[test]
fn an_empty_front_matter_title_is_empty() {
    assert_eq!(parses("---\ntitle: \"\"\n---\nsankey\na,b,1").title, "");
}

#[test]
fn an_entity_in_the_front_matter_title_is_kept_as_written() {
    assert_eq!(
        parses("---\ntitle: \"FM #9829;\"\n---\nsankey\na,b,1").title,
        "FM #9829;"
    );
}

#[test]
fn the_header_texts_are_empty_without_a_statement() {
    let diagram = parses("---\ntitle: T\n---\nsankey\na,b,1");
    assert_eq!(diagram.acc_title, "");
    assert_eq!(diagram.acc_descr, "");
    assert_eq!(diagram.version, 1);
}

// S14: comment lines and directives go before the parse; a `%%` in the middle of a line is text
// (P26, P27, P64-P66).

#[test]
fn comment_lines_and_directives_are_removed() {
    assert_eq!(links(&parses("sankey\n%% comment\na,b,1")).len(), 1);
    assert_eq!(
        links(&parses("sankey\na,b,1\n%% only comment\nb,c,2")).len(),
        2
    );
    assert_eq!(
        links(&parses("sankey\na,b,1\n%%{init: {}}%%\nb,c,2")).len(),
        2
    );
    assert_eq!(
        links(&parses("%% a comment before the header\nsankey\na,b,1")).len(),
        1
    );
}

#[test]
fn a_percent_pair_inside_a_line_is_text() {
    assert_eq!(value_of("1 %% trailing"), 1.0);
}

#[test]
fn a_comment_line_of_one_percent_pair_alone_is_rejected() {
    rejects("sankey\n%%\na,b,1");
}

// S15: the grammar has no title or accessibility statement, and `sankey` is a plain name in a record.

#[test]
fn a_title_or_accessibility_line_is_rejected() {
    rejects("sankey\naccTitle: x\na,b,1");
    rejects("sankey\ntitle: x\na,b,1");
}

#[test]
fn the_keyword_is_a_plain_name_in_a_record() {
    let diagram = parses("sankey\nsankey,c,2\na,b,1");
    assert_eq!(node_ids(&diagram), ["sankey", "c", "a", "b"]);
}

#[test]
fn a_rejected_line_reports_its_line() {
    // The line of blanks is the third line of the text the grammar reads.
    let error = parse("sankey\na,b,1\n\t\nc,d,2").expect_err("rejected");
    assert_eq!(error.line, 3);
}
