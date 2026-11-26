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
}
