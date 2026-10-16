//! Port of packages/mermaid/src/diagrams/class/classDiagram-styles.spec.js (117 lines). Test names
//! follow the spec's titles in snake_case.
//!
//! The spec checks the outcomes of `cssClass`, `classDef`, `style` and the `:::` shorthand on the
//! DB. The outcomes are the classes' `css_classes` and `styles`. The `style_classes` checks in the
//! four `classDef` tests are not in the spec: they check that the `classDef` is kept. The two tests
//! with several declarations check only the names, because the grammar keeps the comma-joined
//! text of one `classDef` as one entry (the `LABEL` token takes the commas), and the spec does not
//! pin that shape.

use ferrigura_model::class::{ClassDiagram, ClassNode, ClassStyle};
use ferrigura_syntax::class::parse_grammar;

fn parse(text: &str) -> ClassDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("{text:?} does not parse: {error:?}"))
}

fn class<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNode {
    diagram
        .class(id)
        .unwrap_or_else(|| panic!("no class with id {id}"))
}

fn style_class(id: &str, styles: &[&str]) -> ClassStyle {
    ClassStyle {
        id: id.to_owned(),
        styles: styles.iter().map(|style| (*style).to_owned()).collect(),
    }
}

/// The ids of the style classes, in the order the diagram keeps them.
fn style_class_ids(diagram: &ClassDiagram) -> Vec<&str> {
    diagram
        .style_classes
        .iter()
        .map(|style_class| style_class.id.as_str())
        .collect()
}

#[test]
fn should_be_possible_to_apply_a_css_class_to_a_class_directly() {
    let diagram = parse("classDiagram\nclass Class01:::exClass");
    assert_eq!(class(&diagram, "Class01").css_classes, "default exClass");
}

#[test]
fn should_be_possible_to_apply_a_css_class_to_a_class_directly_with_struct() {
    let diagram =
        parse("classDiagram\nclass Class1:::exClass {\nint : test\nstring : foo\ntest()\nfoo()\n}");
    assert_eq!(class(&diagram, "Class1").css_classes, "default exClass");
}

#[test]
fn should_be_possible_to_apply_a_css_class_to_a_class_with_relations() {
    let diagram = parse("classDiagram\nClass01 <|-- Class02\ncssClass \"Class01\" exClass");
    assert_eq!(class(&diagram, "Class01").css_classes, "default exClass");
}

#[test]
fn should_be_possible_to_apply_a_css_class_to_a_class() {
    let diagram = parse("classDiagram\nclass Class01\n cssClass \"Class01\" exClass");
    assert_eq!(class(&diagram, "Class01").css_classes, "default exClass");
}

#[test]
fn should_be_possible_to_apply_a_css_class_to_a_comma_separated_list_of_classes() {
    let diagram =
        parse("classDiagram\nclass Class01\n class Class02\n cssClass \"Class01,Class02\" exClass");
    assert_eq!(class(&diagram, "Class01").css_classes, "default exClass");
    assert_eq!(class(&diagram, "Class02").css_classes, "default exClass");
}

#[test]
fn should_be_possible_to_apply_a_style_to_an_individual_node() {
    let diagram = parse(
        "classDiagram\nclass Class01\n class Class02\n style Class01 fill:#f9f,stroke:#333,stroke-width:4px",
    );
    let styles = &class(&diagram, "Class01").styles;
    assert_eq!(styles[0], "fill:#f9f");
    assert_eq!(styles[1], "stroke:#333");
    assert_eq!(styles[2], "stroke-width:4px");
}

#[test]
fn should_be_possible_to_define_and_assign_a_class_inside_the_diagram() {
    let diagram =
        parse("classDiagram\nclass Class01\n cssClass \"Class01\" pink\n classDef pink fill:#f9f");
    assert_eq!(class(&diagram, "Class01").css_classes, "default pink");
    assert_eq!(diagram.style_classes, [style_class("pink", &["fill:#f9f"])]);
}

#[test]
fn should_be_possible_to_define_and_assign_a_class_using_shorthand_inside_the_diagram() {
    let diagram = parse("classDiagram\nclass Class01:::pink\n classDef pink fill:#f9f");
    assert_eq!(class(&diagram, "Class01").css_classes, "default pink");
    assert_eq!(diagram.style_classes, [style_class("pink", &["fill:#f9f"])]);
}

#[test]
fn should_properly_assign_styles_from_a_class_defined_inside_the_diagram() {
    let diagram = parse(
        "classDiagram\nclass Class01:::pink\n classDef pink fill:#f9f,stroke:#333,stroke-width:6px",
    );
    assert_eq!(
        class(&diagram, "Class01").styles,
        ["fill:#f9f", "stroke:#333", "stroke-width:6px"]
    );
    assert_eq!(style_class_ids(&diagram), ["pink"]);
}

#[test]
fn should_properly_assign_multiple_classes_and_styles_from_classes_defined_inside_the_diagram() {
    let diagram = parse(
        "classDiagram\nclass Class01:::pink\n cssClass \"Class01\" bold\n classDef pink fill:#f9f\n classDef bold stroke:#333,stroke-width:6px",
    );
    assert_eq!(
        class(&diagram, "Class01").styles,
        ["fill:#f9f", "stroke:#333", "stroke-width:6px"]
    );
    assert_eq!(class(&diagram, "Class01").css_classes, "default pink bold");
    assert_eq!(style_class_ids(&diagram), ["pink", "bold"]);
}
