//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-node-data.spec.js`. Test names follow
//! the spec's titles in snake_case; a title that repeats after normalising gets `_2`, `_3`, … in
//! source order.
//!
//! The spec reads every result through `getData()`. The port asserts the model equivalent, as
//! `docs-port/flowchart/PLAN.md` section 10.2 decides: the vertex text and the raw shape name. The
//! `@{ … }` tests are shape data and are ignored until step 7. The tests that use no `@{ … }` run
//! now: the diamond, and the two edge-id tests, whose ids the DB sets and `getData` passes through.
//!
//! The last shape-data error test comes from commit 764814a2d on `main`; this branch's copy of the
//! spec does not have it yet.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram};
use ferrigura_syntax::ParseError;
use ferrigura_syntax::flowchart::{FlowchartError, parse};

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex {id}"))
}

fn texts(diagram: &FlowchartDiagram) -> Vec<&str> {
    diagram
        .vertices
        .iter()
        .map(|vertex| vertex.text.as_str())
        .collect()
}

fn syntax_error(text: &str) -> ParseError {
    match parse(text) {
        Err(FlowchartError::Syntax(error)) => error,
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

mod when_parsing_directions {
    use super::*;

    #[test]
    fn should_handle_basic_shape_data_statements() {
        let diagram = parse("flowchart TB\n      D@{ shape: rounded}").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(vertex(&diagram, "D").text, "D");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_2() {
        let diagram = parse("flowchart TB\n      D@{ shape: rounded }").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(vertex(&diagram, "D").text, "D");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with() {
        let diagram = parse("flowchart TB\n      D@{ shape: rounded } & E").expect("parses");

        assert_eq!(diagram.vertices.len(), 2);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram), ["D", "E"]);
    }

    #[test]
    fn should_handle_shape_data_statements_with_edges() {
        let diagram = parse("flowchart TB\n      D@{ shape: rounded } --> E").expect("parses");

        assert_eq!(diagram.vertices.len(), 2);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram), ["D", "E"]);
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_1() {
        let diagram = parse("flowchart TB\n      D@{ shape: rounded } & E --> F").expect("parses");

        assert_eq!(diagram.vertices.len(), 3);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram)[..2], ["D", "E"]);
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_2() {
        let diagram =
            parse("flowchart TB\n      D@{ shape: rounded } & E@{ shape: rounded } --> F")
                .expect("parses");

        assert_eq!(diagram.vertices.len(), 3);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram)[..2], ["D", "E"]);
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_3() {
        let diagram = parse(
            "flowchart TB\n      D@{ shape: rounded } & E@{ shape: rounded } --> F & G@{ shape: rounded }",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 4);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram)[..2], ["D", "E"]);
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_4() {
        let diagram = parse(
            "flowchart TB\n      D@{ shape: rounded } & E@{ shape: rounded } --> F@{ shape: rounded } & G@{ shape: rounded }",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 4);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram)[..2], ["D", "E"]);
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_5_trailing_space() {
        let diagram = parse(
            "flowchart TB\n         D@{ shape: rounded } & E@{ shape: rounded } --> F{ shape: rounded } & G{ shape: rounded }    ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 4);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(texts(&diagram)[..2], ["D", "E"]);
    }

    #[test]
    fn should_no_matter_of_there_are_no_leading_spaces() {
        let diagram = parse("flowchart TB\n      D@{shape: rounded}").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(vertex(&diagram, "D").text, "D");
    }

    #[test]
    fn should_no_matter_of_there_are_many_leading_spaces() {
        let diagram = parse("flowchart TB\n      D@{       shape: rounded}").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(vertex(&diagram, "D").text, "D");
    }

    #[test]
    fn should_be_forgiving_with_many_spaces_before_the_end() {
        let diagram = parse("flowchart TB\n      D@{ shape: rounded         }").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(vertex(&diagram, "D").text, "D");
    }

    #[test]
    fn should_be_possible_to_add_multiple_properties_on_the_same_line() {
        let diagram =
            parse("flowchart TB\n      D@{ shape: rounded , label: \"DD\"}").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("rounded"));
        assert_eq!(vertex(&diagram, "D").text, "DD");
    }

    #[test]
    fn should_be_possible_to_link_to_a_node_with_more_data() {
        let diagram = parse(
            "flowchart TB\n      A --> D@{\n        shape: circle\n        other: \"clock\"\n     }\n\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 2);
        assert_eq!(texts(&diagram), ["A", "D"]);
        assert_eq!(vertex(&diagram, "D").shape.as_deref(), Some("circle"));
        assert_eq!(diagram.edges.len(), 1);
    }

    #[test]
    fn should_not_disturb_adding_multiple_nodes_after_each_other() {
        let diagram = parse(
            "flowchart TB\n      A[hello]\n      B@{\n        shape: circle\n        other: \"clock\"\n     }\n      C[Hello]@{\n        shape: circle\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 3);
        assert_eq!(texts(&diagram), ["hello", "B", "Hello"]);
        assert_eq!(vertex(&diagram, "B").shape.as_deref(), Some("circle"));
        assert_eq!(vertex(&diagram, "C").shape.as_deref(), Some("circle"));
    }

    #[test]
    fn should_use_handle_bracket_end_character_inside_the_shape_data() {
        let diagram = parse(
            "flowchart TB\n      A@{\n        label: \"This is }\"\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").text, "This is }");
    }

    #[test]
    fn should_error_on_nonexistent_shape() {
        let error =
            syntax_error("flowchart TB\n      A@{ shape: this-shape-does-not-exist }\n      ");

        assert!(
            error
                .message
                .contains("No such shape: this-shape-does-not-exist.")
        );
    }

    #[test]
    fn should_error_on_a_shape_name_that_is_only_an_object_property() {
        let error = syntax_error("flowchart TB\n      A@{ shape: constructor }\n      ");

        assert!(error.message.contains("No such shape: constructor."));
    }

    #[test]
    fn should_error_on_internal_only_shape() {
        // This shape does exist, but it is only supposed to be for internal/backwards compatibility use.
        let error = syntax_error("flowchart TB\n      A@{ shape: rect_left_inv_arrow }\n      ");

        assert!(
            error
                .message
                .contains("No such shape: rect_left_inv_arrow. Shape names should be lowercase.")
        );
    }

    #[test]
    fn diamond_shapes_should_work_as_usual() {
        let diagram = parse("flowchart TB\n      A{This is a label}\n").expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").shape.as_deref(), Some("diamond"));
        assert_eq!(vertex(&diagram, "A").text, "This is a label");
    }

    #[test]
    fn multi_line_strings_should_be_supported() {
        let diagram = parse(
            "flowchart TB\n      A@{\n        label: |\n          This is a\n          multiline string\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").text, "This is a\nmultiline string\n");
    }

    #[test]
    fn multi_line_strings_should_be_supported_2() {
        let diagram = parse(
            "flowchart TB\n      A@{\n        label: \"This is a\n          multiline string\"\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").text, "This is a<br/>multiline string");
    }

    #[test]
    fn should_be_possible_to_use_in_strings() {
        let diagram = parse(
            "flowchart TB\n      A@{\n        label: \"This is a string with }\"\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").text, "This is a string with }");
    }

    #[test]
    fn should_be_possible_to_use_in_strings_2() {
        let diagram = parse(
            "flowchart TB\n      A@{\n        label: \"This is a string with @\"\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").text, "This is a string with @");
    }

    #[test]
    fn should_be_possible_to_use_in_strings_3() {
        let diagram = parse(
            "flowchart TB\n      A@{\n        label: \"This is a string with}\"\n        other: \"clock\"\n     }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 1);
        assert_eq!(vertex(&diagram, "A").text, "This is a string with}");
    }

    #[test]
    fn should_be_possible_to_use_syntax_to_add_labels_on_multi_nodes() {
        let diagram = parse(
            "flowchart TB\n       n2[\"label for n2\"] &   n4@{ label: \"label for n4\"}   & n5@{ label: \"label for n5\"}\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 3);
        assert_eq!(
            texts(&diagram),
            ["label for n2", "label for n4", "label for n5"]
        );
    }

    #[test]
    fn should_be_possible_to_use_syntax_to_add_labels_on_multi_nodes_with_edge_link() {
        let diagram = parse(
            "flowchart TD\n    A[\"A\"] --> B[\"for B\"] &    C@{ label: \"for c\"} & E@{label : \"for E\"}\n    D@{label: \"for D\"}\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 5);
        assert_eq!(texts(&diagram), ["A", "for B", "for c", "for E", "for D"]);
    }

    #[test]
    fn should_be_possible_to_use_syntax_in_labels() {
        let diagram = parse(
            "flowchart TD\n    A[\"@A@\"] --> B[\"@for@ B@\"] & C@{ label: \"@for@ c@\"} & E{\"`@for@ E@`\"} & D((\"@for@ D@\"))\n      H1{{\"@for@ H@\"}}\n      H2{{\"`@for@ H@`\"}}\n      Q1{\"@for@ Q@\"}\n      Q2{\"`@for@ Q@`\"}\n      AS1>\"@for@ AS@\"]\n      AS2>\"`@for@ AS@`\"]\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 11);
        assert_eq!(
            texts(&diagram),
            [
                "@A@",
                "@for@ B@",
                "@for@ c@",
                "@for@ E@",
                "@for@ D@",
                "@for@ H@",
                "@for@ H@",
                "@for@ Q@",
                "@for@ Q@",
                "@for@ AS@",
                "@for@ AS@",
            ]
        );
    }

    #[test]
    fn should_handle_unique_edge_creation_with_using_and() {
        let diagram =
            parse("flowchart TD\n     A & B e1@--> C & D\n        A1 e2@--> C1 & D1\n      ")
                .expect("parses");

        assert_eq!(diagram.vertices.len(), 7);
        assert_eq!(diagram.edges.len(), 6);
        assert_eq!(diagram.edges[0].id, "L_A_C_0");
        assert_eq!(diagram.edges[1].id, "L_A_D_0");
        assert_eq!(diagram.edges[2].id, "e1");
        assert_eq!(diagram.edges[3].id, "L_B_D_0");
        assert_eq!(diagram.edges[4].id, "e2");
        assert_eq!(diagram.edges[5].id, "L_A1_D1_0");
    }

    #[test]
    fn should_handle_redefine_same_edge_ids_again() {
        let diagram =
            parse("flowchart TD\n     A & B e1@--> C & D\n        A1 e1@--> C1 & D1\n      ")
                .expect("parses");

        assert_eq!(diagram.vertices.len(), 7);
        assert_eq!(diagram.edges.len(), 6);
        assert_eq!(diagram.edges[0].id, "L_A_C_0");
        assert_eq!(diagram.edges[1].id, "L_A_D_0");
        assert_eq!(diagram.edges[2].id, "e1");
        assert_eq!(diagram.edges[3].id, "L_B_D_0");
        assert_eq!(diagram.edges[4].id, "L_A1_C1_0");
        assert_eq!(diagram.edges[5].id, "L_A1_D1_0");
    }

    #[test]
    fn should_handle_overriding_edge_animate_again() {
        let diagram = parse(
            "flowchart TD\n     A e1@--> B\n     C e2@--> D\n     E e3@--> F\n    e1@{ animate: true }\n    e2@{ animate: false }\n    e3@{ animate: true }\n    e3@{ animate: false }\n      ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 6);
        assert_eq!(diagram.edges.len(), 3);
        assert_eq!(diagram.edges[0].id, "e1");
        assert_eq!(diagram.edges[0].animate, Some(true));
        assert_eq!(diagram.edges[1].id, "e2");
        assert_eq!(diagram.edges[1].animate, Some(false));
        assert_eq!(diagram.edges[2].id, "e3");
        assert_eq!(diagram.edges[2].animate, Some(false));
    }

    #[test]
    #[ignore = "port: skipped upstream: skipped"]
    fn should_be_possible_to_use_syntax_to_add_labels_with_trail_spaces() {
        let diagram = parse(
            "flowchart TB\n       n2[\"label for n2\"] &   n4@{ label: \"label for n4\"}   & n5@{ label: \"label for n5\"} ",
        )
        .expect("parses");

        assert_eq!(diagram.vertices.len(), 3);
        assert_eq!(
            texts(&diagram),
            ["label for n2", "label for n4", "label for n5"]
        );
    }
}
