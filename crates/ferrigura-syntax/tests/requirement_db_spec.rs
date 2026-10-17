//! Port of `packages/mermaid/src/diagrams/requirement/requirementDb.spec.ts`. The DB has no public
//! surface, so each test drives `parse` with the text that makes the DB call of the spec, as the
//! journey and flowchart specs do. The reference's `getData` (the edge ids) is the renderer's stage,
//! not part of the model: that test stays ignored.

use ferrigura_model::requirement::{Direction, RelationshipKind, RequirementDiagram};
use ferrigura_syntax::requirement::parse;

fn parses(text: &str) -> RequirementDiagram {
    parse(text).expect("parses")
}

#[test]
fn should_add_a_requirement() {
    let diagram = parses("requirementDiagram\nrequirement \"requirement\" {\n}");
    assert!(
        diagram
            .requirements
            .iter()
            .any(|requirement| requirement.name == "requirement")
    );
}

#[test]
fn should_add_an_element() {
    let diagram = parses("requirementDiagram\nelement \"element\" {\n}");
    assert!(
        diagram
            .elements
            .iter()
            .any(|element| element.name == "element")
    );
}

#[test]
fn should_add_a_relationship() {
    let diagram = parses("requirementDiagram\nsrc - contains -> dst");
    let found = diagram.relationships.iter().find(|relation| {
        relation.kind == RelationshipKind::Contains
            && relation.src == "src"
            && relation.dst == "dst"
    });
    assert!(found.is_some());
}

#[test]
#[ignore = "the edge ids are made by getData, the renderer's stage; the model keeps the relationships in order"]
fn should_create_unique_edge_ids_for_parallel_relationships() {
    let diagram = parses(
        "requirementDiagram\nrequirement \"src\" {\n}\nrequirement \"dst\" {\n}\nsrc - traces -> dst\nsrc - contains -> dst",
    );
    let order: Vec<_> = diagram
        .relationships
        .iter()
        .map(|relation| (relation.src.as_str(), relation.dst.as_str()))
        .collect();
    assert_eq!(order, [("src", "dst"), ("src", "dst")]);
}

#[test]
fn should_detect_single_class() {
    // The spec defines `stroke-width: 8px`; a style is its tokens joined with no spaces, so the
    // space after the colon is not kept (`fill: red` is `fill:red`).
    let diagram = parses("requirementDiagram\nclassDef a stroke-width:8px");
    let class = diagram
        .classes
        .iter()
        .find(|class| class.id == "a")
        .expect("class a");
    assert_eq!(class.styles, ["stroke-width:8px"]);
}

#[test]
fn should_detect_many_classes() {
    let diagram = parses("requirementDiagram\nclassDef a,b stroke-width:8px");
    for id in ["a", "b"] {
        let class = diagram
            .classes
            .iter()
            .find(|class| class.id == id)
            .expect("class");
        assert_eq!(class.styles, ["stroke-width:8px"]);
    }
}

#[test]
fn should_detect_direction() {
    let diagram = parses("requirementDiagram\ndirection TB");
    assert_eq!(diagram.direction, Direction::Tb);
}

#[test]
fn should_add_styles_to_a_requirement_and_element() {
    let diagram = parses(
        "requirementDiagram\nrequirement \"requirement\" {\n}\nelement \"element\" {\n}\nstyle \"requirement\" color:red\nstyle \"element\" stroke-width:4px,stroke: yellow",
    );
    let requirement = diagram
        .requirements
        .iter()
        .find(|requirement| requirement.name == "requirement")
        .expect("requirement");
    let element = diagram
        .elements
        .iter()
        .find(|element| element.name == "element")
        .expect("element");
    assert_eq!(requirement.css_styles, ["color:red"]);
    assert_eq!(element.css_styles, ["stroke-width:4px", "stroke:yellow"]);
}

#[test]
fn should_add_classes_to_a_requirement_and_element() {
    let diagram = parses(
        "requirementDiagram\nrequirement \"requirement\" {\n}\nelement \"element\" {\n}\nclass requirement,element myClass",
    );
    let requirement = diagram
        .requirements
        .iter()
        .find(|requirement| requirement.name == "requirement")
        .expect("requirement");
    let element = diagram
        .elements
        .iter()
        .find(|element| element.name == "element")
        .expect("element");
    assert_eq!(requirement.classes, ["default", "myClass"]);
    assert_eq!(element.classes, ["default", "myClass"]);
}

#[test]
fn should_add_styles_to_a_requirement_and_element_inherited_from_a_class() {
    let diagram = parses(
        "requirementDiagram\nrequirement \"requirement\" {\n}\nelement \"element\" {\n}\nclassDef myClass color:red\nclassDef myClass2 stroke-width:4px,stroke:yellow\nclass requirement myClass\nclass element myClass2",
    );
    let requirement = diagram
        .requirements
        .iter()
        .find(|requirement| requirement.name == "requirement")
        .expect("requirement");
    let element = diagram
        .elements
        .iter()
        .find(|element| element.name == "element")
        .expect("element");
    assert_eq!(requirement.css_styles, ["color:red"]);
    assert_eq!(element.css_styles, ["stroke-width:4px", "stroke:yellow"]);
}
