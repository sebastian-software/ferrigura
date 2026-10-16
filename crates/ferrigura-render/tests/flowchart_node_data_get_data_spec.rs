//! Port of the `getData` expectations of `packages/mermaid/src/diagrams/flowchart/parser/flow-node-data.spec.js`.
//! The spec's parser tests run in `ferrigura-syntax/tests/flowchart_node_data_spec.rs`, which checks
//! the model; this file checks what `getData()` returns for the same texts: the nodes in their
//! order, their labels and shapes, and the edges with their ids, animation and count.
//!
//! The error tests of the spec have no `getData` and are not repeated here. Test names follow the
//! spec's titles in snake_case, as the syntax crate's port does; a repeated title gets `_2`, `_3`, ….

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::flowchart::data::{LayoutData, get_data};
use ferrigura_syntax::flowchart::parse;

/// The layout data of a flowchart text, with the configuration the spec's DB has.
fn layout(text: &str) -> LayoutData {
    let diagram = parse(text).expect("parses");
    get_data(&diagram, &MermaidConfig::default())
}

mod when_parsing_directions {
    use super::*;

    #[test]
    fn should_handle_basic_shape_data_statements() {
        let data = layout("flowchart TB\n      D@{ shape: rounded}");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_2() {
        let data = layout("flowchart TB\n      D@{ shape: rounded }");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with() {
        let data = layout("flowchart TB\n      D@{ shape: rounded } & E");

        assert_eq!(data.nodes.len(), 2);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_handle_shape_data_statements_with_edges() {
        let data = layout("flowchart TB\n      D@{ shape: rounded } --> E");

        assert_eq!(data.nodes.len(), 2);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_1() {
        let data = layout("flowchart TB\n      D@{ shape: rounded } & E --> F");

        assert_eq!(data.nodes.len(), 3);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_2() {
        let data = layout("flowchart TB\n      D@{ shape: rounded } & E@{ shape: rounded } --> F");

        assert_eq!(data.nodes.len(), 3);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_3() {
        let data = layout(
            "flowchart TB\n      D@{ shape: rounded } & E@{ shape: rounded } --> F & G@{ shape: rounded }",
        );

        assert_eq!(data.nodes.len(), 4);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_4() {
        let data = layout(
            "flowchart TB\n      D@{ shape: rounded } & E@{ shape: rounded } --> F@{ shape: rounded } & G@{ shape: rounded }",
        );

        assert_eq!(data.nodes.len(), 4);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_handle_basic_shape_data_statements_with_amp_and_edges_5_trailing_space() {
        let data = layout(
            "flowchart TB\n         D@{ shape: rounded } & E@{ shape: rounded } --> F{ shape: rounded } & G{ shape: rounded }    ",
        );

        assert_eq!(data.nodes.len(), 4);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
        assert_eq!(data.nodes[1].label, "E");
    }

    #[test]
    fn should_no_matter_of_there_are_no_leading_spaces() {
        let data = layout("flowchart TB\n      D@{shape: rounded}");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
    }

    #[test]
    fn should_no_matter_of_there_are_many_leading_spaces() {
        let data = layout("flowchart TB\n      D@{       shape: rounded}");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
    }

    #[test]
    fn should_be_forgiving_with_many_spaces_before_the_end() {
        let data = layout("flowchart TB\n      D@{ shape: rounded         }");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "D");
    }

    #[test]
    fn should_be_possible_to_add_multiple_properties_on_the_same_line() {
        let data = layout("flowchart TB\n      D@{ shape: rounded , label: \"DD\"}");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "rounded");
        assert_eq!(data.nodes[0].label, "DD");
    }

    #[test]
    fn should_be_possible_to_link_to_a_node_with_more_data() {
        let data = layout(
            "flowchart TB\n      A --> D@{\n        shape: circle\n        other: \"clock\"\n     }\n\n      ",
        );

        assert_eq!(data.nodes.len(), 2);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "A");
        assert_eq!(data.nodes[1].label, "D");
        assert_eq!(data.nodes[1].shape, "circle");

        assert_eq!(data.edges.len(), 1);
    }

    #[test]
    fn should_not_disturb_adding_multiple_nodes_after_each_other() {
        let data = layout(
            "flowchart TB\n      A[hello]\n      B@{\n        shape: circle\n        other: \"clock\"\n     }\n      C[Hello]@{\n        shape: circle\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 3);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "hello");
        assert_eq!(data.nodes[1].shape, "circle");
        assert_eq!(data.nodes[1].label, "B");
        assert_eq!(data.nodes[2].shape, "circle");
        assert_eq!(data.nodes[2].label, "Hello");
    }

    #[test]
    fn should_use_handle_bracket_end_character_inside_the_shape_data() {
        let data = layout(
            "flowchart TB\n      A@{\n        label: \"This is }\"\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "This is }");
    }

    #[test]
    fn diamond_shapes_should_work_as_usual() {
        let data = layout("flowchart TB\n      A{This is a label}\n");

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "diamond");
        assert_eq!(data.nodes[0].label, "This is a label");
    }

    #[test]
    fn multi_line_strings_should_be_supported() {
        let data = layout(
            "flowchart TB\n      A@{\n        label: |\n          This is a\n          multiline string\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "This is a\nmultiline string\n");
    }

    #[test]
    fn multi_line_strings_should_be_supported_2() {
        let data = layout(
            "flowchart TB\n      A@{\n        label: \"This is a\n          multiline string\"\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "This is a<br/>multiline string");
    }

    #[test]
    fn should_be_possible_to_use_in_strings() {
        let data = layout(
            "flowchart TB\n      A@{\n        label: \"This is a string with }\"\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "This is a string with }");
    }

    #[test]
    fn should_be_possible_to_use_in_strings_2() {
        let data = layout(
            "flowchart TB\n      A@{\n        label: \"This is a string with @\"\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "This is a string with @");
    }

    #[test]
    fn should_be_possible_to_use_in_strings_3() {
        let data = layout(
            "flowchart TB\n      A@{\n        label: \"This is a string with}\"\n        other: \"clock\"\n     }\n      ",
        );

        assert_eq!(data.nodes.len(), 1);
        assert_eq!(data.nodes[0].shape, "squareRect");
        assert_eq!(data.nodes[0].label, "This is a string with}");
    }

    #[test]
    fn should_be_possible_to_use_syntax_to_add_labels_on_multi_nodes() {
        let data = layout(
            "flowchart TB\n       n2[\"label for n2\"] &   n4@{ label: \"label for n4\"}   & n5@{ label: \"label for n5\"}\n      ",
        );

        assert_eq!(data.nodes.len(), 3);
        assert_eq!(data.nodes[0].label, "label for n2");
        assert_eq!(data.nodes[1].label, "label for n4");
        assert_eq!(data.nodes[2].label, "label for n5");
    }

    #[test]
    fn should_be_possible_to_use_syntax_to_add_labels_on_multi_nodes_with_edge_link() {
        let data = layout(
            "flowchart TD\n    A[\"A\"] --> B[\"for B\"] &    C@{ label: \"for c\"} & E@{label : \"for E\"}\n    D@{label: \"for D\"}\n      ",
        );

        assert_eq!(data.nodes.len(), 5);
        assert_eq!(data.nodes[0].label, "A");
        assert_eq!(data.nodes[1].label, "for B");
        assert_eq!(data.nodes[2].label, "for c");
        assert_eq!(data.nodes[3].label, "for E");
        assert_eq!(data.nodes[4].label, "for D");
    }

    #[test]
    fn should_be_possible_to_use_syntax_in_labels() {
        let data = layout(
            "flowchart TD\n    A[\"@A@\"] --> B[\"@for@ B@\"] & C@{ label: \"@for@ c@\"} & E{\"`@for@ E@`\"} & D((\"@for@ D@\"))\n      H1{{\"@for@ H@\"}}\n      H2{{\"`@for@ H@`\"}}\n      Q1{\"@for@ Q@\"}\n      Q2{\"`@for@ Q@`\"}\n      AS1>\"@for@ AS@\"]\n      AS2>\"`@for@ AS@`\"]\n      ",
        );

        assert_eq!(data.nodes.len(), 11);
        let labels: Vec<&str> = data.nodes.iter().map(|node| node.label.as_str()).collect();
        assert_eq!(
            labels,
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
        let data =
            layout("flowchart TD\n     A & B e1@--> C & D\n        A1 e2@--> C1 & D1\n      ");

        assert_eq!(data.nodes.len(), 7);
        assert_eq!(data.edges.len(), 6);
        let ids: Vec<&str> = data.edges.iter().map(|edge| edge.id.as_str()).collect();
        assert_eq!(ids[0], "L_A_C_0");
        assert_eq!(ids[1], "L_A_D_0");
        assert_eq!(ids[2], "e1");
        assert_eq!(ids[3], "L_B_D_0");
        assert_eq!(ids[4], "e2");
        assert_eq!(ids[5], "L_A1_D1_0");
    }

    #[test]
    fn should_handle_redefine_same_edge_ids_again() {
        let data =
            layout("flowchart TD\n     A & B e1@--> C & D\n        A1 e1@--> C1 & D1\n      ");

        assert_eq!(data.nodes.len(), 7);
        assert_eq!(data.edges.len(), 6);
        let ids: Vec<&str> = data.edges.iter().map(|edge| edge.id.as_str()).collect();
        assert_eq!(ids[0], "L_A_C_0");
        assert_eq!(ids[1], "L_A_D_0");
        assert_eq!(ids[2], "e1");
        assert_eq!(ids[3], "L_B_D_0");
        assert_eq!(ids[4], "L_A1_C1_0");
        assert_eq!(ids[5], "L_A1_D1_0");
    }

    #[test]
    fn should_handle_overriding_edge_animate_again() {
        let data = layout(
            "flowchart TD\n     A e1@--> B\n     C e2@--> D\n     E e3@--> F\n    e1@{ animate: true }\n    e2@{ animate: false }\n    e3@{ animate: true }\n    e3@{ animate: false }\n      ",
        );

        assert_eq!(data.nodes.len(), 6);
        assert_eq!(data.edges.len(), 3);
        assert_eq!(data.edges[0].id, "e1");
        assert_eq!(data.edges[0].animate, Some(true));
        assert_eq!(data.edges[1].id, "e2");
        assert_eq!(data.edges[1].animate, Some(false));
        assert_eq!(data.edges[2].id, "e3");
        assert_eq!(data.edges[2].animate, Some(false));
    }

    #[test]
    #[ignore = "port: skipped upstream (it.skip in flow-node-data.spec.js)"]
    fn should_be_possible_to_use_syntax_to_add_labels_with_trail_spaces() {
        let data = layout(
            "flowchart TB\n       n2[\"label for n2\"] &   n4@{ label: \"label for n4\"}   & n5@{ label: \"label for n5\"} ",
        );

        assert_eq!(data.nodes.len(), 3);
        assert_eq!(data.nodes[0].label, "label for n2");
        assert_eq!(data.nodes[1].label, "label for n4");
        assert_eq!(data.nodes[2].label, "label for n5");
    }
}
