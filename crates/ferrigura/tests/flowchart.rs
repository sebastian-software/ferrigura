//! Flowcharts through the public entry point: what the root element's box is.

use ferrigura::render;
use ferrigura::{FlowchartRouting, RenderOptions, Renderer, flowchart_elk_stages};
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
            "../../../e2e/diagrams/flowchart/nightly-check/6088-1-should-handle-diamond-shape-intersections.mmd"
        ),
        include_str!(
            "../../../e2e/diagrams/flowchart/nightly-check/shape-themes/02-connect-all-shapes-with-each-other.mmd"
        ),
    ] {
        assert_eq!(
            renderer
                .render_with(text, &options(FlowchartRouting::Adaptive))
                .unwrap(),
            renderer
                .render_with(text, &options(FlowchartRouting::Mermaid))
                .unwrap()
        );
    }
}

fn options(routing: FlowchartRouting) -> RenderOptions {
    RenderOptions {
        id: Some("routing".to_owned()),
        flowchart_routing: routing,
        ..RenderOptions::default()
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
        &options(FlowchartRouting::Adaptive),
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
        .render_with(REVIEW, &options(FlowchartRouting::Adaptive))
        .unwrap();
    let compatible = renderer
        .render_with(REVIEW, &options(FlowchartRouting::Mermaid))
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
    let adaptive = stages(FlowchartRouting::Adaptive);
    let compatible = stages(FlowchartRouting::Mermaid);
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
        .render_with(REVIEW, &options(FlowchartRouting::Mermaid))
        .unwrap();
    renderer
        .render_with(REVIEW, &options(FlowchartRouting::Adaptive))
        .unwrap();
    assert_eq!(
        first,
        renderer
            .render_with(REVIEW, &options(FlowchartRouting::Mermaid))
            .unwrap()
    );
    for text in [
        "classDiagram\n A --> B : calls\n",
        "stateDiagram-v2\n A --> B : next\n",
        "erDiagram\n A ||--o{ B : has\n",
    ] {
        assert_eq!(
            renderer
                .render_with(text, &options(FlowchartRouting::Mermaid))
                .unwrap(),
            renderer
                .render_with(text, &options(FlowchartRouting::Adaptive))
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
