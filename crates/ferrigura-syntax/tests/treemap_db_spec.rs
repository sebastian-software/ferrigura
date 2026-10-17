//! Port of `packages/mermaid/src/diagrams/treemap/utils.test.ts` (`buildHierarchy`), and the DB's
//! own rules that the reference's specs do not reach: the classes and the title precedence.
//!
//! The reference calls `buildHierarchy` on a flat list of items. The port has no such entry point,
//! so each list is written as the text that gives the same items: a section is a quoted name, a
//! leaf a quoted name and a value, and the level is the number of blanks in front of it. The
//! expected trees are the reference's expected objects, with the leaves' fields the model keeps.

use ferrigura_model::treemap::{TreemapDiagram, TreemapNode};
use ferrigura_syntax::treemap::parse;

fn accepts(input: &str) -> TreemapDiagram {
    parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"))
}

/// A section with its children, as the reference's expected objects have them.
fn section(name: &str, children: Vec<TreemapNode>) -> TreemapNode {
    TreemapNode {
        name: name.to_owned(),
        children: Some(children),
        value: None,
        class_selector: None,
        css_compiled_styles: None,
    }
}

/// A leaf with its value.
fn leaf(name: &str, value: f64) -> TreemapNode {
    TreemapNode {
        name: name.to_owned(),
        children: None,
        value: Some(value),
        class_selector: None,
        css_compiled_styles: None,
    }
}

#[test]
fn should_convert_a_flat_array_into_a_hierarchical_structure() {
    let diagram = accepts(
        "treemap\n\
         \"Root\"\n\
         \x20   \"Branch 1\"\n\
         \x20       \"Leaf 1.1\": 10\n\
         \x20       \"Leaf 1.2\": 15\n\
         \x20   \"Branch 2\"\n\
         \x20       \"Leaf 2.1\": 20\n\
         \x20       \"Leaf 2.2\": 25\n\
         \x20       \"Leaf 2.3\": 30\n",
    );
    let expected = vec![section(
        "Root",
        vec![
            section(
                "Branch 1",
                vec![leaf("Leaf 1.1", 10.0), leaf("Leaf 1.2", 15.0)],
            ),
            section(
                "Branch 2",
                vec![
                    leaf("Leaf 2.1", 20.0),
                    leaf("Leaf 2.2", 25.0),
                    leaf("Leaf 2.3", 30.0),
                ],
            ),
        ],
    )];
    assert_eq!(diagram.tree.children, Some(expected));
}

#[test]
fn should_handle_empty_input() {
    let diagram = accepts("treemap");
    assert_eq!(diagram.tree.children, Some(Vec::new()));
}

#[test]
fn should_handle_only_root_nodes() {
    let diagram = accepts("treemap\n\"Root 1\"\n\"Root 2\"\n");
    assert_eq!(
        diagram.tree.children,
        Some(vec![
            section("Root 1", Vec::new()),
            section("Root 2", Vec::new()),
        ])
    );
}

#[test]
fn should_handle_complex_nesting_levels() {
    let diagram = accepts(
        "treemap\n\
         \"Root\"\n\
         \x20 \"Level 1\"\n\
         \x20   \"Level 2\"\n\
         \x20     \"Leaf 1\": 10\n\
         \x20   \"Level 2 again\"\n\
         \x20     \"Leaf 2\": 20\n",
    );
    let expected = vec![section(
        "Root",
        vec![section(
            "Level 1",
            vec![
                section("Level 2", vec![leaf("Leaf 1", 10.0)]),
                section("Level 2 again", vec![leaf("Leaf 2", 20.0)]),
            ],
        )],
    )];
    assert_eq!(diagram.tree.children, Some(expected));
}

#[test]
fn a_class_takes_the_styles_of_each_of_its_definitions_in_order() {
    let diagram = accepts("treemap\nclassDef cc fill:red\nclassDef cc stroke:blue\n\"A\":::cc\n");
    assert_eq!(diagram.classes.len(), 1);
    assert_eq!(diagram.classes[0].id, "cc");
    assert_eq!(diagram.classes[0].styles, ["fill:red", "stroke:blue"]);
}

#[test]
fn a_style_piece_that_is_exactly_a_label_style_is_also_a_text_style() {
    let diagram = accepts("treemap\nclassDef cc font-size:12px,fill:red\n\"A\":::cc\n");
    assert_eq!(diagram.classes[0].styles, ["font-size:12px", "fill:red"]);
    assert!(diagram.classes[0].text_styles.is_empty());
    let diagram = accepts("treemap\nclassDef cc color\n\"A\":::cc\n");
    assert_eq!(diagram.classes[0].text_styles, ["color"]);
}

#[test]
fn the_grammar_title_wins_when_it_is_not_empty_and_the_front_matter_title_fills_in_otherwise() {
    let diagram = accepts("---\ntitle: Front\n---\ntreemap\ntitle Grammar\n\"A\": 1\n");
    assert_eq!(diagram.title, "Grammar");
    let diagram = accepts("---\ntitle: Front\n---\ntreemap\ntitle\n\"A\": 1\n");
    assert_eq!(diagram.title, "Front");
    let diagram = accepts("---\ntitle: Front\n---\ntreemap\n\"A\": 1\n");
    assert_eq!(diagram.title, "Front");
}
