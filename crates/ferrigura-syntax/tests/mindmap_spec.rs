//! Port of `packages/mermaid/src/diagrams/mindmap/mindmap.spec.ts`. Test names follow the spec's
//! titles in snake_case, with the parenthesised part of a title written as words.
//!
//! The reference drives the Jison parser and reads its DB. Here `parse` reads the same text (the
//! reference's `Diagram.fromText` steps are inside it) and the DB state is read from the model: the
//! root is `MindmapDiagram::root`, its children are the DB's children, and `nodeType` is the number
//! the model holds. The reference's `toThrow` becomes an `Err` whose message is the thrown text.

use ferrigura_model::mindmap::{MindmapDiagram, MindmapNode};
use ferrigura_syntax::mindmap::parse;

/// `nodeType` of the DB, as the model holds it.
const ROUNDED_RECT: u8 = 1;
const RECT: u8 = 2;
const CIRCLE: u8 = 3;
const CLOUD: u8 = 4;
const BANG: u8 = 5;
const HEXAGON: u8 = 6;

fn parses(text: &str) -> MindmapDiagram {
    parse(text).expect("parses")
}

fn root(diagram: &MindmapDiagram) -> &MindmapNode {
    diagram.root.as_ref().expect("has a root")
}

fn error_of(text: &str) -> String {
    parse(text).expect_err("is rejected").message
}

#[test]
fn mmp_1_should_handle_a_simple_root_definition_abc122() {
    let diagram = parses("mindmap\n    root");
    assert_eq!(root(&diagram).descr, "root");
}

#[test]
fn mmp_2_should_handle_a_hierarchical_mindmap_definition() {
    let diagram = parses("mindmap\n    root\n      child1\n      child2\n ");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "root");
    assert_eq!(mm.children.len(), 2);
    assert_eq!(mm.children[0].descr, "child1");
    assert_eq!(mm.children[1].descr, "child2");
}

#[test]
fn _3_should_handle_a_simple_root_definition_with_a_shape_and_without_an_id_abc123() {
    let diagram = parses("mindmap\n    (root)");
    assert_eq!(root(&diagram).descr, "root");
}

#[test]
fn mmp_4_should_handle_a_deeper_hierarchical_mindmap_definition() {
    let diagram = parses("mindmap\n    root\n      child1\n        leaf1\n      child2");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "root");
    assert_eq!(mm.children.len(), 2);
    assert_eq!(mm.children[0].descr, "child1");
    assert_eq!(mm.children[0].children[0].descr, "leaf1");
    assert_eq!(mm.children[1].descr, "child2");
}

#[test]
fn _5_multiple_roots_are_illegal() {
    assert_eq!(
        error_of("mindmap\n    root\n    fakeRoot"),
        "There can be only one root. No parent could be found for (\"fakeRoot\")"
    );
}

#[test]
fn mmp_6_real_root_in_wrong_place() {
    assert_eq!(
        error_of("mindmap\n          root\n        fakeRoot\n    realRootWrongPlace"),
        "There can be only one root. No parent could be found for (\"fakeRoot\")"
    );
}

#[test]
fn mmp_7_should_handle_an_id_and_type_for_a_node_definition() {
    let diagram = parses("mindmap\n    root[The root]\n      ");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "The root");
    assert_eq!(mm.node_type, RECT);
}

#[test]
fn mmp_8_should_handle_an_id_and_type_for_a_node_definition() {
    let diagram = parses("mindmap\n    root\n      theId(child1)");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "root");
    assert_eq!(mm.children.len(), 1);
    let child = &mm.children[0];
    assert_eq!(child.descr, "child1");
    assert_eq!(child.node_id, "theId");
    assert_eq!(child.node_type, ROUNDED_RECT);
}

#[test]
fn mmp_9_should_handle_an_id_and_type_for_a_node_definition() {
    let diagram = parses("mindmap\nroot\n      theId(child1)");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "root");
    assert_eq!(mm.children.len(), 1);
    let child = &mm.children[0];
    assert_eq!(child.descr, "child1");
    assert_eq!(child.node_id, "theId");
    assert_eq!(child.node_type, ROUNDED_RECT);
}

#[test]
fn mmp_10_multiple_types_circle() {
    let diagram = parses("mindmap\n root((the root))\n ");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "the root");
    assert!(mm.children.is_empty());
    assert_eq!(mm.node_type, CIRCLE);
}

#[test]
fn mmp_11_multiple_types_cloud() {
    let diagram = parses("mindmap\n root)the root(\n");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "the root");
    assert!(mm.children.is_empty());
    assert_eq!(mm.node_type, CLOUD);
}

#[test]
fn mmp_12_multiple_types_bang() {
    let diagram = parses("mindmap\n root))the root((\n");
    let mm = root(&diagram);
    assert_eq!(mm.descr, "the root");
    assert!(mm.children.is_empty());
    assert_eq!(mm.node_type, BANG);
}

#[test]
fn mmp_12_a_multiple_types_hexagon() {
    let diagram = parses("mindmap\n root{{the root}}\n");
    let mm = root(&diagram);
    assert_eq!(mm.node_type, HEXAGON);
    assert_eq!(mm.descr, "the root");
    assert!(mm.children.is_empty());
}

#[test]
fn mmp_13_should_be_possible_to_set_an_icon_for_the_node() {
    let diagram = parses("mindmap\n    root[The root]\n    ::icon(bomb)\n    ");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "The root");
    assert_eq!(mm.node_type, RECT);
    assert_eq!(mm.icon, "bomb");
}

#[test]
fn mmp_14_should_be_possible_to_set_classes_for_the_node() {
    let diagram = parses("mindmap\n    root[The root]\n    :::m-4 p-8\n    ");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "The root");
    assert_eq!(mm.node_type, RECT);
    assert_eq!(mm.class, "m-4 p-8");
}

#[test]
fn mmp_15_should_be_possible_to_set_both_classes_and_icon_for_the_node() {
    let diagram = parses("mindmap\n    root[The root]\n    :::m-4 p-8\n    ::icon(bomb)\n    ");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "The root");
    assert_eq!(mm.node_type, RECT);
    assert_eq!(mm.class, "m-4 p-8");
    assert_eq!(mm.icon, "bomb");
}

#[test]
fn mmp_16_should_be_possible_to_set_both_classes_and_icon_for_the_node() {
    let diagram = parses("mindmap\n    root[The root]\n    ::icon(bomb)\n    :::m-4 p-8\n    ");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "The root");
    assert_eq!(mm.node_type, RECT);
    assert_eq!(mm.class, "m-4 p-8");
    assert_eq!(mm.icon, "bomb");
}

#[test]
fn mmp_17_should_be_possible_to_use_node_syntax_in_the_descriptions() {
    let diagram = parses("mindmap\n    root[\"String containing []\"]\n");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "String containing []");
}

#[test]
fn mmp_18_should_be_possible_to_use_node_syntax_in_the_descriptions_in_children() {
    let diagram = parses(
        "mindmap\n    root[\"String containing []\"]\n      child1[\"String containing ()\"]\n",
    );
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "String containing []");
    assert_eq!(mm.children.len(), 1);
    assert_eq!(mm.children[0].descr, "String containing ()");
}

#[test]
fn mmp_19_should_be_possible_to_have_a_child_after_a_class_assignment() {
    let diagram = parses(
        "mindmap\n  root(Root)\n    Child(Child)\n    :::hot\n      a(a)\n      b[New Stuff]",
    );
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "Root");
    assert_eq!(mm.children.len(), 1);
    let child = &mm.children[0];
    assert_eq!(child.node_id, "Child");
    assert_eq!(child.children[0].node_id, "a");
    assert_eq!(child.children.len(), 2);
    assert_eq!(child.children[1].node_id, "b");
}

#[test]
fn mmp_20_should_be_possible_to_have_meaningless_empty_rows_in_a_mindmap_abc124() {
    let diagram =
        parses("mindmap\n  root(Root)\n    Child(Child)\n      a(a)\n\n      b[New Stuff]");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "Root");
    assert_eq!(mm.children.len(), 1);
    let child = &mm.children[0];
    assert_eq!(child.node_id, "Child");
    assert_eq!(child.children[0].node_id, "a");
    assert_eq!(child.children.len(), 2);
    assert_eq!(child.children[1].node_id, "b");
}

#[test]
fn mmp_21_should_be_possible_to_have_comments_in_a_mindmap() {
    let diagram = parses(
        "mindmap\n  root(Root)\n    Child(Child)\n      a(a)\n\n      %% This is a comment\n      b[New Stuff]",
    );
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "Root");
    assert_eq!(mm.children.len(), 1);
    let child = &mm.children[0];
    assert_eq!(child.node_id, "Child");
    assert_eq!(child.children[0].node_id, "a");
    assert_eq!(child.children.len(), 2);
    assert_eq!(child.children[1].node_id, "b");
}

#[test]
fn mmp_22_should_be_possible_to_have_comments_at_the_end_of_a_line() {
    let diagram = parses(
        "mindmap\n  root(Root)\n    Child(Child)\n      a(a) %% This is a comment\n      b[New Stuff]",
    );
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.descr, "Root");
    assert_eq!(mm.children.len(), 1);
    let child = &mm.children[0];
    assert_eq!(child.node_id, "Child");
    assert_eq!(child.children[0].node_id, "a");
    assert_eq!(child.children.len(), 2);
    assert_eq!(child.children[1].node_id, "b");
}

#[test]
fn mmp_23_rows_with_only_spaces_should_not_interfere() {
    let diagram = parses("mindmap\nroot\n A\n \n\n B");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.children.len(), 2);
    assert_eq!(mm.children[0].node_id, "A");
    assert_eq!(mm.children[1].node_id, "B");
}

#[test]
fn mmp_24_handle_rows_above_the_mindmap_declarations() {
    let diagram = parses("\n \nmindmap\nroot\n A\n \n\n B");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.children.len(), 2);
    assert_eq!(mm.children[0].node_id, "A");
    assert_eq!(mm.children[1].node_id, "B");
}

#[test]
fn mmp_25_handle_rows_above_the_mindmap_declarations_no_space() {
    let diagram = parses("\n\n\nmindmap\nroot\n A\n \n\n B");
    let mm = root(&diagram);
    assert_eq!(mm.node_id, "root");
    assert_eq!(mm.children.len(), 2);
    assert_eq!(mm.children[0].node_id, "A");
    assert_eq!(mm.children[1].node_id, "B");
}
