//! The set-up of the agentflow render tests: a configuration from the defaults with the given
//! overrides, the globals and the theme tokens of it, and the layout data or the SVG of a text.
//!
//! The facade resolves the appearance of agentflow (`theme`, `look`) from its section; here the
//! top-level `theme` and `look` are set directly, which is what the resolution gives for them.

use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::agentflow::data;
use ferrigura_render::agentflow::{AgentflowSection, render};
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::unified::Section;
use ferrigura_render::unified::data::LayoutData;
use ferrigura_syntax::agentflow::parse;

/// A configuration from the defaults, with `theme` and `look` set as the facade resolves them for
/// agentflow, and the given overrides merged over it.
pub fn config_with(overrides: serde_json::Value) -> MermaidConfig {
    let mut value = defaults();
    let mut base = serde_json::json!({ "theme": "redux-color", "look": "neo" });
    merge(&mut base, overrides);
    let object = value.as_object_mut().expect("the defaults are an object");
    for (key, setting) in base.as_object().expect("an object") {
        object.insert(key.clone(), Value::from(setting.clone()));
    }
    typed(&value).expect("the configuration reads back into its types")
}

/// The overrides, merged over the base object: objects by key, everything else replaced.
fn merge(base: &mut serde_json::Value, overrides: serde_json::Value) {
    match (base, overrides) {
        (serde_json::Value::Object(base), serde_json::Value::Object(overrides)) => {
            for (key, value) in overrides {
                merge(base.entry(key).or_insert(serde_json::Value::Null), value);
            }
        }
        (base, overrides) => *base = overrides,
    }
}

/// What a test renders with: the configuration, its globals, and the theme tokens of one colour
/// scheme, built from `themeVariables` (the redux colour theme's when none are given).
pub struct Setup {
    pub config: MermaidConfig,
    pub globals: Globals,
    pub tokens: ThemeTokens,
}

impl Setup {
    /// The setup of a configuration, with the theme variables of `variables` (or redux-color).
    pub fn new(config: MermaidConfig, variables: Option<serde_json::Value>) -> Self {
        let tokens = match variables {
            Some(variables) => ThemeTokens::from_value(&Value::from(variables)),
            None => ThemeTokens::from_value(&Theme::ReduxColor.variables(None)),
        };
        Self {
            globals: Globals::of(&config),
            config,
            tokens,
        }
    }

    /// The agentflow section of the configuration.
    pub fn section(&self) -> AgentflowSection<'_> {
        AgentflowSection {
            agentflow: &self.config.agentflow,
            unified: Section::of(&self.config),
        }
    }

    /// The layout data `getData` gives the text, with the palette of this setup.
    pub fn data(&self, text: &str) -> LayoutData {
        let diagram = parse(text).expect("the text parses");
        let section = self.section();
        let palette_len = self.tokens.border_color_array.texts().len();
        data::get_data(&diagram, &section, &self.globals, palette_len)
    }

    /// The SVG of the text, with the element id `id`.
    pub fn svg(&self, text: &str, id: &str) -> String {
        let diagram = parse(text).expect("the text parses");
        let section = self.section();
        render(&Context {
            diagram: &diagram,
            section: &section,
            globals: &self.globals,
            theme: ThemeView {
                light: &self.tokens,
                dark: None,
            },
            id,
        })
        .expect("the diagram renders")
    }
}

/// The text of a `<style>` element of an SVG.
pub fn style_of(svg: &str) -> &str {
    let start = svg.find("<style>").map_or(0, |at| at + "<style>".len());
    let end = svg[start..]
        .find("</style>")
        .map_or(svg.len(), |at| start + at);
    &svg[start..end]
}

/// A count of the occurrences of `needle` in `text`.
pub fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}
