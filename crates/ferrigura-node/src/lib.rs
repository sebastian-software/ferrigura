//! The `ferrigura` Node addon, Node.js only; there is no WebAssembly build.
//!
//! `render(text, config?, options?)` returns the SVG of a Mermaid diagram, with the options of the
//! `ferrigura` crate. `Renderer` resolves a configuration once, for many renders:
//! `new Renderer(config?)`, then `renderer.render(text, options?)`. Both give the same SVG for the
//! same configuration, byte for byte.

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
/// `%%{init}%%` directives in `text` apply on top of it, as they do in Mermaid. To render many diagrams
/// with one configuration, use `Renderer`, which resolves the configuration once.
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
    let options = render_options(options)?;
    ferrigura::render_with(&text, &config, &options).map_err(to_error)
}

/// A renderer that resolves its configuration once, for many renders.
///
/// `new Renderer(config?)` takes the configuration that `render` takes as its second argument, and
/// `renderer.render(text, options?)` renders a diagram with it, as `render` does. The configuration is
/// merged onto the defaults when the renderer is made, and the typed view of each diagram type is
/// resolved at its first render; a render then merges only the diagram's own front matter and
/// directives. The output is the same as `render` gives with the same configuration. A renderer lives
/// as long as JavaScript holds it.
#[napi]
pub struct Renderer {
    inner: ferrigura::Renderer,
}

#[napi]
impl Renderer {
    /// Resolves the configuration of a renderer. A value of the wrong type is reported by the render
    /// that reads it, as `render` reports it.
    #[napi(constructor)]
    pub fn new(config: Option<serde_json::Value>) -> Self {
        let config = config.map_or_else(Value::empty_object, Value::from);
        Self {
            inner: ferrigura::Renderer::new(&config),
        }
    }

    /// Renders a diagram to an SVG document, with the configuration of this renderer.
    ///
    /// # Errors
    ///
    /// Throws as `render` does: if the text is not a diagram that ferrigura renders, if it does not
    /// parse, if the configuration or the text holds a value of the wrong type, or if the colour
    /// scheme or the id is not valid.
    #[napi]
    pub fn render(&self, text: String, options: Option<RenderOptionsInput>) -> Result<String> {
        let options = render_options(options)?;
        self.inner.render_with(&text, &options).map_err(to_error)
    }
}

/// The render options of the addon's input.
fn render_options(input: Option<RenderOptionsInput>) -> Result<RenderOptions> {
    Ok(match input {
        Some(input) => RenderOptions {
            id: input.id,
            color_scheme: input
                .color_scheme
                .as_deref()
                .map(colour_scheme)
                .transpose()?,
            now: None,
            git_graph_seed: None,
        },
        None => RenderOptions::default(),
    })
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

fn to_error(error: ferrigura::Error) -> Error {
    Error::new(Status::GenericFailure, error.to_string())
}
