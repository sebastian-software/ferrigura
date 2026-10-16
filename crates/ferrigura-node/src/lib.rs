//! The `ferrigura` Node addon: `render(text, config?, options?)` returns the SVG of a Mermaid diagram,
//! with the options of the `ferrigura` crate. Node.js only; there is no WebAssembly build.

use ferrigura::{ColorScheme, RenderOptions};
use ferrigura_model::value::Value;
use napi::{Error, Result, Status};
use napi_derive::napi;

/// The options that are not Mermaid configuration.
#[napi(object)]
pub struct RenderOptionsInput {
    /// The id of the SVG element, which scopes its styles. By default it is derived from the text and
    /// the configuration, so the same input gives the same id.
    pub id: Option<String>,
    /// The colours: `light`, `dark`, or `auto` to follow the viewer's colour scheme. By default the
    /// configured theme decides.
    pub color_scheme: Option<String>,
}

/// Renders a diagram to an SVG document.
///
/// `config` is the configuration that `mermaid.initialize` takes, as an object. Front matter and
/// `%%{init}%%` directives in `text` apply on top of it, as they do in Mermaid.
///
/// # Errors
///
/// Throws if the text is not a diagram that ferrigura renders, if it does not parse, if the
/// configuration holds a value of the wrong type, or if the colour scheme or the id is not valid.
#[napi]
pub fn render(
    text: String,
    config: Option<serde_json::Value>,
    options: Option<RenderOptionsInput>,
) -> Result<String> {
    let config = config.map_or_else(Value::empty_object, Value::from);
    let options = match options {
        Some(input) => RenderOptions {
            id: input.id,
            color_scheme: input
                .color_scheme
                .as_deref()
                .map(colour_scheme)
                .transpose()?,
            now: None,
        },
        None => RenderOptions::default(),
    };
    ferrigura::render_with(&text, &config, &options)
        .map_err(|error| Error::new(Status::GenericFailure, error.to_string()))
}

fn colour_scheme(name: &str) -> Result<ColorScheme> {
    match name {
        "light" => Ok(ColorScheme::Light),
        "dark" => Ok(ColorScheme::Dark),
        "auto" => Ok(ColorScheme::Auto),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("unknown colour scheme {other}, expected light, dark or auto"),
        )),
    }
}
