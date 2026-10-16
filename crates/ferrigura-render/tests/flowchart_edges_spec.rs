//! Port of `rendering-elements/edges.spec.js`, the cases that need no browser.
//!
//! Ported: `resolveEdgeCurveType` for a curve name, through `Curve::from_name` for the d3 curves
//! and through `paint_paths` for `rounded`, which the path builder takes by name.
//!
//! Ignored: the swimlane clipping of `insertEdge` (the flowchart port is ELK only and never clips
//! a route to its shapes), the config fallback of `resolveEdgeCurveType` (not ported),
//! `computeLabelTransform` (it waits for the label layout, task T2, or is HTML labels), and
//! `setTerminalWidth` (terminal labels are not in the flowchart model).

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::unified::data::Edge;
use ferrigura_render::unified::edges::paint_paths;
use ferrigura_render::unified::geom::Point;
use ferrigura_render::unified::path::Curve;

/// The `d` attribute of the `<path>` element in an edge's SVG.
fn d_attribute(svg: &str) -> Option<&str> {
    let start = svg.find(" d=\"")? + " d=\"".len();
    let end = svg[start..].find('"')?;
    Some(&svg[start..start + end])
}

#[test]
fn should_return_edge_curve_when_it_is_a_string() {
    assert_eq!(Curve::from_name("linear"), Some(Curve::Linear));
    assert_eq!(Curve::from_name("basis"), Some(Curve::Basis));
    assert_eq!(Curve::from_name("cardinal"), Some(Curve::Cardinal));
    // `rounded` is not a d3 curve, so there is no `Curve` for it: the path builder takes the name
    // and draws the rounded corners.
    assert_eq!(Curve::from_name("rounded"), None);
    let edge = Edge {
        id: "L_a_b_0".to_owned(),
        start: "a".to_owned(),
        end: "b".to_owned(),
        curve: Some("rounded".to_owned()),
        points: vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        ..Edge::default()
    };
    let painted = paint_paths(
        std::slice::from_ref(&edge),
        "svg",
        &MermaidConfig::default(),
    );
    let d = d_attribute(&painted[0].svg).expect("a d attribute");
    assert!(d.contains('Q'), "{d}");
}

#[test]
#[ignore = "port: swimlane clipping (the flowchart port is ELK only)"]
fn takes_the_swimlane_path_when_the_layout_data_says_so_and_the_config_disagrees() {}

#[test]
#[ignore = "port: swimlane clipping (the flowchart port is ELK only)"]
fn would_empty_a_two_point_route_if_it_went_by_the_configured_layout_instead() {}

#[test]
#[ignore = "port: config fallback of resolveEdgeCurveType is not ported"]
fn should_fall_back_to_config_flowchart_curve_when_edge_curve_is_undefined() {}

#[test]
#[ignore = "port: config fallback of resolveEdgeCurveType is not ported"]
fn should_fall_back_to_config_flowchart_curve_when_edge_curve_is_not_a_string_d3_function() {}

#[test]
#[ignore = "port: config fallback of resolveEdgeCurveType is not ported"]
fn should_fall_back_to_config_flowchart_curve_when_edge_curve_is_null() {}

#[test]
#[ignore = "port: waits for the label layout (task T2)"]
fn accounts_for_bbox_x_y_offsets_when_centering_svg_label_html_labels_false() {}

#[test]
#[ignore = "port: waits for the label layout (task T2)"]
fn centers_svg_label_correctly_when_bbox_origin_is_at_zero_no_background_offset() {}

#[test]
#[ignore = "port: HTML labels are not in the flowchart port"]
fn centers_html_label_using_only_width_height_ignores_bbox_x_y_when_html_labels_true() {}

#[test]
#[ignore = "port: swimlane clipping (the flowchart port is ELK only)"]
fn honors_duplicated_endpoint_pins_instead_of_recomputing_polygon_intersections() {}

#[test]
#[ignore = "port: swimlane clipping (the flowchart port is ELK only)"]
fn still_clips_source_endpoints_to_the_rendered_shape_boundary() {}

#[test]
#[ignore = "port: swimlane clipping (the flowchart port is ELK only)"]
fn leaves_exact_endpoints_alone_when_the_edge_skips_intersection() {}

#[test]
#[ignore = "port: terminal labels are not in the flowchart model"]
fn never_sizes_the_terminal_label_box_smaller_than_the_measured_label() {}

#[test]
#[ignore = "port: terminal labels are not in the flowchart model"]
fn keeps_the_width_reserved_per_character_for_short_labels() {}
