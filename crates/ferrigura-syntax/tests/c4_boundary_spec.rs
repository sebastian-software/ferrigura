//! Port of `packages/mermaid/src/diagrams/c4/parser/c4Boundary.spec.ts`: seven cases, the rows of
//! its one-row `describe.each` (`Boundary`), one `#[test]` per case (7 tests). The plan is
//! `docs-port/c4/PLAN.md`.
//!
//! The first case compares the whole boundary, as the spec's `toEqual` does. The last case checks
//! the relation and the shapes that the model holds. The spec's `getC4Shape` is not in the public
//! API, so its identity and missing-alias checks are not written here.

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

// c4Boundary.spec.ts:16
#[test]
fn boundary_should_parse_a_c4_diagram_with_one_boundary_correctly() {
    let diagram = parse(
        r#"C4Context
title System Context diagram for Internet Banking System
Boundary(b1, "BankBoundary") {
System(SystemAA, "Internet Banking System")
}"#,
    )
    .expect("parses");
    assert_eq!(diagram.boundaries.len(), 2);
    assert_eq!(
        serde_json::to_value(&diagram.boundaries[1]).expect("serialises"),
        json!({
            "alias": "b1",
            "label": "BankBoundary",
            "type": "system",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Boundary.spec.ts:47
#[test]
fn boundary_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
Boundary(b1, "BankBoundary") {
System(SystemAA, "Internet Banking System")
}"#,
    )
    .expect("parses");
    assert_eq!(diagram.boundaries[1].alias, "b1");
}

// c4Boundary.spec.ts:58
#[test]
fn boundary_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
Boundary(b1, "BankBoundary") {
System(SystemAA, "Internet Banking System")
}"#,
    )
    .expect("parses");
    assert_eq!(diagram.boundaries[1].label, label_text("BankBoundary"));
}

// c4Boundary.spec.ts:71
#[test]
fn boundary_should_parse_the_type() {
    let diagram = parse(
        r#"C4Context
Boundary(b1, "", "company") {
System(SystemAA, "Internet Banking System")
}"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.boundaries[1].boundary_type.as_deref(),
        Some("company")
    );
}

// c4Boundary.spec.ts:82
#[test]
fn boundary_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
Boundary(b1, $link="https://github.com/mermaidjs") {
System(SystemAA, "Internet Banking System")
}"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.boundaries[1].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Boundary.spec.ts:97
#[test]
fn boundary_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
Boundary(b1, $tags="tag1,tag2") {
System(SystemAA, "Internet Banking System")
}"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.boundaries[1].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Boundary.spec.ts:114
#[test]
fn boundary_should_resolve_a_boundary_alias_as_a_relation_endpoint() {
    let diagram = parse(
        r#"C4Context
Boundary(b1, "BankBoundary") {
System(SystemAA, "Internet Banking System")
}
Rel(b1, SystemAA, "Contains")"#,
    )
    .expect("parses");
    // The spec then checks that getC4Shape('b1') is the boundary object, that getC4Shape
    // ('SystemAA') is the system, and that getC4Shape('missing') is undefined. getC4Shape is
    // not in the public API, so this checks what the model holds for the same endpoints.
    assert_eq!(diagram.boundaries[1].alias, "b1");
    let system = diagram
        .shapes
        .iter()
        .find(|shape| shape.alias == "SystemAA")
        .expect("the system is declared");
    assert_eq!(system.type_c4_shape, "system");
    let relation = &diagram.rels[0];
    assert_eq!(relation.from, "b1");
    assert_eq!(relation.to, "SystemAA");
    assert_eq!(relation.label, label_text("Contains"));
}
