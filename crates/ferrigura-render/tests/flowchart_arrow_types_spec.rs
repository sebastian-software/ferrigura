//! Port of `elk/__tests__/arrowTypes.spec.ts`: the arrow types that `prepare` sets on the edges
//! before the ELK layout (the reference's `prepareLayoutForElk`).

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::flowchart::data::{Edge, LayoutData};
use ferrigura_render::flowchart::edges::prepare;

/// The edge from `A` to `B` of `edge_type`, with the given start arrow, after `prepare`.
fn prepared(edge_type: &str, arrow_type_start: &str) -> Edge {
    let mut data = LayoutData {
        edges: vec![Edge {
            id: "e".to_owned(),
            start: "A".to_owned(),
            end: "B".to_owned(),
            edge_type: edge_type.to_owned(),
            arrow_type_start: arrow_type_start.to_owned(),
            ..Edge::default()
        }],
        ..LayoutData::default()
    };
    prepare(&mut data, &MermaidConfig::default());
    data.edges.remove(0)
}

#[test]
fn leaves_the_start_of_a_one_directional_arrow_as_none_not_arrow_open() {
    let edge = prepared("arrow_point", "");
    assert_eq!(edge.arrow_type_start, "none");
    assert_eq!(edge.arrow_type_end, "arrow_point");
}

#[test]
fn uses_none_at_both_ends_for_an_open_arrow() {
    let edge = prepared("arrow_open", "");
    assert_eq!(edge.arrow_type_start, "none");
    assert_eq!(edge.arrow_type_end, "none");
}

#[test]
fn defaults_an_edge_with_no_type_to_none_at_both_ends() {
    let edge = prepared("", "");
    assert_eq!(edge.arrow_type_start, "none");
    assert_eq!(edge.arrow_type_end, "none");
}

#[test]
fn keeps_real_markers_on_both_ends_of_a_double_arrow() {
    let edge = prepared("double_arrow_point", "");
    assert_eq!(edge.arrow_type_start, "arrow_point");
    assert_eq!(edge.arrow_type_end, "arrow_point");
}

#[test]
fn never_emits_arrow_open_as_an_arrow_type_for_any_known_edge_type() {
    let types = [
        "arrow_open",
        "arrow_cross",
        "double_arrow_cross",
        "arrow_point",
        "double_arrow_point",
        "arrow_circle",
        "double_arrow_circle",
        "",
    ];
    for edge_type in types {
        let edge = prepared(edge_type, "");
        assert_ne!(
            edge.arrow_type_start, "arrow_open",
            "edge type {edge_type:?}"
        );
        assert_ne!(edge.arrow_type_end, "arrow_open", "edge type {edge_type:?}");
    }
}

#[test]
fn still_honours_an_arrow_type_start_the_diagram_set_itself() {
    let edge = prepared("arrow_point", "arrow_cross");
    assert_eq!(edge.arrow_type_start, "arrow_cross");
}
