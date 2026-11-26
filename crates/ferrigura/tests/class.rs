//! Class diagrams through the public entry point: the colors of the redux-color theme, as
//! `classBox.ts` and the class `styles.js` write them.

use ferrigura::render;
use ferrigura_model::value::Value;

#[test]
fn each_class_box_takes_its_own_palette_slot() {
    // `classBox.ts` stamps `data-color-id` (`stampColorSlot`), and the palette rules of the sheet
    // color the box by it; without the stamp every box is black on white.
    let svg = render(
        "classDiagram\n  class A\n  class B\n  A --> B\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        svg.contains("classId-A\" data-color-id=\"color-0\""),
        "{svg}"
    );
    assert!(
        svg.contains("classId-B\" data-color-id=\"color-1\""),
        "{svg}"
    );
    assert!(
        svg.contains("[data-color-id=\"color-0\"].node .outer-path path{"),
        "{svg}"
    );
}

#[test]
fn a_class_relation_is_as_wide_as_the_theme_stroke() {
    // `.relation` writes `${options.strokeWidth}` as a value; a `var(--strokeWidth)` that no rule
    // declares would leave the edges at the inherited width.
    let svg = render("classDiagram\n  A --> B\n", &Value::empty_object()).expect("renders");
    assert!(
        svg.contains(" .relation{stroke:var(--lineColor);stroke-width:2;"),
        "{svg}"
    );
    assert!(!svg.contains("var(--strokeWidth)"), "{svg}");
}

#[test]
fn class_click_tooltip_respects_html_labels_and_security_level() {
    let source = "classDiagram\n  class A\n  click A href \"https://example.org\" \"x=y\"\n";
    let html_false = Value::from(serde_json::json!({
        "htmlLabels": false,
        "securityLevel": "custom",
    }));
    let svg_plain = render(source, &html_false).expect("renders");
    assert!(svg_plain.contains("title=\"x=y\""), "{svg_plain}");
    assert!(!svg_plain.contains("&amp;equals;"), "{svg_plain}");

    let html_true = Value::from(serde_json::json!({
        "htmlLabels": true,
        "securityLevel": "custom",
    }));
    let svg_escaped = render(source, &html_true).expect("renders");
    assert!(
        svg_escaped.contains("title=\"x&amp;equals;y\""),
        "{svg_escaped}"
    );

    // Omitting root htmlLabels falls back to legacy flowchart.htmlLabels (matching Mermaid's
    // getEffectiveHtmlLabels), while an explicit root htmlLabels takes precedence over it.
    let flowchart_false = Value::from(serde_json::json!({
        "flowchart": { "htmlLabels": false },
        "securityLevel": "custom",
    }));
    let svg_fallback_plain = render(source, &flowchart_false).expect("renders");
    assert!(
        svg_fallback_plain.contains("title=\"x=y\""),
        "{svg_fallback_plain}"
    );
    assert!(
        !svg_fallback_plain.contains("&amp;equals;"),
        "{svg_fallback_plain}"
    );

    let root_overrides = Value::from(serde_json::json!({
        "htmlLabels": true,
        "flowchart": { "htmlLabels": false },
        "securityLevel": "custom",
    }));
    let svg_root_wins = render(source, &root_overrides).expect("renders");
    assert!(
        svg_root_wins.contains("title=\"x&amp;equals;y\""),
        "{svg_root_wins}"
    );

    // JS String(val).trim() coercion: `[false]` and BOM-prefixed `"false"` evaluate to false.
    for coerced_false in [
        serde_json::json!({ "htmlLabels": [false], "securityLevel": "custom" }),
        serde_json::json!({ "htmlLabels": "\u{FEFF}false", "securityLevel": "custom" }),
    ] {
        let svg_coerced = render(source, &Value::from(coerced_false)).expect("renders");
        assert!(svg_coerced.contains("title=\"x=y\""), "{svg_coerced}");
        assert!(!svg_coerced.contains("&amp;equals;"), "{svg_coerced}");
    }
}
