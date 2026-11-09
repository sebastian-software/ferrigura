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

/// The `<g>` of the node whose id ends with `suffix`, up to the next node.
fn node<'a>(svg: &'a str, suffix: &str) -> &'a str {
    let start = svg
        .find(&format!("{suffix}\" data-look"))
        .expect("the node is drawn");
    let rest = &svg[start..];
    let end = rest[1..]
        .find("<g class=\"node")
        .map_or(rest.len(), |end| end + 1);
    &rest[..end]
}

#[test]
fn the_rough_state_shapes_carry_their_colors_as_attributes() {
    // The state style sheet has no rule for a node's paths, so the reference's presentation
    // attributes color them: without them, a note, a choice and an end state fill black.
    let text = "stateDiagram-v2
  state paid <<choice>>
  [*] --> Cart
  Cart --> paid
  paid --> [*]
  note right of Cart: Text
";
    let svg = render(text, &Value::empty_object()).expect("renders");
    // The note: `noteBkgColor` and `noteBorderColor` (`note.ts`).
    let note = node(&svg, "Cart----note-3");
    assert!(note.contains("fill=\"#fff5ad\""), "{note}");
    assert!(note.contains("stroke=\"#FACC15\""), "{note}");
    // The choice: `mainBkg` and `nodeBorder`, a hollow diamond (`choice.ts`).
    let choice = node(&svg, "state-paid-2");
    assert!(choice.contains("fill=\"#ffffff\""), "{choice}");
    assert!(choice.contains("stroke=\"#28253D\""), "{choice}");
    // The end state: a ring of `mainBkg` around a dot of `stateBorder` (`stateEnd.ts`).
    let end = node(&svg, "root_end-2");
    assert!(end.contains("fill=\"#ffffff\""), "{end}");
    assert!(end.contains("fill=\"#28253D\""), "{end}");
}

#[test]
fn the_nested_text_rule_of_a_state_note_is_written_flat() {
    let svg = render("stateDiagram-v2\n  A --> B\n", &Value::empty_object()).expect("renders");
    assert!(svg.contains(" .state-note text{"), "{svg}");
}
