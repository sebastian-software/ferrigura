//! Real renderer output for the browser's per-design alias experiment.
use ferrigura::{FontStyle, FontWeight, RenderOptions, Renderer, Value};

#[test]
#[ignore = "exports fixtures for the host font alias browser comparison"]
fn export_font_alias_browser_fixtures() {
    let font_dir = format!("{}/crates/ferrigura-text/fonts", env!("CARGO_MANIFEST_DIR"));
    let config = Value::from(serde_json::json!({
        "fontFamily": "Fixture",
        "sequence": { "messageFontWeight": 550 }
    }));
    let mut builder = Renderer::builder(&config).with_all();
    for (family, file, weight, style) in [
        (
            "Fixture",
            "LexicaUltralegible-Regular.ttf",
            400,
            FontStyle::Normal,
        ),
        (
            "Fixture",
            "LexicaUltralegible-Bold.ttf",
            700,
            FontStyle::Normal,
        ),
        (
            "Fixture",
            "LexicaUltralegible-Italic.ttf",
            400,
            FontStyle::Italic,
        ),
        (
            "Fixture",
            "LexicaUltralegible-BoldItalic.ttf",
            700,
            FontStyle::Italic,
        ),
        (
            "Arrows",
            "NotoSansMath-Regular-Arrows.ttf",
            500,
            FontStyle::Normal,
        ),
        (
            "Emoji",
            "TwemojiMozilla-COLRv0-Emoji.ttf",
            300,
            FontStyle::Normal,
        ),
    ] {
        builder = builder
            .font_file_face(
                family,
                format!("{font_dir}/{file}"),
                FontWeight::Number(weight),
                style,
            )
            .unwrap();
    }
    let renderer = builder.font_fallbacks(["Emoji", "Arrows"]).build();
    let cases = [
        ("display", "info"),
        ("sequence", "sequenceDiagram\n A->>B: Plain e\u{301} → 😀"),
        (
            "c4",
            "C4Context\n Person(a, \"Person e\u{301} →\")\n System(b, \"System 😀\")\n Rel(a,b,\"Uses\")",
        ),
        ("plain", "flowchart LR\n A[Plain e\u{301} → 😀] --> B[Next]"),
        (
            "markdown",
            "flowchart LR\n A[\"`Plain **bold** and *italic* e\u{301} → 😀`\"] --> B[Next]",
        ),
    ];
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/font-alias-browser");
    std::fs::create_dir_all(&directory).unwrap();
    for (name, text) in cases {
        let svg = renderer
            .render_with(
                text,
                &RenderOptions {
                    id: Some("alias-check".to_owned()),
                    ..RenderOptions::default()
                },
            )
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        std::fs::write(directory.join(format!("{name}.svg")), svg).unwrap();
    }
}
