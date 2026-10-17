//! Port of `packages/mermaid/src/diagrams/mindmap/mindmapColors.spec.ts`. Test names follow the
//! spec's titles in snake_case; `it.each` rows get the row's value in the name.
//!
//! The colour sections (`section`, `section-edge-N`, the wrap at 11) are assigned by `getData`, which
//! is the renderer stage, so those tests are ignored here (`docs-port/mindmap/PLAN.md`, section 4).
//! The parts the DB holds are live: the classes and icons that `decorateNode` sets, and the empty and
//! root-only trees.

use ferrigura_syntax::mindmap::parse;

#[test]
#[ignore = "port: the colour sections are set by getData, the renderer stage (docs-port/mindmap/PLAN.md, section 4)"]
fn preserves_branch_coloring_with_color_by_undefined() {}

#[test]
#[ignore = "port: the colour sections are set by getData, the renderer stage (docs-port/mindmap/PLAN.md, section 4)"]
fn preserves_branch_coloring_with_color_by_branch() {}

#[test]
#[ignore = "port: the colour sections are set by getData, the renderer stage (docs-port/mindmap/PLAN.md, section 4)"]
fn colors_by_tree_depth_independently_of_indentation_and_branch() {}

#[test]
#[ignore = "port: the colour sections are set by getData, the renderer stage (docs-port/mindmap/PLAN.md, section 4)"]
fn keeps_existing_depth_colors_when_a_preceding_branch_is_inserted() {}

#[test]
#[ignore = "port: the colour sections are set by getData, the renderer stage (docs-port/mindmap/PLAN.md, section 4)"]
fn cycles_through_the_non_root_palette_for_deep_trees() {}

#[test]
fn preserves_custom_classes_and_icons() {
    // The DB keeps the class and the icon of each node; `cssClasses` is the renderer's join of them.
    let diagram = parse(
        "mindmap\nRoot\n  :::custom-root\n  Child[Child]\n    :::custom-child\n    ::icon(mdi:fire)\n",
    )
    .expect("parses");
    let root = diagram.root.expect("has a root");
    assert_eq!(root.class, "custom-root");
    let child = &root.children[0];
    assert_eq!(child.class, "custom-child");
    assert_eq!(child.icon, "mdi:fire");
}

#[test]
#[ignore = "port: the colour sections are set by getData, the renderer stage (docs-port/mindmap/PLAN.md, section 4)"]
fn reassigns_colors_when_the_mode_changes_on_the_same_parsed_tree() {}

#[test]
fn handles_empty_and_root_only_mindmaps() {
    // The empty tree has no root; the root-only tree has one node and no edge. The sections of the
    // root-only tree are `getData`'s and are not checked here.
    let empty = parse("mindmap\n").expect("parses");
    assert!(empty.root.is_none());
    let root_only = parse("mindmap\nRoot\n").expect("parses");
    let root = root_only.root.expect("has a root");
    assert!(root.children.is_empty());
}
