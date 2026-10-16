//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-comments.spec.js`. Test names follow
//! the spec's titles in snake_case. The spec passes its text through `cleanupComments` first; the
//! Rust `parse` does that step itself, so the raw text is passed.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::parse;

/// The vertex with the given id. Panics with the id when there is none.
fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex {id}"))
}

/// Checks that the diagram has the vertices A and B and one arrow from A to B with no text.
fn assert_single_arrow_a_to_b(diagram: &FlowchartDiagram) {
    assert_eq!(vertex(diagram, "A").id, "A");
    assert_eq!(vertex(diagram, "B").id, "B");
    assert_eq!(diagram.edges.len(), 1);
    let edge = &diagram.edges[0];
    assert_eq!(edge.start, "A");
    assert_eq!(edge.end, "B");
    assert_eq!(edge.edge_type, "arrow_point");
    assert_eq!(edge.text, "");
}

mod comments_when_parsing {
    use super::*;

    #[test]
    fn should_handle_comments() {
        let diagram = parse("graph TD;\n%% Comment\n A-->B;").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_comments_at_the_start() {
        let diagram = parse("%% Comment\ngraph TD;\n A-->B;").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_comments_at_the_end() {
        let diagram = parse("graph TD;\n A-->B\n %% Comment at the end\n").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_comments_at_the_end_no_trailing_newline() {
        let diagram = parse("graph TD;\n A-->B\n%% Comment").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_comments_at_the_end_many_trailing_newlines() {
        let diagram = parse("graph TD;\n A-->B\n%% Comment\n\n\n").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_no_trailing_newlines() {
        let diagram = parse("graph TD;\n A-->B").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_many_trailing_newlines() {
        let diagram = parse("graph TD;\n A-->B\n\n").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_a_comment_with_blank_rows_in_between() {
        let diagram = parse("graph TD;\n\n\n %% Comment\n A-->B;").expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }

    #[test]
    fn should_handle_a_comment_with_mermaid_flowchart_code_in_them() {
        let diagram =
            parse("graph TD;\n\n\n %% Test od>Odd shape]-->|Two line<br>edge comment|ro;\n A-->B;")
                .expect("parses");
        assert_single_arrow_a_to_b(&diagram);
    }
}
