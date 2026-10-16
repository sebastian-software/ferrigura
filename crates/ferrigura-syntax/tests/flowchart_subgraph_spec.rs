//! Port of `packages/mermaid/src/diagrams/flowchart/parser/subgraph.spec.js`. Test names follow the
//! spec's titles in snake_case; a title that repeats after normalising gets `_2`, `_3`, … in source
//! order.
//!
//! The spec's `beforeEach` (`clear`, `setGen`) is DB setup and is not ported. The collapsed-subgraph
//! test is shape data (`@{ view: collapsed }`) and its drawing is the `getData` part (M5), which is
//! not ported here.
//!
//! The last test comes from commit 764814a2d on `main`; this branch's copy of the spec does not have
//! it yet.

use ferrigura_model::flowchart::{FlowSubgraph, FlowchartDiagram};
use ferrigura_model::value::Value;
use ferrigura_syntax::flowchart::parse;

fn subgraph<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowSubgraph {
    diagram
        .subgraphs
        .iter()
        .find(|subgraph| subgraph.id == id)
        .unwrap_or_else(|| panic!("no subgraph {id}"))
}

fn has(nodes: &[String], id: &str) -> bool {
    nodes.iter().any(|node| node == id)
}

mod when_parsing_subgraphs {
    use super::*;

    #[test]
    fn should_handle_subgraph_with_tab_indentation() {
        let diagram = parse("graph TB\nsubgraph One\n\ta1-->a2\nend").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.nodes, ["a2", "a1"]);
        assert_eq!(sub.title, "One");
        assert_eq!(sub.id, "One");
    }

    #[test]
    fn should_handle_subgraph_with_chaining_nodes_indentation() {
        let diagram = parse("graph TB\nsubgraph One\n\ta1-->a2-->a3\nend").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.nodes, ["a3", "a2", "a1"]);
        assert_eq!(sub.title, "One");
        assert_eq!(sub.id, "One");
    }

    #[test]
    fn should_handle_subgraph_with_multiple_words_in_title() {
        let diagram = parse("graph TB\nsubgraph \"Some Title\"\n\ta1-->a2\nend").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.nodes, ["a2", "a1"]);
        assert_eq!(sub.title, "Some Title");
        assert_eq!(sub.id, "subGraph0");
    }

    #[test]
    fn should_handle_subgraph_with_id_and_title_notation() {
        let diagram =
            parse("graph TB\nsubgraph some-id[Some Title]\n\ta1-->a2\nend").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.nodes, ["a2", "a1"]);
        assert_eq!(sub.title, "Some Title");
        assert_eq!(sub.id, "some-id");
    }

    #[test]
    #[ignore = "port: skipped upstream: skipped"]
    fn should_handle_subgraph_without_id_and_space_in_title() {
        let diagram = parse("graph TB\nsubgraph Some Title\n\ta1-->a2\nend").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.nodes, ["a1", "a2"]);
        assert_eq!(sub.title, "Some Title");
        assert_eq!(sub.id, "some-id");
    }

    #[test]
    fn should_handle_subgraph_id_starting_with_a_number() {
        let diagram = parse(
            "graph TD\n    A[Christmas] -->|Get money| B(Go shopping)\n    subgraph 1test\n    A\n    end",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.nodes, ["A"]);
        assert_eq!(sub.id, "1test");
    }

    #[test]
    fn should_handle_subgraphs1() {
        let diagram = parse("graph TD;A-->B;subgraph myTitle;c-->d;end;").expect("parses");

        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_with_title_in_quotes() {
        let diagram =
            parse("graph TD;A-->B;subgraph \"title in quotes\";c-->d;end;").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].title, "title in quotes");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_in_old_style_that_was_broken() {
        let diagram =
            parse("graph TD;A-->B;subgraph old style that is broken;c-->d;end;").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].title, "old style that is broken");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_with_dashes_in_the_title() {
        let diagram = parse("graph TD;A-->B;subgraph a-b-c;c-->d;end;").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].title, "a-b-c");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_with_id_and_title_in_brackets() {
        let diagram =
            parse("graph TD;A-->B;subgraph uid1[text of doom];c-->d;end;").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].title, "text of doom");
        assert_eq!(diagram.subgraphs[0].id, "uid1");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_with_id_and_title_in_brackets_and_quotes() {
        let diagram =
            parse("graph TD;A-->B;subgraph uid2[\"text of doom\"];c-->d;end;").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].title, "text of doom");
        assert_eq!(diagram.subgraphs[0].id, "uid2");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_with_id_and_title_in_brackets_without_spaces() {
        let diagram = parse("graph TD;A-->B;subgraph uid2[textofdoom];c-->d;end;").expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].title, "textofdoom");
        assert_eq!(diagram.subgraphs[0].id, "uid2");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs2() {
        let diagram = parse("graph TD\nA-->B\nsubgraph myTitle\n\n c-->d \nend\n").expect("parses");

        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs3() {
        let diagram =
            parse("graph TD\nA-->B\nsubgraph myTitle   \n\n    c-->d \nend\n").expect("parses");

        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_nested_subgraphs() {
        parse(
            "graph TD\nA-->B\nsubgraph myTitle\n\n c-->d \n\n subgraph inner\n\n   e-->f \n end \n\n subgraph inner\n\n   h-->i \n end \n\nend\n",
        )
        .expect("parses");
    }

    #[test]
    fn should_handle_subgraphs4() {
        let diagram = parse("graph TD\nA-->B\nsubgraph myTitle\nc-->d\nend;").expect("parses");

        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs5() {
        let diagram = parse("graph TD\nA-->B\nsubgraph myTitle\nc-- text -->d\nd-->e\n end;")
            .expect("parses");

        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_subgraphs_with_multi_node_statements_in_it() {
        let diagram =
            parse("graph TD\nA-->B\nsubgraph myTitle\na & b --> c & e\n end;").expect("parses");

        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
    }

    #[test]
    fn should_handle_nested_subgraphs_1() {
        let diagram = parse(
            "flowchart TB\n    subgraph A\n    b-->B\n    a\n    end\n    a-->c\n    subgraph B\n      c\n    end",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 2);
        let sub_a = subgraph(&diagram, "A");
        let sub_b = subgraph(&diagram, "B");
        assert_eq!(sub_b.nodes[0], "c");
        assert!(has(&sub_a.nodes, "B"));
        assert!(has(&sub_a.nodes, "b"));
        assert!(has(&sub_a.nodes, "a"));
        assert!(!has(&sub_a.nodes, "c"));
    }

    #[test]
    fn should_handle_nested_subgraphs_2() {
        let diagram = parse(
            "flowchart TB\n    b-->B\n    a-->c\n    subgraph B\n      c\n    end\n    subgraph A\n        a\n        b\n        B\n    end",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 2);
        let sub_a = subgraph(&diagram, "A");
        let sub_b = subgraph(&diagram, "B");
        assert_eq!(sub_b.nodes[0], "c");
        assert!(has(&sub_a.nodes, "B"));
        assert!(has(&sub_a.nodes, "b"));
        assert!(has(&sub_a.nodes, "a"));
        assert!(!has(&sub_a.nodes, "c"));
    }

    #[test]
    fn should_handle_nested_subgraphs_3() {
        let diagram = parse(
            "flowchart TB\n    subgraph B\n      c\n    end\n    a-->c\n    subgraph A\n      b-->B\n      a\n    end",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 2);
        let sub_a = subgraph(&diagram, "A");
        let sub_b = subgraph(&diagram, "B");
        assert_eq!(sub_b.nodes[0], "c");
        assert!(has(&sub_a.nodes, "B"));
        assert!(has(&sub_a.nodes, "b"));
        assert!(has(&sub_a.nodes, "a"));
        assert!(!has(&sub_a.nodes, "c"));
    }

    #[test]
    fn should_correctly_parse_direction_td_inside_a_subgraph() {
        let diagram =
            parse("\n      graph LR\n        subgraph WithTD\n          direction TD\n          A1 --> A2\n        end\n    ")
                .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        let sub = &diagram.subgraphs[0];
        assert_eq!(sub.dir.as_deref(), Some("TD"));
        assert!(has(&sub.nodes, "A1"));
        assert!(has(&sub.nodes, "A2"));
    }

    #[test]
    fn should_attach_metadata_to_a_subgraph_and_collapse_it() {
        let diagram = parse(
            "\n      graph TD\n        subgraph one [My Group]\n          A --> B\n        end\n        C --> A\n        one@{ view: collapsed }\n    ",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(
            subgraph(&diagram, "one").metadata.get("view"),
            Some(&Value::from("collapsed"))
        );
    }

    #[test]
    fn should_list_a_proto_node_once_as_any_other_node() {
        let ordinary = parse("graph TB\nsubgraph One\nb\nb-->a1\nend")
            .expect("parses")
            .subgraphs[0]
            .nodes
            .clone();
        let proto =
            parse("graph TB\nsubgraph One\n__proto__\n__proto__-->a1\nend").expect("parses");

        let expected: Vec<String> = ordinary
            .iter()
            .map(|node| {
                if node == "b" {
                    String::from("__proto__")
                } else {
                    node.clone()
                }
            })
            .collect();
        let nodes = &subgraph(&proto, "One").nodes;
        assert_eq!(*nodes, expected);
        assert_eq!(nodes.iter().filter(|node| *node == "__proto__").count(), 1);
    }
}
