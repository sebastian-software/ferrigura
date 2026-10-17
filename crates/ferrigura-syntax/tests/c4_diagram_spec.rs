//! Port of `packages/mermaid/src/diagrams/c4/parser/c4Diagram.spec.ts`: three cases (3 tests). The
//! plan is `docs-port/c4/PLAN.md`.

use ferrigura_model::c4::C4Text;
use ferrigura_syntax::c4::parse;

const DESCRIPTION: &str = "A customer of the bank, with personal bank accounts.";

/// A label that is a plain string, `{ text: "..." }` in the reference.
fn label_text(text: &str) -> Option<C4Text> {
    Some(C4Text::Plain(text.to_owned()))
}

// c4Diagram.spec.ts:16
#[test]
fn should_handle_a_trailing_whitespaces_after_statements() {
    let ws = " ";
    let text = format!(
        r#"C4Context{ws}
title System Context diagram for Internet Banking System{ws}
Person(customerA, "Banking Customer A", "{DESCRIPTION}"){ws}"#
    );
    parse(&text).expect("parses");
}

// c4Diagram.spec.ts:25
#[test]
fn should_handle_parameter_names_that_are_keywords() {
    let diagram = parse(
        r#"C4Context
title title
Person(Person, "Person", "Person")"#,
    )
    .expect("parses");
    assert_eq!(diagram.title, "title");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "Person");
    assert_eq!(shape.descr.as_deref(), Some("Person"));
    assert_eq!(shape.label, label_text("Person"));
}

// c4Diagram.spec.ts:42
#[test]
fn should_allow_default_in_the_parameters() {
    let diagram = parse(
        r#"C4Context
Person(default, "default", "default")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "default");
    assert_eq!(shape.descr.as_deref(), Some("default"));
    assert_eq!(shape.label, label_text("default"));
}
