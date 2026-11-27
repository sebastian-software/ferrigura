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

/// Resolves a same-document `#...` URL fragment the way HTML navigation does: strip leading and
/// trailing C0 controls and space per WHATWG URL parsing, strip ASCII tab/LF/CR, take the fragment
/// after `#`, and percent-decode UTF-8 bytes.
fn resolved_fragment_target(href: &str) -> String {
    let trimmed = href.trim_matches(|ch: char| ch <= ' ');
    let cleaned: String = trimmed
        .chars()
        .filter(|ch| !matches!(ch, '\t' | '\n' | '\r'))
        .collect();
    let fragment = cleaned
        .strip_prefix('#')
        .expect("expected same-document fragment URL");
    let bytes = fragment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            decoded.push(((hi << 4) | lo) as u8);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).expect("valid UTF-8 decoded fragment")
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

    // Class style entities (`#44;` for `,` inside `rgb(...)`) resolve before emitting SVG `style`.
    let rgb_style = render(
        "classDiagram\n  class A\n  style A fill:rgb(255#44;0#44;0);\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(rgb_style.contains("fill:rgb(255,0,0)"), "{rgb_style}");
    assert!(!rgb_style.contains("ﬂ°"), "{rgb_style}");

    // XML attribute whitespace normalization collapses tabs to spaces, so `Foo Bar` and
    // `Foo#9;Bar` are disambiguated as parsed XML IDs and their fragment links match uniquely.
    let tab_collide = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo Bar`\n  class `Foo#9;Bar`\n  click `Foo Bar` href \"#graph-classId-Foo Bar\"\n  click `Foo#9;Bar` href \"#graph-classId-Foo#9;Bar\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let tab_doc = roxmltree::Document::parse(&tab_collide).expect("well-formed SVG XML");
    let node_ids: Vec<&str> = tab_doc
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .filter(|id| id.starts_with("graph-classId-Foo"))
        .collect();
    assert_eq!(
        node_ids,
        vec!["graph-classId-Foo Bar", "graph-classId-Foo Bar-1"]
    );
    let hrefs: Vec<&str> = tab_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert_eq!(
        hrefs,
        vec!["#graph-classId-Foo Bar", "#graph-classId-Foo%20Bar-1"]
    );
    let resolved_targets: Vec<String> = hrefs
        .iter()
        .map(|href| resolved_fragment_target(href))
        .collect();
    assert_eq!(
        resolved_targets,
        vec!["graph-classId-Foo Bar", "graph-classId-Foo Bar-1"]
    );

    // Terminal-only class relation label groups use the same resolved `data-id` as the edge path.
    let term_only = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  `Invoice#45;Item` \"1\" --> \"*\" Account\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        term_only.contains("data-id=\"id_Invoice-Item_Account_1\""),
        "{term_only}"
    );
    assert!(!term_only.contains("ﬂ°"), "{term_only}");

    // Class DOM IDs resolve Mermaid placeholders only once (no double decode to `©`), matching
    // their URL-encoded fragment links.
    let single_decode = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo#64258;°copy;`\n  click `Foo#64258;°copy;` href \"#graph-classId-Foo#64258;°copy;\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let single_doc = roxmltree::Document::parse(&single_decode).expect("well-formed SVG XML");
    assert!(
        single_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-Fooﬂ°copy;")),
        "{single_decode}"
    );
    let single_href = single_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(
        resolved_fragment_target(single_href),
        "graph-classId-Fooﬂ°copy;"
    );

    // Relation IDs with entity tabs normalize identically across path IDs, label data-ids, and
    // URL-encoded fragment links.
    let tab_rel = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  `Invoice#9;Item` --> Account : owns\n  click Account href \"#graph-id_Invoice#9;Item_Account_1\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let tab_rel_doc = roxmltree::Document::parse(&tab_rel).expect("well-formed SVG XML");
    assert!(
        tab_rel_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-id_Invoice Item_Account_1")),
        "{tab_rel}"
    );
    let tab_rel_href = tab_rel_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(tab_rel_href, "#graph-id_Invoice%20Item_Account_1");
    assert_eq!(
        resolved_fragment_target(tab_rel_href),
        "graph-id_Invoice Item_Account_1"
    );

    // Colliding resolved DOM IDs are disambiguated across all unified diagrams (e.g. requirement).
    let req_collide = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "requirementDiagram\n  requirement \"Order-Item\" {\n    id: 1\n  }\n  requirement \"Order#45;Item\" {\n    id: 2\n  }\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let req_doc = roxmltree::Document::parse(&req_collide).expect("well-formed SVG XML");
    let req_ids: Vec<&str> = req_doc
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .filter(|id| id.starts_with("graph-Order-Item"))
        .collect();
    assert_eq!(req_ids, vec!["graph-Order-Item", "graph-Order-Item-1"]);

    // Renderer-owned SVG IDs (e.g. `{svg_id}-drop-shadow`) are reserved before assigning diagram
    // element IDs so a requirement named `drop#45;shadow` cannot duplicate the drop-shadow filter.
    let shadow_collide = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "requirementDiagram\n  requirement \"drop#45;shadow\" {\n    id: 1\n  }\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let shadow_doc = roxmltree::Document::parse(&shadow_collide).expect("well-formed SVG XML");
    let shadow_filter_ids: Vec<&str> = shadow_doc
        .descendants()
        .filter(|node| node.has_tag_name("filter"))
        .filter_map(|node| node.attribute("id"))
        .collect();
    assert!(shadow_filter_ids.contains(&"graph-drop-shadow"));
    let shadow_node_ids: Vec<&str> = shadow_doc
        .descendants()
        .filter(|node| node.has_tag_name("g"))
        .filter_map(|node| node.attribute("id"))
        .filter(|id| id.starts_with("graph-drop-shadow"))
        .collect();
    assert_eq!(shadow_node_ids, vec!["graph-drop-shadow-1"]);
}

#[test]
fn unified_dom_ids_and_fragment_links_preserve_navigation_and_reserved_defs() {
    // Optional definitions (such as `{svg_id}-gradient`) are only reserved when the current render
    // actually emits them: default requirement diagrams emit no gradient, so a requirement named
    // `gradient` keeps `graph-gradient`, while `useGradient: true` disambiguates it.
    let no_grad = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "requirementDiagram\n  requirement \"gradient\" {\n    id: 1\n  }\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let no_grad_doc = roxmltree::Document::parse(&no_grad).expect("well-formed SVG XML");
    assert!(
        no_grad_doc
            .descendants()
            .any(|node| node.has_tag_name("g") && node.attribute("id") == Some("graph-gradient")),
        "{no_grad}"
    );
    let with_grad_cfg =
        Value::from(serde_json::json!({ "themeVariables": { "useGradient": true } }));
    let with_grad = ferrigura::Renderer::new(&with_grad_cfg)
        .render_with(
            "requirementDiagram\n  requirement \"gradient\" {\n    id: 1\n  }\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let with_grad_doc = roxmltree::Document::parse(&with_grad).expect("well-formed SVG XML");
    assert!(
        with_grad_doc.descendants().any(|node| {
            node.has_tag_name("linearGradient") && node.attribute("id") == Some("graph-gradient")
        }),
        "{with_grad}"
    );
    assert!(
        with_grad_doc
            .descendants()
            .any(|node| node.has_tag_name("g") && node.attribute("id") == Some("graph-gradient-1")),
        "{with_grad}"
    );

    // Distinct control-character entities (`#1;`, `#2;`) that both serialize as U+FFFD are
    // disambiguated after XML character replacement so IDs and fragment links remain unique.
    let ctrl_collide = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Part#1;A`\n  class `Part#2;A`\n  click `Part#1;A` href \"#graph-classId-Part#1;A\"\n  click `Part#2;A` href \"#graph-classId-Part#2;A\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let ctrl_doc = roxmltree::Document::parse(&ctrl_collide).expect("well-formed SVG XML");
    let ctrl_ids: Vec<&str> = ctrl_doc
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .filter(|id| id.starts_with("graph-classId-Part"))
        .collect();
    assert_eq!(
        ctrl_ids,
        vec![
            "graph-classId-Part\u{fffd}A",
            "graph-classId-Part\u{fffd}A-1"
        ]
    );
    let ctrl_targets: Vec<String> = ctrl_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .map(resolved_fragment_target)
        .collect();
    assert_eq!(
        ctrl_targets,
        vec![
            "graph-classId-Part\u{fffd}A",
            "graph-classId-Part\u{fffd}A-1"
        ]
    );

    // Remapped fragments percent-encode trailing spaces (`#32;`) so WHATWG URL parsing does not
    // strip the trailing space before resolving the target element ID.
    let trailing_space = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo#32;`\n  click `Foo#32;` href \"#graph-classId-Foo#32;\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let trailing_doc = roxmltree::Document::parse(&trailing_space).expect("well-formed SVG XML");
    assert!(
        trailing_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-Foo ")),
        "{trailing_space}"
    );
    let trailing_href = trailing_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(trailing_href, "#graph-classId-Foo%20");
    assert_eq!(
        resolved_fragment_target(trailing_href),
        "graph-classId-Foo "
    );

    // In `loose` mode, a literal tab in a backtick-quoted class name normalizes to a space in
    // the XML `id` attribute, so a matching click fragment link is remapped and URL-encoded while
    // unrelated links stay verbatim.
    let loose_config = Value::from(serde_json::json!({ "securityLevel": "loose" }));
    let literal_tab = ferrigura::Renderer::new(&loose_config)
        .render_with(
            "classDiagram\n  class `Tab\tClass`\n  class Other\n  click `Tab\tClass` href \"#graph-classId-Tab\tClass\"\n  click Other href \"https://example.com/\tkeep\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let literal_tab_doc = roxmltree::Document::parse(&literal_tab).expect("well-formed SVG XML");
    assert!(
        literal_tab_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-Tab Class")),
        "{literal_tab}"
    );
    let literal_tab_hrefs: Vec<&str> = literal_tab_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert_eq!(
        literal_tab_hrefs,
        vec!["#graph-classId-Tab%20Class", "https://example.com/\tkeep"]
    );
    assert_eq!(
        resolved_fragment_target(literal_tab_hrefs[0]),
        "graph-classId-Tab Class"
    );

    // Accessibility text uses HTML text-context entity matching (legacy `&not` prefix inside
    // `#notebook;` -> `¬ebook;`), while URLs and tooltips keep the stricter attribute rule.
    let notebook_strict = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  accTitle: #notebook;\n  class A\n  click A href \"https://example.com/?x=#notebook;\" \"#notebook;\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let notebook_doc = roxmltree::Document::parse(&notebook_strict).expect("well-formed SVG XML");
    let notebook_title = notebook_doc
        .descendants()
        .find(|node| node.has_tag_name("title"))
        .and_then(|node| node.text())
        .expect("title text");
    assert_eq!(notebook_title, "¬ebook;");
    let notebook_tooltip = notebook_doc
        .descendants()
        .find(|node| node.has_tag_name("g") && node.attribute("id") == Some("graph-classId-A"))
        .and_then(|node| node.attribute("title"))
        .expect("tooltip");
    assert_eq!(notebook_tooltip, "&notebook;");

    let notebook_loose = ferrigura::Renderer::new(&loose_config)
        .render_with(
            "classDiagram\n  class A\n  click A href \"https://example.com/?x=#notebook;\" \"#notebook;\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let notebook_loose_doc =
        roxmltree::Document::parse(&notebook_loose).expect("well-formed SVG XML");
    let notebook_href = notebook_loose_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(notebook_href, "https://example.com/?x=&notebook;");

    // Declaration order must not let a decoded ID steal an existing literal ID: `Foo#45;Bar`
    // declared before `Foo-Bar` gets the disambiguated suffix while literal `Foo-Bar` keeps its ID.
    let order_preserve = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo#45;Bar`\n  class `Foo-Bar`\n  click `Foo#45;Bar` href \"#graph-classId-Foo#45;Bar\"\n  click `Foo-Bar` href \"#graph-classId-Foo-Bar\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let order_doc = roxmltree::Document::parse(&order_preserve).expect("well-formed SVG XML");
    let order_ids: Vec<&str> = order_doc
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .filter(|id| id.starts_with("graph-classId-Foo-Bar"))
        .collect();
    assert_eq!(
        order_ids,
        vec!["graph-classId-Foo-Bar-1", "graph-classId-Foo-Bar"]
    );
    let order_targets: Vec<String> = order_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .map(resolved_fragment_target)
        .collect();
    assert_eq!(
        order_targets,
        vec!["graph-classId-Foo-Bar-1", "graph-classId-Foo-Bar"]
    );

    // Emitted accessibility `<title>` / `<desc>` IDs (`chart-title-{id}`) are reserved when
    // present, and left free when no accessibility element is emitted.
    let a11y_collide = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "requirementDiagram\n  accTitle: Main\n  requirement \"title#45;chart\" {\n    id: 1\n  }\n",
            &ferrigura::RenderOptions {
                id: Some("chart".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let a11y_doc = roxmltree::Document::parse(&a11y_collide).expect("well-formed SVG XML");
    assert!(
        a11y_doc
            .descendants()
            .any(|node| node.has_tag_name("title")
                && node.attribute("id") == Some("chart-title-chart")),
        "{a11y_collide}"
    );
    assert!(
        a11y_doc
            .descendants()
            .any(|node| node.has_tag_name("g")
                && node.attribute("id") == Some("chart-title-chart-1")),
        "{a11y_collide}"
    );
    let a11y_absent = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "requirementDiagram\n  requirement \"title#45;chart\" {\n    id: 1\n  }\n",
            &ferrigura::RenderOptions {
                id: Some("chart".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let a11y_absent_doc = roxmltree::Document::parse(&a11y_absent).expect("well-formed SVG XML");
    assert!(
        a11y_absent_doc
            .descendants()
            .any(|node| node.has_tag_name("g") && node.attribute("id") == Some("chart-title-chart")),
        "{a11y_absent}"
    );

    // Percent-encoded fragment spellings (e.g. `#graph-classId-New%20Item#32;` in loose mode)
    // still match the remapped class ID and percent-encode the trailing space.
    let pct_frag = ferrigura::Renderer::new(&loose_config)
        .render_with(
            "classDiagram\n  class `New Item#32;`\n  click `New Item#32;` href \"#graph-classId-New%20Item#32;\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let pct_doc = roxmltree::Document::parse(&pct_frag).expect("well-formed SVG XML");
    assert!(
        pct_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-New Item ")),
        "{pct_frag}"
    );
    let pct_href = pct_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(pct_href, "#graph-classId-New%20Item%20");
    assert_eq!(
        resolved_fragment_target(pct_href),
        "graph-classId-New Item "
    );
}

#[test]
fn url_entity_placeholders_and_fragment_precedence_match_reference() {
    let loose_config = Value::from(serde_json::json!({ "securityLevel": "loose" }));

    // Normalized HTTP/HTTPS links with Mermaid entity placeholders (such as `#32;`) resolve the
    // percent-encoded internal placeholder at the SVG sink without decoding unrelated `%XX`
    // escapes such as `%26` (produced here after `sanitizeUrl`'s two decode passes on `%252526`).
    let http_entity = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class A\n  click A href \"https://example.org/Chapter#32;One?q=A%252526B\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let http_doc = roxmltree::Document::parse(&http_entity).expect("well-formed SVG XML");
    let http_href = http_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(http_href, "https://example.org/Chapter%20One?q=A%26B");

    // Hostname entities (such as `#46;` for `.`) resolve before WHATWG/IDNA host normalization
    // instead of turning into Punycode (`xn--...`), including URLs with explicit ports, while
    // invalid hostname entities fail closed.
    let host_entity = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class A\n  class B\n  class C\n  click A href \"https://example#46;org/Chapter#32;One\"\n  click B href \"https://example#32;org/docs\"\n  click C href \"https://example#46;org:8080/path\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let host_doc = roxmltree::Document::parse(&host_entity).expect("well-formed SVG XML");
    let host_hrefs: Vec<&str> = host_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert_eq!(
        host_hrefs,
        vec![
            "https://example.org/Chapter%20One",
            "about:blank",
            "https://example.org:8080/path",
        ]
    );

    // Distinct raw node IDs after XML normalization (`Foo Bar` vs literal-tab `Foo\tBar` in loose
    // mode) remap `#graph-classId-Foo%09Bar` to the disambiguated `Foo Bar-1` class.
    let tab_loose_collide = ferrigura::Renderer::new(&loose_config)
        .render_with(
            "classDiagram\n  class `Foo Bar`\n  class `Foo\tBar`\n  click `Foo\tBar` href \"#graph-classId-Foo%09Bar\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let tab_loose_doc =
        roxmltree::Document::parse(&tab_loose_collide).expect("well-formed SVG XML");
    let tab_loose_href = tab_loose_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(tab_loose_href, "#graph-classId-Foo%20Bar-1");
    assert_eq!(
        resolved_fragment_target(tab_loose_href),
        "graph-classId-Foo Bar-1"
    );

    // Entity-encoded fragment markers (`#35;`) still match remapped disambiguated class IDs.
    let encoded_hash = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo-Bar`\n  class `Foo#45;Bar`\n  click `Foo#45;Bar` href \"#35;graph-classId-Foo#45;Bar\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let encoded_hash_doc = roxmltree::Document::parse(&encoded_hash).expect("well-formed SVG XML");
    let encoded_hash_href = encoded_hash_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(encoded_hash_href, "#graph-classId-Foo-Bar-1");

    // Resolved IDs containing closing Mermaid entity tokens (`Foo#182;ß` -> `Foo¶ß`) are not
    // decoded a second time to `;`.
    let close_token = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo#182;ß`\n  click `Foo#182;ß` href \"#graph-classId-Foo#182;ß\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let close_doc = roxmltree::Document::parse(&close_token).expect("well-formed SVG XML");
    assert!(
        close_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-Foo¶ß")),
        "{close_token}"
    );
    let close_href = close_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(resolved_fragment_target(close_href), "graph-classId-Foo¶ß");

    // Hostname entities are resolved once (at `format_url`), so text that only resembles a
    // placeholder after the first pass (`#64258;°period#182;ß`) is not decoded a second time.
    let single_host = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class A\n  click A href \"https://foo#64258;°period#182;ßbar.example/\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let single_host_doc = roxmltree::Document::parse(&single_host).expect("well-formed SVG XML");
    let single_host_href = single_host_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(
        single_host_href,
        "https://xn--fooflperiodbar-zua6hz6a.example/"
    );

    // Sequence actor menu links also decode percent-encoded entity placeholders in HTTP URLs
    // without double-decoding literal placeholders.
    let seq_link = render(
        "sequenceDiagram\n  participant A\n  link A: Help @ https://example.org/Chapter#32;One\n  A->>A: ping\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let seq_link_doc = roxmltree::Document::parse(&seq_link).expect("well-formed SVG XML");
    let seq_href = seq_link_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(seq_href, "https://example.org/Chapter%20One");

    // Finalized remapped fragments whose target ID contains literal `ﬂ°copy¶ß` keep their
    // percent-encoded bytes intact instead of rewriting `%EF%AC%82%C2%B0copy%C2%B6%C3%9F` to `%C2%A9`.
    let lit_placeholder_frag = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Id#64258;°copy#182;ß`\n  click `Id#64258;°copy#182;ß` href \"#graph-classId-Id#64258;°copy#182;ß\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let lit_doc = roxmltree::Document::parse(&lit_placeholder_frag).expect("well-formed SVG XML");
    assert!(
        lit_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-Idﬂ°copy¶ß")),
        "{lit_placeholder_frag}"
    );
    let lit_href = lit_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(lit_href, "#graph-classId-Id%EF%AC%82%C2%B0copy%C2%B6%C3%9F");
    assert_eq!(
        resolved_fragment_target(lit_href),
        "graph-classId-Idﬂ°copy¶ß"
    );

    // When an unchanged class `A B-C` coexists with `A%20B#45;C` in loose mode, clicking
    // `#graph-classId-A%20B#45;C` prefers the decoded existing target `A B-C` over falling back
    // to the raw `%20` remap of `A%20B#45;C`.
    let prefer_existing = ferrigura::Renderer::new(&loose_config)
        .render_with(
            "classDiagram\n  class `A B-C`\n  class `A%20B#45;C`\n  click `A B-C` href \"#graph-classId-A%20B#45;C\"\n  click `A%20B#45;C` href \"#graph-classId-A%2520B#45;C\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let prefer_doc = roxmltree::Document::parse(&prefer_existing).expect("well-formed SVG XML");
    let prefer_hrefs: Vec<&str> = prefer_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert_eq!(
        prefer_hrefs,
        vec!["#graph-classId-A%20B-C", "#graph-classId-A%2520B-C"]
    );
    assert_eq!(
        resolved_fragment_target(prefer_hrefs[0]),
        "graph-classId-A B-C"
    );
    assert_eq!(
        resolved_fragment_target(prefer_hrefs[1]),
        "graph-classId-A%20B-C"
    );

    // Sequence relative menu links decode entities in attribute context (`/docs/#notebook;` ->
    // `/docs/&notebook;`, not text-context `/docs/¬ebook;`) and avoid double decoding.
    let seq_rel = render(
        "sequenceDiagram\n  participant A\n  link A: Docs @ /docs/#notebook;\n  link A: Lit @ /docs/#64258;°copy#182;ß\n  A->>A: ping\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let seq_rel_doc = roxmltree::Document::parse(&seq_rel).expect("well-formed SVG XML");
    let seq_rel_hrefs: Vec<&str> = seq_rel_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert_eq!(seq_rel_hrefs, vec!["/docs/&notebook;", "/docs/ﬂ°copy¶ß"]);

    // Gantt task links also resolve percent-encoded HTTP entity placeholders (`#32;` -> `%20`).
    let gantt_link = render(
        "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  Task :t1, 2024-01-01, 2d\n  click t1 href \"https://example.org/A#32;B\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let gantt_doc = roxmltree::Document::parse(&gantt_link).expect("well-formed SVG XML");
    let gantt_href = gantt_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(gantt_href, "https://example.org/A%20B");

    // When a subgraph `foo` keeps `graph-foo` and a colliding edge `foo@-->` is disambiguated to
    // `graph-foo-1`, fragment links to `#graph-foo` remain on the subgraph (with or without an
    // unrelated `drop-shadow` edge).
    for extra_edge in ["", "\n  A drop-shadow@--> B"] {
        let source = format!(
            "flowchart LR\n  subgraph foo\n    A\n  end\n  A foo@--> B{extra_edge}\n  click B \"#graph-foo\"\n"
        );
        let sub_svg = ferrigura::Renderer::new(&Value::empty_object())
            .render_with(
                &source,
                &ferrigura::RenderOptions {
                    id: Some("graph".to_owned()),
                    ..ferrigura::RenderOptions::default()
                },
            )
            .expect("renders");
        let sub_doc = roxmltree::Document::parse(&sub_svg).expect("well-formed SVG XML");
        assert!(
            sub_doc
                .descendants()
                .any(|node| node.has_tag_name("g") && node.attribute("id") == Some("graph-foo")),
            "{sub_svg}"
        );
        assert!(
            sub_doc.descendants().any(
                |node| node.has_tag_name("path") && node.attribute("id") == Some("graph-foo-1")
            ),
            "{sub_svg}"
        );
        let sub_href = sub_doc
            .descendants()
            .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
            .expect("href");
        assert_eq!(sub_href, "#graph-foo");
    }

    // When both `A%20B#45;C` and `A B#45;C` exist in loose mode, `#graph-classId-A%20B#45;C`
    // resolves to `A B-C` (decoded target first), while `%2520` targets `A%20B-C`.
    let disambig_pct = ferrigura::Renderer::new(&loose_config)
        .render_with(
            "classDiagram\n  class `A%20B#45;C`\n  class `A B#45;C`\n  click `A B#45;C` href \"#graph-classId-A%20B#45;C\"\n  click `A%20B#45;C` href \"#graph-classId-A%2520B#45;C\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let disambig_doc = roxmltree::Document::parse(&disambig_pct).expect("well-formed SVG XML");
    let disambig_hrefs: Vec<&str> = disambig_doc
        .descendants()
        .filter_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert_eq!(
        disambig_hrefs,
        vec!["#graph-classId-A%2520B-C", "#graph-classId-A%20B-C"]
    );
    assert_eq!(
        resolved_fragment_target(disambig_hrefs[0]),
        "graph-classId-A%20B-C"
    );
    assert_eq!(
        resolved_fragment_target(disambig_hrefs[1]),
        "graph-classId-A B-C"
    );

    // Enabling animation never renames unrelated `draw-mask-*` edge IDs and allocates animation
    // masks around occupied IDs when an edge actually collides with the first mask (`draw-mask-1`
    // for `-->`, where serial 0 is consumed by the arrow marker).
    let anim_999 = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "flowchart LR\n  A draw-mask-999@--> B\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                animation: Some(ferrigura::Animation::default()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let anim_999_doc = roxmltree::Document::parse(&anim_999).expect("well-formed SVG XML");
    assert!(
        anim_999_doc
            .descendants()
            .any(|node| node.has_tag_name("path")
                && node.attribute("id") == Some("graph-draw-mask-999")),
        "{anim_999}"
    );
    let anim_1 = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "flowchart LR\n  A draw-mask-1@--> B\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                animation: Some(ferrigura::Animation::default()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let anim_1_doc = roxmltree::Document::parse(&anim_1).expect("well-formed SVG XML");
    assert!(
        anim_1_doc
            .descendants()
            .any(|node| node.has_tag_name("path")
                && node.attribute("id") == Some("graph-draw-mask-1")),
        "{anim_1}"
    );
    let mask_ids: Vec<&str> = anim_1_doc
        .descendants()
        .filter(|node| node.has_tag_name("mask"))
        .filter_map(|node| node.attribute("id"))
        .collect();
    assert_eq!(mask_ids, vec!["graph-draw-mask-2"]);
    let mut all_ids = std::collections::HashSet::new();
    for id in anim_1_doc
        .descendants()
        .filter_map(|node| node.attribute("id"))
    {
        assert!(all_ids.insert(id), "duplicate SVG id {id:?} in {anim_1}");
    }
}

#[test]
fn gantt_sequence_url_guards_and_protected_dollar_ids() {
    let loose_cfg = Value::from(serde_json::json!({ "securityLevel": "loose" }));

    // Gantt task links validate schemes after entity placeholder resolution.
    let gantt_src = "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  Task :t1, 2024-01-01, 2d\n  click t1 href \"javascript#58;alert(1)\"\n";
    let gantt_strict = ferrigura::render(gantt_src, &Value::empty_object()).expect("renders");
    assert!(
        gantt_strict.contains("xlink:href=\"about:blank\""),
        "{gantt_strict}"
    );
    assert!(
        !gantt_strict.contains("javascript:alert(1)"),
        "{gantt_strict}"
    );
    let gantt_loose = ferrigura::render(gantt_src, &loose_cfg).expect("renders");
    assert!(
        gantt_loose.contains("xlink:href=\"javascript:alert(1)\""),
        "{gantt_loose}"
    );

    // Sequence actor links and icons guard schemes after entity resolution and preserve XML
    // whitespace references (such as tab -> &#9;).
    let seq_src = "sequenceDiagram\n  participant A\n  link A: Tab @ /One#9;Two\n  link A: Bad @ javascript#58;alert(1)\n  properties A: {\"icon\": \"javascript#58;alert(1)\"}\n  A->>A: hi\n";
    let seq_strict = ferrigura::render(seq_src, &Value::empty_object()).expect("renders");
    assert!(
        seq_strict.contains("xlink:href=\"/One&#9;Two\""),
        "{seq_strict}"
    );
    assert!(!seq_strict.contains("javascript:alert(1)"), "{seq_strict}");
    let seq_doc = roxmltree::Document::parse(&seq_strict).expect("well-formed SVG XML");
    let seq_hrefs: Vec<&str> = seq_doc
        .descendants()
        .filter(|n| n.has_tag_name("a") || n.has_tag_name("image"))
        .filter_map(|n| n.attribute(("http://www.w3.org/1999/xlink", "href")))
        .collect();
    assert!(seq_hrefs.contains(&"/One\tTwo"), "{seq_hrefs:?}");
    assert!(seq_hrefs.contains(&"about:blank"), "{seq_hrefs:?}");
    let seq_loose = ferrigura::render(seq_src, &loose_cfg).expect("renders");
    assert!(
        seq_loose.contains("xlink:href=\"javascript:alert(1)\""),
        "{seq_loose}"
    );

    // Protected class IDs containing `$$` survive intact (no JS replacement-pattern corruption).
    let dollar = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "classDiagram\n  class `Foo#64258;°copy$$#182;ß`\n  click `Foo#64258;°copy$$#182;ß` href \"#graph-classId-Foo#64258;°copy$$#182;ß\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    let dollar_doc = roxmltree::Document::parse(&dollar).expect("well-formed SVG XML");
    assert!(
        dollar_doc
            .descendants()
            .any(|node| node.attribute("id") == Some("graph-classId-Fooﬂ°copy$$¶ß")),
        "{dollar}"
    );
    let dollar_href = dollar_doc
        .descendants()
        .find_map(|node| node.attribute(("http://www.w3.org/1999/xlink", "href")))
        .expect("href");
    assert_eq!(
        resolved_fragment_target(dollar_href),
        "graph-classId-Fooﬂ°copy$$¶ß"
    );
}
