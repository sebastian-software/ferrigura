//! State diagrams through the public entry point: the root element, the markers of the look and the
//! title, as `stateRenderer-v3-unified.ts` writes them.

use ferrigura::render;
use ferrigura_model::value::Value;

/// The root element's start tag, up to its closing `>`.
fn root_tag(svg: &str) -> &str {
    let end = svg.find('>').expect("the root element closes");
    &svg[..end]
}

#[test]
fn a_state_diagram_is_a_statediagram_root_with_its_barb_markers() {
    let svg = render(
        "stateDiagram-v2\n  [*] --> A\n  A --> B\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let root = root_tag(&svg);
    assert!(root.contains("class=\"statediagram\""), "{root}");
    assert!(
        root.contains("aria-roledescription=\"stateDiagram\""),
        "{root}"
    );
    // The neo look is the default: the margin variant is the one the edges point at.
    assert!(svg.contains("_stateDiagram-barbEnd-margin\""), "{svg}");
    assert!(svg.contains("marker-end=\"url(#"), "{svg}");
    assert!(svg.contains("_stateDiagram-barbEnd-margin)\""), "{svg}");
}

#[test]
fn a_classic_state_diagram_points_its_edges_at_the_plain_barb() {
    let text = "%%{init: {\"look\":\"classic\"}}%%\nstateDiagram\n  A --> B\n";
    let svg = render(text, &Value::empty_object()).expect("renders");
    assert!(svg.contains("_stateDiagram-barbEnd\""), "{svg}");
    assert!(!svg.contains("-barbEnd-margin\""), "{svg}");
}

#[test]
fn a_state_diagram_title_is_written_at_the_title_margin() {
    let text = "---\ntitle: Machine\n---\nstateDiagram-v2\n  A --> B\n";
    let svg = render(text, &Value::empty_object()).expect("renders");
    assert!(svg.contains("class=\"statediagramTitleText\""), "{svg}");
    assert!(svg.contains(">Machine</text>"), "{svg}");
}
