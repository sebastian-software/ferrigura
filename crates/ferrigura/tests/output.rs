//! The SVG a pie is written as: colours as custom properties with presentation attribute fallback
//! (decision 0007), the scope of the styles to the diagram's id, the colour schemes, and the
//! accessible title and description.

use ferrigura::{ColorScheme, RenderOptions, render, render_with};
use ferrigura_model::value::Value;

/// The light theme's variables that the tests check, as the shipped light theme writes them.
const LIGHT_TITLE: &str = "#28253D";
const LIGHT_FIRST_SLICE: &str = "#f4a8ff";
/// The dark theme's title colour.
const DARK_TITLE: &str = "#ccc";

const PIE: &str = "pie title Sports in Sweden\n  \"Bandy\" : 40\n  \"Ice-Hockey\" : 80\n";

fn svg_with(text: &str, scheme: Option<ColorScheme>, id: Option<&str>) -> String {
    let options = RenderOptions {
        id: id.map(str::to_owned),
        color_scheme: scheme,
        now: None,
        git_graph_seed: None,
        random_seed: None,
    };
    render_with(text, &Value::empty_object(), &options).expect("the pie renders")
}

/// The contents of the `<style>` element.
fn style_of(svg: &str) -> &str {
    let start = svg.find("<style>").expect("a style element") + "<style>".len();
    let end = svg[start..].find("</style>").expect("the style closes") + start;
    &svg[start..end]
}

/// The diagram's id, as the root element names it.
fn id_of(svg: &str) -> String {
    let rest = svg.split_once("id=\"").expect("an id").1;
    rest.split_once('"').expect("the id closes").0.to_owned()
}

/// Every selector of the style (the text before each `{`), with the font faces left out.
fn selectors(css: &str) -> Vec<String> {
    let mut without_faces = String::new();
    let mut rest = css;
    while let Some(start) = rest.find("@font-face{") {
        without_faces.push_str(&rest[..start]);
        let end = rest[start..].find('}').expect("the face closes") + start + 1;
        rest = &rest[end..];
    }
    without_faces.push_str(rest);

    let mut found = Vec::new();
    let mut current = String::new();
    for c in without_faces.chars() {
        match c {
            '{' => {
                let selector = current.trim().to_owned();
                if !selector.is_empty() {
                    found.push(selector);
                }
                current.clear();
            }
            '}' | ';' => current.clear(),
            other => current.push(other),
        }
    }
    found
}

#[test]
fn colours_are_custom_properties_declared_on_the_diagram() {
    let out = svg_with(PIE, None, Some("pie-one"));
    let css = style_of(&out);
    assert!(css.starts_with("@font-face"), "the faces come first");
    assert!(
        css.contains("#pie-one{"),
        "the declarations are on the diagram"
    );
    assert!(
        css.contains(&format!("--pie1: {LIGHT_FIRST_SLICE};")),
        "the first slice colour is a custom property"
    );
    assert!(
        css.contains(&format!("--pieTitleTextColor: {LIGHT_TITLE};")),
        "the title colour is a custom property"
    );
}

#[test]
fn colours_in_rules_are_references_to_the_custom_properties() {
    let out = svg_with(PIE, None, Some("pie-one"));
    let css = style_of(&out);
    assert!(
        css.contains("#pie-one .pieCircle.pie-slot-0{fill: var(--pie1);}"),
        "{css}"
    );
    assert!(css.contains("fill: var(--pieTitleTextColor)"), "{css}");
    assert!(css.contains("stroke: var(--pieOuterStrokeColor)"), "{css}");
}

#[test]
fn elements_keep_their_colour_as_a_presentation_attribute() {
    let out = svg_with(PIE, None, Some("pie-one"));
    assert!(
        out.contains("class=\"pieCircle pie-slot-0\" d=\"")
            && out.contains(&format!("fill=\"{LIGHT_FIRST_SLICE}\"")),
        "{out}"
    );
    assert!(
        out.contains(&format!("class=\"pieTitleText\" fill=\"{LIGHT_TITLE}\"")),
        "{out}"
    );
}

#[test]
fn no_element_sets_a_colour_inline() {
    let out = svg_with(PIE, None, Some("pie-one"));
    let body = out
        .split("</style>")
        .nth(1)
        .expect("a body after the style");
    assert!(!body.contains("style=\"fill"), "{body}");
    assert!(!body.contains("style=\"stroke"), "{body}");
    // The only inline style left is the width of the root element.
    assert_eq!(out.matches(" style=\"").count(), 1, "{out}");
}

#[test]
fn the_light_scheme_writes_the_light_theme_only() {
    let out = svg_with(PIE, Some(ColorScheme::Light), Some("pie-one"));
    assert!(!out.contains("@media"), "{out}");
    assert!(out.contains(&format!("fill=\"{LIGHT_TITLE}\"")), "{out}");
}

#[test]
fn the_dark_scheme_draws_with_the_dark_theme() {
    let out = svg_with(PIE, Some(ColorScheme::Dark), Some("pie-one"));
    assert!(!out.contains("@media"), "{out}");
    assert!(
        out.contains(&format!("--pieTitleTextColor: {DARK_TITLE};")),
        "{out}"
    );
    assert!(out.contains(&format!("fill=\"{DARK_TITLE}\"")), "{out}");
}

#[test]
fn auto_writes_both_sets_and_the_dark_one_under_the_media_query() {
    let out = svg_with(PIE, Some(ColorScheme::Auto), Some("pie-one"));
    let css = style_of(&out);
    let (light_block, dark_block) = css
        .split_once("@media (prefers-color-scheme: dark)")
        .expect("a media query for the dark scheme");
    assert!(
        light_block.contains(&format!("--pieTitleTextColor: {LIGHT_TITLE};")),
        "the light set is written"
    );
    assert!(
        dark_block.starts_with("{#pie-one{"),
        "the dark set is scoped to the diagram"
    );
    assert!(
        dark_block.contains(&format!("--pieTitleTextColor: {DARK_TITLE};")),
        "the dark set is written"
    );
    // The attributes carry the light colours, which viewers without custom properties draw.
    assert!(out.contains(&format!("fill=\"{LIGHT_TITLE}\"")), "{out}");
}

#[test]
fn every_selector_starts_with_the_diagram_id() {
    for scheme in [None, Some(ColorScheme::Light), Some(ColorScheme::Auto)] {
        let out = svg_with(PIE, scheme, Some("pie-one"));
        for selector in selectors(style_of(&out)) {
            assert!(
                selector.starts_with("#pie-one") || selector.starts_with("@media"),
                "{selector} is not scoped, in {scheme:?}"
            );
        }
    }
}

#[test]
fn two_diagrams_with_different_ids_share_no_selector() {
    let first = svg_with(PIE, None, Some("pie-one"));
    let second = svg_with("pie\n  \"x\" : 1\n", None, Some("pie-two"));
    for selector in selectors(style_of(&first)) {
        assert!(!selector.starts_with("#pie-two"), "{selector}");
    }
    for selector in selectors(style_of(&second)) {
        assert!(!selector.starts_with("#pie-one"), "{selector}");
    }
}

#[test]
fn the_default_id_comes_from_the_text_and_the_configuration() {
    let first = render(PIE, &Value::empty_object()).expect("renders");
    let again = render(PIE, &Value::empty_object()).expect("renders");
    let other = render("pie\n  \"x\" : 1\n", &Value::empty_object()).expect("renders");
    assert_eq!(id_of(&first), id_of(&again));
    assert_ne!(id_of(&first), id_of(&other));
    let id = id_of(&first);
    assert!(id.starts_with("ferrigura-"), "{id}");
    assert_eq!(id.len(), "ferrigura-".len() + 16, "{id}");
}

#[test]
fn an_id_that_is_not_a_css_identifier_is_refused() {
    for bad in ["1pie", "my pie", "a.b", "", "x{}"] {
        let options = RenderOptions {
            id: Some(bad.to_owned()),
            color_scheme: None,
            now: None,
            git_graph_seed: None,
            random_seed: None,
        };
        assert!(
            render_with(PIE, &Value::empty_object(), &options).is_err(),
            "{bad:?} was accepted"
        );
    }
    assert!(svg_with(PIE, None, Some("my-pie_2")).contains("id=\"my-pie_2\""));
}

#[test]
fn accessible_title_and_description_are_the_first_children() {
    let text = "pie title Sports\n  accTitle: A neat title\n  accDescr: A neat description\n  \"Bandy\" : 40\n";
    let out = svg_with(text, None, Some("pie-one"));
    let start = out.find("<svg").expect("a root");
    let after_root = &out[out[start..].find('>').expect("the root closes") + start + 1..];
    assert!(
        after_root.starts_with("<title id=\"chart-title-pie-one\">A neat title</title><desc id=\"chart-desc-pie-one\">A neat description</desc>"),
        "{out}"
    );
}

#[test]
fn the_root_refers_to_the_title_and_the_description() {
    let text = "pie\n  accTitle: A neat title\n  accDescr: A neat description\n  \"Bandy\" : 40\n";
    let out = svg_with(text, None, Some("pie-one"));
    assert!(out.contains("role=\"graphics-document document\""), "{out}");
    assert!(out.contains("aria-roledescription=\"pie\""), "{out}");
    assert!(
        out.contains("aria-labelledby=\"chart-title-pie-one\""),
        "{out}"
    );
    assert!(
        out.contains("aria-describedby=\"chart-desc-pie-one\""),
        "{out}"
    );
}

#[test]
fn without_accessible_texts_there_is_no_title_or_description() {
    let out = svg_with(PIE, None, Some("pie-one"));
    assert!(!out.contains("<title"), "{out}");
    assert!(!out.contains("<desc"), "{out}");
    assert!(!out.contains("aria-labelledby"), "{out}");
    assert!(!out.contains("aria-describedby"), "{out}");
}

#[test]
fn accessible_texts_are_escaped() {
    let text = "pie\n  accTitle: a < b & c\n  \"Bandy\" : 40\n";
    let out = svg_with(text, None, Some("pie-one"));
    assert!(out.contains("a &lt; b &amp; c</title>"), "{out}");
}
