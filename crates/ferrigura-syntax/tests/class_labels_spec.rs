//! Port of packages/mermaid/src/diagrams/class/classDiagram.spec.ts (lines 2030-2252). Test names
//! follow the spec's titles in snake_case.
//!
//! The spec's `member.getDisplayDetails().displayText` checks (`+member1`) are derived by
//! `display_text` from the model's `text`, the reference's `text`: the leading backslash of the
//! visibility escape is dropped, `&lt;` and `&gt;` are decoded, and the result is trimmed. The model
//! does not keep `displayText` itself, so the helper stands in for `getDisplayDetails`.
//!
//! The last test checks JavaScript object properties (`Object.hasOwn` on the ClassDB object). It is
//! ignored: the port has no such object.

use ferrigura_model::class::{ClassDiagram, ClassMember, ClassNode};
use ferrigura_syntax::class::parse_grammar;

/// The diagram that `parser.parse(text)` leaves in the spec's DB.
fn parse_ok(text: &str) -> ClassDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("expected a parse, got {error:?}"))
}

/// `classDb.getClass(id)`: the class with this id.
fn class<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNode {
    diagram
        .class(id)
        .unwrap_or_else(|| panic!("expected a class {id}"))
}

/// `member.getDisplayDetails().displayText`, derived from the model's `text`: one leading backslash
/// (the visibility escape) is dropped, `&lt;` and `&gt;` are decoded back to `<` and `>`, and the
/// result is trimmed.
fn display_text(member: &ClassMember) -> String {
    let text = member.text.strip_prefix('\\').unwrap_or(&member.text);
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_owned()
}

#[test]
fn should_parse_a_class_with_a_text_label() {
    let diagram = parse_ok(
        r#"classDiagram
  class C1["Class 1 with text label"]
  C1 -->  C2
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    let c2 = class(&diagram, "C2");
    assert_eq!(c2.label, "C2");
}

#[test]
fn should_parse_two_classes_with_text_labels() {
    let diagram = parse_ok(
        r#"classDiagram
  class C1["Class 1 with text label"]
  class C2["Class 2 with chars @?"]
  C1 -->  C2
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    let c2 = class(&diagram, "C2");
    assert_eq!(c2.label, "Class 2 with chars @?");
}

#[test]
fn should_parse_a_class_with_a_text_label_and_members() {
    let diagram = parse_ok(
        r#"classDiagram
  class C1["Class 1 with text label"] {
    +member1
  }
  C1 -->  C2
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.members.len(), 1);
    assert_eq!(display_text(&c1.members[0]), "+member1");
    let c2 = class(&diagram, "C2");
    assert_eq!(c2.label, "C2");
}

#[test]
fn should_parse_a_class_with_a_text_label_members_and_annotation() {
    let diagram = parse_ok(
        r#"classDiagram
  class C1["Class 1 with text label"] {
    <<interface>>
    +member1
  }
  C1 -->  C2
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.members.len(), 1);
    assert_eq!(c1.annotations.len(), 1);
    assert_eq!(c1.annotations[0], "interface");
    assert_eq!(display_text(&c1.members[0]), "+member1");
    let c2 = class(&diagram, "C2");
    assert_eq!(c2.label, "C2");
}

#[test]
fn should_parse_a_class_with_text_label_and_css_class_shorthand() {
    let diagram = parse_ok(
        r#"classDiagram
class C1["Class 1 with text label"]:::styleClass {
  +member1
}
C1 -->  C2
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.css_classes, "default styleClass");
    assert_eq!(display_text(&c1.members[0]), "+member1");
}

#[test]
fn should_parse_a_class_with_text_label_and_css_class() {
    let diagram = parse_ok(
        r#"classDiagram
class C1["Class 1 with text label"] {
  +member1
}
C1 --> C2
cssClass "C1" styleClass
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.css_classes, "default styleClass");
    assert_eq!(display_text(&c1.members[0]), "+member1");
}

#[test]
fn should_parse_two_classes_with_text_labels_and_css_classes() {
    let diagram = parse_ok(
        r#"classDiagram
class C1["Class 1 with text label"] {
  +member1
}
class C2["Long long long long long long long long long long label"]
C1 --> C2
cssClass "C1,C2" styleClass
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.css_classes, "default styleClass");
    let c2 = class(&diagram, "C2");
    assert_eq!(
        c2.label,
        "Long long long long long long long long long long label"
    );
    assert_eq!(c2.css_classes, "default styleClass");
}

#[test]
fn should_parse_two_classes_with_text_labels_and_css_class_shorthands() {
    let diagram = parse_ok(
        r#"classDiagram
class C1["Class 1 with text label"]:::styleClass1 {
  +member1
}
class C2["Class 2 !@#$%^&*() label"]:::styleClass2
C1 --> C2
"#,
    );
    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.css_classes, "default styleClass1");
    let c2 = class(&diagram, "C2");
    assert_eq!(c2.label, "Class 2 !@#$%^&*() label");
    assert_eq!(c2.css_classes, "default styleClass2");
}

#[test]
fn should_parse_multiple_classes_with_same_text_labels() {
    let diagram = parse_ok(
        r#"classDiagram
class C1["Class with text label"]
class C2["Class with text label"]
class C3["Class with text label"]
C1 --> C2
C3 ..> C2
"#,
    );
    assert_eq!(class(&diagram, "C1").label, "Class with text label");
    assert_eq!(class(&diagram, "C2").label, "Class with text label");
    assert_eq!(class(&diagram, "C3").label, "Class with text label");
}

#[test]
fn should_parse_classes_with_different_text_labels() {
    let diagram = parse_ok(
        r#"classDiagram
class C1["OneWord"]
class C2["With, Comma"]
class C3["With (Brackets)"]
class C4["With [Brackets]"]
class C5["With {Brackets}"]
class C6[" "]
class C7["With 1 number"]
class C8["With . period..."]
class C9["With - dash"]
class C10["With _ underscore"]
class C11["With ' single quote"]
class C12["With ~!@#$%^&*()_+=-/?"]
class C13["With Città foreign language"]
"#,
    );
    assert_eq!(class(&diagram, "C1").label, "OneWord");
    assert_eq!(class(&diagram, "C2").label, "With, Comma");
    assert_eq!(class(&diagram, "C3").label, "With (Brackets)");
    assert_eq!(class(&diagram, "C4").label, "With [Brackets]");
    assert_eq!(class(&diagram, "C5").label, "With {Brackets}");
    assert_eq!(class(&diagram, "C6").label, " ");
    assert_eq!(class(&diagram, "C7").label, "With 1 number");
    assert_eq!(class(&diagram, "C8").label, "With . period...");
    assert_eq!(class(&diagram, "C9").label, "With - dash");
    assert_eq!(class(&diagram, "C10").label, "With _ underscore");
    assert_eq!(class(&diagram, "C11").label, "With ' single quote");
    assert_eq!(class(&diagram, "C12").label, "With ~!@#$%^&*()_+=-/?");
    assert_eq!(class(&diagram, "C13").label, "With Città foreign language");
}

#[test]
#[ignore = "port: JavaScript object model, own properties of the ClassDB object (Object.hasOwn)"]
fn should_have_functions_used_in_class_jison_as_own_property() {}
