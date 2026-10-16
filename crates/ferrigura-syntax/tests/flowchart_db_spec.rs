//! Port of `packages/mermaid/src/diagrams/flowchart/flowDb.spec.ts`. Test names follow the spec's
//! titles in snake_case; a title that repeats after normalising gets `_2`, `_3`, … in source order.
//!
//! The DB has no public surface here, so each test drives it through `parse`:
//! - `exist` and `makeUniq` are DB internals. Their behaviour is checked on the subgraphs' nodes of a
//!   parsed diagram, as the brief asks.
//! - `addClass` is `classDef` (styles and classes, step 5).
//! - The `getData()` tests, the collapsible-subgraph tests and the colour-slot tests are rendering
//!   (M5). Where a test also checks the model (the interpolate, the subgraph nodes, the metadata), the
//!   model part is the body; the `getData` lines are not ported.
//! - The own-property test of `class` checks the JavaScript object and is not ported.

use std::collections::BTreeMap;

use ferrigura_model::flowchart::{FlowClass, FlowSubgraph, FlowchartDiagram};
use ferrigura_syntax::flowchart::parse;
use serde_json::Value;

/// Four subgraphs of four, the fixture of the spec's `beforeEach`: `s1` holds `a b c e`, `s2` holds
/// `f g h`, `s3` holds `i j`, `s4` holds `k`.
const FIXTURE: &str = "flowchart TB\nsubgraph s1\na\nb\nc\ne\nend\nsubgraph s2\nf\ng\nh\nend\nsubgraph s3\ni\nj\nend\nsubgraph s4\nk\nend\n";

fn subgraph<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowSubgraph {
    diagram
        .subgraphs
        .iter()
        .find(|subgraph| subgraph.id == id)
        .unwrap_or_else(|| panic!("no subgraph {id}"))
}

fn class<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowClass {
    diagram
        .classes
        .iter()
        .find(|class| class.id == id)
        .unwrap_or_else(|| panic!("no class {id}"))
}

fn ids(diagram: &FlowchartDiagram) -> Vec<&str> {
    diagram
        .subgraphs
        .iter()
        .map(|subgraph| subgraph.id.as_str())
        .collect()
}

fn has(nodes: &[String], id: &str) -> bool {
    nodes.iter().any(|node| node == id)
}

fn in_any_subgraph(diagram: &FlowchartDiagram, id: &str) -> bool {
    diagram
        .subgraphs
        .iter()
        .any(|subgraph| has(&subgraph.nodes, id))
}

fn collapsed(diagram: &FlowchartDiagram, id: &str) -> bool {
    subgraph(diagram, id).metadata.get("view") == Some(&Value::from("collapsed"))
}

mod flow_db_subgraphs {
    use super::*;

    mod exist {
        use super::*;

        #[test]
        fn should_return_true_when_the_is_exists_in_a_subgraph() {
            let diagram = parse(FIXTURE).expect("parses");

            assert!(in_any_subgraph(&diagram, "a"));
            assert!(in_any_subgraph(&diagram, "h"));
            assert!(in_any_subgraph(&diagram, "j"));
            assert!(in_any_subgraph(&diagram, "k"));
        }

        #[test]
        fn should_return_false_when_the_is_exists_in_a_subgraph() {
            let diagram = parse(FIXTURE).expect("parses");

            assert!(!in_any_subgraph(&diagram, "a2"));
            assert!(!in_any_subgraph(&diagram, "l"));
        }
    }

    mod make_uniq {
        use super::*;

        #[test]
        fn should_remove_ids_from_sungraph_that_already_exists_in_another_subgraph_even_if_it_gets_empty()
         {
            let diagram = parse(&format!("{FIXTURE}subgraph x\ni\nj\nend")).expect("parses");

            assert!(subgraph(&diagram, "x").nodes.is_empty());
        }

        #[test]
        fn should_remove_ids_from_sungraph_that_already_exists_in_another_subgraph() {
            let diagram = parse(&format!("{FIXTURE}subgraph x\ni\nj\no\nend")).expect("parses");

            assert_eq!(subgraph(&diagram, "x").nodes, ["o"]);
        }

        #[test]
        fn should_not_remove_ids_from_subgraph_if_they_are_unique() {
            let diagram = parse(&format!("{FIXTURE}subgraph x\nq\nr\ns\nend")).expect("parses");

            assert_eq!(subgraph(&diagram, "x").nodes, ["q", "r", "s"]);
        }
    }
}

mod flow_db_add_class {
    use super::*;

    #[test]
    fn should_detect_many_classes() {
        let diagram = parse("flowchart TB\nclassDef a,b stroke-width: 8px").expect("parses");

        assert_eq!(class(&diagram, "a").styles, ["stroke-width: 8px"]);
        assert_eq!(class(&diagram, "b").styles, ["stroke-width: 8px"]);
    }

    #[test]
    fn should_detect_single_class() {
        let diagram = parse("flowchart TB\nclassDef a stroke-width: 8px").expect("parses");

        assert_eq!(class(&diagram, "a").styles, ["stroke-width: 8px"]);
    }
}

mod flow_db_class {
    #[test]
    #[ignore = "port: JavaScript API, not ported"]
    fn should_have_functions_used_in_flow_jison_as_own_property() {
        // The spec checks that the JISON's functions are own properties of the JavaScript DB object.
        // The Rust DB is not a JavaScript object, so there is nothing to check here.
    }
}

mod flow_db_get_data {
    use super::*;

    #[test]
    fn should_use_default_interpolate_for_edges_without_specific_interpolate() {
        // The spec checks the rendered `curve`. The model keeps the default that `linkStyle` sets and
        // leaves the fallback to the renderer.
        let diagram =
            parse("flowchart TB\nA-->B\nlinkStyle default interpolate stepBefore").expect("parses");

        assert_eq!(diagram.default_interpolate.as_deref(), Some("stepBefore"));
    }

    #[test]
    fn should_prioritize_edge_specific_interpolate_over_default_interpolate() {
        let diagram = parse(
            "flowchart TB\nA-->B\nlinkStyle default interpolate stepBefore\nlinkStyle 0 interpolate basis",
        )
        .expect("parses");

        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
    }

    #[test]
    fn should_support_modifying_interpolate_using_edge_id_syntax() {
        let diagram = parse(
            "flowchart TB\nA-->B\nA e2@-->C\nB e3@-->D\nC-->D\nlinkStyle default interpolate stepBefore\nlinkStyle 0 interpolate basis\ne2@{ curve: monotoneX }\ne3@{ curve: catmullRom }",
        )
        .expect("parses");

        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("monotoneX"));
        assert_eq!(diagram.edges[2].interpolate.as_deref(), Some("catmullRom"));
        assert_eq!(diagram.edges[3].interpolate, None);
    }
}

mod flow_db_collapsible_subgraphs {
    use super::*;

    #[test]
    fn renders_a_subgraph_normally_as_a_group_when_no_collapse_metadata_is_set() {
        // The spec checks `isGroup`, which is rendering. The model keeps the members and the edge.
        let diagram = parse("flowchart TB\nA-->B\nC-->A\nsubgraph sub1 [My Group]\nA\nB\nend")
            .expect("parses");

        assert!(has(&subgraph(&diagram, "sub1").nodes, "A"));
        assert!(has(&subgraph(&diagram, "sub1").nodes, "B"));
        assert!(
            diagram
                .edges
                .iter()
                .any(|edge| edge.start == "C" && edge.end == "A")
        );
    }

    #[test]
    fn forwards_subgraph_metadata_to_the_group_node_so_layouts_can_read_it() {
        let diagram =
            parse("flowchart TB\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ algorithm: elk.box }")
                .expect("parses");

        assert_eq!(
            subgraph(&diagram, "sub1").metadata.get("algorithm"),
            Some(&Value::from("elk.box"))
        );
    }

    #[test]
    fn renders_a_collapsed_subgraph_as_a_single_collapsedgroup_node_and_hides_its_members() {
        let diagram = parse(
            "flowchart TB\nA-->B\nC-->A\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ view: collapsed }",
        )
        .expect("parses");

        assert!(collapsed(&diagram, "sub1"));
        assert_eq!(subgraph(&diagram, "sub1").title, "My Group");
    }

    #[test]
    fn redirects_edges_crossing_into_a_collapsed_subgraph_to_the_collapsed_node() {
        let diagram = parse(
            "flowchart TB\nC-->A\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ view: collapsed }",
        )
        .expect("parses");

        assert!(collapsed(&diagram, "sub1"));
    }

    #[test]
    fn drops_edges_fully_internal_to_a_collapsed_subgraph_avoids_a_self_loop() {
        let diagram = parse(
            "flowchart TB\nA-->B\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ view: collapsed }",
        )
        .expect("parses");

        assert!(collapsed(&diagram, "sub1"));
    }

    #[test]
    fn redirects_edges_to_the_outermost_collapsed_ancestor_for_nested_collapsed_subgraphs() {
        let diagram = parse(
            "flowchart TB\nC-->A\nsubgraph inner [Inner]\nA\nB\nend\nsubgraph outer [Outer]\ninner\nend\ninner@{ view: collapsed }\nouter@{ view: collapsed }",
        )
        .expect("parses");

        assert!(collapsed(&diagram, "inner"));
        assert!(collapsed(&diagram, "outer"));
    }
}

mod flow_db_direction {
    use super::*;

    #[test]
    fn should_set_direction_to_tb_when_td_is_set() {
        let diagram = parse("flowchart TD\nA-->B").expect("parses");

        assert_eq!(diagram.direction, "TB");
    }

    #[test]
    fn should_correctly_set_direction_irrespective_of_leading_spaces() {
        let diagram = parse("flowchart  TD\nA-->B").expect("parses");

        assert_eq!(diagram.direction, "TB");
    }
}

mod flow_db_subgraph_colour_slots {
    use super::*;

    #[test]
    fn numbers_flat_subgraphs_in_source_order() {
        // The spec checks `colorIndex`, which is rendering. The model keeps the subgraphs in closing
        // order, which for these flat subgraphs is the source order.
        let diagram = parse(
            "flowchart TB\nA\nB\nC\nsubgraph first [First]\nA\nend\nsubgraph second [Second]\nB\nend\nsubgraph third [Third]\nC\nend",
        )
        .expect("parses");

        assert_eq!(ids(&diagram), ["first", "second", "third"]);
    }

    #[test]
    fn numbers_nested_subgraphs_in_source_order_parent_before_its_children() {
        // The model lists subgraphs in closing order, so the nested ones come before their parent.
        // The colour numbering that the spec checks is rendering.
        let diagram = parse(
            "flowchart TB\nA\nB\nC\nD\nsubgraph InnerOne\nA\nend\nsubgraph InnerTwo\nB\nend\nsubgraph Outer\nInnerOne\nInnerTwo\nend\nsubgraph Sibling\nD\nend",
        )
        .expect("parses");

        assert_eq!(ids(&diagram), ["InnerOne", "InnerTwo", "Outer", "Sibling"]);
    }

    #[test]
    fn keeps_a_collapsed_subgraph_on_its_own_slot_so_the_cycle_does_not_shift() {
        let diagram = parse(
            "flowchart TB\nA\nB\nC\nsubgraph first [First]\nA\nend\nsubgraph second [Second]\nB\nend\nsubgraph third [Third]\nC\nend\nsecond@{ view: collapsed }",
        )
        .expect("parses");

        assert_eq!(ids(&diagram), ["first", "second", "third"]);
        assert!(collapsed(&diagram, "second"));
    }

    #[test]
    fn leaves_plain_vertices_without_a_slot() {
        // A plain vertex has no colour slot in the model; the subgraph is the only one listed.
        let diagram = parse("flowchart TB\nA\nsubgraph only [Only]\nA\nend").expect("parses");

        assert_eq!(ids(&diagram), ["only"]);
    }
}

mod flow_db_repeated_subgraph_ids_issue_8326 {
    use super::*;

    #[test]
    fn merges_a_repeated_subgraph_id_into_a_single_subgraph() {
        let diagram =
            parse("flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend").expect("parses");

        assert_eq!(ids(&diagram), ["S"]);
        assert_eq!(subgraph(&diagram, "S").nodes, ["x", "y"]);
    }

    #[test]
    fn keeps_the_title_of_the_first_declaration() {
        let diagram =
            parse("flowchart LR\nsubgraph S [First]\n  x\nend\nsubgraph S [Second]\n  y\nend")
                .expect("parses");

        assert_eq!(ids(&diagram), ["S"]);
        assert_eq!(subgraph(&diagram, "S").title, "First");
    }

    #[test]
    fn produces_a_single_group_node_in_getdata_with_both_members_parented_to_it() {
        // The spec checks the `getData` nodes and their `parentId`. The model keeps one subgraph
        // with both members.
        let diagram =
            parse("flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend").expect("parses");

        assert_eq!(ids(&diagram), ["S"]);
        assert_eq!(subgraph(&diagram, "S").nodes, ["x", "y"]);
    }

    #[test]
    fn applies_a_class_set_after_a_repeated_declaration_to_the_merged_subgraph() {
        let diagram = parse(
            "flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend\nclassDef hot fill:#f00\nclass S hot",
        )
        .expect("parses");

        assert!(
            subgraph(&diagram, "S")
                .classes
                .iter()
                .any(|class| class == "hot")
        );
    }

    #[test]
    fn collapses_a_repeated_subgraph_when_its_collapsed_metadata_comes_after_the_repeated_declaration()
     {
        let diagram = parse(
            "flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend\nS@{ view: collapsed }",
        )
        .expect("parses");

        assert_eq!(
            subgraph(&diagram, "S").metadata,
            BTreeMap::from([(String::from("view"), Value::from("collapsed"))])
        );
    }

    #[test]
    fn collapses_a_repeated_subgraph_when_its_collapsed_metadata_comes_between_the_two_declarations()
     {
        let diagram = parse(
            "flowchart LR\nsubgraph S\n  x\nend\nS@{ view: collapsed }\nsubgraph S\n  y\nend",
        )
        .expect("parses");

        assert_eq!(
            subgraph(&diagram, "S").metadata,
            BTreeMap::from([(String::from("view"), Value::from("collapsed"))])
        );
    }

    #[test]
    fn flattens_a_subgraph_redeclared_inside_its_own_body_instead_of_creating_a_cycle() {
        // The spec's `getData` lines (one `S` node, no self-parenting) are rendering and are not
        // ported here.
        let diagram = parse("flowchart LR\nsubgraph S\n  subgraph S\n    x\n  end\n  y\nend")
            .expect("parses");

        assert_eq!(ids(&diagram), ["S"]);
        let members = &subgraph(&diagram, "S").nodes;
        assert!(!has(members, "S"));
        assert!(has(members, "x"));
        assert!(has(members, "y"));
    }

    #[test]
    fn merges_an_empty_first_declaration_with_a_later_one_containing_a_nested_subgraph() {
        // The spec's `getData` lines (the single `S` node and the `parentId`s) are rendering and are
        // not ported here.
        let diagram =
            parse("flowchart LR\nsubgraph S\nend\nsubgraph S\n  subgraph T\n    y\n  end\nend")
                .expect("parses");

        let mut sorted = ids(&diagram);
        sorted.sort_unstable();
        assert_eq!(sorted, ["S", "T"]);
        assert_eq!(subgraph(&diagram, "S").nodes, ["T"]);
        assert_eq!(subgraph(&diagram, "T").nodes, ["y"]);
    }

    #[test]
    fn keeps_distinct_and_anonymous_subgraphs_separate() {
        let diagram = parse(
            "flowchart LR\nsubgraph A\n  a\nend\nsubgraph B\n  b\nend\nsubgraph \"Same Title\"\n  c\nend\nsubgraph \"Same Title\"\n  d\nend",
        )
        .expect("parses");

        assert_eq!(diagram.subgraphs.len(), 4);
        assert_eq!(subgraph(&diagram, "A").nodes, ["a"]);
        assert_eq!(subgraph(&diagram, "B").nodes, ["b"]);
        assert_eq!(subgraph(&diagram, "subGraph2").nodes, ["c"]);
        assert_eq!(subgraph(&diagram, "subGraph3").nodes, ["d"]);
    }
}
