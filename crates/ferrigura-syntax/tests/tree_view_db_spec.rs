//! Port of `packages/mermaid/src/diagrams/treeView/db.spec.ts`. The reference calls `db.addNode`
//! directly; the port has no public DB, so each call becomes the line the grammar would read for it
//! (`level` spaces, then the name, with a trailing `/` for a directory), and the test reads the
//! model. The expected values are the reference's.
//!
//! The configuration test (`getConfig`) is the renderer's: `tree_view_render_spec.rs` in
//! `ferrigura-render` checks the section's defaults.

use ferrigura_model::tree_view::{NodeType, TreeViewDiagram, TreeViewNode};
use ferrigura_syntax::tree_view::parse;

/// A node line as the grammar reads an `addNode(level, name, nodeType, …)` call: the indentation,
/// the name, a `/` for a directory and the annotations in their order.
fn line(level: usize, name: &str, node_type: NodeType) -> String {
    let slash = if node_type == NodeType::Directory {
        "/"
    } else {
        ""
    };
    format!("{}{name}{slash}", " ".repeat(level))
}

/// The diagram of the given node lines, after the keyword.
fn diagram(lines: &[String]) -> TreeViewDiagram {
    let text = format!("treeView-beta\n{}", lines.join("\n"));
    parse(&text).expect("the lines parse")
}

mod clear {
    use super::*;

    #[test]
    fn should_reset_state_to_initial_root() {
        // A new parse starts from the root alone, as `clear` resets the DB.
        let root = diagram(&[]).root;
        assert!(root.children.is_empty());
        assert_eq!(root.name, "/");
        assert_eq!(root.level, -1);
    }
}

mod get_root {
    use super::*;

    #[test]
    fn should_return_a_root_node_at_level_minus_one() {
        let root = diagram(&[]).root;
        assert_eq!(root.level, -1);
        assert_eq!(root.node_type, NodeType::Directory);
        assert!(root.children.is_empty());
    }
}

mod get_count {
    use super::*;

    #[test]
    fn should_start_at_1() {
        assert_eq!(diagram(&[]).count, 1);
    }

    #[test]
    fn should_increment_with_each_added_node() {
        let one = diagram(&[line(0, "file1.ts", NodeType::File)]);
        assert_eq!(one.count, 2);
        let two = diagram(&[
            line(0, "file1.ts", NodeType::File),
            line(0, "file2.ts", NodeType::File),
        ]);
        assert_eq!(two.count, 3);
    }
}

mod add_node {
    use super::*;

    #[test]
    fn should_add_a_child_to_root_at_level_0() {
        let root = diagram(&[line(0, "index.ts", NodeType::File)]).root;
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "index.ts");
        assert_eq!(root.children[0].node_type, NodeType::File);
        assert_eq!(root.children[0].level, 0);
    }

    #[test]
    fn should_add_nested_children_using_increasing_levels() {
        let root = diagram(&[
            line(0, "src", NodeType::Directory),
            line(4, "index.ts", NodeType::File),
        ])
        .root;
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "src");
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].children[0].name, "index.ts");
    }

    #[test]
    fn should_pop_stack_when_level_decreases() {
        let root = diagram(&[
            line(0, "src", NodeType::Directory),
            line(4, "index.ts", NodeType::File),
            line(0, "README.md", NodeType::File),
        ])
        .root;
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].name, "src");
        assert_eq!(root.children[1].name, "README.md");
    }

    #[test]
    fn should_add_siblings_at_the_same_level() {
        let root = diagram(&[
            line(0, "file1.ts", NodeType::File),
            line(0, "file2.ts", NodeType::File),
            line(0, "file3.ts", NodeType::File),
        ])
        .root;
        assert_eq!(root.children.len(), 3);
    }

    #[test]
    fn should_store_css_class() {
        let root = diagram(&["app.ts :::highlight".to_owned()]).root;
        assert_eq!(root.children[0].css_class, "highlight");
    }

    #[test]
    fn should_store_icon() {
        let root = diagram(&["data.bin icon(logos:mysql)".to_owned()]).root;
        assert_eq!(root.children[0].icon, "logos:mysql");
    }

    #[test]
    fn should_store_description() {
        let root = diagram(&["index.ts ## entry point".to_owned()]).root;
        assert_eq!(root.children[0].description, "entry point");
    }

    #[test]
    fn should_store_all_optional_fields_together() {
        let root =
            diagram(&["App.tsx :::highlight icon(logos:react) ## main component".to_owned()]).root;
        let n: &TreeViewNode = &root.children[0];
        assert_eq!(n.name, "App.tsx");
        assert_eq!(n.css_class, "highlight");
        assert_eq!(n.icon, "logos:react");
        assert_eq!(n.description, "main component");
    }

    #[test]
    fn should_build_a_deep_tree() {
        let root = diagram(&[
            line(0, "src", NodeType::Directory),
            line(4, "components", NodeType::Directory),
            line(8, "Button.tsx", NodeType::File),
            line(8, "Header.tsx", NodeType::File),
            line(4, "utils", NodeType::Directory),
            line(8, "helpers.ts", NodeType::File),
        ])
        .root;
        assert_eq!(root.children[0].children[0].children.len(), 2);
        assert_eq!(root.children[0].children[1].name, "utils");
        assert_eq!(root.children[0].children[1].children[0].name, "helpers.ts");
    }

    #[test]
    fn should_assign_unique_incrementing_ids() {
        let root = diagram(&[
            line(0, "a", NodeType::File),
            line(0, "b", NodeType::File),
            line(0, "c", NodeType::File),
        ])
        .root;
        let ids: Vec<u32> = root.children.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}

mod accessibility {
    use super::*;

    #[test]
    fn should_set_and_get_acc_title() {
        let diagram = parse("treeView-beta\naccTitle: My Tree\na").expect("parses");
        assert_eq!(diagram.acc_title, "My Tree");
    }

    #[test]
    fn should_set_and_get_acc_description() {
        let diagram = parse("treeView-beta\naccDescr: A file tree\na").expect("parses");
        assert_eq!(diagram.acc_descr, "A file tree");
    }

    #[test]
    fn should_set_and_get_diagram_title() {
        let diagram = parse("treeView-beta\ntitle File Structure\na").expect("parses");
        assert_eq!(diagram.title, "File Structure");
    }
}
