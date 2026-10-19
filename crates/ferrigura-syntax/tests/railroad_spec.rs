//! The reference's railroad parser specs (`packages/mermaid/src/diagrams/railroad/parser/`), for the
//! four syntaxes: `railroadDiagram.spec.ts`, `ebnfDiagram.spec.ts`, `abnfDiagram.spec.ts` and
//! `pegDiagram.spec.ts`. The reference reads the rules from `db.getRules()`, and so does the port,
//! through the model. The quirks of the grammars are in `railroad_grammar.rs`, and the DB's
//! behaviour in `railroad_db_spec.rs`.

use ferrigura_model::railroad::{RailroadDiagram, RailroadExpr, RailroadRule};
use ferrigura_syntax::railroad::parse;

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

fn terminal(value: &str) -> RailroadExpr {
    RailroadExpr::Terminal {
        value: value.to_owned(),
    }
}

fn nonterminal(name: &str) -> RailroadExpr {
    RailroadExpr::Nonterminal {
        name: name.to_owned(),
    }
}

fn special(text: &str) -> RailroadExpr {
    RailroadExpr::Special {
        text: text.to_owned(),
    }
}

fn sequence(elements: Vec<RailroadExpr>) -> RailroadExpr {
    RailroadExpr::Sequence { elements }
}

fn choice(alternatives: Vec<RailroadExpr>) -> RailroadExpr {
    RailroadExpr::Choice { alternatives }
}

fn optional(element: RailroadExpr) -> RailroadExpr {
    RailroadExpr::Optional {
        element: Box::new(element),
    }
}

fn repeat(element: RailroadExpr, min: f64, max: f64) -> RailroadExpr {
    RailroadExpr::Repetition {
        element: Box::new(element),
        min,
        max,
    }
}

const INF: f64 = f64::INFINITY;

/// The minimum of a repetition at the top of a definition.
fn min_of(expr: &RailroadExpr) -> f64 {
    match expr {
        RailroadExpr::Repetition { min, .. } => *min,
        other => panic!("not a repetition: {other:?}"),
    }
}

mod ir {
    use super::*;

    const HEAD: &str = "railroad-beta\n";

    fn ir(body: &str) -> String {
        format!("{HEAD}{body}")
    }

    #[test]
    fn should_parse_terminal() {
        let all = rules(&ir("rule = terminal(\"hello\") ;\n"));
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "rule");
        assert_eq!(all[0].definition, terminal("hello"));
    }

    #[test]
    fn should_parse_nonterminal() {
        assert_eq!(
            definition(&ir("rule = nonterminal(\"expression\") ;\n")),
            nonterminal("expression")
        );
    }

    #[test]
    fn should_parse_sequence() {
        assert_eq!(
            definition(&ir(
                "rule = sequence(terminal(\"a\"), terminal(\"b\"), terminal(\"c\")) ;\n"
            )),
            sequence(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_collapse_single_element_sequence() {
        assert_eq!(
            definition(&ir("rule = sequence(terminal(\"a\")) ;\n")),
            terminal("a")
        );
    }

    #[test]
    fn should_parse_choice() {
        assert_eq!(
            definition(&ir(
                "rule = choice(terminal(\"a\"), terminal(\"b\"), terminal(\"c\")) ;\n"
            )),
            choice(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_collapse_single_element_choice() {
        assert_eq!(
            definition(&ir("rule = choice(terminal(\"a\")) ;\n")),
            terminal("a")
        );
    }

    #[test]
    fn should_parse_optional() {
        assert_eq!(
            definition(&ir("rule = optional(terminal(\"a\")) ;\n")),
            optional(terminal("a"))
        );
    }

    #[test]
    fn should_parse_zero_or_more_with_min_zero_and_max_infinity() {
        assert_eq!(
            definition(&ir("rule = zeroOrMore(terminal(\"a\")) ;\n")),
            repeat(terminal("a"), 0.0, INF)
        );
    }

    #[test]
    fn should_parse_one_or_more_with_min_one() {
        assert_eq!(
            definition(&ir("rule = oneOrMore(terminal(\"a\")) ;\n")),
            repeat(terminal("a"), 1.0, INF)
        );
    }

    #[test]
    fn should_parse_special() {
        assert_eq!(
            definition(&ir("rule = special(\"any character\") ;\n")),
            special("any character")
        );
    }

    #[test]
    fn should_parse_title() {
        let diagram = parsed(&ir("title \"Test Grammar\"\nrule = terminal(\"a\") ;\n"));
        assert_eq!(diagram.title, "Test Grammar");
    }

    #[test]
    fn should_parse_multiple_rules_in_order() {
        let names: Vec<String> = rules(&ir(
            "rule1 = terminal(\"a\") ;\nrule2 = terminal(\"b\") ;\nrule3 = terminal(\"c\") ;\n",
        ))
        .into_iter()
        .map(|rule| rule.name)
        .collect();
        assert_eq!(names, ["rule1", "rule2", "rule3"]);
    }

    #[test]
    fn should_handle_nested_structures() {
        assert_eq!(
            definition(&ir(
                "rule = sequence(\n    choice(terminal(\"a\"), terminal(\"b\")),\n    oneOrMore(choice(terminal(\"c\"), terminal(\"d\")))\n) ;\n"
            )),
            sequence(vec![
                choice(vec![terminal("a"), terminal("b")]),
                repeat(choice(vec![terminal("c"), terminal("d")]), 1.0, INF),
            ])
        );
    }

    #[test]
    fn should_parse_a_complex_expression_grammar() {
        let all = rules(&ir(
            "digit = choice(\n    terminal(\"0\"), terminal(\"1\"), terminal(\"2\"),\n    terminal(\"3\"), terminal(\"4\"), terminal(\"5\"),\n    terminal(\"6\"), terminal(\"7\"), terminal(\"8\"), terminal(\"9\")\n) ;\nnumber = oneOrMore(nonterminal(\"digit\")) ;\n",
        ));
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "digit");
        assert_eq!(all[1].name, "number");
    }

    #[test]
    fn should_reject_a_text_without_the_keyword() {
        rejected("rule = terminal(\"a\") ;\n");
    }

    #[test]
    fn should_reject_a_missing_semicolon() {
        rejected("railroad-beta\nrule = terminal(\"a\")\n");
    }

    #[test]
    fn should_reject_an_unterminated_string() {
        rejected("railroad-beta\nrule = terminal(\"unclosed) ;\n");
    }

    #[test]
    fn should_handle_an_empty_rules_list() {
        let diagram = parsed("railroad-beta\ntitle \"Empty Grammar\"\n");
        assert!(diagram.rules.is_empty());
        assert_eq!(diagram.title, "Empty Grammar");
    }

    #[test]
    fn should_keep_whitespace_in_strings() {
        assert_eq!(
            definition(&ir("rule = terminal(\"hello world\") ;\n")),
            terminal("hello world")
        );
    }

    #[test]
    fn should_handle_underscores_and_digits_in_rule_names() {
        assert_eq!(
            rules(&ir("my_rule = terminal(\"test\") ;\n"))[0].name,
            "my_rule"
        );
        assert_eq!(
            rules(&ir("rule123 = terminal(\"test\") ;\n"))[0].name,
            "rule123"
        );
    }

    #[test]
    fn should_parse_deeply_nested_sequences_of_choices() {
        assert_eq!(
            definition(&ir(
                "rule = sequence(\n    choice(terminal(\"a\"), terminal(\"b\")),\n    choice(terminal(\"c\"), terminal(\"d\"))\n) ;\n"
            )),
            sequence(vec![
                choice(vec![terminal("a"), terminal("b")]),
                choice(vec![terminal("c"), terminal("d")]),
            ])
        );
    }

    #[test]
    fn should_parse_zero_or_more_of_a_choice() {
        assert_eq!(
            definition(&ir(
                "rule = zeroOrMore(choice(terminal(\"a\"), terminal(\"b\"))) ;\n"
            )),
            repeat(choice(vec![terminal("a"), terminal("b")]), 0.0, INF)
        );
    }

    #[test]
    fn should_skip_block_comments() {
        assert_eq!(
            definition(&ir("/* this is a comment */\nrule = terminal(\"a\") ;\n")),
            terminal("a")
        );
    }

    #[test]
    fn should_parse_an_expression_grammar_with_its_title() {
        let text = ir(concat!(
            "title \"Simple Expression\"\n",
            "expression = sequence(\n    nonterminal(\"term\"),\n    zeroOrMore(sequence(\n        choice(terminal(\"+\"), terminal(\"-\")),\n        nonterminal(\"term\")\n    ))\n) ;\n",
            "term = sequence(\n    nonterminal(\"factor\"),\n    zeroOrMore(sequence(\n        choice(terminal(\"*\"), terminal(\"/\")),\n        nonterminal(\"factor\")\n    ))\n) ;\n",
            "factor = choice(\n    nonterminal(\"number\"),\n    sequence(terminal(\"(\"), nonterminal(\"expression\"), terminal(\")\"))\n) ;\n",
            "number = oneOrMore(nonterminal(\"digit\")) ;\n",
            "digit = choice(\n    terminal(\"0\"), terminal(\"1\"), terminal(\"2\"),\n    terminal(\"3\"), terminal(\"4\"), terminal(\"5\"),\n    terminal(\"6\"), terminal(\"7\"), terminal(\"8\"), terminal(\"9\")\n) ;\n",
        ));
        let diagram = parsed(&text);
        let names: Vec<&str> = diagram
            .rules
            .iter()
            .map(|rule| rule.name.as_str())
            .collect();
        assert_eq!(names, ["expression", "term", "factor", "number", "digit"]);
        assert_eq!(diagram.title, "Simple Expression");
    }

    #[test]
    fn should_parse_a_json_grammar_excerpt() {
        let text = ir(concat!(
            "title \"JSON Subset\"\n",
            "value = choice(\n    nonterminal(\"string\"),\n    nonterminal(\"number\"),\n    terminal(\"true\"),\n    terminal(\"false\"),\n    terminal(\"null\")\n) ;\n",
            "string = sequence(\n    terminal(\"\\\"\"),\n    zeroOrMore(nonterminal(\"character\")),\n    terminal(\"\\\"\")\n) ;\n",
            "number = oneOrMore(nonterminal(\"digit\")) ;\n",
            "character = choice(nonterminal(\"letter\"), nonterminal(\"digit\")) ;\n",
            "letter = choice(terminal(\"a\"), terminal(\"b\"), terminal(\"c\")) ;\n",
            "digit = choice(terminal(\"0\"), terminal(\"1\"), terminal(\"2\")) ;\n",
        ));
        let diagram = parsed(&text);
        assert_eq!(diagram.rules.len(), 6);
        assert_eq!(diagram.title, "JSON Subset");
        assert_eq!(
            diagram.rules[1].definition.clone(),
            sequence(vec![
                terminal("\""),
                repeat(nonterminal("character"), 0.0, INF),
                terminal("\"")
            ])
        );
    }
}

mod ebnf {
    use super::*;

    const HEAD: &str = "railroad-ebnf-beta\n";

    fn ebnf(body: &str) -> String {
        format!("{HEAD}{body}")
    }

    #[test]
    fn should_parse_a_simple_terminal() {
        let all = rules(&ebnf("rule = \"terminal\" ;\n"));
        assert_eq!(all[0].name, "rule");
        assert_eq!(all[0].definition, terminal("terminal"));
    }

    #[test]
    fn should_parse_a_non_terminal_reference() {
        assert_eq!(
            definition(&ebnf("rule = other_rule ;\n")),
            nonterminal("other_rule")
        );
    }

    #[test]
    fn should_parse_a_sequence() {
        assert_eq!(
            definition(&ebnf("rule = \"a\" \"b\" \"c\" ;\n")),
            sequence(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_parse_a_choice() {
        assert_eq!(
            definition(&ebnf("rule = \"a\" | \"b\" | \"c\" ;\n")),
            choice(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_parse_an_optional_in_the_w3c_form() {
        assert_eq!(
            definition(&ebnf("rule = \"a\"? ;\n")),
            optional(terminal("a"))
        );
    }

    #[test]
    fn should_parse_an_optional_in_the_iso_form() {
        assert_eq!(
            definition(&ebnf("rule = [ \"a\" ] ;\n")),
            optional(terminal("a"))
        );
    }

    #[test]
    fn should_parse_a_zero_or_more_repetition() {
        let expr = definition(&ebnf("rule = \"a\"* ;\n"));
        assert_eq!(min_of(&expr), 0.0);
        assert_eq!(expr, repeat(terminal("a"), 0.0, INF));
    }

    #[test]
    fn should_parse_a_one_or_more_repetition() {
        let expr = definition(&ebnf("rule = \"a\"+ ;\n"));
        assert_eq!(min_of(&expr), 1.0);
        assert_eq!(expr, repeat(terminal("a"), 1.0, INF));
    }

    #[test]
    fn should_parse_the_iso_repetition() {
        let expr = definition(&ebnf("rule = { \"a\" } ;\n"));
        assert_eq!(min_of(&expr), 0.0);
        assert_eq!(expr, repeat(terminal("a"), 0.0, INF));
    }

    #[test]
    fn should_parse_a_title() {
        assert_eq!(
            parsed(&ebnf("title \"Test Grammar\"\nrule = \"a\" ;\n")).title,
            "Test Grammar"
        );
    }

    #[test]
    fn should_parse_a_special_sequence() {
        assert_eq!(
            definition(&ebnf("rule = ? special ? ;\n")),
            special("special")
        );
    }

    #[test]
    fn should_parse_an_expression_grammar() {
        let text = ebnf(concat!(
            "title \"Simple Expression\"\n",
            "expression = term ( \"+\" term | \"-\" term )* ;\n",
            "term = factor ( \"*\" factor | \"/\" factor )* ;\n",
            "factor = number | \"(\" expression \")\" ;\n",
            "number = digit+ ;\n",
            "digit = \"0\" | \"1\" | \"2\" | \"3\" | \"4\" | \"5\" | \"6\" | \"7\" | \"8\" | \"9\" ;\n",
        ));
        let diagram = parsed(&text);
        assert_eq!(diagram.rules.len(), 5);
        assert_eq!(diagram.title, "Simple Expression");
    }

    #[test]
    fn should_reject_a_text_without_the_keyword() {
        rejected("rule = \"a\" ;\n");
    }

    #[test]
    fn should_reject_a_missing_semicolon() {
        rejected("railroad-ebnf-beta\nrule = \"a\"\n");
    }
}

mod abnf {
    use super::*;

    const HEAD: &str = "railroad-abnf-beta\n";

    fn abnf(body: &str) -> String {
        format!("{HEAD}{body}")
    }

    #[test]
    fn should_parse_a_string_literal() {
        let all = rules(&abnf("rule = \"hello\" ;\n"));
        assert_eq!(all[0].name, "rule");
        assert_eq!(all[0].definition, terminal("hello"));
    }

    #[test]
    fn should_keep_a_numeric_value_as_written() {
        assert_eq!(definition(&abnf("rule = %x41 ;\n")), terminal("%x41"));
    }

    #[test]
    fn should_parse_a_rule_name_reference() {
        assert_eq!(
            definition(&abnf("rule = other-rule ;\n")),
            nonterminal("other-rule")
        );
    }

    #[test]
    fn should_parse_a_concatenation() {
        assert_eq!(
            definition(&abnf("rule = \"a\" \"b\" \"c\" ;\n")),
            sequence(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_parse_an_alternation() {
        assert_eq!(
            definition(&abnf("rule = \"a\" / \"b\" / \"c\" ;\n")),
            choice(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_parse_an_optional_group() {
        assert_eq!(
            definition(&abnf("rule = [ \"a\" ] ;\n")),
            optional(terminal("a"))
        );
    }

    #[test]
    fn should_parse_a_zero_or_more_repetition() {
        assert_eq!(
            definition(&abnf("rule = *\"a\" ;\n")),
            repeat(terminal("a"), 0.0, INF)
        );
    }

    #[test]
    fn should_parse_a_one_or_more_repetition() {
        assert_eq!(
            definition(&abnf("rule = 1*\"a\" ;\n")),
            repeat(terminal("a"), 1.0, INF)
        );
    }

    #[test]
    fn should_parse_a_title() {
        assert_eq!(
            parsed(&abnf("title \"Test Grammar\"\nrule = \"a\" ;\n")).title,
            "Test Grammar"
        );
    }

    #[test]
    fn should_parse_a_uri_like_grammar() {
        let text = abnf(concat!(
            "title \"URI\"\n",
            "URI = scheme \":\" hier-part ;\n",
            "scheme = ALPHA *( ALPHA / DIGIT / \"+\" / \"-\" / \".\" ) ;\n",
            "hier-part = \"//\" authority path-abempty ;\n",
        ));
        let diagram = parsed(&text);
        let names: Vec<&str> = diagram
            .rules
            .iter()
            .map(|rule| rule.name.as_str())
            .collect();
        assert_eq!(names, ["URI", "scheme", "hier-part"]);
    }

    #[test]
    fn should_reject_a_text_without_the_keyword() {
        rejected("rule = \"a\" ;\n");
    }

    #[test]
    fn should_reject_a_missing_semicolon() {
        rejected("railroad-abnf-beta\nrule = \"a\"\n");
    }
}

mod peg {
    use super::*;

    const HEAD: &str = "railroad-peg-beta\n";

    fn peg(body: &str) -> String {
        format!("{HEAD}{body}")
    }

    #[test]
    fn should_parse_a_literal() {
        let all = rules(&peg("rule <- \"hello\" ;\n"));
        assert_eq!(all[0].name, "rule");
        assert_eq!(all[0].definition, terminal("hello"));
    }

    #[test]
    fn should_parse_an_identifier_reference() {
        assert_eq!(
            definition(&peg("rule <- other_rule ;\n")),
            nonterminal("other_rule")
        );
    }

    #[test]
    fn should_parse_a_sequence() {
        assert_eq!(
            definition(&peg("rule <- \"a\" \"b\" \"c\" ;\n")),
            sequence(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_parse_an_ordered_choice() {
        assert_eq!(
            definition(&peg("rule <- \"a\" / \"b\" / \"c\" ;\n")),
            choice(vec![terminal("a"), terminal("b"), terminal("c")])
        );
    }

    #[test]
    fn should_parse_the_optional_suffix() {
        assert_eq!(
            definition(&peg("rule <- \"a\"? ;\n")),
            optional(terminal("a"))
        );
    }

    #[test]
    fn should_parse_the_zero_or_more_suffix() {
        assert_eq!(
            definition(&peg("rule <- \"a\"* ;\n")),
            repeat(terminal("a"), 0.0, INF)
        );
    }

    #[test]
    fn should_parse_the_one_or_more_suffix() {
        assert_eq!(
            definition(&peg("rule <- \"a\"+ ;\n")),
            repeat(terminal("a"), 1.0, INF)
        );
    }

    #[test]
    fn should_parse_any_character_as_a_special() {
        assert_eq!(definition(&peg("rule <- . ;\n")), special("."));
    }

    #[test]
    fn should_parse_the_and_lookahead_prefix() {
        assert_eq!(definition(&peg("rule <- &\"a\" ;\n")), special("&\"a\""));
    }

    #[test]
    fn should_parse_the_not_lookahead_prefix() {
        assert_eq!(definition(&peg("rule <- !\"a\" ;\n")), special("!\"a\""));
    }

    #[test]
    fn should_parse_a_title() {
        assert_eq!(
            parsed(&peg("title \"Test Grammar\"\nrule <- \"a\" ;\n")).title,
            "Test Grammar"
        );
    }

    #[test]
    fn should_parse_an_expression_grammar() {
        let text = peg(concat!(
            "title \"Calculator\"\n",
            "Expression <- Term ((\"+\" / \"-\") Term)* ;\n",
            "Term <- Factor ((\"*\" / \"/\") Factor)* ;\n",
            "Factor <- Number / \"(\" Expression \")\" ;\n",
            "Number <- Digit+ ;\n",
            "Digit <- \"0\" / \"1\" / \"2\" / \"3\" / \"4\" / \"5\" / \"6\" / \"7\" / \"8\" / \"9\" ;\n",
        ));
        let diagram = parsed(&text);
        assert_eq!(diagram.rules.len(), 5);
        assert_eq!(diagram.title, "Calculator");
        assert_eq!(diagram.rules[0].name, "Expression");
    }

    #[test]
    fn should_reject_a_text_without_the_keyword() {
        rejected("rule <- \"a\" ;\n");
    }

    #[test]
    fn should_reject_a_missing_semicolon() {
        rejected("railroad-peg-beta\nrule <- \"a\"\n");
    }
}
