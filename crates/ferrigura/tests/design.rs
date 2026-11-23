//! The design mode (`docs/decisions/0023-design-mode.md`): ferrigura's own look for a flowchart. It
//! changes paint and leaves the geometry of the default rendering as it is.

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

/// The style sheet of an SVG.
fn style_sheet(svg: &str) -> &str {
    let start = svg.find("<style>").expect("a style sheet") + "<style>".len();
    let end = svg.find("</style>").expect("the style sheet's end");
    &svg[start..end]
}

#[test]
fn the_design_mode_keeps_the_geometry_of_the_default_rendering() {
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
        for name in [
            "viewBox",
            "transform",
            "data-points",
            "d",
            "points",
            "x",
            "y",
            "width",
            "height",
            "r",
        ] {
            assert_eq!(
                attribute_values(&plain, name),
                attribute_values(&designed, name),
                "{name} with {scheme:?}"
            );
        }
    }
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
    assert_eq!(attribute_values(&svg, "rx"), ["12", "8", "8", "8"]);
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
    assert!(designed.contains("fill=\"#F2F4F8\" stroke=\"#D6DCE5\""));
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
    assert!(style_sheet(&dark).contains("--terminalBkg:#4F70F0;"));
    assert!(style_sheet(&auto).contains("--mainBkg:light-dark(#FFFFFF,#1D212A);"));
    assert!(style_sheet(&auto).contains("--terminalBkg:light-dark(#2F54D6,#4F70F0);"));
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
    assert!(style_sheet(&svg).contains("--terminalBkg:#4F70F0;"));
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
    assert!(sheet.contains("--nodeBorder:#C4CBD6;"));

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
