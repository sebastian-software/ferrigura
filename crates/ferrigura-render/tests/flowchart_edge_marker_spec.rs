//! Port of `rendering-elements/edgeMarker.spec.ts`: the `marker-start` and `marker-end` that an
//! edge's path carries for its arrow types, as `paint_paths` writes them.
//!
//! Ignored: the aggregation and composition markers, which only the class diagram draws.

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::flowchart::data::Edge;
use ferrigura_render::flowchart::edges::paint_paths;
use ferrigura_render::flowchart::geom::Point;

/// The `<path>` of a two-point edge with the given arrow types.
fn painted_path(arrow_type_start: &str, arrow_type_end: &str) -> String {
    let edge = Edge {
        id: "L_a_b_0".to_owned(),
        start: "a".to_owned(),
        end: "b".to_owned(),
        arrow_type_start: arrow_type_start.to_owned(),
        arrow_type_end: arrow_type_end.to_owned(),
        points: vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)],
        ..Edge::default()
    };
    paint_paths(
        std::slice::from_ref(&edge),
        "svg",
        &MermaidConfig::default(),
    )
    .remove(0)
    .svg
}

#[test]
fn should_add_markers_for_arrow_cross_arrow_point() {
    let svg = painted_path("arrow_cross", "arrow_point");
    assert!(
        svg.contains(r#"marker-start="url(#svg_flowchart-v2-crossStart)""#),
        "{svg}"
    );
    assert!(
        svg.contains(r#"marker-end="url(#svg_flowchart-v2-pointEnd)""#),
        "{svg}"
    );
}

#[test]
#[ignore = "port: not in flowchart port (aggregation markers are class-diagram only)"]
fn should_add_markers_for_aggregation_arrow_point() {}

#[test]
#[ignore = "port: not in flowchart port (aggregation markers are class-diagram only)"]
fn should_add_markers_for_arrow_point_aggregation() {}

#[test]
#[ignore = "port: not in flowchart port (aggregation and composition are class-diagram only)"]
fn should_add_markers_for_aggregation_composition() {}

#[test]
fn should_not_add_invalid_markers() {
    let svg = painted_path(
        "this is an invalid marker",
        ") url(https://my-malicious-site.example)",
    );
    assert!(!svg.contains("marker-start"), "{svg}");
    assert!(!svg.contains("marker-end"), "{svg}");
}
