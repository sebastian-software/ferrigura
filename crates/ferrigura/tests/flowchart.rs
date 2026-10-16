//! Flowcharts through the public entry point: what the root element's box is.

use ferrigura::render;
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
