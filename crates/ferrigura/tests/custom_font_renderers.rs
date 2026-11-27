//! Custom renderers resolve host files for their measurement and SVG text roles.

use ferrigura::{FontStyle, FontWeight, RenderOptions, Renderer, Value};

const SANS: &[u8] = include_bytes!("../../ferrigura-text/fonts/LexicaUltralegible-Regular.ttf");
const MONO: &[u8] =
    include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");
const ARROWS: &[u8] = include_bytes!("../../ferrigura-text/fonts/NotoSansMath-Regular-Arrows.ttf");

const CASES: [(&str, &str); 4] = [
    (
        "sequence",
        "sequenceDiagram\n autonumber\n AVATAR->>width: AVATAR width",
    ),
    (
        "gantt",
        "gantt\n dateFormat YYYY-MM-DD\n todayMarker off\n AVATAR width :a, 2026-01-01, 2d",
    ),
    (
        "xychart",
        "xychart\n title \"AVATAR width\"\n x-axis [AVATAR, width]\n y-axis 0 --> 10\n line [3,7]",
    ),
    (
        "quadrant",
        "quadrantChart\n title AVATAR width\n x-axis AVATAR --> width\n y-axis AVATAR --> width\n AVATAR: [0.3, 0.6]",
    ),
];

fn config() -> Value {
    Value::from(serde_json::json!({
        "fontFamily":"Fixture"
    }))
}

fn renderer(bytes: &[u8]) -> Renderer {
    Renderer::builder(&config())
        .with_all()
        .font("Fixture", bytes.to_vec())
        .unwrap()
        .font("monospace", bytes.to_vec())
        .unwrap()
        .build()
}

fn view_box(svg: &str) -> String {
    roxmltree::Document::parse(svg)
        .unwrap()
        .root_element()
        .attribute("viewBox")
        .unwrap()
        .to_owned()
}

#[test]
fn all_custom_diagram_types_embed_and_bind_supplied_faces() {
    let renderer = renderer(SANS);
    for (kind, text) in CASES {
        let svg = renderer
            .render(text)
            .unwrap_or_else(|error| panic!("{kind}: {error}"));
        roxmltree::Document::parse(&svg).unwrap();
        assert!(svg.contains("@font-face{"), "{kind}");
        assert!(svg.contains("var(--fg-font-"), "{kind}");
        assert!(svg.contains("font-synthesis:none"), "{kind}");
    }
}

#[test]
fn same_alias_different_files_change_custom_renderer_geometry() {
    let sans = renderer(SANS);
    let mono = renderer(MONO);
    let options = RenderOptions {
        id: Some("same".into()),
        ..RenderOptions::default()
    };
    let (_, text) = CASES[0];
    let first = sans.render_with(text, &options).unwrap();
    let second = mono.render_with(text, &options).unwrap();
    assert_ne!(view_box(&first), view_box(&second));
}

#[test]
fn a_custom_renderer_binds_fallback_only_and_mixed_runs() {
    let renderer = Renderer::builder(&config())
        .with_all()
        .font("Fixture", SANS.to_vec())
        .unwrap()
        .font("Arrows", ARROWS.to_vec())
        .unwrap()
        .font_fallbacks(["Arrows"])
        .build();
    for text in [
        "sequenceDiagram\n A->>B: →",
        "sequenceDiagram\n A->>B: AV → AV",
    ] {
        let svg = renderer.render(text).unwrap();
        assert!(svg.matches("@font-face{").count() >= 2);
        assert!(svg.contains("var(--fg-font-"));
    }
}

/// The browser harness consumes these exact public-API fixtures, including their role inheritance.
/// Distinct regular/bold/italic designs expose CSS that accidentally inherits the primary family.
#[test]
#[ignore = "exports fixtures for pnpm -C tools/port test:custom-font-renderers"]
fn export_custom_renderer_browser_fixtures() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/custom-font-renderers");
    std::fs::create_dir_all(&directory).unwrap();
    let mut files = Vec::new();
    for (variant, bytes, bold) in [("sans", SANS, MONO), ("mono", MONO, SANS)] {
        let proof_config = Value::from(serde_json::json!({
            "fontFamily":"Fixture", "sequence":{"messageFontWeight":700}
        }));
        let renderer = Renderer::builder(&proof_config)
            .with_all()
            .font("Fixture", bytes.to_vec())
            .unwrap()
            .font_face(
                "Fixture",
                bold.to_vec(),
                FontWeight::Bold,
                FontStyle::Normal,
            )
            .unwrap()
            .font_face(
                "Fixture",
                MONO.to_vec(),
                FontWeight::Normal,
                FontStyle::Italic,
            )
            .unwrap()
            .font_face(
                "Fixture",
                MONO.to_vec(),
                FontWeight::Bold,
                FontStyle::Italic,
            )
            .unwrap()
            .font("monospace", bytes.to_vec())
            .unwrap()
            .build();
        for (kind, text) in CASES {
            let svg = renderer
                .render_with(
                    text,
                    &RenderOptions {
                        id: Some("custom-font-proof".into()),
                        ..RenderOptions::default()
                    },
                )
                .unwrap_or_else(|error| panic!("{kind}: {error}"));
            let file = format!("{kind}-{variant}.svg");
            std::fs::write(directory.join(&file), svg).unwrap();
            files.push(file);
        }
    }
    std::fs::write(
        directory.join("fixtures.json"),
        serde_json::to_string(&files).unwrap(),
    )
    .unwrap();
}
