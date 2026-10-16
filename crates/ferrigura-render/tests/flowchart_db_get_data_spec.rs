//! Port of the `getData` cases of `packages/mermaid/src/diagrams/flowchart/flowDb.spec.ts`: the
//! `flow db getData` interpolation tests, the collapsible subgraphs, the subgraph colour slots and
//! the repeated subgraph ids. Test names follow the spec's titles in snake_case; a title that
//! repeats gets `_2`, as in the syntax crate's ports (`ferrigura-syntax/tests/flowchart_db_spec.rs`).
//!
//! The spec drives the DB with `addVertex`, `addLink` and `addSubGraph` calls. Each test here writes
//! the same statements as diagram text and reads the layout data of the parsed diagram, which is the
//! DB state the reference's `getData()` reads. The configuration is the default one, as the DB has
//! it without a diagram scope.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_render::flowchart::data::{LayoutData, Node, get_data};
use ferrigura_syntax::flowchart::parse;

/// The layout data of a flowchart text.
fn layout(text: &str) -> LayoutData {
    let diagram = parse(text).expect("parses");
    get_data(&diagram, &MermaidConfig::default())
}

/// The node with the id, if `getData` returns one.
fn node<'a>(data: &'a LayoutData, id: &str) -> Option<&'a Node> {
    data.nodes.iter().find(|node| node.id == id)
}

/// The node ids in `getData` order.
fn node_ids(data: &LayoutData) -> Vec<&str> {
    data.nodes.iter().map(|node| node.id.as_str()).collect()
}

/// The colour slot of a subgraph node.
fn slot(data: &LayoutData, id: &str) -> Option<u32> {
    node(data, id).and_then(|node| node.color_index)
}

mod flow_db_get_data {
    use super::*;

    #[test]
    fn should_use_default_interpolate_for_edges_without_specific_interpolate() {
        let data = layout("flowchart TB\nA\nB\nA-->B\nlinkStyle default interpolate stepBefore");

        assert_eq!(data.edges[0].curve.as_deref(), Some("stepBefore"));
    }

    #[test]
    fn should_prioritize_edge_specific_interpolate_over_default_interpolate() {
        let data = layout(
            "flowchart TB\nA\nB\nA-->B\nlinkStyle default interpolate stepBefore\nlinkStyle 0 interpolate basis",
        );

        assert_eq!(data.edges[0].curve.as_deref(), Some("basis"));
    }

    #[test]
    fn should_support_modifying_interpolate_using_edge_id_syntax() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nD\nA-->B\nA e2@-->C\nB e3@-->D\nC-->D\nlinkStyle default interpolate stepBefore\nlinkStyle 0 interpolate basis\ne2@{ curve: monotoneX }\ne3@{ curve: catmullRom }",
        );

        assert_eq!(data.edges[0].curve.as_deref(), Some("basis"));
        assert_eq!(data.edges[1].curve.as_deref(), Some("monotoneX"));
        assert_eq!(data.edges[2].curve.as_deref(), Some("catmullRom"));
        assert_eq!(data.edges[3].curve.as_deref(), Some("stepBefore"));
    }
}

mod flow_db_collapsible_subgraphs {
    use super::*;

    #[test]
    fn renders_a_subgraph_normally_as_a_group_when_no_collapse_metadata_is_set() {
        let data =
            layout("flowchart TB\nA\nB\nC\nA-->B\nC-->A\nsubgraph sub1 [My Group]\nA\nB\nend");

        assert_eq!(node(&data, "sub1").map(|node| node.is_group), Some(true));
        let ids = node_ids(&data);
        for id in ["A", "B", "C", "sub1"] {
            assert!(ids.contains(&id), "missing {id}");
        }
        assert!(
            data.edges
                .iter()
                .any(|edge| edge.start == "C" && edge.end == "A")
        );
    }

    #[test]
    fn forwards_subgraph_metadata_to_the_group_node_so_layouts_can_read_it() {
        let data = layout(
            "flowchart TB\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ algorithm: elk.box }",
        );

        let sub = node(&data, "sub1").expect("sub1 is drawn");
        assert!(sub.is_group);
        assert_eq!(sub.metadata.get("algorithm"), Some(&Value::from("elk.box")));
    }

    #[test]
    fn renders_a_collapsed_subgraph_as_a_single_collapsedgroup_node_and_hides_its_members() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nA-->B\nC-->A\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ view: collapsed }",
        );

        let sub = node(&data, "sub1").expect("sub1 is drawn");
        assert_eq!(sub.shape, "collapsedGroup");
        assert!(!sub.is_group);
        assert_eq!(sub.label, "My Group");
        assert!(node(&data, "A").is_none());
        assert!(node(&data, "B").is_none());
        assert!(node(&data, "C").is_some());
    }

    #[test]
    fn redirects_edges_crossing_into_a_collapsed_subgraph_to_the_collapsed_node() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nC-->A\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ view: collapsed }",
        );

        assert!(
            data.edges
                .iter()
                .any(|edge| edge.start == "C" && edge.end == "sub1")
        );
        assert!(data.edges.iter().all(|edge| edge.end != "A"));
    }

    #[test]
    fn drops_edges_fully_internal_to_a_collapsed_subgraph_avoids_a_self_loop() {
        let data = layout(
            "flowchart TB\nA\nB\nA-->B\nsubgraph sub1 [My Group]\nA\nB\nend\nsub1@{ view: collapsed }",
        );

        assert!(data.edges.is_empty());
    }

    #[test]
    fn redirects_edges_to_the_outermost_collapsed_ancestor_for_nested_collapsed_subgraphs() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nC-->A\nsubgraph inner [Inner]\nA\nB\nend\nsubgraph outer [Outer]\ninner\nend\ninner@{ view: collapsed }\nouter@{ view: collapsed }",
        );

        assert_eq!(
            node(&data, "outer").map(|node| node.shape.as_str()),
            Some("collapsedGroup")
        );
        assert!(node(&data, "inner").is_none());
        assert!(node(&data, "A").is_none());
        assert!(
            data.edges
                .iter()
                .any(|edge| edge.start == "C" && edge.end == "outer")
        );
    }
}

mod flow_db_subgraph_colour_slots {
    use super::*;

    #[test]
    fn numbers_flat_subgraphs_in_source_order() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nsubgraph first [First]\nA\nend\nsubgraph second [Second]\nB\nend\nsubgraph third [Third]\nC\nend",
        );

        let slots = [
            slot(&data, "first"),
            slot(&data, "second"),
            slot(&data, "third"),
        ];
        assert_eq!(slots, [Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn numbers_nested_subgraphs_in_source_order_parent_before_its_children() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nD\nsubgraph InnerOne\nA\nend\nsubgraph InnerTwo\nB\nend\nsubgraph Outer\nInnerOne\nInnerTwo\nend\nsubgraph Sibling\nD\nend",
        );

        let slots = [
            slot(&data, "Outer"),
            slot(&data, "InnerOne"),
            slot(&data, "InnerTwo"),
            slot(&data, "Sibling"),
        ];
        assert_eq!(slots, [Some(0), Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn keeps_a_collapsed_subgraph_on_its_own_slot_so_the_cycle_does_not_shift() {
        let data = layout(
            "flowchart TB\nA\nB\nC\nsubgraph first [First]\nA\nend\nsubgraph second [Second]\nB\nend\nsubgraph third [Third]\nC\nend\nsecond@{ view: collapsed }",
        );

        assert_eq!(slot(&data, "first"), Some(0));
        assert_eq!(slot(&data, "second"), Some(1));
        assert_eq!(slot(&data, "third"), Some(2));
    }

    #[test]
    fn leaves_plain_vertices_without_a_slot() {
        let data = layout("flowchart TB\nA\nsubgraph only [Only]\nA\nend");

        assert_eq!(node(&data, "A").and_then(|node| node.color_index), None);
        assert_eq!(slot(&data, "only"), Some(0));
    }
}

mod flow_db_repeated_subgraph_ids_issue_8326 {
    use super::*;

    /// The nodes with the id, which a repeated subgraph id must keep to one.
    fn count(data: &LayoutData, id: &str) -> usize {
        data.nodes.iter().filter(|node| node.id == id).count()
    }

    #[test]
    fn produces_a_single_group_node_in_getdata_with_both_members_parented_to_it() {
        let data = layout("flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend");

        assert_eq!(count(&data, "S"), 1);
        assert_eq!(node(&data, "S").map(|node| node.is_group), Some(true));
        assert_eq!(
            node(&data, "x").and_then(|node| node.parent_id.as_deref()),
            Some("S")
        );
        assert_eq!(
            node(&data, "y").and_then(|node| node.parent_id.as_deref()),
            Some("S")
        );
        let ids = node_ids(&data);
        let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn applies_a_class_set_after_a_repeated_declaration_to_the_merged_subgraph() {
        let data = layout(
            "flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend\nclassDef hot fill:#f00\nclass S hot",
        );

        assert!(
            node(&data, "S")
                .expect("S is drawn")
                .css_classes
                .contains("hot")
        );
    }

    #[test]
    fn collapses_a_repeated_subgraph_when_its_collapsed_metadata_comes_after_the_repeated_declaration()
     {
        let data = layout(
            "flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend\nS@{ view: collapsed }",
        );

        assert_eq!(count(&data, "S"), 1);
        assert_eq!(
            node(&data, "S").map(|node| node.shape.as_str()),
            Some("collapsedGroup")
        );
        assert!(node(&data, "x").is_none());
        assert!(node(&data, "y").is_none());
    }

    #[test]
    fn collapses_a_repeated_subgraph_when_its_collapsed_metadata_comes_between_the_two_declarations()
     {
        let data = layout(
            "flowchart LR\nsubgraph S\n  x\nend\nS@{ view: collapsed }\nsubgraph S\n  y\nend",
        );

        assert_eq!(count(&data, "S"), 1);
        assert_eq!(
            node(&data, "S").map(|node| node.shape.as_str()),
            Some("collapsedGroup")
        );
        assert!(node(&data, "x").is_none());
        assert!(node(&data, "y").is_none());
    }

    #[test]
    fn flattens_a_subgraph_redeclared_inside_its_own_body_instead_of_creating_a_cycle() {
        let data = layout("flowchart LR\nsubgraph S\n  subgraph S\n    x\n  end\n  y\nend");

        assert_eq!(count(&data, "S"), 1);
        assert!(
            data.nodes
                .iter()
                .all(|node| node.parent_id.as_deref() != Some(node.id.as_str()))
        );
        assert_eq!(
            node(&data, "x").and_then(|node| node.parent_id.as_deref()),
            Some("S")
        );
        assert_eq!(
            node(&data, "y").and_then(|node| node.parent_id.as_deref()),
            Some("S")
        );
    }

    #[test]
    fn merges_an_empty_first_declaration_with_a_later_one_containing_a_nested_subgraph() {
        let data =
            layout("flowchart LR\nsubgraph S\nend\nsubgraph S\n  subgraph T\n    y\n  end\nend");

        assert_eq!(count(&data, "S"), 1);
        assert_eq!(
            node(&data, "T").and_then(|node| node.parent_id.as_deref()),
            Some("S")
        );
        assert_eq!(
            node(&data, "y").and_then(|node| node.parent_id.as_deref()),
            Some("T")
        );
    }
}
