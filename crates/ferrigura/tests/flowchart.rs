//! Flowcharts through the public entry point: what the root element's box is.

use ferrigura::render;
use ferrigura::{RenderOptions, Renderer, flowchart_elk_stages};
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
            renderer.render_with(text, &options(false)).unwrap(),
            renderer.render_with(text, &options(true)).unwrap()
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
        let stages = flowchart_elk_stages(&text, &Value::empty_object(), &options(false)).unwrap();
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
                .render_with(&text, &options(false))
                .unwrap();
            assert_straight_routes(&svg, 3);
        }
    }
}

#[test]
fn local_cross_hierarchy_and_self_loop_labels_use_their_own_containers() {
    let text = "flowchart TD\n subgraph left[Left]\n A -->|Local| B\n A -->|Repeat| A\n end\n subgraph right[Right]\n C --> D\n end\n subgraph plain[Plain]\n E --> F\n end\n B -->|Across| C\n";
    let stages = flowchart_elk_stages(text, &Value::empty_object(), &options(false)).unwrap();
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
        let stages = flowchart_elk_stages(&text, &Value::empty_object(), &options(false)).unwrap();
        let gap = elk_node(&stages["elkInput"], "workflow").unwrap()["layoutOptions"]["elk.spacing.portPort"].as_f64().unwrap();
        assert!(
            gap > 60.0,
            "the long label needs more than the 44-unit height-based gap: {gap}"
        );
        let svg = Renderer::new(&Value::empty_object())
            .render_with(&text, &options(false))
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
    let stages = flowchart_elk_stages(&text, &Value::empty_object(), &options(false)).unwrap();
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
    let stages = flowchart_elk_stages(text, &Value::empty_object(), &options(false)).unwrap();
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
        .render_with(text, &options(false))
        .unwrap();
    assert_straight_routes(&svg, 3);
}

fn options(compat: bool) -> RenderOptions {
    RenderOptions {
        id: Some("routing".to_owned()),
        compat,
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
        let adaptive = renderer.render_with(text, &options(false)).unwrap();
        let compatible = renderer.render_with(text, &options(true)).unwrap();
        assert!(adaptive.contains(opaque), "{theme}");
        assert!(compatible.contains(opaque), "{theme}");
        // A single edge needs no attachment changes; the paint is identical too.
        assert_eq!(adaptive, compatible, "{theme}");
        assert_eq!(
            adaptive,
            renderer.render_with(text, &options(false)).unwrap(),
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
            .render_with("flowchart LR\n A -->|Label| B\n", &options(false))
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
    let stages = flowchart_elk_stages(&text, &Value::empty_object(), &options(false)).unwrap();
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
    let adaptive = renderer.render_with(REVIEW, &options(false)).unwrap();
    let compatible = renderer.render_with(REVIEW, &options(true)).unwrap();
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
    let adaptive = stages(false);
    let compatible = stages(true);
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
    let first = renderer.render_with(REVIEW, &options(true)).unwrap();
    renderer.render_with(REVIEW, &options(false)).unwrap();
    assert_eq!(first, renderer.render_with(REVIEW, &options(true)).unwrap());
    for text in [
        "classDiagram\n A --> B : calls\n",
        "stateDiagram-v2\n A --> B : next\n",
        "erDiagram\n A ||--o{ B : has\n",
    ] {
        assert_eq!(
            renderer.render_with(text, &options(true)).unwrap(),
            renderer.render_with(text, &options(false)).unwrap()
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

#[test]
fn flowchart_click_and_image_urls_are_sanitized_under_default_security_level() {
    for unsafe_url in [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,<script>alert(1)</script>",
        "notes://do-your-thing/id",
        "notes#58;meeting",
        "a%3Ab",
        "javascript&colon;alert(1)",
        "%6aavascript:alert(1)",
        "java\tscript:alert(1)",
        "java\\tscript:alert(1)",
    ] {
        let click_input = format!("flowchart TD\nA[x]\nclick A \"{unsafe_url}\"\n");
        let svg = render(&click_input, &Value::empty_object()).expect("renders");
        assert!(
            svg.contains("xlink:href=\"about:blank\""),
            "click url={unsafe_url:?}: {svg}"
        );
        assert!(
            !svg.contains("javascript:") && !svg.contains("vbscript:"),
            "click url={unsafe_url:?}: {svg}"
        );

        let image_input = format!("flowchart TD\nA@{{ img: \"{unsafe_url}\", w: 48, h: 48 }}\n");
        let img_svg = render(&image_input, &Value::empty_object()).expect("renders");
        assert!(
            img_svg.contains("<image href=\"about:blank\""),
            "img url={unsafe_url:?}: {img_svg}"
        );
        assert!(
            !img_svg.contains("javascript:") && !img_svg.contains("vbscript:"),
            "img url={unsafe_url:?}: {img_svg}"
        );
    }

    // Valid https URLs continue to work, without double-decoding click URLs or decoding
    // percent-encoded delimiters in image URLs.
    let valid_click = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/docs\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        valid_click.contains("xlink:href=\"https://example.com/docs\""),
        "{valid_click}"
    );
    let encoded_click = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/files?q=A%252526B\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded_click.contains("xlink:href=\"https://example.com/files?q=A%26B\""),
        "{encoded_click}"
    );
    let valid_image = render(
        "flowchart TD\nA@{ img: \"https://example.com/logo.png\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        valid_image.contains("<image href=\"https://example.com/logo.png\""),
        "{valid_image}"
    );
    let encoded_image = render(
        "flowchart TD\nA@{ img: \"https://example.com/A%23B.png\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded_image.contains("<image href=\"https://example.com/A%23B.png\""),
        "{encoded_image}"
    );
    let relative_colon_image = render(
        "flowchart TD\nA@{ img: \"data/chart.svg?time=12:30\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        relative_colon_image.contains("<image href=\"data/chart.svg?time=12:30\""),
        "{relative_colon_image}"
    );
    for raster_url in [
        "data:image/png;base64,iVBORw0KGgo=",
        "data:image/x-png;base64,iVBORw0KGgo=",
        "data:image/pjpeg;base64,/9j/4AAQ",
        " data:image/png;base64,iVBORw0KGgo=",
        "data: image/png;base64,iVBORw0KGgo=",
        "data:image/png ;base64,iVBORw0KGgo=",
        "data:image/apng;base64,iVBORw0KGgo=",
        "data:image/tiff;base64,SUkqAAg=",
        "data:image/jxl;base64,/wo=",
    ] {
        let raster_image = render(
            &format!("flowchart TD\nA@{{ img: \"{raster_url}\", w: 48, h: 48 }}\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            raster_image.contains(&format!("<image href=\"{raster_url}\"")),
            "url={raster_url:?}: {raster_image}"
        );
    }
    let tab_raster_image = render(
        "flowchart TD\nA@{ img: \"data:im#9;age/png;base64,iVBORw0KGgo=\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        tab_raster_image.contains("<image href=\"data:im&#9;age/png;base64,iVBORw0KGgo=\""),
        "{tab_raster_image}"
    );
    let svg_data_image = render(
        "flowchart TD\nA@{ img: \"data:image/svg+xml,<svg onload=alert(1)>\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        svg_data_image.contains("<image href=\"about:blank\""),
        "{svg_data_image}"
    );
    let raster_data_click = render(
        "flowchart TD\nA[x]\nclick A \"data:image/png;base64,iVBORw0KGgo=\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        raster_data_click.contains("xlink:href=\"about:blank\""),
        "{raster_data_click}"
    );
    let token_image = Renderer::new(&Value::empty_object())
        .render_with(
            "flowchart TD\nA@{ img: \"{svg-id}:alert(1)\", w: 48, h: 48 }\n",
            &RenderOptions {
                id: Some("javascript".to_owned()),
                ..RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        token_image.contains("<image href=\"{svg-id}:alert(1)\""),
        "{token_image}"
    );
    assert!(
        !token_image.contains("href=\"javascript:alert(1)\""),
        "{token_image}"
    );

    let entity_click = render(
        "flowchart TD\nA[x]\nclick A \"https#58;//example.org/\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        entity_click.contains("xlink:href=\"https://example.org/\""),
        "{entity_click}"
    );
    let entity_image = render(
        "flowchart TD\nA@{ img: \"https#58;//example.org/logo.png\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        entity_image.contains("<image href=\"https://example.org/logo.png\""),
        "{entity_image}"
    );

    // Over-budget parser click URLs still fail closed even when another segment has a malformed %.
    let over_budget_malformed = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/a%252525252525252541?value=50%\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        over_budget_malformed.contains("xlink:href=\"about:blank\""),
        "{over_budget_malformed}"
    );

    // Untrusted front matter cannot bypass default strict securityLevel.
    let fm_bypass = render(
        "---\nconfig:\n  securityLevel: loose\n---\nflowchart TD\nA[x]\nclick A \"javascript:alert(1)\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        fm_bypass.contains("xlink:href=\"about:blank\""),
        "{fm_bypass}"
    );

    // Explicit host securityLevel: loose preserves raw URLs.
    let loose_cfg = Value::from(serde_json::json!({ "securityLevel": "loose" }));
    let loose_click = render(
        "flowchart TD\nA[x]\nclick A \"javascript:alert(1)\"\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_click.contains("xlink:href=\"javascript:alert(1)\""),
        "{loose_click}"
    );
    let loose_img = render(
        "flowchart TD\nA@{ img: \"javascript:alert(1)\", w: 48, h: 48 }\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_img.contains("<image href=\"javascript:alert(1)\""),
        "{loose_img}"
    );
    let deep_url = "https://example.org/file%25252525252525252541";
    let loose_class = render(
        &format!("classDiagram\nclass A\nclick A href \"{deep_url}\"\n"),
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_class.contains(&format!("xlink:href=\"{deep_url}\"")),
        "{loose_class}"
    );
    let loose_agentflow = render(
        &format!("agentflow-beta TB\nA[\"Task\"]\nclick A href \"{deep_url}\"\n"),
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_agentflow.contains(&format!("xlink:href=\"{deep_url}\"")),
        "{loose_agentflow}"
    );
}

#[test]
fn kanban_ticket_links_are_sanitized_under_default_security_level() {
    for (base_url, ticket) in [
        ("javascript:alert(1)//#TICKET#", "MC-1"),
        ("vbscript:msgbox(1)//#TICKET#", "MC-1"),
        ("javascript&colon;alert(1)//#TICKET#", "MC-1"),
        ("%6aavascript:alert(1)//#TICKET#", "MC-1"),
        ("&#106;avascript:alert(1)//#TICKET#", "MC-1"),
        ("notes&#x3A;#TICKET#", "MC-1"),
        ("#TICKET#", "notes#58;meeting"),
        ("#TICKET#", "javascript:alert(1)"),
    ] {
        let input = format!(
            "%%{{init: {{\"kanban\": {{\"ticketBaseUrl\": \"{base_url}\"}}}}}}%%\nkanban\n  Todo\n    Task@{{ ticket: '{ticket}' }}\n"
        );
        let svg = render(&input, &Value::empty_object()).expect("renders");
        assert!(
            svg.contains("xlink:href=\"about:blank\""),
            "base={base_url:?} ticket={ticket:?}: {svg}"
        );
        assert!(
            !svg.contains("xlink:href=\"javascript:") && !svg.contains("xlink:href=\"vbscript:"),
            "base={base_url:?} ticket={ticket:?}: {svg}"
        );
    }

    let valid = render(
        "%%{init: {\"kanban\": {\"ticketBaseUrl\": \"https://tickets.example.com/browse/#TICKET#\"}}}%%\nkanban\n  Todo\n    Task@{ ticket: MC-42 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        valid.contains("xlink:href=\"https://tickets.example.com/browse/MC-42\""),
        "{valid}"
    );

    let encoded = render(
        "%%{init: {\"kanban\": {\"ticketBaseUrl\": \"https://tickets.example.com/browse/#TICKET#\"}}}%%\nkanban\n  Todo\n    Task@{ ticket: 'A%23B' }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded.contains("xlink:href=\"https://tickets.example.com/browse/A%23B\""),
        "{encoded}"
    );

    let single_pass = render(
        "%%{init: {\"kanban\": {\"ticketBaseUrl\": \"/docs/#TICKET#\"}}}%%\nkanban\n  Todo\n    Task@{ ticket: '#64258;°copy;' }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        single_pass.contains("xlink:href=\"/docs/ﬂ°copy;\""),
        "{single_pass}"
    );
    assert!(
        !single_pass.contains("xlink:href=\"/docs/©\""),
        "{single_pass}"
    );

    let loose_cfg = Value::from(serde_json::json!({ "securityLevel": "loose" }));
    let loose = render(
        "%%{init: {\"kanban\": {\"ticketBaseUrl\": \"javascript:alert(1)//#TICKET#\"}}}%%\nkanban\n  Todo\n    Task@{ ticket: MC-42 }\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose.contains("xlink:href=\"javascript:alert(1)//MC-42\""),
        "{loose}"
    );
}
