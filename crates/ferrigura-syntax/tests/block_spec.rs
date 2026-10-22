//! Port of packages/mermaid/src/diagrams/block/parser/block.spec.ts. Test names follow the spec's
//! titles in snake_case; a title that repeats gets `_2` in source order.
//!
//! The spec reads the DB after each parse (`db.getBlocks()`, `db.getEdges()`, `db.getClasses()`,
//! `db.getColumns('root')`). Here the model is the DB's state, so each test reads the same values
//! from it: the root's blocks, the edges, the class definitions, and the root's columns. The
//! `getColumns` of the reference returns the block's columns, or its number of children when they
//! are `0`; no spec input reaches that case.

use ferrigura_model::block::{Block, BlockDiagram, ClassDef};
use ferrigura_syntax::block::{parse, parse_with_warnings};

fn parsed(text: &str) -> BlockDiagram {
    parse(text).expect("the diagram parses")
}

/// The root's columns, as `db.getColumns('root')` reads them.
fn columns(diagram: &BlockDiagram) -> f64 {
    diagram.columns.unwrap_or(-1.0)
}

/// The children of a block; a block with no children list has none.
fn children(block: &Block) -> &[Block] {
    block.children.as_deref().unwrap_or(&[])
}

fn class_def<'a>(diagram: &'a BlockDiagram, id: &str) -> Option<&'a ClassDef> {
    diagram.class_defs.iter().find(|class| class.id == id)
}

#[test]
fn a_diagram_with_a_node() {
    let diagram = parsed("block-beta\n    id\n");
    assert_eq!(diagram.blocks.len(), 1);
    assert_eq!(diagram.blocks[0].id.as_deref(), Some("id"));
    assert_eq!(diagram.blocks[0].label.as_deref(), Some("id"));
}

#[test]
fn a_node_with_a_square_shape_and_a_label() {
    let diagram = parsed("block\n    id[\"A label\"]\n    ");
    assert_eq!(diagram.blocks.len(), 1);
    assert_eq!(diagram.blocks[0].id.as_deref(), Some("id"));
    assert_eq!(diagram.blocks[0].label.as_deref(), Some("A label"));
    assert_eq!(diagram.blocks[0].block_type.as_deref(), Some("square"));
}

#[test]
#[ignore = "port: the sanitiser runs DOMPurify's default configuration; a dompurifyConfig is not read (docs-port/block/PLAN.md, section 11)"]
fn sanitizes_labels_with_the_current_dompurify_config() {
    let diagram = parsed("block\n    id[\"<b>Bold</b>\"]\n    ");
    assert_eq!(diagram.blocks[0].label.as_deref(), Some("Bold"));
}

#[test]
fn a_diagram_with_multiple_nodes() {
    let diagram = parsed("block\n    id1\n    id2\n");
    assert_eq!(diagram.blocks.len(), 2);
    assert_eq!(diagram.blocks[0].id.as_deref(), Some("id1"));
    assert_eq!(diagram.blocks[0].label.as_deref(), Some("id1"));
    assert_eq!(diagram.blocks[0].block_type.as_deref(), Some("na"));
    assert_eq!(diagram.blocks[1].id.as_deref(), Some("id2"));
    assert_eq!(diagram.blocks[1].label.as_deref(), Some("id2"));
    assert_eq!(diagram.blocks[1].block_type.as_deref(), Some("na"));
}

#[test]
fn a_diagram_with_multiple_nodes_2() {
    let diagram = parsed("block\n    id1\n    id2\n    id3\n");
    assert_eq!(diagram.blocks.len(), 3);
    assert_eq!(diagram.blocks[0].id.as_deref(), Some("id1"));
    assert_eq!(diagram.blocks[0].label.as_deref(), Some("id1"));
    assert_eq!(diagram.blocks[0].block_type.as_deref(), Some("na"));
    assert_eq!(diagram.blocks[1].id.as_deref(), Some("id2"));
    assert_eq!(diagram.blocks[1].label.as_deref(), Some("id2"));
    assert_eq!(diagram.blocks[1].block_type.as_deref(), Some("na"));
    assert_eq!(diagram.blocks[2].id.as_deref(), Some("id3"));
    assert_eq!(diagram.blocks[2].label.as_deref(), Some("id3"));
    assert_eq!(diagram.blocks[2].block_type.as_deref(), Some("na"));
}

#[test]
fn a_node_with_a_square_shape_and_a_label_2() {
    let diagram = parsed("block\n    id[\"A label\"]\n    id2");
    assert_eq!(diagram.blocks.len(), 2);
    assert_eq!(diagram.blocks[0].id.as_deref(), Some("id"));
    assert_eq!(diagram.blocks[0].label.as_deref(), Some("A label"));
    assert_eq!(diagram.blocks[0].block_type.as_deref(), Some("square"));
    assert_eq!(diagram.blocks[1].id.as_deref(), Some("id2"));
    assert_eq!(diagram.blocks[1].label.as_deref(), Some("id2"));
    assert_eq!(diagram.blocks[1].block_type.as_deref(), Some("na"));
}

#[test]
fn a_diagram_with_multiple_nodes_with_edges_abc123() {
    let diagram = parsed("block\n    id1[\"first\"]  -->   id2[\"second\"]\n");
    assert_eq!(diagram.blocks.len(), 2);
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].start.as_deref(), Some("id1"));
    assert_eq!(diagram.edges[0].end, "id2");
    assert_eq!(diagram.edges[0].arrow_type_end, "arrow_point");
}

#[test]
fn a_diagram_with_multiple_nodes_with_edges_abc123_2() {
    let diagram = parsed("block\n    id1[\"first\"]  -- \"a label\" -->   id2[\"second\"]\n");
    assert_eq!(diagram.blocks.len(), 2);
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].start.as_deref(), Some("id1"));
    assert_eq!(diagram.edges[0].end, "id2");
    assert_eq!(diagram.edges[0].arrow_type_end, "arrow_point");
    assert_eq!(diagram.edges[0].label, "a label");
}

#[test]
fn a_diagram_with_column_statements() {
    let diagram = parsed("block\n    columns 2\n    block1[\"Block 1\"]\n");
    assert_eq!(columns(&diagram), 2.0);
    assert_eq!(diagram.blocks.len(), 1);
}

#[test]
fn a_diagram_without_column_statements() {
    let diagram = parsed("block\n    block1[\"Block 1\"]\n");
    assert_eq!(columns(&diagram), -1.0);
    assert_eq!(diagram.blocks.len(), 1);
}

#[test]
fn a_diagram_with_auto_column_statements() {
    let diagram = parsed("block\n    columns auto\n    block1[\"Block 1\"]\n");
    assert_eq!(columns(&diagram), -1.0);
    assert_eq!(diagram.blocks.len(), 1);
}

#[test]
fn blocks_next_to_each_other() {
    let diagram =
        parsed("block\n    columns 2\n    block1[\"Block 1\"]\n    block2[\"Block 2\"]\n  ");
    assert_eq!(columns(&diagram), 2.0);
    assert_eq!(diagram.blocks.len(), 2);
}

#[test]
fn blocks_on_top_of_each_other() {
    let diagram =
        parsed("block\n    columns 1\n    block1[\"Block 1\"]\n    block2[\"Block 2\"]\n  ");
    assert_eq!(columns(&diagram), 1.0);
    assert_eq!(diagram.blocks.len(), 2);
}

#[test]
fn compound_blocks_2() {
    let diagram =
        parsed("block\n    block\n      aBlock[\"ABlock\"]\n      bBlock[\"BBlock\"]\n    end\n  ");
    assert_eq!(diagram.blocks.len(), 1);
    let composite = &diagram.blocks[0];
    assert_eq!(children(composite).len(), 2);
    assert!(composite.id.is_some());
    assert_eq!(composite.label.as_deref(), Some(""));
    assert_eq!(composite.block_type.as_deref(), Some("composite"));

    let a_block = &children(composite)[0];
    assert!(a_block.id.is_some());
    assert_eq!(a_block.label.as_deref(), Some("ABlock"));
    assert_eq!(a_block.block_type.as_deref(), Some("square"));

    let b_block = &children(composite)[1];
    assert!(b_block.id.is_some());
    assert_eq!(b_block.label.as_deref(), Some("BBlock"));
    assert_eq!(b_block.block_type.as_deref(), Some("square"));
}

#[test]
fn compound_blocks_of_compound_blocks() {
    let diagram = parsed(
        "block\n    block\n      aBlock[\"ABlock\"]\n      block\n        bBlock[\"BBlock\"]\n      end\n    end\n  ",
    );
    let outer = &diagram.blocks[0];
    let a_block = &children(outer)[0];
    let second_composite = &children(outer)[1];
    let b_block = &children(second_composite)[0];

    assert_eq!(children(outer).len(), 2);
    assert!(outer.id.is_some());
    assert_eq!(outer.label.as_deref(), Some(""));
    assert_eq!(outer.block_type.as_deref(), Some("composite"));

    assert_eq!(children(second_composite).len(), 1);
    assert!(second_composite.id.is_some());
    assert_eq!(second_composite.label.as_deref(), Some(""));
    assert_eq!(second_composite.block_type.as_deref(), Some("composite"));

    assert!(a_block.id.is_some());
    assert_eq!(a_block.label.as_deref(), Some("ABlock"));
    assert_eq!(a_block.block_type.as_deref(), Some("square"));

    assert!(b_block.id.is_some());
    assert_eq!(b_block.label.as_deref(), Some("BBlock"));
    assert_eq!(b_block.block_type.as_deref(), Some("square"));
}

#[test]
fn compound_blocks_with_title() {
    let diagram = parsed(
        "block\n    block:compoundBlock[\"Compound block\"]\n      columns 1\n      block2[\"Block 2\"]\n    end\n  ",
    );
    assert_eq!(diagram.blocks.len(), 1);
    let compound = &diagram.blocks[0];
    let block2 = &children(compound)[0];

    assert_eq!(children(compound).len(), 1);
    assert_eq!(compound.id.as_deref(), Some("compoundBlock"));
    assert_eq!(compound.label.as_deref(), Some("Compound block"));
    assert_eq!(compound.block_type.as_deref(), Some("composite"));

    assert_eq!(block2.id.as_deref(), Some("block2"));
    assert_eq!(block2.label.as_deref(), Some("Block 2"));
    assert_eq!(block2.block_type.as_deref(), Some("square"));
}

#[test]
fn blocks_mixed_with_compound_blocks() {
    let diagram = parsed(
        "block\n    columns 1\n    block1[\"Block 1\"]\n\n    block\n      columns 2\n      block2[\"Block 2\"]\n      block3[\"Block 3\"]\n    end\n  ",
    );
    assert_eq!(diagram.blocks.len(), 2);
    let compound = &diagram.blocks[1];
    let block2 = &children(compound)[0];

    assert_eq!(children(compound).len(), 2);
    assert_eq!(block2.id.as_deref(), Some("block2"));
    assert_eq!(block2.label.as_deref(), Some("Block 2"));
    assert_eq!(block2.block_type.as_deref(), Some("square"));
}

#[test]
fn arrow_blocks() {
    let diagram = parsed(
        "block\n    columns 3\n    block1[\"Block 1\"]\n    blockArrow<[\"&nbsp;&nbsp;&nbsp;\"]>(right)\n    block2[\"Block 2\"]",
    );
    assert_eq!(diagram.blocks.len(), 3);
    let block1 = &diagram.blocks[0];
    let block_arrow = &diagram.blocks[1];
    let block2 = &diagram.blocks[2];

    assert_eq!(block1.id.as_deref(), Some("block1"));
    assert_eq!(block_arrow.id.as_deref(), Some("blockArrow"));
    assert_eq!(block2.id.as_deref(), Some("block2"));
    assert_eq!(block2.label.as_deref(), Some("Block 2"));
    assert_eq!(block2.block_type.as_deref(), Some("square"));
    assert_eq!(block_arrow.block_type.as_deref(), Some("block_arrow"));
    assert!(
        block_arrow
            .directions
            .as_ref()
            .is_some_and(|directions| directions.iter().any(|d| d == "right"))
    );
}

#[test]
fn arrow_blocks_with_multiple_points() {
    let diagram = parsed(
        "block\n    columns 1\n    A\n    blockArrow<[\"&nbsp;&nbsp;&nbsp;\"]>(up, down)\n    block\n      columns 3\n        B\n        C\n        D\n    end",
    );
    assert_eq!(diagram.blocks.len(), 3);
    let block_arrow = &diagram.blocks[1];
    assert_eq!(block_arrow.block_type.as_deref(), Some("block_arrow"));
    let directions = block_arrow.directions.clone().unwrap_or_default();
    assert!(directions.iter().any(|d| d == "up"));
    assert!(directions.iter().any(|d| d == "down"));
    assert!(!directions.iter().any(|d| d == "right"));
}

#[test]
fn blocks_with_different_widths() {
    let diagram =
        parsed("block\n    columns 3\n    one[\"One Slot\"]\n    two[\"Two slots\"]:2\n    ");
    assert_eq!(diagram.blocks.len(), 2);
    assert_eq!(diagram.blocks[1].width_in_columns, Some(2.0));
}

#[test]
fn empty_blocks() {
    let diagram =
        parsed("block\n    columns 3\n    space\n    middle[\"In the middle\"]\n    space\n    ");
    assert_eq!(diagram.blocks.len(), 3);
    let sp1 = &diagram.blocks[0];
    let middle = &diagram.blocks[1];
    let sp2 = &diagram.blocks[2];
    assert_eq!(sp1.block_type.as_deref(), Some("space"));
    assert_eq!(sp2.block_type.as_deref(), Some("space"));
    assert_eq!(middle.label.as_deref(), Some("In the middle"));
}

#[test]
fn classdef_statements_applied_to_a_block() {
    let diagram = parsed(
        "block\n    classDef black color:#ffffff, fill:#000000;\n\n    mc[\"Memcache\"]\n    class mc black\n    ",
    );
    assert_eq!(diagram.blocks.len(), 1);
    let mc = &diagram.blocks[0];
    assert!(
        mc.classes
            .as_ref()
            .is_some_and(|classes| classes.iter().any(|c| c == "black"))
    );
    let black = class_def(&diagram, "black").expect("the class is defined");
    assert_eq!(black.id, "black");
    assert_eq!(black.styles[0], "color:#ffffff");
}

#[test]
fn style_statements_applied_to_a_block() {
    let diagram = parsed(
        "block\ncolumns 1\n    B[\"A wide one in the middle\"]\n  style B fill:#f9F,stroke:#333,stroke-width:4px\n        ",
    );
    assert_eq!(diagram.blocks.len(), 1);
    let styles = diagram.blocks[0].styles.clone().unwrap_or_default();
    assert!(styles.iter().any(|style| style == "fill:#f9F"));
}

#[test]
fn should_log_a_warning_when_block_width_exceeds_column_width() {
    let text = "block-beta\n  columns 1\n  A:1\n  B:2\n  C:3\n  D:4\n  E:3\n  F:2\n  G:1";
    let (_, warnings) = parse_with_warnings(text).expect("the diagram parses");
    assert!(
        warnings
            .iter()
            .any(|warning| { warning == "Block B width 2 exceeds configured column width 1" })
    );
}

#[test]
fn should_work_with_a_proto_property() {
    validate_property("__proto__");
}

#[test]
fn should_work_with_a_constructor_property() {
    validate_property("constructor");
}

/// The spec's `validateProperty`: the property as a node id, and as a class name and a class of a
/// node, parses without an error.
fn validate_property(property: &str) {
    assert!(parse(&format!("block\n{property}")).is_ok());
    assert!(
        parse(&format!(
            "block\nA; classDef {property} color:#ffffff,fill:#000000; class A {property}"
        ))
        .is_ok()
    );
}
