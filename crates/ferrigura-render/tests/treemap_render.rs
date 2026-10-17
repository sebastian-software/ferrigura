//! The treemap renderer on the cases that `docs-port/treemap/NOTES.md` (section 6) names. The
//! geometry of the fixtures and the probes is compared with Chromium by `pnpm port:geometry
//! treemap`; these tests hold the reference's rules that the markup and the errors show.

use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::theme::Themes;
use ferrigura_render::treemap::{TreemapInput, TreemapSettings, render};

/// Renders a treemap with the redux colour theme and the `treemap` section given as JSON.
fn draw(text: &str, section: serde_json::Value) -> Result<String, RenderError> {
    let diagram = ferrigura_syntax::treemap::parse(text).expect("the treemap parses");
    let effective = Value::from(serde_json::json!({ "treemap": section }));
    let settings = TreemapSettings::of(&effective);
    let variables = Theme::ReduxColor.variables(None);
    let config = ferrigura_model::config::typed(&ferrigura_model::config::defaults())
        .expect("the defaults are typed");
    render(&TreemapInput {
        diagram: &diagram,
        config: &config,
        settings: &settings,
        themes: Themes::Single(&variables),
        id: "t",
    })
}

/// The `viewBox` attribute of the root element.
fn view_box(svg: &str) -> &str {
    let start = svg.find("viewBox=\"").map_or(0, |index| index + 9);
    let rest = &svg[start..];
    &rest[..rest.find('"').unwrap_or(0)]
}

#[test]
fn the_basic_tree_has_the_viewbox_the_reference_gives() {
    // NOTES 6.8: the box of the layout is [10, 35, 980, 355], and the viewport adds 8 px.
    let svg = draw(
        "treemap-beta\n\"Root\"\n  \"A\": 10\n  \"B\": 20\n",
        serde_json::json!({}),
    )
    .expect("the tree renders");
    assert_eq!(view_box(&svg), "2 27 996 371");
}

#[test]
fn a_dollar_format_with_an_invalid_specifier_fails_the_render_when_a_value_is_shown() {
    let error = draw(
        "treemap\n  \"Section\"\n    \"Leaf\": 10\n",
        serde_json::json!({ "valueFormat": "$xyz" }),
    )
    .expect_err("`$xyz` fails the render");
    assert_eq!(
        error.to_string(),
        "invalid configuration: invalid format: xyz"
    );
}

#[test]
fn a_dollar_format_with_an_invalid_specifier_does_not_fail_when_no_value_is_shown() {
    let svg = draw(
        "treemap\n  \"Section\"\n    \"Leaf\": 10\n",
        serde_json::json!({ "valueFormat": "$xyz", "showValues": false }),
    );
    assert!(svg.is_ok());
}

#[test]
fn an_invalid_specifier_without_a_dollar_falls_back_to_the_comma_format() {
    let svg = draw(
        "treemap\n  \"Section\"\n    \"Leaf\": 1234\n",
        serde_json::json!({ "valueFormat": "xyz" }),
    )
    .expect("the tree renders");
    assert!(svg.contains(">1,234<"), "{svg}");
}

#[test]
fn a_placeholder_for_an_entity_is_written_as_the_entity() {
    let svg = draw(
        "treemap\n  \"Section\"\n    \"Entity #quot; here\": 1\n",
        serde_json::json!({}),
    )
    .expect("the tree renders");
    assert!(svg.contains(">Entity &quot; here</text>"), "{svg}");
}

#[test]
fn a_zero_value_shows_no_value_text() {
    let svg = draw(
        "treemap\n  \"Section\"\n    \"Nothing\": 0\n    \"Something\": 5\n",
        serde_json::json!({ "valueFormat": "," }),
    )
    .expect("the tree renders");
    assert!(svg.contains("class=\"treemapValue\""), "{svg}");
    assert!(!svg.contains(">0</text>"), "{svg}");
}

#[test]
fn show_values_off_writes_no_value_text() {
    let svg = draw(
        "treemap\n  \"Section\"\n    \"Leaf\": 10\n",
        serde_json::json!({ "showValues": false }),
    )
    .expect("the tree renders");
    // The style sheet names the classes too, so the elements are looked for by their class.
    assert!(!svg.contains("class=\"treemapValue\""), "{svg}");
    assert!(!svg.contains("class=\"treemapSectionValue\""), "{svg}");
}

#[test]
fn the_title_is_written_with_the_title_class() {
    let svg = draw(
        "---\ntitle: A title\n---\ntreemap\n  \"Section\"\n    \"Leaf\": 10\n",
        serde_json::json!({}),
    )
    .expect("the tree renders");
    assert!(svg.contains("class=\"treemapTitle\""), "{svg}");
    assert!(svg.contains(">A title</text>"), "{svg}");
}
