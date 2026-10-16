//! Port of packages/mermaid/src/diagrams/class/classDiagram.spec.ts (lines 790 to 1223: classes
//! defined in brackets, comments, click statements and annotations). Test names follow the spec's
//! titles in snake_case.
//!
//! The spec's `getClass` reads become `diagram.class(..)`. The spec reads a member's text through
//! `getDisplayDetails().displayText`, which the model does not keep; `display_text` derives it from
//! the member's `text`.
//!
//! Click statements: the spy assertions become outcome assertions on the model (the link, its
//! target, the tooltip, `haveCallback`, the `clickable` class). The two callback tests without a
//! tooltip check only the callback's function and arguments, so they are ignored: the callback is
//! not in the port (decision 0001). The two callback tests with a tooltip keep the tooltip and
//! `haveCallback` checks.

use ferrigura_model::class::{ClassDiagram, ClassMember, ClassNode};
use ferrigura_syntax::class::parse_grammar;

/// Parses the text with the grammar and the DB, as the spec's `parser.parse(str)` does.
fn parse_ok(text: &str) -> ClassDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("{error}"))
}

fn class<'a>(diagram: &'a ClassDiagram, id: &str) -> &'a ClassNode {
    diagram.class(id).unwrap_or_else(|| panic!("no class {id}"))
}

/// The reference's `getDisplayDetails().displayText`: the member's text without the backslash that
/// escapes its visibility, with `&lt;` and `&gt;` read back as `<` and `>`, and trimmed.
fn display_text(member: &ClassMember) -> String {
    member
        .text
        .strip_prefix('\\')
        .unwrap_or(member.text.as_str())
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_owned()
}

mod when_parsing_class_defined_in_brackets {
    use super::*;

    #[test]
    fn should_handle_member_definitions() {
        parse_ok("classDiagram\nclass Car{\n+int wheels\n}");
    }

    #[test]
    fn should_handle_method_definitions() {
        parse_ok("classDiagram\nclass Car{\n+size()\n}");
    }

    #[test]
    fn should_handle_a_mix_of_members_defined_in_and_outside_of_brackets() {
        parse_ok("classDiagram\nclass Car{\n+int wheels\n}\nCar : +ArrayList size()\n");
    }

    #[test]
    fn should_handle_member_and_method_definitions() {
        parse_ok("classDiagram\nclass Dummy_Class {\nString data\nvoid methods()\n}");
    }

    #[test]
    fn should_handle_return_types_on_methods() {
        parse_ok(
            "classDiagram\nclass Flight {\nint flightNumber\ndatetime departureTime\n\
             getDepartureTime() datetime\n}",
        );
    }

    #[test]
    fn should_add_bracket_members_in_right_order() {
        let diagram = parse_ok(
            "classDiagram\nclass Class1 {\nint testMember\ntest()\nstring fooMember\nfoo()\n}",
        );

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.members.len(), 2);
        assert_eq!(actual.methods.len(), 2);
        assert_eq!(display_text(&actual.members[0]), "int testMember");
        assert_eq!(display_text(&actual.members[1]), "string fooMember");
        assert_eq!(display_text(&actual.methods[0]), "test()");
        assert_eq!(display_text(&actual.methods[1]), "foo()");
    }

    #[test]
    fn should_parse_a_class_with_a_text_label_and_members() {
        let diagram =
            parse_ok("classDiagram\nclass C1[\"Class 1 with text label\"] {\n+member1\n}");

        let c1 = class(&diagram, "C1");
        assert_eq!(c1.label, "Class 1 with text label");
        assert_eq!(c1.members.len(), 1);
        assert_eq!(display_text(&c1.members[0]), "+member1");
    }

    #[test]
    fn should_parse_a_class_with_a_text_label_members_and_annotation() {
        let diagram = parse_ok(
            "classDiagram\nclass C1[\"Class 1 with text label\"] {\n<<interface>>\n+member1\n}",
        );

        let c1 = class(&diagram, "C1");
        assert_eq!(c1.label, "Class 1 with text label");
        assert_eq!(c1.members.len(), 1);
        assert_eq!(display_text(&c1.members[0]), "+member1");
        assert_eq!(c1.annotations.len(), 1);
        assert_eq!(c1.annotations[0], "interface");
    }
}

mod when_parsing_comments {
    use super::*;

    #[test]
    fn should_handle_comments_at_the_start() {
        parse_ok(concat!(
            "%% Comment\n",
            "        classDiagram\n",
            "        class Class1 {\n",
            "          int : test\n",
            "          string : foo\n",
            "          test()\n",
            "          foo()\n",
            "        }",
        ));
    }

    #[test]
    fn should_handle_comments_at_the_end() {
        parse_ok(
            "classDiagram\nclass Class1 {\nint : test\nstring : foo\ntest()\nfoo()\n\n}\n%% Comment\n",
        );
    }

    #[test]
    fn should_handle_comments_at_the_end_no_trailing_newline() {
        parse_ok(
            "classDiagram\nclass Class1 {\nint : test\nstring : foo\ntest()\nfoo()\n}\n%% Comment",
        );
    }

    #[test]
    fn should_handle_a_comment_with_multiple_line_feeds() {
        parse_ok(
            "classDiagram\n\n\n%% Comment\n\nclass Class1 {\nint : test\nstring : foo\ntest()\n\
             foo()\n}",
        );
    }

    #[test]
    fn should_handle_a_comment_with_mermaid_class_diagram_code_in_them() {
        parse_ok(
            "classDiagram\n%% Comment Class1 <|-- Class02\nclass Class1 {\nint : test\n\
             string : foo\ntest()\nfoo()\n}",
        );
    }

    #[test]
    fn should_handle_a_comment_inside_brackets() {
        parse_ok(
            "classDiagram\nclass Class1 {\n%% Comment Class1 <|-- Class02\nint : test\n\
             string : foo\ntest()\nfoo()\n}",
        );
    }
}

mod when_parsing_click_statements {
    use super::*;

    #[test]
    fn should_handle_href_link() {
        let diagram = parse_ok("classDiagram\nclass Class1 \nclick Class1 href \"google.com\" ");

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.link.as_deref(), Some("google.com"));
        assert_eq!(actual.css_classes, "default clickable");
    }

    #[test]
    fn should_handle_href_link_with_tooltip() {
        let diagram = parse_ok(
            "classDiagram\nclass Class1 \nclick Class1 href \"google.com\" \"A Tooltip\" ",
        );

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.link.as_deref(), Some("google.com"));
        assert_eq!(actual.tooltip.as_deref(), Some("A Tooltip"));
        assert_eq!(actual.css_classes, "default clickable");
    }

    #[test]
    fn should_handle_href_link_with_tooltip_and_target() {
        let diagram = parse_ok(
            "classDiagram\nclass Class1\nClass1 : someMethod()\n\
             click Class1 href \"google.com\" \"A tooltip\" _self",
        );

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.link.as_deref(), Some("google.com"));
        assert_eq!(actual.link_target.as_deref(), Some("_self"));
        assert_eq!(actual.tooltip.as_deref(), Some("A tooltip"));
        assert_eq!(actual.css_classes, "default clickable");
    }

    #[test]
    #[ignore = "port: callbacks are not ported (decision 0001)"]
    fn should_handle_function_call() {
        parse_ok("classDiagram\nclass Class1 \nclick Class1 call functionCall() ");
    }

    #[test]
    fn should_handle_function_call_with_tooltip() {
        let diagram = parse_ok(
            "classDiagram\nclass Class1 \nclick Class1 call functionCall() \"A Tooltip\" ",
        );

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.have_callback, Some(true));
        assert_eq!(actual.tooltip.as_deref(), Some("A Tooltip"));
    }

    #[test]
    #[ignore = "port: callbacks are not ported (decision 0001)"]
    fn should_handle_function_call_with_an_arbitrary_number_of_args() {
        parse_ok(
            "classDiagram\nclass Class1\nClass1 : someMethod()\nclick Class1 call functionCall(test, test1, test2)",
        );
    }

    #[test]
    fn should_handle_function_call_with_an_arbitrary_number_of_args_and_tooltip() {
        let diagram = parse_ok(
            "classDiagram\nclass Class1\nClass1 : someMethod()\n\
             click Class1 call functionCall(\"test0\", test1, test2) \"A Tooltip\"",
        );

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.have_callback, Some(true));
        assert_eq!(actual.tooltip.as_deref(), Some("A Tooltip"));
    }
}

mod when_parsing_annotations {
    use super::*;

    #[test]
    fn should_handle_class_annotations() {
        let diagram = parse_ok("classDiagram\nclass Class1\n<<interface>> Class1");

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.members.len(), 0);
        assert_eq!(actual.methods.len(), 0);
        assert_eq!(actual.annotations[0], "interface");
    }

    #[test]
    fn should_handle_class_annotations_with_members_and_methods() {
        let diagram = parse_ok(
            "classDiagram\nclass Class1\nClass1 : int test\nClass1 : test()\n<<interface>> Class1",
        );

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.members.len(), 1);
        assert_eq!(actual.methods.len(), 1);
        assert_eq!(actual.annotations[0], "interface");
    }

    #[test]
    fn should_handle_class_annotations_in_brackets() {
        let diagram = parse_ok("classDiagram\nclass Class1 {\n<<interface>>\n}");

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.members.len(), 0);
        assert_eq!(actual.methods.len(), 0);
        assert_eq!(actual.annotations[0], "interface");
    }

    #[test]
    fn should_handle_class_annotations_in_brackets_with_members_and_methods() {
        let diagram =
            parse_ok("classDiagram\nclass Class1 {\n<<interface>>\nint : test\ntest()\n}");

        let actual = class(&diagram, "Class1");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.members.len(), 1);
        assert_eq!(actual.methods.len(), 1);
        assert_eq!(actual.annotations[0], "interface");
    }

    #[test]
    fn should_handle_the_docs_example_separate_line_annotation_with_members() {
        let diagram = parse_ok(
            "classDiagram\nclass Shape\n<<interface>> Shape\nShape : noOfVertices\nShape : draw()",
        );

        let actual = class(&diagram, "Shape");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.annotations[0], "interface");
        assert_eq!(actual.members.len(), 1);
        assert_eq!(actual.methods.len(), 1);
    }

    #[test]
    fn should_handle_the_docs_example_nested_annotation_with_enumeration() {
        let diagram = parse_ok(
            "classDiagram\nclass Shape{\n    <<interface>>\n    noOfVertices\n    draw()\n}\n\
             class Color{\n    <<enumeration>>\n    RED\n    BLUE\n    GREEN\n    WHITE\n    BLACK\n}",
        );

        let shape = class(&diagram, "Shape");
        assert_eq!(shape.annotations[0], "interface");
        let color = class(&diagram, "Color");
        assert_eq!(color.annotations[0], "enumeration");
    }

    #[test]
    fn should_handle_inline_annotation_syntax_class_shape_interface() {
        let diagram = parse_ok("classDiagram\nclass Shape <<interface>>");

        let actual = class(&diagram, "Shape");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.annotations[0], "interface");
    }

    #[test]
    fn should_handle_inline_annotation_with_members_class_shape_interface_body() {
        let diagram =
            parse_ok("classDiagram\nclass Shape <<interface>> {\n    noOfVertices\n    draw()\n}");

        let actual = class(&diagram, "Shape");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.annotations[0], "interface");
        assert_eq!(actual.members.len(), 1);
        assert_eq!(actual.methods.len(), 1);
    }

    #[test]
    fn should_handle_inline_annotation_with_empty_body_class_shape_interface() {
        let diagram = parse_ok("classDiagram\nclass Shape <<interface>> {\n}");

        let actual = class(&diagram, "Shape");
        assert_eq!(actual.annotations.len(), 1);
        assert_eq!(actual.annotations[0], "interface");
    }
}
