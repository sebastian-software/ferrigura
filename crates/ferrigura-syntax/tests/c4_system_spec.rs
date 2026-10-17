//! Port of `packages/mermaid/src/diagrams/c4/parser/c4System.spec.ts`: seven cases for each of
//! the six system kinds, one `#[test]` per case and kind (42 tests). The plan is
//! `docs-port/c4/PLAN.md`.
//!
//! The first case of each kind compares the whole shape, as the spec's `toEqual` does.

use ferrigura_model::c4::C4Text;
use ferrigura_syntax::c4::parse;
use serde_json::json;

const DESCRIPTION: &str =
    "Allows customers to view information about their bank accounts, and make payments.";

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

// c4System.spec.ts:23
#[test]
fn system_should_parse_a_c4_diagram_with_one_system_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title System Context diagram for Internet Banking System
System(SystemAA, "Internet Banking System", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "SystemAA",
            "label": "Internet Banking System",
            "descr": DESCRIPTION,
            "typeC4Shape": "system",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4System.spec.ts:55
#[test]
fn system_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
System(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "SystemAA");
}

// c4System.spec.ts:64
#[test]
fn system_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
System(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking System")
    );
}

// c4System.spec.ts:75
#[test]
fn system_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
System(SystemAA, "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4System.spec.ts:86
#[test]
fn system_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
System(SystemAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4System.spec.ts:99
#[test]
fn system_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
System(SystemAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4System.spec.ts:112
#[test]
fn system_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
System(SystemAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4System.spec.ts:23
#[test]
fn system_db_should_parse_a_c4_diagram_with_one_system_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title System Context diagram for Internet Banking System
SystemDb(SystemAA, "Internet Banking System", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "SystemAA",
            "label": "Internet Banking System",
            "descr": DESCRIPTION,
            "typeC4Shape": "system_db",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4System.spec.ts:55
#[test]
fn system_db_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
SystemDb(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "SystemAA");
}

// c4System.spec.ts:64
#[test]
fn system_db_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
SystemDb(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking System")
    );
}

// c4System.spec.ts:75
#[test]
fn system_db_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
SystemDb(SystemAA, "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4System.spec.ts:86
#[test]
fn system_db_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
SystemDb(SystemAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4System.spec.ts:99
#[test]
fn system_db_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
SystemDb(SystemAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4System.spec.ts:112
#[test]
fn system_db_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
SystemDb(SystemAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4System.spec.ts:23
#[test]
fn system_queue_should_parse_a_c4_diagram_with_one_system_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title System Context diagram for Internet Banking System
SystemQueue(SystemAA, "Internet Banking System", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "SystemAA",
            "label": "Internet Banking System",
            "descr": DESCRIPTION,
            "typeC4Shape": "system_queue",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4System.spec.ts:55
#[test]
fn system_queue_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
SystemQueue(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "SystemAA");
}

// c4System.spec.ts:64
#[test]
fn system_queue_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
SystemQueue(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking System")
    );
}

// c4System.spec.ts:75
#[test]
fn system_queue_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
SystemQueue(SystemAA, "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4System.spec.ts:86
#[test]
fn system_queue_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
SystemQueue(SystemAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4System.spec.ts:99
#[test]
fn system_queue_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
SystemQueue(SystemAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4System.spec.ts:112
#[test]
fn system_queue_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
SystemQueue(SystemAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4System.spec.ts:23
#[test]
fn system_ext_should_parse_a_c4_diagram_with_one_system_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title System Context diagram for Internet Banking System
System_Ext(SystemAA, "Internet Banking System", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "SystemAA",
            "label": "Internet Banking System",
            "descr": DESCRIPTION,
            "typeC4Shape": "external_system",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4System.spec.ts:55
#[test]
fn system_ext_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
System_Ext(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "SystemAA");
}

// c4System.spec.ts:64
#[test]
fn system_ext_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
System_Ext(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking System")
    );
}

// c4System.spec.ts:75
#[test]
fn system_ext_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
System_Ext(SystemAA, "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4System.spec.ts:86
#[test]
fn system_ext_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
System_Ext(SystemAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4System.spec.ts:99
#[test]
fn system_ext_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
System_Ext(SystemAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4System.spec.ts:112
#[test]
fn system_ext_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
System_Ext(SystemAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4System.spec.ts:23
#[test]
fn system_db_ext_should_parse_a_c4_diagram_with_one_system_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title System Context diagram for Internet Banking System
SystemDb_Ext(SystemAA, "Internet Banking System", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "SystemAA",
            "label": "Internet Banking System",
            "descr": DESCRIPTION,
            "typeC4Shape": "external_system_db",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4System.spec.ts:55
#[test]
fn system_db_ext_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
SystemDb_Ext(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "SystemAA");
}

// c4System.spec.ts:64
#[test]
fn system_db_ext_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
SystemDb_Ext(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking System")
    );
}

// c4System.spec.ts:75
#[test]
fn system_db_ext_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
SystemDb_Ext(SystemAA, "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4System.spec.ts:86
#[test]
fn system_db_ext_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
SystemDb_Ext(SystemAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4System.spec.ts:99
#[test]
fn system_db_ext_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
SystemDb_Ext(SystemAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4System.spec.ts:112
#[test]
fn system_db_ext_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
SystemDb_Ext(SystemAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4System.spec.ts:23
#[test]
fn system_queue_ext_should_parse_a_c4_diagram_with_one_system_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title System Context diagram for Internet Banking System
SystemQueue_Ext(SystemAA, "Internet Banking System", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "SystemAA",
            "label": "Internet Banking System",
            "descr": DESCRIPTION,
            "typeC4Shape": "external_system_queue",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4System.spec.ts:55
#[test]
fn system_queue_ext_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
SystemQueue_Ext(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "SystemAA");
}

// c4System.spec.ts:64
#[test]
fn system_queue_ext_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
SystemQueue_Ext(SystemAA, "Internet Banking System")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking System")
    );
}

// c4System.spec.ts:75
#[test]
fn system_queue_ext_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
SystemQueue_Ext(SystemAA, "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4System.spec.ts:86
#[test]
fn system_queue_ext_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
SystemQueue_Ext(SystemAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4System.spec.ts:99
#[test]
fn system_queue_ext_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
SystemQueue_Ext(SystemAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4System.spec.ts:112
#[test]
fn system_queue_ext_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
SystemQueue_Ext(SystemAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}
