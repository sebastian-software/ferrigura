//! Port of `packages/mermaid/src/diagrams/er/parser/subgraph.spec.js`. Test names follow the spec's
//! titles in snake_case. The spec's `beforeEach` (a fresh DB) is the fresh parse of each test.

use ferrigura_model::er::{ErDiagram, ErSubgraph};
use ferrigura_syntax::er::parse_grammar;

fn parse(text: &str) -> ErDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("{text:?} does not parse: {error}"))
}

fn subgraph<'a>(diagram: &'a ErDiagram, id: &str) -> &'a ErSubgraph {
    diagram
        .subgraphs
        .iter()
        .find(|subgraph| subgraph.id == id)
        .unwrap_or_else(|| panic!("no subgraph {id}"))
}

fn entity_id(diagram: &ErDiagram, name: &str) -> String {
    diagram
        .entities
        .iter()
        .find(|entity| entity.name == name)
        .unwrap_or_else(|| panic!("no entity {name}"))
        .id
        .clone()
}

mod when_parsing_er_subgraphs {
    use super::*;

    #[test]
    fn should_handle_subgraph_with_simple_nodes_and_id_title() {
        let diagram = parse("erDiagram\nsubgraph Group\nA\nB\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes, ["A", "B"]);
        assert_eq!(group.title, "Group");
        assert_eq!(group.id, "Group");
    }

    #[test]
    fn should_handle_subgraph_with_multiple_words_in_id_title() {
        let diagram = parse("erDiagram\nsubgraph \"Some Title\"\nA\nB\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.title, "Some Title");
        assert_eq!(group.id, "Some Title");
    }

    #[test]
    fn should_handle_subgraph_with_id_and_title_notation() {
        let diagram = parse("erDiagram\nsubgraph id [Title]\nA\nB\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.title, "Title");
        assert_eq!(group.id, "id");
    }

    #[test]
    fn should_handle_special_characters_in_subgraph_title() {
        let diagram = parse("erDiagram\nsubgraph \"Group 1.5\"\nA\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes.len(), 1);
        assert_eq!(group.title, "Group 1.5");
        assert_eq!(group.id, "Group 1.5");
    }

    #[test]
    fn should_handle_empty_lines_inside_subgraph() {
        let diagram = parse("erDiagram\nsubgraph Group\n\nA\n\nB\n\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes, ["A", "B"]);
        assert_eq!(group.title, "Group");
        assert_eq!(group.id, "Group");
    }

    #[test]
    fn should_ignore_redundant_whitespace_around_end() {
        let diagram = parse("erDiagram\nsubgraph Group\nA\nend   \n");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes, ["A"]);
        assert_eq!(group.title, "Group");
        assert_eq!(group.id, "Group");
    }

    #[test]
    fn should_handle_empty_subgraph() {
        let diagram = parse("erDiagram\nsubgraph empty\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert!(group.nodes.is_empty());
        assert_eq!(group.title, "empty");
        assert_eq!(group.id, "empty");
    }

    #[test]
    fn should_correctly_parse_direction_rl_inside_a_subgraph() {
        let diagram = parse("erDiagram subgraph WithRL\ndirection RL\nA\nB\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.dir.as_deref(), Some("RL"));
        assert_eq!(group.nodes[0], "A");
        assert_eq!(group.nodes[1], "B");
    }

    #[test]
    fn should_include_both_endpoints_of_a_relationship_inside_a_subgraph() {
        let diagram = parse("erDiagram\nsubgraph Group\nA }|--|| B : \"relates to\"\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes, ["A", "B"]);
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].entity_a, entity_id(&diagram, "A"));
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
    }

    #[test]
    fn should_handle_relationship_between_subgraph_entity_and_external_entity() {
        let diagram = parse("erDiagram\nsubgraph Group\nA\nend\nA }|--|| B : \"relates to\"\n");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes, ["A"]);
        assert!(!group.nodes.contains(&"B".to_owned()));
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].entity_a, entity_id(&diagram, "A"));
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
    }

    #[test]
    fn should_handle_relationship_between_subgraphs() {
        let diagram = parse(
            "erDiagram\nsubgraph G1\n  A\nend\nsubgraph G2\n  B\nend\nG1 }|--|| G2 : \"relates to\"\n",
        );

        assert_eq!(diagram.subgraphs.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].entity_a, "G1");
        assert_eq!(diagram.relationships[0].entity_b, "G2");
    }

    #[test]
    fn should_handle_relationship_between_entities_in_different_subgraphs() {
        let diagram = parse(
            "erDiagram\nsubgraph G1\n  A\nend\nsubgraph G2\n  B\nend\nA }|--|| B : \"relates to\"\n",
        );

        assert_eq!(diagram.subgraphs.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.subgraphs[0].nodes[0], "A");
        assert_eq!(diagram.subgraphs[1].nodes[0], "B");
        assert_eq!(diagram.relationships[0].entity_a, entity_id(&diagram, "A"));
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
    }

    #[test]
    fn should_handle_relationship_between_entity_and_subgraph() {
        let diagram = parse("erDiagram\nsubgraph Group\nA\nend\nGroup }|--|| B : \"relates to\"\n");

        assert_eq!(diagram.subgraphs.len(), 1);
        let group = &diagram.subgraphs[0];
        assert_eq!(group.nodes.len(), 1);
        assert!(!group.nodes.contains(&"B".to_owned()));
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].entity_a, "Group");
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
    }

    #[test]
    fn should_handle_multiple_relationships_inside_a_subgraph() {
        let diagram =
            parse("erDiagram\nsubgraph Group\n  A }|--|| B : \"r1\"\n  B }|--|| C : \"r2\"\nend");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].nodes.len(), 3);
        assert_eq!(diagram.relationships.len(), 2);
        assert_eq!(diagram.relationships[0].entity_a, entity_id(&diagram, "A"));
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
        assert_eq!(diagram.relationships[1].entity_a, entity_id(&diagram, "B"));
        assert_eq!(diagram.relationships[1].entity_b, entity_id(&diagram, "C"));
    }
}

mod when_parsing_er_nested_subgraphs {
    use super::*;

    #[test]
    fn should_handle_nested_subgraphs_correctly() {
        let diagram = parse("erDiagram\nsubgraph outer\n  A\n  subgraph inner\n    B\n  end\nend");

        assert_eq!(diagram.subgraphs.len(), 2);
        let outer = subgraph(&diagram, "outer");
        let inner = subgraph(&diagram, "inner");
        assert_eq!(outer.nodes.len(), 2);
        assert_eq!(inner.nodes.len(), 1);
        assert_eq!(outer.nodes[0], "A");
        assert_eq!(inner.nodes[0], "B");
        assert_eq!(outer.title, "outer");
        assert_eq!(inner.title, "inner");
    }

    #[test]
    fn should_handle_nested_subgraphs_with_irregular_indentation() {
        let diagram = parse(
            "erDiagram\nsubgraph outer\n    A\n      B\n  subgraph inner\n        C\n  end\nend",
        );

        assert_eq!(diagram.subgraphs.len(), 2);
        let outer = subgraph(&diagram, "outer");
        let inner = subgraph(&diagram, "inner");
        assert_eq!(outer.nodes.len(), 3);
        assert_eq!(inner.nodes.len(), 1);
        assert_eq!(outer.nodes[0], "A");
        assert_eq!(outer.nodes[1], "B");
        assert_eq!(inner.nodes[0], "C");
        assert_eq!(outer.title, "outer");
        assert_eq!(inner.title, "inner");
    }

    #[test]
    fn should_handle_relationships_inside_nested_subgraphs() {
        let diagram = parse(
            "erDiagram\nsubgraph outer\n  subgraph inner\n    A }|--|| B : \"relates to\"\n  end\nend",
        );

        assert_eq!(diagram.subgraphs.len(), 2);
        let outer = subgraph(&diagram, "outer");
        let inner = subgraph(&diagram, "inner");
        assert_eq!(outer.nodes.len(), 1);
        assert_eq!(inner.nodes.len(), 2);
        assert_eq!(inner.nodes[0], "A");
        assert_eq!(inner.nodes[1], "B");
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].entity_a, entity_id(&diagram, "A"));
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
    }

    #[test]
    fn should_handle_relationship_between_outer_and_inner_subgraph_entities() {
        let diagram = parse(
            "erDiagram\nsubgraph outer\n  A\n  subgraph inner\n    B\n  end\nend\nA }|--|| B : \"relates to\"\n",
        );

        assert_eq!(diagram.subgraphs.len(), 2);
        let outer = subgraph(&diagram, "outer");
        let inner = subgraph(&diagram, "inner");
        assert_eq!(outer.nodes.len(), 2);
        assert_eq!(inner.nodes.len(), 1);
        assert_eq!(outer.nodes[0], "A");
        assert_eq!(inner.nodes[0], "B");
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].entity_a, entity_id(&diagram, "A"));
        assert_eq!(diagram.relationships[0].entity_b, entity_id(&diagram, "B"));
    }
}
