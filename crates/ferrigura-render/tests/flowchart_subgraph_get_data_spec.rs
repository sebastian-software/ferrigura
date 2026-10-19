//! Port of the `getData` expectation of `packages/mermaid/src/diagrams/flowchart/parser/subgraph.spec.js`:
//! a subgraph with `@{ view: collapsed }` is drawn as one collapsed node, its members are hidden,
//! and the edges that cross its border are redirected to it. The parser's part of the test is in
//! `ferrigura-syntax/tests/flowchart_subgraph_spec.rs`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::context::Globals;
use ferrigura_render::flowchart::data::{LayoutData, get_data};
use ferrigura_render::unified::Section;
use ferrigura_syntax::flowchart::parse;

/// The layout data of a flowchart text, with the configuration the spec's DB has.
fn layout(text: &str) -> LayoutData {
    let diagram = parse(text).expect("parses");
    let config = MermaidConfig::default();
    get_data(&diagram, &Section::of(&config), &Globals::of(&config))
}

mod when_parsing_subgraphs {
    use super::*;

    #[test]
    fn should_attach_metadata_to_a_subgraph_and_collapse_it() {
        let data = layout(
            "\n      graph TD\n        subgraph one [My Group]\n          A --> B\n        end\n        C --> A\n        one@{ view: collapsed }\n    ",
        );

        let collapsed = data
            .nodes
            .iter()
            .find(|node| node.id == "one")
            .expect("one is drawn");
        assert_eq!(collapsed.shape, "collapsedGroup");
        assert!(!collapsed.is_group);
        assert!(data.nodes.iter().all(|node| node.id != "A"));
        assert!(data.nodes.iter().all(|node| node.id != "B"));
        assert!(
            data.edges
                .iter()
                .any(|edge| edge.start == "C" && edge.end == "one")
        );
        assert!(data.edges.iter().all(|edge| edge.end != "A"));
    }
}
