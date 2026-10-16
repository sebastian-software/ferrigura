//! ferrigura: renders Mermaid diagram text to SVG, without a browser.
//!
//! [`render`] is the whole interface: the diagram text, and the configuration the user would pass
//! to `initialize`, go in, and an SVG document comes out. [`render_with`] takes the options that
//! are not Mermaid configuration: the id of the SVG and the colour scheme.

#![warn(missing_docs)]

use std::hash::Hasher;

use ferrigura_model::config::{effective, typed, user_theme_variables};
use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::flowchart::{self, FlowchartInput};
use ferrigura_render::pie::{PieInput, Themes, render as render_pie};
use ferrigura_syntax::{DiagramType, ParseError, config, detect, pie};

pub use ferrigura_render::RenderError as Error;

/// The colour scheme of the output.
///
/// The configuration decides the colours by default: the configured theme, as Mermaid does. `Light`
/// and `Dark` draw with the shipped light and dark themes, and `Auto` writes both, so that one SVG
/// follows the viewer's `prefers-color-scheme`. The user's `themeVariables` apply to both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// The light theme, `redux-color`.
    Light,
    /// The dark theme, `redux-dark-color`.
    Dark,
    /// Both themes; the dark one applies under `prefers-color-scheme: dark`.
    Auto,
}

/// The options of a render that are not Mermaid configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderOptions {
    /// The id of the SVG element, which scopes its styles. It must start with an ASCII letter and
    /// contain only ASCII letters, digits, `-` and `_`. By default the id is derived from the text
    /// and the configuration, so the same input gives the same id.
    pub id: Option<String>,
    /// The colour scheme. `None` keeps the colours of the configured theme.
    pub color_scheme: Option<ColorScheme>,
}

/// Renders a diagram to an SVG document.
///
/// `config` is the configuration the user would pass to `initialize`, as a [`Value`]; an empty object
/// gives the defaults. Front matter and `%%{init}%%` directives in `text` apply on top of it, as they
/// do in Mermaid.
///
/// # Errors
///
/// Returns an error if the text is not a diagram that ferrigura renders, if it does not parse, if
/// the configuration holds a value of the wrong type, or if a bundled font cannot be read.
pub fn render(text: &str, config: &Value) -> Result<String, RenderError> {
    render_with(text, config, &RenderOptions::default())
}

/// Renders a diagram with the options given. See [`render`].
///
/// # Errors
///
/// Returns the errors of [`render`], and an error if the id is not a valid CSS identifier.
pub fn render_with(
    text: &str,
    config: &Value,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    match detect(text) {
        Some(DiagramType::Pie) => render_pie_text(text, config, options),
        Some(DiagramType::Flowchart) => with_flowchart(text, config, options, flowchart::render),
        Some(DiagramType::Info) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the info diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::Timeline) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the timeline diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::Journey) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the journey diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::GitGraph) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the gitGraph diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::Er) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the ER diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::Class) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the class diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::Sequence) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the sequence diagram is not rendered yet".to_owned(),
        })),
        Some(DiagramType::State) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the state diagram is not rendered yet".to_owned(),
        })),
        None => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "no diagram type detected".to_owned(),
        })),
    }
}

fn render_pie_text(
    text: &str,
    site: &Value,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = match &options.id {
        Some(id) if valid_id(id) => id.clone(),
        Some(id) => return Err(RenderError::Options(format!("invalid id {id:?}"))),
        None => derived_id(text, site),
    };
    let directive = config::read(text)?;
    let effective_config = effective(Some(site), Some("pie"), &directive);
    let typed_config =
        typed(&effective_config).map_err(|error| RenderError::Config(error.to_string()))?;

    let configured = effective_config
        .get("themeVariables")
        .cloned()
        .unwrap_or_else(Value::empty_object);
    let overrides = user_theme_variables(Some(site), &directive);
    let light = Theme::ReduxColor.variables(Some(&overrides));
    let dark = Theme::ReduxDarkColor.variables(Some(&overrides));
    let themes = match options.color_scheme {
        None => Themes::Single(&configured),
        Some(ColorScheme::Light) => Themes::Single(&light),
        Some(ColorScheme::Dark) => Themes::Single(&dark),
        Some(ColorScheme::Auto) => Themes::Schemes {
            light: &light,
            dark: &dark,
        },
    };

    let diagram = pie::parse(text)?;
    render_pie(&PieInput {
        diagram: &diagram,
        config: &typed_config.pie,
        themes,
        id: &id,
    })
}

/// The graph a flowchart hands to ELK and the graph ELK returns, as elkjs JSON:
/// `{"elkInput": …, "elkOutput": …}`. For the differential comparison with the reference
/// (`tools/port/geometry/flowchart-geometry.ts`); not part of the interface.
///
/// # Errors
///
/// Returns the errors of [`render_with`] for a flowchart, and an error for any other diagram.
#[doc(hidden)]
pub fn flowchart_elk_stages(
    text: &str,
    config: &Value,
    options: &RenderOptions,
) -> Result<serde_json::Value, RenderError> {
    if detect(text) != Some(DiagramType::Flowchart) {
        return Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "not a flowchart".to_owned(),
        }));
    }
    let stages = with_flowchart(text, config, options, flowchart::elk_stages)?;
    Ok(serde_json::json!({
        "elkInput": stages.input,
        "elkOutput": stages.output,
    }))
}

/// Parses a flowchart, resolves its configuration and theme, and hands them to `step`.
fn with_flowchart<T>(
    text: &str,
    site: &Value,
    options: &RenderOptions,
    step: impl FnOnce(&FlowchartInput<'_>) -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    let id = match &options.id {
        Some(id) if valid_id(id) => id.clone(),
        Some(id) => return Err(RenderError::Options(format!("invalid id {id:?}"))),
        None => derived_id(text, site),
    };
    let directive = config::read(text)?;
    let effective_config = effective(Some(site), Some("flowchart"), &directive);
    let typed_config =
        typed(&effective_config).map_err(|error| RenderError::Config(error.to_string()))?;

    let configured = effective_config
        .get("themeVariables")
        .cloned()
        .unwrap_or_else(Value::empty_object);
    let overrides = user_theme_variables(Some(site), &directive);
    let light = Theme::ReduxColor.variables(Some(&overrides));
    let dark = Theme::ReduxDarkColor.variables(Some(&overrides));
    let themes = match options.color_scheme {
        None => Themes::Single(&configured),
        Some(ColorScheme::Light) => Themes::Single(&light),
        Some(ColorScheme::Dark) => Themes::Single(&dark),
        Some(ColorScheme::Auto) => Themes::Schemes {
            light: &light,
            dark: &dark,
        },
    };

    let diagram = ferrigura_syntax::flowchart::parse(text)
        .map_err(|ferrigura_syntax::flowchart::FlowchartError::Syntax(error)| error)?;
    step(&FlowchartInput {
        diagram: &diagram,
        config: &typed_config,
        themes,
        id: &id,
    })
}

/// Whether an id can scope the styles of a diagram: a CSS identifier that needs no escaping.
fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The id a diagram gets when none is given: `ferrigura-` and a 64-bit FNV-1a hash of the text and the
/// configuration, so the same input always gives the same id.
fn derived_id(text: &str, site: &Value) -> String {
    let configuration = serde_json::to_string(site).unwrap_or_default();
    let mut hash = Fnv1a::new();
    hash.write(text.as_bytes());
    hash.write(&[0xff]);
    hash.write(configuration.as_bytes());
    format!("ferrigura-{:016x}", hash.finish())
}

/// 64-bit FNV-1a, which is stable across Rust releases, unlike the standard hasher.
struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv1a {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}
