//! Helpers of the railroad tests: the reference's stub measurement, the default spacing and colours,
//! and the rendering of a parsed text with a theme.

use ferrigura_model::config::RailroadDiagramConfig;
use ferrigura_model::railroad::RailroadDiagram;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::common::Element;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::railroad::layout::{Drawing, Measure, Spacing, TextSize, draw_rules};
use ferrigura_render::railroad::style::Colours;
use ferrigura_render::railroad::text::TextMeasure;
use ferrigura_render::railroad::{RailroadSection, render};
use ferrigura_text::TextError;

/// The measurement of `railroadRenderer.spec.ts`: `getBBox` is the text's length times 8, at least
/// 1, and 16 high. The length counts UTF-16 units, as JavaScript does.
pub struct Stub;

impl Measure for Stub {
    fn text_size(&mut self, text: &str) -> Result<TextSize, TextError> {
        Ok(TextSize {
            width: (text.encode_utf16().count() as f64 * 8.0).max(1.0),
            height: 16.0,
        })
    }
}

/// The spacing of the reference's defaults (`DEFAULT_RAILROAD_CONFIG`).
pub fn spacing() -> Spacing {
    Spacing {
        padding: 10.0,
        vertical_separation: 8.0,
        horizontal_separation: 10.0,
        arc_radius: 10.0,
        marker_radius: 5.0,
    }
}

/// Colours that the tests do not look at, except where they are named.
pub fn colours() -> Colours {
    Colours {
        terminal_fill: "#ffffde".to_owned(),
        terminal_stroke: "#333333".to_owned(),
        terminal_text: "#000021".to_owned(),
        non_terminal_fill: "#ececff".to_owned(),
        non_terminal_stroke: "#9370db".to_owned(),
        non_terminal_text: "#000021".to_owned(),
        line: "#333333".to_owned(),
        marker_fill: "#333333".to_owned(),
        comment_fill: "#e8e8e8".to_owned(),
        comment_stroke: "#888888".to_owned(),
        comment_text: "#666666".to_owned(),
        special_fill: "#f0e0ff".to_owned(),
        special_stroke: "#8800cc".to_owned(),
        rule_name: "#000066".to_owned(),
    }
}

/// Parses a text of any of the four syntaxes.
pub fn parse(text: &str) -> RailroadDiagram {
    ferrigura_syntax::railroad::parse(text).expect("the text parses")
}

/// The drawing of a text with the reference's stub measurement.
pub fn draw_stub(text: &str) -> Drawing {
    let diagram = parse(text);
    draw_rules(&diagram.rules, &spacing(), &colours(), &mut Stub).expect("the stub measures")
}

/// The drawing of a text with the bundled face at 14 px.
pub fn draw_real(text: &str) -> Drawing {
    let diagram = parse(text);
    let mut measure = TextMeasure::new(14.0);
    draw_rules(&diagram.rules, &spacing(), &colours(), &mut measure).expect("the text measures")
}

/// The redux theme's tokens, light or dark.
pub fn tokens(theme: Theme) -> ThemeTokens {
    ThemeTokens::from_value(&theme.variables(None))
}

/// Tokens from a map of theme variables.
pub fn tokens_of(variables: serde_json::Value) -> ThemeTokens {
    ThemeTokens::from_value(&Value::from(variables))
}

/// Renders a text with a section, the light tokens and, when given, the dark ones.
pub fn render_with(
    text: &str,
    config: &RailroadDiagramConfig,
    light: &ThemeTokens,
    dark: Option<&ThemeTokens>,
    detector: &'static str,
) -> String {
    let diagram = parse(text);
    let globals = Globals {
        look: "classic".to_owned(),
        theme: "redux-color".to_owned(),
        font_family: "\"Lexica Ultralegible\", \"Noto Sans Math\"".to_owned(),
        font_size: 16.0,
        security_level: "strict".to_owned(),
        layout: "dagre".to_owned(),
        html_labels: Some(false),
    };
    let section = RailroadSection { config, detector };
    render(&Context {
        diagram: &diagram,
        section: &section,
        globals: &globals,
        theme: ThemeView { light, dark },
        id: "railroad-test",
    })
    .expect("the diagram renders")
}

/// The light redux theme, with the configuration's defaults.
pub fn render_default(text: &str) -> String {
    let light = tokens(Theme::ReduxColor);
    render_with(
        text,
        &RailroadDiagramConfig::default(),
        &light,
        None,
        "railroad",
    )
}

/// Every element below `element`, in document order.
pub fn descendants(element: &Element) -> Vec<&Element> {
    fn collect<'a>(element: &'a Element, out: &mut Vec<&'a Element>) {
        for child in element.children() {
            out.push(child);
            collect(child, out);
        }
    }
    let mut out = Vec::new();
    collect(element, &mut out);
    out
}

/// The elements of `elements` whose class is `class`.
pub fn classed<'a>(elements: &[&'a Element], class: &str) -> Vec<&'a Element> {
    elements
        .iter()
        .copied()
        .filter(|element| element.get("class") == Some(class))
        .collect()
}

/// The direct children of `element` whose class is `class`.
pub fn children_classed<'a>(element: &'a Element, class: &str) -> Vec<&'a Element> {
    element
        .children()
        .iter()
        .filter(|child| child.get("class") == Some(class))
        .collect()
}

/// The `d` of a path element.
pub fn path_data(element: &Element) -> &str {
    element.get("d").unwrap_or_default()
}
