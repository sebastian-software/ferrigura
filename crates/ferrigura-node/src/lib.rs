//! The `ferrigura` Node addon, Node.js only; there is no WebAssembly build.
//!
//! `render(text, config?, options?, fonts?)` returns the SVG of a Mermaid diagram, with the options of the
//! `ferrigura` crate. `Renderer` resolves a configuration once, for many renders:
//! `new Renderer(config?, fonts?)`, then `renderer.render(text, options?)`. Both give the same SVG for the
//! same configuration, byte for byte.
//!
//! The crate has one cargo feature per diagram type, named as its constructor in
//! `ferrigura::diagrams`, and `all`, the default. A build with fewer features draws fewer types and
//! links only those (decision 0020); a text of another type is refused as not registered.

use ferrigura::diagrams::{self, Plugin};
use ferrigura::{Animation, ColorScheme, FontStyle, FontWeight, RenderOptions, RendererBuilder};
use ferrigura_model::value::Value;
use napi::bindgen_prelude::Uint8Array;
use napi::{Error, Result, Status};
use napi_derive::napi;

/// One host-supplied face. The JavaScript facade reads local files before calling the addon.
#[napi(object)]
pub struct FontInput {
    /// Explicit family name, including spaces when needed.
    pub family: String,
    /// Font bytes, copied into the renderer during construction.
    pub data: Uint8Array,
    /// Integer CSS weight from 1 through 1000. Defaults to 400.
    pub weight: Option<f64>,
    /// `normal` (default) or `italic`.
    pub style: Option<String>,
}

/// Renderer-scoped font registration, separate from Mermaid configuration and SVG options.
#[napi(object)]
pub struct FontOptionsInput {
    /// Faces registered once before rendering. The first family is the default unless configured.
    pub fonts: Option<Vec<FontInput>>,
    /// Registered fallback family names, in order. No files are discovered or downloaded.
    pub fallbacks: Option<Vec<String>>,
}

/// The options that are not Mermaid configuration.
#[napi(object)]
pub struct RenderOptionsInput {
    /// Optional draw animation. An empty object enables the default speed and source order.
    pub animation: Option<AnimationInput>,
    /// Mermaid-compatible geometry when true. False (default) enables optimized flowchart routing
    /// and compact sequence participant boxes. Font, color and readability fixes always apply.
    pub compat: Option<bool>,
    /// The id of the SVG element, which scopes its styles. By default it is derived from the text and
    /// the configuration, so the same input gives the same id.
    pub id: Option<String>,
    /// The colors: `light`, `dark`, or `auto` to follow the viewer's color scheme. By default the
    /// configured theme decides.
    pub color_scheme: Option<String>,
}

/// Options for one-time, script-free SVG draw animation.
/// Supports flowcharts, Gantt, Git graphs, mind maps, pie charts, and XY charts.
#[napi(object)]
pub struct AnimationInput {
    /// Positive, finite multiplier of 700 SVG user units per second on average. Defaults to 1.
    /// Each connection, bar, or sector eases in and out independently.
    pub speed: Option<f64>,
    /// `source` (default) draws one line at a time; `flow` draws branches in parallel.
    pub order: Option<String>,
}

/// Renders a diagram to an SVG document.
///
/// `config` is the configuration that `mermaid.initialize` takes, as an object. Front matter and
/// `%%{init}%%` directives in `text` apply on top of it, as they do in Mermaid. To render many diagrams
/// with one configuration, use `Renderer`, which resolves the configuration once.
///
/// # Errors
///
/// Throws if the text is not a diagram that ferrigura renders or of a type this build leaves out, if it
/// does not parse, if the configuration holds a value of the wrong type, or if the color scheme or the
/// id is not valid.
#[napi]
pub fn render(
    text: String,
    config: Option<serde_json::Value>,
    options: Option<RenderOptionsInput>,
    fonts: Option<FontOptionsInput>,
) -> Result<String> {
    let config = config.map_or_else(Value::empty_object, Value::from);
    let options = render_options(options)?;
    font_builder(builder(&config), fonts)?
        .shared()
        .render_with(&text, &options)
        .map_err(to_error)
}

/// A renderer that resolves its configuration once, for many renders.
///
/// `new Renderer(config?, fonts?)` takes the configuration that `render` takes as its second argument, and
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
    ///
    /// # Errors
    /// Throws immediately if supplied font bytes or face metadata are invalid.
    #[napi(constructor)]
    pub fn new(config: Option<serde_json::Value>, fonts: Option<FontOptionsInput>) -> Result<Self> {
        let config = config.map_or_else(Value::empty_object, Value::from);
        Ok(Self {
            inner: font_builder(builder(&config), fonts)?.build(),
        })
    }

    /// Renders a diagram to an SVG document, with the configuration of this renderer.
    ///
    /// # Errors
    ///
    /// Throws as `render` does: if the text is not a diagram that ferrigura renders, if it does not
    /// parse, if the configuration or the text holds a value of the wrong type, or if the color
    /// scheme or the id is not valid.
    #[napi]
    pub fn render(&self, text: String, options: Option<RenderOptionsInput>) -> Result<String> {
        let options = render_options(options)?;
        self.inner.render_with(&text, &options).map_err(to_error)
    }
}

/// Applies only owned bytes: file and network loading never enter the addon.
fn font_builder(
    mut builder: RendererBuilder,
    input: Option<FontOptionsInput>,
) -> Result<RendererBuilder> {
    if let Some(input) = input {
        for font in input.fonts.unwrap_or_default() {
            let weight = font.weight.unwrap_or(400.0);
            if !weight.is_finite() || weight.fract() != 0.0 || !(1.0..=1000.0).contains(&weight) {
                return Err(Error::new(
                    Status::InvalidArg,
                    "font weight must be an integer from 1 through 1000",
                ));
            }
            let style = match font.style.as_deref().unwrap_or("normal") {
                "normal" => FontStyle::Normal,
                "italic" => FontStyle::Italic,
                _ => {
                    return Err(Error::new(
                        Status::InvalidArg,
                        "font style must be normal or italic",
                    ));
                }
            };
            builder = builder
                .font_face(
                    &font.family,
                    font.data.to_vec(),
                    FontWeight::Number(weight as u16),
                    style,
                )
                .map_err(|error| {
                    Error::new(
                        Status::InvalidArg,
                        format!("cannot register font {:?}: {error}", font.family),
                    )
                })?;
        }
        if let Some(fallbacks) = input.fallbacks {
            builder = builder.font_fallbacks(fallbacks);
        }
    }
    Ok(builder)
}

/// The render options of the addon's input.
fn render_options(input: Option<RenderOptionsInput>) -> Result<RenderOptions> {
    Ok(match input {
        Some(input) => RenderOptions {
            animation: input
                .animation
                .map(|input| {
                    let mut animation = Animation::new(input.speed.unwrap_or(1.0))
                        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
                    animation.order = input
                        .order
                        .as_deref()
                        .map(str::parse)
                        .transpose()
                        .map_err(|message| Error::new(Status::InvalidArg, message))?
                        .unwrap_or_default();
                    Ok::<_, Error>(animation)
                })
                .transpose()?,
            compat: input.compat.unwrap_or(false),
            id: input.id,
            color_scheme: input
                .color_scheme
                .as_deref()
                .map(color_scheme)
                .transpose()?,
            now: None,
            git_graph_seed: None,
            random_seed: None,
        },
        None => RenderOptions::default(),
    })
}

fn color_scheme(name: &str) -> Result<ColorScheme> {
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

#[cfg(not(any(
    feature = "pie",
    feature = "info",
    feature = "flowchart",
    feature = "timeline",
    feature = "journey",
    feature = "git_graph",
    feature = "er",
    feature = "class",
    feature = "sequence",
    feature = "state",
    feature = "gantt",
    feature = "mindmap",
    feature = "block",
    feature = "packet",
    feature = "xy_chart",
    feature = "sankey",
    feature = "quadrant_chart",
    feature = "requirement",
    feature = "c4",
    feature = "kanban",
    feature = "architecture",
    feature = "treemap",
    feature = "ishikawa",
    feature = "event_modeling",
    feature = "agentflow",
    feature = "cynefin",
    feature = "radar",
    feature = "railroad",
    feature = "swimlanes",
    feature = "tree_view",
    feature = "use_case",
    feature = "venn",
    feature = "wardley"
)))]
compile_error!(
    "ferrigura-node draws no diagram type: enable at least one feature, or the default `all`"
);

/// The plugins of this build, one per enabled feature.
#[expect(
    clippy::vec_init_then_push,
    reason = "a `vec!` literal cannot hold `#[cfg]` elements"
)]
fn plugins() -> Vec<Plugin> {
    let mut plugins = Vec::new();
    #[cfg(feature = "pie")]
    plugins.push(diagrams::pie());
    #[cfg(feature = "info")]
    plugins.push(diagrams::info());
    #[cfg(feature = "flowchart")]
    plugins.push(diagrams::flowchart());
    #[cfg(feature = "timeline")]
    plugins.push(diagrams::timeline());
    #[cfg(feature = "journey")]
    plugins.push(diagrams::journey());
    #[cfg(feature = "git_graph")]
    plugins.push(diagrams::git_graph());
    #[cfg(feature = "er")]
    plugins.push(diagrams::er());
    #[cfg(feature = "class")]
    plugins.push(diagrams::class());
    #[cfg(feature = "sequence")]
    plugins.push(diagrams::sequence());
    #[cfg(feature = "state")]
    plugins.push(diagrams::state());
    #[cfg(feature = "gantt")]
    plugins.push(diagrams::gantt());
    #[cfg(feature = "mindmap")]
    plugins.push(diagrams::mindmap());
    #[cfg(feature = "block")]
    plugins.push(diagrams::block());
    #[cfg(feature = "packet")]
    plugins.push(diagrams::packet());
    #[cfg(feature = "xy_chart")]
    plugins.push(diagrams::xy_chart());
    #[cfg(feature = "sankey")]
    plugins.push(diagrams::sankey());
    #[cfg(feature = "quadrant_chart")]
    plugins.push(diagrams::quadrant_chart());
    #[cfg(feature = "requirement")]
    plugins.push(diagrams::requirement());
    #[cfg(feature = "c4")]
    plugins.push(diagrams::c4());
    #[cfg(feature = "kanban")]
    plugins.push(diagrams::kanban());
    #[cfg(feature = "architecture")]
    plugins.push(diagrams::architecture());
    #[cfg(feature = "treemap")]
    plugins.push(diagrams::treemap());
    #[cfg(feature = "ishikawa")]
    plugins.push(diagrams::ishikawa());
    #[cfg(feature = "event_modeling")]
    plugins.push(diagrams::event_modeling());
    #[cfg(feature = "agentflow")]
    plugins.push(diagrams::agentflow());
    #[cfg(feature = "cynefin")]
    plugins.push(diagrams::cynefin());
    #[cfg(feature = "radar")]
    plugins.push(diagrams::radar());
    #[cfg(feature = "railroad")]
    plugins.push(diagrams::railroad());
    #[cfg(feature = "swimlanes")]
    plugins.push(diagrams::swimlanes());
    #[cfg(feature = "tree_view")]
    plugins.push(diagrams::tree_view());
    #[cfg(feature = "use_case")]
    plugins.push(diagrams::use_case());
    #[cfg(feature = "venn")]
    plugins.push(diagrams::venn());
    #[cfg(feature = "wardley")]
    plugins.push(diagrams::wardley());
    plugins
}

/// A builder with the plugins of this build.
fn builder(config: &Value) -> RendererBuilder {
    plugins()
        .into_iter()
        .fold(ferrigura::Renderer::builder(config), RendererBuilder::with)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Which features this build has, in the order of `plugins`.
    const FEATURES: [bool; 33] = [
        cfg!(feature = "pie"),
        cfg!(feature = "info"),
        cfg!(feature = "flowchart"),
        cfg!(feature = "timeline"),
        cfg!(feature = "journey"),
        cfg!(feature = "git_graph"),
        cfg!(feature = "er"),
        cfg!(feature = "class"),
        cfg!(feature = "sequence"),
        cfg!(feature = "state"),
        cfg!(feature = "gantt"),
        cfg!(feature = "mindmap"),
        cfg!(feature = "block"),
        cfg!(feature = "packet"),
        cfg!(feature = "xy_chart"),
        cfg!(feature = "sankey"),
        cfg!(feature = "quadrant_chart"),
        cfg!(feature = "requirement"),
        cfg!(feature = "c4"),
        cfg!(feature = "kanban"),
        cfg!(feature = "architecture"),
        cfg!(feature = "treemap"),
        cfg!(feature = "ishikawa"),
        cfg!(feature = "event_modeling"),
        cfg!(feature = "agentflow"),
        cfg!(feature = "cynefin"),
        cfg!(feature = "radar"),
        cfg!(feature = "railroad"),
        cfg!(feature = "swimlanes"),
        cfg!(feature = "tree_view"),
        cfg!(feature = "use_case"),
        cfg!(feature = "venn"),
        cfg!(feature = "wardley"),
    ];

    #[test]
    fn one_plugin_per_enabled_feature() {
        let plugins = plugins();
        let kinds: HashSet<_> = plugins.iter().map(Plugin::kind).collect();
        assert_eq!(kinds.len(), plugins.len());
        assert_eq!(plugins.len(), FEATURES.iter().filter(|on| **on).count());
    }

    #[cfg(feature = "sequence")]
    #[test]
    fn draws_a_type_whose_feature_is_on() {
        let svg = render(
            "sequenceDiagram\n  A->>B: Hi\n".to_owned(),
            None,
            None,
            None,
        )
        .unwrap();
        assert!(svg.starts_with("<svg "));
    }

    #[cfg(not(feature = "pie"))]
    #[test]
    fn refuses_a_type_whose_feature_is_off() {
        let error = render("pie\n  \"Dogs\" : 1\n".to_owned(), None, None, None).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("pie"), "{message}");
        assert!(message.contains("not registered"), "{message}");
    }
}
