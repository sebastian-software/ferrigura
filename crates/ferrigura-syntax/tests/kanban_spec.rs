//! Port of `packages/mermaid/src/diagrams/kanban/kanban.spec.ts`, the group `when parsing a kanban`.
//! Test names follow the spec's titles in snake_case.
//!
//! The reference reads the children of a section through `getData`, which is the renderer's input.
//! Here a child is a node whose `parent_id` is the section's id, which is the filter `getData`
//! applies (`kanbanDb.ts:73`). `getData` itself is not ported. The reference's expected error (a
//! throw with a message) becomes a rejection at the position where the grammar raises it, as
//! `docs-port/kanban/PLAN.md` decides.

use ferrigura_model::kanban::{KanbanDiagram, KanbanNode};
use ferrigura_syntax::kanban::parse;

/// The sections of a diagram, in order.
fn sections(diagram: &KanbanDiagram) -> Vec<&KanbanNode> {
    diagram
        .nodes
        .iter()
        .filter(|node| node.is_section)
        .collect()
}

/// The children of a section: the nodes whose parent is the section's id, in order.
fn children<'a>(diagram: &'a KanbanDiagram, section: &KanbanNode) -> Vec<&'a KanbanNode> {
    diagram
        .nodes
        .iter()
        .filter(|node| node.parent_id.as_deref() == Some(section.id.as_str()))
        .collect()
}

/// A label as text. Every label of these tests is a text.
fn label_of(node: &KanbanNode) -> &str {
    node.label.as_str().expect("the label is a text")
}

#[test]
fn knbn_1_should_handle_a_simple_root_definition_abc122() {
    let diagram = parse("kanban\n    root").expect("parses");
    let sections = sections(&diagram);
    assert_eq!(sections.len(), 1);
    assert_eq!(label_of(sections[0]), "root");
}

#[test]
fn knbn_2_should_handle_a_hierarchical_kanban_definition() {
    let diagram = parse("kanban\n    root\n      child1\n      child2\n ").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections.len(), 1);
    assert_eq!(label_of(sections[0]), "root");
    assert_eq!(children.len(), 2);
    assert_eq!(label_of(children[0]), "child1");
    assert_eq!(label_of(children[1]), "child2");
}

#[test]
fn _3_should_handle_a_simple_root_definition_with_a_shape_and_without_an_id_abc123() {
    let diagram = parse("kanban\n    (root)").expect("parses");
    let sections = sections(&diagram);
    assert_eq!(label_of(sections[0]), "root");
}

#[test]
fn knbn_4_should_not_distinguish_between_deeper_hierarchical_levels_in_the_kanban_definition() {
    // The reference is less picky than a level check: the deeper node is a child of its section.
    let diagram =
        parse("kanban\n    root\n      child1\n        leaf1\n      child2").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections.len(), 1);
    assert_eq!(children.len(), 3);
}

#[test]
fn _5_multiple_sections_are_ok() {
    let diagram = parse("kanban\n    section1\n    section2").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections.len(), 2);
    assert_eq!(label_of(sections[0]), "section1");
    assert_eq!(label_of(sections[1]), "section2");
}

#[test]
fn knbn_6_real_root_in_wrong_place() {
    // The third line is the first one the DB refuses: `fakeRoot` is a child, and the node at the
    // level of the first node is not a section after it. The error is raised at `realRootWrongPlace`.
    let error = parse("kanban\n          root\n        fakeRoot\n    realRootWrongPlace")
        .expect_err("rejects");
    assert_eq!(error.line, 4);
}

#[test]
fn knbn_7_should_handle_an_id_and_type_for_a_node_definition() {
    let diagram = parse("kanban\n    root[The root]\n      ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "The root");
}

#[test]
fn knbn_8_should_handle_an_id_and_type_for_a_node_definition() {
    let diagram = parse("kanban\n    root\n      theId(child1)").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(label_of(sections[0]), "root");
    assert_eq!(children.len(), 1);
    assert_eq!(label_of(children[0]), "child1");
    assert_eq!(children[0].id, "theId");
}

#[test]
fn knbn_9_should_handle_an_id_and_type_for_a_node_definition() {
    let diagram = parse("kanban\nroot\n      theId(child1)").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(label_of(sections[0]), "root");
    assert_eq!(children.len(), 1);
    assert_eq!(label_of(children[0]), "child1");
    assert_eq!(children[0].id, "theId");
}

#[test]
fn knbn_13_should_be_possible_to_set_an_icon_for_the_node() {
    let diagram = parse("kanban\n    root[The root]\n    ::icon(bomb)\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "The root");
    assert_eq!(sections[0].icon.as_deref(), Some("bomb"));
}

#[test]
fn knbn_14_should_be_possible_to_set_classes_for_the_node() {
    let diagram = parse("kanban\n    root[The root]\n    :::m-4 p-8\n    ").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "The root");
    assert_eq!(sections[0].css_classes.as_deref(), Some("m-4 p-8"));
}

#[test]
fn knbn_15_should_be_possible_to_set_both_classes_and_icon_for_the_node() {
    let diagram = parse("kanban\n    root[The root]\n    :::m-4 p-8\n    ::icon(bomb)\n    ")
        .expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "The root");
    assert_eq!(sections[0].css_classes.as_deref(), Some("m-4 p-8"));
    assert_eq!(sections[0].icon.as_deref(), Some("bomb"));
}

#[test]
fn knbn_16_should_be_possible_to_set_both_classes_and_icon_for_the_node() {
    let diagram = parse("kanban\n    root[The root]\n    ::icon(bomb)\n    :::m-4 p-8\n    ")
        .expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "The root");
    assert_eq!(sections[0].css_classes.as_deref(), Some("m-4 p-8"));
    assert_eq!(sections[0].icon.as_deref(), Some("bomb"));
}

#[test]
fn knbn_17_should_be_possible_to_use_node_syntax_in_the_descriptions() {
    let diagram = parse("kanban\n    root[\"String containing []\"]\n").expect("parses");
    let sections = sections(&diagram);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "String containing []");
}

#[test]
fn knbn_18_should_be_possible_to_use_node_syntax_in_the_descriptions_in_children() {
    let diagram = parse(
        "kanban\n    root[\"String containing []\"]\n      child1[\"String containing ()\"]\n",
    )
    .expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "String containing []");
    assert_eq!(children.len(), 1);
    assert_eq!(label_of(children[0]), "String containing ()");
}

#[test]
fn knbn_19_should_be_possible_to_have_a_child_after_a_class_assignment() {
    let diagram =
        parse("kanban\n  root(Root)\n    Child(Child)\n    :::hot\n      a(a)\n      b[New Stuff]")
            .expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "Root");
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].id, "Child");
    assert_eq!(children[1].id, "a");
    assert_eq!(children[2].id, "b");
}

#[test]
fn knbn_20_should_be_possible_to_have_meaningless_empty_rows_in_a_kanban_abc124() {
    let diagram = parse("kanban\n  root(Root)\n    Child(Child)\n      a(a)\n\n      b[New Stuff]")
        .expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "Root");
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].id, "Child");
    assert_eq!(children[1].id, "a");
    assert_eq!(children[2].id, "b");
}

#[test]
fn knbn_21_should_be_possible_to_have_comments_in_a_kanban() {
    let diagram = parse(
        "kanban\n  root(Root)\n    Child(Child)\n      a(a)\n\n      %% This is a comment\n      b[New Stuff]",
    )
    .expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "Root");
    assert_eq!(children[0].id, "Child");
    assert_eq!(children[1].id, "a");
    assert_eq!(children[2].id, "b");
    assert_eq!(children.len(), 3);
}

#[test]
fn knbn_22_should_be_possible_to_have_comments_at_the_end_of_a_line() {
    let diagram = parse(
        "kanban\n  root(Root)\n    Child(Child)\n      a(a) %% This is a comment\n      b[New Stuff]",
    )
    .expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(label_of(sections[0]), "Root");
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].id, "Child");
    assert_eq!(children[1].id, "a");
    assert_eq!(children[2].id, "b");
}

#[test]
fn knbn_23_rows_with_only_spaces_should_not_interfere() {
    let diagram = parse("kanban\nroot\n A\n \n\n B").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].id, "A");
    assert_eq!(children[1].id, "B");
}

#[test]
fn knbn_24_handle_rows_above_the_kanban_declarations() {
    let diagram = parse("\n \nkanban\nroot\n A\n \n\n B").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].id, "A");
    assert_eq!(children[1].id, "B");
}

#[test]
fn knbn_25_handle_rows_above_the_kanban_declarations_no_space() {
    let diagram = parse("\n\n\nkanban\nroot\n A\n \n\n B").expect("parses");
    let sections = sections(&diagram);
    let children = children(&diagram, sections[0]);

    assert_eq!(sections[0].id, "root");
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].id, "A");
    assert_eq!(children[1].id, "B");
}
