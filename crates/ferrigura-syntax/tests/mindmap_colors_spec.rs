//! Port of `packages/mermaid/src/diagrams/mindmap/mindmapColors.spec.ts`. Test names follow the
//! spec's titles in snake_case; `it.each` rows get the row's value in the name.
//!
//! The colour sections (`section`, `section-edge-N`, the wrap at 11) are assigned by `getData`, the renderer
//! stage: they are ported with it to `crates/ferrigura-render/tests/mindmap_get_data.rs`.
//! The parts the DB holds are live: the classes and icons that `decorateNode` sets, and the empty and
//! root-only trees.

use ferrigura_syntax::mindmap::parse;

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
fn handles_empty_and_root_only_mindmaps() {
    // The empty tree has no root; the root-only tree has one node and no edge. The sections of the
    // root-only tree are `getData`'s and are not checked here.
    let empty = parse("mindmap\n").expect("parses");
    assert!(empty.root.is_none());
    let root_only = parse("mindmap\nRoot\n").expect("parses");
    let root = root_only.root.expect("has a root");
    assert!(root.children.is_empty());
}
