//! Port of the `getData` cases of `packages/mermaid/src/diagrams/class/classDiagram-colorIndex.spec.ts`
//! and of `packages/mermaid/src/diagrams/class/classDiagram.spec.ts` (the compact-namespace cases).
//! Test names follow the spec's titles in snake_case, as the ignored stubs of the syntax crate had
//! them (`ferrigura-syntax/tests/class_color_index_spec.rs`, `class_definition_spec.rs`), which this
//! file replaces.
//!
//! Each test parses the text, sets the configuration the spec sets, and reads the layout data that
//! `getData` gives (`crates/ferrigura-render/src/class/data.rs`), with the light theme.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_render::class::data::get_data;
use ferrigura_render::context::Globals;
use ferrigura_render::unified::Section;
use ferrigura_render::unified::data::{LayoutData, Node};
use ferrigura_syntax::class::parse_grammar;

/// The layout data of a class text under `config`.
fn layout_with(text: &str, config: &MermaidConfig) -> LayoutData {
    let diagram = parse_grammar(text).unwrap_or_else(|error| panic!("{text:?}: {error:?}"));
    let theme = ThemeTokens::from_value(&Theme::ReduxColor.variables(None));
    get_data(&diagram, &Section::of(config), &Globals::of(config), &theme)
}

/// The layout data of a class text under the default configuration.
fn layout(text: &str) -> LayoutData {
    layout_with(text, &MermaidConfig::default())
}

/// The node with the id, which `getData` must give.
fn node<'a>(data: &'a LayoutData, id: &str) -> &'a Node {
    data.nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// The ids of the group nodes, sorted as the spec sorts them.
fn group_ids(data: &LayoutData) -> Vec<&str> {
    let mut ids: Vec<&str> = data
        .nodes
        .iter()
        .filter(|node| node.is_group)
        .map(|node| node.id.as_str())
        .collect();
    ids.sort_unstable();
    ids
}

/// A configuration with `class.hierarchicalNamespaces` set as the given value.
fn hierarchical(value: bool) -> MermaidConfig {
    let mut config = MermaidConfig::default();
    config.class.hierarchical_namespaces = value;
    config
}

#[test]
fn should_give_each_class_its_own_slot_in_declaration_order() {
    let data = layout("classDiagram\nclass Order\nclass Customer\nclass Payment");
    let slots: Vec<Option<u32>> = ["Order", "Customer", "Payment"]
        .iter()
        .map(|id| node(&data, id).color_index)
        .collect();
    assert_eq!(slots, [Some(0), Some(1), Some(2)]);
}

#[test]
fn should_not_spend_a_slot_on_a_namespace_container() {
    let data = layout("classDiagram\nnamespace shop {\n class Order\n}\nclass Customer");
    assert!(node(&data, "shop").is_group, "the namespace is a group");
    assert_eq!(node(&data, "shop").color_index, None);
    assert_eq!(node(&data, "Order").color_index, Some(0));
    assert_eq!(node(&data, "Customer").color_index, Some(1));
}

#[test]
fn should_not_spend_a_slot_on_a_note() {
    let data = layout("classDiagram\nclass Order\nnote for Order \"a note\"\nclass Customer");
    let note = data
        .nodes
        .iter()
        .find(|node| node.shape == "note")
        .expect("the note is a node");
    assert_eq!(note.color_index, None);
    assert_eq!(node(&data, "Order").color_index, Some(0));
    assert_eq!(node(&data, "Customer").color_index, Some(1));
}

#[test]
fn emits_only_explicit_namespaces_from_get_data_in_compact_mode() {
    let data = layout_with(
        r#"classDiagram
          namespace Company.Engineering.Backend {
            class Developer {
              +writeCode()
            }
          }
          namespace Company {
            class CEO {
              +makeDecisions()
            }
          }"#,
        &hierarchical(false),
    );
    // Only the two user-declared namespaces survive: the implicit ancestor is elided.
    assert_eq!(group_ids(&data), ["Company", "Company.Engineering.Backend"]);
    // Labels are the full qualified id in compact mode, and namespaces have no parent.
    let backend = node(&data, "Company.Engineering.Backend");
    assert_eq!(backend.label, "Company.Engineering.Backend");
    assert_eq!(backend.parent_id, None);
    // Classes stay under their own declared namespace.
    assert_eq!(
        node(&data, "Developer").parent_id.as_deref(),
        Some("Company.Engineering.Backend")
    );
    assert_eq!(node(&data, "CEO").parent_id.as_deref(), Some("Company"));
}

#[test]
fn moves_classes_inside_implicit_namespaces_to_the_nearest_explicit_ancestor_in_compact_mode() {
    let data = layout_with(
        r#"classDiagram
          namespace Company.Engineering.Backend {
            class Developer
          }
          namespace Company {
            class CEO
          }"#,
        &hierarchical(false),
    );
    assert_eq!(group_ids(&data), ["Company", "Company.Engineering.Backend"]);
    // `Developer` sits in the implicit `Company.Engineering`, which is not drawn: it moves to the
    // declared `Company.Engineering.Backend`, which is its own namespace.
    assert_eq!(
        node(&data, "Developer").parent_id.as_deref(),
        Some("Company.Engineering.Backend")
    );
}

#[test]
fn defaults_to_hierarchical_rendering_when_hierarchical_namespaces_is_unset() {
    let data = layout("classDiagram\nnamespace A.B {\n  class Foo\n}");
    // Both the implicit ancestor `A` and the declared leaf `A.B` render.
    assert_eq!(group_ids(&data), ["A", "A.B"]);
    let leaf = node(&data, "A.B");
    // In hierarchical mode the namespace label is the short segment.
    assert_eq!(leaf.label, "B");
    assert_eq!(leaf.parent_id.as_deref(), Some("A"));
}

#[test]
fn a_note_on_a_class_is_joined_to_it_by_a_dotted_edge() {
    let data = layout("classDiagram\nclass Order\nnote for Order \"a note\"");
    let edge = data
        .edges
        .iter()
        .find(|edge| edge.id == "edgeNote0")
        .expect("the note edge");
    assert_eq!(edge.start, "note0");
    assert_eq!(edge.end, "Order");
    assert_eq!(edge.pattern, "dotted");
    assert_eq!(
        edge.start_label_right, None,
        "a note edge has no cardinalities"
    );
}

#[test]
fn a_relation_without_cardinalities_has_empty_terminal_labels() {
    let data = layout("classDiagram\nAnimal <|-- Square : extends");
    let edge = &data.edges[0];
    assert_eq!(edge.id, "id_Animal_Square_1");
    assert_eq!(edge.arrow_type_start, "extension");
    assert_eq!(edge.arrow_type_end, "none");
    assert_eq!(edge.start_label_right.as_deref(), Some(""));
    assert_eq!(edge.end_label_left.as_deref(), Some(""));
    assert_eq!(edge.label, "extends");
}
