//! Port of `packages/mermaid/src/diagrams/c4/parser/c4Person.spec.ts`: eight cases (8 tests). The
//! plan is `docs-port/c4/PLAN.md`.
//!
//! The first case compares the whole shape, as the spec's `toEqual` does. The JSON form of the
//! model leaves out the absent fields, which the reference holds as `undefined`.

use ferrigura_model::c4::C4Text;
use ferrigura_syntax::c4::parse;
use serde_json::json;

/// The value of `key` in a label that holds a named argument, `{ text: { key: value } }`.
fn named_label<'a>(label: Option<&'a C4Text>, key: &str) -> Option<&'a str> {
    match label? {
        C4Text::Named(map) => map.get(key).map(String::as_str),
        C4Text::Plain(_) => None,
    }
}

/// A label that is a plain string, `{ text: "..." }` in the reference.
fn label_text(text: &str) -> Option<C4Text> {
    Some(C4Text::Plain(text.to_owned()))
}

// c4Person.spec.ts:16
#[test]
fn should_parse_a_c4_diagram_with_one_person_correctly() {
    let diagram = parse(
        r#"C4Context
title System Context diagram for Internet Banking System
Person(customerA, "Banking Customer A", "A customer of the bank, with personal bank accounts.")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "customerA",
            "label": "Banking Customer A",
            "descr": "A customer of the bank, with personal bank accounts.",
            "typeC4Shape": "person",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Person.spec.ts:43
#[test]
fn should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
Person(customerA, "Banking Customer A")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "customerA");
}

// c4Person.spec.ts:52
#[test]
fn should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
Person(customerA, "Banking Customer A")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].label, label_text("Banking Customer A"));
}

// c4Person.spec.ts:63
#[test]
fn should_parse_the_description() {
    let diagram = parse(
        r#"C4Context
Person(customerA, "", "A customer of the bank, with personal bank accounts.")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].descr.as_deref(),
        Some("A customer of the bank, with personal bank accounts.")
    );
}

// c4Person.spec.ts:74
#[test]
fn should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
Person(customerA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Person.spec.ts:87
#[test]
fn should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
Person(customerA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Person.spec.ts:100
#[test]
fn should_parse_tags() {
    let diagram = parse(
        r#"C4Context
Person(customerA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Person.spec.ts:113
#[test]
fn should_keep_named_attributes_that_land_in_an_earlier_positional_slot() {
    let diagram = parse(
        "C4Context\n\
         Person(customerA, \"Banking Customer A\", \"A customer.\", \
         $tags=\"v1.0+internal\", $link=\"https://example.com\")",
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "customerA");
    assert_eq!(shape.tags.as_deref(), Some("v1.0+internal"));
    assert_eq!(shape.link.as_deref(), Some("https://example.com"));
}
