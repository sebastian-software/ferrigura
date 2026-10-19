//! The render of agentflow: the structure the reference's renderer writes (`renderer.ts`, `styles.ts`,
//! `clusters.js` `flowGroup`), and one case for each quirk the port reproduces in the output: the keys
//! that do nothing, the title and the view box, the markers, the palette that a non-colour theme does not
//! have, and the classes of the vertices.

#[path = "support/agentflow.rs"]
#[allow(dead_code)]
mod support;

use support::{Setup, config_with, count, style_of};

/// The SVG of a text, with the default configuration and the redux colour theme.
fn svg(text: &str) -> String {
    Setup::new(config_with(serde_json::json!({})), None).svg(text, "p0")
}

/// The `viewBox` of an SVG, as its four numbers.
fn view_box(svg: &str) -> Vec<f64> {
    let start = svg.find("viewBox=\"").expect("a viewBox") + "viewBox=\"".len();
    let end = svg[start..].find('"').expect("the end of the viewBox") + start;
    svg[start..end]
        .split(' ')
        .map(|number| number.parse().expect("a number"))
        .collect()
}

const BASIC: &str = "agentflow-beta TB\n  flow reviewer[\"Review Agent\"]\n    changes[\"Gather changes\"]@{ shape: input }\n    analyse[\"Analyse diff\"]@{ shape: task }\n    changes --> analyse\n  end";

#[test]
fn the_root_element_is_an_agentflow_with_the_title_and_the_drop_shadows() {
    let svg = svg(BASIC);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"p0\" width=\"100%\""));
    assert!(svg.contains("class=\"agentflow\""));
    assert!(svg.contains("aria-roledescription=\"agentflow\""));
    // `setupViewPortForSVG` gives the root's class and the size; the two filters follow the drawing.
    assert!(svg.contains("<filter id=\"p0-drop-shadow\" height=\"130%\" width=\"130%\">"));
    assert!(svg.contains("<filter id=\"p0-drop-shadow-small\" height=\"150%\" width=\"150%\">"));
    assert!(svg.ends_with("</defs></svg>"));
}

#[test]
fn the_markers_are_the_flowchart_twelve_and_the_hierarchy_pair() {
    // `insertMarkers(['point', 'circle', 'cross', 'hierarchy'], 'agentflow')`: the ids carry the diagram's
    // name, and the hierarchy markers have no margin variant.
    let svg = svg(BASIC);
    // Fourteen markers: the twelve of point, circle and cross, with their margins, and the two hierarchy ones.
    assert_eq!(count(&svg, "<marker "), 14);
    assert_eq!(count(&svg, "class=\"marker cross agentflow\""), 4);
    assert!(svg.contains("id=\"p0_agentflow-hierarchyEnd\""));
    assert!(svg.contains("id=\"p0_agentflow-hierarchyStart\""));
    assert!(svg.contains("class=\"marker hierarchy agentflow\""));
    assert!(svg.contains("id=\"p0_agentflow-pointEnd-margin\""));
}

#[test]
fn the_title_sits_above_the_drawing_at_the_title_margin() {
    // `utils.insertTitle` with `titleTopMargin` (25 by default): the title's baseline is 25 above the origin.
    let svg = svg(&format!("---\ntitle: Flow\n---\n{BASIC}"));
    assert!(
        svg.contains("<text text-anchor=\"middle\" x=")
            && svg.contains("y=\"-25\" class=\"agentflowTitleText\">Flow</text>"),
        "{svg}"
    );
}

#[test]
fn the_view_box_grows_with_the_diagram_padding() {
    // `setupViewPortForSVG(svg, diagramPadding)`: the drawn box with the padding on each side.
    let setup = |padding: u32| {
        Setup::new(
            config_with(serde_json::json!({ "agentflow": { "diagramPadding": padding } })),
            None,
        )
        .svg("agentflow-beta TB\n  a --> b", "p0")
    };
    let small = view_box(&setup(8));
    let large = view_box(&setup(30));
    assert_eq!(large[2] - small[2], 44.0);
    assert_eq!(large[3] - small[3], 44.0);
    assert_eq!(large[0], small[0] - 22.0);
}

#[test]
fn use_max_width_off_writes_the_width_and_the_height() {
    let svg = Setup::new(
        config_with(serde_json::json!({ "agentflow": { "useMaxWidth": false } })),
        None,
    )
    .svg("agentflow-beta TB\n  a --> b", "p0");
    assert!(svg.contains(" height=\"150\" width=\"166\""), "{svg}");
    assert!(!svg.contains("style=\"max-width"), "{svg}");
}

#[test]
fn node_and_rank_spacing_do_nothing() {
    // `nodeSpacing` and `rankSpacing` are written to the layout data and read by nothing (probed in the
    // reference: the SVG is identical with 300).
    let text = "agentflow-beta TB\n  a --> b\n  b --> c";
    let default = svg(text);
    let spaced = Setup::new(
        config_with(serde_json::json!({ "agentflow": { "nodeSpacing": 300, "rankSpacing": 300 } })),
        None,
    )
    .svg(text, "p0");
    assert_eq!(default, spaced);
}

#[test]
fn the_wrapping_width_wraps_the_labels() {
    // `agentflow.wrappingWidth` is read by the labels: a narrow width breaks a label into more lines.
    let text = "agentflow-beta TB\n  a[\"a fairly long label that wraps over several lines\"]";
    let wide = svg(text);
    let narrow = Setup::new(
        config_with(serde_json::json!({ "agentflow": { "wrappingWidth": 40 } })),
        None,
    )
    .svg(text, "p0");
    assert_ne!(wide, narrow);
    assert!(count(&narrow, "text-inner-tspan") > count(&wide, "text-inner-tspan"));
}

#[test]
fn a_container_is_a_transparent_frame_with_the_flow_container_stroke() {
    // The `flowGroup` frame: `fill="none"`, the flow container stroke, a 0.75px stroke and the `flow-cluster`
    // class; its colour is the palette rule's (`styles.ts` `.flow-cluster rect`).
    let svg = svg(BASIC);
    assert!(
        svg.contains("class=\"cluster flow-cluster\" id=\"p0-reviewer\" data-look=\"neo\" data-color-id=\"color-7\""),
        "{svg}"
    );
    assert!(svg.contains("fill=\"none\" stroke=\"hsl(-120, 0%, 70%)\" stroke-width=\"0.75px\""));
    assert!(style_of(&svg).contains(
        ".flow-cluster rect{fill:none;stroke:var(--clusterBorder);stroke-width:0.75px;}"
    ));
}

#[test]
fn a_vertex_is_a_node_of_its_kind_with_its_dom_id() {
    let svg = svg(BASIC);
    assert!(
        svg.contains("class=\"node default af-kind-input\" id=\"p0-agentflow-changes-0\""),
        "{svg}"
    );
    assert!(
        svg.contains("class=\"node default af-kind-task\" id=\"p0-agentflow-analyse-2\""),
        "{svg}"
    );
}

#[test]
fn a_connector_is_a_rounded_node_of_the_connector_kind() {
    let svg = svg("agentflow-beta TB\n  connector x[\"X\"]\n  x --> a");
    assert!(svg.contains("af-kind-connector"), "{svg}");
}

#[test]
fn a_default_theme_draws_no_palette_and_keeps_the_kind_classes() {
    // The palette is on only for the colour themes (`colorThemeGate.ts`). The vertices still carry their
    // kind's class, which the sheet does not name.
    let svg = Setup::new(
        config_with(serde_json::json!({ "theme": "default", "look": "neo" })),
        None,
    )
    .svg(BASIC, "p0");
    assert!(!style_of(&svg).contains("[data-look=\"neo\"].node.af-kind-"));
    assert!(!svg.contains("data-color-id"));
    assert!(svg.contains("af-kind-input"));
}

#[test]
fn a_class_definition_styles_the_shapes_of_its_nodes_important() {
    // `createCssStyles`: a class's declarations are `!important` on the shapes of its nodes.
    let svg = svg("agentflow-beta TB\n  classDef hot fill:#f00\n  a:::hot --> b");
    assert!(
        style_of(&svg).contains("#p0 .hot rect{fill:#f00 !important;}"),
        "{}",
        style_of(&svg)
    );
}

#[test]
fn a_collapsed_flow_is_a_flow_collapsed_frame_with_its_slot() {
    let svg =
        svg("agentflow-beta TB\n  flow p[\"P\"]\n    a --> b\n  end\n  p@{ view: \"collapsed\" }");
    assert!(
        svg.contains("class=\"basic label-container flow-collapsed\""),
        "{svg}"
    );
    assert!(svg.contains("data-color-id=\"color-7\""), "{svg}");
    assert!(svg.contains("class=\"collapsed-separator\""));
    assert_eq!(count(&svg, "class=\"collapsed-indicator\""), 3);
}

#[test]
fn an_edge_draws_its_arrow_with_the_margin_marker_of_the_neo_look() {
    let svg = svg("agentflow-beta TB\n  a --> b");
    assert!(
        svg.contains("marker-end=\"url(#p0_agentflow-pointEnd-margin)\""),
        "{svg}"
    );
}
