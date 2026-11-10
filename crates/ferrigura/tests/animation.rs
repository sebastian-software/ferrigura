//! The public draw option: native, deterministic SVG with unchanged static geometry.

use ferrigura::{Animation, AnimationOrder, ColorScheme, RenderOptions, Renderer, Value};

const TEXT: &str = "flowchart TD\n subgraph group[Work]\n A[Start] edge@-->|go| B{Review}\n B -.-> A\n end\n B <--> C[Done]\n click A href \"https://example.com\"\n linkStyle 0 stroke:#f00,stroke-width:4px\n";

fn paths(svg: &str) -> Vec<(String, String, String)> {
    roxmltree::Document::parse(svg)
        .unwrap()
        .descendants()
        .filter(|node| node.attribute("data-edge") == Some("true"))
        .map(|node| {
            (
                node.attribute("data-id").unwrap().to_owned(),
                node.attribute("d").unwrap_or_default().to_owned(),
                node.attribute("style").unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn enabling_animation_preserves_geometry_styles_links_and_font_subsets() {
    let renderer = Renderer::new(&Value::empty_object());
    for compat in [false, true] {
        for order in [AnimationOrder::Source, AnimationOrder::Flow] {
            let mut options = RenderOptions {
                id: Some("draw-test".to_owned()),
                compat,
                color_scheme: Some(ColorScheme::Auto),
                ..RenderOptions::default()
            };
            let plain = renderer.render_with(TEXT, &options).unwrap();
            let mut animation = Animation::new(1.5).unwrap();
            animation.order = order;
            options.animation = Some(animation);
            let animated = renderer.render_with(TEXT, &options).unwrap();
            assert_eq!(paths(&plain), paths(&animated));
            let plain_xml = roxmltree::Document::parse(&plain).unwrap();
            let animated_xml = roxmltree::Document::parse(&animated).unwrap();
            assert_eq!(
                plain_xml.root_element().attribute("viewBox"),
                animated_xml.root_element().attribute("viewBox")
            );
            assert!(animated.contains("prefers-reduced-motion:no-preference"));
            assert!(animated.contains("data-draw-duration="));
            assert!(animated.contains("data:font/ttf;base64,"));
            assert!(animated.contains("href=\"https://example.com\""));
            assert!(!animated.contains("<script"));
            let draw_style = animated_xml
                .descendants()
                .find(|node| node.has_attribute("data-draw-style"))
                .unwrap()
                .text()
                .unwrap();
            assert!(
                !draw_style.contains("infinite"),
                "the draw animation runs once"
            );
            assert_eq!(animated, renderer.render_with(TEXT, &options).unwrap());
            options.animation = None;
            assert_eq!(
                plain,
                renderer.render_with(TEXT, &options).unwrap(),
                "animation does not leak into a reused renderer"
            );
        }
    }
}

#[test]
fn unsupported_types_are_explicit_and_speed_overflow_is_an_option_error() {
    let renderer = Renderer::new(&Value::empty_object());
    let mut options = RenderOptions {
        animation: Some(Animation::default()),
        ..RenderOptions::default()
    };
    let error = renderer
        .render_with("sequenceDiagram\n A->>B: Hi\n", &options)
        .unwrap_err();
    assert!(matches!(error, ferrigura::Error::Options(_)));
    options.animation = Some(Animation::new(f64::MIN_POSITIVE).unwrap());
    assert!(matches!(
        renderer.render_with("flowchart LR\nA-->B\n", &options),
        Err(ferrigura::Error::Options(_))
    ));
    assert!(renderer.render("pie\n\"A\":1\n").is_ok());
}

const CHARTS: &[(&str, &str)] = &[
    (
        "gantt",
        "gantt\n dateFormat YYYY-MM-DD\n todayMarker off\n section Work\n Design :a, 2026-01-05, 3d\n Build :b, after a, 4d\n Ship :milestone, after b, 0d\n",
    ),
    (
        "gitgraph",
        "gitGraph\n commit id: \"Start\"\n branch docs\n commit id: \"Guide\"\n checkout main\n commit id: \"Fix\"\n merge docs id: \"Merge\"\n",
    ),
    (
        "mindmap",
        "---\nconfig:\n layout: tidy-tree\n---\nmindmap\n root((Release))\n  Research\n   Interviews\n  Build\n   Parser\n   SVG\n",
    ),
    (
        "pie",
        "pie showData\n title Work\n \"Layout\":45\n \"Text\":30\n \"SVG\":25\n",
    ),
    (
        "donut",
        "---\nconfig:\n pie:\n  donutHole: 0.55\n---\npie\n \"Compile\":60\n \"Link\":25\n \"Package\":15\n",
    ),
    ("pie-small", "pie\n \"Tiny\":0.1\n \"Main\":99.9\n"),
    (
        "xy-named",
        "xychart\n x-axis [Mon, Tue, Wed]\n y-axis 0 --> 100\n bar [5, 10, 15]\n line \"Trend\" [20, 55, 80]\n bar \"Builds\" [30, 60, 90]\n line \"Goal\" [35, 65, 95]\n",
    ),
    (
        "xychart",
        "xychart-beta\n x-axis [Mon, Tue, Wed]\n y-axis 0 --> 100\n bar [30, 60, 90]\n line [20, 55, 80]\n",
    ),
    (
        "xy-horizontal",
        "xychart-beta horizontal\n x-axis [Planned, Active, Done]\n y-axis 0 --> 50\n bar [40, 26, 18]\n",
    ),
];

#[derive(Debug, PartialEq)]
struct Mark {
    tag: String,
    attributes: Vec<(String, String)>,
    text: String,
}

fn geometry(svg: &str) -> Vec<Mark> {
    roxmltree::Document::parse(svg)
        .unwrap()
        .descendants()
        .filter(|node| {
            matches!(
                node.tag_name().name(),
                "path" | "rect" | "circle" | "ellipse" | "polygon" | "polyline" | "line" | "text"
            )
        })
        .filter(|node| {
            !node.ancestors().any(|ancestor| {
                ancestor.has_attribute("data-draw-defs")
                    || ancestor.attribute("data-draw-part") == Some("marker")
            })
        })
        .map(|node| Mark {
            tag: node.tag_name().name().to_owned(),
            attributes: node
                .attributes()
                .filter(|attribute| !matches!(attribute.name(), "marker-start" | "marker-end"))
                .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
                .collect(),
            text: node.text().unwrap_or_default().to_owned(),
        })
        .collect()
}

#[test]
fn each_supported_diagram_preserves_its_static_marks_and_releases_per_render_state() {
    let renderer = Renderer::new(&Value::empty_object());
    for (name, text) in CHARTS {
        let mut options = RenderOptions {
            id: Some(format!("draw-{name}")),
            ..RenderOptions::default()
        };
        let plain = renderer.render_with(text, &options).unwrap();
        for order in [AnimationOrder::Source, AnimationOrder::Flow] {
            let mut animation = Animation::new(1.5).unwrap();
            animation.order = order;
            options.animation = Some(animation);
            let animated = renderer.render_with(text, &options).unwrap();
            assert_eq!(geometry(&plain), geometry(&animated), "{name}/{order:?}");
            let xml = roxmltree::Document::parse(&animated).unwrap();
            let duration = xml
                .descendants()
                .find_map(|node| node.attribute("data-draw-duration"))
                .unwrap()
                .parse::<f64>()
                .unwrap();
            assert!(duration.is_finite() && duration > 0.0, "{name}");
            assert!(animated.contains("cubic-bezier(0.4,0,0.2,1)"), "{name}");
            assert!(
                xml.descendants()
                    .any(|node| node.attribute("data-draw-part") == Some("mask")),
                "{name}"
            );
            assert_eq!(
                animated,
                renderer.render_with(text, &options).unwrap(),
                "{name}"
            );
            options.animation = None;
            assert_eq!(
                plain,
                renderer.render_with(text, &options).unwrap(),
                "{name}"
            );
        }
    }
}

#[test]
fn empty_and_node_only_flowcharts_have_finite_timing() {
    let options = RenderOptions {
        animation: Some(Animation::default()),
        ..RenderOptions::default()
    };
    for text in ["flowchart TD\n", "flowchart TD\n A[Alone]\n B[Separate]\n"] {
        let svg = ferrigura::render_with(text, &Value::empty_object(), &options).unwrap();
        let xml = roxmltree::Document::parse(&svg).unwrap();
        let timing = xml
            .descendants()
            .find_map(|node| node.attribute("data-draw-duration"))
            .unwrap();
        assert!(timing.parse::<f64>().unwrap().is_finite());
    }
}
