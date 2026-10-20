//! The render of a swimlane diagram: the structure the reference's renderer writes (`flowRenderer-v3-unified.ts`,
//! `clusters/swimlane.js`, `common/index.ts`, `edgeLabelNodes.ts`), and one case for each quirk the port
//! reproduces in the output: the lanes in reverse source order, the body and then the title band, the title
//! of an `LR` lane turned, the default lane for the loose nodes, the label node of a labelled edge (the
//! label is drawn as a node, its text starts at its origin and is shifted by half its width), the virtual
//! edges that are never drawn, the clipping of the edges to the outlines of their nodes, the line hops and
//! their styles, and the layouts that are refused.
//!
//! The expected numbers that are exact in the reference's output (the edge ends at a node's outline, the
//! lanes' title width) are the reference's, as its Chromium render gives them.

#[path = "support/swimlanes.rs"]
#[allow(dead_code)]
mod support;

use ferrigura_render::RenderError;
use ferrigura_render::unified::edges::swimlane_points;
use ferrigura_render::unified::geom::Point;
use regex::Regex;
use support::{Setup, config_with, count, fixture, svg};

/// A lane's `data-id`, and the first rectangle of its `class`, as the numbers of its `x`, `y`, `width`
/// and `height`.
fn lane_rect(svg: &str, lane: &str, class: &str) -> Vec<f64> {
    let group = svg
        .find(&format!("data-id=\"{lane}\""))
        .unwrap_or_else(|| panic!("no lane {lane}"));
    let from = &svg[group..];
    let at = from
        .find(&format!("class=\"{class}\""))
        .unwrap_or_else(|| panic!("no {class} of {lane}"));
    let rect = &from[at..from[at..].find("/>").map_or(from.len(), |end| at + end)];
    ["x", "y", "width", "height"]
        .iter()
        .map(|name| attribute(rect, name))
        .collect()
}

/// A numeric attribute of an element's text.
fn attribute(element: &str, name: &str) -> f64 {
    let pattern = Regex::new(&format!(" {name}=\"([^\"]*)\"")).expect("a pattern");
    pattern
        .captures(element)
        .unwrap_or_else(|| panic!("no {name} in {element}"))[1]
        .parse()
        .expect("a number")
}

/// The arc commands of the paths of an SVG: the line hops are the arcs of the paths' `d` attributes.
fn arcs(svg: &str) -> usize {
    let pattern = Regex::new(" d=\"([^\"]*)\"").expect("a pattern");
    pattern
        .captures_iter(svg)
        .map(|capture| count(&capture[1], "A"))
        .sum()
}

/// The first `d` of the path whose `data-id` is `edge`.
fn path_of(svg: &str, edge: &str) -> String {
    let at = svg
        .find(&format!("data-id=\"{edge}\""))
        .unwrap_or_else(|| panic!("no edge {edge}"));
    let start = svg[..at].rfind(" d=\"").expect("a d") + " d=\"".len();
    let end = start + svg[start..].find('"').expect("the end of d");
    svg[start..end].to_owned()
}

const SIMPLE: &str = "swimlane-beta LR\n  subgraph Dep1\n   Start\n   Process1\n  end\n  subgraph Dep2\n  Process2\n  Process3\n  end\n\n  Start --> Process1 --> Process2 --> Process3";

#[test]
fn the_root_element_is_a_swimlane_flowchart_with_its_twelve_markers() {
    let svg = svg(SIMPLE);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"p0\""));
    assert!(svg.contains("class=\"flowchart\""));
    // `aria-roledescription` is the diagram's own name, not the flowchart's (`flowRenderer-v3-unified.ts`).
    assert!(svg.contains("aria-roledescription=\"swimlane\""));
    // `insertMarkers(['point', 'circle', 'cross'], 'swimlane')`: the markers carry the diagram's type.
    assert_eq!(count(&svg, "<marker "), 12);
    assert_eq!(count(&svg, "class=\"marker cross swimlane\""), 4);
    assert!(svg.contains("id=\"p0_swimlane-pointEnd-margin\""));
}

#[test]
fn the_lanes_are_drawn_in_the_reverse_of_their_source_order() {
    // The reference inserts the lanes last-first (`toGraphView`'s reversal of the groups, and the DB's order
    // of the subgraphs): `Dep2` is painted before `Dep1`, though the source lists `Dep1` first.
    let svg = svg(SIMPLE);
    let dep2 = svg.find("data-id=\"Dep2\"").expect("the lane Dep2");
    let dep1 = svg.find("data-id=\"Dep1\"").expect("the lane Dep1");
    assert!(dep2 < dep1, "Dep2 must come before Dep1");
}

#[test]
fn a_lane_is_its_body_then_its_title_band_then_its_label() {
    // `swimlane.js` inserts each rectangle as the first child, so the body, inserted last, comes first.
    let svg = svg(SIMPLE);
    let lane = svg.find("data-id=\"Dep1\"").expect("the lane");
    let from = &svg[lane..];
    let body = from.find("class=\"swimlane-body\"").expect("the body");
    let title = from
        .find("class=\"swimlane-title\"")
        .expect("the title band");
    let label = from
        .find("class=\"cluster-label swimlane-label\"")
        .expect("the label");
    assert!(
        body < title && title < label,
        "body, title band and label in that order"
    );
    assert!(
        svg.contains("class=\"cluster swimlane \""),
        "the lane's class, with the reference's trailing space"
    );
}

#[test]
fn a_left_to_right_lane_turns_its_label_and_its_title_band_is_a_strip_on_the_left() {
    // `LR`: the title band is a vertical strip of the label's height plus 8 (`titlePaddingY` of 4 on each
    // side), and the label is turned by `rotate(-90)`.
    let svg = svg(SIMPLE);
    assert!(svg.contains("rotate(-90)"));
    let title = lane_rect(&svg, "Dep1", "swimlane-title");
    assert_eq!(
        title[2], 25.0,
        "a one-line title is 17 px high, with 4 px on each side"
    );
    // The band is as tall as its lane.
    let body = lane_rect(&svg, "Dep1", "swimlane-body");
    assert_eq!(title[3], body[3]);
}

#[test]
fn a_top_to_bottom_lane_has_its_title_band_above_its_body() {
    // `TB`: the band is on top, and the body starts where the band ends, so that the borders touch.
    let svg = svg("swimlane-beta TB\n  subgraph Lane\n    A\n  end\n  A --> B");
    assert!(!svg.contains("rotate(-90)"));
    let title = lane_rect(&svg, "Lane", "swimlane-title");
    let body = lane_rect(&svg, "Lane", "swimlane-body");
    assert_eq!(
        body[1],
        title[1] + title[3],
        "the body starts at the bottom of the band"
    );
    assert!(title[3] < body[3], "the band is shorter than the body");
}

#[test]
fn the_loose_nodes_go_into_a_default_lane_in_the_next_colour_slot() {
    // `prepareLayoutForSwimlanes`: the nodes outside every subgraph make a lane, `__swimlane_default__`,
    // with padding 20 and the next palette slot after the largest one any node has.
    let svg = svg("swimlane-beta LR\n  subgraph X\n    A\n  end\n  B --> A");
    assert!(svg.contains("data-id=\"__swimlane_default__\""));
    assert!(
        svg.contains("data-color-id=\"color-1\""),
        "X is slot 0, so the default lane is slot 1"
    );
}

#[test]
fn a_diagram_without_loose_nodes_has_no_default_lane() {
    let svg = svg(SIMPLE);
    assert!(!svg.contains("__swimlane_default__"));
}

#[test]
fn a_labelled_edge_is_drawn_as_a_label_node_and_not_as_an_edge_label() {
    // `createEdgeLabelNodes`: the label moves to a node of the layout, which is painted in the nodes group.
    let svg = fixture_svg("pr-check/9-edge-labels.mmd");
    assert!(
        svg.contains("<g class=\"edgeLabels\"></g>"),
        "no edge label is drawn"
    );
    assert!(svg.contains("edge-label-Start-Process1-L_Start_Process1_0"));
    assert!(svg.contains("class=\"label edgeLabel\""));
}

#[test]
fn a_label_node_text_starts_at_its_origin_and_is_shifted_by_half_its_width() {
    // `labelRect.ts`: the label's lines start at its origin, and the group moves by minus half the label's
    // size, so the label is centred on the node. "Yes" is 21.28 px wide: the shift is -10.64.
    let svg = fixture_svg("pr-check/9-edge-labels.mmd");
    let at = svg
        .find("class=\"label edgeLabel\"")
        .expect("the label node");
    let node = &svg[at..];
    assert!(
        node.contains("<g class=\"label\" transform=\"translate(-10.640625, -9.5)\">"),
        "{node}"
    );
    assert!(
        node.contains("<tspan class=\"text-outer-tspan row\" x=\"0\""),
        "the line starts at x 0"
    );
}

#[test]
fn an_edge_is_clipped_to_the_outline_of_its_nodes() {
    // The reference's `insertEdge` clips a straight edge to both nodes: Start is 152 px wide and centred at 36,
    // so the edge starts at its right side, x = 112.
    let svg = svg(SIMPLE);
    assert!(
        path_of(&svg, "L_Start_Process1_0").starts_with("M112,0"),
        "{}",
        path_of(&svg, "L_Start_Process1_0")
    );
}

#[test]
fn a_virtual_edge_of_a_label_is_never_drawn() {
    // The two `-to-label` and `-from-label` edges are laid out and not drawn (`isLayoutOnly`).
    let svg = fixture_svg("pr-check/9-edge-labels.mmd");
    assert_eq!(
        count(&svg, "data-edge=\"true\""),
        2,
        "the two edges of the loop, and no virtual edge"
    );
    assert!(!svg.contains("-to-label"));
}

#[test]
fn every_lane_has_one_label() {
    let svg = svg(SIMPLE);
    assert_eq!(count(&svg, "class=\"cluster-label swimlane-label\""), 2);
}

#[test]
fn the_palette_gives_each_lane_its_colour_slot_on_a_colour_theme() {
    let svg = svg(SIMPLE);
    assert!(svg.contains("data-color-id=\"color-0\""));
    assert!(svg.contains("data-color-id=\"color-1\""));
}

#[test]
fn a_lane_on_a_theme_without_a_palette_has_no_colour_slot() {
    let setup = Setup::new(
        config_with(serde_json::json!({ "theme": "default" })),
        Some(serde_json::json!({ "clusterBkg": "#ECECFF", "clusterBorder": "#9370DB" })),
    );
    let svg = setup.svg(SIMPLE, "p0");
    // The style sheet names the attribute in its exemption rule; no element carries a colour slot.
    assert!(!svg.contains("data-color-id=\"color-"));
    assert!(svg.contains("class=\"cluster swimlane"));
}

#[test]
fn the_title_is_centred_above_the_drawing_at_the_title_margin() {
    // `utils.insertTitle` with `titleTopMargin` (25 by default).
    let svg = svg(&format!("---\ntitle: Flow\n---\n{SIMPLE}"));
    assert!(
        svg.contains("y=\"-25\" class=\"flowchartTitleText\">Flow</text>"),
        "{svg}"
    );
}

#[test]
fn a_line_hop_is_an_arc_by_default_and_none_when_the_option_is_false() {
    // `applySwimlaneLineJumps`: `swimlane.lineHops` is `arc` by default; `false` draws no hop.
    let text = fixture("pr-check/7-car-sales-constr.mmd");
    assert!(
        arcs(&svg(&text)) > 0,
        "the crossing edges of the fixture draw an arc"
    );
    let off = Setup::new(
        config_with(serde_json::json!({ "swimlane": { "lineHops": false } })),
        None,
    );
    assert_eq!(arcs(&off.svg(&text, "p0")), 0);
}

#[test]
fn a_gap_line_hop_draws_no_arc() {
    // `lineHops: 'gap'`: the crossed edge is broken, and there is no arc command (`adjustLayout.ts` 9-14).
    let text = fixture("pr-check/7-car-sales-constr.mmd");
    let gap = Setup::new(
        config_with(serde_json::json!({ "swimlane": { "lineHops": "gap" } })),
        None,
    );
    assert_eq!(arcs(&gap.svg(&text, "p0")), 0);
}

#[test]
fn an_elk_layout_draws_the_diagram_without_lanes() {
    // `layout: elk` is a registered engine: the diagram is laid out as a flowchart, with no lane shape.
    let setup = Setup::new(config_with(serde_json::json!({ "layout": "elk" })), None);
    let svg = setup.svg(SIMPLE, "p0");
    assert!(!svg.contains("class=\"swimlane-body\""));
    assert!(svg.contains("class=\"cluster "));
}

#[test]
fn a_registered_layout_the_port_does_not_implement_is_refused() {
    let setup = Setup::new(
        config_with(serde_json::json!({ "layout": "cose-bilkent" })),
        None,
    );
    assert!(matches!(
        setup.try_svg(SIMPLE, "p0"),
        Err(RenderError::Unsupported(_))
    ));
}

#[test]
fn a_non_default_lane_option_is_refused_by_the_engine() {
    // `ignoreCrossLaneEdges: false` is the gravity layering, which the port does not implement.
    let setup = Setup::new(
        config_with(serde_json::json!({ "swimlane": { "ignoreCrossLaneEdges": false } })),
        None,
    );
    match setup.try_svg(SIMPLE, "p0") {
        Err(RenderError::Layout(message)) => {
            assert!(message.contains("ignoreCrossLaneEdges"), "{message}")
        }
        other => panic!("expected a layout error, got {other:?}"),
    }
}

#[test]
fn a_label_node_sits_in_the_lane_of_its_source_when_the_edge_stays_in_the_lane() {
    // `createEdgeLabelNodes`: the label is in the source's lane, unless the edge crosses lanes; then it is in the
    // target's. Here the edge crosses from `A` to `B`, so the label is in `B`'s lane.
    let svg = svg(
        "swimlane-beta TB\n  subgraph A\n    X\n  end\n  subgraph B\n    Y\n  end\n  X -- Hop --> Y",
    );
    let label_y = label_node_y(&svg, "edge-label-X-Y-L_X_Y_0");
    let lane = lane_rect(&svg, "B", "swimlane-body");
    assert!(
        lane[1] <= label_y && label_y <= lane[1] + lane[3],
        "the label is in B: {label_y} in {lane:?}"
    );
}

/// The vertical position of a label node, from the `translate` of its group, which follows its id.
fn label_node_y(svg: &str, id: &str) -> f64 {
    let start = svg
        .find(&format!("-{id}\""))
        .unwrap_or_else(|| panic!("no label node {id}"));
    let group = start
        + svg[start..]
            .find("transform=\"translate(")
            .expect("the group's transform");
    let numbers: Vec<f64> = svg[group + "transform=\"translate(".len()..]
        .split(')')
        .next()
        .expect("the translate")
        .split(',')
        .map(|part| part.trim().parse().expect("a number"))
        .collect();
    numbers[1]
}

#[test]
fn the_engine_clips_a_straight_edge_to_both_outlines() {
    // `insertEdge`'s swimlane branch: a two-point edge has both of its points moved to the outlines.
    let clipped = swimlane_points(
        &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        |point| Point::new(point.x + 1.0, point.y),
        |point| Point::new(point.x - 1.0, point.y),
    );
    assert_eq!(clipped, vec![Point::new(1.0, 0.0), Point::new(9.0, 0.0)]);
}

#[test]
fn an_inner_point_that_the_clip_lands_on_is_dropped() {
    // Both clipped ends land within the tolerance of their inner points, so they are dropped, and the inner
    // points are kept: no segment is left with no length.
    let clipped = swimlane_points(
        &[
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(5.0, 10.0),
            Point::new(10.0, 10.0),
        ],
        |point| Point::new(point.x + 0.1, point.y),
        |point| point,
    );
    assert_eq!(clipped, vec![Point::new(5.0, 0.0), Point::new(5.0, 10.0)]);
}

#[test]
fn a_last_inner_point_within_the_tolerance_of_the_end_is_kept_without_clipping() {
    // A last segment that ends on its inner point is pinned: the head outline is not asked for it.
    let clipped = swimlane_points(
        &[
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(5.2, 0.1),
        ],
        |point| point,
        |_| panic!("a pinned end is not clipped"),
    );
    assert_eq!(clipped.last().copied(), Some(Point::new(5.0, 0.0)));
}

/// The SVG of a fixture of the swimlane e2e diagrams, with the default configuration.
fn fixture_svg(path: &str) -> String {
    svg(&fixture(path))
}
