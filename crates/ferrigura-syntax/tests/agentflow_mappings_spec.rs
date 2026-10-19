//! Ports of three reference specs on the element mappings of the agentflow parser:
//!
//! - `agentflow-mappings.spec.ts` is `element_mappings`.
//! - `agentflow-inline-metadata-mapping.spec.ts` (issue #60) is `inline_metadata_mapping`.
//! - `agentflow-attachment-mapping.spec.ts` (issue #75) is `attachment_mapping`.
//!
//! Every test in them asserts an element mapping (`addVertexMapping` and the other hooks,
//! `getElementAtPosition`, `getElementsOnLine`, `getMappingStats`), a source position, or the
//! editor's frontmatter offset. Those are the editor's API, which is not ported (open question
//! Q-g), so each test is kept under its reference name, ignored, and panics if it is run.

mod element_mappings {
    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn opts_into_source_faithful_parsing_so_diagram_ts_keeps_comments_and_offsets() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn starts_with_an_empty_element_mappings_list() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn captures_a_position_for_a_plain_labelled_vertex() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn captures_vertices_for_inline_shapes() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn captures_a_position_for_an_edge_statement() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn captures_separate_mappings_for_sequential_edge_statements() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn captures_a_position_spanning_from_keyword_to_end_for_a_flow_container() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn captures_positions_for_nested_flow_containers() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn shifts_positions_by_the_frontmatter_line_offset() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn reports_un_shifted_positions_when_no_frontmatter_offset_is_set() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn get_elements_on_line_returns_all_mappings_that_intersect_a_given_line() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn get_element_at_position_prefers_the_innermost_span() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn get_mapping_stats_reports_counts_per_statement_type() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn clear_drops_element_mappings_and_the_frontmatter_offset() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }
}

mod inline_metadata_mapping {
    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn inline_spans_the_block_so_cursors_inside_resolve_to_the_node() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn inline_block_on_an_edge_destination_spans_the_block() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn single_line_inline_metadata_keeps_the_node_on_its_declaration_line() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn a_node_without_inline_metadata_is_unaffected_decl_only_span() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }
}

mod attachment_mapping {
    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn a_standalone_task_attachment_maps_as_attachment_not_a_second_vertex() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn a_flow_attachment_maps_as_attachment_so_the_flow_id_owns_no_vertex_mapping() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn a_connector_attachment_maps_as_attachment() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn an_edge_metadata_attachment_after_a_e1_b_maps_as_attachment() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn a_multiline_standalone_block_spans_through_the_closing_brace_and_is_hit_testable() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn inline_id_keeps_its_single_widened_vertex_mapping_issue_60() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn a_bare_reference_without_a_block_stays_a_vertex_mapping() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }

    #[test]
    #[ignore = "element mappings and source positions are not ported (open question Q-g)"]
    fn get_mapping_stats_counts_attachments_separately() {
        panic!("element mappings and source positions are not ported (open question Q-g)");
    }
}
