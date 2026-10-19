//! The helpers of the venn tests: a diagram is parsed, its configuration is the defaults with the
//! overrides of the test, and it is drawn with a theme given as variables. The test files include
//! this module with `mod venn_common;`.

#![allow(dead_code)]

use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::venn::{DEFAULT_SEED, VennSection, render};

/// What a test changes on top of the defaults.
#[derive(Default)]
pub struct Options {
    /// Overrides of the venn section of the configuration (`config.venn`).
    pub venn: serde_json::Value,
    /// Overrides of the top level of the configuration (`theme`, `look` and so on).
    pub top: serde_json::Value,
    /// Overrides of the theme variables (`themeVariables`), on top of `redux-color`.
    pub variables: serde_json::Value,
    /// The `look` the globals give (`handDrawn` for the hand-drawn look).
    pub look: Option<&'static str>,
    /// The seed of the layout's random numbers; the default seed when `None`.
    pub seed: Option<u32>,
    /// The redux dark theme instead of the light one.
    pub dark_theme: bool,
}

/// The configuration of a test: the defaults, with the overrides merged in.
pub fn config(options: &Options) -> MermaidConfig {
    let mut json = serde_json::to_value(defaults()).expect("the defaults serialise");
    merge(&mut json, &options.top);
    let venn = json
        .as_object_mut()
        .expect("the defaults are an object")
        .entry("venn")
        .or_insert_with(|| serde_json::json!({}));
    merge(venn, &options.venn);
    typed(&Value::from(json)).expect("the configuration is typed")
}

/// Merges the members of `overrides` into `target`, as objects are merged; any other value replaces.
fn merge(target: &mut serde_json::Value, overrides: &serde_json::Value) {
    match (target, overrides) {
        (serde_json::Value::Object(target), serde_json::Value::Object(overrides)) => {
            for (key, value) in overrides {
                merge(
                    target.entry(key.clone()).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        }
        (target, overrides) if !overrides.is_null() => *target = overrides.clone(),
        _ => {}
    }
}

/// The theme tokens of a test: the redux theme of the options, with its variables overridden.
pub fn tokens(options: &Options) -> ThemeTokens {
    let theme = if options.dark_theme {
        Theme::ReduxDarkColor
    } else {
        Theme::ReduxColor
    };
    let variables = if options.variables.is_null() {
        theme.variables(None)
    } else {
        theme.variables(Some(&Value::from(options.variables.clone())))
    };
    ThemeTokens::from_value(&variables)
}

/// Parses the diagram text.
pub fn parse(text: &str) -> ferrigura_model::venn::VennDiagram {
    ferrigura_syntax::venn::parse(text).expect("the venn diagram parses")
}

/// Renders the diagram of `text` with the options, under the id `venn1`.
pub fn draw(text: &str, options: &Options) -> Result<String, RenderError> {
    let diagram = parse(text);
    let config = config(options);
    let tokens = tokens(options);
    let mut globals = Globals::of(&config);
    if let Some(look) = options.look {
        globals.look = look.to_owned();
    }
    render(&Context {
        diagram: &diagram,
        section: &VennSection {
            config: &config.venn,
            seed: options.seed.unwrap_or(DEFAULT_SEED),
        },
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "venn1",
    })
}

/// The render of a diagram with the defaults.
pub fn plain(text: &str) -> String {
    draw(text, &Options::default()).expect("the diagram renders")
}

/// Every element whose start tag contains `needle`, as the text of the tag.
pub fn start_tags<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
    let prefix = format!("<{tag}");
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find(&prefix) {
        let after = &rest[start + prefix.len()..];
        if !after.starts_with([' ', '>', '/']) {
            rest = &rest[start + prefix.len()..];
            continue;
        }
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        found.push(&rest[start..start + end + 1]);
        rest = &rest[start + end + 1..];
    }
    found
}

/// The value of attribute `name` in a start tag, as written (`None` when the tag has none).
pub fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    Some(&tag[start..start + end])
}

/// The text between the start tag that contains `marker` and its end tag.
pub fn inner_text<'a>(svg: &'a str, marker: &str) -> Option<&'a str> {
    let at = svg.find(marker)?;
    let open_end = at + svg[at..].find('>')? + 1;
    let close = open_end + svg[open_end..].find("</")?;
    Some(&svg[open_end..close])
}
