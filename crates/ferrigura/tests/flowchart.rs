//! Flowcharts through the public entry point: viewport and layout-stage contracts.

use ferrigura::{RenderOptions, flowchart_elk_stages, render, render_with};
use ferrigura_model::value::Value;

/// The tolerance of the comparison with the reference, in SVG user units.
const TOLERANCE: f64 = 0.05;

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

/// The straightener acts on the laid-out routes, after ELK has positioned nodes and ports.
/// The vertical cycle exposes jogs; its horizontal counterpart needs no change. Neither changes
/// ELK's input or output.
#[test]
fn straightening_changes_the_svg_without_changing_the_elk_graph() {
    let sources = [
        (
            include_str!("../../../e2e/diagrams/flowchart/visual-check/review-cycle-default.mmd"),
            true,
        ),
        (
            include_str!(
                "../../../e2e/diagrams/flowchart/visual-check/review-cycle-horizontal.mmd"
            ),
            false,
        ),
    ];
    let default = Value::empty_object();
    let unstraightened = Value::from(serde_json::json!({
        "elk": { "straightenEdges": false }
    }));
    // An explicit id prevents configuration hashing from making the SVGs different by itself.
    let options = RenderOptions {
        id: Some("review-cycle".to_owned()),
        ..RenderOptions::default()
    };
    for (source, changes) in sources {
        let before = flowchart_elk_stages(source, &default, &options).expect("default ELK stages");
        let after = flowchart_elk_stages(source, &unstraightened, &options)
            .expect("unstraightened ELK stages");
        assert_eq!(
            before["elkInput"], after["elkInput"],
            "the layout inputs stay the same"
        );
        assert_eq!(
            before["elkOutput"], after["elkOutput"],
            "the layout outputs stay the same"
        );
        let straightened = render_with(source, &default, &options).expect("default SVG");
        let original = render_with(source, &unstraightened, &options).expect("unstraightened SVG");
        assert_eq!(
            straightened != original,
            changes,
            "only the vertical cycle needs terminal straightening"
        );
    }
}
