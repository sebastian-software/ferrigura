// DRAFT: not compiled or run yet. Target: crates/ferrigura-syntax/tests/flowchart_style_spec.rs

//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-style.spec.js`. Test names follow
//! the spec's titles in snake_case.
//!
//! Every test here is about styles, classes, `:::` or `linkStyle`, which the port reaches at step
//! 5. The bodies are written now so that step 5 only has to lift the ignore attributes.
//! The spec's `vi.spyOn(log, 'warn')` has no counterpart in the model; that test keeps the model
//! checks and drops the log check.

use ferrigura_model::flowchart::{FlowClass, FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::{FlowchartError, parse_grammar};

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex with id {id}"))
}

fn class<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowClass {
    diagram
        .classes
        .iter()
        .find(|class| class.id == id)
        .unwrap_or_else(|| panic!("no class with id {id}"))
}

/// The message of the syntax error of `text`. Panics when the text parses, or fails without a
/// syntax error.
fn syntax_error_message(text: &str) -> String {
    match parse_grammar(text) {
        Err(FlowchartError::Syntax(error)) => error.message,
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

mod style_when_parsing {
    use super::*;

    #[test]
    fn should_handle_styles_for_vertices() {
        let diagram = parse_grammar("graph TD;style Q background:#fff;").expect("parses");
        assert_eq!(vertex(&diagram, "Q").styles.len(), 1);
        assert_eq!(vertex(&diagram, "Q").styles[0], "background:#fff");
    }

    #[test]
    fn should_handle_multiple_styles_for_a_vortex() {
        let diagram = parse_grammar("graph TD;style R background:#fff,border:1px solid red;")
            .expect("parses");
        assert_eq!(vertex(&diagram, "R").styles.len(), 2);
        assert_eq!(vertex(&diagram, "R").styles[0], "background:#fff");
        assert_eq!(vertex(&diagram, "R").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_handle_multiple_styles_in_a_graph() {
        let diagram = parse_grammar(
            "graph TD;style S background:#aaa;\nstyle T background:#bbb,border:1px solid red;",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "S").styles.len(), 1);
        assert_eq!(vertex(&diagram, "T").styles.len(), 2);
        assert_eq!(vertex(&diagram, "S").styles[0], "background:#aaa");
        assert_eq!(vertex(&diagram, "T").styles[0], "background:#bbb");
        assert_eq!(vertex(&diagram, "T").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_handle_styles_and_graph_definitions_in_a_graph() {
        let diagram = parse_grammar(
            "graph TD;S-->T;\nstyle S background:#aaa;\nstyle T background:#bbb,border:1px solid red;",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "S").styles.len(), 1);
        assert_eq!(vertex(&diagram, "T").styles.len(), 2);
        assert_eq!(vertex(&diagram, "S").styles[0], "background:#aaa");
        assert_eq!(vertex(&diagram, "T").styles[0], "background:#bbb");
        assert_eq!(vertex(&diagram, "T").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_handle_styles_and_graph_definitions_in_a_graph_2() {
        let diagram = parse_grammar("graph TD;style T background:#bbb,border:1px solid red;")
            .expect("parses");
        assert_eq!(vertex(&diagram, "T").styles.len(), 2);
        assert_eq!(vertex(&diagram, "T").styles[0], "background:#bbb");
        assert_eq!(vertex(&diagram, "T").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_keep_node_label_text_if_already_defined_when_a_style_is_applied() {
        let diagram = parse_grammar(
            "graph TD;A(( ));B((Test));C;style A background:#fff;style D border:1px solid red;",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").text, "");
        assert_eq!(vertex(&diagram, "B").text, "Test");
        assert_eq!(vertex(&diagram, "C").text, "C");
        assert_eq!(vertex(&diagram, "D").text, "D");
    }

    #[test]
    fn should_be_possible_to_declare_a_class() {
        let diagram =
            parse_grammar("graph TD;classDef exClass background:#bbb,border:1px solid red;")
                .expect("parses");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_declare_multiple_classes() {
        let diagram = parse_grammar(
            "graph TD;classDef firstClass,secondClass background:#bbb,border:1px solid red;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "firstClass").styles.len(), 2);
        assert_eq!(class(&diagram, "firstClass").styles[0], "background:#bbb");
        assert_eq!(
            class(&diagram, "firstClass").styles[1],
            "border:1px solid red"
        );
        assert_eq!(class(&diagram, "secondClass").styles.len(), 2);
        assert_eq!(class(&diagram, "secondClass").styles[0], "background:#bbb");
        assert_eq!(
            class(&diagram, "secondClass").styles[1],
            "border:1px solid red"
        );
    }

    #[test]
    fn should_be_possible_to_declare_a_class_with_a_dot_in_the_style() {
        let diagram =
            parse_grammar("graph TD;classDef exClass background:#bbb,border:1.5px solid red;")
                .expect("parses");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(
            class(&diagram, "exClass").styles[1],
            "border:1.5px solid red"
        );
    }

    #[test]
    fn should_be_possible_to_declare_a_class_with_a_space_in_the_style() {
        let diagram =
            parse_grammar("graph TD;classDef exClass background:  #bbb,border:1.5px solid red;")
                .expect("parses");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:  #bbb");
        assert_eq!(
            class(&diagram, "exClass").styles[1],
            "border:1.5px solid red"
        );
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_vertex() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\na-->b;\nclass a exClass;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_vertex_with_an_id_containing() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\na_a-->b_b;\nclass a_a exClass;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_vertex_directly() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\na-->b[test]:::exClass;",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "b").classes[0], "exClass");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_vertex_directly_usecase_a_text_class() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\nb[test]:::exClass;",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "b").classes[0], "exClass");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_vertex_directly_usecase_a_text_class_b_test2() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\nA[test]:::exClass-->B[test2];",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").classes[0], "exClass");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_vertex_directly_2() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\na-->b[1 a a text!.]:::exClass;",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "b").classes[0], "exClass");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
    }

    #[test]
    fn should_be_possible_to_apply_a_class_to_a_comma_separated_list_of_vertices() {
        let diagram = parse_grammar(
            "graph TD;\nclassDef exClass background:#bbb,border:1px solid red;\na-->b;\nclass a,b exClass;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exClass").styles.len(), 2);
        assert_eq!(class(&diagram, "exClass").styles[0], "background:#bbb");
        assert_eq!(class(&diagram, "exClass").styles[1], "border:1px solid red");
        assert_eq!(vertex(&diagram, "a").classes[0], "exClass");
        assert_eq!(vertex(&diagram, "b").classes[0], "exClass");
    }

    #[test]
    fn should_handle_style_definitions_with_more_then_1_digit_in_a_row() {
        let diagram = parse_grammar(
            "graph TD\nA-->B1\nA-->B2\nA-->B3\nA-->B4\nA-->B5\nA-->B6\nA-->B7\nA-->B8\nA-->B9\nA-->B10\nA-->B11\nlinkStyle 10 stroke-width:1px;",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_style_definitions_within_number_of_edges() {
        // The message is a DB message, so its text is kept.
        let message =
            syntax_error_message("graph TD\n    A-->B\n    linkStyle 1 stroke-width:1px;");
        assert!(
            message.contains(
                "The index 1 for linkStyle is out of bounds. Valid indices for linkStyle are between 0 and 0."
            ),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn should_handle_style_definitions_within_number_of_edges_2() {
        let diagram = parse_grammar("graph TD\n    A-->B\n    linkStyle 0 stroke-width:1px;")
            .expect("parses");
        let style = diagram.edges[0]
            .style
            .as_ref()
            .expect("the edge has a style");
        assert_eq!(style[0], "stroke-width:1px");
    }

    #[test]
    fn should_handle_multi_numbered_style_definitions_with_more_then_1_digit_in_a_row() {
        let diagram = parse_grammar(
            "graph TD\nA-->B1\nA-->B2\nA-->B3\nA-->B4\nA-->B5\nA-->B6\nA-->B7\nA-->B8\nA-->B9\nA-->B10\nA-->B11\nA-->B12\nlinkStyle 10,11 stroke-width:1px;",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_classdefs_with_style_in_classes() {
        let diagram =
            parse_grammar("graph TD\nA-->B\nclassDef exClass font-style:bold;").expect("parses");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_classdefs_with_in_classes() {
        let diagram = parse_grammar(
            "graph TD\nA-->B\nclassDef exClass fill:#f96,stroke:#333,stroke-width:4px,font-size:50%,font-style:bold;",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_warn_when_style_targets_a_non_existent_node() {
        // The reference logs a warning here. The model has no log, so only the model is checked.
        let diagram = parse_grammar("graph TD;\nAA-->BB;\nstyle A fill:#f00;").expect("parses");
        // Node A still exists for backward compatibility.
        assert_eq!(vertex(&diagram, "A").styles[0], "fill:#f00");
        // AA and BB exist from the edge definition.
        assert_eq!(vertex(&diagram, "AA").id, "AA");
        assert_eq!(vertex(&diagram, "BB").id, "BB");
    }

    #[test]
    fn should_handle_multiple_vertices_with_style() {
        let diagram = parse_grammar(
            "\n    graph TD\n      classDef C1 stroke-dasharray:4\n      classDef C2 stroke-dasharray:6\n      A & B:::C1 & D:::C1 --> E:::C2\n    ",
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").classes.len(), 0);
        assert_eq!(vertex(&diagram, "B").classes[0], "C1");
        assert_eq!(vertex(&diagram, "D").classes[0], "C1");
        assert_eq!(vertex(&diagram, "E").classes[0], "C2");
    }
}
