//! Port of packages/mermaid/src/diagrams/class/classDiagram.spec.ts (lines 12-789). Test names follow
//! the spec's titles in snake_case, with camelCase words split (`accTitle` is `acc_title`).
//!
//! Ignored tests:
//! - The `it.skip` test of a leading newline stays ignored.
//!
//! The three `getData()` tests of the compact and hierarchical namespaces are in
//! `ferrigura-render/tests/class_db_get_data_spec.rs`.
//!
//! The five `it.each` families over the keyword list each run as one test that loops over the
//! keywords. The assertion message names every failing keyword. The note families check the note
//! text; the error family asserts that the parse fails and does not check the message wording.
//!
//! The inline snapshot of `should parse diagram with direction` is compared as whole model values.
//! `domId` and `shape` are rendering, so the expected values leave them out.

use ferrigura_model::class::{
    ClassDiagram, ClassMember, ClassNamespace, ClassNode, ClassRelation, LineType, MemberType,
    RelationEnd,
};
use ferrigura_syntax::class::parse_grammar;

/// The keywords of the spec's `it.each` list, shared by the five note families.
const KEYWORDS: &[&str] = &[
    "direction",
    "classDiagram",
    "classDiagram-v2",
    "namespace",
    "{}",
    "{",
    "}",
    "()",
    "(",
    ")",
    "[]",
    "[",
    "]",
    "class",
    "\n",
    "cssClass",
    "callback",
    "link",
    "click",
    "note",
    "note for",
    "<<",
    ">>",
    "call ",
    "~",
    "~Generic~",
    "_self",
    "_blank",
    "_parent",
    "_top",
    "<|",
    "|>",
    ">",
    "<",
    "*",
    "o",
    "\\",
    "--",
    "..",
    "-->",
    "--|>",
    ": label",
    ":::",
    ".",
    "+",
    "alphaNum",
    "!",
    "0123",
    "function()",
    "function(arg1, arg2)",
];

/// The diagram of `text`, read as the reference's `parser.parse(text)` reads it. Panics on a parse
/// error.
fn parse(text: &str) -> ClassDiagram {
    parse_grammar(text).unwrap_or_else(|error| {
        panic!(
            "the diagram does not parse at line {}: {}",
            error.line, error.message
        )
    })
}

/// The class with this id; panics when the diagram has none.
fn class<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNode {
    diagram
        .class(id)
        .unwrap_or_else(|| panic!("the diagram has no class {id}"))
}

/// The namespace with this id; panics when the diagram has none.
fn namespace<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNamespace {
    diagram
        .namespace(id)
        .unwrap_or_else(|| panic!("the diagram has no namespace {id}"))
}

/// The reference's `getDisplayDetails().displayText`: the model's `text` without the backslash that
/// escapes the visibility for the renderer. The members of this chunk hold no `<` or `>`, so for
/// them this is exact.
fn display_text(member: &ClassMember) -> &str {
    member.text.strip_prefix('\\').unwrap_or(&member.text)
}

/// `namespaces.get(parent).children.has(child)`: the children of a namespace are the namespaces
/// whose parent is it.
fn has_child(diagram: &ClassDiagram, parent: &str, child: &str) -> bool {
    diagram
        .namespaces
        .iter()
        .any(|namespace| namespace.id == child && namespace.parent.as_deref() == Some(parent))
}

/// The text of the first note of the diagram `text`, or why there is none.
fn first_note_text(text: &str) -> Result<String, String> {
    let diagram = parse_grammar(text).map_err(|error| error.message)?;
    diagram
        .notes
        .first()
        .map(|note| note.text.clone())
        .ok_or_else(|| "the diagram has no note".to_owned())
}

/// The keywords whose note does not read back as `make` expects, each with what was read.
fn failing_keywords(make: impl Fn(&str) -> (String, String)) -> Vec<String> {
    KEYWORDS
        .iter()
        .filter_map(|&keyword| {
            let (text, expected) = make(keyword);
            let actual = first_note_text(&text);
            (actual.as_deref().ok() != Some(expected.as_str()))
                .then(|| format!("{keyword:?}: expected {expected:?}, got {actual:?}"))
        })
        .collect()
}

/// Fails the test, naming every case in `failures`, when there are any.
fn assert_no_failures(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} keyword case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn should_handle_classes_within_namespaces() {
    let diagram = parse(
        r#"classDiagram
        namespace Company.Project {
          class User {
            +login(username: String, password: String)
            +logout()
          }
        }
        namespace Company.Project.Module {
          class Admin {
            +addUser(user: User)
            +removeUser(user: User)
          }
        }"#,
    );

    let user = class(&diagram, "User");
    let admin = class(&diagram, "Admin");

    // Check if the classes are correctly registered under their respective namespaces
    assert_eq!(user.parent.as_deref(), Some("Company.Project"));
    assert_eq!(admin.parent.as_deref(), Some("Company.Project.Module"));
    assert_eq!(user.methods.len(), 2);
    assert_eq!(admin.methods.len(), 2);
}

#[test]
fn should_handle_generic_classes_within_namespaces() {
    let diagram = parse(
        r#"classDiagram
    namespace Company.Project.Module {
        class GenericClass~T~ {
            +addItem(item: T)
            +getItem() T
        }
    }"#,
    );

    let generic_class = class(&diagram, "GenericClass");
    assert_eq!(generic_class.class_type, "T");
    assert_eq!(display_text(&generic_class.methods[0]), "+addItem(item: T)");
    assert_eq!(display_text(&generic_class.methods[1]), "+getItem() : T");
}

#[test]
fn should_handle_nested_namespaces_and_relationships() {
    let diagram = parse(
        r#"      classDiagram
        namespace Company.Project.Module.SubModule {
          class Report {
            +generatePDF(data: List)
            +generateCSV(data: List)
          }
        }
        namespace Company.Project.Module {
          class Admin {
            +generateReport()
          }
        }
        Admin --> Report : generates"#,
    );

    let report = class(&diagram, "Report");
    let admin = class(&diagram, "Admin");
    let relations = &diagram.relations;

    assert_eq!(
        report.parent.as_deref(),
        Some("Company.Project.Module.SubModule")
    );
    assert_eq!(admin.parent.as_deref(), Some("Company.Project.Module"));
    assert_eq!(relations[0].id1, "Admin");
    assert_eq!(relations[0].id2, "Report");
    assert_eq!(relations[0].title.as_deref(), Some("generates"));

    // Verify intermediate namespaces were created with hierarchy
    assert!(diagram.namespace("Company").is_some());
    assert!(diagram.namespace("Company.Project").is_some());
    assert!(diagram.namespace("Company.Project.Module").is_some());
    assert!(
        diagram
            .namespace("Company.Project.Module.SubModule")
            .is_some()
    );

    // Verify parent-child relationships
    assert_eq!(namespace(&diagram, "Company").parent, None);
    assert_eq!(
        namespace(&diagram, "Company.Project").parent.as_deref(),
        Some("Company")
    );
    assert_eq!(
        namespace(&diagram, "Company.Project.Module")
            .parent
            .as_deref(),
        Some("Company.Project")
    );
    assert_eq!(
        namespace(&diagram, "Company.Project.Module.SubModule")
            .parent
            .as_deref(),
        Some("Company.Project.Module")
    );

    // Verify children maps
    assert!(has_child(&diagram, "Company", "Company.Project"));
    assert!(has_child(
        &diagram,
        "Company.Project",
        "Company.Project.Module"
    ));
    assert!(has_child(
        &diagram,
        "Company.Project.Module",
        "Company.Project.Module.SubModule"
    ));

    // Only the user-declared leaves are marked explicit; intermediate ancestors are not.
    assert!(namespace(&diagram, "Company.Project.Module.SubModule").explicit);
    assert!(namespace(&diagram, "Company.Project.Module").explicit);
    assert!(!namespace(&diagram, "Company.Project").explicit);
    assert!(!namespace(&diagram, "Company").explicit);
}

#[test]
fn should_handle_syntactically_nested_namespace_blocks() {
    let diagram = parse(
        r#"classDiagram
        namespace Outer {
          namespace Inner {
            class Foo {
              +bar()
            }
          }
          class Baz {
            +qux()
          }
        }"#,
    );

    let foo = class(&diagram, "Foo");
    let baz = class(&diagram, "Baz");

    // Foo should be in the qualified inner namespace
    assert_eq!(foo.parent.as_deref(), Some("Outer.Inner"));
    // Baz should be in the outer namespace
    assert_eq!(baz.parent.as_deref(), Some("Outer"));

    // Verify namespace hierarchy
    assert!(diagram.namespace("Outer").is_some());
    assert!(diagram.namespace("Outer.Inner").is_some());
    assert_eq!(
        namespace(&diagram, "Outer.Inner").parent.as_deref(),
        Some("Outer")
    );
    assert!(has_child(&diagram, "Outer", "Outer.Inner"));

    // Both are explicitly declared via syntactic nesting, so both are `explicit`.
    assert!(namespace(&diagram, "Outer").explicit);
    assert!(namespace(&diagram, "Outer.Inner").explicit);
}

#[test]
fn should_support_custom_labels_on_namespaces() {
    let diagram = parse(
        r#"classDiagram
        namespace Auth["Authentication Service"] {
          class UserService {
            +login()
          }
        }"#,
    );

    let auth = namespace(&diagram, "Auth");
    assert_eq!(auth.id, "Auth");
    assert_eq!(auth.label, "Authentication Service");

    let user_service = class(&diagram, "UserService");
    assert_eq!(user_service.parent.as_deref(), Some("Auth"));
}

#[test]
fn should_handle_acc_title_and_acc_descr() {
    let diagram = parse(
        r#"classDiagram
            accTitle: My Title
            accDescr: My Description"#,
    );

    assert_eq!(diagram.acc_title, "My Title");
    assert_eq!(diagram.acc_descr, "My Description");
}

#[test]
fn should_handle_acc_title_and_multiline_acc_descr() {
    let diagram = parse(
        r#"classDiagram
            accTitle: My Title
            accDescr {
              This is my multi
              line description
            }"#,
    );

    assert_eq!(diagram.acc_title, "My Title");
    assert_eq!(diagram.acc_descr, "This is my multi\nline description");
}

#[test]
#[ignore = "port: skipped upstream (it.skip)"]
fn should_handle_a_leading_newline() {
    // The reference expects that the parse does not throw.
    let text = "\nclassDiagram\nclass Car";
    assert!(parse_grammar(text).is_ok());
}

#[test]
fn should_handle_fully_qualified_class_names_in_relationships() {
    let diagram = parse(
        r#"classDiagram
        namespace Company.Project.Module {
          class User
          class Admin
        }
          Admin --> User : manages"#,
    );

    let relations = &diagram.relations;
    assert_eq!(relations[0].id1, "Admin");
    assert_eq!(relations[0].id2, "User");
    assert_eq!(relations[0].title.as_deref(), Some("manages"));
}

#[test]
fn should_handle_backticked_class_names() {
    parse("classDiagram\nclass `Car`");
}

#[test]
fn should_handle_class_names_with_dash() {
    let diagram = parse("classDiagram\nclass Ca-r");

    assert_eq!(class(&diagram, "Ca-r").label, "Ca-r");
}

#[test]
fn should_handle_class_names_with_underscore() {
    parse("classDiagram\nclass `A_Car`");
}

#[test]
fn should_handle_parsing_of_separators() {
    parse(
        r#"classDiagram
class Foo1 {
  You can use
  several lines
..
as you want
and group
==
things together.
__
You can have as many groups
as you want
--
End of class
}

class User {
.. Simple Getter ..
+ getName()
+ getAddress()
.. Some setter ..
+ setName()
__ private data __
int age
-- encrypted --
String password
}"#,
    );
}

#[test]
fn should_parse_a_class_with_a_text_label() {
    let diagram = parse("classDiagram\nclass C1[\"Class 1 with text label\"]");

    assert_eq!(class(&diagram, "C1").label, "Class 1 with text label");
}

#[test]
fn should_parse_two_classes_with_text_labels() {
    let diagram = parse(
        "classDiagram\nclass C1[\"Class 1 with text label\"]\nclass C2[\"Class 2 with chars @?\"]\n",
    );

    assert_eq!(class(&diagram, "C1").label, "Class 1 with text label");
    assert_eq!(class(&diagram, "C2").label, "Class 2 with chars @?");
}

#[test]
fn should_parse_a_class_with_a_text_label_and_member() {
    let diagram = parse("classDiagram\nclass C1[\"Class 1 with text label\"]\nC1: member1");

    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.members.len(), 1);
    assert_eq!(display_text(&c1.members[0]), "member1");
}

#[test]
fn should_parse_a_class_with_a_text_label_member_and_annotation() {
    let diagram = parse(
        "classDiagram\nclass C1[\"Class 1 with text label\"]\n<<interface>> C1\nC1 : int member1",
    );

    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.members.len(), 1);
    assert_eq!(display_text(&c1.members[0]), "int member1");
    assert_eq!(c1.annotations.len(), 1);
    assert_eq!(c1.annotations[0], "interface");
}

#[test]
fn should_parse_a_class_with_text_label_and_css_class_shorthand() {
    let diagram = parse("classDiagram\nclass C1[\"Class 1 with text label\"]:::styleClass");

    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.css_classes, "default styleClass");
}

#[test]
fn should_parse_a_class_with_text_label_and_css_class() {
    let diagram = parse(
        "classDiagram\nclass C1[\"Class 1 with text label\"]\nC1 : int member1\ncssClass \"C1\" styleClass",
    );

    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(display_text(&c1.members[0]), "int member1");
    assert_eq!(c1.css_classes, "default styleClass");
}

#[test]
fn should_parse_two_classes_with_text_labels_and_css_classes() {
    let diagram = parse(
        "classDiagram\nclass C1[\"Class 1 with text label\"]\nC1 : int member1\nclass C2[\"Long long long long long long long long long long label\"]\ncssClass \"C1,C2\" styleClass",
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
    let diagram = parse(
        "classDiagram\nclass C1[\"Class 1 with text label\"]:::styleClass1\nclass C2[\"Class 2 !@#$%^&*() label\"]:::styleClass2",
    );

    let c1 = class(&diagram, "C1");
    assert_eq!(c1.label, "Class 1 with text label");
    assert_eq!(c1.css_classes, "default styleClass1");

    let c2 = class(&diagram, "C2");
    assert_eq!(c2.label, "Class 2 !@#$%^&*() label");
    assert_eq!(c2.css_classes, "default styleClass2");
}

#[test]
fn should_apply_css_classes_to_generic_classes() {
    let diagram = parse("classDiagram\nclass C1~T~\ncssClass \"C1~T~\" styleClass");

    assert_eq!(class(&diagram, "C1").css_classes, "default styleClass");
}

#[test]
fn should_apply_css_classes_to_generic_classes_via_shorthand() {
    let diagram = parse("classDiagram\nclass C1~T~:::styleClass");

    assert_eq!(class(&diagram, "C1").css_classes, "default styleClass");
}

#[test]
fn should_bind_click_event_on_generic_class() {
    let diagram = parse("classDiagram\nclass C1~T~\nclick C1~T~ call someFunction()");

    assert_eq!(class(&diagram, "C1").have_callback, Some(true));
}

#[test]
fn should_bind_tooltip_on_generic_class() {
    let diagram = parse("classDiagram\nclass C1~T~\nclick C1~T~ call cb() \"a tip\"");

    assert_eq!(class(&diagram, "C1").tooltip.as_deref(), Some("a tip"));
}

#[test]
fn should_bind_link_on_generic_class() {
    let diagram = parse("classDiagram\nclass C1~T~\nlink C1~T~ \"https://example.com\"");

    let c1 = class(&diagram, "C1");
    assert_eq!(c1.link.as_deref(), Some("https://example.com/"));
    assert_eq!(c1.css_classes, "default clickable");
}

#[test]
fn should_parse_multiple_classes_with_same_text_labels() {
    let diagram = parse(
        r#"classDiagram
class C1["Class with text label"]
class C2["Class with text label"]
class C3["Class with text label"]"#,
    );

    assert_eq!(class(&diagram, "C1").label, "Class with text label");
    assert_eq!(class(&diagram, "C2").label, "Class with text label");
    assert_eq!(class(&diagram, "C3").label, "Class with text label");
}

#[test]
fn should_parse_classes_with_different_text_labels() {
    let diagram = parse(
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
fn should_handle_note_for() {
    parse("classDiagram\nClass11 <|.. Class12\nnote for Class11 \"test\"\n");
}

#[test]
fn should_handle_note() {
    parse("classDiagram\nnote \"test\"\n");
}

#[test]
fn should_handle_a_note_with_a_keyword_in_it() {
    let failures = failing_keywords(|keyword| {
        (
            format!(
                "classDiagram\n                     note \"This is a keyword: {keyword}. It truly is.\"\n                  "
            ),
            format!("This is a keyword: {keyword}. It truly is."),
        )
    });
    assert_no_failures(&failures);
}

#[test]
fn should_handle_note_with_a_keyword_at_beginning_of_string() {
    let failures = failing_keywords(|keyword| {
        (
            format!("classDiagram\n                      note \"{keyword}\""),
            keyword.to_owned(),
        )
    });
    assert_no_failures(&failures);
}

#[test]
fn should_handle_a_note_for_with_a_keyword_in_it() {
    let failures = failing_keywords(|keyword| {
        (
            format!(
                "classDiagram\n                   class Something {{\n                     int id\n                     string name\n                   }}\n                   note for Something \"This is a keyword: {keyword}. It truly is.\"\n                   "
            ),
            format!("This is a keyword: {keyword}. It truly is."),
        )
    });
    assert_no_failures(&failures);
}

#[test]
fn should_handle_a_note_for_with_a_keyword_at_beginning_of_string() {
    let failures = failing_keywords(|keyword| {
        (
            format!(
                "classDiagram\n                    class Something {{\n                      int id\n                      string name\n                    }}\n                    note for Something \"{keyword}\"\n                    "
            ),
            keyword.to_owned(),
        )
    });
    assert_no_failures(&failures);
}

#[test]
fn should_elicit_error_for_a_keyword_after_note_token() {
    let failures: Vec<String> = KEYWORDS
        .iter()
        .filter_map(|&keyword| {
            let text = format!("classDiagram\n                   note {keyword}");
            parse_grammar(&text)
                .is_ok()
                .then(|| format!("{keyword:?}: parsed, expected the parse to fail"))
        })
        .collect();
    assert_no_failures(&failures);
}

#[test]
fn should_parse_diagram_with_direction() {
    let diagram = parse(
        r#"classDiagram
          direction TB
          class Student {
            -idCard : IdCard
          }
          class IdCard{
            -id : int
            -name : string
          }
          class Bike{
            -id : int
            -name : string
          }
          Student "1" --o "1" IdCard : carries
          Student "1" --o "1" Bike : rides"#,
    );

    // The `toMatchObject` check: the id and label, one member, no methods or annotations.
    let student = class(&diagram, "Student");
    assert_eq!(student.id, "Student");
    assert_eq!(student.label, "Student");
    assert_eq!(student.members.len(), 1);
    assert_eq!(student.members[0].id, "idCard : IdCard");
    assert_eq!(student.members[0].visibility, "-");
    assert!(student.methods.is_empty());
    assert!(student.annotations.is_empty());
    assert_eq!(student.css_classes, "default");

    // The inline snapshot of the class, as a whole model value. `domId` and `shape` are rendering,
    // so the expected value leaves them out.
    let expected_student = ClassNode {
        id: "Student".to_owned(),
        class_type: String::new(),
        label: "Student".to_owned(),
        text: "Student".to_owned(),
        css_classes: "default".to_owned(),
        annotations: Vec::new(),
        members: vec![ClassMember {
            member_type: MemberType::Attribute,
            visibility: "-".to_owned(),
            classifier: String::new(),
            id: "idCard : IdCard".to_owned(),
            parameters: None,
            return_type: None,
            text: "\\-idCard : IdCard".to_owned(),
        }],
        methods: Vec::new(),
        styles: Vec::new(),
        parent: None,
        link: None,
        link_target: None,
        tooltip: None,
        have_callback: None,
    };
    assert_eq!(student, &expected_student);

    // The inline snapshot of the relations: `type1` is `none`, `type2` `0` (aggregation), and
    // `lineType` `0` (solid).
    assert_eq!(diagram.relations.len(), 2);
    let expected_relations = vec![
        ClassRelation {
            id1: "Student".to_owned(),
            id2: "IdCard".to_owned(),
            relation_title1: "1".to_owned(),
            relation_title2: "1".to_owned(),
            title: Some("carries".to_owned()),
            type1: RelationEnd::None,
            type2: RelationEnd::Aggregation,
            line_type: LineType::Solid,
        },
        ClassRelation {
            id1: "Student".to_owned(),
            id2: "Bike".to_owned(),
            relation_title1: "1".to_owned(),
            relation_title2: "1".to_owned(),
            title: Some("rides".to_owned()),
            type1: RelationEnd::None,
            type2: RelationEnd::Aggregation,
            line_type: LineType::Solid,
        },
    ];
    assert_eq!(diagram.relations, expected_relations);
}

#[test]
fn should_revert_direction_to_default_once_direction_is_removed() {
    let first = parse("classDiagram\n          direction RL\n          class A");
    assert_eq!(first.direction, "RL");

    // The reference empties the DB with `clear()`; a new diagram starts from the default direction.
    let second = parse("classDiagram\n          class B");
    assert_eq!(second.direction, "TB");
}
