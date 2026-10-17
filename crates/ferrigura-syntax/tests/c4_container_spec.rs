//! Port of `packages/mermaid/src/diagrams/c4/parser/c4Container.spec.ts`: eight cases for each
//! of the six container kinds, one `#[test]` per case and kind (48 tests). The plan is
//! `docs-port/c4/PLAN.md`.
//!
//! The first case of each kind compares the whole shape, as the spec's `toEqual` does. The JSON
//! form of the model leaves out the absent fields, which the reference holds as `undefined`.

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

// c4Container.spec.ts:23
#[test]
fn container_should_parse_a_c4_diagram_with_one_container_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title Container diagram for Internet Banking Container
Container(ContainerAA, "Internet Banking Container", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "ContainerAA",
            "label": "Internet Banking Container",
            "descr": DESCRIPTION,
            "techn": "Technology",
            "typeC4Shape": "container",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Container.spec.ts:56
#[test]
fn container_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
Container(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "ContainerAA");
}

// c4Container.spec.ts:65
#[test]
fn container_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
Container(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking Container")
    );
}

// c4Container.spec.ts:76
#[test]
fn container_should_parse_the_technology() {
    let diagram = parse(
        r#"C4Context
Container(ContainerAA, "", "Java")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].techn.as_deref(), Some("Java"));
}

// c4Container.spec.ts:87
#[test]
fn container_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
Container(ContainerAA, "", "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4Container.spec.ts:98
#[test]
fn container_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
Container(ContainerAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Container.spec.ts:111
#[test]
fn container_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
Container(ContainerAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Container.spec.ts:124
#[test]
fn container_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
Container(ContainerAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Container.spec.ts:23
#[test]
fn container_db_should_parse_a_c4_diagram_with_one_container_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title Container diagram for Internet Banking Container
ContainerDb(ContainerAA, "Internet Banking Container", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "ContainerAA",
            "label": "Internet Banking Container",
            "descr": DESCRIPTION,
            "techn": "Technology",
            "typeC4Shape": "container_db",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Container.spec.ts:56
#[test]
fn container_db_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
ContainerDb(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "ContainerAA");
}

// c4Container.spec.ts:65
#[test]
fn container_db_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
ContainerDb(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking Container")
    );
}

// c4Container.spec.ts:76
#[test]
fn container_db_should_parse_the_technology() {
    let diagram = parse(
        r#"C4Context
ContainerDb(ContainerAA, "", "Java")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].techn.as_deref(), Some("Java"));
}

// c4Container.spec.ts:87
#[test]
fn container_db_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
ContainerDb(ContainerAA, "", "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4Container.spec.ts:98
#[test]
fn container_db_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
ContainerDb(ContainerAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Container.spec.ts:111
#[test]
fn container_db_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
ContainerDb(ContainerAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Container.spec.ts:124
#[test]
fn container_db_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
ContainerDb(ContainerAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Container.spec.ts:23
#[test]
fn container_queue_should_parse_a_c4_diagram_with_one_container_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title Container diagram for Internet Banking Container
ContainerQueue(ContainerAA, "Internet Banking Container", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "ContainerAA",
            "label": "Internet Banking Container",
            "descr": DESCRIPTION,
            "techn": "Technology",
            "typeC4Shape": "container_queue",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Container.spec.ts:56
#[test]
fn container_queue_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
ContainerQueue(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "ContainerAA");
}

// c4Container.spec.ts:65
#[test]
fn container_queue_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
ContainerQueue(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking Container")
    );
}

// c4Container.spec.ts:76
#[test]
fn container_queue_should_parse_the_technology() {
    let diagram = parse(
        r#"C4Context
ContainerQueue(ContainerAA, "", "Java")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].techn.as_deref(), Some("Java"));
}

// c4Container.spec.ts:87
#[test]
fn container_queue_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
ContainerQueue(ContainerAA, "", "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4Container.spec.ts:98
#[test]
fn container_queue_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
ContainerQueue(ContainerAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Container.spec.ts:111
#[test]
fn container_queue_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
ContainerQueue(ContainerAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Container.spec.ts:124
#[test]
fn container_queue_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
ContainerQueue(ContainerAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Container.spec.ts:23
#[test]
fn container_ext_should_parse_a_c4_diagram_with_one_container_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title Container diagram for Internet Banking Container
Container_Ext(ContainerAA, "Internet Banking Container", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "ContainerAA",
            "label": "Internet Banking Container",
            "descr": DESCRIPTION,
            "techn": "Technology",
            "typeC4Shape": "external_container",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Container.spec.ts:56
#[test]
fn container_ext_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
Container_Ext(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "ContainerAA");
}

// c4Container.spec.ts:65
#[test]
fn container_ext_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
Container_Ext(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking Container")
    );
}

// c4Container.spec.ts:76
#[test]
fn container_ext_should_parse_the_technology() {
    let diagram = parse(
        r#"C4Context
Container_Ext(ContainerAA, "", "Java")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].techn.as_deref(), Some("Java"));
}

// c4Container.spec.ts:87
#[test]
fn container_ext_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
Container_Ext(ContainerAA, "", "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4Container.spec.ts:98
#[test]
fn container_ext_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
Container_Ext(ContainerAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Container.spec.ts:111
#[test]
fn container_ext_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
Container_Ext(ContainerAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Container.spec.ts:124
#[test]
fn container_ext_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
Container_Ext(ContainerAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Container.spec.ts:23
#[test]
fn container_db_ext_should_parse_a_c4_diagram_with_one_container_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title Container diagram for Internet Banking Container
ContainerDb_Ext(ContainerAA, "Internet Banking Container", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "ContainerAA",
            "label": "Internet Banking Container",
            "descr": DESCRIPTION,
            "techn": "Technology",
            "typeC4Shape": "external_container_db",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Container.spec.ts:56
#[test]
fn container_db_ext_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
ContainerDb_Ext(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "ContainerAA");
}

// c4Container.spec.ts:65
#[test]
fn container_db_ext_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
ContainerDb_Ext(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking Container")
    );
}

// c4Container.spec.ts:76
#[test]
fn container_db_ext_should_parse_the_technology() {
    let diagram = parse(
        r#"C4Context
ContainerDb_Ext(ContainerAA, "", "Java")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].techn.as_deref(), Some("Java"));
}

// c4Container.spec.ts:87
#[test]
fn container_db_ext_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
ContainerDb_Ext(ContainerAA, "", "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4Container.spec.ts:98
#[test]
fn container_db_ext_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
ContainerDb_Ext(ContainerAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Container.spec.ts:111
#[test]
fn container_db_ext_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
ContainerDb_Ext(ContainerAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Container.spec.ts:124
#[test]
fn container_db_ext_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
ContainerDb_Ext(ContainerAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}

// c4Container.spec.ts:23
#[test]
fn container_queue_ext_should_parse_a_c4_diagram_with_one_container_correctly() {
    let diagram = parse(&format!(
        r#"C4Context
title Container diagram for Internet Banking Container
ContainerQueue_Ext(ContainerAA, "Internet Banking Container", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    assert_eq!(
        serde_json::to_value(&diagram.shapes[0]).expect("serialises"),
        json!({
            "alias": "ContainerAA",
            "label": "Internet Banking Container",
            "descr": DESCRIPTION,
            "techn": "Technology",
            "typeC4Shape": "external_container_queue",
            "parentBoundary": "global",
            "wrap": false
        })
    );
}

// c4Container.spec.ts:56
#[test]
fn container_queue_ext_should_parse_the_alias() {
    let diagram = parse(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].alias, "ContainerAA");
}

// c4Container.spec.ts:65
#[test]
fn container_queue_ext_should_parse_the_label() {
    let diagram = parse(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, "Internet Banking Container")"#,
    )
    .expect("parses");
    assert_eq!(
        diagram.shapes[0].label,
        label_text("Internet Banking Container")
    );
}

// c4Container.spec.ts:76
#[test]
fn container_queue_ext_should_parse_the_technology() {
    let diagram = parse(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, "", "Java")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].techn.as_deref(), Some("Java"));
}

// c4Container.spec.ts:87
#[test]
fn container_queue_ext_should_parse_the_description() {
    let diagram = parse(&format!(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, "", "", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some(DESCRIPTION));
}

// c4Container.spec.ts:98
#[test]
fn container_queue_ext_should_parse_a_sprite() {
    let diagram = parse(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, $sprite="users")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "sprite"),
        Some("users")
    );
}

// c4Container.spec.ts:111
#[test]
fn container_queue_ext_should_parse_a_link() {
    let diagram = parse(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, $link="https://github.com/mermaidjs")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "link"),
        Some("https://github.com/mermaidjs")
    );
}

// c4Container.spec.ts:124
#[test]
fn container_queue_ext_should_parse_tags() {
    let diagram = parse(
        r#"C4Context
ContainerQueue_Ext(ContainerAA, $tags="tag1,tag2")"#,
    )
    .expect("parses");
    assert_eq!(
        named_label(diagram.shapes[0].label.as_ref(), "tags"),
        Some("tag1,tag2")
    );
}
