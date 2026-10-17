//! Port of `packages/mermaid/src/diagrams/mindmap/mindmapDb.getData.test.ts`. Test names follow the
//! spec's titles in snake_case.
//!
//! `getData` is the renderer stage: the flat nodes and edges, the shapes, the classes, the sections
//! and the layout. None of it is in the model, so every test is ignored with that reason. The
//! parse-level facts these tests lean on (ids, shapes, classes, icons) are checked in `mindmap_spec`
//! and `mindmap_colors_spec`.

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_return_empty_data_when_no_mindmap_is_set() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_return_structured_data_for_simple_mindmap() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_return_structured_data_for_hierarchical_mindmap() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_preserve_node_properties_in_processed_data() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_generate_unique_edge_ids() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_handle_nodes_with_missing_optional_properties() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_centre_labels_of_every_node_shape_issue_8387() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_assign_correct_section_classes_based_on_sibling_position() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_preserve_custom_classes_while_adding_section_classes() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_not_create_any_fake_root_nodes() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_assign_correct_section_classes_to_edges() {}

#[test]
#[ignore = "port: getData is the renderer stage, not the model (docs-port/mindmap/PLAN.md, section 4)"]
fn should_wrap_section_numbers_when_there_are_more_than_11_level_2_nodes() {}
