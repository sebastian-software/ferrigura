//! Port of `packages/mermaid/src/diagrams/treeView/parser.spec.ts`: the full pipeline, from the text
//! through the grammar and the DB to the model, and the box-drawing equivalence. Test names follow
//! the reference's `it` titles in snake_case; the `describe` groups are modules.
//!
//! The reference reads `db.getRoot().children[index]`; here the same node is `root.children[index]`
//! of the model. An absent icon, class or description is `""` in the model, as the DB's `undefined`.

use ferrigura_model::tree_view::{NodeType, TreeViewNode};
use ferrigura_syntax::tree_view::parse;

/// Parses `treeView-beta` followed by one line, and returns the root's child at `index`.
fn parse_and_get_node(line: &str, index: usize) -> TreeViewNode {
    let diagram = parse(&format!("treeView-beta\n{line}")).expect("the line parses");
    diagram.root.children[index].clone()
}

fn node(line: &str) -> TreeViewNode {
    parse_and_get_node(line, 0)
}

mod bare_labels {
    use super::*;

    #[test]
    fn should_parse_a_simple_filename() {
        let n = node("index.js");
        assert_eq!(n.name, "index.js");
        assert_eq!(n.node_type, NodeType::File);
    }

    #[test]
    fn should_detect_directory_from_trailing_slash() {
        let n = node("src/");
        assert_eq!(n.name, "src");
        assert_eq!(n.node_type, NodeType::Directory);
        assert_eq!(n.icon, "");
    }

    #[test]
    fn should_parse_dotfiles() {
        let n = node(".gitignore");
        assert_eq!(n.name, ".gitignore");
        assert_eq!(n.node_type, NodeType::File);
        assert_eq!(n.icon, "");
    }

    #[test]
    fn should_parse_filenames_with_hyphens() {
        let n = node("docker-compose.yml");
        assert_eq!(n.name, "docker-compose.yml");
        assert_eq!(n.icon, "");
    }
}

mod quoted_labels {
    use super::*;

    #[test]
    fn should_strip_double_quotes() {
        let n = node("\"my file.js\"");
        assert_eq!(n.name, "my file.js");
        assert_eq!(n.node_type, NodeType::File);
    }

    #[test]
    fn should_strip_single_quotes() {
        let n = node("'my folder/'");
        assert_eq!(n.name, "my folder");
        assert_eq!(n.node_type, NodeType::Directory);
    }

    #[test]
    fn should_handle_empty_quoted_string() {
        assert_eq!(node("\"\"").name, "");
    }
}

mod class_annotation {
    use super::*;

    #[test]
    fn should_extract_class_from_bare_label() {
        let n = node("index.js :::highlight");
        assert_eq!(n.name, "index.js");
        assert_eq!(n.css_class, "highlight");
    }

    #[test]
    fn should_extract_class_from_quoted_label() {
        let n = node("\"my file.js\" :::important");
        assert_eq!(n.name, "my file.js");
        assert_eq!(n.css_class, "important");
    }

    #[test]
    fn should_handle_class_with_hyphens() {
        assert_eq!(node("file.ts :::my-class").css_class, "my-class");
    }
}

mod icon_annotation {
    use super::*;

    #[test]
    fn should_extract_icon_override() {
        let n = node("data.bin icon(folder)");
        assert_eq!(n.name, "data.bin");
        assert_eq!(n.icon, "folder");
    }

    #[test]
    fn should_accept_prefixed_iconify_names() {
        let n = node("App.tsx icon(logos:react)");
        assert_eq!(n.name, "App.tsx");
        assert_eq!(n.icon, "logos:react");
    }
}

mod description {
    use super::*;

    #[test]
    fn should_extract_description_from_bare_label() {
        let n = node("index.js ## entry point");
        assert_eq!(n.name, "index.js");
        assert_eq!(n.description, "entry point");
    }

    #[test]
    fn should_extract_description_from_quoted_label() {
        let n = node("\"my file.js\" ## the main file");
        assert_eq!(n.name, "my file.js");
        assert_eq!(n.description, "the main file");
    }

    #[test]
    fn should_handle_description_with_no_text_after_hash_hash() {
        let n = node("file.txt ##");
        assert_eq!(n.name, "file.txt");
        assert_eq!(n.description, "");
    }
}

mod combined_annotations {
    use super::*;

    #[test]
    fn should_handle_all_annotations_together() {
        let n = node("App.tsx :::highlight icon(logos:react) ## main component");
        assert_eq!(n.name, "App.tsx");
        assert_eq!(n.css_class, "highlight");
        assert_eq!(n.icon, "logos:react");
        assert_eq!(n.description, "main component");
    }

    #[test]
    fn should_handle_quoted_label_with_all_annotations() {
        let n = node("\"my app.tsx\" :::highlight icon(logos:react) ## main component");
        assert_eq!(n.name, "my app.tsx");
        assert_eq!(n.css_class, "highlight");
        assert_eq!(n.icon, "logos:react");
        assert_eq!(n.description, "main component");
    }

    #[test]
    fn should_handle_directory_with_class() {
        let n = node("src/ :::highlight");
        assert_eq!(n.name, "src");
        assert_eq!(n.node_type, NodeType::Directory);
        assert_eq!(n.css_class, "highlight");
        assert_eq!(n.icon, "");
    }
}

mod bare_labels_with_spaces {
    use super::*;

    #[test]
    fn should_handle_folder_names_with_spaces() {
        let n = node("My Documents/");
        assert_eq!(n.name, "My Documents");
        assert_eq!(n.node_type, NodeType::Directory);
        assert_eq!(n.icon, "");
    }

    #[test]
    fn should_handle_folder_names_with_spaces_and_annotations() {
        let n = node("My Documents/ :::highlight");
        assert_eq!(n.name, "My Documents");
        assert_eq!(n.node_type, NodeType::Directory);
        assert_eq!(n.css_class, "highlight");
    }

    #[test]
    fn should_handle_file_names_with_spaces() {
        let n = node("my file.ts ## some description");
        assert_eq!(n.name, "my file.ts");
        assert_eq!(n.node_type, NodeType::File);
        assert_eq!(n.icon, "");
        assert_eq!(n.description, "some description");
    }

    #[test]
    fn should_preserve_consecutive_spaces_and_unicode_in_bare_names() {
        let n = node("But  _  _ton💓.tsx");
        assert_eq!(n.name, "But  _  _ton💓.tsx");
        assert_eq!(n.node_type, NodeType::File);
    }

    #[test]
    fn should_preserve_consecutive_spaces_and_unicode_in_quoted_names() {
        assert_eq!(node("\"But  _  _ton💓.tsx\"").name, "But  _  _ton💓.tsx");
    }

    #[test]
    fn should_trim_trailing_whitespace_from_bare_names() {
        assert_eq!(node("index.js  ").name, "index.js");
    }
}

mod default_icons {
    use super::*;

    #[test]
    fn should_not_assign_an_icon_without_an_annotation() {
        // The defaults are resolved at render time (`showIcons`).
        assert_eq!(node("utils.ts").icon, "");
        assert_eq!(node("components/").icon, "");
    }
}

mod icon_suppression {
    use super::*;

    #[test]
    fn should_set_icon_to_none_for_icon_none() {
        let n = node("index.js icon(none)");
        assert_eq!(n.name, "index.js");
        assert_eq!(n.icon, "none");
    }

    #[test]
    fn should_set_icon_to_none_for_empty_icon() {
        let n = node("index.js icon()");
        assert_eq!(n.name, "index.js");
        assert_eq!(n.icon, "none");
    }

    #[test]
    fn should_suppress_icon_on_a_directory() {
        let n = node("src/ icon(none)");
        assert_eq!(n.name, "src");
        assert_eq!(n.node_type, NodeType::Directory);
        assert_eq!(n.icon, "none");
    }

    #[test]
    fn should_combine_icon_none_with_other_annotations() {
        let n = node("app.ts icon(none) :::highlight ## entry point");
        assert_eq!(n.name, "app.ts");
        assert_eq!(n.icon, "none");
        assert_eq!(n.css_class, "highlight");
        assert_eq!(n.description, "entry point");
    }
}

/// The box-drawing input and the indent input give the same tree, as `box-drawing ↔ indent
/// equivalence` checks. The whole root is compared, ids included.
mod box_drawing_indent_equivalence {
    use super::*;

    fn assert_same_tree(indent: &str, box_drawing: &str) {
        let indent_tree = parse(indent).expect("the indent input parses").root;
        let box_tree = parse(box_drawing)
            .expect("the box-drawing input parses")
            .root;
        assert_eq!(box_tree, indent_tree);
    }

    #[test]
    fn flat_list() {
        assert_same_tree(
            "treeView-beta\n    src/\n        index.js\n        App.tsx\n    package.json",
            "treeView-beta\n├── src/\n│   ├── index.js\n│   └── App.tsx\n└── package.json",
        );
    }

    #[test]
    fn nested_directories() {
        assert_same_tree(
            "treeView-beta\n    src/\n        components/\n            Button.tsx\n            Modal.tsx\n        utils/\n            helpers.ts\n    README.md",
            "treeView-beta\n├── src/\n│   ├── components/\n│   │   ├── Button.tsx\n│   │   └── Modal.tsx\n│   └── utils/\n│       └── helpers.ts\n└── README.md",
        );
    }

    #[test]
    fn with_annotations() {
        assert_same_tree(
            "treeView-beta\n    src/\n        App.tsx :::highlight ## main component\n        index.js ## entry point\n    package.json",
            "treeView-beta\n├── src/\n│   ├── App.tsx :::highlight ## main component\n│   └── index.js ## entry point\n└── package.json",
        );
    }

    #[test]
    fn deeply_nested_4_levels() {
        assert_same_tree(
            "treeView-beta\n    a/\n        b/\n            c/\n                d.txt\n            e.txt\n        f.txt",
            "treeView-beta\n└── a/\n    ├── b/\n    │   ├── c/\n    │   │   └── d.txt\n    │   └── e.txt\n    └── f.txt",
        );
    }
}
