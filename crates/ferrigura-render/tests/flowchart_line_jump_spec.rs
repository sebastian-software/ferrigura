//! Port of `rendering-elements/lineJump.spec.ts`: where edges cross, and the path each hop gives the
//! edge that gives way.
//!
//! The reference maps onto the Rust API as follows: `findEdgeIntersections` is
//! `find_edge_intersections`, and `processEdgesWithJumps` is `apply_jumps`, which returns only the
//! edges it rewrote. `isStraightPath` is not in the Rust API. `applyLineJumpsToSvg` works on the
//! DOM, so its cases are ignored, except the dash array, which `rewrite_dash_array` takes as a
//! length. The reference's `enabled` flag is `elk.lineHops` in `paint_paths`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_render::unified::data::Edge;
use ferrigura_render::unified::edges::paint_paths;
use ferrigura_render::unified::geom::Point;
use ferrigura_render::unified::line_jumps::{
    EdgeGeom, JumpStyle, LineJumpConfig, Rewrite, apply_jumps, curve_supports_line_hops,
    find_edge_intersections, rewrite_dash_array,
};

const ARC: LineJumpConfig = LineJumpConfig {
    jump_radius: 1.0,
    jump_style: JumpStyle::Arc,
};

/// The radius of the production hops, with the arc style.
const ROOMY: LineJumpConfig = LineJumpConfig {
    jump_radius: 6.0,
    jump_style: JumpStyle::Arc,
};

/// An edge whose curve the layout did not set. The reference's `!curve` takes hops for it, and the
/// empty string does the same here: `None` stands for a curve that is a function, which refuses them.
fn edge(id: &str, points: &[(f64, f64)]) -> EdgeGeom {
    EdgeGeom {
        id: id.to_owned(),
        points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        curve: Some(String::new()),
        arrow_type_start: "none".to_owned(),
        arrow_type_end: "none".to_owned(),
        style: String::new(),
    }
}

/// An edge that the layout routed with the `rounded` curve.
fn rounded(id: &str, points: &[(f64, f64)]) -> EdgeGeom {
    EdgeGeom {
        curve: Some("rounded".to_owned()),
        ..edge(id, points)
    }
}

/// The `d` that the hops give the edge at `index`, if they rewrote it.
fn rewritten(rewrites: &[Rewrite], index: usize) -> Option<&str> {
    rewrites
        .iter()
        .find(|rewrite| rewrite.index == index)
        .map(|rewrite| rewrite.d.as_str())
}

/// The `d` attribute of the `<path>` element in an edge's SVG.
fn d_attribute(svg: &str) -> &str {
    let start = svg.find(" d=\"").expect("a d attribute") + " d=\"".len();
    let end = svg[start..].find('"').expect("the end of d");
    &svg[start..start + end]
}

/// A straight edge of the layout, with the `linear` curve, for `paint_paths`.
fn painted_edge(id: &str, points: &[(f64, f64)]) -> Edge {
    Edge {
        id: id.to_owned(),
        start: "a".to_owned(),
        end: "b".to_owned(),
        curve: Some("linear".to_owned()),
        points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        ..Edge::default()
    }
}

/// The geometry of `elk-edge-cases/many-subgraphs-and-edges`: `design-system -> mermaid-chart-app`
/// turns east again at (430.6, 180.1), and `infrastructure -> auth-service` crosses it 10px later.
fn cramped() -> Vec<EdgeGeom> {
    vec![
        rounded(
            "designSystemToApp",
            &[
                (395.6, 275.3),
                (430.6, 275.3),
                (430.6, 180.1),
                (502.1, 180.1),
            ],
        ),
        rounded("infrastructureToAuth", &[(440.6, 120.0), (440.6, 320.0)]),
    ]
}

/// The geometry of `org -> platform` and `design -> app` from `knsv2.html`. The turn of
/// `org -> platform` is at `turn_y`: the crossing sits inside its corner unless the turn is high.
fn across_a_corner(turn_y: f64) -> Vec<EdgeGeom> {
    vec![
        rounded(
            "orgToPlatform",
            &[
                (169.0, turn_y),
                (249.0, turn_y),
                (249.0, 1345.091),
                (552.5, 1345.091),
            ],
        ),
        rounded(
            "designToApp",
            &[
                (229.0, 1382.007),
                (229.0, 1033.625),
                (269.0, 1033.625),
                (351.5, 1033.625),
            ],
        ),
    ]
}

#[test]
fn detects_a_single_perpendicular_crossing_and_assigns_the_jump_to_the_horizontal_edge() {
    let edges = [
        edge("e1", &[(0.0, 5.0), (10.0, 5.0)]),
        edge("e2", &[(5.0, 0.0), (5.0, 10.0)]),
    ];
    let crossings = find_edge_intersections(&edges);
    assert_eq!(crossings.len(), 1);
    let crossing = &crossings[0];
    // Orthogonal rule: the horizontal edge hops over the vertical one.
    assert_eq!(crossing.jump_edge, 0);
    assert_eq!(crossing.other_edge, 1);
    assert_eq!(crossing.point, Point::new(5.0, 5.0));
}

#[test]
fn keeps_horizontal_hops_over_vertical_even_when_input_order_is_reversed() {
    let edges = [
        edge("vertical", &[(5.0, 0.0), (5.0, 10.0)]),
        edge("horizontal", &[(0.0, 5.0), (10.0, 5.0)]),
    ];
    let crossings = find_edge_intersections(&edges);
    assert_eq!(crossings.len(), 1);
    // Still the horizontal edge that gets the jump, whatever the order.
    assert_eq!(crossings[0].jump_edge, 1);
    assert_eq!(crossings[0].other_edge, 0);
}

#[test]
fn falls_back_to_later_index_wins_when_both_segments_share_orientation() {
    // Two diagonal edges crossing each other: both are horizontal-dominant, so the tie goes to the
    // later edge in the input.
    let edges = [
        edge("a", &[(0.0, 0.0), (10.0, 2.0)]),
        edge("b", &[(0.0, 2.0), (10.0, 0.0)]),
    ];
    let crossings = find_edge_intersections(&edges);
    assert_eq!(crossings.len(), 1);
    assert_eq!(crossings[0].jump_edge, 1);
    assert_eq!(crossings[0].other_edge, 0);
}

#[test]
fn does_not_treat_a_t_junction_endpoint_touching_another_edge_as_a_crossing() {
    // `e2` ends exactly on `e1`, like an arrowhead landing on a backbone edge.
    let edges = [
        edge("e1", &[(0.0, 5.0), (10.0, 5.0)]),
        edge("e2", &[(5.0, 0.0), (5.0, 5.0)]),
    ];
    assert!(find_edge_intersections(&edges).is_empty());
}

#[test]
fn does_not_treat_shared_start_points_as_crossings() {
    // Two edges fanning out from the same node.
    let edges = [
        edge("e1", &[(0.0, 0.0), (10.0, 5.0)]),
        edge("e2", &[(0.0, 0.0), (10.0, -5.0)]),
    ];
    assert!(find_edge_intersections(&edges).is_empty());
}

#[test]
fn does_not_treat_shared_end_points_as_crossings() {
    // Two edges converging on the same node.
    let edges = [
        edge("e1", &[(0.0, 0.0), (10.0, 5.0)]),
        edge("e2", &[(0.0, 10.0), (10.0, 5.0)]),
    ];
    assert!(find_edge_intersections(&edges).is_empty());
}

#[test]
fn ignores_parallel_non_intersecting_segments() {
    let edges = [
        edge("e1", &[(0.0, 0.0), (10.0, 0.0)]),
        edge("e2", &[(0.0, 5.0), (10.0, 5.0)]),
    ];
    assert!(find_edge_intersections(&edges).is_empty());
}

#[test]
fn leaves_the_vertical_edge_untouched_and_puts_the_arc_jump_on_the_horizontal_edge() {
    let edges = [
        edge("h", &[(0.0, 5.0), (10.0, 5.0)]),
        edge("v", &[(5.0, 0.0), (5.0, 10.0)]),
    ];
    let rewrites = apply_jumps(&edges, &ARC);
    // Only the horizontal edge is rewritten: the vertical one keeps its path.
    assert_eq!(rewrites.len(), 1);
    // It carries a unit-radius arc that bumps up at the crossing.
    assert_eq!(
        rewritten(&rewrites, 0),
        Some("M0,5 L4,5 A1,1 0 0 1 6,5 L10,5")
    );
}

#[test]
fn emits_two_arcs_in_t_order_when_a_horizontal_edge_crosses_multiple_verticals() {
    let edges = [
        edge("v1", &[(3.0, 0.0), (3.0, 10.0)]),
        edge("v2", &[(7.0, 0.0), (7.0, 10.0)]),
        edge("h", &[(0.0, 5.0), (10.0, 5.0)]),
    ];
    let rewrites = apply_jumps(&edges, &ARC);
    assert_eq!(rewrites.len(), 1);
    assert_eq!(
        rewritten(&rewrites, 2),
        Some("M0,5 L2,5 A1,1 0 0 1 4,5 L6,5 A1,1 0 0 1 8,5 L10,5")
    );
}

#[test]
fn shrinks_jump_radii_when_adjacent_crossings_would_overlap() {
    // Two crossings 1.6 apart on a horizontal segment, with radius 1.0. The arcs must shrink to
    // fit: each at most half the gap, and the path stays monotonic along the segment.
    let edges = [
        edge("v1", &[(4.2, 0.0), (4.2, 10.0)]),
        edge("v2", &[(5.8, 0.0), (5.8, 10.0)]),
        edge("h", &[(0.0, 5.0), (10.0, 5.0)]),
    ];
    let rewrites = apply_jumps(&edges, &ARC);
    let d = rewritten(&rewrites, 2).expect("the horizontal edge is rewritten");
    assert!(d.starts_with("M0,5 "), "{d}");
    assert!(d.ends_with(" L10,5"), "{d}");
    assert_eq!(d.matches('A').count(), 2, "{d}");
    let radius: f64 = d
        .split(' ')
        .find_map(|part| part.strip_prefix('A'))
        .and_then(|arc| arc.split(',').next())
        .and_then(|value| value.parse().ok())
        .expect("an arc radius");
    assert!(radius <= 0.8, "{d}");
    assert!(radius > 0.0, "{d}");
}

#[test]
fn drops_both_hops_when_crossings_are_too_close_to_carry_one_each() {
    // Two crossings 0.6 apart: each would get 0.3, which does not clear the stroke it hops.
    let edges = [
        edge("v1", &[(4.7, 0.0), (4.7, 10.0)]),
        edge("v2", &[(5.3, 0.0), (5.3, 10.0)]),
        edge("h", &[(0.0, 5.0), (10.0, 5.0)]),
    ];
    let rewrites = apply_jumps(&edges, &ARC);
    assert_eq!(rewritten(&rewrites, 2), Some("M0,5 L10,5"));
}

#[test]
fn returns_plain_polylines_for_every_edge_when_disabled() {
    let edges = [
        painted_edge("h", &[(0.0, 50.0), (100.0, 50.0)]),
        painted_edge("v", &[(50.0, 0.0), (50.0, 100.0)]),
    ];
    let hopped = paint_paths(&edges, "svg", &MermaidConfig::default());
    assert!(
        d_attribute(&hopped[0].svg).contains("A6,6"),
        "{}",
        hopped[0].svg
    );

    let mut config = MermaidConfig::default();
    config.elk.line_hops = Some(Value::from(serde_json::json!(false)));
    let plain = paint_paths(&edges, "svg", &config);
    assert!(
        !d_attribute(&plain[0].svg).contains('A'),
        "{}",
        plain[0].svg
    );
    assert_eq!(d_attribute(&plain[1].svg), d_attribute(&hopped[1].svg));
}

#[test]
fn emits_an_m_move_on_the_horizontal_edge_when_jump_style_is_gap() {
    let edges = [
        edge("h", &[(0.0, 5.0), (10.0, 5.0)]),
        edge("v", &[(5.0, 0.0), (5.0, 10.0)]),
    ];
    let gap = LineJumpConfig {
        jump_radius: 1.0,
        jump_style: JumpStyle::Gap,
    };
    let rewrites = apply_jumps(&edges, &gap);
    assert_eq!(rewrites.len(), 1);
    // The gap variant lifts the pen and resumes on the far side of the crossing.
    assert_eq!(rewritten(&rewrites, 0), Some("M0,5 L4,5 M6,5 L10,5"));
}

#[test]
#[ignore = "port: not in flowchart port (is_straight_path is removed; the curve name decides)"]
fn accepts_pure_m_l_paths() {}

#[test]
#[ignore = "port: not in flowchart port (is_straight_path is removed; the curve name decides)"]
fn rejects_paths_containing_curve_commands() {}

#[test]
#[ignore = "port: DOM (applyLineJumpsToSvg)"]
fn rewrites_the_d_attribute_of_the_horizontal_edge_for_a_perpendicular_crossing() {}

#[test]
#[ignore = "port: DOM (applyLineJumpsToSvg)"]
fn rewrites_a_rounded_edge_whose_rendered_d_contains_corner_rounding_q_commands() {}

#[test]
#[ignore = "port: DOM (applyLineJumpsToSvg)"]
fn skips_edges_whose_curve_hint_is_a_true_smoothing_curve() {}

#[test]
#[ignore = "port: DOM (applyLineJumpsToSvg; the rendered d attribute)"]
fn skips_edges_whose_rendered_d_contains_curve_commands() {}

#[test]
#[ignore = "port: DOM (data-points decoding)"]
fn prefers_data_points_the_geometry_edges_js_actually_rendered_over_edge_points() {}

#[test]
#[ignore = "port: DOM (applyLineJumpsToSvg)"]
fn applies_marker_offsets_so_the_rewritten_endpoint_matches_edges_js() {}

#[test]
fn recomputes_stroke_dasharray_against_the_new_total_length_to_preserve_neo_look_marker_clearance()
{
    // The reference reads the length from the DOM (`getTotalLength`, stubbed to 60). Here the
    // length is an argument: the hidden ends (9 and 9) stay, and the stroke between them takes
    // the rest.
    let style = "stroke-dasharray: 0 9 42 9; stroke-dashoffset: 0; stroke: #000;";
    assert_eq!(
        rewrite_dash_array(style, 60.0).as_deref(),
        Some("stroke-dasharray: 0 9 42 9; stroke-dashoffset: 0; stroke: #000;")
    );
    assert_eq!(
        rewrite_dash_array(style, 100.0).as_deref(),
        Some("stroke-dasharray: 0 9 82 9; stroke-dashoffset: 0; stroke: #000;")
    );
}

#[test]
#[ignore = "port: DOM (applyLineJumpsToSvg; there is no enabled flag in apply_jumps)"]
fn does_nothing_when_disabled() {}

#[test]
fn leaves_the_crossing_alone_rather_than_drawing_an_undersized_arc() {
    let edges = cramped();
    // The crossing IS found: this is about what gets drawn for it.
    assert_eq!(find_edge_intersections(&edges).len(), 1);

    let rewrites = apply_jumps(&edges, &ROOMY);
    let d = rewritten(&rewrites, 0).expect("the edge with the crossing is rewritten");
    // No arc anywhere on the path, and no zero-length `L` parked on the corner's tangent point.
    assert!(!d.contains('A'), "{d}");
    assert!(!d.contains("L437.696,180.086 L437.696,180.086"), "{d}");
}

#[test]
fn still_hops_once_the_bend_is_far_enough_away() {
    // The same edge and crossing, but the turn moved back, so the lane is 20px instead of 10px.
    let mut edges = cramped();
    edges[0] = rounded(
        "designSystemToApp",
        &[
            (395.6, 275.3),
            (420.6, 275.3),
            (420.6, 180.1),
            (502.1, 180.1),
        ],
    );
    let rewrites = apply_jumps(&edges, &ROOMY);
    let d = rewritten(&rewrites, 0).expect("the edge with the crossing is rewritten");
    assert!(d.contains("A6,6 0 0 1"), "{d}");
}

#[test]
fn ignores_a_crossing_that_lands_inside_the_other_edges_corner() {
    // Nothing is wrong with the hopping edge here: its own bend is 20px back, so it could take a
    // full 6px arc. The problem is the crossing, which sits inside the other edge's corner.
    let edges = across_a_corner(1031.102);
    assert!(find_edge_intersections(&edges).is_empty());
    assert!(rewritten(&apply_jumps(&edges, &ROOMY), 1).is_none());
}

#[test]
fn hops_normally_once_that_corner_is_out_of_the_way() {
    // The other edge turns 30px higher, so by y=1033.625 it has settled onto its vertical.
    let edges = across_a_corner(1001.102);
    assert_eq!(find_edge_intersections(&edges).len(), 1);
    let rewrites = apply_jumps(&edges, &ROOMY);
    let d = rewritten(&rewrites, 1).expect("the crossed edge is rewritten");
    assert!(d.contains("A6,6"), "{d}");
}

#[test]
fn keeps_a_straight_run_between_the_corner_and_a_hop_it_does_draw() {
    // The bend is at x=100, so its rounding ends at x=107.07. The crossing at x=113 leaves 5.93px,
    // and the hop must keep 2px of straight line before it.
    let edges = [
        rounded("bend", &[(100.0, 0.0), (100.0, 100.0), (300.0, 100.0)]),
        edge("crosser", &[(113.0, 0.0), (113.0, 200.0)]),
    ];
    let rewrites = apply_jumps(&edges, &ROOMY);
    let d = rewritten(&rewrites, 0).expect("the bend is rewritten");

    let parts: Vec<&str> = d.split(' ').collect();
    let arc_at = parts
        .iter()
        .position(|part| part.starts_with('A'))
        .unwrap_or_else(|| panic!("an arc in {d}"));
    let open_at: f64 = parts[arc_at - 1]
        .strip_prefix('L')
        .and_then(|rest| rest.split(',').next())
        .and_then(|value| value.parse().ok())
        .expect("the straight run before the arc");
    let radius: f64 = parts[arc_at]
        .strip_prefix('A')
        .and_then(|rest| rest.split(',').next())
        .and_then(|value| value.parse().ok())
        .expect("the radius of the arc");

    // Radius gives way, not the clearance: 2px of straight line survives before the arc.
    assert!((open_at - 107.071 - 2.0).abs() < 0.005, "{d}");
    assert!((radius - 3.929).abs() < 0.005, "{d}");
}

#[test]
fn the_curve_hint_decides_which_edges_take_hops() {
    for curve in ["linear", "rounded", "step", "stepBefore", "stepAfter", ""] {
        assert!(curve_supports_line_hops(curve), "{curve:?}");
    }
    for curve in ["basis", "monotoneX", "cardinal", "natural"] {
        assert!(!curve_supports_line_hops(curve), "{curve:?}");
    }
    // A smoothing curve on the horizontal edge takes no hop, though the crossing is there.
    let mut horizontal = edge("h", &[(0.0, 5.0), (10.0, 5.0)]);
    horizontal.curve = Some("basis".to_owned());
    let vertical = edge("v", &[(5.0, 0.0), (5.0, 10.0)]);
    assert_eq!(
        find_edge_intersections(&[horizontal.clone(), vertical.clone()]).len(),
        1
    );
    assert!(apply_jumps(&[horizontal, vertical], &ARC).is_empty());
}
