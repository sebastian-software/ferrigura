//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-direction.spec.js`. Test names follow
//! the spec's titles in snake_case. The subgraph checks read the model's `subgraphs`, and the
//! subgraph direction is its `dir`.

use ferrigura_model::flowchart::{FlowSubgraph, FlowchartDiagram};
use ferrigura_syntax::flowchart::parse;

/// The subgraph with the given id. Panics with the id when there is none.
fn subgraph<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowSubgraph {
    diagram
        .subgraphs
        .iter()
        .find(|subgraph| subgraph.id == id)
        .unwrap_or_else(|| panic!("no subgraph {id}"))
}

mod when_parsing_directions {
    use super::*;

    #[test]
    fn should_use_default_direction_from_top_level() {
        let diagram = parse(
            r#"flowchart TB
    subgraph A
      a --> b
    end"#,
        )
        .expect("parses");
        assert_eq!(diagram.subgraphs.len(), 1);
        let subgraph = &diagram.subgraphs[0];
        assert_eq!(subgraph.nodes.len(), 2);
        assert_eq!(subgraph.nodes[0], "b");
        assert_eq!(subgraph.nodes[1], "a");
        assert_eq!(subgraph.id, "A");
        assert_eq!(subgraph.dir, None);
    }

    #[test]
    fn should_handle_a_subgraph_with_a_direction() {
        let diagram = parse(
            r#"flowchart TB
    subgraph A
      direction BT
      a --> b
    end"#,
        )
        .expect("parses");
        assert_eq!(diagram.subgraphs.len(), 1);
        let subgraph = &diagram.subgraphs[0];
        assert_eq!(subgraph.nodes.len(), 2);
        assert_eq!(subgraph.nodes[0], "b");
        assert_eq!(subgraph.nodes[1], "a");
        assert_eq!(subgraph.id, "A");
        assert_eq!(subgraph.dir.as_deref(), Some("BT"));
    }

    #[test]
    fn should_use_the_last_defined_direction() {
        let diagram = parse(
            r#"flowchart TB
    subgraph A
      direction BT
      a --> b
      direction RL
    end"#,
        )
        .expect("parses");
        assert_eq!(diagram.subgraphs.len(), 1);
        let subgraph = &diagram.subgraphs[0];
        assert_eq!(subgraph.nodes.len(), 2);
        assert_eq!(subgraph.nodes[0], "b");
        assert_eq!(subgraph.nodes[1], "a");
        assert_eq!(subgraph.id, "A");
        assert_eq!(subgraph.dir.as_deref(), Some("RL"));
    }

    #[test]
    fn should_handle_nested_subgraphs_1() {
        let diagram = parse(
            r#"flowchart TB
    subgraph A
      direction RL
      b-->B
      a
    end
    a-->c
    subgraph B
      direction LR
      c
    end"#,
        )
        .expect("parses");
        assert_eq!(diagram.subgraphs.len(), 2);

        let subgraph_a = subgraph(&diagram, "A");
        let subgraph_b = subgraph(&diagram, "B");

        assert_eq!(subgraph_b.nodes[0], "c");
        assert_eq!(subgraph_b.dir.as_deref(), Some("LR"));
        assert!(subgraph_a.nodes.iter().any(|node| node == "B"));
        assert!(subgraph_a.nodes.iter().any(|node| node == "b"));
        assert!(subgraph_a.nodes.iter().any(|node| node == "a"));
        assert!(!subgraph_a.nodes.iter().any(|node| node == "c"));
        assert_eq!(subgraph_a.dir.as_deref(), Some("RL"));
    }
}
