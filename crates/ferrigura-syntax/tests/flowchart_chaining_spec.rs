//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-vertice-chaining.spec.js`. Test
//! names follow the spec's titles in snake_case. Where two titles are the same, the later test gets
//! a `_2` suffix.

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

/// Checks that edge `index` runs from `start` to `end` as an arrow with no text.
fn assert_arrow(diagram: &FlowchartDiagram, index: usize, start: &str, end: &str) {
    let edge = &diagram.edges[index];
    assert_eq!(edge.start, start);
    assert_eq!(edge.end, end);
    assert_eq!(edge.edge_type, "arrow_point");
    assert_eq!(edge.text, "");
}

mod when_parsing_flowcharts {
    use super::*;

    #[test]
    fn should_handle_chaining_of_vertices() {
        let diagram = parse(
            r#"
    graph TD
      A-->B-->C;
    "#,
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(diagram.edges.len(), 2);
        assert_arrow(&diagram, 0, "A", "B");
        assert_arrow(&diagram, 1, "B", "C");
    }

    #[test]
    fn should_handle_chaining_of_vertices_2() {
        let diagram = parse(
            r#"
    graph TD
      A & B --> C;
    "#,
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(diagram.edges.len(), 2);
        assert_arrow(&diagram, 0, "A", "C");
        assert_arrow(&diagram, 1, "B", "C");
    }

    #[test]
    fn should_multiple_vertices_in_link_statement_in_the_beginning() {
        let diagram = parse(
            r#"
    graph TD
      A-->B & C;
    "#,
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(diagram.edges.len(), 2);
        assert_arrow(&diagram, 0, "A", "B");
        assert_arrow(&diagram, 1, "A", "C");
    }

    #[test]
    fn should_multiple_vertices_in_link_statement_at_the_end() {
        let diagram = parse(
            r#"
    graph TD
      A & B--> C & D;
    "#,
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(vertex(&diagram, "D").id, "D");
        assert_eq!(diagram.edges.len(), 4);
        assert_arrow(&diagram, 0, "A", "C");
        assert_arrow(&diagram, 1, "A", "D");
        assert_arrow(&diagram, 2, "B", "C");
        assert_arrow(&diagram, 3, "B", "D");
    }

    #[test]
    fn should_handle_chaining_of_vertices_at_both_ends_at_once() {
        let diagram = parse(
            r#"
    graph TD
      A & B--> C & D;
    "#,
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(vertex(&diagram, "D").id, "D");
        assert_eq!(diagram.edges.len(), 4);
        assert_arrow(&diagram, 0, "A", "C");
        assert_arrow(&diagram, 1, "A", "D");
        assert_arrow(&diagram, 2, "B", "C");
        assert_arrow(&diagram, 3, "B", "D");
    }

    #[test]
    fn should_handle_chaining_and_multiple_nodes_in_link_statement_fvc() {
        let diagram = parse(
            r#"
    graph TD
      A --> B & B2 & C --> D2;
    "#,
        )
        .expect("parses");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "B2").id, "B2");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(vertex(&diagram, "D2").id, "D2");
        assert_eq!(diagram.edges.len(), 6);
        assert_arrow(&diagram, 0, "A", "B");
        assert_arrow(&diagram, 1, "A", "B2");
        assert_arrow(&diagram, 2, "A", "C");
        assert_arrow(&diagram, 3, "B", "D2");
        assert_arrow(&diagram, 4, "B2", "D2");
        assert_arrow(&diagram, 5, "C", "D2");
    }

    #[test]
    fn should_handle_chaining_and_multiple_nodes_in_link_statement_with_extra_info_in_statements() {
        let diagram = parse(
            r#"
    graph TD
      A[ h ] -- hello --> B[" test "]:::exClass & C --> D;
      classDef exClass background:#bbb,border:1px solid red;
    "#,
        )
        .expect("parses");
        let class = diagram
            .classes
            .iter()
            .find(|class| class.id == "exClass")
            .expect("class exClass");
        assert_eq!(class.styles.len(), 2);
        assert_eq!(class.styles[0], "background:#bbb");
        assert_eq!(class.styles[1], "border:1px solid red");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "B").classes[0], "exClass");
        assert_eq!(vertex(&diagram, "C").id, "C");
        assert_eq!(vertex(&diagram, "D").id, "D");
        assert_eq!(diagram.edges.len(), 4);
        assert_eq!(diagram.edges[0].start, "A");
        assert_eq!(diagram.edges[0].end, "B");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
        assert_eq!(diagram.edges[0].text, "hello");
        assert_eq!(diagram.edges[1].start, "A");
        assert_eq!(diagram.edges[1].end, "C");
        assert_eq!(diagram.edges[1].edge_type, "arrow_point");
        assert_eq!(diagram.edges[1].text, "hello");
        assert_arrow(&diagram, 2, "B", "D");
        assert_arrow(&diagram, 3, "C", "D");
    }
}
