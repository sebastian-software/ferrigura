use ferrigura::{LayoutEngine, RenderOptions, Renderer, Value};

fn renderer() -> Renderer {
    Renderer::builder(&Value::empty_object()).with_all().build()
}

#[test]
fn explicit_elk_preserves_legacy_output() {
    let renderer = renderer();
    let source = "---\nconfig:\n  layout: dagre\n---\nflowchart LR\nA --> B";
    for compat in [false, true] {
        let options = RenderOptions {
            compat,
            ..RenderOptions::default()
        };
        assert_eq!(
            renderer
                .render_with_engine(source, &options, LayoutEngine::Elk)
                .unwrap(),
            renderer.render_with(source, &options).unwrap(),
        );
    }
    let error = renderer
        .render_with_engine(
            "gantt\n  title Plan",
            &RenderOptions::default(),
            LayoutEngine::Elk,
        )
        .unwrap_err();
    assert!(error.to_string().contains("ELK is not available"));
}

#[cfg(not(feature = "experimental-tala"))]
#[test]
fn tala_request_in_a_default_build_has_an_actionable_error() {
    let error = renderer()
        .render_with_engine(
            "flowchart LR\nA[Node]",
            &RenderOptions::default(),
            LayoutEngine::Tala,
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("enable the experimental-tala feature")
    );
}

#[cfg(feature = "experimental-tala")]
#[test]
fn tala_uses_native_placement_and_routing_without_falling_back() {
    let renderer = renderer();
    let options = RenderOptions::default();
    let source = "---\nconfig:\n  layout: elk\n---\nflowchart TB\nA[Worker] -->|retry| A";
    let render = || {
        renderer
            .render_with_engine(source, &options, LayoutEngine::Tala)
            .unwrap()
    };
    let svg = render();
    assert_eq!(svg, render());
    assert_ne!(svg, renderer.render_with(source, &options).unwrap());
    let root = roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(root.root_element().tag_name().name(), "svg");
    assert!(root.descendants().any(|node| node.text() == Some("retry")));

    for source in [
        "flowchart LR\nA --> B\nA --> C",
        "flowchart LR\nsubgraph System\nA[Worker]\nC[Store]\nend\nA --> B[Queue]",
    ] {
        let error = renderer
            .render_with_engine(source, &options, LayoutEngine::Tala)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("outside the experimental TALA port"),
            "{error}"
        );
        assert!(renderer.render_with(source, &options).is_ok());
    }
    for (source, options, expected) in [
        (
            "gantt\n  title Plan",
            RenderOptions::default(),
            "only available for flowcharts",
        ),
        (
            source,
            RenderOptions {
                compat: true,
                ..options
            },
            "cannot be used with compatibility mode",
        ),
    ] {
        let error = renderer
            .render_with_engine(source, &options, LayoutEngine::Tala)
            .unwrap_err();
        assert!(error.to_string().contains(expected));
    }
}

#[cfg(feature = "experimental-tala")]
#[test]
fn tala_draws_connected_paths_and_bounded_nested_subgraphs() {
    let renderer = renderer();
    let options = RenderOptions::default();
    for (text, labels) in [
        (
            "flowchart LR\nA[Alpha] -->|ok| B[Beta]",
            &["Alpha", "Beta", "ok"][..],
        ),
        (
            "flowchart LR\nAlpha -->|Ready| Beta --> Gamma --> Delta",
            &["Alpha", "Beta", "Gamma", "Delta", "Ready"][..],
        ),
        (
            "flowchart TB\nsubgraph Outer\nsubgraph Inner\nA[Worker]\nend\nend",
            &["Outer", "Inner", "Worker"][..],
        ),
        (
            "flowchart LR\nsubgraph System\nA[Worker]\nend\nA --> B[Queue]",
            &["System", "Worker", "Queue"][..],
        ),
        (
            "flowchart LR\nSensor -->|reading| Panel\nSensor -->|rate| Panel",
            &["Sensor", "Panel", "reading", "rate"][..],
        ),
        (
            "flowchart TB\nsubgraph Left\nA[Worker]\nend\nsubgraph Right\nB[Queue]\nend",
            &["Left", "Worker", "Right", "Queue"][..],
        ),
    ] {
        let svg = renderer
            .render_with_engine(text, &options, LayoutEngine::Tala)
            .unwrap_or_else(|error| panic!("{text}: {error}"));
        let xml = roxmltree::Document::parse(&svg).unwrap();
        for label in labels {
            assert!(
                xml.descendants().any(|node| node.text() == Some(label)),
                "{label}"
            );
        }
    }
}

#[cfg(feature = "experimental-tala")]
#[test]
fn tala_uses_the_shared_theme_font_and_animation_path() {
    use ferrigura::{Animation, ColorScheme};

    let font = include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");
    let renderer = Renderer::builder(&Value::empty_object())
        .with_all()
        .font("Fixture Mono", font.to_vec())
        .unwrap()
        .build();
    let source = "flowchart LR\n A[Worker] -->|retry| A";
    let render = |options| {
        renderer
            .render_with_engine(source, &options, LayoutEngine::Tala)
            .unwrap()
    };
    let options = RenderOptions {
        color_scheme: Some(ColorScheme::Auto),
        id: Some("tala-styled".into()),
        ..RenderOptions::default()
    };
    let still = render(options.clone());
    assert!(still.contains("id=\"tala-styled\""));
    assert!(still.contains("light-dark("));
    assert!(still.contains("data:font/"));
    assert!(still.contains("--fg-font-"));
    assert!(!still.contains("data-draw-style"));

    let animated = render(RenderOptions {
        animation: Some(Animation::default()),
        ..options
    });
    assert!(animated.contains("data-draw-part=\"edge\""));
    assert!(animated.contains("prefers-reduced-motion:no-preference"));
}

#[cfg(feature = "experimental-tala")]
#[test]
fn tala_reports_its_work_limit_without_changing_subsequent_renders() {
    let renderer = renderer();
    let render =
        |source| renderer.render_with_engine(source, &RenderOptions::default(), LayoutEngine::Tala);
    let small = "flowchart TB\nA --> A";
    let reference = render(small).unwrap();
    let large = format!("flowchart TB\n{}", "A --> A\n".repeat(33));
    let error = render(&large).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("TALA input exceeds its work limit")
    );
    assert_eq!(render(small).unwrap(), reference);
}
