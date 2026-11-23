//! The design mode (`docs/decisions/0023-design-mode.md`): ferrigura's own look for a flowchart. It
//! changes paint and leaves the geometry of the default rendering as it is, except for the place
//! of a group's title.

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

/// An SVG without the transforms of its group titles, which the design mode places itself.
fn without_title_transforms(svg: &str) -> String {
    let marker = "<g class=\"cluster-label \" transform=\"";
    let mut out = String::new();
    let mut rest = svg;
    while let Some(at) = rest.find(marker) {
        let value = at + marker.len();
        out.push_str(&rest[..value]);
        rest = &rest[value + rest[value..].find('"').expect("the attribute's end")..];
    }
    out.push_str(rest);
    out
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

/// The part of an SVG that holds its edge paths.
fn edges(svg: &str) -> &str {
    let from = svg
        .find("<g class=\"edges edgePaths\">")
        .expect("the edges");
    &svg[from..from
        + svg[from..]
            .find("<g class=\"edgeLabels\">")
            .expect("the labels")]
}

/// The style sheet of an SVG.
fn style_sheet(svg: &str) -> &str {
    let start = svg.find("<style>").expect("a style sheet") + "<style>".len();
    let end = svg.find("</style>").expect("the style sheet's end");
    &svg[start..end]
}

#[test]
fn the_design_mode_keeps_the_nodes_and_routes_of_the_default_rendering() {
    let renderer = Renderer::new(&Value::empty_object());
    for scheme in [
        None,
        Some(ColorScheme::Light),
        Some(ColorScheme::Dark),
        Some(ColorScheme::Auto),
    ] {
        let plain = renderer.render_with(FLOW, &default(scheme)).unwrap();
        let designed = renderer.render_with(FLOW, &design(scheme)).unwrap();
        assert_ne!(plain, designed);
        let (plain, designed) = (
            without_title_transforms(&plain),
            without_title_transforms(&designed),
        );
        // The drawing's box, every position, and the points of every route.
        assert_eq!(
            attribute_values(&plain, "viewBox")[0],
            attribute_values(&designed, "viewBox")[0],
            "the box with {scheme:?}"
        );
        for name in ["transform", "data-points"] {
            assert_eq!(
                attribute_values(&plain, name),
                attribute_values(&designed, name),
                "{name} with {scheme:?}"
            );
        }
        // The outline of every node.
        let (plain, designed) = (nodes(&plain), nodes(&designed));
        for name in ["d", "points", "x", "y", "width", "height", "r"] {
            assert_eq!(
                attribute_values(plain, name),
                attribute_values(designed, name),
                "{name} of the nodes with {scheme:?}"
            );
        }
    }
}

#[test]
fn an_edge_turns_on_a_wider_radius_through_the_same_points() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    let (plain, designed) = (edges(&plain), edges(&designed));
    assert_eq!(
        attribute_values(plain, "data-points"),
        attribute_values(designed, "data-points")
    );
    // The first edge turns twice: its corners start 5 before the turn by default, 10 in the mode.
    let turn = |svg: &str| attribute_values(svg, "d")[0].matches('Q').count();
    assert_eq!(turn(plain), 2);
    assert_eq!(turn(designed), 2);
    assert_ne!(
        attribute_values(plain, "d"),
        attribute_values(designed, "d")
    );
    // A straight edge is the same path.
    let straight = "flowchart TD\n  A --> B\n";
    let plain = renderer.render_with(straight, &default(None)).unwrap();
    let designed = renderer.render_with(straight, &design(None)).unwrap();
    assert_eq!(
        attribute_values(edges(&plain), "d"),
        attribute_values(edges(&designed), "d")
    );
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
fn an_edge_label_sits_in_a_chip() {
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
    let (x, width, height, radius) = background(&plain);
    let (chip_x, chip_width, chip_height, chip_radius) = background(&designed);
    assert_eq!(radius, None);
    assert_eq!(chip_radius.as_deref(), Some("6"));
    assert_eq!(chip_x, x - 5.0);
    assert_eq!(chip_width, width + 10.0);
    assert_eq!(chip_height, height);
    assert!(
        style_sheet(&designed).contains(
            "#d .edgeLabel rect.background{stroke:var(--clusterBorder);stroke-width:1px;}"
        )
    );
}

#[test]
fn a_group_title_sits_at_the_left_and_clear_of_the_edges() {
    let renderer = Renderer::new(&Value::empty_object());
    let plain = renderer.render_with(FLOW, &default(None)).unwrap();
    let designed = renderer.render_with(FLOW, &design(None)).unwrap();
    // The default centers the title of `Billing` on its frame, where the edge from `Load profile`
    // enters; the design mode sets it 16 from the frame's left.
    assert_eq!(frame_x(&plain), frame_x(&designed));
    assert_eq!(title_x(&designed), frame_x(&designed) + 16.0);
    assert!(title_x(&plain) > title_x(&designed));

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
