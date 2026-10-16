//! ferrigura: renders Mermaid diagram text to SVG, without a browser.
//!
//! [`render`] is the whole interface: the diagram text, and the configuration the user would pass
//! to `initialize`, go in, and an SVG document comes out.

#![warn(missing_docs)]

use ferrigura_model::config::{assign_with_depth, effective, sanitize_directive, typed};
use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::pie::{PieInput, render as render_pie};
use ferrigura_syntax::{DiagramType, ParseError, config, detect, pie};

pub use ferrigura_render::RenderError as Error;

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
    match detect(text) {
        Some(DiagramType::Pie) => render_pie_text(text, config),
        Some(DiagramType::Info) => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "the info diagram is not rendered yet".to_owned(),
        })),
        None => Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "no diagram type detected".to_owned(),
        })),
    }
}

fn render_pie_text(text: &str, site: &Value) -> Result<String, RenderError> {
    let mut directive = config::read(text)?;
    let effective_config = effective(Some(site), Some("pie"), &directive);
    let typed_config =
        typed(&effective_config).map_err(|error| RenderError::Config(error.to_string()))?;

    sanitize_directive(&mut directive);
    let mut overrides = site
        .get("themeVariables")
        .cloned()
        .unwrap_or_else(Value::empty_object);
    if let Some(directive_overrides) = directive.get("themeVariables") {
        assign_with_depth(&mut overrides, directive_overrides, 2);
    }
    let theme_name = effective_config
        .get("theme")
        .and_then(Value::as_str)
        .unwrap_or("default");
    let theme_variables = Theme::from_name(theme_name).variables(Some(&overrides));

    let diagram = pie::parse(text)?;
    render_pie(&PieInput {
        diagram: &diagram,
        config: &typed_config.pie,
        theme: &theme_variables,
        id: "ferrigura",
    })
}
