//! Port of the layout-data specs of packages/mermaid/src/diagrams/agentflow/parser/agentflow.spec.ts,
//! agentflow-containment-cycle.spec.ts and the edge redirect of agentflow.spec.ts (issue #53): what
//! `getData` gives the layout. The reference's cases are kept, and the quirks the port reproduces each
//! have a case of their own.

#[path = "support/agentflow.rs"]
#[allow(dead_code)]
mod support;

use std::collections::HashMap;

use ferrigura_render::unified::data::{LayoutData, Node};
use support::{Setup, config_with};

/// The `getData` of a text, with the default configuration and the redux colour theme.
fn data(text: &str) -> LayoutData {
    Setup::new(config_with(serde_json::json!({})), None).data(text)
}

/// The node with the id.
fn node<'a>(data: &'a LayoutData, id: &str) -> &'a Node {
    data.nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// The parent of every node, by id.
fn parents(data: &LayoutData) -> HashMap<&str, Option<&str>> {
    data.nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_deref()))
        .collect()
}

/// The fixture of the reference's containment cycle specs: `A` and `B` each contain the other's task.
fn mutual(view_a: &str, view_b: &str) -> String {
    format!(
        "agentflow-beta TB\n  flow A[\"Flow A\"]{view_a}\n    a1[\"Task A1\"]\n    a1 --> B\n  end\n  flow B[\"Flow B\"]{view_b}\n    b1[\"Task B1\"]\n    b1 --> A\n  end\n"
    )
}

#[test]
fn a_flow_is_a_group_of_the_flow_group_shape() {
    // agentflow.spec.ts: `exposes flow as the container type in layout data`.
    let data = data("agentflow-beta TB\n  flow pipeline[\"Pipeline\"]\n    a --> b\n  end");
    let pipeline = node(&data, "pipeline");
    assert!(pipeline.is_group);
    assert_eq!(pipeline.shape, "flowGroup");
    assert_eq!(pipeline.padding, 8.0);
}

#[test]
fn a_collapsed_flow_is_a_collapsed_group_with_its_type_in_its_metadata() {
    let data = data(
        "agentflow-beta TB\n  flow pipeline[\"Pipeline\"]\n    a --> b\n  end\n  pipeline@{ view: \"collapsed\" }",
    );
    let pipeline = node(&data, "pipeline");
    assert!(!pipeline.is_group);
    assert_eq!(pipeline.shape, "collapsedGroup");
    assert_eq!(
        pipeline
            .metadata
            .get("containerType")
            .and_then(|value| value.as_str()),
        Some("flow")
    );
    assert_eq!(pipeline.padding, 8.0);
}

#[test]
fn drops_the_edges_that_collapse_into_one_container_issue_53() {
    // agentflow.spec.ts: `still drops collapse-induced self-loops (issue #53 invariant)`. Both ends of the
    // internal edge collapse to `p`, so the edge would be a `p --> p` self-loop, and it is dropped.
    let data =
        data("agentflow-beta TB\n  flow p[\"P\"]\n    a --> b\n  end\n  p@{ view: \"collapsed\" }");
    assert!(data.edges.is_empty());
}

#[test]
fn keeps_a_genuine_self_loop_beside_a_collapse() {
    let data = data(
        "agentflow-beta TB\n  flow p[\"P\"]\n    a --> b\n  end\n  p@{ view: \"collapsed\" }\n  c --> c",
    );
    assert_eq!(data.edges.len(), 1);
    assert_eq!(
        (data.edges[0].start.as_str(), data.edges[0].end.as_str()),
        ("c", "c")
    );
}

#[test]
fn redirects_an_edge_into_a_collapsed_container_to_it() {
    // issue #53: an edge whose end is hidden by a collapsed container ends at that container.
    let data = data(
        "agentflow-beta TB\n  flow p[\"P\"]\n    a --> b\n  end\n  c --> a\n  p@{ view: \"collapsed\" }",
    );
    let edge = data
        .edges
        .iter()
        .find(|edge| edge.start == "c")
        .expect("the edge from c");
    assert_eq!(edge.end, "p");
}

#[test]
fn a_cycle_of_containers_closes_no_parent_chain() {
    // agentflow-containment-cycle.spec.ts: `does not emit a parent chain that closes a cycle`.
    let data = data(&mutual("", ""));
    let parent = parents(&data);
    for id in parent.keys() {
        let mut seen = Vec::new();
        let mut current = parent.get(id).copied().flatten();
        while let Some(next) = current {
            assert!(!seen.contains(&next), "a cycle through {next}");
            seen.push(next);
            current = parent.get(next).copied().flatten();
        }
    }
}

#[test]
fn keeps_the_first_containment_and_drops_only_the_one_closing_the_loop() {
    // The tasks keep their real containers; `B` is `A`'s parent, and `A` has none.
    let data = data(&mutual("", ""));
    let parent = parents(&data);
    assert_eq!(parent["A"], Some("B"));
    assert_eq!(parent["B"], None);
    assert_eq!(parent["a1"], Some("A"));
    assert_eq!(parent["b1"], Some("B"));
}

#[test]
fn one_collapsed_still_draws_the_collapsed_container_with_no_parent() {
    // agentflow-containment-cycle.spec.ts: `still draws the collapsed container it was asked to draw`.
    let data = data(&mutual("@{ view: collapsed }", ""));
    let collapsed = node(&data, "A");
    assert_eq!(collapsed.shape, "collapsedGroup");
    assert_eq!(collapsed.parent_id, None);
}

#[test]
fn both_containers_collapsed_draw_at_least_one_of_them() {
    let data = data(&mutual("@{ view: collapsed }", "@{ view: collapsed }"));
    assert!(
        data.nodes
            .iter()
            .any(|node| node.id == "A" || node.id == "B")
    );
}

#[test]
fn a_linkstyle_default_applies_to_every_edge_including_the_earlier_ones() {
    // `updateLink('default')` sets `defaultStyle` on the array, and `getData` gives it to every edge.
    let data = data("agentflow-beta TB\n  a --> b\n  b --> c\n  linkStyle default stroke:red");
    assert_eq!(data.edges.len(), 2);
    for edge in &data.edges {
        assert!(
            edge.style.iter().any(|style| style == "stroke:red"),
            "{edge:?}"
        );
    }
}

#[test]
fn an_edge_without_an_interpolation_takes_the_default_one_made_after_it() {
    // `curve` is the edge's own, else the default that the statement after the edges sets, else
    // `flowchart.curve`: the earlier edge takes `linear` too.
    let data = data("agentflow-beta TB\n  a --> b\n  linkStyle default interpolate linear");
    assert_eq!(data.edges[0].curve.as_deref(), Some("linear"));
}

#[test]
fn an_edge_takes_the_flowchart_curve_when_nothing_sets_one() {
    let data = data("agentflow-beta TB\n  a --> b");
    assert_eq!(data.edges[0].curve.as_deref(), Some("basis"));
}

#[test]
fn a_container_takes_the_direction_of_the_diagram_when_flowchart_inherit_dir_is_set() {
    // `flowchart.inheritDir` gives a container with no direction of its own the diagram's direction.
    let text = "agentflow-beta LR\n  flow f[\"F\"]\n    a --> b\n  end";
    let plain = data(text);
    assert_eq!(node(&plain, "f").dir, None);
    let inheriting = Setup::new(
        config_with(serde_json::json!({ "flowchart": { "inheritDir": true } })),
        None,
    )
    .data(text);
    assert_eq!(node(&inheriting, "f").dir.as_deref(), Some("LR"));
}

#[test]
fn a_zero_padding_of_the_vertices_is_the_default_padding() {
    // `flowchart.padding || 8`: a configured zero gives the default, as the reference's `||` does.
    let text = "agentflow-beta TB\n  a --> b";
    let zero = Setup::new(
        config_with(serde_json::json!({ "flowchart": { "padding": 0 } })),
        None,
    )
    .data(text);
    assert_eq!(node(&zero, "a").padding, 8.0);
    let twenty = Setup::new(
        config_with(serde_json::json!({ "flowchart": { "padding": 20 } })),
        None,
    )
    .data(text);
    assert_eq!(node(&twenty, "a").padding, 20.0);
}

#[test]
fn a_vertex_takes_the_wrapping_width_and_the_minimum_width_of_the_section() {
    let data = data("agentflow-beta TB\n  a --> b");
    assert_eq!(node(&data, "a").wrapping_width, Some(120.0));
    assert_eq!(node(&data, "a").min_width, Some(120.0));
}

#[test]
fn the_dom_id_counts_the_vertex_calls_before_the_vertex() {
    // `vertexCounter` counts every `addVertex` call, so `b` is the second call's vertex and `c` the fourth.
    let data = data("agentflow-beta TB\n  a --> b\n  b --> c");
    assert_eq!(node(&data, "a").dom_id, "agentflow-a-0");
    assert_eq!(node(&data, "b").dom_id, "agentflow-b-1");
    assert_eq!(node(&data, "c").dom_id, "agentflow-c-3");
}

#[test]
fn a_container_is_numbered_by_its_declaration_and_the_kinds_take_their_classes() {
    // The colour slots: a vertex takes its kind's class, a container takes a slot.
    let data = data(
        "agentflow-beta TB\n  flow outer[\"O\"]\n    flow inner[\"I\"]\n      a[\"a\"]\n    end\n  end\n  b@{ shape: tool }",
    );
    assert_eq!(node(&data, "outer").color_index, Some(7));
    assert_eq!(node(&data, "inner").color_index, Some(8));
    assert!(node(&data, "b").css_classes.contains("af-kind-tool"));
    assert!(node(&data, "a").css_classes.contains("af-kind-task"));
    assert_eq!(node(&data, "b").shape, "subroutine");
}

#[test]
fn a_connector_is_a_rounded_node_of_the_connector_kind() {
    let data = data("agentflow-beta TB\n  connector x[\"X\"]\n  x --> a");
    let connector = node(&data, "x");
    assert_eq!(connector.shape, "roundedRect");
    assert!(
        connector.css_classes.contains("af-kind-connector"),
        "{}",
        connector.css_classes
    );
}
