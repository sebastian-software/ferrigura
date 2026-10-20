//! The set-up of the swimlane render tests: a configuration from the defaults with the given overrides,
//! the globals and the theme tokens of it, and the SVG of a text.
//!
//! The facade resolves the appearance of a swimlane diagram (`theme`, `look`, `layout`) from its
//! section, and the layout engine is the `swimlane` one; here the top-level values are set directly,
//! which is what the resolution gives for them.

use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::swimlanes::{SwimlaneSection, render};
use ferrigura_render::unified::Section;
use ferrigura_syntax::swimlanes::parse;

/// A configuration from the defaults, with `theme`, `look` and `layout` set as the facade resolves them
/// for a swimlane diagram, and the given overrides merged over it.
pub fn config_with(overrides: serde_json::Value) -> MermaidConfig {
    let mut value = defaults();
    let mut base =
        serde_json::json!({ "theme": "redux-color", "look": "neo", "layout": "swimlane" });
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

/// What a test renders with: the configuration, its globals, and the theme tokens of one colour scheme.
pub struct Setup {
    pub config: MermaidConfig,
    pub globals: Globals,
    pub tokens: ThemeTokens,
}

impl Setup {
    /// The setup of a configuration, with the theme variables of `variables`, or the redux colour theme's.
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

    /// The swimlane section of the configuration.
    pub fn section(&self) -> SwimlaneSection<'_> {
        SwimlaneSection {
            swimlane: &self.config.swimlane,
            unified: Section::of(&self.config),
        }
    }

    /// The SVG of the text with the element id `id`, or the error the render gives.
    pub fn try_svg(&self, text: &str, id: &str) -> Result<String, RenderError> {
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
    }

    /// The SVG of the text, which must render.
    pub fn svg(&self, text: &str, id: &str) -> String {
        self.try_svg(text, id).expect("the diagram renders")
    }
}

/// The SVG of a text with the default configuration and the redux colour theme.
pub fn svg(text: &str) -> String {
    Setup::new(config_with(serde_json::json!({})), None).svg(text, "p0")
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

/// The fixture file of the swimlane e2e diagrams, read from the repository.
pub fn fixture(path: &str) -> String {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../e2e/diagrams/swimlanes");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}
