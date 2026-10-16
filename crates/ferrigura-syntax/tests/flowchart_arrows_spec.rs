//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-arrows.spec.js`. Test names follow
//! the spec's titles in snake_case.

use ferrigura_model::flowchart::{FlowEdge, FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::parse;

/// The vertex with the given id. Panics with the id when there is none.
fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex {id}"))
}

/// The checks of the single-edge tests: vertices A and B exist, and there is one edge from A to B.
fn single_edge_a_to_b(diagram: &FlowchartDiagram) -> &FlowEdge {
    assert_eq!(vertex(diagram, "A").id, "A");
    assert_eq!(vertex(diagram, "B").id, "B");
    assert_eq!(diagram.edges.len(), 1);
    let edge = &diagram.edges[0];
    assert_eq!(edge.start, "A");
    assert_eq!(edge.end, "B");
    edge
}

/// Checks the single edge from A to B: its type, text, stroke and length.
fn check_edge(diagram: &FlowchartDiagram, edge_type: &str, text: &str, stroke: &str, length: u32) {
    let edge = single_edge_a_to_b(diagram);
    assert_eq!(edge.edge_type, edge_type);
    assert_eq!(edge.text, text);
    assert_eq!(edge.stroke, stroke);
    assert_eq!(edge.length, Some(length));
}

mod arrows_when_parsing {
    use super::*;

    #[test]
    fn should_handle_a_nodes_and_edges() {
        let diagram = parse("graph TD;\nA-->B;").expect("parses");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_angle_bracket_as_direction_lr() {
        let diagram = parse("graph >;A-->B;").expect("parses");
        assert_eq!(diagram.direction, "LR");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_angle_bracket_as_direction_rl() {
        let diagram = parse("graph <;A-->B;").expect("parses");
        assert_eq!(diagram.direction, "RL");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_caret_as_direction_bt() {
        let diagram = parse("graph ^;A-->B;").expect("parses");
        assert_eq!(diagram.direction, "BT");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_lower_case_v_as_direction_tb() {
        let diagram = parse("graph v;A-->B;").expect("parses");
        assert_eq!(diagram.direction, "TB");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_a_nodes_and_edges_and_a_space_between_link_and_node() {
        let diagram = parse("graph TD;A --> B;").expect("parses");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_a_nodes_and_edges_a_space_between_link_and_node_and_each_line_ending_without_semicolon()
     {
        let diagram = parse("graph TD\nA --> B\n style e red").expect("parses");
        check_edge(&diagram, "arrow_point", "", "normal", 1);
    }

    #[test]
    fn should_handle_statements_ending_without_semicolon() {
        let diagram = parse("graph TD\nA-->B\nB-->C").expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[1].start, "B");
        assert_eq!(diagram.edges[1].end, "C");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
        assert_eq!(diagram.edges[0].text, "");
        assert_eq!(diagram.edges[0].stroke, "normal");
        assert_eq!(diagram.edges[0].length, Some(1));
    }

    mod it_should_handle_multi_directional_arrows {
        mod point {
            use crate::*;

            #[test]
            fn should_handle_double_edged_nodes_and_edges() {
                let diagram = parse("graph TD;\nA<-->B;").expect("parses");
                check_edge(&diagram, "double_arrow_point", "", "normal", 1);
            }

            #[test]
            fn should_handle_double_edged_nodes_with_text() {
                let diagram = parse("graph TD;\nA<-- text -->B;").expect("parses");
                check_edge(&diagram, "double_arrow_point", "text", "normal", 1);
            }

            #[test]
            fn should_handle_double_edged_nodes_and_edges_on_thick_arrows() {
                let diagram = parse("graph TD;\nA<==>B;").expect("parses");
                check_edge(&diagram, "double_arrow_point", "", "thick", 1);
            }

            #[test]
            fn should_handle_double_edged_nodes_with_text_on_thick_arrows() {
                let diagram = parse("graph TD;\nA<== text ==>B;").expect("parses");
                check_edge(&diagram, "double_arrow_point", "text", "thick", 1);
            }

            #[test]
            fn should_handle_double_edged_nodes_and_edges_on_dotted_arrows() {
                let diagram = parse("graph TD;\nA<-.->B;").expect("parses");
                check_edge(&diagram, "double_arrow_point", "", "dotted", 1);
            }

            #[test]
            fn should_handle_double_edged_nodes_with_text_on_dotted_arrows() {
                let diagram = parse("graph TD;\nA<-. text .->B;").expect("parses");
                check_edge(&diagram, "double_arrow_point", "text", "dotted", 1);
            }
        }
    }
}
