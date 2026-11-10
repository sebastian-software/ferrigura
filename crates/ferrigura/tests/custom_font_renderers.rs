//! Custom renderers resolve host files for their measurement and SVG text roles.

use ferrigura::{FontStyle, FontWeight, RenderOptions, Renderer, Value};

const SANS: &[u8] = include_bytes!("../../ferrigura-text/fonts/LexicaUltralegible-Regular.ttf");
const MONO: &[u8] =
    include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");
const ARROWS: &[u8] = include_bytes!("../../ferrigura-text/fonts/NotoSansMath-Regular-Arrows.ttf");

const CASES: [(&str, &str); 22] = [
    (
        "sequence",
        "sequenceDiagram\n autonumber\n AVATAR->>width: AVATAR width",
    ),
    (
        "gantt",
        "gantt\n dateFormat YYYY-MM-DD\n todayMarker off\n AVATAR width :a, 2026-01-01, 2d",
    ),
    (
        "journey",
        "journey\n title AVATAR width\n section AVATAR\n AVATAR width: 5: AVATAR AVATAR AVATAR AVATAR AVATAR AVATAR AVATAR AVATAR AVATAR AVATAR",
    ),
    (
        "gitgraph",
        "gitGraph\n commit id: \"AVATAR width\" tag: \"AVATAR\"",
    ),
    (
        "timeline",
        "timeline\n title AVATAR width\n AVATAR : AVATAR width",
    ),
    (
        "c4",
        "C4Context\n Person(a, \"AVATAR width\", \"AVATAR\")\n System(b, \"AVATAR\")\n Rel(a,b,\"AVATAR width\",\"AVATAR\")",
    ),
    (
        "architecture",
        "architecture-beta\n service a(server)[AVATAR width]",
    ),
    ("kanban", "kanban\n AVATAR[AVATAR width]\n  a[AVATAR width]"),
    (
        "treemap",
        "treemap-beta\n \"AVATAR width\"\n  \"AVATAR width\": 10",
    ),
    ("ishikawa", "ishikawa-beta\n AVATAR width\n  AVATAR width"),
    (
        "eventmodeling",
        "eventmodeling\n tf 01 ui AVATARwidth\n tf 02 cmd AVATARwidth",
    ),
    (
        "wardley",
        "wardley-beta\n title AVATAR width\n anchor AVATAR [0.9, 0.8]\n component width [0.6, 0.5]",
    ),
    (
        "cynefin",
        "cynefin-beta\n title AVATAR width\n complex\n complicated\n clear\n chaotic",
    ),
    (
        "xychart",
        "xychart\n title \"AVATAR width\"\n x-axis [AVATAR, width]\n y-axis 0 --> 10\n line [3,7]",
    ),
    (
        "radar",
        "radar-beta\n title AVATAR width\n axis AVATAR, width, AV\n curve c1{1,2,3}",
    ),
    (
        "railroad",
        "railroad-beta\n AVATAR = terminal(\"AVATAR width\") ;",
    ),
    ("sankey", "sankey\n\n AVATAR,width,10"),
    (
        "packet",
        "packet\n title AVATAR width\n 0-10: \"AVATAR width\"",
    ),
    (
        "quadrant",
        "quadrantChart\n title AVATAR width\n x-axis AVATAR --> width\n y-axis AVATAR --> width\n AVATAR: [0.3, 0.6]",
    ),
    ("info", "info"),
    (
        "venn",
        "venn-beta\n set AVATAR\n set width\n union AVATAR, width",
    ),
    (
        "tree-view",
        "treeView-beta\n \"AVATAR width\"\n  \"AVATAR width\"",
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
    for kind in [
        "sequence",
        "journey",
        "gitgraph",
        "timeline",
        "ishikawa",
        "railroad",
        "tree-view",
    ] {
        let (_, text) = CASES.iter().find(|(name, _)| *name == kind).unwrap();
        let first = sans.render_with(text, &options).unwrap();
        let second = mono.render_with(text, &options).unwrap();
        assert_ne!(view_box(&first), view_box(&second), "{kind}");
    }
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

#[test]
fn configured_custom_font_roles_reject_unregistered_names() {
    let config = Value::from(
        serde_json::json!({"fontFamily":"Fixture", "journey":{"taskFontFamily":"MissingTask","titleFontFamily":"Fixture"}}),
    );
    let renderer = Renderer::builder(&config)
        .with_all()
        .font("Fixture", SANS.to_vec())
        .unwrap()
        .build();
    let error = renderer.render(CASES[2].1).unwrap_err().to_string();
    assert!(error.contains("MissingTask"), "{error}");
    assert!(error.contains("register"), "{error}");
}

#[test]
fn treemap_class_family_weight_and_style_select_actual_registered_designs() {
    let config = config();
    let renderer = Renderer::builder(&config)
        .with_all()
        .font("Fixture", SANS.to_vec())
        .unwrap()
        .font_face(
            "Leaf",
            MONO.to_vec(),
            FontWeight::Number(500),
            FontStyle::Italic,
        )
        .unwrap()
        .build();
    let text = "treemap-beta\n \"AVATAR\"\n  \"AVATAR width\": 10:::custom\n classDef custom font-family: Leaf,font-weight: 500,font-style: italic";
    let svg = renderer.render(text).unwrap();
    assert!(svg.contains("font-weight:500"));
    assert!(svg.contains("font-style:italic"));
    assert!(svg.matches("@font-face{").count() >= 2);
    let missing = text.replace("Leaf", "UnregisteredLeaf");
    let error = renderer.render(&missing).unwrap_err().to_string();
    assert!(error.contains("UnregisteredLeaf"), "{error}");
}

#[test]
fn explicit_c4_family_roles_remain_strict_after_global_inheritance() {
    let config = Value::from(
        serde_json::json!({"fontFamily":"Fixture", "c4":{"personFontFamily":"MissingPerson"}}),
    );
    let renderer = Renderer::builder(&config)
        .with_all()
        .font("Fixture", SANS.to_vec())
        .unwrap()
        .build();
    let error = renderer.render(CASES[5].1).unwrap_err().to_string();
    assert!(error.contains("MissingPerson"), "{error}");
    assert!(error.contains("register"), "{error}");
}
