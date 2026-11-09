//! Flowcharts through the public entry point: what the root element's box is.

use ferrigura::render;
use ferrigura::{FlowchartMode, RenderOptions, Renderer, flowchart_elk_stages};
use ferrigura_model::value::Value;

/// The tolerance of the comparison with the reference, in SVG user units.
const TOLERANCE: f64 = 0.05;

const REVIEW: &str = "flowchart TD\n draft[Draft] --> review{Review}\n review -->|Approved| publish[Publish]\n review -->|Changes| draft\n";

#[test]
fn unlabeled_parent_graphs_keep_their_existing_routing() {
    let renderer = Renderer::new(&Value::empty_object());
    for text in [
        "flowchart LR\n A{First} --> B{Second}\n A --> B\n A --> B\n",
        include_str!(
            "../../../e2e/diagrams/flowchart/nightly-check/shape-themes/02-connect-all-shapes-with-each-other.mmd"
        ),
    ] {
        assert_eq!(
            renderer
                .render_with(text, &options(FlowchartMode::Adaptive))
                .unwrap(),
            renderer
                .render_with(text, &options(FlowchartMode::Mermaid))
                .unwrap()
        );
    }
}

/// The node with an id in the measured input or the laid-out output.
fn elk_node<'a>(graph: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    if graph["id"] == id {
        return Some(graph);
    }
    graph["children"]
        .as_array()?
        .iter()
        .find_map(|child| elk_node(child, id))
}

fn assert_straight_routes(svg: &str, expected_count: usize) {
    let document = roxmltree::Document::parse(svg).unwrap();
    let paths: Vec<_> = document
        .descendants()
        .filter(|node| node.has_attribute("data-points"))
        .collect();
    assert_eq!(paths.len(), expected_count);
    for path in paths {
        let d = path.attribute("d").unwrap();
        assert!(
            d.chars()
                .all(|c| !c.is_ascii_alphabetic() || c == 'M' || c == 'L'),
            "{d}"
        );
        let numbers: Vec<f64> = d
            .split(['M', 'L', ','])
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        let points = numbers.as_chunks::<2>().0;
        assert!(points.len() >= 2, "{d}");
        let vertical = points
            .iter()
            .all(|p| (p[0] - points[0][0]).abs() < TOLERANCE);
        let horizontal = points
            .iter()
            .all(|p| (p[1] - points[0][1]).abs() < TOLERANCE);
        assert!(vertical || horizontal, "{d}");
    }
}

#[test]
fn nested_edge_labels_center_their_actual_container_in_every_direction() {
    for direction in ["TD", "BT", "LR", "RL"] {
        let body = REVIEW.split_once('\n').unwrap().1;
        let text = format!(
            "flowchart {direction}\n subgraph outer[Outer]\n subgraph workflow[Workflow]\n{body} end\n end\n"
        );
        let stages = flowchart_elk_stages(
            &text,
            &Value::empty_object(),
            &options(FlowchartMode::Adaptive),
        )
        .unwrap();
        let input = &stages["elkInput"];
        for id in ["root", "outer"] {
            assert!(
                elk_node(input, id).unwrap()["layoutOptions"]
                    .get("elk.spacing.portPort")
                    .is_none(),
                "{direction}: {id}"
            );
        }
        assert_eq!(
            elk_node(input, "workflow").unwrap()["layoutOptions"]["elk.spacing.portPort"],
            44.0
        );
        for id in ["draft", "review", "publish"] {
            assert_eq!(
                elk_node(input, id).unwrap()["layoutOptions"]["elk.portAlignment.default"],
                "CENTER",
                "{direction}: {id}"
            );
        }
        // In the vertical review loop, all three routes can stay straight inside both frames.
        if matches!(direction, "TD" | "BT") {
            let svg = Renderer::new(&Value::empty_object())
                .render_with(&text, &options(FlowchartMode::Adaptive))
                .unwrap();
            assert_straight_routes(&svg, 3);
        }
    }
}

#[test]
fn local_cross_hierarchy_and_self_loop_labels_use_their_own_containers() {
    let text = "flowchart TD\n subgraph left[Left]\n A -->|Local| B\n A -->|Repeat| A\n end\n subgraph right[Right]\n C --> D\n end\n subgraph plain[Plain]\n E --> F\n end\n B -->|Across| C\n";
    let stages = flowchart_elk_stages(
        text,
        &Value::empty_object(),
        &options(FlowchartMode::Adaptive),
    )
    .unwrap();
    let input = &stages["elkInput"];
    for id in ["root", "left", "right"] {
        assert_eq!(
            elk_node(input, id).unwrap()["layoutOptions"]["elk.spacing.portPort"],
            44.0,
            "{id}"
        );
    }
    for id in ["left", "right", "A", "B", "C", "D"] {
        assert_eq!(
            elk_node(input, id).unwrap()["layoutOptions"]["elk.portAlignment.default"],
            "CENTER",
            "{id}"
        );
    }
    for id in ["E", "F"] {
        assert!(
            elk_node(input, id).unwrap()["layoutOptions"]
                .get("elk.portAlignment.default")
                .is_none(),
            "unrelated plain container: {id}"
        );
    }
    assert!(
        elk_node(input, "plain").unwrap()["layoutOptions"]
            .get("elk.spacing.portPort")
            .is_none()
    );
}

#[test]
fn wide_inline_labels_have_room_for_the_neighboring_vertical_route() {
    let source = include_str!(
        "../../../e2e/diagrams/flowchart/regressions/routing-subgraph-long-review-label-td.mmd"
    );
    // Check an inherited vertical direction, and an explicit vertical direction in a horizontal graph.
    for text in [
        source.to_owned(),
        source.replace("flowchart TD", "flowchart LR").replace(
            "subgraph workflow[Workflow]",
            "subgraph workflow[Workflow]\n direction TB",
        ),
    ] {
        let stages = flowchart_elk_stages(
            &text,
            &Value::empty_object(),
            &options(FlowchartMode::Adaptive),
        )
        .unwrap();
        let gap = elk_node(&stages["elkInput"], "workflow").unwrap()["layoutOptions"]["elk.spacing.portPort"].as_f64().unwrap();
        assert!(
            gap > 60.0,
            "the long label needs more than the 44-unit height-based gap: {gap}"
        );
        let svg = Renderer::new(&Value::empty_object())
            .render_with(&text, &options(FlowchartMode::Adaptive))
            .unwrap();
        assert_straight_routes(&svg, 3);
    }
}

#[test]
fn a_merged_subgraphs_inert_direction_does_not_change_the_clearance_axis() {
    let source = include_str!(
        "../../../e2e/diagrams/flowchart/regressions/routing-subgraph-long-review-label-td.mmd"
    );
    let text = source.replace("flowchart TD", "flowchart LR").replace(
        "subgraph workflow[Workflow]",
        "subgraph workflow[Workflow]\n direction TB",
    ) + " publish --> outside[Outside]\n";
    let stages = flowchart_elk_stages(
        &text,
        &Value::empty_object(),
        &options(FlowchartMode::Adaptive),
    )
    .unwrap();
    let workflow = elk_node(&stages["elkInput"], "workflow").unwrap();
    assert_eq!(
        workflow["layoutOptions"]["elk.hierarchyHandling"],
        "INCLUDE_CHILDREN"
    );
    assert_eq!(workflow["layoutOptions"]["elk.direction"], "DOWN");
    assert_eq!(
        workflow["layoutOptions"]["elk.spacing.portPort"], 44.0,
        "the layout inherits RIGHT from the root"
    );
}

#[test]
fn existing_nested_diamond_labels_configure_the_inner_graph() {
    let text = include_str!(
        "../../../e2e/diagrams/flowchart/nightly-check/6088-1-should-handle-diamond-shape-intersections.mmd"
    );
    let stages = flowchart_elk_stages(
        text,
        &Value::empty_object(),
        &options(FlowchartMode::Adaptive),
    )
    .unwrap();
    let input = &stages["elkInput"];
    assert!(input["layoutOptions"].get("elk.spacing.portPort").is_none());
    assert_eq!(
        elk_node(input, "S2").unwrap()["layoutOptions"]["elk.spacing.portPort"],
        44.0
    );
    assert_eq!(
        elk_node(input, "s1").unwrap()["layoutOptions"]["elk.spacing.portPort"],
        44.0
    );
    for id in ["s1", "D", "I"] {
        assert_eq!(
            elk_node(input, id).unwrap()["layoutOptions"]["elk.portAlignment.default"],
            "CENTER",
            "{id}"
        );
    }
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &options(FlowchartMode::Adaptive))
        .unwrap();
    assert_straight_routes(&svg, 3);
}

fn options(routing: FlowchartMode) -> RenderOptions {
    RenderOptions {
        id: Some("routing".to_owned()),
        flowchart_mode: routing,
        ..RenderOptions::default()
    }
}

#[test]
fn edge_label_backgrounds_are_opaque_in_both_routing_modes() {
    let text = "flowchart LR\n A -->|Readable label| B\n";
    let opaque = "#routing .edgeLabel rect{opacity:1;";
    for theme in [
        "redux-color",
        "redux-dark-color",
        "redux",
        "redux-dark",
        "neo",
        "neo-dark",
    ] {
        let renderer = Renderer::new(&Value::from(serde_json::json!({ "theme": theme })));
        let adaptive = renderer
            .render_with(text, &options(FlowchartMode::Adaptive))
            .unwrap();
        let compatible = renderer
            .render_with(text, &options(FlowchartMode::Mermaid))
            .unwrap();
        assert!(adaptive.contains(opaque), "{theme}");
        assert!(compatible.contains(opaque), "{theme}");
        // A single edge needs no attachment changes; the paint is identical too.
        assert_eq!(adaptive, compatible, "{theme}");
        assert_eq!(
            adaptive,
            renderer
                .render_with(text, &options(FlowchartMode::Adaptive))
                .unwrap(),
            "{theme}"
        );
    }
}

#[test]
fn full_opacity_keeps_explicit_theme_background_colors() {
    for color in ["transparent", "rgba(25,50,75,0.25)", "#abcdef"] {
        let renderer = Renderer::new(&Value::from(serde_json::json!({
            "themeVariables": { "edgeLabelBackground": color }
        })));
        let svg = renderer
            .render_with(
                "flowchart LR\n A -->|Label| B\n",
                &options(FlowchartMode::Adaptive),
            )
            .unwrap();
        assert!(
            svg.contains(&format!("--edgeLabelBackground:{color};")),
            "{color}"
        );
        assert!(svg.contains("#routing .edgeLabel rect{opacity:1;"));
    }
}

#[test]
fn wrapped_labels_increase_the_port_gap_instead_of_using_a_fixed_example_value() {
    let text = REVIEW.replace(
        "Changes",
        "Please revise the complete implementation before submitting again",
    );
    let stages = flowchart_elk_stages(
        &text,
        &Value::empty_object(),
        &options(FlowchartMode::Adaptive),
    )
    .unwrap();
    let gap = stages["elkInput"]["layoutOptions"]["elk.spacing.portPort"]
        .as_f64()
        .unwrap();
    assert!(
        gap > 44.0,
        "a multiline label needs more room than the single-line review example: {gap}"
    );
}

#[test]
fn adaptive_routing_straightens_the_review_cycle_without_moving_or_resizing_nodes() {
    let renderer = Renderer::new(&Value::empty_object());
    let adaptive = renderer
        .render_with(REVIEW, &options(FlowchartMode::Adaptive))
        .unwrap();
    let compatible = renderer
        .render_with(REVIEW, &options(FlowchartMode::Mermaid))
        .unwrap();
    assert_eq!(
        adaptive,
        renderer
            .render_with(
                REVIEW,
                &RenderOptions {
                    id: Some("routing".to_owned()),
                    ..RenderOptions::default()
                }
            )
            .unwrap()
    );
    let document = roxmltree::Document::parse(&adaptive).unwrap();
    let paths: Vec<_> = document
        .descendants()
        .filter(|node| node.has_attribute("data-points"))
        .collect();
    assert_eq!(paths.len(), 3);
    for path in paths {
        // This fixture needs only vertical lines. A curve or a lateral step is a regression.
        let d = path.attribute("d").unwrap();
        assert!(
            d.chars()
                .all(|c| !c.is_ascii_alphabetic() || c == 'M' || c == 'L'),
            "{d}"
        );
        let points: Vec<f64> = d
            .split(['M', 'L', ','])
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        assert!(
            points
                .as_chunks::<2>()
                .0
                .iter()
                .all(|p| (p[0] - points[0]).abs() < TOLERANCE),
            "{d}"
        );
    }
    assert_ne!(
        adaptive, compatible,
        "the compatibility path retains the original jogs"
    );
    let stages =
        |routing| flowchart_elk_stages(REVIEW, &Value::empty_object(), &options(routing)).unwrap();
    let adaptive = stages(FlowchartMode::Adaptive);
    let compatible = stages(FlowchartMode::Mermaid);
    for (a, b) in adaptive["elkOutput"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .zip(compatible["elkOutput"]["children"].as_array().unwrap())
    {
        for field in ["id", "x", "y", "width", "height"] {
            assert_eq!(a[field], b[field], "node {} {field}", a["id"]);
        }
    }
    assert!(
        compatible["elkInput"]["layoutOptions"]
            .get("elk.spacing.portPort")
            .is_none()
    );
    assert_eq!(
        adaptive["elkInput"]["layoutOptions"]["elk.spacing.portPort"],
        44.0
    );
}

#[test]
fn routing_is_per_render_and_does_not_affect_other_diagram_types() {
    let renderer = Renderer::new(&Value::empty_object());
    let first = renderer
        .render_with(REVIEW, &options(FlowchartMode::Mermaid))
        .unwrap();
    renderer
        .render_with(REVIEW, &options(FlowchartMode::Adaptive))
        .unwrap();
    assert_eq!(
        first,
        renderer
            .render_with(REVIEW, &options(FlowchartMode::Mermaid))
            .unwrap()
    );
    for text in [
        "classDiagram\n A --> B : calls\n",
        "stateDiagram-v2\n A --> B : next\n",
        "erDiagram\n A ||--o{ B : has\n",
    ] {
        assert_eq!(
            renderer
                .render_with(text, &options(FlowchartMode::Mermaid))
                .unwrap(),
            renderer
                .render_with(text, &options(FlowchartMode::Adaptive))
                .unwrap()
        );
    }
}

/// The `viewBox` of the root element, as its four numbers.
fn view_box(svg: &str) -> [f64; 4] {
    let start = svg.find("viewBox=\"").expect("a viewBox") + "viewBox=\"".len();
    let end = svg[start..].find('"').expect("the viewBox closes") + start;
    let numbers: Vec<f64> = svg[start..end]
        .split_whitespace()
        .map(|number| number.parse().expect("a number"))
        .collect();
    [numbers[0], numbers[1], numbers[2], numbers[3]]
}

/// The subgraph `B` has no node of its own, since `b` belongs to `A`, so its frame has height zero
/// and is not rendered. `getBBox` leaves it out of the drawing's box, so the viewBox starts at the
/// title of `B`, one pixel below the frame's top. The reference gives `viewBox="4 5.0000009 392 164"`
/// (`visual-check/57-handle-nested-subgraphs-with-outgoing-links-4`).
#[test]
fn an_empty_subgraphs_frame_does_not_move_the_view_box() {
    let text = r#"%%{init: {"htmlLabels":false,"fontFamily":"\"Lexica Ultralegible\"","securityLevel":"strict"}}%%
flowchart LR
  subgraph A
  a -->b
  end
  subgraph B
  b
  end
"#;
    let svg = render(text, &Value::empty_object()).expect("the flowchart renders");
    let [x, y, width, height] = view_box(&svg);
    assert!((x - 4.0).abs() < TOLERANCE, "x is {x}");
    assert!((y - 5.0).abs() < TOLERANCE, "y is {y}, the reference has 5");
    assert!((width - 392.0).abs() < TOLERANCE, "width is {width}");
    assert!(
        (height - 164.0).abs() < TOLERANCE,
        "height is {height}, the reference has 164"
    );
}
