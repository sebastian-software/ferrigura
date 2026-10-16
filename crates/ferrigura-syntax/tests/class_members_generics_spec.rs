//! Port of packages/mermaid/src/diagrams/class/classDiagram.spec.ts (lines 1224 to 1493: the
//! members and method definitions, and the generics groups). Test names follow the spec's titles
//! in snake_case.
//!
//! The reference's `getDisplayDetails()` gives a `displayText` and a `cssStyle`. The model keeps no
//! display text: `display_text` composes it from the visibility, name, parameters and return type,
//! as `getDisplayDetails` does. The model's `text` is the reference's `text` field (it carries a
//! backslash before the visibility), so the checks on `displayText` do not read `text`. The
//! `cssStyle` is the renderer's style for a classifier (`*` abstract, `$` static), so the checks
//! on it assert the classifier, which the model keeps.

use ferrigura_model::class::{ClassDiagram, ClassMember, ClassNode, MemberType};
use ferrigura_syntax::class::parse_grammar;

fn parse_ok(text: &str) -> ClassDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

fn class<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNode {
    diagram
        .class(id)
        .unwrap_or_else(|| panic!("no class with id {id}"))
}

/// The reference's `getDisplayDetails().displayText`: the visibility and the name, then for a method
/// `(parameters)` and, when there is one, ` : return type`, trimmed. An empty return type counts as
/// none, as `if (this.returnType)` reads it. The generic conversion of `parseGenericTypes` is left
/// out, since no check that reads the display text has a generic type.
fn display_text(member: &ClassMember) -> String {
    let mut text = format!("{}{}", member.visibility, member.id);
    if member.member_type == MemberType::Method {
        text.push_str(&format!("({})", member.parameters.as_deref().unwrap_or("")));
        if let Some(return_type) = member
            .return_type
            .as_deref()
            .filter(|name| !name.is_empty())
        {
            text.push_str(" : ");
            text.push_str(return_type);
        }
    }
    text.trim().to_owned()
}

#[test]
fn should_handle_simple_member_declaration() {
    parse_ok(concat!("classDiagram\n", "class Car\n", "Car : wheels"));
}

#[test]
fn should_handle_direct_member_declaration() {
    let diagram = parse_ok("classDiagram\nCar : wheels");
    let car = class(&diagram, "Car");
    assert_eq!(car.members.len(), 1);
    assert_eq!(car.members[0].id, "wheels");
}

#[test]
fn should_handle_direct_member_declaration_with_type() {
    let diagram = parse_ok("classDiagram\nCar : int wheels");
    let car = class(&diagram, "Car");
    assert_eq!(car.members.len(), 1);
    assert_eq!(car.members[0].id, "int wheels");
}

#[test]
fn should_handle_simple_member_declaration_with_type() {
    parse_ok(concat!("classDiagram\n", "class Car\n", "Car : int wheels"));
}

#[test]
fn should_handle_visibility() {
    let diagram = parse_ok(concat!(
        "classDiagram\n",
        "class actual\n",
        "actual : -int privateMember\n",
        "actual : +int publicMember\n",
        "actual : #int protectedMember\n",
        "actual : ~int privatePackage",
    ));

    let actual = class(&diagram, "actual");
    assert_eq!(actual.members.len(), 4);
    assert_eq!(actual.methods.len(), 0);
    assert_eq!(display_text(&actual.members[0]), "-int privateMember");
    assert_eq!(display_text(&actual.members[1]), "+int publicMember");
    assert_eq!(display_text(&actual.members[2]), "#int protectedMember");
    assert_eq!(display_text(&actual.members[3]), "~int privatePackage");
}

#[test]
fn should_handle_generic_types() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Car\n",
        "Car : -List~Wheel~ wheels"
    ));
}

#[test]
fn should_handle_method_definition() {
    parse_ok(concat!("classDiagram\n", "class Car\n", "Car : GetSize()"));
}

#[test]
fn should_handle_simple_return_types() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Object\n",
        "Object : getObject() Object"
    ));
}

#[test]
fn should_handle_return_types_as_array() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Object\n",
        "Object : getObjects() Object[]"
    ));
}

#[test]
fn should_handle_visibility_2() {
    parse_ok(concat!(
        "classDiagram\n",
        "class actual\n",
        "actual : -privateMethod()\n",
        "actual : +publicMethod()\n",
        "actual : #protectedMethod()\n",
    ));
}

#[test]
fn should_handle_abstract_methods() {
    let diagram = parse_ok(concat!(
        "classDiagram\n",
        "class Class1\n",
        "Class1 : someMethod()*"
    ));

    let actual = class(&diagram, "Class1");
    assert_eq!(actual.annotations.len(), 0);
    assert_eq!(actual.members.len(), 0);
    assert_eq!(actual.methods.len(), 1);
    let method = &actual.methods[0];
    assert_eq!(display_text(method), "someMethod()");
    assert_eq!(method.classifier, "*");
}

#[test]
fn should_handle_static_methods() {
    let diagram = parse_ok(concat!(
        "classDiagram\n",
        "class Class1\n",
        "Class1 : someMethod()$"
    ));

    let actual = class(&diagram, "Class1");
    assert_eq!(actual.annotations.len(), 0);
    assert_eq!(actual.members.len(), 0);
    assert_eq!(actual.methods.len(), 1);
    let method = &actual.methods[0];
    assert_eq!(display_text(method), "someMethod()");
    assert_eq!(method.classifier, "$");
}

#[test]
fn should_handle_generic_types_in_arguments() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Car\n",
        "Car : +setWheels(List~Wheel~ wheels)"
    ));
}

#[test]
fn should_handle_generic_return_types() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Car\n",
        "Car : +getWheels() List~Wheel~"
    ));
}

#[test]
fn should_handle_generic_types_in_members_in_class_with_brackets() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Car {\n",
        "List~Wheel~ wheels\n",
        "setWheels(List~Wheel~ wheels)\n",
        "+getWheels() List~Wheel~\n",
        "}",
    ));
}

#[test]
fn should_handle_an_empty_class_body_with_braces() {
    let diagram = parse_ok("classDiagram\nclass EmptyClass {}");
    let actual = class(&diagram, "EmptyClass");
    assert_eq!(actual.label, "EmptyClass");
    assert_eq!(actual.members.len(), 0);
    assert_eq!(actual.methods.len(), 0);
}

#[test]
fn should_handle_generic_class() {
    parse_ok(concat!("classDiagram\n", "class Car~T~"));
}

#[test]
fn should_handle_generic_class_with_relationships() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Car~T~\n",
        "Driver -- Car : drives >\n",
        "Car *-- Wheel : have 4 >\n",
        "Car -- Person : < owns",
    ));
}

#[test]
fn should_handle_generic_class_with_a_literal_name() {
    parse_ok(concat!(
        "classDiagram\n",
        "class `Car`~T~\n",
        "Driver -- `Car` : drives >\n",
        "`Car` *-- Wheel : have 4 >\n",
        "`Car` -- Person : < owns",
    ));
}

#[test]
fn should_handle_generic_class_with_brackets() {
    parse_ok(concat!(
        "classDiagram\n",
        "class Dummy_Class~T~ {\n",
        "String data\n",
        "void methods()\n",
        "}\n",
        "\n",
        "class Flight {\n",
        "Integer flightNumber\n",
        "Date departureTime\n",
        "}",
    ));
}

#[test]
fn should_handle_generic_class_with_brackets_and_a_literal_name() {
    parse_ok(concat!(
        "classDiagram\n",
        "class `Dummy_Class`~T~ {\n",
        "String data\n",
        "  void methods()\n",
        "}\n",
        "\n",
        "class Flight {\n",
        "   flightNumber : Integer\n",
        "   departureTime : Date\n",
        "}",
    ));
}

#[test]
fn should_handle_namespace() {
    parse_ok(concat!(
        "classDiagram\n",
        "namespace Namespace1 { class Class1 }\n",
        "namespace Namespace2 { class Class1\n",
        "}\n",
        "namespace Namespace3 {\n",
        "class Class1 {\n",
        "int : test\n",
        "string : foo\n",
        "test()\n",
        "foo()\n",
        "}\n",
        "}\n",
        "namespace Namespace4 {\n",
        "class Class1 {\n",
        "int : test\n",
        "string : foo\n",
        "test()\n",
        "foo()\n",
        "}\n",
        "class Class2 {\n",
        "int : test\n",
        "string : foo\n",
        "test()\n",
        "foo()\n",
        "}\n",
        "}\n",
    ));
}

#[test]
fn should_handle_namespace_with_generic_types() {
    parse_ok(concat!(
        "classDiagram\n",
        "\n",
        "namespace space {\n",
        "    class Square~Shape~{\n",
        "        int id\n",
        "        List~int~ position\n",
        "        setPoints(List~int~ points)\n",
        "        getPoints() List~int~\n",
        "    }\n",
        "}",
    ));
}
