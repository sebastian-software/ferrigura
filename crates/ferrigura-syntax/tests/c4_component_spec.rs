//! Port of `packages/mermaid/src/diagrams/c4/parser/c4Component.spec.ts`: two cases for each of
//! the six component kinds (12 tests). The plan is `docs-port/c4/PLAN.md`.

use ferrigura_model::c4::C4Text;
use ferrigura_syntax::c4::parse;

const DESCRIPTION: &str =
    "Allows customers to view information about their bank accounts, and make payments.";

/// A label that is a plain string, `{ text: "..." }` in the reference.
fn label_text(text: &str) -> Option<C4Text> {
    Some(C4Text::Plain(text.to_owned()))
}

// c4Component.spec.ts:23
#[test]
fn component_should_parse_a_c4_diagram_with_one_component_correctly() {
    let diagram = parse(&format!(
        r#"C4Component
title Component diagram for Internet Banking Component
Component(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "ComponentAA");
    assert_eq!(shape.descr.as_deref(), Some(DESCRIPTION));
    assert_eq!(shape.label, label_text("Internet Banking Component"));
    assert_eq!(shape.techn.as_deref(), Some("Technology"));
    assert_eq!(shape.type_c4_shape, "component");
}

// c4Component.spec.ts:51
#[test]
fn component_should_handle_a_trailing_whitespaces_after_component() {
    let ws = " ";
    parse(&format!(
        r#"C4Component{ws}
title Component diagram for Internet Banking Component{ws}
Component(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}"){ws}"#
    ))
    .expect("parses");
}

// c4Component.spec.ts:23
#[test]
fn component_db_should_parse_a_c4_diagram_with_one_component_correctly() {
    let diagram = parse(&format!(
        r#"C4Component
title Component diagram for Internet Banking Component
ComponentDb(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "ComponentAA");
    assert_eq!(shape.descr.as_deref(), Some(DESCRIPTION));
    assert_eq!(shape.label, label_text("Internet Banking Component"));
    assert_eq!(shape.techn.as_deref(), Some("Technology"));
    assert_eq!(shape.type_c4_shape, "component_db");
}

// c4Component.spec.ts:51
#[test]
fn component_db_should_handle_a_trailing_whitespaces_after_component() {
    let ws = " ";
    parse(&format!(
        r#"C4Component{ws}
title Component diagram for Internet Banking Component{ws}
ComponentDb(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}"){ws}"#
    ))
    .expect("parses");
}

// c4Component.spec.ts:23
#[test]
fn component_queue_should_parse_a_c4_diagram_with_one_component_correctly() {
    let diagram = parse(&format!(
        r#"C4Component
title Component diagram for Internet Banking Component
ComponentQueue(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "ComponentAA");
    assert_eq!(shape.descr.as_deref(), Some(DESCRIPTION));
    assert_eq!(shape.label, label_text("Internet Banking Component"));
    assert_eq!(shape.techn.as_deref(), Some("Technology"));
    assert_eq!(shape.type_c4_shape, "component_queue");
}

// c4Component.spec.ts:51
#[test]
fn component_queue_should_handle_a_trailing_whitespaces_after_component() {
    let ws = " ";
    parse(&format!(
        r#"C4Component{ws}
title Component diagram for Internet Banking Component{ws}
ComponentQueue(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}"){ws}"#
    ))
    .expect("parses");
}

// c4Component.spec.ts:23
#[test]
fn component_ext_should_parse_a_c4_diagram_with_one_component_correctly() {
    let diagram = parse(&format!(
        r#"C4Component
title Component diagram for Internet Banking Component
Component_Ext(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "ComponentAA");
    assert_eq!(shape.descr.as_deref(), Some(DESCRIPTION));
    assert_eq!(shape.label, label_text("Internet Banking Component"));
    assert_eq!(shape.techn.as_deref(), Some("Technology"));
    assert_eq!(shape.type_c4_shape, "external_component");
}

// c4Component.spec.ts:51
#[test]
fn component_ext_should_handle_a_trailing_whitespaces_after_component() {
    let ws = " ";
    parse(&format!(
        r#"C4Component{ws}
title Component diagram for Internet Banking Component{ws}
Component_Ext(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}"){ws}"#
    ))
    .expect("parses");
}

// c4Component.spec.ts:23
#[test]
fn component_db_ext_should_parse_a_c4_diagram_with_one_component_correctly() {
    let diagram = parse(&format!(
        r#"C4Component
title Component diagram for Internet Banking Component
ComponentDb_Ext(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "ComponentAA");
    assert_eq!(shape.descr.as_deref(), Some(DESCRIPTION));
    assert_eq!(shape.label, label_text("Internet Banking Component"));
    assert_eq!(shape.techn.as_deref(), Some("Technology"));
    assert_eq!(shape.type_c4_shape, "external_component_db");
}

// c4Component.spec.ts:51
#[test]
fn component_db_ext_should_handle_a_trailing_whitespaces_after_component() {
    let ws = " ";
    parse(&format!(
        r#"C4Component{ws}
title Component diagram for Internet Banking Component{ws}
ComponentDb_Ext(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}"){ws}"#
    ))
    .expect("parses");
}

// c4Component.spec.ts:23
#[test]
fn component_queue_ext_should_parse_a_c4_diagram_with_one_component_correctly() {
    let diagram = parse(&format!(
        r#"C4Component
title Component diagram for Internet Banking Component
ComponentQueue_Ext(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}")"#
    ))
    .expect("parses");
    assert_eq!(diagram.shapes.len(), 1);
    let shape = &diagram.shapes[0];
    assert_eq!(shape.alias, "ComponentAA");
    assert_eq!(shape.descr.as_deref(), Some(DESCRIPTION));
    assert_eq!(shape.label, label_text("Internet Banking Component"));
    assert_eq!(shape.techn.as_deref(), Some("Technology"));
    assert_eq!(shape.type_c4_shape, "external_component_queue");
}

// c4Component.spec.ts:51
#[test]
fn component_queue_ext_should_handle_a_trailing_whitespaces_after_component() {
    let ws = " ";
    parse(&format!(
        r#"C4Component{ws}
title Component diagram for Internet Banking Component{ws}
ComponentQueue_Ext(ComponentAA, "Internet Banking Component", "Technology", "{DESCRIPTION}"){ws}"#
    ))
    .expect("parses");
}
