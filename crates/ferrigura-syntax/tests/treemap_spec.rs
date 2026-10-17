//! Port of `packages/parser/tests/treemap.test.ts`. Test names follow the spec's titles.
//!
//! The reference asserts Langium AST fields (`$type`, `TreemapRows`, `name`, `value`); the model of
//! this port carries the same facts after the DB rules, so the assertions read the model. Where the
//! reference could not check a fact (its titles, its `classDef` text, its class statements), the
//! port checks it on the model, or, for the class statements that the reference reports as parser
//! errors, asserts the rejection.

use ferrigura_model::treemap::{TreemapDiagram, TreemapNode};
use ferrigura_syntax::treemap::parse;

fn accepts(input: &str) -> TreemapDiagram {
    parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"))
}

/// The root's children, the top-level nodes.
fn roots(diagram: &TreemapDiagram) -> &[TreemapNode] {
    diagram.tree.children.as_deref().unwrap_or_default()
}

/// The children of a section.
fn children(node: &TreemapNode) -> &[TreemapNode] {
    node.children.as_deref().unwrap_or_default()
}

mod parse_spec {
    use super::*;

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_treemap() {
            let diagram = accepts("treemap");
            assert!(roots(&diagram).is_empty());
        }

        #[test]
        fn should_parse_a_section_node() {
            let diagram = accepts("treemap\n\"Root\"");
            assert_eq!(roots(&diagram).len(), 1);
            let section = &roots(&diagram)[0];
            assert_eq!(section.name, "Root");
            assert!(section.children.is_some(), "a section has children");
            assert_eq!(section.value, None);
        }

        #[test]
        fn should_parse_a_section_with_leaf_nodes() {
            let diagram = accepts("treemap\n\"Root\"\n  \"Child1\" , 100\n  \"Child2\" : 200\n");
            let section = &roots(&diagram)[0];
            assert_eq!(section.name, "Root");
            let leaves = children(section);
            assert_eq!(leaves.len(), 2);
            assert_eq!(leaves[0].name, "Child1");
            assert_eq!(leaves[0].value, Some(100.0));
            assert!(leaves[0].children.is_none(), "a leaf has no children");
            assert_eq!(leaves[1].name, "Child2");
            assert_eq!(leaves[1].value, Some(200.0));
        }
    }

    mod data_types {
        use super::*;

        #[test]
        fn should_correctly_parse_string_values() {
            let diagram = accepts("treemap\n\"My Section\"");
            assert_eq!(roots(&diagram)[0].name, "My Section");
        }

        #[test]
        fn should_correctly_parse_number_values() {
            let diagram = accepts("treemap\n\"Item\" : 123.45");
            let leaf = &roots(&diagram)[0];
            assert_eq!(leaf.name, "Item");
            assert_eq!(leaf.value, Some(123.45));
        }
    }

    mod validation {
        use super::*;

        #[test]
        fn should_parse_multiple_root_nodes() {
            // The reference's validator reports two roots, but its parse path does not run the
            // validator, so the parse accepts them (`docs-port/treemap/NOTES.md` section 2.7).
            let diagram = accepts("treemap\n\"Root1\"\n\"Root2\"");
            let names: Vec<&str> = roots(&diagram)
                .iter()
                .map(|node| node.name.as_str())
                .collect();
            assert_eq!(names, ["Root1", "Root2"]);
        }
    }

    mod title_and_accessibilities {
        use super::*;

        #[test]
        fn should_parse_a_treemap_with_title() {
            let diagram = accepts("treemap\ntitle My Treemap Diagram\n\"Root\"\n  \"Child\": 100");
            assert_eq!(diagram.title, "My Treemap Diagram");
            assert_eq!(roots(&diagram).len(), 1);
        }

        #[test]
        fn should_parse_a_treemap_with_acc_title() {
            let diagram =
                accepts("treemap\naccTitle: Accessible Title\n\"Root\"\n  \"Child\": 100");
            assert_eq!(diagram.acc_title, "Accessible Title");
            assert_eq!(roots(&diagram).len(), 1);
        }

        #[test]
        fn should_parse_a_treemap_with_acc_descr() {
            let diagram = accepts(
                "treemap\naccDescr: This is an accessible description\n\"Root\"\n  \"Child\": 100",
            );
            assert_eq!(diagram.acc_descr, "This is an accessible description");
            assert_eq!(roots(&diagram).len(), 1);
        }

        #[test]
        fn should_parse_a_treemap_with_multiple_accessibility_attributes() {
            let diagram = accepts(
                "treemap\ntitle My Treemap Diagram\naccTitle: Accessible Title\naccDescr: This is an accessible description\n\"Root\"\n  \"Child\": 100",
            );
            assert_eq!(diagram.title, "My Treemap Diagram");
            assert_eq!(diagram.acc_title, "Accessible Title");
            assert_eq!(diagram.acc_descr, "This is an accessible description");
            assert_eq!(roots(&diagram).len(), 1);
        }
    }

    mod class_def_and_class_statements {
        use super::*;

        #[test]
        fn should_parse_a_class_def_statement() {
            let diagram = accepts("treemap\nclassDef myClass fill:red;");
            assert_eq!(diagram.classes.len(), 1);
            assert_eq!(diagram.classes[0].id, "myClass");
            assert_eq!(diagram.classes[0].styles, ["fill:red"]);
            assert!(roots(&diagram).is_empty());
        }

        #[test]
        fn should_parse_a_class_def_statement_without_semicolon() {
            let diagram = accepts("treemap\nclassDef myClass fill:red");
            assert_eq!(diagram.classes[0].id, "myClass");
            assert_eq!(diagram.classes[0].styles, ["fill:red"]);
        }

        #[test]
        fn should_parse_a_class_def_statement_with_multiple_style_properties() {
            let diagram =
                accepts("treemap\nclassDef complexClass fill:blue stroke:#ff0000 stroke-width:2px");
            assert_eq!(diagram.classes[0].id, "complexClass");
            assert_eq!(
                diagram.classes[0].styles,
                ["fill:blue stroke:#ff0000 stroke-width:2px"]
            );
        }

        #[test]
        fn should_reject_a_class_assignment_statement() {
            // The reference's parser reports these as errors and still returns a tree, and its spec
            // does not check the errors. The port rejects them, as `class` is an identifier there.
            assert!(parse("treemap\nclass myNode myClass").is_err());
        }

        #[test]
        fn should_reject_a_class_assignment_statement_with_semicolon() {
            assert!(parse("treemap\nclass myNode myClass;").is_err());
        }

        #[test]
        fn should_parse_a_section_with_inline_class_style_using_triple_colon() {
            let diagram = accepts("treemap\n\"My Section\":::sectionClass");
            let section = &roots(&diagram)[0];
            assert_eq!(section.name, "My Section");
            assert_eq!(section.class_selector.as_deref(), Some("sectionClass"));
        }

        #[test]
        fn should_parse_a_leaf_with_inline_class_style_using_triple_colon() {
            let diagram = accepts("treemap\n\"My Leaf\" : 100:::leafClass");
            let leaf = &roots(&diagram)[0];
            assert_eq!(leaf.name, "My Leaf");
            assert_eq!(leaf.value, Some(100.0));
            assert_eq!(leaf.class_selector.as_deref(), Some("leafClass"));
        }
    }
}
