//! The quirks of the four railroad grammars and their lexers that the port keeps, one test per probe
//! of `docs-port/railroad/NOTES.md`, section 2.8 (the ids `R-`, `E-`, `A-`, `P-` and `X-`), and the
//! quirks of section 2.9. Each expected value is the reference's, and the differential run checks the
//! same inputs against the reference (`tools/port/cases.ts`).

use ferrigura_model::railroad::{RailroadDiagram, RailroadExpr, RailroadRule};
use ferrigura_syntax::railroad::parse;

const INF: f64 = f64::INFINITY;

fn parsed(text: &str) -> RailroadDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

fn rejected(text: &str) {
    assert!(parse(text).is_err(), "{text:?} should be rejected");
}

fn rules(text: &str) -> Vec<RailroadRule> {
    parsed(text).rules
}

fn definition(text: &str) -> RailroadExpr {
    let mut all = rules(text);
    assert_eq!(all.len(), 1, "one rule expected in {text:?}");
    all.remove(0).definition
}

fn t(value: &str) -> RailroadExpr {
    RailroadExpr::Terminal {
        value: value.to_owned(),
    }
}

fn nt(name: &str) -> RailroadExpr {
    RailroadExpr::Nonterminal {
        name: name.to_owned(),
    }
}

fn sp(text: &str) -> RailroadExpr {
    RailroadExpr::Special {
        text: text.to_owned(),
    }
}

fn seq(elements: Vec<RailroadExpr>) -> RailroadExpr {
    RailroadExpr::Sequence { elements }
}

fn opt(element: RailroadExpr) -> RailroadExpr {
    RailroadExpr::Optional {
        element: Box::new(element),
    }
}

fn rep(element: RailroadExpr, min: f64, max: f64) -> RailroadExpr {
    RailroadExpr::Repetition {
        element: Box::new(element),
        min,
        max,
    }
}

const IR: &str = "railroad-beta\n";
const EBNF: &str = "railroad-ebnf-beta\n";
const ABNF: &str = "railroad-abnf-beta\n";
const PEG: &str = "railroad-peg-beta\n";

// Section 2.8, the IR grammar.

/// R-05: the TITLE token eats the prefix of an identifier; `titles` is the title and the rule `s`.
#[test]
fn r05_a_title_keyword_eats_the_prefix_of_a_word() {
    let diagram = parsed(&format!("{IR}titles = terminal(\"a\") ;\n"));
    assert_eq!(diagram.title, "");
    assert_eq!(diagram.rules[0].name, "s");
    assert_eq!(diagram.rules[0].definition, t("a"));
}

/// R-06, R-56: a keyword is not a rule name; R-07 an identifier with a longer name is.
#[test]
fn r06_r07_r56_word_keywords_need_the_longer_identifier() {
    rejected(&format!("{IR}sequence = terminal(\"a\") ;\n"));
    rejected(&format!("{IR}nonterminal = terminal(\"a\") ;\n"));
    assert_eq!(
        rules(&format!("{IR}sequenceX = terminal(\"a\") ;\n"))[0].name,
        "sequenceX"
    );
}

/// R-48: a capital does not make a keyword, and the call names are case-sensitive.
#[test]
fn r48_calls_are_case_sensitive() {
    rejected(&format!("{IR}Terminal(\"a\")\n"));
}

/// R-47, X-17, X-22: header lines after the first rule are errors; X-29 and X-30: the last one wins,
/// even when it is empty.
#[test]
fn r47_x17_x22_header_lines_come_first_and_the_last_wins() {
    rejected(&format!("{IR}r = terminal(\"a\") ;\ntitle foo\n"));
    rejected(&format!("{IR}r = terminal(\"a\") ;\naccTitle: x\n"));
    rejected(&format!("{IR}r = terminal(\"a\") ;\ntitle \"x\"\n"));
    assert_eq!(
        parsed(&format!("{IR}title foo\ntitle\nr = terminal(\"a\") ;\n")).title,
        ""
    );
    let diagram = parsed(&format!(
        "{IR}accTitle: foo\naccTitle:\nr = terminal(\"a\") ;\n"
    ));
    assert_eq!(diagram.acc_title, "");
}

/// R-01, R-03, R-12, R-14: a rule, a bare header, a one-element sequence and a trailing token.
#[test]
fn r01_r03_r12_r14_basic_forms() {
    assert_eq!(
        definition(&format!("{IR}digit = terminal(\"0\") ;")),
        t("0")
    );
    assert!(rules(IR).is_empty());
    assert_eq!(
        definition(&format!("{IR}r = sequence(terminal(\"a\")) ;")),
        t("a")
    );
    rejected(&format!("{IR}r = terminal(\"a\") ; junk"));
}

/// R-08, R-34, R-35, R-48: a missing `;`, a comma after a rule, an empty sequence, a capital call.
#[test]
fn r08_r34_r35_malformed_rules_are_rejected() {
    rejected(&format!("{IR}r = terminal(\"a\")"));
    rejected(&format!("{IR}r = terminal(\"a\"),"));
    rejected(&format!("{IR}r = sequence() ;"));
}

/// R-09, R-11: escapes in a string; R-11 decodes `\n`, `\t` and a backslash escape.
#[test]
fn r09_r11_string_escapes() {
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"a\\\"b\") ;")),
        t("a\"b")
    );
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"a\\nb\\tc\\\\d\") ;")),
        t("a\nb\tc\\d")
    );
}

/// R-18, X-26, X-27, X-28: the DB sanitises and keeps the entity placeholders.
#[test]
fn r18_x26_x27_x28_the_db_sanitises_and_keeps_the_placeholders() {
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"<b>x</b> & y\") ;")),
        t("<b>x</b> &amp; y")
    );
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"&lt;b&gt;\") ;")),
        t("&lt;b&gt;")
    );
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"#amp;\") ;")),
        t("ﬂ°amp¶ß")
    );
    assert_eq!(
        definition(&format!("{IR}r = nonterminal(\"AT&T <b>bold</b>\") ;")),
        nt("AT&amp;T <b>bold</b>")
    );
}

/// R-19, R-22, R-20, R-21: duplicate rules are kept, the last title line wins, the title is
/// unquoted and its escapes decoded.
#[test]
fn r19_r20_r21_r22_titles_and_duplicates() {
    assert_eq!(
        rules(&format!(
            "{IR}r = terminal(\"a\") ;\nr = terminal(\"b\") ;\n"
        ))
        .len(),
        2
    );
    assert_eq!(
        parsed(&format!(
            "{IR}title foo\ntitle bar\nr = terminal(\"a\") ;\n"
        ))
        .title,
        "bar"
    );
    assert_eq!(
        parsed(&format!(
            "{IR}title Hello   <b>x</b>\nr = terminal(\"a\") ;\n"
        ))
        .title,
        "Hello <b>x</b>"
    );
    assert_eq!(
        parsed(&format!(
            "{IR}title \"Quoted \\\"t\\\"\"\nr = terminal(\"a\") ;\n"
        ))
        .title,
        "Quoted \"t\""
    );
}

/// X-13: a single-quoted title is unquoted too.
#[test]
fn x13_single_quoted_titles_are_unquoted() {
    assert_eq!(
        parsed(&format!("{IR}title 'it'\nr = terminal(\"a\") ;\n")).title,
        "it"
    );
}

/// R-37, R-38, R-25, R-27, R-28, R-31, R-57, R-59: YAML blocks, comments, CRLF and raw line breaks.
#[test]
fn r25_r27_r28_r31_r37_r38_r57_r59_hidden_text_and_breaks() {
    assert!(parse(&format!("{IR}---\nx: 1\n---\nr = terminal(\"a\") ;\n")).is_ok());
    rejected(&format!("{IR}---\nr = terminal(\"a\") ;\n"));
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"a\") ; %% c\n")),
        t("a")
    );
    assert_eq!(
        definition(&format!("{IR}%%{{init: x}}%%\nr = terminal(\"a\") ;\n")),
        t("a")
    );
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"a\") ;\r\n")),
        t("a")
    );
    assert_eq!(
        rules(&format!(
            "{IR}a = terminal(\"x\") ; b = terminal(\"y\") ;\n"
        ))
        .len(),
        2
    );
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"a\nb\") ;")),
        t("a\nb")
    );
}

/// R-40, R-41: rule names are `[A-Z_a-z][\w-]*`.
#[test]
fn r40_r41_rule_name_shape() {
    for name in ["r-2", "_r", "R_x9"] {
        assert_eq!(
            rules(&format!("{IR}{name} = terminal(\"a\") ;\n"))[0].name,
            name
        );
    }
    rejected(&format!("{IR}1r = terminal(\"a\") ;\n"));
}

/// X-01 to X-05: the header is case-sensitive, and a suffix glued to it is not the header. The EBNF
/// text of the same body is valid EBNF, since `terminal` is a name there, so it is not rejected.
#[test]
fn x01_to_x05_headers_are_case_sensitive_and_bounded() {
    assert!(parse("railroad-ebnf-beta\nr = terminal(\"a\") ;\n").is_ok());
    rejected("RAILROAD-BETA\nr = terminal(\"a\") ;\n");
    rejected("railroad-betax\nr = terminal(\"a\") ;\n");
    rejected("railroad-beta\nr = terminal(\"a\") ;\nr = ;");
}

/// X-06, X-08, X-09: a header followed by a comment, an indented header, a comment before it.
#[test]
fn x06_x08_x09_header_placement() {
    assert!(parse("railroad-beta%%c\n").is_ok());
    assert!(parse("  railroad-beta\n  r = terminal(\"a\") ;\n").is_ok());
    assert!(parse("%% before\nrailroad-beta\n").is_ok());
}

/// X-23: the IR escapes are `\n`, `\r` and `\t` only; `\b`, `\f`, `\v` and `\0` stand for the letter.
#[test]
fn x23_ir_decodes_only_three_escapes() {
    assert_eq!(
        definition(&format!("{IR}r = terminal(\"a\\bc\\fd\\ve\\0f\") ;")),
        t("abcfdve0f")
    );
}

/// Section 2.9, quirk 16 and the header case (quirk 1): a name that is not a rule name.
#[test]
fn quirk16_a_leading_digit_is_no_name() {
    rejected(&format!("{IR}1x = terminal(\"a\") ;\n"));
}

// Section 2.8, the EBNF grammar.

/// E-01, E-02, E-03, E-04: the two definition forms and the comma or juxtaposition of a sequence.
#[test]
fn e01_to_e04_definitions_and_sequences() {
    assert_eq!(definition(&format!("{EBNF}rule = \"a\" ;")), t("a"));
    assert_eq!(definition(&format!("{EBNF}rule ::= \"a\" ;")), t("a"));
    assert_eq!(
        definition(&format!("{EBNF}rule = \"a\" , \"b\" ;")),
        seq(vec![t("a"), t("b")])
    );
    assert_eq!(
        definition(&format!("{EBNF}rule = \"a\" \"b\" ;")),
        seq(vec![t("a"), t("b")])
    );
}

/// E-05, E-06, E-08, E-09, E-10: optional and repetition forms.
#[test]
fn e05_to_e10_optional_and_repetition() {
    assert_eq!(definition(&format!("{EBNF}r = [ \"a\" ] ;")), opt(t("a")));
    assert_eq!(
        definition(&format!("{EBNF}r = {{ \"a\" }} ;")),
        rep(t("a"), 0.0, INF)
    );
    assert_eq!(definition(&format!("{EBNF}r = \"a\"? ;")), opt(t("a")));
    assert_eq!(
        definition(&format!("{EBNF}r = \"a\"* ;")),
        rep(t("a"), 0.0, INF)
    );
    assert_eq!(
        definition(&format!("{EBNF}r = \"a\"+ ;")),
        rep(t("a"), 1.0, INF)
    );
}

/// E-11, quirk 5: the exception `A - B` is the sequence of `A`, the text `-` and `B`.
#[test]
fn e11_the_exception_is_literal_text() {
    assert_eq!(
        definition(&format!("{EBNF}r = \"a\" - \"b\" ;")),
        seq(vec![t("a"), t("-"), t("b")])
    );
}

/// E-12, E-13, E-14, E-28, quirk 6: the special sequence, trimmed, with its line break kept.
#[test]
fn e12_e13_e14_e28_special_sequences() {
    assert_eq!(
        definition(&format!("{EBNF}r = ? special ? ;")),
        sp("special")
    );
    assert_eq!(definition(&format!("{EBNF}r = ?a? ;")), sp("a"));
    rejected(&format!("{EBNF}r = ? ? ;"));
    assert_eq!(definition(&format!("{EBNF}r = ? a\nb ? ;")), sp("a\nb"));
}

/// E-15, E-16, E-17: an ISO comment, unclosed, and a string that is not a comment.
#[test]
fn e15_e16_e17_iso_comments() {
    assert_eq!(
        definition(&format!("{EBNF}r = (* iso comment *) \"a\" ;")),
        t("a")
    );
    rejected(&format!("{EBNF}r = (* unclosed ;"));
    assert_eq!(definition(&format!("{EBNF}r = \"(*\" ;")), t("(*"));
}

/// E-23, E-35, quirk 7: postfixes nest in their order.
#[test]
fn e23_e35_postfixes_nest() {
    assert_eq!(
        definition(&format!("{EBNF}r = {{\"a\"}}* ;")),
        rep(rep(t("a"), 0.0, INF), 0.0, INF)
    );
    assert_eq!(
        definition(&format!("{EBNF}r = [ \"a\" ]? ;")),
        opt(opt(t("a")))
    );
}

/// E-41: single quotes are strings; E-43: a `---` on its own line is an error.
#[test]
fn e41_e43_single_quotes_and_yaml() {
    assert_eq!(definition(&format!("{EBNF}r = 'a' ;")), t("a"));
    rejected(&format!("{EBNF}r = \"a\" ;\n---\n"));
}

/// E-22, E-37: a rule without its `;` runs into the next rule; a special after the `;` is an error.
#[test]
fn e22_e37_missing_terminators_are_errors() {
    rejected(&format!("{EBNF}a = \"x\"\nb = \"y\" ;"));
    rejected(&format!("{EBNF}r = \"a\" ; ? x ?"));
}

// Section 2.8, the ABNF grammar.

/// A-01, A-03, A-04, A-05, A-25, A-43, quirk 10: a numeric value is kept as written.
#[test]
fn a01_a03_a04_a05_a25_a43_numeric_values_are_kept() {
    assert_eq!(definition(&format!("{ABNF}r = \"a\" ;")), t("a"));
    assert_eq!(definition(&format!("{ABNF}r = %x41-5A ;")), t("%x41-5A"));
    assert_eq!(definition(&format!("{ABNF}r = %x41.42 ;")), t("%x41.42"));
    assert_eq!(
        definition(&format!("{ABNF}r = %d65 %b1010 ;")),
        seq(vec![t("%d65"), t("%b1010")])
    );
    assert_eq!(definition(&format!("{ABNF}r = %X4a ;")), t("%X4a"));
    assert_eq!(definition(&format!("{ABNF}r = %x4 ;")), t("%x4"));
}

/// A-26: a numeric range that ends in a dash is an error.
#[test]
fn a26_a_range_must_end_in_digits() {
    rejected(&format!("{ABNF}r = %x41- ;"));
}

/// A-06 to A-12, A-37, A-38, quirk 9: the repeats, with the min and the max kept.
#[test]
fn a06_to_a12_a37_a38_repeats() {
    assert_eq!(
        definition(&format!("{ABNF}r = 3DIGIT ;")),
        rep(nt("DIGIT"), 3.0, 3.0)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = 1*3DIGIT ;")),
        rep(nt("DIGIT"), 1.0, 3.0)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = *DIGIT ;")),
        rep(nt("DIGIT"), 0.0, INF)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = 1*DIGIT ;")),
        rep(nt("DIGIT"), 1.0, INF)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = *1DIGIT ;")),
        opt(nt("DIGIT"))
    );
    assert_eq!(
        definition(&format!("{ABNF}r = 0*1DIGIT ;")),
        opt(nt("DIGIT"))
    );
    assert_eq!(
        definition(&format!("{ABNF}r = 2 DIGIT ;")),
        rep(nt("DIGIT"), 2.0, 2.0)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = *0DIGIT ;")),
        rep(nt("DIGIT"), 0.0, 0.0)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = 2*1DIGIT ;")),
        rep(nt("DIGIT"), 2.0, 1.0)
    );
}

/// A-33, A-34: a repeat before a string.
#[test]
fn a33_a34_repeats_of_strings() {
    assert_eq!(
        definition(&format!("{ABNF}r = 3*\"a\" ;")),
        rep(t("a"), 3.0, INF)
    );
    assert_eq!(
        definition(&format!("{ABNF}r = *\"a\" ;")),
        rep(t("a"), 0.0, INF)
    );
}

/// A-35: a repeat with nothing to repeat is an error.
#[test]
fn a35_a_repeat_needs_an_element() {
    rejected(&format!("{ABNF}r = 3 ;"));
}

/// A-15, A-16, A-17, A-18, A-19, quirk 8: the ABNF `;` is a comment when text follows it on the line.
#[test]
fn a15_to_a19_the_semicolon_is_a_comment_when_text_follows() {
    rejected(&format!("{ABNF}r = \"a\" ; comment\n"));
    rejected(&format!("{ABNF}r = \"a\" ; \n"));
    assert_eq!(definition(&format!("{ABNF}r = \"a\";\n")), t("a"));
    rejected(&format!("{ABNF}r = \"a\";x\n"));
    assert_eq!(definition(&format!("{ABNF}r = \"a\"\n;\n")), t("a"));
}

/// A-40: a comment line between rules.
#[test]
fn a40_a_comment_line_between_rules() {
    assert_eq!(
        rules(&format!("{ABNF}r = \"a\" ;\n; comment line\ns = \"b\" ;\n")).len(),
        2
    );
}

/// A-22: a rule name with a dash; A-30: a quote inside a string; A-29: an unclosed string.
#[test]
fn a22_a29_a30_names_and_strings() {
    assert_eq!(definition(&format!("{ABNF}r = a-b ;")), nt("a-b"));
    assert_eq!(definition(&format!("{ABNF}r = \"a'b\" ;")), t("a'b"));
    rejected(&format!("{ABNF}r = \"a ;"));
}

/// A-32, quirk 10: a quoted title keeps its backslashes, unlike the IR title.
#[test]
fn a32_a_quoted_title_is_not_decoded() {
    assert_eq!(
        parsed(&format!("{ABNF}title \"a\\nb\"\nr = \"a\" ;")).title,
        "a\\nb"
    );
}

/// Quirk 16: the ABNF name is `[A-Za-z][A-Za-z0-9-]*`, so a leading underscore is a lexer error.
#[test]
fn quirk16_an_abnf_name_starts_with_a_letter() {
    rejected(&format!("{ABNF}_r = \"a\" ;"));
}

/// A-22 and its contrast: a name is an identifier, a string is a terminal.
#[test]
fn abnf_names_and_strings_are_different_forms() {
    assert_eq!(definition(&format!("{ABNF}r = ALPHA ;")), nt("ALPHA"));
}

// Section 2.8, the PEG grammar.

/// P-03, P-04, P-05, P-06, P-16, P-14, P-15, quirk 11: a lookahead's label is a quoted terminal, a
/// rule name, a special's text, or `(...)` for any other expression.
#[test]
fn p03_to_p16_lookahead_labels() {
    assert_eq!(
        definition(&format!("{PEG}R <- &\"a\" \"b\" ;")),
        seq(vec![sp("&\"a\""), t("b")])
    );
    assert_eq!(
        definition(&format!("{PEG}R <- !A B ;")),
        seq(vec![sp("!A"), nt("B")])
    );
    assert_eq!(definition(&format!("{PEG}R <- &(A B) ;")), sp("&(...)"));
    assert_eq!(definition(&format!("{PEG}R <- . ;")), sp("."));
    assert_eq!(definition(&format!("{PEG}R <- &A ;")), sp("&A"));
    assert_eq!(definition(&format!("{PEG}R <- &\"a\"* ;")), sp("&(...)"));
    assert_eq!(definition(&format!("{PEG}R <- !\"x\"? ;")), sp("!(...)"));
}

/// P-31: the label of a quoted terminal does not escape its own quote.
#[test]
fn p31_the_label_does_not_escape_the_quote() {
    assert_eq!(
        definition(&format!("{PEG}R <- &\"a\\\"\" ;")),
        sp("&\"a\"\"")
    );
}

/// P-07: the suffixes of a sequence; P-23: two dots are two specials.
#[test]
fn p07_p23_suffixes_and_dots() {
    assert_eq!(
        definition(&format!("{PEG}R <- \"a\"? \"b\"* \"c\"+ ;")),
        seq(vec![
            opt(t("a")),
            rep(t("b"), 0.0, INF),
            rep(t("c"), 1.0, INF)
        ])
    );
    assert_eq!(
        definition(&format!("{PEG}R <- . . ;")),
        seq(vec![sp("."), sp(".")])
    );
}

/// P-10, P-11, P-12, quirk 12: one prefix and one suffix at most.
#[test]
fn p10_p11_p12_prefixes_and_suffixes_are_single() {
    rejected(&format!("{PEG}R <- &&\"a\" ;"));
    rejected(&format!("{PEG}R <- ! & \"a\" ;"));
    rejected(&format!("{PEG}R <- \"a\"?* ;"));
}

/// P-26: the PEG rule uses `<-`; P-30: a line break in a string; P-20: an empty string.
#[test]
fn p20_p26_p30_definitions_and_strings() {
    rejected(&format!("{PEG}R = \"a\" ;"));
    assert_eq!(definition(&format!("{PEG}R <- \"\\n\" ;")), t("\n"));
    assert_eq!(definition(&format!("{PEG}R <- \"\" ;")), t(""));
}

/// P-09: a hash comment is hidden.
#[test]
fn p09_a_hash_comment_is_hidden() {
    assert_eq!(definition(&format!("{PEG}R <- \"a\" # comment\n;")), t("a"));
}

// Section 2.9, the rest.

/// Quirk 13: a line break inside an IR string is accepted (R-59), and a backslash before a line break
/// is an error (no escape can take it).
#[test]
fn quirk13_a_line_break_in_a_string_and_an_escaped_one() {
    assert!(parse(&format!("{IR}r = terminal(\"a\nb\") ;")).is_ok());
    rejected(&format!("{IR}r = terminal(\"a\\\nb\") ;"));
}

/// Quirk 15: a `---` block that is not closed is a lexer error (R-38).
#[test]
fn quirk15_an_unclosed_yaml_block_is_an_error() {
    rejected(&format!("{IR}r = terminal(\"a\") ;\n---\nx\n"));
}

/// Q5: the max of a repetition is kept, and a min above a max is kept (A-38).
#[test]
fn q5_the_max_of_a_repetition_is_kept() {
    match definition(&format!("{ABNF}r = 1*3\"a\" ;")) {
        RailroadExpr::Repetition { min, max, .. } => assert_eq!((min, max), (1.0, 3.0)),
        other => panic!("not a repetition: {other:?}"),
    }
}

/// Q8: a front matter title never reaches the model, in any syntax (probed through `Diagram.fromText`).
#[test]
fn q8_the_front_matter_title_is_dropped_in_every_syntax() {
    for body in [
        "railroad-beta\nr = terminal(\"a\") ;\n",
        "railroad-ebnf-beta\nr = \"a\" ;\n",
        "railroad-abnf-beta\nr = \"a\" ;\n",
        "railroad-peg-beta\nR <- \"a\" ;\n",
    ] {
        let text = format!("---\ntitle: FM\n---\n{body}");
        assert_eq!(parsed(&text).title, "", "{text:?}");
    }
}

/// Q8: a title in the grammar wins over the front matter, which is dropped anyway.
#[test]
fn q8_the_grammar_title_is_the_title() {
    assert_eq!(
        parsed("---\ntitle: FM\n---\nrailroad-beta\ntitle G\nr = terminal(\"a\") ;\n").title,
        "G"
    );
}
