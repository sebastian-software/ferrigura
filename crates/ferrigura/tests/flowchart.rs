//! Flowcharts through the public entry point: what the root element's box is, the label
//! backgrounds of `compat`, and the security of links and images.

use ferrigura::render;
use ferrigura::{RenderOptions, Renderer};
use ferrigura_model::value::Value;

/// The tolerance of the comparison with the reference, in SVG user units.
const TOLERANCE: f64 = 0.05;

const REVIEW: &str = "flowchart TD\n draft[Draft] --> review{Review}\n review -->|Approved| publish[Publish]\n review -->|Changes| draft\n";

fn options(compat: bool) -> RenderOptions {
    RenderOptions {
        id: Some("routing".to_owned()),
        compat,
        ..RenderOptions::default()
    }
}

#[test]
fn compat_draws_edge_label_backgrounds_at_full_opacity_in_every_theme() {
    let text = "flowchart LR\n A -->|Readable label| B\n";
    let opaque = "#routing .edgeLabel rect{opacity:1;";
    for theme in [
        "redux-color",
        "redux-dark-color",
        "redux",
        "redux-dark",
        "neo",
        "neo-dark",
    ] {
        let renderer = Renderer::new(&Value::from(serde_json::json!({ "theme": theme })));
        let compatible = renderer.render_with(text, &options(true)).unwrap();
        assert!(compatible.contains(opaque), "{theme}");
    }
}

#[test]
fn full_opacity_keeps_explicit_theme_background_colors() {
    for color in ["transparent", "rgba(25,50,75,0.25)", "#abcdef"] {
        let renderer = Renderer::new(&Value::from(serde_json::json!({
            "themeVariables": { "edgeLabelBackground": color }
        })));
        let svg = renderer
            .render_with("flowchart LR\n A -->|Label| B\n", &options(true))
            .unwrap();
        assert!(
            svg.contains(&format!("--edgeLabelBackground:{color};")),
            "{color}"
        );
        assert!(svg.contains("#routing .edgeLabel rect{opacity:1;"));
    }
}

#[test]
fn compat_is_per_render() {
    let renderer = Renderer::new(&Value::empty_object());
    let first = renderer.render_with(REVIEW, &options(true)).unwrap();
    let drawn = renderer.render_with(REVIEW, &options(false)).unwrap();
    assert_ne!(first, drawn);
    assert_eq!(first, renderer.render_with(REVIEW, &options(true)).unwrap());
    assert_eq!(
        drawn,
        renderer.render_with(REVIEW, &options(false)).unwrap()
    );
}

/// The `viewBox` of the root element, as its four numbers.
fn view_box(svg: &str) -> [f64; 4] {
    let start = svg.find("viewBox=\"").expect("a viewBox") + "viewBox=\"".len();
    let end = svg[start..].find('"').expect("the viewBox closes") + start;
    let numbers: Vec<f64> = svg[start..end]
        .split_whitespace()
        .map(|number| number.parse().expect("a number"))
        .collect();
    [numbers[0], numbers[1], numbers[2], numbers[3]]
}

/// The subgraph `B` has no node of its own, since `b` belongs to `A`, so its frame has height zero
/// and is not rendered. `getBBox` leaves it out of the drawing's box, so the viewBox starts at the
/// title of `B`, one pixel below the frame's top. The reference gives `viewBox="4 5.0000009 392 164"`
/// (`visual-check/57-handle-nested-subgraphs-with-outgoing-links-4`).
#[test]
fn an_empty_subgraphs_frame_does_not_move_the_view_box() {
    let text = r#"%%{init: {"htmlLabels":false,"fontFamily":"\"Lexica Ultralegible\"","securityLevel":"strict"}}%%
flowchart LR
  subgraph A
  a -->b
  end
  subgraph B
  b
  end
"#;
    let svg = Renderer::new(&Value::empty_object())
        .render_with(text, &options(true))
        .expect("the flowchart renders");
    let [x, y, width, height] = view_box(&svg);
    assert!((x - 4.0).abs() < TOLERANCE, "x is {x}");
    assert!((y - 5.0).abs() < TOLERANCE, "y is {y}, the reference has 5");
    assert!((width - 392.0).abs() < TOLERANCE, "width is {width}");
    assert!(
        (height - 164.0).abs() < TOLERANCE,
        "height is {height}, the reference has 164"
    );
}

#[test]
fn flowchart_click_and_image_urls_are_sanitized_under_default_security_level() {
    for unsafe_url in [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,<script>alert(1)</script>",
        "notes://do-your-thing/id",
        "notes#58;meeting",
        "a:b",
        "javascript%3Aalert(1)",
        "javascript&colon;alert(1)",
        "%6aavascript:alert(1)",
        "java\tscript:alert(1)",
        "java\\tscript:alert(1)",
    ] {
        let click_input = format!("flowchart TD\nA[x]\nclick A \"{unsafe_url}\"\n");
        let svg = render(&click_input, &Value::empty_object()).expect("renders");
        assert!(
            svg.contains("xlink:href=\"about:blank\""),
            "click url={unsafe_url:?}: {svg}"
        );
        assert!(
            !svg.contains("javascript:") && !svg.contains("vbscript:"),
            "click url={unsafe_url:?}: {svg}"
        );

        let image_input = format!("flowchart TD\nA@{{ img: \"{unsafe_url}\", w: 48, h: 48 }}\n");
        let img_svg = render(&image_input, &Value::empty_object()).expect("renders");
        assert!(
            img_svg.contains("<image href=\"about:blank\""),
            "img url={unsafe_url:?}: {img_svg}"
        );
        assert!(
            !img_svg.contains("javascript:") && !img_svg.contains("vbscript:"),
            "img url={unsafe_url:?}: {img_svg}"
        );
    }

    // Valid https URLs continue to work, without double-decoding click URLs or decoding
    // percent-encoded delimiters in image URLs.
    let valid_click = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/docs\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        valid_click.contains("xlink:href=\"https://example.com/docs\""),
        "{valid_click}"
    );
    let encoded_click = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/files?q=A%26B\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded_click.contains("xlink:href=\"https://example.com/files?q=A%26B\""),
        "{encoded_click}"
    );
    let relative_encoded_click = render(
        "flowchart TD\nA[x]\nclick A \"/docs/a%2Fb?q=A%26B\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        relative_encoded_click.contains("xlink:href=\"/docs/a%2Fb?q=A%26B\""),
        "{relative_encoded_click}"
    );
    let relative_colon_click = render(
        "flowchart TD\nA[x]\nclick A \"data/chart.svg?time=12:30\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        relative_colon_click.contains("xlink:href=\"data/chart.svg?time=12:30\""),
        "{relative_colon_click}"
    );
    let encoded_backslash_click = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/backups%5Cnew.zip\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded_backslash_click.contains("xlink:href=\"https://example.com/backups%5Cnew.zip\""),
        "{encoded_backslash_click}"
    );
    let relative_fragment_colon_click = render(
        "flowchart TD\nA[x]\nclick A \"data#part:two\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        relative_fragment_colon_click.contains("xlink:href=\"data#part:two\""),
        "{relative_fragment_colon_click}"
    );
    let literal_fragment_target = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "flowchart TD\nsubgraph Foo%41\nA\nend\nsubgraph FooA\nB\nend\nclick B \"#graph-Foo%41\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        literal_fragment_target.contains("xlink:href=\"#graph-Foo%41\""),
        "{literal_fragment_target}"
    );
    // Numeric character references in one URL component decode as Unicode without discarding
    // percent-encoded delimiters in unaffected components.
    for (raw, expected) in [
        (
            "https://example.org/a%2Fb?note=&#169;",
            "https://example.org/a%2Fb?note=%C2%A9",
        ),
        (
            "https://example.org/a&#xA9;b",
            "https://example.org/a%C2%A9b",
        ),
        (
            "https://example.org/a&#128512;b",
            "https://example.org/a%F0%9F%98%80b",
        ),
    ] {
        let svg = render(
            &format!("flowchart TD\nA[x]\nclick A \"{raw}\"\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            svg.contains(&format!("xlink:href=\"{expected}\"")),
            "raw={raw}: {svg}"
        );
    }
    // Multiline `accDescr` blocks whose opening `{` is on the next line do not leak unmatched
    // quotes into subsequent `style` lines.
    let next_line_acc_descr = render(
        "flowchart LR\n  accDescr\n  {\n    The \"metric\n  }\n  A\n  style A fill:#abc;\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        next_line_acc_descr.contains("fill:#abc"),
        "{next_line_acc_descr}"
    );
    assert!(
        !next_line_acc_descr.contains("&amp;abc;"),
        "{next_line_acc_descr}"
    );
    // `accDescr {` after a `;` statement separator also isolates unmatched quotes.
    let semi_acc_descr = render(
        "flowchart LR; accDescr {\n  The \"metric\n}\nA\nstyle A fill:#abc;\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(semi_acc_descr.contains("fill:#abc"), "{semi_acc_descr}");
    assert!(!semi_acc_descr.contains("&amp;abc;"), "{semi_acc_descr}");
    // Escaped HTTPS userinfo (`%23alice@`) stays valid instead of being blanked when `%23` is
    // decoded.
    let escaped_userinfo = render(
        "flowchart TD\nA[x]\nclick A \"https://%23alice@example.org/report\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        escaped_userinfo.contains("xlink:href=\"https://%23alice@example.org/report\""),
        "{escaped_userinfo}"
    );
    // Fragment spellings that already resolve via HTML percent-decoding (`#graph-%47%2541` ->
    // `graph-G%41`) are preserved instead of rewritten to a colliding literal ID (`graph-G%2541`).
    let no_decoy_rewrite = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "flowchart TD\nsubgraph G%41\nA\nend\nsubgraph G%2541\nB\nend\nclick B \"#graph-%47%2541\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        no_decoy_rewrite.contains("xlink:href=\"#graph-%47%2541\""),
        "{no_decoy_rewrite}"
    );
    // When a fragment link MUST be rewritten to target `graph-G%41` while `graph-G%2541` also
    // exists, the rewritten URL avoids the `#graph-G%2541` exact-match collision.
    let decoy_avoid = ferrigura::Renderer::new(&Value::empty_object())
        .render_with(
            "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  T1 : G#37;41, 2024-01-01, 1d\n  T2 : G%2541, 2024-01-02, 1d\n  click G%2541 href \"#graph-G#37;41\"\n",
            &ferrigura::RenderOptions {
                id: Some("graph".to_owned()),
                ..ferrigura::RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        !decoy_avoid.contains("xlink:href=\"#graph-G%2541\""),
        "{decoy_avoid}"
    );
    assert!(
        decoy_avoid.contains("xlink:href=\"#%67raph-G%2541\""),
        "{decoy_avoid}"
    );
    // Relative click URLs with percent-encoded path and colon delimiters preserve their spelling.
    for rel in ["docs%2Freport%3A2026.pdf", "data%2Freport%3A2026.pdf"] {
        let svg = render(
            &format!("flowchart TD\nA[x]\nclick A \"{rel}\"\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            svg.contains(&format!("xlink:href=\"{rel}\"")),
            "rel={rel}: {svg}"
        );
    }
    let valid_image = render(
        "flowchart TD\nA@{ img: \"https://example.com/logo.png\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        valid_image.contains("<image href=\"https://example.com/logo.png\""),
        "{valid_image}"
    );
    let encoded_image = render(
        "flowchart TD\nA@{ img: \"https://example.com/A%23B.png\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        encoded_image.contains("<image href=\"https://example.com/A%23B.png\""),
        "{encoded_image}"
    );
    let relative_colon_image = render(
        "flowchart TD\nA@{ img: \"data/chart.svg?time=12:30\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        relative_colon_image.contains("<image href=\"data/chart.svg?time=12:30\""),
        "{relative_colon_image}"
    );
    for raster_url in [
        "data:image/png;base64,iVBORw0KGgo=",
        "data:image/x-png;base64,iVBORw0KGgo=",
        "data:image/pjpeg;base64,/9j/4AAQ",
        " data:image/png;base64,iVBORw0KGgo=",
        "data: image/png;base64,iVBORw0KGgo=",
        "data:image/png ;base64,iVBORw0KGgo=",
        "data:image/apng;base64,iVBORw0KGgo=",
        "data:image/tiff;base64,SUkqAAg=",
        "data:image/jxl;base64,/wo=",
    ] {
        let raster_image = render(
            &format!("flowchart TD\nA@{{ img: \"{raster_url}\", w: 48, h: 48 }}\n"),
            &Value::empty_object(),
        )
        .expect("renders");
        assert!(
            raster_image.contains(&format!("<image href=\"{raster_url}\"")),
            "url={raster_url:?}: {raster_image}"
        );
    }
    let tab_raster_image = render(
        "flowchart TD\nA@{ img: \"data:im#9;age/png;base64,iVBORw0KGgo=\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        tab_raster_image.contains("<image href=\"data:im&#9;age/png;base64,iVBORw0KGgo=\""),
        "{tab_raster_image}"
    );
    let svg_data_image = render(
        "flowchart TD\nA@{ img: \"data:image/svg+xml,<svg onload=alert(1)>\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        svg_data_image.contains("<image href=\"about:blank\""),
        "{svg_data_image}"
    );
    let raster_data_click = render(
        "flowchart TD\nA[x]\nclick A \"data:image/png;base64,iVBORw0KGgo=\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        raster_data_click.contains("xlink:href=\"about:blank\""),
        "{raster_data_click}"
    );
    let token_image = Renderer::new(&Value::empty_object())
        .render_with(
            "flowchart TD\nA@{ img: \"{svg-id}:alert(1)\", w: 48, h: 48 }\n",
            &RenderOptions {
                id: Some("javascript".to_owned()),
                ..RenderOptions::default()
            },
        )
        .expect("renders");
    assert!(
        token_image.contains("<image href=\"{svg-id}:alert(1)\""),
        "{token_image}"
    );
    assert!(
        !token_image.contains("href=\"javascript:alert(1)\""),
        "{token_image}"
    );

    let entity_click = render(
        "flowchart TD\nA[x]\nclick A \"https#58;//example.org/\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        entity_click.contains("xlink:href=\"https://example.org/\""),
        "{entity_click}"
    );
    let entity_image = render(
        "flowchart TD\nA@{ img: \"https#58;//example.org/logo.png\", w: 48, h: 48 }\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        entity_image.contains("<image href=\"https://example.org/logo.png\""),
        "{entity_image}"
    );

    // Over-budget parser click URLs still fail closed even when another segment has a malformed %.
    let over_budget_malformed = render(
        "flowchart TD\nA[x]\nclick A \"https://example.com/a%252525252525252541?value=50%\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        over_budget_malformed.contains("xlink:href=\"about:blank\""),
        "{over_budget_malformed}"
    );

    // Untrusted front matter cannot bypass default strict securityLevel.
    let fm_bypass = render(
        "---\nconfig:\n  securityLevel: loose\n---\nflowchart TD\nA[x]\nclick A \"javascript:alert(1)\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        fm_bypass.contains("xlink:href=\"about:blank\""),
        "{fm_bypass}"
    );

    // Explicit host securityLevel: loose preserves raw URLs.
    let loose_cfg = Value::from(serde_json::json!({ "securityLevel": "loose" }));
    let loose_click = render(
        "flowchart TD\nA[x]\nclick A \"javascript:alert(1)\"\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_click.contains("xlink:href=\"javascript:alert(1)\""),
        "{loose_click}"
    );
    let loose_img = render(
        "flowchart TD\nA@{ img: \"javascript:alert(1)\", w: 48, h: 48 }\n",
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_img.contains("<image href=\"javascript:alert(1)\""),
        "{loose_img}"
    );
    let deep_url = "https://example.org/file%25252525252525252541";
    let loose_class = render(
        &format!("classDiagram\nclass A\nclick A href \"{deep_url}\"\n"),
        &loose_cfg,
    )
    .expect("renders");
    assert!(
        loose_class.contains(&format!("xlink:href=\"{deep_url}\"")),
        "{loose_class}"
    );

    // Safe relative click URLs containing non-escape backslashes or percent-encoded path
    // separators before `:` keep their original spelling, while backslash whitespace-escape scheme
    // obfuscation fails closed.
    let rel_backslash = render(
        "flowchart TD\nA[x]\nB[y]\nC[z]\nD[w]\nclick A \"data\\archive:2026.pdf\"\nclick B \"java\\tscript:alert(1)\"\nclick C href \"data%2Freport:2026.pdf\"\nclick D href \"docs%2Freport:2026.pdf\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        rel_backslash.contains("xlink:href=\"data\\archive:2026.pdf\""),
        "{rel_backslash}"
    );
    assert!(
        rel_backslash.contains("xlink:href=\"about:blank\""),
        "{rel_backslash}"
    );
    assert!(
        rel_backslash.contains("xlink:href=\"data%2Freport:2026.pdf\""),
        "{rel_backslash}"
    );
    assert!(
        rel_backslash.contains("xlink:href=\"docs%2Freport:2026.pdf\""),
        "{rel_backslash}"
    );

    // Relative filenames with spaces before `:` stay intact, and flowchart image URLs resolve
    // HTML numeric references without inserting a stray `&`.
    let space_and_img_ref = render(
        "flowchart TD\nA[x]\nB@{ img: \"https://example.org/word&#169;.png\", w: 48, h: 48 }\nC[z]\nclick A \"version 1: release notes.html\"\nclick C \"docs%3Amanual/one.html\"\n",
        &Value::empty_object(),
    )
    .expect("renders");
    assert!(
        space_and_img_ref.contains("xlink:href=\"version 1: release notes.html\""),
        "{space_and_img_ref}"
    );
    assert!(
        space_and_img_ref.contains("xlink:href=\"docs%3Amanual/one.html\""),
        "{space_and_img_ref}"
    );
    assert!(
        space_and_img_ref.contains("<image href=\"https://example.org/word©.png\""),
        "{space_and_img_ref}"
    );
    assert!(
        !space_and_img_ref.contains("word&amp;©.png"),
        "{space_and_img_ref}"
    );
}
