//! The design mode (`docs/decisions/0023-design-mode.md`): ferrigura's own look and measures for a
//! flowchart, laid out by the engine of the default rendering.

use ferrigura::{ColorScheme, Error, RenderOptions, Renderer, Value};

/// A flowchart with every role, a neutral node, a labeled edge and a subgraph.
const FLOW: &str = "flowchart TD\n  A([Request]) --> B{Authenticated?}\n  B -- yes --> C[Load profile]\n  B -- no --> D[Login page]\n  C --> E[(User DB)]\n  subgraph Billing\n    F[Create invoice] --> G[[Charge card]]\n  end\n  C --> F\n  G --> H((Done))\n  D -.-> A\n";

/// The options of a render in the design mode with a fixed id.
fn design(scheme: Option<ColorScheme>) -> RenderOptions {
    RenderOptions {
        design: true,
        id: Some("d".to_owned()),
        color_scheme: scheme,
        ..RenderOptions::default()
    }
}

/// The options of a default render with the same id.
fn default(scheme: Option<ColorScheme>) -> RenderOptions {
    RenderOptions {
        id: Some("d".to_owned()),
        color_scheme: scheme,
        ..RenderOptions::default()
    }
}

/// The values of every occurrence of an attribute, in document order.
fn attribute_values<'a>(svg: &'a str, name: &str) -> Vec<&'a str> {
    let pattern = format!(" {name}=\"");
    svg.match_indices(&pattern)
        .filter_map(|(at, _)| {
            let rest = &svg[at + pattern.len()..];
            rest.find('"').map(|end| &rest[..end])
        })
        .collect()
}

/// The horizontal place of the first group title of an SVG.
fn title_x(svg: &str) -> f64 {
    let marker = "<g class=\"cluster-label \" transform=\"translate(";
    let rest = &svg[svg.find(marker).expect("a group title") + marker.len()..];
    rest[..rest.find(',').expect("the x")]
        .parse()
        .expect("a number")
}

/// The left edge of the first group frame of an SVG.
fn frame_x(svg: &str) -> f64 {
    let group = &svg[svg.find("<g class=\"cluster ").expect("a group")..];
    let marker = " x=\"";
    let rest = &group[group.find(marker).expect("the frame's x") + marker.len()..];
    rest[..rest.find('"').expect("the x's end")]
        .parse()
        .expect("a number")
}

/// The part of an SVG that holds its nodes.
fn nodes(svg: &str) -> &str {
    &svg[svg.find("<g class=\"nodes\">").expect("the nodes")..]
}

/// The style sheet of an SVG.
fn style_sheet(svg: &str) -> &str {
    let start = svg.find("<style>").expect("a style sheet") + "<style>".len();
    let end = svg.find("</style>").expect("the style sheet's end");
    &svg[start..end]
}

/// The size of the outline of the node with the given id: the `rect` of its group.
fn node_size(svg: &str, id: &str) -> (f64, f64) {
    let group = &svg[svg
        .find(&format!("id=\"d-flowchart-{id}-"))
        .expect("the node")..];
    let rect = &group[group
        .find("<rect class=\"basic label-container\"")
        .expect("its rect")..];
    let rect = &rect[..rect.find("/>").expect("the rect's end")];
    let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
    (number("width"), number("height"))
}

#[test]
fn a_node_gives_the_edges_of_its_busiest_side_room() {
    let renderer = Renderer::new(&Value::empty_object());
    // Four edges meet `E` on one side, three leave `A`, one meets `F`.
    let across = "flowchart LR\n  A[Alpha] --> E[Echo]\n  B[Beta] --> E\n  C[Gamma] --> E\n  D[Delta] --> E\n  A --> F[Foxtrot]\n  A --> G[Golf]\n";
    let plain = renderer.render_with(across, &default(None)).unwrap();
    let designed = renderer.render_with(across, &design(None)).unwrap();
    let (width, height) = node_size(&plain, "E");
    // 20 for each edge after the first, and 12 at both ends of the side.
    assert_eq!(node_size(&designed, "E"), (width, 84.0));
    assert_eq!(node_size(&designed, "A"), (width, 64.0));
    assert_eq!(node_size(&designed, "F"), (width, height));
    assert!(height < 64.0);

    // From top to bottom the side runs along the width, which a node usually has already.
    let down = across.replace("flowchart LR", "flowchart TD");
    let plain = renderer.render_with(&down, &default(None)).unwrap();
    let designed = renderer.render_with(&down, &design(None)).unwrap();
    assert_eq!(node_size(&designed, "E"), node_size(&plain, "E"));
    let many = "flowchart TD\n  A --> Z[Sink]\n  B --> Z\n  C --> Z\n  D --> Z\n  E --> Z\n  F --> Z\n  G --> Z\n  H --> Z\n  I --> Z\n";
    let plain = renderer.render_with(many, &default(None)).unwrap();
    let designed = renderer.render_with(many, &design(None)).unwrap();
    let (width, height) = node_size(&plain, "Z");
    assert!(width < 184.0);
    assert_eq!(node_size(&designed, "Z"), (184.0, height));
}

/// The points of the route of the edge with the given id.
fn route(svg: &str, id: &str) -> Vec<(f64, f64)> {
    let edge = &svg[svg.find(&format!("data-id=\"{id}\"")).expect("the edge")..];
    let points = attribute_values(edge, "data-points")[0];
    let json = String::from_utf8(base64_decode(points)).expect("the points are text");
    let json: serde_json::Value = serde_json::from_str(&json).expect("the points are JSON");
    json.as_array()
        .expect("a list of points")
        .iter()
        .map(|point| (point["x"].as_f64().unwrap(), point["y"].as_f64().unwrap()))
        .collect()
}

/// Decodes standard base64 with padding.
fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut bits, mut count) = (0_u32, 0);
    for byte in text.bytes().filter(|byte| *byte != b'=') {
        let value = ALPHABET.iter().position(|c| *c == byte).expect("base64") as u32;
        bits = (bits << 6) | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    out
}

#[test]
fn the_mode_places_nodes_so_that_edges_run_straight() {
    let renderer = Renderer::new(&Value::empty_object());
    // Two sources, two middle nodes and a sink: Brandes & Koepf leaves steps that a placement by
    // network simplex does not.
    let text = "flowchart LR\n  A[Alpha] --> D[Delta]\n  B[Beta] --> C[Gamma]\n  A --> C\n  B --> D\n  A -- sync --> E[Echo]\n  B == bulk ==> E\n  C <--> E\n  D --o E\n  D --x F[Foxtrot]\n";
    let straight = |svg: &str, id: &str| {
        let points = route(svg, id);
        points
            .iter()
            .all(|point| (point.1 - points[0].1).abs() < 1e-6)
    };
    let plain = renderer.render_with(text, &default(None)).unwrap();
    let designed = renderer.render_with(text, &design(None)).unwrap();
    let edges = ["L_A_C_0", "L_B_D_0", "L_C_E_0"];
    assert!(edges.iter().any(|edge| !straight(&plain, edge)));
    for edge in edges {
        assert!(straight(&designed, edge), "{edge} bends");
    }
    // A placement strategy the configuration names is the user's: the mode keeps it.
    let config = Value::from(serde_json::json!({
        "elk": { "nodePlacementStrategy": "BRANDES_KOEPF" }
    }));
    let kept = Renderer::new(&config)
        .render_with(text, &design(None))
        .unwrap();
    assert!(edges.iter().any(|edge| !straight(&kept, edge)));
}

#[test]
fn the_mode_sets_a_diagram_with_more_room() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    // A node without a busy side keeps its size.
    assert_eq!(node_size(&plain, "C"), node_size(&designed, "C"));
    // The group pads its members by 32, and by 44 at the top.
    let frame = |svg: &str| -> (f64, f64) {
        let group = &svg[svg.find("<g class=\"cluster ").expect("a group")..];
        let rect = &group[..group.find("/>").expect("the frame's end")];
        let number = |name: &str| -> f64 { attribute_values(rect, name)[0].parse().unwrap() };
        (number("width"), number("height"))
    };
    let ((width, height), (roomy_width, roomy_height)) = (frame(&plain), frame(&designed));
    assert!((roomy_width - (width + 2.0 * (32.0 - 24.0))).abs() < 1e-6);
    assert!((roomy_height - (height + (44.0 - 24.0) + (32.0 - 24.0))).abs() < 1e-6);
}

#[test]
fn the_arrowheads_are_the_modes_own() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    let head = "id=\"d_flowchart-v2-pointEnd-margin\" class=\"marker flowchart-v2\" viewBox=";
    assert!(plain.contains(&format!("{head}\"0 0 11.5 14\"")));
    assert!(designed.contains(&format!("{head}\"0 0 10 9\" refX=\"10\" refY=\"4.5\"")));
    assert!(designed.contains("<path d=\"M 0 0 L 10 4.5 L 0 9 L 2.25 4.5 z\""));
    // The other markers are the reference's.
    let circle = "id=\"d_flowchart-v2-circleEnd-margin\"";
    let definition = |svg: &str| {
        let at = svg.find(circle).expect("the circle marker");
        svg[at..at + svg[at..].find("</marker>").expect("its end")].to_owned()
    };
    assert_eq!(definition(&plain), definition(&designed));
    // A colored edge takes a colored copy of the mode's arrowhead.
    let coloured = format!("{FLOW}  linkStyle 0 stroke:#d33\n");
    let designed = renderer.render_with(&coloured, &design(None)).unwrap();
    let at = designed
        .find("id=\"d_flowchart-v2-pointEnd-margin__d33\"")
        .expect("the colored marker");
    assert!(designed[at..].starts_with(
        "id=\"d_flowchart-v2-pointEnd-margin__d33\" class=\"marker flowchart-v2\" viewBox=\"0 0 10 9\""
    ));
    // A thick edge stays slimmer than its arrowhead.
    assert!(style_sheet(&designed).contains("#d .edge-thickness-thick{stroke-width:3px;}"));
}

#[test]
fn an_edge_label_interrupts_its_edge() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    let background = |svg: &str| {
        let labels = &svg[svg.find("<g class=\"edgeLabels\">").expect("the labels")..];
        let labels = &labels[..labels.find("<g class=\"nodes\">").expect("the nodes")];
        let number = |name: &str| -> f64 { attribute_values(labels, name)[0].parse().unwrap() };
        (
            number("x"),
            number("width"),
            number("height"),
            attribute_values(labels, "rx")
                .first()
                .copied()
                .map(str::to_owned),
        )
    };
    // The label's box is 5 wider on each side than the default's background, with round corners.
    let (x, width, height, radius) = background(&plain);
    let (gap_x, gap_width, gap_height, gap_radius) = background(&designed);
    assert_eq!(radius, None);
    assert_eq!(gap_radius.as_deref(), Some("6"));
    assert_eq!(gap_x, x - 5.0);
    assert_eq!(gap_width, width + 10.0);
    assert_eq!(gap_height, height);
    // The box is not painted: the edges are masked there, with one hole for each of the two labels.
    assert!(
        style_sheet(&designed).contains("#d .edgeLabel rect.background{fill:none;stroke:none;}")
    );
    let mask = &designed[designed
        .find("<mask id=\"d-label-gaps\"")
        .expect("the mask")..];
    let mask = &mask[..mask.find("</mask>").expect("the mask's end")];
    assert_eq!(mask.matches("fill=\"black\"").count(), 2);
    assert_eq!(mask.matches("fill=\"white\"").count(), 1);
    assert!(designed.contains("</mask><g mask=\"url(#d-label-gaps)\"><path "));
    assert!(!plain.contains("<mask"));
    // A diagram without edge labels has no mask.
    let bare = renderer
        .render_with("flowchart TD\n  A --> B\n", &design(None))
        .unwrap();
    assert!(!bare.contains("<mask"));
}

#[test]
fn a_group_title_sits_at_the_left_and_clear_of_the_edges() {
    let renderer = Renderer::new(&Value::empty_object());
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    // The title of `Billing` is 16 from the frame's left and 12 below its top.
    assert_eq!(title_x(&designed), frame_x(&designed) + 16.0);
    let frame_y: f64 = {
        let group = &designed[designed.find("<g class=\"cluster ").expect("a group")..];
        attribute_values(group, "y")[0].parse().unwrap()
    };
    let marker = "<g class=\"cluster-label \" transform=\"translate(";
    let title = &designed[designed.find(marker).expect("a title") + marker.len()..];
    let title_y: f64 = title
        [title.find(", ").expect("the y") + 2..title.find(')').expect("the end")]
        .parse()
        .unwrap();
    assert!((title_y - (frame_y + 12.0)).abs() < 1e-6);

    // An edge that enters where the title would sit moves the title past it.
    let crossed = "flowchart TD\n  A[Start] --> B[One]\n  A --> C[Two]\n  subgraph G [A long group title]\n    B\n    C\n  end\n";
    let designed = renderer.render_with(crossed, &design(None)).unwrap();
    assert!(title_x(&designed) > frame_x(&designed) + 16.0);
    // A short title has room before that edge and stays at the left.
    let short = crossed.replace("A long group title", "Group");
    let designed = renderer.render_with(&short, &design(None)).unwrap();
    assert_eq!(title_x(&designed), frame_x(&designed) + 16.0);
}

#[test]
fn an_absent_option_leaves_the_output_unchanged() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render(FLOW).unwrap();
    let off = RenderOptions {
        design: false,
        ..RenderOptions::default()
    };
    assert_eq!(renderer.render_with(FLOW, &off).unwrap(), plain);
    assert!(!plain.contains("data-role"));
    // A render in the mode does not leak into the next one.
    renderer.render_with(FLOW, &design(None)).unwrap();
    assert_eq!(renderer.render(FLOW).unwrap(), plain);
}

#[test]
fn a_shape_with_a_meaning_carries_its_role() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    assert_eq!(
        attribute_values(&svg, "data-role"),
        ["terminal", "decision", "data", "terminal"]
    );
    let sheet = style_sheet(&svg);
    for rule in [
        "#d [data-role=\"terminal\"].node rect,",
        "fill:var(--terminalBkg);stroke:var(--terminalBorder);",
        "#d [data-role=\"decision\"].node .label text{fill:var(--decisionText);",
        "fill:var(--dataBkg);stroke:var(--dataBorder);",
    ] {
        assert!(sheet.contains(rule), "{rule}");
    }
}

#[test]
fn outlines_are_hairlines_and_corners_are_round() {
    let renderer = Renderer::new(&Value::empty_object());
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("#d .edge-thickness-normal{stroke-width:1.5px;}"));
    assert!(sheet.contains("stroke:var(--nodeBorder);stroke-width:1.25px;"));
    assert!(!sheet.contains("stroke-width:2px"));
    // Three rectangle nodes and the frame of the subgraph.
    assert_eq!(attribute_values(nodes(&svg), "rx"), ["8", "8", "8"]);
    assert!(svg.contains("<rect rx=\"12\" ry=\"12\""));
    assert!(svg.contains("<feDropShadow dx=\"0\" dy=\"1\" stdDeviation=\"1.5\""));
}

#[test]
fn a_group_is_neutral_and_takes_no_palette_slot() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    assert!(plain.contains("data-color-id=\"color-0\""));
    assert!(!designed.contains("data-color-id"));
    assert!(!style_sheet(&designed).contains("--borderColor0"));
    assert!(designed.contains("fill=\"#F2F4F8\" stroke=\"#8891A2\""));
    assert!(
        style_sheet(&designed).contains(
            "#d [data-look=\"neo\"].cluster rect{stroke:var(--clusterBorder);filter:none;}"
        )
    );
}

#[test]
fn the_color_scheme_chooses_the_tone() {
    let renderer = Renderer::new(&Value::empty_object());
    let light = renderer
        .render_with(FLOW, &design(Some(ColorScheme::Light)))
        .unwrap();
    let dark = renderer
        .render_with(FLOW, &design(Some(ColorScheme::Dark)))
        .unwrap();
    let auto = renderer
        .render_with(FLOW, &design(Some(ColorScheme::Auto)))
        .unwrap();
    assert!(style_sheet(&light).contains("--mainBkg:#FFFFFF;"));
    assert!(style_sheet(&light).contains("--terminalBkg:#2F54D6;"));
    assert!(style_sheet(&dark).contains("--mainBkg:#1D212A;"));
    assert!(style_sheet(&dark).contains("--terminalBkg:#4263EB;"));
    assert!(style_sheet(&auto).contains("--mainBkg:light-dark(#FFFFFF,#1D212A);"));
    assert!(style_sheet(&auto).contains("--terminalBkg:light-dark(#2F54D6,#4263EB);"));
    // Without a color scheme the render is light.
    assert_eq!(
        style_sheet(&renderer.render_with(FLOW, &design(None)).unwrap()),
        style_sheet(&light)
    );
}

#[test]
fn a_configured_dark_theme_chooses_the_dark_tone() {
    let config = Value::from(serde_json::json!({ "theme": "redux-dark-color" }));
    let renderer = Renderer::new(&config);
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    assert!(style_sheet(&svg).contains("--mainBkg:#1D212A;"));
    assert!(style_sheet(&svg).contains("--terminalBkg:#4263EB;"));
    // A directive that sets the theme does the same.
    let text = format!("%%{{init: {{\"theme\": \"redux-dark-color\"}}}}%%\n{FLOW}");
    let directed = Renderer::new(&Value::empty_object())
        .render_with(&text, &design(None))
        .unwrap();
    assert!(style_sheet(&directed).contains("--mainBkg:#1D212A;"));
}

#[test]
fn the_users_theme_variables_and_classes_keep_the_last_word() {
    let config = Value::from(serde_json::json!({
        "themeVariables": { "mainBkg": "#fafafa", "lineColor": "#ff0000" }
    }));
    let renderer = Renderer::new(&config);
    let svg = renderer.render_with(FLOW, &design(None)).unwrap();
    let sheet = style_sheet(&svg);
    assert!(sheet.contains("--mainBkg:#fafafa;"));
    assert!(sheet.contains("--lineColor:#ff0000;"));
    assert!(sheet.contains("--nodeBorder:#7C8698;"));

    let classed = format!("{FLOW}  classDef hot fill:#f96,stroke:#333\n  class A hot\n");
    let svg = Renderer::new(&Value::empty_object())
        .render_with(&classed, &design(None))
        .unwrap();
    let sheet = style_sheet(&svg);
    let role = sheet
        .find("#d [data-role=\"terminal\"].node rect")
        .expect("the role's rule");
    let class = sheet.find("#d .hot rect{").expect("the class's rule");
    assert!(role < class, "the class rule comes after the role's");
    assert!(sheet[class..].starts_with("#d .hot rect{fill:#f96 !important;"));
}

#[test]
fn the_draw_animation_works_in_the_design_mode() {
    let renderer = Renderer::new(&Value::empty_object());
    let options = RenderOptions {
        animation: Some(ferrigura::Animation::default()),
        ..design(None)
    };
    let svg = renderer.render_with(FLOW, &options).unwrap();
    assert!(svg.contains("data-role=\"terminal\""));
    assert!(svg.contains("@keyframes"));
}

#[test]
fn the_mode_excludes_compat_and_other_diagram_types() {
    let renderer = Renderer::new(&Value::empty_object());
    let both = RenderOptions {
        compat: true,
        ..design(None)
    };
    match renderer.render_with(FLOW, &both) {
        Err(Error::Options(message)) => assert!(message.contains("compat"), "{message}"),
        other => panic!("expected an option error, got {other:?}"),
    }
    match renderer.render_with("pie\n \"a\" : 1\n", &design(None)) {
        Err(Error::Options(message)) => assert!(message.contains("design mode"), "{message}"),
        other => panic!("expected an option error, got {other:?}"),
    }
}
