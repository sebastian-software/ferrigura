//! State diagrams through the public entry point: the root element, the markers of the look and the
//! title, as `stateRenderer-v3-unified.ts` writes them.

use ferrigura::render;
use ferrigura_model::value::Value;

/// The root element's start tag, up to its closing `>`.
fn root_tag(svg: &str) -> &str {
    let end = svg.find('>').expect("the root element closes");
    &svg[..end]
}

#[test]
fn a_state_diagram_is_a_statediagram_root_with_its_barb_markers() {
    let svg = render(
        "stateDiagram-v2\n  [*] --> A\n  A --> B\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let root = root_tag(&svg);
    assert!(root.contains("class=\"statediagram\""), "{root}");
    assert!(
        root.contains("aria-roledescription=\"stateDiagram\""),
        "{root}"
    );
    // The neo look is the default: the margin variant is the one the edges point at.
    assert!(svg.contains("_stateDiagram-barbEnd-margin\""), "{svg}");
    assert!(svg.contains("marker-end=\"url(#"), "{svg}");
    assert!(svg.contains("_stateDiagram-barbEnd-margin)\""), "{svg}");
}

#[test]
fn a_classic_state_diagram_points_its_edges_at_the_plain_barb() {
    let text = "%%{init: {\"look\":\"classic\"}}%%\nstateDiagram\n  A --> B\n";
    let svg = render(text, &Value::empty_object()).expect("renders");
    assert!(svg.contains("_stateDiagram-barbEnd\""), "{svg}");
    assert!(!svg.contains("-barbEnd-margin\""), "{svg}");
}

#[test]
fn a_state_diagram_title_is_written_at_the_title_margin() {
    let text = "---\ntitle: Machine\n---\nstateDiagram-v2\n  A --> B\n";
    let svg = render(text, &Value::empty_object()).expect("renders");
    assert!(svg.contains("class=\"statediagramTitleText\""), "{svg}");
    assert!(svg.contains(">Machine</text>"), "{svg}");
}

/// The `<g>` of the node whose id ends with `suffix`, up to the next node.
fn node<'a>(svg: &'a str, suffix: &str) -> &'a str {
    let start = svg
        .find(&format!("{suffix}\" data-look"))
        .expect("the node is drawn");
    let rest = &svg[start..];
    let end = rest[1..]
        .find("<g class=\"node")
        .map_or(rest.len(), |end| end + 1);
    &rest[..end]
}

#[test]
fn the_rough_state_shapes_carry_their_colors_as_attributes() {
    // The state style sheet has no rule for a node's paths, so the reference's presentation
    // attributes color them: without them, a note, a choice and an end state fill black.
    let text = "stateDiagram-v2
  state paid <<choice>>
  [*] --> Cart
  Cart --> paid
  paid --> [*]
  note right of Cart: Text
";
    let svg = render(text, &Value::empty_object()).expect("renders");
    // The note: `noteBkgColor` and `noteBorderColor` (`note.ts`).
    let note = node(&svg, "Cart----note-3");
    assert!(note.contains("fill=\"#fff5ad\""), "{note}");
    assert!(note.contains("stroke=\"#FACC15\""), "{note}");
    // The choice: `mainBkg` and `nodeBorder`, a hollow diamond (`choice.ts`).
    let choice = node(&svg, "state-paid-2");
    assert!(choice.contains("fill=\"#ffffff\""), "{choice}");
    assert!(choice.contains("stroke=\"#28253D\""), "{choice}");
    // The end state: a ring of `mainBkg` around a dot of `stateBorder` (`stateEnd.ts`).
    let end = node(&svg, "root_end-2");
    assert!(end.contains("fill=\"#ffffff\""), "{end}");
    assert!(end.contains("fill=\"#28253D\""), "{end}");
}

#[test]
fn the_nested_text_rule_of_a_state_note_is_written_flat() {
    let svg = render("stateDiagram-v2\n  A --> B\n", &Value::empty_object()).expect("renders");
    assert!(svg.contains(" .state-note text{"), "{svg}");
}

#[test]
fn state_click_urls_are_sanitized_under_default_security_level() {
    for unsafe_url in [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,<script>alert(1)</script>",
        "javascript&colon;alert(1)",
        "%6aavascript:alert(1)",
        "java\tscript:alert(1)",
        "java\\tscript:alert(1)",
    ] {
        let href_input = format!("stateDiagram-v2\n  s1\n  click s1 href \"{unsafe_url}\"\n");
        let href_svg = render(&href_input, &Value::empty_object()).expect("renders");
        assert!(
            href_svg.contains("xlink:href=\"about:blank\""),
            "href url={unsafe_url:?}: {href_svg}"
        );
        assert!(
            !href_svg.contains("javascript:") && !href_svg.contains("vbscript:"),
            "href url={unsafe_url:?}: {href_svg}"
        );

        let tooltip_input =
            format!("stateDiagram-v2\n  s1\n  click s1 \"{unsafe_url}\" \"Open\"\n");
        let tooltip_svg = render(&tooltip_input, &Value::empty_object()).expect("renders");
        assert!(
            tooltip_svg.contains("xlink:href=\"about:blank\""),
            "tooltip url={unsafe_url:?}: {tooltip_svg}"
        );
    }

    let valid = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"https://google.com\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        valid.contains("xlink:href=\"https://google.com\""),
        "{valid}"
    );

    let encoded = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"https://example.com/A%23B\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded.contains("xlink:href=\"https://example.com/A%23B\""),
        "{encoded}"
    );

    let relative_colon = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"data/chart.svg?time=12:30\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        relative_colon.contains("xlink:href=\"data/chart.svg?time=12:30\""),
        "{relative_colon}"
    );

    for unsupported in [
        "notes://do-your-thing/id",
        "notes%25253Adoc",
        "notes%3Afoo%FF",
        "notes#58;meeting",
        "javascript#58;alert(1)",
    ] {
        let unsupported_scheme = render(
            &format!("stateDiagram-v2\n  s1\n  click s1 href \"{unsupported}\"\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            unsupported_scheme.contains("xlink:href=\"about:blank\""),
            "url={unsupported:?}: {unsupported_scheme}"
        );
    }

    let entity_https = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"https#58;//example.org/\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        entity_https.contains("xlink:href=\"https://example.org/\""),
        "{entity_https}"
    );

    let named_colon_https = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"https&colon;//example.org\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        named_colon_https.contains("xlink:href=\"https&amp;colon;//example.org\""),
        "{named_colon_https}"
    );

    // Placeholders are resolved only once at the SVG sink, not re-interpreted by a second pass.
    let single_pass = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"/docs/#64258;°copy;\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        single_pass.contains("xlink:href=\"/docs/ﬂ°copy;\""),
        "{single_pass}"
    );
    assert!(
        !single_pass.contains("xlink:href=\"/docs/©\""),
        "{single_pass}"
    );

    // HTML attribute-value entity rules keep legacy prefixes like `&not` literal when followed by
    // ASCII alphanumerics.
    let attr_entity = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"/docs/#notebook_1;\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        attr_entity.contains("xlink:href=\"/docs/&amp;notebook_1;\""),
        "{attr_entity}"
    );
    assert!(!attr_entity.contains("¬ebook_1;"), "{attr_entity}");

    // Decoded tab/newline/CR characters in URLs are serialized as numeric XML references so XML
    // attribute normalization preserves them rather than turning them into spaces.
    let tab_svg = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"https://example.org/a#9;b\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    let tab_doc = roxmltree::Document::parse(&tab_svg).expect("well-formed SVG XML");
    let anchor = tab_doc
        .descendants()
        .find(|n| n.has_tag_name("a"))
        .expect("anchor");
    let href = anchor
        .attribute(("http://www.w3.org/1999/xlink", "href"))
        .expect("xlink:href");
    assert_eq!(href, "https://example.org/a\tb");

    // Encoded delimiters revealed during sink decoding stop decoding before the query payload.
    let encoded_query = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"item%3Fq=A%2525252525252525252541\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded_query.contains("xlink:href=\"item%3Fq=A%2525252525252525252541\""),
        "{encoded_query}"
    );

    let opaque_cid = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"cid:asset%2525252525252525252541@example.com\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        opaque_cid.contains("xlink:href=\"cid:asset%2525252525252525252541@example.com\""),
        "{opaque_cid}"
    );

    for (raw, escaped) in [
        ("R&#section:1", "R&amp;#section:1"),
        (
            "R&D#A%2525252525252525252541",
            "R&amp;D#A%2525252525252525252541",
        ),
        (
            "my%20notes#A%2525252525252525252541",
            "my%20notes#A%2525252525252525252541",
        ),
    ] {
        let fragment = render(
            &format!("stateDiagram-v2\n  s1\n  click s1 href \"{raw}\"\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            fragment.contains(&format!("xlink:href=\"{escaped}\"")),
            "raw={raw:?}: {fragment}"
        );
    }

    let loose_cfg = Value::from(serde_json::json!({ "securityLevel": "loose" }));
    let loose = render(
        "stateDiagram-v2\n  s1\n  click s1 href \"javascript:alert(1)\"\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose.contains("xlink:href=\"javascript:alert(1)\""),
        "{loose}"
    );

    // An empty state link still declares xmlns:xlink whenever an anchor is emitted.
    for config in [Value::empty_object(), loose_cfg] {
        let empty =
            render("stateDiagram-v2\n  s1\n  click s1 href \"\"\n", &config).expect("renders");
        assert!(empty.contains("xmlns:xlink="), "{empty}");
        roxmltree::Document::parse(&empty).expect("well-formed SVG XML");
    }
}
