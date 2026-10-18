//! The reference's railroad DB specs (`packages/mermaid/src/diagrams/railroad/railroadDb.spec.ts`).
//! The DB has no public surface, so each behaviour is driven through `parse`, as the reference's
//! `Diagram.fromText` drives it: the DB is cleared before each parse, the header texts and the rules
//! are set in the grammar's order, and every text is sanitised as it is stored.

use ferrigura_model::railroad::{RailroadDiagram, RailroadExpr};
use ferrigura_syntax::railroad::parse;

fn parsed(text: &str) -> RailroadDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

fn value_of(text: &str) -> String {
    match &parsed(text).rules[0].definition {
        RailroadExpr::Terminal { value } => value.clone(),
        other => panic!("not a terminal: {other:?}"),
    }
}

#[test]
fn should_store_and_return_rules() {
    let diagram = parsed("railroad-beta\ntest = terminal(\"test\") ;\n");
    assert_eq!(diagram.rules.len(), 1);
    assert_eq!(diagram.rules[0].name, "test");
    assert_eq!(
        value_of("railroad-beta\ntest = terminal(\"test\") ;\n"),
        "test"
    );
}

#[test]
fn should_handle_the_title() {
    assert_eq!(
        parsed("railroad-beta\ntitle Test Title\nr = terminal(\"a\") ;\n").title,
        "Test Title"
    );
}

#[test]
fn should_sanitize_titles_before_storing_them() {
    let title =
        parsed("railroad-beta\ntitle <script>alert(1)</script>Safe Title\nr = terminal(\"a\") ;\n")
            .title;
    assert!(!title.contains("<script>"), "{title:?}");
    assert!(title.contains("Safe Title"), "{title:?}");
}

#[test]
fn should_start_each_parse_from_an_empty_db() {
    let first = parsed("railroad-beta\ntitle Test\nr = terminal(\"a\") ;\n");
    assert_eq!(first.title, "Test");
    let second = parsed("railroad-beta\nr = terminal(\"b\") ;\n");
    assert_eq!(second.title, "");
    assert_eq!(second.rules.len(), 1);
}

#[test]
fn should_keep_duplicate_rules_in_order() {
    let rules =
        parsed("railroad-beta\ntest = terminal(\"first\") ;\ntest = terminal(\"second\") ;\n")
            .rules;
    assert_eq!(rules.len(), 2);
    assert_eq!(
        rules[0].definition,
        RailroadExpr::Terminal {
            value: "first".to_owned()
        }
    );
    assert_eq!(
        rules[1].definition,
        RailroadExpr::Terminal {
            value: "second".to_owned()
        }
    );
}

#[test]
fn should_handle_the_accessibility_title() {
    assert_eq!(
        parsed("railroad-beta\naccTitle: Accessible Title\nr = terminal(\"a\") ;\n").acc_title,
        "Accessible Title"
    );
}

#[test]
fn should_handle_the_accessibility_description() {
    assert_eq!(
        parsed("railroad-beta\naccDescr: Accessible Description\nr = terminal(\"a\") ;\n")
            .acc_descr,
        "Accessible Description"
    );
}

#[test]
fn should_keep_the_accessibility_description_block_lines() {
    let diagram =
        parsed("railroad-beta\naccDescr {\n    line one\n    line two\n}\nr = terminal(\"a\") ;\n");
    assert_eq!(diagram.acc_descr, "line one\nline two");
}

#[test]
fn should_clear_every_header_text_between_parses() {
    let diagram = parsed("railroad-beta\nr = terminal(\"a\") ;\n");
    assert_eq!(diagram.title, "");
    assert_eq!(diagram.acc_title, "");
    assert_eq!(diagram.acc_descr, "");
    assert_eq!(diagram.rules.len(), 1);
}

#[test]
fn should_sanitize_terminal_values_at_the_db_boundary() {
    let value = value_of("railroad-beta\nr = terminal(\"value<script>alert(1)</script>\") ;\n");
    assert!(!value.contains("<script>"), "{value:?}");
    assert!(value.contains("value"), "{value:?}");
}

#[test]
fn should_sanitize_nonterminal_names_at_the_db_boundary() {
    let definition = parsed("railroad-beta\nr = nonterminal(\"AT&T <b>bold</b>\") ;\n").rules[0]
        .definition
        .clone();
    assert_eq!(
        definition,
        RailroadExpr::Nonterminal {
            name: "AT&amp;T <b>bold</b>".to_owned()
        }
    );
}
