//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow.spec.js`. Test names follow the
//! spec's titles in snake_case; a title that repeats after normalising gets `_2`, `_3`, … in source
//! order.
//!
//! The spec's `for` loops over the unsafe property names `__proto__` and `constructor` are the
//! `unsafe_id_tests!` macro, invoked once per name. The `charTest` cases are the `char_test`
//! helper, called from tests whose titles repeat once their punctuation is dropped: for the
//! characters `.`, `:`, `,`, `+`, `*`, `<` and `&` the names are `should_be_able_to_parse_a` and
//! its `_2` to `_7` in that order. The commented-out cases (`_`, `>`, `=`) are not ported.
//!
//! The spec's `cleanupComments` calls are not repeated: `parse` does that step itself.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::parse;

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex {id}"))
}

fn char_test(character: &str, expected: &str) {
    let diagram = parse(&format!("graph TD;A({character})-->B;")).expect("parses");
    assert_eq!(vertex(&diagram, "A").id, "A");
    assert_eq!(vertex(&diagram, "B").id, "B");
    assert_eq!(vertex(&diagram, "A").text, expected);
}

macro_rules! unsafe_id_tests {
    ($prop:literal, $node:ident, $tooltip:ident, $class:ident, $subgraph:ident) => {
        #[test]
        fn $node() {
            parse(concat!("graph LR\n", $prop, " --> A;")).expect("parses");
        }

        #[test]
        fn $tooltip() {
            parse(concat!(
                "graph LR\nclick ",
                $prop,
                " callback \"",
                $prop,
                "\";"
            ))
            .expect("parses");
        }

        #[test]
        fn $class() {
            parse(concat!(
                "graph LR\n",
                $prop,
                " --> A;\nclassDef ",
                $prop,
                " color:#ffffff,fill:#000000;\nclass ",
                $prop,
                " ",
                $prop,
                ";"
            ))
            .expect("parses");
        }

        #[test]
        fn $subgraph() {
            parse(concat!(
                "graph LR\n",
                $prop,
                " --> A;\nsubgraph ",
                $prop,
                "\n C --> D;\nend;"
            ))
            .expect("parses");
        }
    };
}

mod parsing_a_flow_chart {
    use super::*;

    #[test]
    fn should_handle_a_trailing_whitespaces_after_statements() {
        let diagram = parse("graph TD;\n\n\n %% Comment\n A-->B; \n B-->C;").expect("parses");

        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[0].start, "A");
        assert_eq!(diagram.edges[0].end, "B");
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
        assert_eq!(diagram.edges[0].text, "");
    }

    #[test]
    fn should_accept_swimlane_beta_as_a_graph_keyword() {
        let diagram = parse("swimlane-beta LR;A-->B;").expect("parses");

        assert_eq!(diagram.direction, "LR");
        assert_eq!(vertex(&diagram, "A").id, "A");
        assert_eq!(vertex(&diagram, "B").id, "B");
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].start, "A");
        assert_eq!(diagram.edges[0].end, "B");
    }

    #[test]
    fn should_handle_node_names_with_end_substring() {
        let diagram = parse("graph TD\nendpoint --> sender").expect("parses");

        assert_eq!(vertex(&diagram, "endpoint").id, "endpoint");
        assert_eq!(vertex(&diagram, "sender").id, "sender");
        assert_eq!(diagram.edges[0].start, "endpoint");
        assert_eq!(diagram.edges[0].end, "sender");
    }

    #[test]
    fn should_handle_node_names_ending_with_keywords() {
        let diagram = parse("graph TD\nblend --> monograph").expect("parses");

        assert_eq!(vertex(&diagram, "blend").id, "blend");
        assert_eq!(vertex(&diagram, "monograph").id, "monograph");
        assert_eq!(diagram.edges[0].start, "blend");
        assert_eq!(diagram.edges[0].end, "monograph");
    }

    #[test]
    fn should_allow_default_in_the_node_name_id() {
        let diagram = parse("graph TD\ndefault --> monograph").expect("parses");

        assert_eq!(vertex(&diagram, "default").id, "default");
        assert_eq!(vertex(&diagram, "monograph").id, "monograph");
        assert_eq!(diagram.edges[0].start, "default");
        assert_eq!(diagram.edges[0].end, "monograph");
    }

    mod special_characters_should_be_handled {
        use super::*;

        #[test]
        fn should_be_able_to_parse_a() {
            char_test(".", ".");
            char_test("Start 103a.a1", "Start 103a.a1");
        }

        #[test]
        fn should_be_able_to_parse_a_2() {
            char_test(":", ":");
        }

        #[test]
        fn should_be_able_to_parse_a_3() {
            char_test(",", ",");
        }

        #[test]
        fn should_be_able_to_parse_text_containing() {
            char_test("a-b", "a-b");
        }

        #[test]
        fn should_be_able_to_parse_a_4() {
            char_test("+", "+");
        }

        #[test]
        fn should_be_able_to_parse_a_5() {
            char_test("*", "*");
        }

        /// The DB sanitises the text with DOMPurify, which writes a `<` that starts no tag as `&lt;`.
        #[test]
        fn should_be_able_to_parse_a_6() {
            char_test("<", "&lt;");
        }

        #[test]
        fn should_be_able_to_parse_a_7() {
            char_test("&", "&");
        }
    }

    #[test]
    fn should_be_possible_to_use_direction_in_node_ids() {
        let diagram = parse("graph TD;\n  node1TB\n").expect("parses");

        assert_eq!(vertex(&diagram, "node1TB").id, "node1TB");
    }

    #[test]
    fn should_be_possible_to_use_direction_in_node_ids_2() {
        let diagram = parse("graph TD;A--x|text including URL space|B;").expect("parses");

        assert_eq!(vertex(&diagram, "A").id, "A");
    }

    #[test]
    fn should_be_possible_to_use_numbers_as_labels() {
        let diagram = parse("graph TB;subgraph \"number as labels\";1;end;").expect("parses");

        assert_eq!(vertex(&diagram, "1").id, "1");
    }

    #[test]
    fn should_add_acc_title_and_acc_descr_to_flow_chart() {
        let diagram = parse(
            "graph LR\naccTitle: Big decisions\naccDescr: Flow chart of the decision making process\nA[Hard] -->|Text| B(Round)\nB --> C{Decision}\nC -->|One| D[Result 1]\nC -->|Two| E[Result 2]\n",
        )
        .expect("parses");

        assert_eq!(diagram.acc_title, "Big decisions");
        assert_eq!(
            diagram.acc_descr,
            "Flow chart of the decision making process"
        );
    }

    #[test]
    fn should_add_acc_title_and_a_multi_line_acc_descr_to_flow_chart() {
        let diagram = parse(
            "graph LR\naccTitle: Big decisions\n\naccDescr {\nFlow chart of the decision making process\nwith a second line\n}\n\nA[Hard] -->|Text| B(Round)\nB --> C{Decision}\nC -->|One| D[Result 1]\nC -->|Two| E[Result 2]\n",
        )
        .expect("parses");

        assert_eq!(diagram.acc_title, "Big decisions");
        assert_eq!(
            diagram.acc_descr,
            "Flow chart of the decision making process\nwith a second line"
        );
    }

    unsafe_id_tests!(
        "__proto__",
        should_work_with_node_id_proto,
        should_work_with_tooltip_id_proto,
        should_work_with_class_id_proto,
        should_work_with_subgraph_id_proto
    );

    unsafe_id_tests!(
        "constructor",
        should_work_with_node_id_constructor,
        should_work_with_tooltip_id_constructor,
        should_work_with_class_id_constructor,
        should_work_with_subgraph_id_constructor
    );
}
