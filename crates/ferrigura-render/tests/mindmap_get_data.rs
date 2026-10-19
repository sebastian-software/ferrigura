//! Port of `packages/mermaid/src/diagrams/mindmap/mindmapDb.getData.test.ts` and of the section tests of
//! `mindmapColors.spec.ts`. The reference's `getData` is `ferrigura_render::mindmap::data::layout_data`:
//! the flat nodes and edges, the sections and classes, and the shapes with their overrides.
//!
//! The spec titles are in snake_case. The reference's specs drive a hand-built tree, which the port
//! builds from the mindmap text, as the DB does.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::LabelType;
use ferrigura_render::context::Globals;
use ferrigura_render::mindmap::data::layout_data;
use ferrigura_render::unified::data::{LayoutData, Node};
use ferrigura_syntax::mindmap::parse;

/// The layout data of a mindmap text, with the colouring of `colorBy`.
fn data(text: &str, color_by: &str) -> LayoutData {
    let diagram = parse(text).expect("the text parses");
    let mut config = MermaidConfig::default();
    config.mindmap.color_by = color_by.to_owned();
    layout_data(&diagram, &Globals::of(&config), &config.mindmap)
}

fn node<'a>(data: &'a LayoutData, id: &str) -> &'a Node {
    data.nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

#[test]
fn should_return_empty_data_when_no_mindmap_is_set() {
    let data = data("mindmap\n", "branch");
    assert!(data.nodes.is_empty());
    assert!(data.edges.is_empty());
}

#[test]
fn should_return_structured_data_for_simple_mindmap() {
    let data = data("mindmap\nroot\n  child\n", "branch");
    let ids: Vec<&str> = data.nodes.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(ids, ["0", "1"]);
    assert_eq!(node(&data, "0").label, "root");
    assert_eq!(node(&data, "1").label, "child");
    assert_eq!(data.edges.len(), 1);
    let edge = &data.edges[0];
    assert_eq!(edge.id, "edge_0_1");
    assert_eq!((edge.start.as_str(), edge.end.as_str()), ("0", "1"));
    assert_eq!(edge.curve.as_deref(), Some("basis"));
    assert_eq!(edge.thickness, "normal");
}

#[test]
fn should_not_create_any_fake_root_nodes() {
    let data = data("mindmap\nroot\n  a\n    b\n  c\n", "branch");
    assert_eq!(data.nodes.len(), 4);
    assert_eq!(data.edges.len(), 3);
}

#[test]
fn should_generate_unique_edge_ids() {
    let data = data("mindmap\nroot\n  a\n    b\n    c\n  d\n", "branch");
    let mut ids: Vec<&str> = data.edges.iter().map(|edge| edge.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), data.edges.len());
}

#[test]
fn the_root_has_the_root_classes_and_no_section() {
    let data = data("mindmap\nroot\n  child\n", "branch");
    assert_eq!(
        node(&data, "0").css_classes,
        "mindmap-node section-root section--1"
    );
}

#[test]
fn should_assign_correct_section_classes_based_on_sibling_position() {
    // The children of the root are numbered by their position; their descendants inherit it.
    let data = data(
        "mindmap\nroot\n  A\n    A1\n      A11\n  B\n  C\n    C1\n",
        "branch",
    );
    assert_eq!(node(&data, "1").css_classes, "mindmap-node section-0");
    assert_eq!(node(&data, "2").css_classes, "mindmap-node section-0");
    assert_eq!(node(&data, "3").css_classes, "mindmap-node section-0");
    assert_eq!(node(&data, "4").css_classes, "mindmap-node section-1");
    assert_eq!(node(&data, "5").css_classes, "mindmap-node section-2");
    assert_eq!(node(&data, "6").css_classes, "mindmap-node section-2");
}

#[test]
fn should_wrap_section_numbers_when_there_are_more_than_11_level_2_nodes() {
    let mut text = String::from("mindmap\nroot\n");
    for index in 0..13 {
        text.push_str(&format!("  n{index}\n"));
    }
    let data = data(&text, "branch");
    // Node `n<k>` has id `k + 1`: the twelfth child (index 11) is the first to wrap.
    assert_eq!(node(&data, "11").css_classes, "mindmap-node section-10");
    assert_eq!(node(&data, "12").css_classes, "mindmap-node section-0");
    assert_eq!(node(&data, "13").css_classes, "mindmap-node section-1");
}

#[test]
fn colors_by_tree_depth_independently_of_indentation_and_branch() {
    // With `colorBy: depth`, a node takes `(depth - 1) % 11`, whatever its indentation.
    let data = data("mindmap\nroot\n  a\n      b\n        c\n  d\n", "depth");
    assert_eq!(node(&data, "1").css_classes, "mindmap-node section-0");
    assert_eq!(node(&data, "2").css_classes, "mindmap-node section-1");
    assert_eq!(node(&data, "3").css_classes, "mindmap-node section-2");
    // `d` is a child of the root, at depth 1, so it is section 0 as well.
    assert_eq!(node(&data, "4").css_classes, "mindmap-node section-0");
}

#[test]
fn should_preserve_custom_classes_while_adding_section_classes() {
    let data = data("mindmap\nroot\n  child\n    :::custom-class\n", "branch");
    assert_eq!(
        node(&data, "1").css_classes,
        "mindmap-node section-0 custom-class"
    );
}

#[test]
fn should_assign_correct_section_classes_to_edges() {
    // An edge takes the section of its child and the depth of its parent's level plus one.
    let data = data("mindmap\nroot\n  a\n    b\n", "branch");
    assert_eq!(data.edges[0].classes, "edge section-edge-0 edge-depth-1");
    // The depth class follows the indentation: `a` is at level 2, so its edge is depth 3.
    assert_eq!(data.edges[1].classes, "edge section-edge-0 edge-depth-3");
}

#[test]
fn should_centre_labels_of_every_node_shape_issue_8387() {
    let data = data(
        "mindmap\nroot((Cell))\n  a[Square]\n  b(Rounded)\n",
        "branch",
    );
    for node in &data.nodes {
        assert_eq!(node.label_type, LabelType::Markdown);
        assert_eq!(node.center_label, Some(true));
    }
}

#[test]
fn should_handle_nodes_with_missing_optional_properties() {
    let data = data("mindmap\nroot\n  child\n", "branch");
    let child = node(&data, "1");
    assert!(child.icon.is_none());
    assert!(!child.is_group);
    assert_eq!(child.look, "classic");
}

#[test]
fn the_shapes_take_the_overrides_of_the_renderer() {
    // `rounded`: width 0 and padding 15; `rect`: width 0 and padding 10; `hexagon`: width and height
    // 0 with the DB's doubled padding; `mindmapCircle`: the DB's width and padding, unchanged.
    let data = data(
        "mindmap\nroot(Root)\n  rect[A]\n  hex{{B}}\n  circle((C))\n",
        "branch",
    );
    let root = node(&data, "0");
    assert_eq!(
        (root.shape.as_str(), root.width, root.padding),
        ("rounded", Some(0.0), 15.0)
    );
    let rect = node(&data, "1");
    assert_eq!(
        (rect.shape.as_str(), rect.width, rect.padding),
        ("rect", Some(0.0), 10.0)
    );
    let hexagon = node(&data, "2");
    assert_eq!(hexagon.shape, "hexagon");
    assert_eq!(
        (hexagon.width, hexagon.height, hexagon.padding),
        (Some(0.0), Some(0.0), 20.0)
    );
    let circle = node(&data, "3");
    assert_eq!(
        (circle.shape.as_str(), circle.width, circle.padding),
        ("mindmapCircle", Some(200.0), 10.0)
    );
}
