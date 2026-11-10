//! Supplied local files must determine both layout and the fonts named by the SVG.

use ferrigura::{FontStyle, FontWeight, RenderOptions, Renderer, Value};

const SANS: &[u8] = include_bytes!("../../ferrigura-text/fonts/LexicaUltralegible-Regular.ttf");
const MONO: &[u8] =
    include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");
const ARROWS: &[u8] = include_bytes!("../../ferrigura-text/fonts/NotoSansMath-Regular-Arrows.ttf");
const CFF: &[u8] = include_bytes!("../../ferrigura-text/tests/fixtures/SourceCodePro-Regular.otf");

fn renderer(bytes: &[u8]) -> Renderer {
    Renderer::builder(&Value::empty_object())
        .with_all()
        .font("Fixture", bytes.to_vec())
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
fn supplied_same_alias_fonts_change_unified_markdown_and_custom_layout() {
    let sans = renderer(SANS);
    let mono = renderer(MONO);
    let options = RenderOptions {
        id: Some("same".into()),
        ..RenderOptions::default()
    };
    for text in [
        "flowchart LR\n A[AVATAR width]",
        "flowchart LR\n A[\"`**AVATAR** *width*`\"]",
        "pie\n \"AVATAR width\": 1\n \"i\": 2",
    ] {
        let first = sans.render_with(text, &options).unwrap();
        let second = mono.render_with(text, &options).unwrap();
        assert_ne!(view_box(&first), view_box(&second), "{text}");
        assert!(first.contains("font-synthesis:none"));
        assert!(second.contains("--fg-font-"));
    }
}

#[test]
fn concurrent_renderers_keep_same_alias_bytes_and_svg_identities_isolated() {
    let first = renderer(SANS);
    let second = renderer(MONO);
    let text = "flowchart LR\n A[AVATAR]-->B[width]";
    let expected_first = first.render(text).unwrap();
    let expected_second = second.render(text).unwrap();
    let first_id = roxmltree::Document::parse(&expected_first)
        .unwrap()
        .root_element()
        .attribute("id")
        .unwrap()
        .to_owned();
    let second_id = roxmltree::Document::parse(&expected_second)
        .unwrap()
        .root_element()
        .attribute("id")
        .unwrap()
        .to_owned();
    assert_ne!(first_id, second_id);
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            for _ in 0..8 {
                assert_eq!(first.render(text).unwrap(), expected_first);
            }
        });
        let b = scope.spawn(|| {
            for _ in 0..8 {
                assert_eq!(second.render(text).unwrap(), expected_second);
            }
        });
        a.join().unwrap();
        b.join().unwrap();
    });
}

#[test]
fn fallback_only_and_mixed_labels_bind_the_original_requested_face() {
    let renderer = Renderer::builder(&Value::empty_object())
        .with_all()
        .font("Primary", SANS.to_vec())
        .unwrap()
        .font("Arrows", ARROWS.to_vec())
        .unwrap()
        .font_fallbacks(["Arrows"])
        .build();
    let only = renderer.render("flowchart LR\n A[\"→\"]").unwrap();
    assert_eq!(only.matches("@font-face{").count(), 1);
    assert!(only.contains("font-family:var(--fg-font-"));
    assert!(only.contains("font-synthesis:none"));
    let mixed = renderer.render("flowchart LR\n A[\"AV → AV\"]").unwrap();
    assert_eq!(mixed.matches("@font-face{").count(), 2);
}

#[test]
fn missing_registered_primary_names_fail_with_registration_guidance() {
    let config = Value::from(serde_json::json!({"fontFamily":"Unregistered"}));
    let renderer = Renderer::builder(&config)
        .with_all()
        .font("Fixture", SANS.to_vec())
        .unwrap()
        .build();
    let error = renderer
        .render("flowchart LR\n A[AV]")
        .unwrap_err()
        .to_string();
    assert!(error.contains("Unregistered"));
    assert!(error.contains("register"));
}

#[test]
fn supplied_weight_and_style_fallbacks_keep_distinct_measured_designs() {
    let mut fonts = ferrigura::FontRegistry::new();
    fonts
        .register(
            "Code",
            CFF.to_vec(),
            FontWeight::Number(800),
            FontStyle::Normal,
        )
        .unwrap();
    assert!(
        fonts
            .resolve("Code", FontWeight::Normal, FontStyle::Normal)
            .unwrap()
            .covers('→')
    );
    let renderer = Renderer::builder(&Value::empty_object())
        .with_all()
        .font_face(
            "Primary",
            SANS.to_vec(),
            FontWeight::Number(600),
            FontStyle::Normal,
        )
        .unwrap()
        .font_face(
            "Fallback",
            ARROWS.to_vec(),
            FontWeight::Number(500),
            FontStyle::Normal,
        )
        .unwrap()
        .font_face(
            "Fallback",
            CFF.to_vec(),
            FontWeight::Number(800),
            FontStyle::Normal,
        )
        .unwrap()
        .font_fallbacks(["Fallback"])
        .build();
    let svg = renderer
        .render("flowchart LR\n A[\"`**→** *→* →`\"]")
        .unwrap();
    // Inter-word blanks use the primary face; the arrows use two distinct fallback designs.
    assert_eq!(svg.matches("@font-face{").count(), 3);
    assert!(svg.contains("font-weight:500"));
    assert!(svg.contains("font-weight:800"));
}
