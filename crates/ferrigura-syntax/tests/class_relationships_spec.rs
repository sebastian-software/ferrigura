//! Port of `packages/mermaid/src/diagrams/class/classDiagram.spec.ts` (lines 1494 to 2029: the
//! `when parsing basic relationships` block). Test names follow the spec's titles in snake_case.
//!
//! The spec's `parser.parse(str)` is `parse_grammar`, which takes the text as it is. Three
//! translations are worth knowing about:
//! - `getDisplayDetails().displayText` is rebuilt from the parsed fields by `display_text`. The
//!   model keeps the DB's `text`, which writes the visibility with a backslash.
//! - The abstract and static `cssStyle` checks are the renderer's mapping of the classifier, so the
//!   port checks the classifier (`*` for abstract, `$` for static).
//! - Callbacks are not ported (decision 0001). The callback tests that only check the callback
//!   function or its arguments are ignored. The callback test that also checks a tooltip keeps the
//!   tooltip and the outcome `haveCallback`.

use ferrigura_model::class::{
    ClassDiagram, ClassMember, ClassNamespace, ClassNode, LineType, MemberType, RelationEnd,
};
use ferrigura_syntax::class::parse_grammar;

/// The spec's `parser.parse(str)`: the text as it is, panicking with the parse error.
fn parse_text(text: &str) -> ClassDiagram {
    parse_grammar(text).expect("parses")
}

/// `classDb.getClass(id)`: the class with this id, panicking when there is none.
fn class_named<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNode {
    diagram.class(id).unwrap_or_else(|| panic!("no class {id}"))
}

/// `namespace.classes.get(id)` of the spec: whether the namespace holds the class with this id.
fn namespace_holds(namespace: &ClassNamespace, id: &str) -> bool {
    namespace.classes.iter().any(|class| class == id)
}

/// `getDisplayDetails().displayText` of the reference, from the parsed fields: the visibility and
/// the name, and for a method `(parameters) : return type`, trimmed. Generic types (`~T~`) are not
/// rewritten; no member in this block has one.
fn display_text(member: &ClassMember) -> String {
    let mut text = format!("{}{}", member.visibility, member.id);
    if member.member_type == MemberType::Method {
        text.push('(');
        text.push_str(member.parameters.as_deref().unwrap_or_default());
        text.push(')');
        if let Some(value) = member
            .return_type
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            text.push_str(" : ");
            text.push_str(value);
        }
    }
    text.trim().to_owned()
}

#[test]
fn should_handle_all_basic_relationships() {
    parse_text(
        r#"classDiagram
Class1 <|-- Class02
Class03 *-- Class04
Class05 o-- Class06
Class07 .. Class08
Class09 -- Class1"#,
    );
}

#[test]
fn should_handle_backticked_class_name() {
    parse_text(
        r#"classDiagram
`Class1` <|-- Class02
Class03 *-- Class04
Class05 o-- Class06
Class07 .. Class08
Class09 -- Class1"#,
    );
}

#[test]
fn should_handle_generics() {
    let diagram = parse_text(
        r#"classDiagram
Class1~T~ <|-- Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class1").class_type, "T");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::Extension);
    assert_eq!(relation.type2, RelationEnd::None);
    assert_eq!(relation.line_type, LineType::Solid);
}

#[test]
fn should_handle_relationships_with_labels() {
    parse_text(
        r#"classDiagram
class Car
Driver -- Car : drives >
Car *-- Wheel : have 4 >
Car -- Person : < owns"#,
    );
}

#[test]
fn should_handle_relation_definitions_extension() {
    let diagram = parse_text(
        r#"classDiagram
Class1 <|-- Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::Extension);
    assert_eq!(relation.type2, RelationEnd::None);
    assert_eq!(relation.line_type, LineType::Solid);
}

#[test]
fn should_handle_relation_definition_of_different_types_and_directions() {
    parse_text(
        r#"classDiagram
Class11 <|.. Class12
Class13 --> Class14
Class15 ..> Class16
Class17 ..|> Class18
Class19 <--* Class20"#,
    );
}

#[test]
fn should_handle_cardinality_and_labels() {
    parse_text(
        r#"classDiagram
Class1 "1" *-- "many" Class02 : contains
Class03 o-- Class04 : aggregation
Class05 --> "1" Class06"#,
    );
}

#[test]
fn should_handle_dashed_relation_definition_of_different_types_and_directions() {
    parse_text(
        r#"classDiagram
Class11 <|.. Class12
Class13 <.. Class14
Class15 ..|> Class16
Class17 ..> Class18
Class19 .. Class20"#,
    );
}

#[test]
fn should_handle_relation_definitions_aggregation_and_dotted_line() {
    let diagram = parse_text(
        r#"classDiagram
Class1 o.. Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::Aggregation);
    assert_eq!(relation.type2, RelationEnd::None);
    assert_eq!(relation.line_type, LineType::Dotted);
}

#[test]
fn should_handle_relation_definitions_composition_on_both_sides() {
    let diagram = parse_text(
        r#"classDiagram
Class1 *--* Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::Composition);
    assert_eq!(relation.type2, RelationEnd::Composition);
    assert_eq!(relation.line_type, LineType::Solid);
}

#[test]
fn should_handle_relation_definitions_with_no_types() {
    let diagram = parse_text(
        r#"classDiagram
Class1 -- Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::None);
    assert_eq!(relation.type2, RelationEnd::None);
    assert_eq!(relation.line_type, LineType::Solid);
}

#[test]
fn should_handle_relation_definitions_with_type_only_on_right_side() {
    let diagram = parse_text(
        r#"classDiagram
Class1 --|> Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::None);
    assert_eq!(relation.type2, RelationEnd::Extension);
    assert_eq!(relation.line_type, LineType::Solid);
}

#[test]
fn should_handle_multiple_classes_and_relation_definitions() {
    let diagram = parse_text(
        r#"classDiagram
Class1 <|-- Class02
Class03 *-- Class04
Class05 o-- Class06
Class07 .. Class08
Class09 -- Class10"#,
    );

    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(class_named(&diagram, "Class10").id, "Class10");

    assert_eq!(diagram.relations.len(), 5);

    let first = &diagram.relations[0];
    assert_eq!(first.type1, RelationEnd::Extension);
    assert_eq!(first.type2, RelationEnd::None);
    assert_eq!(first.line_type, LineType::Solid);
    let fourth = &diagram.relations[3];
    assert_eq!(fourth.type1, RelationEnd::None);
    assert_eq!(fourth.type2, RelationEnd::None);
    assert_eq!(fourth.line_type, LineType::Dotted);
}

#[test]
fn should_handle_generic_class_with_relation_definitions() {
    let diagram = parse_text(
        r#"classDiagram
Class01~T~ <|-- Class02"#,
    );

    assert_eq!(class_named(&diagram, "Class01").id, "Class01");
    assert_eq!(class_named(&diagram, "Class01").class_type, "T");
    assert_eq!(class_named(&diagram, "Class02").id, "Class02");
    let relation = &diagram.relations[0];
    assert_eq!(relation.type1, RelationEnd::Extension);
    assert_eq!(relation.type2, RelationEnd::None);
    assert_eq!(relation.line_type, LineType::Solid);
}

#[test]
fn should_handle_class_annotations() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
<<interface>> Class1"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.annotations.len(), 1);
    assert_eq!(test_class.members.len(), 0);
    assert_eq!(test_class.methods.len(), 0);
    assert_eq!(test_class.annotations[0], "interface");
}

#[test]
fn should_handle_class_annotations_with_members_and_methods() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : int test
Class1 : test()
<<interface>> Class1"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.annotations.len(), 1);
    assert_eq!(test_class.members.len(), 1);
    assert_eq!(test_class.methods.len(), 1);
    assert_eq!(test_class.annotations[0], "interface");
}

#[test]
fn should_handle_class_annotations_in_brackets() {
    let diagram = parse_text(
        r#"classDiagram
class Class1 {
<<interface>>
}"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.annotations.len(), 1);
    assert_eq!(test_class.members.len(), 0);
    assert_eq!(test_class.methods.len(), 0);
    assert_eq!(test_class.annotations[0], "interface");
}

#[test]
fn should_handle_class_annotations_in_brackets_with_members_and_methods() {
    let diagram = parse_text(
        r#"classDiagram
class Class1 {
<<interface>>
int : test
test()
}"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.annotations.len(), 1);
    assert_eq!(test_class.members.len(), 1);
    assert_eq!(test_class.methods.len(), 1);
    assert_eq!(test_class.annotations[0], "interface");
}

#[test]
fn should_add_bracket_members_in_right_order() {
    let diagram = parse_text(
        r#"classDiagram
class Class1 {
int : test
string : foo
test()
foo()
}"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.members.len(), 2);
    assert_eq!(test_class.methods.len(), 2);
    assert_eq!(display_text(&test_class.members[0]), "int : test");
    assert_eq!(display_text(&test_class.members[1]), "string : foo");
    assert_eq!(display_text(&test_class.methods[0]), "test()");
    assert_eq!(display_text(&test_class.methods[1]), "foo()");
}

#[test]
fn should_handle_abstract_methods() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()*"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.annotations.len(), 0);
    assert_eq!(test_class.members.len(), 0);
    assert_eq!(test_class.methods.len(), 1);
    let method = &test_class.methods[0];
    assert_eq!(display_text(method), "someMethod()");
    // The spec's `cssStyle` is `font-style:italic;`, the renderer's mapping of this classifier.
    assert_eq!(method.classifier, "*");
}

#[test]
fn should_handle_static_methods() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()$"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.annotations.len(), 0);
    assert_eq!(test_class.members.len(), 0);
    assert_eq!(test_class.methods.len(), 1);
    let method = &test_class.methods[0];
    assert_eq!(display_text(method), "someMethod()");
    // The spec's `cssStyle` is `text-decoration:underline;`, the renderer's mapping of this
    // classifier.
    assert_eq!(method.classifier, "$");
}

#[test]
fn should_associate_link_and_css_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
link Class1 "google.com""#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.css_classes, "default clickable");
}

#[test]
fn should_associate_click_and_href_link_and_css_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 href "google.com""#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.css_classes, "default clickable");
}

#[test]
fn should_associate_link_with_tooltip() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
link Class1 "google.com" "A tooltip""#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.tooltip.as_deref(), Some("A tooltip"));
    assert_eq!(test_class.css_classes, "default clickable");
}

#[test]
fn should_associate_click_and_href_link_with_tooltip() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 href "google.com" "A tooltip""#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.tooltip.as_deref(), Some("A tooltip"));
    assert_eq!(test_class.css_classes, "default clickable");
}

#[test]
fn should_associate_click_and_href_link_with_tooltip_and_target_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 href "google.com" "A tooltip" _self"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.link_target.as_deref(), Some("_self"));
    assert_eq!(test_class.tooltip.as_deref(), Some("A tooltip"));
}

#[test]
fn should_associate_click_and_href_link_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 href "google.com""#,
    );

    // The reference's `setLink` without a target gives `_blank`.
    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.link_target.as_deref(), Some("_blank"));
}

#[test]
fn should_associate_click_and_href_link_with_target_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 href "google.com" _self"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.link_target.as_deref(), Some("_self"));
}

#[test]
fn should_associate_link_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
link Class1 "google.com" "A tooltip" _self"#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.link.as_deref(), Some("google.com"));
    assert_eq!(test_class.link_target.as_deref(), Some("_self"));
    assert_eq!(test_class.tooltip.as_deref(), Some("A tooltip"));
}

#[test]
#[ignore = "port: callbacks are not ported (decision 0001)"]
fn should_associate_callback_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
callback Class1 "functionCall""#,
    );

    assert_eq!(class_named(&diagram, "Class1").have_callback, Some(true));
}

#[test]
#[ignore = "port: callbacks are not ported (decision 0001)"]
fn should_associate_click_and_call_callback_appropriately() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 call functionCall()"#,
    );

    assert_eq!(class_named(&diagram, "Class1").have_callback, Some(true));
}

#[test]
#[ignore = "port: callbacks are not ported (decision 0001)"]
fn should_associate_callback_appropriately_with_an_arbitrary_number_of_args() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 call functionCall("test0", test1, test2)"#,
    );

    assert_eq!(class_named(&diagram, "Class1").have_callback, Some(true));
}

#[test]
fn should_associate_callback_with_tooltip() {
    let diagram = parse_text(
        r#"classDiagram
class Class1
Class1 : someMethod()
click Class1 call functionCall() "A tooltip""#,
    );

    let test_class = class_named(&diagram, "Class1");
    assert_eq!(test_class.have_callback, Some(true));
    assert_eq!(test_class.tooltip.as_deref(), Some("A tooltip"));
}

#[test]
fn should_add_classes_namespaces() {
    let diagram = parse_text(
        r#"classDiagram
namespace Namespace1 {
class Class1 {
int : test
string : foo
test()
foo()
}
class Class2
}"#,
    );

    let test_namespace = diagram
        .namespace("Namespace1")
        .expect("namespace Namespace1");
    let children = diagram
        .namespaces
        .iter()
        .filter(|namespace| namespace.parent.as_deref() == Some("Namespace1"))
        .count();
    assert_eq!(test_namespace.classes.len(), 2);
    assert_eq!(children, 0);
    assert!(namespace_holds(test_namespace, "Class1"));
    assert_eq!(class_named(&diagram, "Class1").id, "Class1");
    assert_eq!(diagram.classes.len(), 2);
}

#[test]
fn should_add_relations_between_classes_of_different_namespaces() {
    let diagram = parse_text(
        r#"classDiagram
      A1 --> B1
      namespace A {
        class A1 {
          +foo : string
        }
        class A2 {
          +bar : int
        }
      }
      namespace B {
        class B1 {
          +foo : bool
        }
        class B2 {
          +bar : float
        }
      }
      A2 --> B2"#,
    );

    let test_namespace_a = diagram.namespace("A").expect("namespace A");
    let test_namespace_b = diagram.namespace("B").expect("namespace B");
    assert_eq!(test_namespace_a.classes.len(), 2);
    assert!(namespace_holds(test_namespace_a, "A1"));
    assert_eq!(
        display_text(&class_named(&diagram, "A1").members[0]),
        "+foo : string"
    );
    assert!(namespace_holds(test_namespace_a, "A2"));
    assert_eq!(
        display_text(&class_named(&diagram, "A2").members[0]),
        "+bar : int"
    );
    assert_eq!(test_namespace_b.classes.len(), 2);
    assert!(namespace_holds(test_namespace_b, "B1"));
    assert_eq!(
        display_text(&class_named(&diagram, "B1").members[0]),
        "+foo : bool"
    );
    assert!(namespace_holds(test_namespace_b, "B2"));
    assert_eq!(
        display_text(&class_named(&diagram, "B2").members[0]),
        "+bar : float"
    );
    assert_eq!(diagram.classes.len(), 4);
    assert_eq!(class_named(&diagram, "A1").parent.as_deref(), Some("A"));
    assert_eq!(class_named(&diagram, "A2").parent.as_deref(), Some("A"));
    assert_eq!(class_named(&diagram, "B1").parent.as_deref(), Some("B"));
    assert_eq!(class_named(&diagram, "B2").parent.as_deref(), Some("B"));
    assert_eq!(diagram.relations[0].id1, "A1");
    assert_eq!(diagram.relations[0].id2, "B1");
    assert_eq!(diagram.relations[1].id1, "A2");
    assert_eq!(diagram.relations[1].id2, "B2");
}
