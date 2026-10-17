//! Port of `packages/mermaid/src/diagrams/requirement/parser/requirementDiagram.spec.js`. Test names
//! follow the spec's titles in snake_case; the two `__proto__` and `constructor` titles of the loop
//! become `proto` and `constructor`.
//!
//! The reference reads its DB through `RequirementDB`; here the DB state is read from the model.
//! Each text is the reference's own, joined with line breaks as the spec joins its lines.

use ferrigura_model::requirement::{
    Direction, RelationshipKind, RequirementClass, RequirementDiagram, RequirementKind,
};
use ferrigura_syntax::requirement::parse;

fn doc(lines: &[&str]) -> String {
    lines.join("\n")
}

fn parses(text: &str) -> RequirementDiagram {
    parse(text).expect("parses")
}

fn requirement<'a>(
    diagram: &'a RequirementDiagram,
    name: &str,
) -> &'a ferrigura_model::requirement::Requirement {
    diagram
        .requirements
        .iter()
        .find(|r| r.name == name)
        .expect("requirement")
}

fn element<'a>(
    diagram: &'a RequirementDiagram,
    name: &str,
) -> &'a ferrigura_model::requirement::Element {
    diagram
        .elements
        .iter()
        .find(|e| e.name == name)
        .expect("element")
}

fn class<'a>(diagram: &'a RequirementDiagram, id: &str) -> &'a RequirementClass {
    diagram.classes.iter().find(|c| c.id == id).expect("class")
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// A requirement block with the given kind, with its properties as the spec writes them.
fn requirement_block(
    kind: &str,
    name: &str,
    id: &str,
    text: &str,
    risk: &str,
    method: &str,
) -> String {
    doc(&[
        "requirementDiagram",
        "",
        &format!("{kind} {name} {{"),
        &format!("id: {id}"),
        &format!("text: {text}"),
        &format!("risk: {risk}"),
        &format!("verifymethod: {method}"),
        "}",
    ])
}

#[test]
fn will_accept_full_requirement_definition() {
    let diagram = parses(&requirement_block(
        "requirement",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(diagram.requirements.len(), 1);
    let found = requirement(&diagram, "test_req");
    assert_eq!(found.requirement_id, "test_id");
    assert_eq!(found.text, "the test text.");
    assert!(diagram.elements.is_empty());
    assert!(diagram.relationships.is_empty());
}

#[test]
fn will_accept_full_element_definition() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "element test_el {",
        "type: test_type",
        "docref: test_ref",
        "}",
    ]));

    assert!(diagram.requirements.is_empty());
    assert_eq!(diagram.elements.len(), 1);
    let found = element(&diagram, "test_el");
    assert_eq!(found.kind, "test_type");
    assert_eq!(found.doc_ref, "test_ref");
    assert!(diagram.relationships.is_empty());
}

#[test]
fn will_use_a_accessibility_title_and_description_accdescr() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "      accTitle: test title",
        "      accDescr: my chart description",
        "      element test_name {",
        "      type: test_type",
        "      docref: test_ref",
        "      }",
    ]));

    assert_eq!(diagram.acc_title, "test title");
    assert_eq!(diagram.acc_descr, "my chart description");
}

#[test]
fn will_use_a_accessibility_title_and_multiline_description_accdescr() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "      accTitle: test title",
        "      accDescr {",
        "        my chart description",
        "line 2",
        "      }",
        "      element test_name {",
        "      type: test_type",
        "      docref: test_ref",
        "      }",
    ]));

    assert_eq!(diagram.acc_title, "test title");
    assert_eq!(diagram.acc_descr, "my chart description\nline 2");
}

#[test]
fn will_accept_full_relationship_definition() {
    let diagram = parses(&doc(&["requirementDiagram", "", "a - contains -> b"]));

    assert!(diagram.requirements.is_empty());
    assert!(diagram.elements.is_empty());
    assert_eq!(diagram.relationships.len(), 1);
    let found = &diagram.relationships[0];
    assert_eq!(found.src, "a");
    assert_eq!(found.dst, "b");
}

#[test]
fn will_accept_requirement_type_of_requirement_definition() {
    let diagram = parses(&requirement_block(
        "requirement",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").kind,
        RequirementKind::Requirement
    );
}

#[test]
fn will_accept_functionalrequirement_type_of_requirement_definition() {
    let diagram = parses(&requirement_block(
        "functionalRequirement",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").kind,
        RequirementKind::FunctionalRequirement
    );
}

#[test]
fn will_accept_interfacerequirement_type_of_requirement_definition() {
    let diagram = parses(&requirement_block(
        "interfaceRequirement",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").kind,
        RequirementKind::InterfaceRequirement
    );
}

#[test]
fn will_accept_performancerequirement_type_of_requirement_definition() {
    let diagram = parses(&requirement_block(
        "performanceRequirement",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").kind,
        RequirementKind::PerformanceRequirement
    );
}

#[test]
fn will_accept_physicalrequirement_type_of_requirement_definition() {
    let diagram = parses(&requirement_block(
        "physicalRequirement",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").kind,
        RequirementKind::PhysicalRequirement
    );
}

#[test]
fn will_accept_designconstraint_type_of_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").kind,
        RequirementKind::DesignConstraint
    );
}

#[test]
fn will_accept_low_type_of_risk_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "Low",
        "Analysis",
    ));

    assert_eq!(requirement(&diagram, "test_req").risk, "Low");
}

#[test]
fn will_accept_medium_type_of_risk_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "Medium",
        "Analysis",
    ));

    assert_eq!(requirement(&diagram, "test_req").risk, "Medium");
}

#[test]
fn will_accept_high_type_of_risk_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(requirement(&diagram, "test_req").risk, "High");
}

#[test]
fn will_accept_analysis_type_of_verification_method_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Analysis",
    ));

    assert_eq!(requirement(&diagram, "test_req").verify_method, "Analysis");
}

#[test]
fn will_accept_inspection_type_of_verification_method_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Inspection",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").verify_method,
        "Inspection"
    );
}

#[test]
fn will_accept_test_type_of_verification_method_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Test",
    ));

    assert_eq!(requirement(&diagram, "test_req").verify_method, "Test");
}

#[test]
fn will_accept_demonstration_type_of_verification_method_requirement_definition() {
    let diagram = parses(&requirement_block(
        "designConstraint",
        "test_req",
        "test_id",
        "the test text.",
        "High",
        "Demonstration",
    ));

    assert_eq!(
        requirement(&diagram, "test_req").verify_method,
        "Demonstration"
    );
}

/// The relationship statement of the spec: `a - KEYWORD -> b`.
fn relationship_model(keyword: &str) -> RequirementDiagram {
    parses(&doc(&[
        "requirementDiagram",
        "",
        &format!("a - {keyword} -> b"),
    ]))
}

#[test]
fn will_accept_contains_relationship_definition() {
    let diagram = relationship_model("contains");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Contains);
}

#[test]
fn will_accept_copies_relationship_definition() {
    let diagram = relationship_model("copies");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Copies);
}

#[test]
fn will_accept_derives_relationship_definition() {
    let diagram = relationship_model("derives");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Derives);
}

#[test]
fn will_accept_satisfies_relationship_definition() {
    let diagram = relationship_model("satisfies");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Satisfies);
}

#[test]
fn will_accept_verifies_relationship_definition() {
    let diagram = relationship_model("verifies");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Verifies);
}

#[test]
fn will_accept_refines_relationship_definition() {
    let diagram = relationship_model("refines");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Refines);
}

#[test]
fn will_accept_traces_relationship_definition() {
    let diagram = relationship_model("traces");
    assert_eq!(diagram.relationships[0].kind, RelationshipKind::Traces);
}

// The spec loops over `__proto__` and `constructor`: a name that is an object property in
// JavaScript is an ordinary name here.

#[test]
fn will_accept_proto_as_requirement_id() {
    let diagram = parses(
        "requirementDiagram\n      requirement __proto__ {\n        id: 1\n        text: the test text.\n        risk: high\n        verifymethod: test\n      }",
    );
    assert_eq!(diagram.requirements.len(), 1);
}

#[test]
fn will_accept_constructor_as_requirement_id() {
    let diagram = parses(
        "requirementDiagram\n      requirement constructor {\n        id: 1\n        text: the test text.\n        risk: high\n        verifymethod: test\n      }",
    );
    assert_eq!(diagram.requirements.len(), 1);
}

#[test]
fn will_accept_proto_as_element_id() {
    let diagram =
        parses("requirementDiagram\n      element __proto__ {\n        type: simulation\n      }");
    assert_eq!(diagram.elements.len(), 1);
}

#[test]
fn will_accept_constructor_as_element_id() {
    let diagram = parses(
        "requirementDiagram\n      element constructor {\n        type: simulation\n      }",
    );
    assert_eq!(diagram.elements.len(), 1);
}

#[test]
fn will_accept_styling_a_requirement() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement test_req {",
        "}",
        "style test_req fill:#f9f,stroke:#333,stroke-width:4px",
        "",
    ]));

    assert_eq!(
        requirement(&diagram, "test_req").css_styles,
        strings(&["fill:#f9f", "stroke:#333", "stroke-width:4px"])
    );
}

#[test]
fn will_accept_styling_an_element() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "element test_element {",
        "}",
        "style test_element fill:#f9f,stroke:#333,stroke-width:4px",
        "",
    ]));

    assert_eq!(
        element(&diagram, "test_element").css_styles,
        strings(&["fill:#f9f", "stroke:#333", "stroke-width:4px"])
    );
}

#[test]
fn will_accept_styling_multiple_things_at_once() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement test_requirement {",
        "}",
        "element test_element {",
        "}",
        "style test_requirement,test_element fill:#f9f,stroke:#333,stroke-width:4px",
        "",
    ]));

    let styles = strings(&["fill:#f9f", "stroke:#333", "stroke-width:4px"]);
    assert_eq!(requirement(&diagram, "test_requirement").css_styles, styles);
    assert_eq!(element(&diagram, "test_element").css_styles, styles);
}

#[test]
fn will_accept_defining_a_class() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "classDef myClass fill:#f9f,stroke:#333,stroke-width:4px",
        "",
    ]));

    assert_eq!(
        class(&diagram, "myClass"),
        &RequirementClass {
            id: "myClass".to_owned(),
            styles: strings(&["fill:#f9f", "stroke:#333", "stroke-width:4px"]),
            text_styles: Vec::new(),
        }
    );
}

#[test]
fn will_accept_defining_multiple_classes_at_once() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "classDef firstClass,secondClass fill:#f9f,stroke:#333,stroke-width:4px",
        "",
    ]));

    let styles = strings(&["fill:#f9f", "stroke:#333", "stroke-width:4px"]);
    assert_eq!(
        class(&diagram, "firstClass"),
        &RequirementClass {
            id: "firstClass".to_owned(),
            styles: styles.clone(),
            text_styles: Vec::new(),
        }
    );
    assert_eq!(
        class(&diagram, "secondClass"),
        &RequirementClass {
            id: "secondClass".to_owned(),
            styles,
            text_styles: Vec::new(),
        }
    );
}

#[test]
fn will_accept_assigning_a_class_via_the_class_statement() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement myReq {",
        "}",
        "classDef myClass fill:#f9f,stroke:#333,stroke-width:4px",
        "class myReq myClass",
        "",
    ]));

    assert_eq!(
        requirement(&diagram, "myReq").classes,
        strings(&["default", "myClass"])
    );
}

#[test]
fn will_accept_assigning_multiple_classes_to_multiple_things_via_the_class_statement() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement req {",
        "}",
        "element elem {",
        "}",
        "classDef class1,class2 fill:#f9f,stroke:#333,stroke-width:4px",
        "class req,elem class1,class2",
        "",
    ]));

    assert_eq!(
        requirement(&diagram, "req").classes,
        strings(&["default", "class1", "class2"])
    );
    assert_eq!(
        element(&diagram, "elem").classes,
        strings(&["default", "class1", "class2"])
    );
}

#[test]
fn will_accept_assigning_a_class_via_the_shorthand_syntax() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement myReq {",
        "}",
        "classDef myClass fill:#f9f,stroke:#333,stroke-width:4px",
        "myReq:::myClass",
        "",
    ]));

    assert_eq!(
        requirement(&diagram, "myReq").classes,
        strings(&["default", "myClass"])
    );
}

#[test]
fn will_accept_assigning_multiple_classes_via_the_shorthand_syntax() {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement myReq {",
        "}",
        "classDef class1 fill:#f9f,stroke:#333,stroke-width:4px",
        "classDef class2 color:blue",
        "myReq:::class1,class2",
        "",
    ]));

    assert_eq!(
        requirement(&diagram, "myReq").classes,
        strings(&["default", "class1", "class2"])
    );
}

#[test]
fn will_accept_assigning_a_class_or_multiple_via_the_shorthand_syntax_when_defining_a_requirement_or_element()
 {
    let diagram = parses(&doc(&[
        "requirementDiagram",
        "",
        "requirement myReq:::class1 {",
        "}",
        "element myElem:::class1,class2 {",
        "}",
        "",
        "classDef class1 fill:#f9f,stroke:#333,stroke-width:4px",
        "classDef class2 color:blue",
        "",
    ]));

    assert_eq!(
        requirement(&diagram, "myReq").classes,
        strings(&["default", "class1"])
    );
    assert_eq!(
        element(&diagram, "myElem").classes,
        strings(&["default", "class1", "class2"])
    );
}

#[test]
fn will_accept_direction_tb() {
    let diagram = parses(&doc(&["requirementDiagram", "", "direction TB", ""]));
    assert_eq!(diagram.direction, Direction::Tb);
}

#[test]
fn will_accept_direction_bt() {
    let diagram = parses(&doc(&["requirementDiagram", "", "direction BT", ""]));
    assert_eq!(diagram.direction, Direction::Bt);
}

#[test]
fn will_accept_direction_lr() {
    let diagram = parses(&doc(&["requirementDiagram", "", "direction LR", ""]));
    assert_eq!(diagram.direction, Direction::Lr);
}

#[test]
fn will_accept_direction_rl() {
    let diagram = parses(&doc(&["requirementDiagram", "", "direction RL", ""]));
    assert_eq!(diagram.direction, Direction::Rl);
}
