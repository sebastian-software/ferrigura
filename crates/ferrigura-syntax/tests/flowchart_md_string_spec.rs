// DRAFT: not compiled or run yet. Target: crates/ferrigura-syntax/tests/flowchart_md_string_spec.rs

//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-md-string.spec.js`. Test names
//! follow the spec's titles in snake_case.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram, LabelType};
use ferrigura_syntax::flowchart::parse_grammar;

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex with id {id}"))
}

mod parsing_a_flow_chart_with_markdown_strings {
    use super::*;

    #[test]
    fn markdown_formatting_in_nodes_and_labels() {
        let diagram = parse_grammar(
            "flowchart\nA[\"`The cat in **the** hat`\"]-- \"`The *bat* in the chat`\" -->B[\"The dog in the hog\"] -- \"The rat in the mat\" -->C;",
        )
        .expect("parses");

        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "A").text, "The cat in **the** hat");
        assert_eq!(vertex(&diagram, "A").label_type, LabelType::Markdown);
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(vertex(&diagram, "B").text, "The dog in the hog");
        assert_eq!(vertex(&diagram, "B").label_type, LabelType::String);
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[0].start, "A");
        assert_eq!(diagram.edges[0].end, "B");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
        assert_eq!(diagram.edges[0].text, "The *bat* in the chat");
        assert_eq!(diagram.edges[0].label_type, LabelType::Markdown);
        assert_eq!(diagram.edges[1].start, "B");
        assert_eq!(diagram.edges[1].end, "C");
        assert_eq!(diagram.edges[1].edge_type, "arrow_point");
        assert_eq!(diagram.edges[1].text, "The rat in the mat");
        assert_eq!(diagram.edges[1].label_type, LabelType::String);
    }

    #[test]
    fn markdown_formatting_in_subgraphs() {
        let diagram = parse_grammar(
            "flowchart LR\nsubgraph \"One\"\n  a(\"`The **cat**\n  in the hat`\") -- \"1o\" --> b{{\"`The **dog** in the hog`\"}}\nend\nsubgraph \"`**Two**`\"\n  c(\"`The **cat**\n  in the hat`\") -- \"`1o **ipa**`\" --> d(\"The dog in the hog\")\nend",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 2);
        let subgraph = &diagram.subgraphs[0];

        assert_eq!(subgraph.nodes.len(), 2);
        assert_eq!(subgraph.title, "One");
        assert_eq!(subgraph.label_type, LabelType::Text);

        let subgraph2 = &diagram.subgraphs[1];
        assert_eq!(subgraph2.nodes.len(), 2);
        assert_eq!(subgraph2.title, "**Two**");
        assert_eq!(subgraph2.label_type, LabelType::Markdown);
    }
}
