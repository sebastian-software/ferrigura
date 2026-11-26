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

#[test]
fn class_click_urls_resolve_mermaid_entities_and_enforce_security_level() {
    let entity_https = render(
        "classDiagram\n  class A\n  click A href \"https#58;//example.org/\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        entity_https.contains("xlink:href=\"https://example.org/\""),
        "{entity_https}"
    );

    // Percent-encoded `#` decoded by `formatUrl` must not be re-encoded into a Mermaid entity
    // placeholder after URL formatting (matching Mermaid's `encodeEntities` -> `formatUrl` order).
    let percent_hash = render(
        "classDiagram\n  class A\n  click A href \"/docs/%23copy;\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        percent_hash.contains("xlink:href=\"/docs/#copy;\""),
        "{percent_hash}"
    );
    assert!(
        !percent_hash.contains("xlink:href=\"/docs/©\""),
        "{percent_hash}"
    );

    for unsafe_url in [
        "javascript#58;alert(1)",
        "javascript#colon;alert(1)",
        "notes#58;meeting",
    ] {
        let svg = render(
            &format!("classDiagram\n  class A\n  click A href \"{unsafe_url}\"\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            svg.contains("xlink:href=\"about:blank\""),
            "url={unsafe_url:?}: {svg}"
        );
    }

    let loose_cfg = Value::from(serde_json::json!({ "securityLevel": "loose" }));
    let loose = render(
        "classDiagram\n  class A\n  click A href \"javascript#58;alert(1)\"\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose.contains("xlink:href=\"javascript:alert(1)\""),
        "{loose}"
    );

    // Accessibility titles/descriptions and click tooltips decode Mermaid `#...;` entities instead
    // of leaking internal placeholders.
    let a11y_and_tooltip = render(
        "classDiagram\n  accTitle: Price #36;10\n  accDescr: Cost #36;20\n  class A\n  click A href \"https://example.org\" \"Price #36;10\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        a11y_and_tooltip.contains(">Price $10</title>"),
        "{a11y_and_tooltip}"
    );
    assert!(
        a11y_and_tooltip.contains(">Cost $20</desc>"),
        "{a11y_and_tooltip}"
    );
    assert!(
        a11y_and_tooltip.contains("title=\"Price $10\""),
        "{a11y_and_tooltip}"
    );
    assert!(!a11y_and_tooltip.contains("ﬂ°"), "{a11y_and_tooltip}");

    // Parse errors report the original source column even when earlier tokens on the line contain
    // Mermaid `#...;` entities.
    let err = ferrigura_syntax::class::parse("classDiagram\nclass A[\"#36;\"] @\n")
        .expect_err("invalid token fails");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 17);

    // Fragment links to class IDs containing Mermaid entities match the emitted element ID.
    let entity_id = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Invoice#45;Item`\n  click `Invoice#45;Item` href \"#graph-classId-Invoice#45;Item\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        entity_id.contains("id=\"graph-classId-Invoice-Item\""),
        "{entity_id}"
    );
    assert!(
        entity_id.contains("xlink:href=\"#graph-classId-Invoice-Item\""),
        "{entity_id}"
    );

    // Decoded line breaks and tabs in tooltips round-trip through XML attribute parsing.
    let nl_tooltip = render(
        "classDiagram\n  class A\n  click A href \"https://example.org\" \"First#10;Second#9;End\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let doc = roxmltree::Document::parse(&nl_tooltip).expect("well-formed SVG XML");
    let tooltip_val = doc
        .descendants()
        .find_map(|node| node.attribute("title"))
        .expect("title attribute");
    assert_eq!(tooltip_val, "First\nSecond\tEnd");

    // Color-semicolon stripping respects JS line terminators (U+2028/U+2029) and never crosses them.
    let ls_title = render(
        "classDiagram\n  accTitle: style fill:#fff\u{2028}Cost #36;\n  class A\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(ls_title.contains("Cost $</title>"), "{ls_title}");

    // Relation IDs resolve Mermaid entities consistently with fragment links.
    let relation_id = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  `Invoice#45;Item` --> Account : owns\n  click Account href \"#graph-id_Invoice#45;Item_Account_1\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        relation_id.contains("id=\"graph-id_Invoice-Item_Account_1\""),
        "{relation_id}"
    );
    assert!(
        relation_id.contains("data-id=\"id_Invoice-Item_Account_1\""),
        "{relation_id}"
    );
    assert!(
        relation_id.contains("xlink:href=\"#graph-id_Invoice-Item_Account_1\""),
        "{relation_id}"
    );

    // Colliding resolved DOM IDs (`Foo-Bar` vs `Foo#45;Bar`) are disambiguated along with their
    // fragment links so no duplicate IDs are emitted.
    let colliding = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo-Bar`\n  class `Foo#45;Bar`\n  click `Foo-Bar` href \"#graph-classId-Foo-Bar\"\n  click `Foo#45;Bar` href \"#graph-classId-Foo#45;Bar\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        colliding.contains("id=\"graph-classId-Foo-Bar\""),
        "{colliding}"
    );
    assert!(
        colliding.contains("id=\"graph-classId-Foo-Bar-1\""),
        "{colliding}"
    );
    assert!(
        colliding.contains("xlink:href=\"#graph-classId-Foo-Bar\""),
        "{colliding}"
    );
    assert!(
        colliding.contains("xlink:href=\"#graph-classId-Foo-Bar-1\""),
        "{colliding}"
    );

    // Sequence accessibility text decodes placeholders only once (no double decoding).
    let seq_a11y = render(
        "sequenceDiagram\n  accTitle: #64258;°copy;\n  accDescr: #64258;°copy;\n  A->>B: hi\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let seq_doc = roxmltree::Document::parse(&seq_a11y).expect("well-formed SVG XML");
    let seq_title = seq_doc
        .descendants()
        .find(|node| node.has_tag_name("title"))
        .and_then(|node| node.text())
        .expect("title text");
    let seq_desc = seq_doc
        .descendants()
        .find(|node| node.has_tag_name("desc"))
        .and_then(|node| node.text())
        .expect("desc text");
    assert_eq!(seq_title, "ﬂ°copy;");
    assert_eq!(seq_desc, "ﬂ°copy;");
}
