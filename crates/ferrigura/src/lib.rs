#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hasher;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ferrigura_model::config::{MermaidConfig, SiteConfig, typed_top};
use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::flowchart;
use ferrigura_render::unified::Section;
use ferrigura_syntax::{ParseError, config, detect};

pub mod diagrams;

use diagrams::Plugin;

/// The diagram types, as the detector names them: what a [`diagrams::Plugin`] draws.
pub use ferrigura_syntax::DiagramType;

/// The configuration and the values of Mermaid's `initialize` object, front matter and directives.
pub use ferrigura_model::value::Value;
pub use ferrigura_render::RenderError as Error;
use ferrigura_render::flowchart::policy::FlowchartRouting;

/// The color scheme of the output.
///
/// The configuration decides the colors by default: the configured theme, as Mermaid does. `Light`
/// and `Dark` draw with the shipped light and dark themes, and `Auto` writes both as `light-dark()`
/// values, so that one SVG follows the `color-scheme` of the page it is inlined in, or the viewer's
/// scheme as a standalone file. The user's `themeVariables` apply to both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// The light theme, `redux-color`.
    Light,
    /// The dark theme, `redux-dark-color`.
    Dark,
    /// Both themes, as `light-dark()` values.
    Auto,
}

/// The options of a render that are not Mermaid configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderOptions {
    /// Use Mermaid-compatible geometry instead of Ferrigura's optimizations. Defaults to false:
    /// flowchart ports are centered with label-aware spacing and sequence participant boxes fit
    /// their labels. Font, color and readability fixes apply in both modes.
    pub compat: bool,
    /// The id of the SVG element, which scopes its styles. It must start with an ASCII letter and
    /// contain only ASCII letters, digits, `-` and `_`. By default the id is derived from the text
    /// and the configuration, so the same input gives the same id.
    pub id: Option<String>,
    /// The color scheme. `None` keeps the colors of the configured theme.
    pub color_scheme: Option<ColorScheme>,
    /// The current time, in milliseconds since the epoch, for the dates that depend on it (the
    /// gantt diagram's today marker and dates without a day). `None` reads the system clock.
    pub now: Option<i64>,
    /// The seed of the ids a gitGraph generates for its commits, merges and cherry-picks that have
    /// no `id:`. `None` derives them from the sequence number. The geometry harness gives the
    /// reference's seed, so that the ids match the reference's; it is not part of the interface.
    #[doc(hidden)]
    pub git_graph_seed: Option<u32>,
    /// The seed of the random numbers a venn layout draws when it has eight areas or more (the
    /// reference draws them from `Math.random`). `None` gives the venn renderer's default seed. The
    /// geometry harness gives the reference's seed, so that the draws match; it is not part of the
    /// interface.
    #[doc(hidden)]
    pub random_seed: Option<u32>,
}

/// A renderer, with the site configuration resolved once.
///
/// [`Renderer::new`] takes the configuration the user would pass to `initialize`. The renderer holds
/// it merged onto the defaults and typed, and the light and the dark theme with the user's
/// `themeVariables`, as typed tokens. Its renders merge only the diagram's own front matter and directives on
/// top, and only when there are any: a text without them is drawn with the resolved configuration as
/// it is. The typed view of a diagram type is resolved at its first render and kept after that.
///
/// The configuration of a renderer does not change once it is made, and the renderer is `Send` and
/// `Sync`, so one renderer can serve renders on several threads at once. For the same configuration,
/// its output is the output of [`render_with`], byte for byte.
///
/// ```
/// use ferrigura::{RenderOptions, Renderer};
/// use ferrigura::Value;
///
/// let renderer = Renderer::new(&Value::empty_object());
/// let svg = renderer.render("pie\n  \"Dogs\" : 386\n  \"Cats\" : 85\n")?;
/// assert!(svg.starts_with("<svg "));
/// let svg = renderer.render_with(
///     "pie\n  \"Dogs\" : 386\n",
///     &RenderOptions { id: Some("pets".to_owned()), ..RenderOptions::default() },
/// )?;
/// assert!(svg.contains("id=\"pets\""));
/// # Ok::<(), ferrigura::Error>(())
/// ```
#[derive(Debug)]
pub struct Renderer {
    /// The site configuration, merged onto the defaults.
    site: SiteConfig,
    /// The configuration as it serializes, which the derived id hashes.
    configuration: String,
    /// The user's theme overrides, which the two schemes below are computed with.
    overrides: Value,
    /// The tokens of `redux-color` with the overrides: the light scheme.
    light_tokens: ThemeTokens,
    /// The tokens of `redux-dark-color` with the overrides: the dark scheme.
    dark_tokens: ThemeTokens,
    /// The views of the diagram types, made at their first render. A view that cannot be resolved
    /// keeps its error, which the render reports as it always has.
    views: Mutex<HashMap<DiagramType, Result<Arc<View>, String>>>,
    /// The diagram types this renderer draws, one plugin per kind.
    plugins: Vec<Plugin>,
}

impl Renderer {
    /// Resolves the configuration of a renderer that draws every diagram type. `config` is what
    /// the user would pass to `initialize`, as a [`Value`]; an empty object gives the defaults.
    ///
    /// Making a renderer does not fail. A value of the wrong type is reported by the render that reads
    /// it, as [`render`] reports it.
    #[must_use]
    pub fn new(config: &Value) -> Self {
        Self::builder(config).with_all().build()
    }

    /// A builder for a renderer that draws only the diagram types it registers. See
    /// [`RendererBuilder`].
    #[must_use]
    pub fn builder(config: &Value) -> RendererBuilder {
        RendererBuilder {
            config: config.clone(),
            plugins: Vec::new(),
        }
    }

    /// The plugin of a diagram type, or the error of a type this renderer does not draw.
    fn plugin(&self, kind: DiagramType) -> Result<Plugin, RenderError> {
        self.plugins
            .iter()
            .find(|plugin| plugin.kind() == kind)
            .copied()
            .ok_or_else(|| {
                RenderError::Unsupported(format!(
                    "the diagram type `{}` is not registered in this renderer",
                    kind.config_key()
                ))
            })
    }

    /// Renders a diagram to an SVG document, with the default options. See [`render`].
    ///
    /// # Errors
    ///
    /// Returns the errors of [`render`].
    pub fn render(&self, text: &str) -> Result<String, RenderError> {
        self.render_with(text, &RenderOptions::default())
    }

    /// Renders a diagram with the options given. See [`render_with`].
    ///
    /// # Errors
    ///
    /// Returns the errors of [`render_with`].
    pub fn render_with(&self, text: &str, options: &RenderOptions) -> Result<String, RenderError> {
        let Some(kind) = detect(text) else {
            return Err(RenderError::Parse(ParseError {
                line: 1,
                column: 1,
                message: "no diagram type detected".to_owned(),
            }));
        };
        let plugin = self.plugin(kind)?;
        let prepared = self.prepare(text, options, plugin)?;
        plugin.render(text, &prepared, options)
    }

    /// What a render needs besides its diagram, as `mermaidAPI` resolves it before it calls the
    /// diagram's renderer: the id, the configuration in the diagram's scope, and the tokens of the
    /// schemes that the render draws.
    ///
    /// A text without front matter or directives uses the view of its diagram type and the tokens
    /// of the renderer as they are. Otherwise the directive is merged on top here, and the tokens are
    /// computed again only when the directive changes the user's overrides.
    fn prepare(
        &self,
        text: &str,
        options: &RenderOptions,
        plugin: Plugin,
    ) -> Result<Prepared<'_>, RenderError> {
        let id = match &options.id {
            Some(id) if valid_id(id) => id.clone(),
            Some(id) => return Err(RenderError::Options(format!("invalid id {id:?}"))),
            None => derived_id(text, &self.configuration),
        };
        let directive = config::read(text)?;
        let scheme = options.color_scheme;
        if is_empty_object(&directive) {
            return Ok(Prepared {
                id,
                view: self.view(plugin)?,
                light_tokens: Cow::Borrowed(&self.light_tokens),
                dark_tokens: Cow::Borrowed(&self.dark_tokens),
                scheme,
            });
        }
        let view = View::resolve(&self.site, plugin, &directive).map_err(RenderError::Config)?;
        let overrides = self.site.user_theme_variables(&directive);
        let (light_tokens, dark_tokens) = if overrides == self.overrides {
            (
                Cow::Borrowed(&self.light_tokens),
                Cow::Borrowed(&self.dark_tokens),
            )
        } else {
            (
                Cow::Owned(scheme_tokens(
                    Theme::ReduxColor,
                    matches!(scheme, Some(ColorScheme::Light | ColorScheme::Auto)),
                    &overrides,
                )),
                Cow::Owned(scheme_tokens(
                    Theme::ReduxDarkColor,
                    matches!(scheme, Some(ColorScheme::Dark | ColorScheme::Auto)),
                    &overrides,
                )),
            )
        };
        Ok(Prepared {
            id,
            view: Arc::new(view),
            light_tokens,
            dark_tokens,
            scheme,
        })
    }

    /// The view of a diagram type with no directive, resolved at its first render.
    fn view(&self, plugin: Plugin) -> Result<Arc<View>, RenderError> {
        let kind = plugin.kind();
        let known = lock(&self.views).get(&kind).cloned();
        let resolved = match known {
            Some(resolved) => resolved,
            None => {
                let resolved =
                    View::resolve(&self.site, plugin, &Value::empty_object()).map(Arc::new);
                lock(&self.views).entry(kind).or_insert(resolved).clone()
            }
        };
        resolved.map_err(RenderError::Config)
    }
}

/// Makes a [`Renderer`] that draws only the diagram types it registers.
///
/// Each diagram type is a [`diagrams::Plugin`]; a program that registers only the types it draws
/// links only their parsers, renderers and layout engines, since nothing else refers to the rest.
/// [`Renderer::new`] registers them all.
///
/// ```
/// use ferrigura::{Renderer, Value, diagrams};
///
/// let renderer = Renderer::builder(&Value::empty_object())
///     .with(diagrams::pie())
///     .with(diagrams::sequence())
///     .build();
/// assert!(renderer.render("pie\n  \"Dogs\" : 386\n").is_ok());
/// assert!(renderer.render("flowchart LR\n  a --> b\n").is_err());
/// ```
#[derive(Debug)]
pub struct RendererBuilder {
    /// The configuration the renderer is made with.
    config: Value,
    /// The plugins registered so far, one per kind.
    plugins: Vec<Plugin>,
}

impl RendererBuilder {
    /// Registers a diagram type. A plugin of a kind that is registered already replaces it.
    #[must_use]
    pub fn with(mut self, plugin: Plugin) -> Self {
        self.plugins.retain(|known| known.kind() != plugin.kind());
        self.plugins.push(plugin);
        self
    }

    /// Registers every diagram type, as [`Renderer::new`] does.
    #[must_use]
    pub fn with_all(self) -> Self {
        diagrams::all().into_iter().fold(self, Self::with)
    }

    /// The renderer of this configuration and these plugins, kept for the next call. A renderer that
    /// is kept already is returned; otherwise it is made as [`build`](Self::build) makes it and kept,
    /// in the place of the least recently used one once a handful are kept. [`render`] uses it with
    /// every plugin; a program that renders one diagram at a time with the same configuration uses
    /// it so that the configuration is not resolved again. The order of registration does not matter.
    #[must_use]
    pub fn shared(self) -> Arc<Renderer> {
        let mut kinds: Vec<DiagramType> = self.plugins.iter().map(Plugin::kind).collect();
        kinds.sort_unstable_by(|a, b| a.config_key().cmp(b.config_key()));
        if let Some(renderer) = kept(&self.config, &kinds) {
            return renderer;
        }
        // Two threads that miss at once keep two equal renderers for a while; that costs memory only.
        let config = self.config.clone();
        let renderer = Arc::new(self.build());
        let mut cache = lock(&SHORTHAND);
        if cache.len() == SHORTHAND_CAPACITY {
            cache.remove(0);
        }
        cache.push(Shared {
            config,
            kinds,
            renderer: Arc::clone(&renderer),
        });
        renderer
    }

    /// Resolves the configuration and makes the renderer. This does not fail; see [`Renderer::new`].
    #[must_use]
    pub fn build(self) -> Renderer {
        let site = SiteConfig::new(Some(&self.config));
        let overrides = site.user_theme_variables(&Value::empty_object());
        Renderer {
            light_tokens: ThemeTokens::from_value(
                &Theme::ReduxColor.variables_shared(Some(&overrides)),
            ),
            dark_tokens: ThemeTokens::from_value(
                &Theme::ReduxDarkColor.variables_shared(Some(&overrides)),
            ),
            configuration: serde_json::to_string(&self.config).unwrap_or_default(),
            overrides,
            site,
            views: Mutex::new(HashMap::new()),
            plugins: self.plugins,
        }
    }
}

/// Renders a diagram to an SVG document.
///
/// `config` is the configuration the user would pass to `initialize`, as a [`Value`]; an empty object
/// gives the defaults. Front matter and `%%{init}%%` directives in `text` apply on top of it, as they
/// do in Mermaid. This is a shorthand for [`Renderer`]: the renderer of `config` is kept, so a render
/// with the same configuration does not resolve it again.
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
    Renderer::builder(config)
        .with_all()
        .shared()
        .render_with(text, options)
}

/// The view of a diagram type, its configuration in the diagram's scope and typed, the tokens of its
/// configured theme, and the globals the renderers read.
#[derive(Debug)]
struct View {
    /// The configuration in the diagram's scope, untyped: the facade reads from it what the schema
    /// does not type as the reference reads it (packet's `bitsPerRow`, the treemap's settings, the
    /// sequence's `wrap`).
    effective: Value,
    /// The sections of the same configuration that the diagram type's renderer reads, typed; the
    /// other sections keep their defaults, and the top level is read into `globals`.
    config: MermaidConfig,
    /// The tokens of the configured theme: the `themeVariables` of the configuration.
    configured_tokens: ThemeTokens,
    /// The global options of the configuration, as the view resolves them.
    globals: Globals,
}

impl View {
    /// The view of a diagram type, with the directive merged on top of the site configuration.
    ///
    /// # Errors
    ///
    /// Returns the message of the error when the configuration cannot be read into its types.
    fn resolve(site: &SiteConfig, plugin: Plugin, directive: &Value) -> Result<Self, String> {
        let kind = plugin.kind();
        let effective = site.effective(Some(kind.config_key()), directive);
        let mut top = typed_top(&effective).map_err(|error| error.to_string())?;
        // `getData` sets the mindmap's layout to cose-bilkent unless the top-level `layout` is set by the
        // user (`mindmapDb.ts` 361-367): the schema's default, `elk`, does not reach a mindmap.
        if matches!(kind, DiagramType::Mindmap)
            && !user_sets_layout(Some(site.settings()), directive)
        {
            top.layout = "cose-bilkent".to_owned();
        }
        // Only the sections this diagram type reads are typed, so a program that registers few
        // plugins links the reading code of their sections only (decision 0020).
        let mut config = MermaidConfig::default();
        plugin
            .read(&effective, &mut config)
            .map_err(|error| error.to_string())?;
        let configured_tokens = ThemeTokens::from_value(
            effective
                .get("themeVariables")
                .unwrap_or(&Value::empty_object()),
        );
        Ok(Self {
            globals: Globals::of_top(&top),
            configured_tokens,
            effective,
            config,
        })
    }
}

/// What a render needs besides its diagram: see [`Renderer::prepare`].
struct Prepared<'a> {
    id: String,
    view: Arc<View>,
    light_tokens: Cow<'a, ThemeTokens>,
    dark_tokens: Cow<'a, ThemeTokens>,
    scheme: Option<ColorScheme>,
}

impl Prepared<'_> {
    /// The theme of the render as typed tokens: the configured theme, or the scheme that the color
    /// scheme chooses, with the light scheme as the primary one when the render draws both.
    fn theme(&self) -> ThemeView<'_> {
        match self.scheme {
            None => ThemeView {
                light: &self.view.configured_tokens,
                dark: None,
            },
            Some(ColorScheme::Light) => ThemeView {
                light: &self.light_tokens,
                dark: None,
            },
            Some(ColorScheme::Dark) => ThemeView {
                light: &self.dark_tokens,
                dark: None,
            },
            Some(ColorScheme::Auto) => ThemeView {
                light: &self.light_tokens,
                dark: Some(&self.dark_tokens),
            },
        }
    }
}

/// Whether the top-level `layout` is set by the user: in the site configuration (`initialize`) or in
/// the text's directives and front matter. A diagram-scoped `layout` does not count
/// (`userDefinedConfig`, `config.ts` 305-317, `mindmapDb.ts` 361-362).
fn user_sets_layout(site: Option<&Value>, directive: &Value) -> bool {
    let set = |value: &Value| value.get("layout").is_some_and(|layout| !layout.is_null());
    site.is_some_and(set) || set(directive)
}

/// The tokens of a color scheme, when the render draws it. A scheme that is not drawn is not
/// computed: the configured theme decides the colors then, and the scheme gives no tokens.
fn scheme_tokens(theme: Theme, drawn: bool, overrides: &Value) -> ThemeTokens {
    if drawn {
        ThemeTokens::from_value(&theme.variables_shared(Some(overrides)))
    } else {
        ThemeTokens::from_value(&Value::empty_object())
    }
}

/// Whether a value is an object without keys, which is what a text without directives gives.
fn is_empty_object(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.is_empty())
}

/// `autoWrap()` of the sequence diagram after `init`: a truthy top-level `wrap` is copied into
/// `sequence.wrap` (`sequenceDiagram.ts` 21-23), which is then read, else `false`.
fn sequence_wrap(effective: &Value) -> bool {
    matches!(effective.get("wrap"), Some(Value::Bool(true)))
        || matches!(
            effective
                .get("sequence")
                .and_then(|section| section.get("wrap")),
            Some(Value::Bool(true))
        )
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
    let renderer = Renderer::builder(config).with_all().shared();
    let prepared = renderer.prepare(text, options, diagrams::flowchart())?;
    let stages = with_flowchart(text, &prepared, options.compat, flowchart::elk_stages)?;
    Ok(serde_json::json!({
        "elkInput": stages.input,
        "elkOutput": stages.output,
    }))
}

/// Parses a flowchart and hands it, with its sections, globals and theme, to `step`.
fn with_flowchart<T>(
    text: &str,
    prepared: &Prepared<'_>,
    compat: bool,
    step: impl FnOnce(&Context<'_, FlowchartDiagram, Section<'_>>) -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    let diagram = ferrigura_syntax::flowchart::parse(text)
        .map_err(|ferrigura_syntax::flowchart::FlowchartError::Syntax(error)| error)?;
    let mut section = Section::of(&prepared.view.config);
    section.flowchart_routing = if compat {
        FlowchartRouting::Mermaid
    } else {
        FlowchartRouting::Adaptive
    };
    step(&Context {
        diagram: &diagram,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id: &prepared.id,
    })
}

/// The renderers of [`RendererBuilder::shared`], which [`render`] and [`render_with`] use with every
/// plugin, each with the configuration and the kinds it was made from, least recently used first.
static SHORTHAND: Mutex<Vec<Shared>> = Mutex::new(Vec::new());

/// How many renderers the shorthand keeps. One that is not kept is made again at its next render;
/// the output does not depend on it.
const SHORTHAND_CAPACITY: usize = 8;

/// A renderer the shorthand keeps, with what it was made from.
struct Shared {
    config: Value,
    /// The kinds the renderer draws, sorted, so that the order of registration does not matter.
    kinds: Vec<DiagramType>,
    renderer: Arc<Renderer>,
}

/// The renderer that the shorthand keeps for a configuration and a set of kinds, which becomes the
/// most recently used.
fn kept(config: &Value, kinds: &[DiagramType]) -> Option<Arc<Renderer>> {
    let mut cache = lock(&SHORTHAND);
    let at = cache
        .iter()
        .position(|entry| entry.kinds == kinds && identical(&entry.config, config))?;
    let entry = cache.remove(at);
    let renderer = Arc::clone(&entry.renderer);
    cache.push(entry);
    Some(renderer)
}

/// Whether two configurations are the same value, number by number: `0` and `-0` are not, as the
/// derived id tells them apart, and a `NaN` is the same `NaN`.
fn identical(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| identical(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((key_a, a), (key_b, b))| key_a == key_b && identical(a, b))
        }
        _ => false,
    }
}

/// The guard of a cache. A panic in another render does not keep it shut: the caches hold nothing a
/// panic could leave half written.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether an id can scope the styles of a diagram: a CSS identifier that needs no escaping.
fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The id a diagram gets when none is given: `ferrigura-` and a 64-bit FNV-1a hash of the text and the
/// configuration as it serializes, so the same input always gives the same id.
fn derived_id(text: &str, configuration: &str) -> String {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn a_renderer_can_serve_renders_on_several_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Renderer>();
    }

    #[test]
    fn configurations_are_the_same_only_number_by_number() {
        assert!(identical(&Value::Number(0.0), &Value::Number(0.0)));
        assert!(!identical(&Value::Number(0.0), &Value::Number(-0.0)));
        assert!(identical(
            &Value::Number(f64::NAN),
            &Value::Number(f64::NAN)
        ));
        assert!(!identical(
            &Value::String("NaN".to_owned()),
            &Value::Number(f64::NAN)
        ));
        let object = Value::Object(BTreeMap::from([("a".to_owned(), Value::Number(-0.0))]));
        assert!(!identical(&object, &Value::empty_object()));
    }
}
