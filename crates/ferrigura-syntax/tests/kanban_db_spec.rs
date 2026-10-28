//! Port of `packages/mermaid/src/diagrams/kanban/kanban.spec.ts`, the group `item data data`: the
//! shape data of a node, read from the DB. Test names follow the spec's titles in snake_case.
//!
//! The reference reads these fields through `getSections`, which is the DB and not `getData`, so
//! they are the DB's fields here as well.

use ferrigura_model::kanban::{KanbanDiagram, KanbanNode};
use ferrigura_model::value::Value;
use ferrigura_syntax::kanban::parse;

/// The sections of a diagram, in order.
fn sections(diagram: &KanbanDiagram) -> Vec<&KanbanNode> {
    diagram
        .nodes
        .iter()
        .filter(|node| node.is_section)
        .collect()
}

#[test]
fn knbn_30_should_be_possible_to_set_the_priority() {
    let diagram = parse("kanban\n        root@{ priority: high }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].priority, Some(Value::String("high".to_owned())));
}

#[test]
fn knbn_31_should_be_possible_to_set_the_assignment() {
    let diagram = parse("kanban\n        root@{ assigned: knsv }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].assigned.as_deref(), Some("knsv"));
}

#[test]
fn knbn_32_should_be_possible_to_set_the_icon() {
    let diagram = parse("kanban\n        root@{ icon: star }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].icon.as_deref(), Some("star"));
}

#[test]
fn knbn_33_should_be_possible_to_set_the_icon() {
    let diagram = parse("kanban\n        root@{ icon: star }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].icon.as_deref(), Some("star"));
}

#[test]
fn knbn_34_should_be_possible_to_set_the_metadata_using_multiple_lines() {
    let diagram = parse(
        "kanban\n        root@{\n          icon: star\n          assigned: knsv\n        }\n    ",
    )
    .expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].icon.as_deref(), Some("star"));
    assert_eq!(sections[0].assigned.as_deref(), Some("knsv"));
}

#[test]
fn knbn_35_should_be_possible_to_set_the_metadata_using_one_line() {
    let diagram =
        parse("kanban\n        root@{ icon: star, assigned: knsv }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].icon.as_deref(), Some("star"));
    assert_eq!(sections[0].assigned.as_deref(), Some("knsv"));
}

#[test]
fn knbn_36_should_be_possible_to_set_the_label_using_the_new_syntax() {
    let diagram =
        parse("kanban\n        root@{ icon: star, label: 'fix things' }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].label, Value::String("fix things".to_owned()));
}

#[test]
fn knbn_37_should_be_possible_to_set_the_external_id() {
    let diagram = parse("kanban\n        root@{ ticket: MC-1234 }\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(sections[0].ticket.as_deref(), Some("MC-1234"));
}
