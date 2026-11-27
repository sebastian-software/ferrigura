#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::collections::HashMap;
use std::hash::Hasher;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use ferrigura_model::config::{MermaidConfig, ResolvedThemeVariables, SiteConfig, typed_top};
use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::design::{self, Tone};
use ferrigura_render::flowchart;
use ferrigura_render::unified::Section;
use ferrigura_syntax::{ParseError, config, detect_prepared, preprocess};

pub mod diagrams;

use diagrams::Plugin;

pub use ferrigura_text::{FontRegistry, FontStyle, FontWeight};

pub use ferrigura_render::animation::{Animation, AnimationOrder};
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
    /// Draw in the design mode: ferrigura's own look, with hairline outlines, color by the role of
    /// a node, rounded corners and soft shadows, and with its own measures: a node grows to give its
    /// edges room, and spacings and group paddings are wider. Flowcharts, state diagrams and class
    /// diagrams support it; other diagram types return an option error, and so does the combination
    /// with `compat`. The user's `themeVariables` and `classDef` styles still apply on top.
    pub design: bool,
    /// Optional one-time SVG draw animation for flowcharts, Gantt, mind maps, pie charts, and XY
    /// charts. `None` leaves the SVG unchanged.
    /// The default draws one line at a time, averaging 700 SVG units per second times its speed,
    /// with independent easing for each section. Other diagram types return an option error.
    pub animation: Option<Animation>,
    /// The id of the SVG element, which scopes its styles. It must start with an ASCII letter and
    /// contain only ASCII letters, digits, `-` and `_`. By default the id is derived from the text
    /// and the configuration, so the same input gives the same id.
    pub id: Option<String>,
    /// The color scheme. `None` keeps the colors of the configured theme.
    pub color_scheme: Option<ColorScheme>,
    /// The current time, in milliseconds since the epoch, for the dates that depend on it (the
    /// gantt diagram's today marker and dates without a day). `None` reads the system clock.
    pub now: Option<i64>,
}

/// A renderer, with the site configuration resolved once.
///
/// [`Renderer::new`] takes the configuration the user would pass to `initialize`. The renderer
/// resolves the site configuration once and caches its computed theme variables and typed theme
/// tokens on first use, sharing them across diagram types. Its renders merge only the diagram's own
/// front matter and directives on top, and only when there are any: a text without them is drawn
/// with the resolved configuration as it is. The typed view of each diagram type is resolved at its
/// first render and kept after that, and so are the views and resolved scheme/design tokens of the
/// last few front matters and directives it rendered.
///
/// For multi-diagram documents or batch builds with site-owned settings (such as a shared `theme`,
/// `themeVariables`, or host fonts registered through [`RendererBuilder`]), construct one
/// `Renderer` and reuse it across all diagrams instead of rebuilding it per diagram.
///
/// The configuration of a renderer does not change once it is made, and the renderer is `Send` and
/// `Sync`, so one renderer can serve renders on several threads at once. For the same configuration,
/// its output is the output of [`render_with`], byte for byte.
///
/// ```
/// use ferrigura::{RenderOptions, Renderer, Value};
///
/// // Resolve site-owned settings once and reuse the renderer across diagrams.
/// let site: Value = serde_json::json!({
///     "theme": "redux-color",
///     "themeVariables": { "primaryColor": "#336699" }
/// }).into();
/// let renderer = Renderer::new(&site);
/// let pie = renderer.render("pie\n  \"Dogs\" : 386\n  \"Cats\" : 85\n")?;
/// assert!(pie.starts_with("<svg "));
/// let flow = renderer.render_with(
///     "flowchart LR\n  A --> B\n",
///     &RenderOptions { id: Some("pets".to_owned()), ..RenderOptions::default() },
/// )?;
/// assert!(flow.contains("id=\"pets\""));
/// # Ok::<(), ferrigura::Error>(())
/// ```
#[derive(Debug)]
pub struct Renderer {
    /// The site configuration, merged onto the defaults.
    site: SiteConfig,
    /// Validated `(maxTextSize, maxEdges)` limits from the site configuration, or the config error
    /// message reported when rendering.
    limits: Result<(usize, usize), String>,
    /// The configuration as it serializes, which the derived id hashes.
    configuration: String,
    /// The user's theme overrides, which the two schemes below are computed with.
    overrides: Value,
    /// The tokens of `redux-color` with the site overrides, resolved lazily and shared with views
    /// whose effective theme variables match.
    light_tokens: OnceLock<Arc<ThemeTokens>>,
    /// The tokens of `redux-dark-color` with the site overrides, resolved lazily and shared with
    /// views whose effective theme variables match.
    dark_tokens: OnceLock<Arc<ThemeTokens>>,
    /// The light tokens of the design mode with the site overrides, resolved at first use.
    design_light_tokens: OnceLock<Arc<ThemeTokens>>,
    /// The dark tokens of the design mode with the site overrides, resolved at first use.
    design_dark_tokens: OnceLock<Arc<ThemeTokens>>,
    /// The views of the diagram types, made at their first render. A view that cannot be resolved
    /// keeps its error, which the render reports as it always has.
    views: Mutex<HashMap<DiagramType, Result<Arc<View>, String>>>,
    /// The views of the last few directives, least recently used first: diagrams that carry the
    /// same front matter or `init` directive share their view and resolved scheme/design tokens. A
    /// view that cannot be resolved is not kept, so its error is resolved again at each render.
    directive_views: Mutex<Vec<DirectiveView>>,
    /// The diagram types this renderer draws, one plugin per kind.
    plugins: Vec<Plugin>,
    /// Explicit host fonts; `None` preserves the bundled defaults.
    fonts: Option<Arc<FontRegistry>>,
    subset_cache: Arc<ferrigura_text::SubsetCache>,
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
            fonts: None,
        }
    }

    /// The plugin of a diagram type, or the error of a type this renderer does not register.
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
        ferrigura_text::with_subset_cache(&self.subset_cache, || {
            self.render_with_cache(text, options)
        })
    }

    fn render_with_cache(
        &self,
        text: &str,
        options: &RenderOptions,
    ) -> Result<String, RenderError> {
        let max_edges = self.limits(text)?;
        let (source, kind) = ferrigura_syntax::prepare_and_detect(text).map_err(|error| {
            ferrigura_syntax::detect_unsupported(text)
                .map_or(RenderError::Parse(error), unsupported)
        })?;
        let plugin = self.plugin(kind)?;
        if options.animation.is_some()
            && !matches!(
                kind,
                DiagramType::Flowchart
                    | DiagramType::Gantt
                    | DiagramType::Mindmap
                    | DiagramType::Pie
                    | DiagramType::XyChart
            )
        {
            return Err(RenderError::Options(
                "draw animation is not supported for this diagram type".to_owned(),
            ));
        }
        if options.design {
            if options.compat {
                return Err(RenderError::Options(
                    "the design mode cannot be combined with compat".to_owned(),
                ));
            }
            if !matches!(
                kind,
                DiagramType::Flowchart | DiagramType::State | DiagramType::Class
            ) {
                return Err(RenderError::Options(
                    "the design mode is not supported for this diagram type".to_owned(),
                ));
            }
        }
        let prepared = self.prepare(text, source, options, plugin, max_edges)?;
        plugin.render(&prepared, options)
    }

    /// Checks the text against `maxTextSize` and returns `maxEdges`.
    fn limits(&self, text: &str) -> Result<usize, RenderError> {
        let (max_text, max_edges) = self.limits.clone().map_err(RenderError::Config)?;
        // UTF-16 code unit count is always <= UTF-8 byte length, so skip the UTF-16 scan when the
        // byte length already fits within `max_text`.
        if text.len() > max_text
            && text.encode_utf16().take(max_text.saturating_add(1)).count() > max_text
        {
            return Err(RenderError::Options(format!(
                "diagram text exceeds maxTextSize of {max_text} UTF-16 code units"
            )));
        }
        Ok(max_edges)
    }

    /// Cached typed tokens for a shipped theme with the site's `themeVariables` overrides.
    fn site_tokens(&self, theme: Theme) -> Arc<ThemeTokens> {
        let slot = match theme {
            Theme::ReduxColor => &self.light_tokens,
            Theme::ReduxDarkColor => &self.dark_tokens,
        };
        Arc::clone(slot.get_or_init(|| {
            if self.site.has_theme_overrides() {
                Arc::new(ThemeTokens::from_value(
                    self.site.site_theme_variables(theme),
                ))
            } else {
                theme.default_tokens()
            }
        }))
    }

    /// Cached design-mode tokens for `tone` with the site's `themeVariables` overrides.
    fn design_tone_tokens(&self, tone: Tone) -> Arc<ThemeTokens> {
        let slot = match tone {
            Tone::Light => &self.design_light_tokens,
            Tone::Dark => &self.design_dark_tokens,
        };
        Arc::clone(slot.get_or_init(|| design_tokens_for(tone, &self.overrides)))
    }

    /// Resolves only the scheme or design tokens that `Prepared::theme` will actually read.
    fn resolve_tokens(
        &self,
        scheme: Option<ColorScheme>,
        design: Option<Tone>,
        directive: Option<(&Value, &DirectiveTokens)>,
    ) -> (Arc<ThemeTokens>, Arc<ThemeTokens>) {
        let need_light = matches!(scheme, Some(ColorScheme::Light | ColorScheme::Auto))
            || (scheme.is_none() && design == Some(Tone::Light));
        let need_dark = matches!(scheme, Some(ColorScheme::Dark | ColorScheme::Auto))
            || (scheme.is_none() && design == Some(Tone::Dark));
        if !need_light && !need_dark {
            return (ThemeTokens::empty_shared(), ThemeTokens::empty_shared());
        }
        let directive_overrides = directive.and_then(|(dir, tokens)| {
            if dir.get("themeVariables").is_none() && dir.get("fontFamily").is_none() {
                return None;
            }
            let overrides = tokens.overrides(&self.site, dir);
            (!identical(overrides, &self.overrides)).then_some((overrides, tokens))
        });
        let Some((overrides, tokens)) = directive_overrides else {
            let light = if need_light {
                if design.is_some() {
                    self.design_tone_tokens(Tone::Light)
                } else {
                    self.site_tokens(Theme::ReduxColor)
                }
            } else {
                ThemeTokens::empty_shared()
            };
            let dark = if need_dark {
                if design.is_some() {
                    self.design_tone_tokens(Tone::Dark)
                } else {
                    self.site_tokens(Theme::ReduxDarkColor)
                }
            } else {
                ThemeTokens::empty_shared()
            };
            return (light, dark);
        };
        let light = if need_light {
            if design.is_some() {
                Arc::clone(
                    tokens
                        .design_light
                        .get_or_init(|| design_tokens_for(Tone::Light, overrides)),
                )
            } else {
                Arc::clone(
                    tokens
                        .light
                        .get_or_init(|| Theme::ReduxColor.tokens_shared(Some(overrides))),
                )
            }
        } else {
            ThemeTokens::empty_shared()
        };
        let dark = if need_dark {
            if design.is_some() {
                Arc::clone(
                    tokens
                        .design_dark
                        .get_or_init(|| design_tokens_for(Tone::Dark, overrides)),
                )
            } else {
                Arc::clone(
                    tokens
                        .dark
                        .get_or_init(|| Theme::ReduxDarkColor.tokens_shared(Some(overrides))),
                )
            }
        } else {
            ThemeTokens::empty_shared()
        };
        (light, dark)
    }

    /// What a render needs besides its diagram, as `mermaidAPI` resolves it before it calls the
    /// diagram's renderer: the id, the configuration in the diagram's scope, and the tokens of the
    /// schemes that the render draws.
    ///
    /// A text without front matter or directives uses the view of its diagram type and the tokens
    /// of the renderer as they are. Otherwise the directive is merged on top, unless the renderer
    /// kept the view of the same directive, and the tokens are computed again only when the
    /// directive changes the user's overrides.
    fn prepare(
        &self,
        text: &str,
        source: preprocess::Prepared,
        options: &RenderOptions,
        plugin: Plugin,
        max_edges: usize,
    ) -> Result<Prepared<'_>, RenderError> {
        let id = match &options.id {
            Some(id) if valid_id(id) => id.clone(),
            Some(id) => return Err(RenderError::Options(format!("invalid id {id:?}"))),
            None => derived_id(text, &self.configuration),
        };
        let directive = config::read_prepared(&source, Some(plugin.kind()))?;
        let scheme = options.color_scheme;
        if is_empty_object(&directive) {
            let view = self.view(plugin)?;
            let design = options.design.then(|| design_tone(scheme, &view));
            let (light_tokens, dark_tokens) = self.resolve_tokens(scheme, design, None);
            return Ok(Prepared {
                source,
                directive,
                id,
                max_edges,
                design,
                view,
                light_tokens,
                dark_tokens,
                scheme,
                fonts: self.fonts.as_ref(),
            });
        }
        let (view, directive_tokens) = self.directive_view(plugin, &directive)?;
        let design = options.design.then(|| design_tone(scheme, &view));
        let (light_tokens, dark_tokens) =
            self.resolve_tokens(scheme, design, Some((&directive, &directive_tokens)));
        Ok(Prepared {
            source,
            directive,
            id,
            max_edges,
            design,
            view,
            light_tokens,
            dark_tokens,
            scheme,
            fonts: self.fonts.as_ref(),
        })
    }

    /// The view of a diagram type with a directive: a kept one, or one resolved now and kept. The
    /// view depends on the site configuration, the supplied fonts, the diagram type and the
    /// directive; the first two are the renderer's, so the type and the directive are the key.
    fn directive_view(
        &self,
        plugin: Plugin,
        directive: &Value,
    ) -> Result<(Arc<View>, Arc<DirectiveTokens>), RenderError> {
        let kind = plugin.kind();
        {
            let mut kept = lock(&self.directive_views);
            if let Some(at) = kept
                .iter()
                .position(|entry| entry.kind == kind && identical(&entry.directive, directive))
            {
                let entry = kept.remove(at);
                let result = (Arc::clone(&entry.view), Arc::clone(&entry.tokens));
                kept.push(entry);
                return Ok(result);
            }
        }
        // Two threads that miss at once keep two equal views for a while; that costs memory only.
        let view = View::resolve(
            &self.site,
            plugin,
            directive,
            self.fonts.is_some(),
            |theme| self.site_tokens(theme),
        )
        .map(Arc::new)
        .map_err(RenderError::Config)?;
        let tokens = Arc::new(DirectiveTokens::default());
        let mut kept = lock(&self.directive_views);
        if kept.len() == DIRECTIVE_VIEWS_CAPACITY {
            kept.remove(0);
        }
        kept.push(DirectiveView {
            kind,
            directive: directive.clone(),
            view: Arc::clone(&view),
            tokens: Arc::clone(&tokens),
        });
        Ok((view, tokens))
    }

    /// The view of a diagram type with no directive, resolved at its first render.
    fn view(&self, plugin: Plugin) -> Result<Arc<View>, RenderError> {
        let kind = plugin.kind();
        let known = lock(&self.views).get(&kind).cloned();
        let resolved = match known {
            Some(resolved) => resolved,
            None => {
                let resolved = View::resolve(
                    &self.site,
                    plugin,
                    &Value::empty_object(),
                    self.fonts.is_some(),
                    |theme| self.site_tokens(theme),
                )
                .map(Arc::new);
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
    /// Fonts registered by the host.
    fonts: Option<FontRegistry>,
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
    pub fn with_all(mut self) -> Self {
        if self.plugins.is_empty() {
            self.plugins = diagrams::all();
            return self;
        }
        diagrams::all().into_iter().fold(self, Self::with)
    }

    /// Registers a regular upright face from owned local TTF or OTF bytes.
    ///
    /// The first registered family becomes the site font family when the configuration does not
    /// explicitly set `fontFamily`. Diagram configuration still chooses names only.
    ///
    /// # Errors
    /// Returns a typed text error when the font or its metadata is invalid.
    pub fn font(self, family: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Result<Self, Error> {
        self.font_face(family, bytes, FontWeight::Normal, FontStyle::Normal)
    }

    /// Registers one design with an explicit CSS weight and style.
    ///
    /// # Errors
    /// Returns a typed text error when the font or its metadata is invalid.
    pub fn font_face(
        self,
        family: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<Self, Error> {
        self.font_face_shared(family, Arc::new(bytes.into()), weight, style)
    }

    /// Registers one design from shared font bytes, avoiding extra copies when several families
    /// alias the same underlying face.
    ///
    /// # Errors
    /// Returns a typed text error when the font or its metadata is invalid.
    pub fn font_face_shared(
        mut self,
        family: impl Into<String>,
        bytes: Arc<Vec<u8>>,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<Self, Error> {
        let family = family.into();
        let first = self.fonts.as_ref().is_none_or(FontRegistry::is_empty);
        self.fonts
            .get_or_insert_with(FontRegistry::new)
            .register_shared(family.clone(), bytes, weight, style)?;
        if first {
            if !self.config.is_object() {
                self.config = Value::empty_object();
            }
            if let Value::Object(config) = &mut self.config {
                config
                    .entry("fontFamily".into())
                    .or_insert_with(|| Value::String(family.clone()));
                let primary = config
                    .get("fontFamily")
                    .and_then(Value::as_str)
                    .unwrap_or(&family)
                    .to_owned();
                let theme = config
                    .entry("themeVariables".into())
                    .or_insert_with(Value::empty_object);
                if !theme.is_object() {
                    *theme = Value::empty_object();
                }
                if let Value::Object(theme) = theme
                    && theme
                        .get("fontFamily")
                        .is_none_or(|value| value.is_null() || value.as_str() == Some(""))
                {
                    theme.insert("fontFamily".into(), Value::String(primary));
                }
            }
        }
        Ok(self)
    }

    /// Reads and registers a regular upright face from a local file immediately.
    ///
    /// # Errors
    /// Returns a typed text error if reading or registration fails.
    pub fn font_file(
        self,
        family: impl Into<String>,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, Error> {
        self.font_file_face(family, path, FontWeight::Normal, FontStyle::Normal)
    }

    /// Reads and registers a local face with an explicit CSS weight and style.
    ///
    /// # Errors
    /// Returns a typed text error if reading or registration fails.
    pub fn font_file_face(
        self,
        family: impl Into<String>,
        path: impl AsRef<std::path::Path>,
        weight: FontWeight,
        style: FontStyle,
    ) -> Result<Self, Error> {
        let bytes = FontRegistry::read_font_file(path.as_ref())?;
        self.font_face(family, bytes, weight, style)
    }

    /// Sets the fallback family names after each requested CSS family list.
    ///
    /// All files remain host inputs; this does not load fonts by name.
    #[must_use]
    pub fn font_fallbacks(mut self, families: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let families: Vec<String> = families.into_iter().map(Into::into).collect();
        if families.is_empty() && self.fonts.is_none() {
            return self;
        }
        self.fonts
            .get_or_insert_with(FontRegistry::new)
            .set_fallbacks(families);
        self
    }

    /// The renderer of this configuration and these plugins, kept for the next call. A renderer that
    /// is kept already is returned; otherwise it is made as [`build`](Self::build) makes it and kept,
    /// in the place of the least recently used one once a handful are kept. [`render`] uses it with
    /// every plugin; a program that renders one diagram at a time with the same configuration uses
    /// it so that the configuration is not resolved again. The order of registration does not matter.
    #[must_use]
    pub fn shared(self) -> Arc<Renderer> {
        let font_fingerprint = self.fonts.as_ref().map(FontRegistry::fingerprint);
        let font_bytes = self
            .fonts
            .as_ref()
            .map_or(0, FontRegistry::cached_font_bytes);
        let mut kinds: Vec<DiagramType> = self.plugins.iter().map(Plugin::kind).collect();
        kinds.sort_unstable_by(|a, b| a.config_key().cmp(b.config_key()));
        if font_bytes <= SHORTHAND_FONT_BYTE_BUDGET
            && let Some(renderer) =
                kept(&self.config, &kinds, font_fingerprint, self.fonts.as_ref())
        {
            return renderer;
        }
        // Two threads that miss at once keep two equal renderers for a while; that costs memory only.
        let config = if font_bytes <= SHORTHAND_FONT_BYTE_BUDGET {
            Some(self.config.clone())
        } else {
            None
        };
        let renderer = Arc::new(self.build());
        let Some(config) = config else {
            return renderer;
        };
        let mut cache = lock(&SHORTHAND);
        while cache.len() >= SHORTHAND_CAPACITY
            || (font_bytes > 0
                && cache
                    .iter()
                    .map(|entry| entry.font_bytes)
                    .fold(0usize, usize::saturating_add)
                    .saturating_add(font_bytes)
                    > SHORTHAND_FONT_BYTE_BUDGET)
        {
            if cache.is_empty() {
                break;
            }
            cache.remove(0);
        }
        cache.push(Shared {
            config,
            kinds,
            font_fingerprint,
            font_bytes,
            renderer: Arc::clone(&renderer),
        });
        renderer
    }

    /// Resolves the configuration and makes the renderer. This does not fail; see [`Renderer::new`].
    #[must_use]
    pub fn build(mut self) -> Renderer {
        let font_fingerprint = self.fonts.as_ref().map(FontRegistry::fingerprint);
        let site = SiteConfig::new(Some(&self.config));
        let limits = parse_limits(site.settings());
        let overrides = site.user_theme_variables(&Value::empty_object());
        let configuration = {
            let mut configuration = serde_json::to_string(&self.config)
                .unwrap_or_else(|_| canonical_value_json(&self.config));
            if let Some(fingerprint) = font_fingerprint {
                configuration.push_str(&format!(";fonts={fingerprint:016x}"));
            }
            configuration
        };
        let plugins = std::mem::take(&mut self.plugins);
        let fonts = self.fonts.take().map(Arc::new);
        Renderer {
            limits,
            light_tokens: OnceLock::new(),
            dark_tokens: OnceLock::new(),
            design_light_tokens: OnceLock::new(),
            design_dark_tokens: OnceLock::new(),
            configuration,
            overrides,
            site,
            views: Mutex::new(HashMap::new()),
            directive_views: Mutex::new(Vec::new()),
            plugins,
            fonts,
            subset_cache: Arc::default(),
        }
    }
}

impl Drop for RendererBuilder {
    fn drop(&mut self) {
        ferrigura_model::value::drop_iterative(std::mem::take(&mut self.config));
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        ferrigura_model::value::drop_iterative(std::mem::take(&mut self.overrides));
    }
}

/// Iterative JSON-like serialization used as a fallback for [`derived_id`] when `serde_json`
/// rejects a configuration (for example, nesting beyond 128 levels or non-finite numbers).
fn canonical_value_json(value: &Value) -> String {
    enum Step<'a> {
        Value(&'a Value),
        Raw(&'static str),
        Str(&'a str),
    }

    let mut out = String::new();
    let mut stack = vec![Step::Value(value)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Raw(text) => out.push_str(text),
            Step::Str(text) => {
                out.push_str(&serde_json::to_string(text).unwrap_or_default());
            }
            Step::Value(current) => match current {
                Value::Null => out.push_str("null"),
                Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
                Value::Number(number) => {
                    out.push_str(&ferrigura_model::value::number_text(*number));
                }
                Value::String(text) => {
                    out.push_str(&serde_json::to_string(text).unwrap_or_default());
                }
                Value::Array(items) => {
                    out.push('[');
                    stack.push(Step::Raw("]"));
                    for (index, item) in items.iter().enumerate().rev() {
                        stack.push(Step::Value(item));
                        if index > 0 {
                            stack.push(Step::Raw(","));
                        }
                    }
                }
                Value::Object(map) => {
                    out.push('{');
                    stack.push(Step::Raw("}"));
                    for (index, (key, entry)) in map.iter().enumerate().rev() {
                        stack.push(Step::Value(entry));
                        stack.push(Step::Raw(":"));
                        stack.push(Step::Str(key));
                        if index > 0 {
                            stack.push(Step::Raw(","));
                        }
                    }
                }
            },
        }
    }
    out
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
/// the configuration holds a value of the wrong type, or if a bundled font cannot be read. A text
/// of a diagram type that ferrigura does not draw gives [`Error::Unsupported`], which names the type
/// and the types ferrigura draws ([`DiagramType::ALL`]).
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
    /// does not type as the reference reads it (the sequence's `wrap`).
    effective: Value,
    /// The sections of the same configuration that the diagram type's renderer reads, typed; the
    /// other sections keep their defaults, and the top level is read into `globals`.
    config: MermaidConfig,
    /// The tokens of the configured theme: the `themeVariables` of the configuration. A view whose
    /// theme variables are those of the view without a directive shares that view's tokens.
    configured_tokens: Arc<ThemeTokens>,
    /// The global options of the configuration, as the view resolves them.
    globals: Globals,
}

impl View {
    /// The view of a diagram type, with the directive merged on top of the site configuration.
    /// `site_tokens` supplies the renderer's cached tokens for a shipped theme with site overrides,
    /// so views whose resolved `themeVariables` match the site's share the same `Arc<ThemeTokens>`.
    ///
    /// # Errors
    ///
    /// Returns the message of the error when the configuration cannot be read into its types.
    fn resolve(
        site: &SiteConfig,
        plugin: Plugin,
        directive: &Value,
        supplied_fonts: bool,
        site_tokens: impl FnOnce(Theme) -> Arc<ThemeTokens>,
    ) -> Result<Self, String> {
        let kind = plugin.kind();
        let diagram_key = kind.config_key();
        let (mut effective, resolved_theme) =
            site.effective_for(Some(diagram_key), extra_sections(diagram_key), directive);
        if supplied_fonts {
            let theme_variables = match &resolved_theme {
                ResolvedThemeVariables::Site(theme) => site.site_theme_variables(*theme),
                ResolvedThemeVariables::Custom(variables) => variables,
            };
            let primary = theme_variables
                .get("fontFamily")
                .or_else(|| effective.get("fontFamily"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let Some(primary) = primary {
                inherit_font_roles(
                    &mut effective,
                    &mut Vec::new(),
                    site.settings(),
                    directive,
                    &primary,
                );
            }
        }
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
        // Share the renderer's cached `Arc<ThemeTokens>` whenever the resolved theme variables
        // match the site's variables for the active theme.
        let configured_tokens = match resolved_theme {
            ResolvedThemeVariables::Site(theme) => site_tokens(theme),
            ResolvedThemeVariables::Custom(variables) => {
                let theme = Theme::from_name(
                    effective
                        .get("theme")
                        .and_then(Value::as_str)
                        .unwrap_or(site.site_theme()),
                );
                if identical(&variables, site.site_theme_variables(theme)) {
                    site_tokens(theme)
                } else {
                    Arc::new(ThemeTokens::from_value(&variables))
                }
            }
        };
        Ok(Self {
            globals: Globals::of_top(&top),
            configured_tokens,
            effective,
            config,
        })
    }
}

/// Additional configuration sections a diagram type reads beyond its own `config_key()`.
fn extra_sections(diagram_key: &str) -> &'static [&'static str] {
    const UNIFIED: &[&str] = &["flowchart", "elk", "class", "er", "state"];
    match diagram_key {
        "flowchart" | "er" | "class" | "state" | "mindmap" | "block" | "requirement"
        | "agentflow" | "swimlane" | "usecase" => UNIFIED,
        "kanban" => &["mindmap"],
        _ => &[],
    }
}

/// Validates and extracts `(maxTextSize, maxEdges)` from site settings.
fn parse_limits(settings: &Value) -> Result<(usize, usize), String> {
    let read = |key: &str, default: usize| -> Result<usize, String> {
        match settings.get(key) {
            None => Ok(default),
            Some(Value::Number(n))
                if n.is_finite()
                    && *n >= 0.0
                    && n.fract() == 0.0
                    && *n <= 9_007_199_254_740_991.0
                    && *n <= usize::MAX as f64 =>
            {
                Ok(*n as usize)
            }
            Some(_) => Err(format!("{key} must be a non-negative safe integer")),
        }
    };
    let max_text = read("maxTextSize", 50_000)?;
    let max_edges = read("maxEdges", 500)?;
    Ok((max_text, max_edges))
}

/// A supplied primary replaces schema-default role names, while explicit site/diagram roles
/// remain requests to the registry. Provenance must be checked before the typed view loses it.
fn inherit_font_roles(
    value: &mut Value,
    path: &mut Vec<String>,
    site: &Value,
    directive: &Value,
    primary: &str,
) {
    fn requested(source: &Value, path: &[String]) -> bool {
        path.iter()
            .try_fold(source, |value, key| value.get(key))
            .is_some_and(|value| !value.is_null())
    }
    let Value::Object(properties) = value else {
        return;
    };
    if path.len() >= 128 {
        return;
    }
    for (key, child) in properties {
        path.push(key.clone());
        if path.len() > 1
            && (key == "fontFamily" || key.ends_with("FontFamily"))
            && matches!(child, Value::String(_))
            && !requested(site, path)
            && !requested(directive, path)
        {
            *child = Value::String(primary.to_owned());
        } else {
            inherit_font_roles(child, path, site, directive, primary);
        }
        path.pop();
    }
}

/// What a render needs besides its diagram: see [`Renderer::prepare`].
struct Prepared<'a> {
    source: preprocess::Prepared,
    directive: Value,
    max_edges: usize,
    fonts: Option<&'a Arc<FontRegistry>>,
    id: String,
    view: Arc<View>,
    light_tokens: Arc<ThemeTokens>,
    dark_tokens: Arc<ThemeTokens>,
    scheme: Option<ColorScheme>,
    /// The tone of the design mode, when the render draws in it.
    design: Option<Tone>,
}

impl Prepared<'_> {
    /// The theme of the render as typed tokens: the configured theme, or the scheme that the color
    /// scheme chooses, with the light scheme as the primary one when the render draws both. In the
    /// design mode without a color scheme, the configured theme chooses the tone.
    fn theme(&self) -> ThemeView<'_> {
        match self.scheme {
            None if self.design == Some(Tone::Dark) => ThemeView {
                light: &self.dark_tokens,
                dark: None,
            },
            None if self.design.is_some() => ThemeView {
                light: &self.light_tokens,
                dark: None,
            },
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

/// The tokens of one tone of the design mode: the shipped theme with the mode's variables laid
/// under the user's overrides, which win key by key. Reuses a process-wide cache when there are
/// no overrides.
fn design_tokens_for(tone: Tone, overrides: &Value) -> Arc<ThemeTokens> {
    static LIGHT: LazyLock<Arc<ThemeTokens>> =
        LazyLock::new(|| compute_design_tokens(Tone::Light, &Value::empty_object()));
    static DARK: LazyLock<Arc<ThemeTokens>> =
        LazyLock::new(|| compute_design_tokens(Tone::Dark, &Value::empty_object()));
    if !matches!(overrides, Value::Object(map) if !map.is_empty()) {
        return match tone {
            Tone::Light => Arc::clone(&LIGHT),
            Tone::Dark => Arc::clone(&DARK),
        };
    }
    compute_design_tokens(tone, overrides)
}

fn compute_design_tokens(tone: Tone, overrides: &Value) -> Arc<ThemeTokens> {
    let theme = match tone {
        Tone::Light => Theme::ReduxColor,
        Tone::Dark => Theme::ReduxDarkColor,
    };
    let mut variables = design::theme_variables(tone);
    if let (Value::Object(variables), Value::Object(overrides)) = (&mut variables, overrides) {
        for (name, value) in overrides {
            variables.insert(name.clone(), value.clone());
        }
    }
    Arc::new(ThemeTokens::from_value(
        &theme.variables_shared(Some(&variables)),
    ))
}

/// The tone of the one token set a render in the design mode draws with: the color scheme's, and the
/// configured theme's without one. A render with both schemes draws with the light set.
fn design_tone(scheme: Option<ColorScheme>, view: &View) -> Tone {
    match scheme {
        Some(ColorScheme::Dark) => Tone::Dark,
        Some(ColorScheme::Light | ColorScheme::Auto) => Tone::Light,
        None => match Theme::from_name(&view.globals.theme) {
            Theme::ReduxDarkColor => Tone::Dark,
            Theme::ReduxColor => Tone::Light,
        },
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
    let renderer = Renderer::builder(config).with_all().shared();
    let max_edges = renderer.limits(text)?;
    let Some(source) = preprocess::prepare(text)
        .ok()
        .filter(|source| detect_prepared(source) == Some(DiagramType::Flowchart))
    else {
        return Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "not a flowchart".to_owned(),
        }));
    };
    let prepared = renderer.prepare(text, source, options, diagrams::flowchart(), max_edges)?;
    let stages = with_flowchart(&prepared, options.compat, flowchart::elk_stages)?;
    Ok(serde_json::json!({
        "elkInput": stages.input,
        "elkOutput": stages.output,
    }))
}

/// Parses a flowchart and hands it, with its sections, globals and theme, to `step`.
fn with_flowchart<T>(
    prepared: &Prepared<'_>,
    compat: bool,
    step: impl FnOnce(&Context<'_, FlowchartDiagram, Section<'_>>) -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    let diagram = ferrigura_syntax::flowchart::parse_prepared_with_security(
        &prepared.source,
        ferrigura_syntax::flowchart::Options {
            max_edges: prepared.max_edges,
        },
        ferrigura_syntax::url::SecurityLevel::from_config(&prepared.view.globals.security_level),
    )
    .map_err(|ferrigura_syntax::flowchart::FlowchartError::Syntax(error)| error)?;
    let mut section = Section::of(&prepared.view.config);
    section.flowchart_routing = if compat {
        FlowchartRouting::Mermaid
    } else {
        FlowchartRouting::Adaptive
    };
    section.design = prepared.design;
    step(&Context {
        fonts: prepared.fonts,
        diagram: &diagram,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id: &prepared.id,
    })
}

/// How many views of directives a renderer keeps. One that is not kept is resolved again at its next
/// render; the output does not depend on it.
const DIRECTIVE_VIEWS_CAPACITY: usize = 8;

/// A view a renderer keeps, with the diagram type, the directive it was resolved for, and any
/// lazily resolved scheme/design tokens for that directive.
#[derive(Debug)]
struct DirectiveView {
    kind: DiagramType,
    directive: Value,
    view: Arc<View>,
    tokens: Arc<DirectiveTokens>,
}

/// Lazily resolved scheme and design tokens for a cached directive, shared across cache hits.
#[derive(Debug, Default)]
struct DirectiveTokens {
    overrides: OnceLock<Value>,
    light: OnceLock<Arc<ThemeTokens>>,
    dark: OnceLock<Arc<ThemeTokens>>,
    design_light: OnceLock<Arc<ThemeTokens>>,
    design_dark: OnceLock<Arc<ThemeTokens>>,
}

impl DirectiveTokens {
    fn overrides(&self, site: &SiteConfig, directive: &Value) -> &Value {
        self.overrides
            .get_or_init(|| site.user_theme_variables(directive))
    }
}

/// The renderers of [`RendererBuilder::shared`], which [`render`] and [`render_with`] use with every
/// plugin, each with the configuration and the kinds it was made from, least recently used first.
static SHORTHAND: Mutex<Vec<Shared>> = Mutex::new(Vec::new());

/// How many renderers the shorthand keeps. One that is not kept is made again at its next render;
/// the output does not depend on it.
const SHORTHAND_CAPACITY: usize = 8;

/// Maximum retained supplied-font bytes across all renderers in [`SHORTHAND`].
const SHORTHAND_FONT_BYTE_BUDGET: usize = 16 * 1024 * 1024;

/// A renderer the shorthand keeps, with what it was made from.
struct Shared {
    config: Value,
    /// The kinds the renderer draws, sorted, so that the order of registration does not matter.
    kinds: Vec<DiagramType>,
    /// The fingerprint of supplied fonts, or `None` when using bundled defaults.
    font_fingerprint: Option<u64>,
    /// Estimated retained bytes of supplied font data owned by `renderer`.
    font_bytes: usize,
    renderer: Arc<Renderer>,
}

impl Drop for Shared {
    fn drop(&mut self) {
        ferrigura_model::value::drop_iterative(std::mem::take(&mut self.config));
    }
}

/// The renderer that the shorthand keeps for a configuration, a set of kinds, and any supplied
/// fonts, which becomes the most recently used.
fn kept(
    config: &Value,
    kinds: &[DiagramType],
    font_fingerprint: Option<u64>,
    fonts: Option<&FontRegistry>,
) -> Option<Arc<Renderer>> {
    let mut cache = lock(&SHORTHAND);
    let at = cache.iter().position(|entry| {
        entry.font_fingerprint == font_fingerprint
            && entry.kinds == kinds
            && identical(&entry.config, config)
            && match (fonts, entry.renderer.fonts.as_deref()) {
                (None, None) => true,
                (Some(requested), Some(cached)) => requested.same_as(cached),
                _ => false,
            }
    })?;
    let entry = cache.remove(at);
    let renderer = Arc::clone(&entry.renderer);
    cache.push(entry);
    Some(renderer)
}

/// Whether two configurations are the same value, number by number: `0` and `-0` are not, as the
/// derived id tells them apart, and a `NaN` is the same `NaN`.
fn identical(a: &Value, b: &Value) -> bool {
    let mut stack = vec![(a, b)];
    while let Some((left, right)) = stack.pop() {
        match (left, right) {
            (Value::Null, Value::Null) => {}
            (Value::Bool(a), Value::Bool(b)) if a == b => {}
            (Value::Number(a), Value::Number(b)) if a.to_bits() == b.to_bits() => {}
            (Value::String(a), Value::String(b)) if a == b => {}
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                stack.extend(a.iter().zip(b.iter()));
            }
            (Value::Object(a), Value::Object(b)) if a.len() == b.len() => {
                for ((key_a, a), (key_b, b)) in a.iter().zip(b.iter()) {
                    if key_a != key_b {
                        return false;
                    }
                    stack.push((a, b));
                }
            }
            _ => return false,
        }
    }
    true
}

/// The guard of a cache. A panic in another render does not keep it shut: the caches hold nothing a
/// panic could leave half written.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The error of a text whose header starts a diagram type that ferrigura does not draw: it names
/// that type and lists the types ferrigura draws.
fn unsupported(name: &str) -> RenderError {
    let drawn: Vec<&str> = DiagramType::ALL.iter().map(|kind| kind.name()).collect();
    let (last, rest) = drawn.split_last().unwrap_or((&"", &[]));
    RenderError::Unsupported(format!(
        "ferrigura does not draw {name} diagrams; it draws {} and {last} diagrams",
        rest.join(", ")
    ))
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

#[cfg(test)]
mod font_input_tests {
    use super::*;
    const SANS: &[u8] = include_bytes!("../../ferrigura-text/fonts/LexicaUltralegible-Regular.ttf");
    const MONO: &[u8] =
        include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");

    #[test]
    fn font_inputs_are_owned_and_validated_by_the_builder() {
        let renderer = Renderer::builder(&Value::empty_object())
            .with_all()
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .build();
        assert!(renderer.fonts.is_some());
        assert_eq!(
            renderer.view(diagrams::pie()).unwrap().globals.font_family,
            "Fixture"
        );
        assert!(matches!(
            Renderer::builder(&Value::empty_object()).font("Broken", vec![1, 2]),
            Err(Error::Text(_))
        ));
    }

    #[test]
    fn explicit_family_is_kept_and_fallback_order_does_not_change_default_selection() {
        let renderer = Renderer::builder(&Value::empty_object())
            .with_all()
            .font_fallbacks(["Symbols"])
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .build();
        assert_eq!(
            renderer.view(diagrams::pie()).unwrap().globals.font_family,
            "Fixture"
        );
        let config = Value::Object(std::collections::BTreeMap::from([(
            "fontFamily".into(),
            Value::String("Chosen".into()),
        )]));
        let renderer = Renderer::builder(&config)
            .with_all()
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .build();
        assert_eq!(
            renderer.view(diagrams::pie()).unwrap().globals.font_family,
            "Chosen"
        );
    }

    #[test]
    fn empty_fallback_options_keep_the_compatible_default_renderer() {
        let ordinary = Renderer::new(&Value::empty_object());
        let empty = Renderer::builder(&Value::empty_object())
            .with_all()
            .font_fallbacks(std::iter::empty::<String>())
            .build();
        assert!(empty.fonts.is_none());
        assert_eq!(
            ordinary.render("pie\n \"A\": 1").unwrap(),
            empty.render("pie\n \"A\": 1").unwrap()
        );
    }

    #[test]
    fn ignored_root_config_values_still_choose_the_first_supplied_family() {
        for config in [
            Value::Null,
            Value::Bool(true),
            Value::String("ignored".into()),
        ] {
            assert!(Renderer::new(&config).render("pie\n \"A\": 1").is_ok());
            let renderer = Renderer::builder(&config)
                .with_all()
                .font("Fixture", SANS.to_vec())
                .unwrap()
                .build();
            assert_eq!(
                renderer.view(diagrams::pie()).unwrap().globals.font_family,
                "Fixture"
            );
        }
    }

    #[test]
    fn only_implicit_per_type_families_inherit_the_supplied_primary() {
        let renderer = Renderer::builder(&Value::empty_object())
            .with_all()
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .build();
        let view = renderer.view(diagrams::sequence()).unwrap();
        assert_eq!(
            view.effective
                .get("sequence")
                .and_then(|section| section.get("actorFontFamily"))
                .and_then(Value::as_str),
            Some("Fixture")
        );
        let config = Value::from(serde_json::json!({"sequence":{"actorFontFamily":"Open Sans"}}));
        let renderer = Renderer::builder(&config)
            .with_all()
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .build();
        let view = renderer.view(diagrams::sequence()).unwrap();
        assert_eq!(
            view.effective
                .get("sequence")
                .and_then(|section| section.get("actorFontFamily"))
                .and_then(Value::as_str),
            Some("Open Sans")
        );
    }

    #[test]
    fn supplied_bytes_change_svg_identity_and_do_not_reuse_a_config_only_cache() {
        let first = Renderer::builder(&Value::empty_object())
            .with_all()
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .shared();
        let first_again = Renderer::builder(&Value::empty_object())
            .with_all()
            .font("Fixture", SANS.to_vec())
            .unwrap()
            .shared();
        let second = Renderer::builder(&Value::empty_object())
            .with_all()
            .font("Fixture", MONO.to_vec())
            .unwrap()
            .shared();
        assert!(Arc::ptr_eq(&first, &first_again));
        assert!(!Arc::ptr_eq(&first, &second));
        assert_ne!(first.configuration, second.configuration);
    }
}

#[cfg(test)]
mod subset_cache_tests {
    use super::*;

    #[test]
    fn repeated_distinct_diagrams_and_concurrent_renders_match_uncached_svg() {
        let renderer = Renderer::new(&Value::empty_object());
        let options = RenderOptions {
            id: Some("test".into()),
            now: Some(1_700_000_000_000),
            ..RenderOptions::default()
        };
        let diagrams = [
            "pie\n  \"Alpha\" : 1\n  \"Beta\" : 2\n",
            "pie\n  \"Beta\" : 1\n  \"Alpha\" : 2\n",
        ];
        let expected: Vec<_> = diagrams
            .iter()
            .map(|text| renderer.render_with_cache(text, &options).unwrap())
            .collect();
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let renderer = &renderer;
                let options = &options;
                let expected = &expected;
                scope.spawn(move || {
                    for _ in 0..5 {
                        for (text, expected) in diagrams.iter().zip(expected) {
                            assert_eq!(&renderer.render_with(text, options).unwrap(), expected);
                        }
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;

    fn json(value: serde_json::Value) -> Value {
        Value::from(value)
    }

    /// Two resolutions are the same view, or fail with the same message.
    fn assert_same(
        shared: &Result<View, String>,
        full: &Result<View, String>,
        diagram_key: &str,
        case: &str,
    ) {
        match (shared, full) {
            (Ok(shared), Ok(full)) => {
                let scoped = shared
                    .effective
                    .as_object()
                    .expect("scoped effective object");
                assert!(!scoped.contains_key("themeVariables"), "{case}");
                for (key, value) in scoped {
                    let expected = full
                        .effective
                        .get(key)
                        .unwrap_or_else(|| panic!("{case}: missing {key} in full effective"));
                    assert!(identical(value, expected), "{case}: key {key}");
                }
                assert_eq!(
                    shared.effective.get(diagram_key),
                    full.effective.get(diagram_key),
                    "{case}: diagram section {diagram_key}"
                );
                for &section in extra_sections(diagram_key) {
                    assert_eq!(
                        shared.effective.get(section),
                        full.effective.get(section),
                        "{case}: extra section {section}"
                    );
                }
                assert_eq!(
                    shared.effective.get("wrap"),
                    full.effective.get("wrap"),
                    "{case}: wrap"
                );
                if diagram_key == "sequence" {
                    assert_eq!(
                        sequence_wrap(&shared.effective),
                        sequence_wrap(&full.effective),
                        "{case}: sequence_wrap"
                    );
                }
                assert_eq!(shared.config, full.config, "{case}");
                assert_eq!(*shared.configured_tokens, *full.configured_tokens, "{case}");
                assert_eq!(shared.globals, full.globals, "{case}");
            }
            (Err(shared), Err(full)) => assert_eq!(shared, full, "{case}"),
            _ => panic!("{case}: one resolution failed and the other did not"),
        }
    }

    #[test]
    fn a_view_that_shares_with_the_plain_view_equals_the_one_resolved_in_full() {
        let sites = [
            (json(serde_json::json!({})), false),
            (
                json(serde_json::json!({
                    "theme": "neutral",
                    "themeVariables": { "primaryColor": "#336699" },
                    "flowchart": { "curve": "linear" },
                    "layout": "dagre",
                })),
                false,
            ),
            (
                json(serde_json::json!({
                    "fontFamily": "Fixture",
                    "themeVariables": { "fontFamily": "Fixture" },
                    "sequence": { "actorFontFamily": "Open Sans" },
                })),
                true,
            ),
        ];
        let directives = [
            serde_json::json!({}),
            serde_json::json!({ "logLevel": 1, "htmlLabels": false }),
            serde_json::json!({ "pie": { "textPosition": 0.9 }, "gantt": { "topAxis": true } }),
            serde_json::json!({ "theme": "dark" }),
            serde_json::json!({ "flowchart": { "theme": "redux-dark-color" } }),
            serde_json::json!({ "themeVariables": { "primaryColor": "#ff0000" } }),
            serde_json::json!({ "fontFamily": "courier" }),
            serde_json::json!({ "sequence": { "actorFontFamily": "courier" } }),
            serde_json::json!({
                "flowchart": { "subGraphTitleMargin": { "top": 5 } },
                "themeVariables": { "xyChart": { "titleColor": "#123456" } },
            }),
            serde_json::json!({ "layout": "elk", "look": "handDrawn" }),
            serde_json::json!({ "securityLevel": "loose", "secure": [] }),
            serde_json::json!({ "fontSize": "big", "pie": "round", "themeVariables": 3 }),
        ];
        for (site, supplied_fonts) in &sites {
            let site = SiteConfig::new(Some(site));
            for plugin in diagrams::all() {
                for directive in &directives {
                    let directive = json(directive.clone());
                    let case = format!("{:?} with {directive:?}", plugin.kind());
                    let full = resolve_full_uncached(&site, plugin, &directive, *supplied_fonts);
                    let light = OnceLock::new();
                    let dark = OnceLock::new();
                    let shared = View::resolve(&site, plugin, &directive, *supplied_fonts, |t| {
                        let slot = match t {
                            Theme::ReduxColor => &light,
                            Theme::ReduxDarkColor => &dark,
                        };
                        Arc::clone(slot.get_or_init(|| {
                            Arc::new(ThemeTokens::from_value(site.site_theme_variables(t)))
                        }))
                    });
                    assert_same(&shared, &full, plugin.kind().config_key(), &case);
                }
            }
        }
    }

    /// Reference uncached resolution using the full `SiteConfig::effective` map, to verify that
    /// scoped resolution produces identical typed config, globals, theme tokens, and untyped keys.
    fn resolve_full_uncached(
        site: &SiteConfig,
        plugin: Plugin,
        directive: &Value,
        supplied_fonts: bool,
    ) -> Result<View, String> {
        let kind = plugin.kind();
        let mut effective = site.effective(Some(kind.config_key()), directive);
        if supplied_fonts {
            let primary = effective
                .get("themeVariables")
                .and_then(|theme| theme.get("fontFamily"))
                .or_else(|| effective.get("fontFamily"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let Some(primary) = primary {
                inherit_font_roles(
                    &mut effective,
                    &mut Vec::new(),
                    site.settings(),
                    directive,
                    &primary,
                );
            }
        }
        let mut top = typed_top(&effective).map_err(|error| error.to_string())?;
        if matches!(kind, DiagramType::Mindmap)
            && !user_sets_layout(Some(site.settings()), directive)
        {
            top.layout = "cose-bilkent".to_owned();
        }
        let mut config = MermaidConfig::default();
        plugin
            .read(&effective, &mut config)
            .map_err(|error| error.to_string())?;
        let variables = effective.get("themeVariables");
        let configured_tokens = Arc::new(ThemeTokens::from_value(
            variables.unwrap_or(&Value::empty_object()),
        ));
        Ok(View {
            globals: Globals::of_top(&top),
            configured_tokens,
            effective,
            config,
        })
    }

    #[test]
    fn a_directive_that_leaves_the_theme_alone_shares_the_tokens_of_the_plain_view() {
        let renderer = Renderer::new(&Value::empty_object());
        let plugin = diagrams::pie();
        let plain = renderer.view(plugin).unwrap();
        let resolve = |directive: serde_json::Value| {
            View::resolve(&renderer.site, plugin, &json(directive), false, |theme| {
                renderer.site_tokens(theme)
            })
            .unwrap()
        };
        let untouched = resolve(serde_json::json!({ "pie": { "textPosition": 0.9 } }));
        assert!(Arc::ptr_eq(
            &untouched.configured_tokens,
            &plain.configured_tokens
        ));
        // Plain views of other light-themed diagram types share the exact same `Arc<ThemeTokens>`.
        let flow = renderer.view(diagrams::flowchart()).unwrap();
        assert!(Arc::ptr_eq(
            &flow.configured_tokens,
            &plain.configured_tokens
        ));
        // A dark-themed directive uses the dark site tokens (distinct from light, shared with renderer).
        let themed = resolve(serde_json::json!({ "theme": "dark" }));
        assert!(!Arc::ptr_eq(
            &themed.configured_tokens,
            &plain.configured_tokens
        ));
        assert!(Arc::ptr_eq(
            &themed.configured_tokens,
            &renderer.site_tokens(Theme::ReduxDarkColor)
        ));
    }

    #[test]
    fn directive_cache_reuses_scheme_and_design_tokens_with_pinned_parity() {
        const SANS: &[u8] =
            include_bytes!("../../ferrigura-text/fonts/LexicaUltralegible-Regular.ttf");
        const MONO: &[u8] =
            include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");

        let sites: Vec<(Value, bool)> = vec![
            (Value::empty_object(), false),
            (
                json(serde_json::json!({
                    "theme": "redux-color",
                    "themeVariables": { "primaryColor": "#336699" },
                })),
                false,
            ),
            (
                json(serde_json::json!({
                    "theme": "redux-dark-color",
                    "themeVariables": { "primaryColor": "#663399" },
                })),
                true,
            ),
        ];

        let diagrams = [
            "flowchart LR\n  A --> B\n",
            "%%{init: {'theme': 'dark', 'themeVariables': {'primaryColor': '#ff4400'}}}%%\nflowchart LR\n  A --> B\n",
            "---\nconfig:\n  theme: redux-color\n  themeVariables:\n    primaryColor: '#00aa55'\n    lineColor: '#224466'\n---\nflowchart LR\n  A --> B\n",
            "%%{init: {'flowchart': {'curve': 'linear'}}}%%\nflowchart LR\n  A --> B\n",
            "%%{init: {'fontFamily': 'Mono'}}%%\nstateDiagram-v2\n  A --> B\n",
            "---\nconfig:\n  theme: dark\n---\nclassDiagram\n  class Animal\n",
        ];

        let option_sets = [
            RenderOptions {
                id: Some("pinned".into()),
                ..RenderOptions::default()
            },
            RenderOptions {
                id: Some("pinned".into()),
                color_scheme: Some(ColorScheme::Light),
                ..RenderOptions::default()
            },
            RenderOptions {
                id: Some("pinned".into()),
                color_scheme: Some(ColorScheme::Dark),
                ..RenderOptions::default()
            },
            RenderOptions {
                id: Some("pinned".into()),
                color_scheme: Some(ColorScheme::Auto),
                ..RenderOptions::default()
            },
            RenderOptions {
                id: Some("pinned".into()),
                design: true,
                ..RenderOptions::default()
            },
            RenderOptions {
                id: Some("pinned".into()),
                design: true,
                color_scheme: Some(ColorScheme::Auto),
                ..RenderOptions::default()
            },
            RenderOptions {
                id: Some("pinned".into()),
                design: true,
                color_scheme: Some(ColorScheme::Dark),
                ..RenderOptions::default()
            },
        ];

        for (site, with_font) in &sites {
            let make_renderer = || {
                let builder = Renderer::builder(site).with_all();
                if *with_font {
                    builder
                        .font("Fixture", SANS.to_vec())
                        .unwrap()
                        .font("Mono", MONO.to_vec())
                        .unwrap()
                        .build()
                } else {
                    builder.build()
                }
            };
            let shared = make_renderer();

            // Repeated, alternating, and distinct directive renders must match fresh renderers
            // byte-for-byte across every colorScheme and design combination.
            for pass in 0..3 {
                for options in &option_sets {
                    for text in &diagrams {
                        let actual = shared.render_with(text, options).unwrap();
                        let fresh = make_renderer().render_with(text, options).unwrap();
                        assert_eq!(
                            actual, fresh,
                            "pass={pass} options={options:?} text={text:?}"
                        );
                    }
                }
            }

            // Verify cached DirectiveTokens actually reuse Arc<ThemeTokens> on hits.
            let dir = json(serde_json::json!({
                "theme": "dark",
                "themeVariables": { "primaryColor": "#ff4400" },
            }));
            let (v1, t1) = shared.directive_view(diagrams::flowchart(), &dir).unwrap();
            let (l1, d1) = shared.resolve_tokens(Some(ColorScheme::Auto), None, Some((&dir, &t1)));
            let (dl1, dd1) = shared.resolve_tokens(
                Some(ColorScheme::Auto),
                Some(Tone::Light),
                Some((&dir, &t1)),
            );
            let (v2, t2) = shared.directive_view(diagrams::flowchart(), &dir).unwrap();
            let (l2, d2) = shared.resolve_tokens(Some(ColorScheme::Auto), None, Some((&dir, &t2)));
            let (dl2, dd2) = shared.resolve_tokens(
                Some(ColorScheme::Auto),
                Some(Tone::Light),
                Some((&dir, &t2)),
            );
            assert!(Arc::ptr_eq(&v1, &v2));
            assert!(Arc::ptr_eq(&t1, &t2));
            assert!(Arc::ptr_eq(&l1, &l2));
            assert!(Arc::ptr_eq(&d1, &d2));
            assert!(Arc::ptr_eq(&dl1, &dl2));
            assert!(Arc::ptr_eq(&dd1, &dd2));
        }
    }

    #[test]
    fn secure_keys_and_front_matter_directive_precedence_are_preserved() {
        let site = json(serde_json::json!({
            "secure": ["fontSize", "theme"],
            "theme": "redux-color",
            "fontSize": 16,
        }));
        let shared = Renderer::new(&site);
        let options = RenderOptions {
            id: Some("sec".into()),
            ..RenderOptions::default()
        };

        // Secure keys cannot be overridden by front matter or init directives.
        let blocked =
            "%%{init: {'theme': 'dark', 'fontSize': 40, 'secure': []}}%%\npie\n  \"A\" : 1\n";
        let plain = "pie\n  \"A\" : 1\n";
        assert_eq!(
            shared.render_with(blocked, &options).unwrap(),
            shared.render_with(plain, &options).unwrap()
        );
        assert_eq!(
            shared.render_with(blocked, &options).unwrap(),
            Renderer::new(&site).render_with(blocked, &options).unwrap()
        );

        // Front matter + init directive precedence matches fresh renders across repeated hits.
        let combined = "---\nconfig:\n  theme: dark\n  pie:\n    textPosition: 0.9\n---\n%%{init: {'theme': 'redux-color'}}%%\npie\n  \"A\" : 1\n  \"B\" : 2\n";
        let default_renderer = Renderer::new(&Value::empty_object());
        let expected = Renderer::new(&Value::empty_object())
            .render_with(combined, &options)
            .unwrap();
        for _ in 0..3 {
            assert_eq!(
                default_renderer.render_with(combined, &options).unwrap(),
                expected
            );
        }

        // Invalid site limit errors are still deferred to render time and preserved on every call.
        let invalid_limits = Renderer::new(&json(serde_json::json!({ "maxTextSize": "big" })));
        for _ in 0..2 {
            assert!(matches!(
                invalid_limits.render("pie\n  \"A\" : 1\n"),
                Err(Error::Config(msg)) if msg.contains("maxTextSize")
            ));
        }
    }

    #[test]
    fn limits_utf8_fast_path_and_multibyte_utf16_boundaries() {
        // ASCII: UTF-8 byte length == UTF-16 code unit count.
        let renderer = Renderer::new(&json(serde_json::json!({ "maxTextSize": 10 })));
        assert!(renderer.limits("0123456789").is_ok());
        assert!(matches!(
            renderer.limits("01234567890"),
            Err(Error::Options(msg)) if msg.contains("maxTextSize")
        ));

        // 2-byte UTF-8 (é): 6 chars = 12 UTF-8 bytes (> 10), 6 UTF-16 code units (<= 10) -> Ok!
        assert!(renderer.limits("éééééé").is_ok());
        // 10 chars = 20 UTF-8 bytes, 10 UTF-16 code units -> Ok; 11 -> Err.
        assert!(renderer.limits("é".repeat(10).as_str()).is_ok());
        assert!(renderer.limits("é".repeat(11).as_str()).is_err());

        // 3-byte UTF-8 BMP (中): 10 chars = 30 UTF-8 bytes, 10 UTF-16 code units -> Ok; 11 -> Err.
        assert!(renderer.limits("中".repeat(10).as_str()).is_ok());
        assert!(renderer.limits("中".repeat(11).as_str()).is_err());

        // 4-byte UTF-8 astral emoji (😀): each is 2 UTF-16 code units (surrogate pair).
        // 5 emoji = 20 UTF-8 bytes, 10 UTF-16 code units -> Ok; 6 emoji = 12 UTF-16 -> Err.
        assert!(renderer.limits("😀".repeat(5).as_str()).is_ok());
        assert!(renderer.limits("😀".repeat(6).as_str()).is_err());
    }

    #[test]
    fn kept_views_of_directives_render_as_fresh_renderers_do() {
        // More distinct directives than the renderer keeps, each rendered again after the others
        // pushed it out and on several threads at once, against a renderer that has kept nothing.
        let mut texts: Vec<String> = (0..DIRECTIVE_VIEWS_CAPACITY + 3)
            .map(|index| {
                format!(
                    "---\nconfig:\n  pie:\n    textPosition: 0.{index}5\n---\npie\n  \"A\" : 1\n  \"B\" : 2\n"
                )
            })
            .collect();
        texts.push("%%{init: {'theme': 'dark'}}%%\npie\n  \"A\" : 1\n".to_owned());
        texts.push("%%{init: {'fontFamily': 'courier'}}%%\nflowchart LR\n  A --> B\n".to_owned());
        texts
            .push("%%{init: {'fontFamily': 'courier'}}%%\nstateDiagram-v2\n  A --> B\n".to_owned());
        let options = RenderOptions {
            id: Some("test".into()),
            now: Some(1_700_000_000_000),
            ..RenderOptions::default()
        };
        let expected: Vec<_> = texts
            .iter()
            .map(|text| Renderer::new(&Value::empty_object()).render_with(text, &options))
            .map(|svg| svg.map_err(|error| error.to_string()))
            .collect();
        assert!(expected.iter().all(Result::is_ok));
        let renderer = Renderer::new(&Value::empty_object());
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let (renderer, options, texts, expected) = (&renderer, &options, &texts, &expected);
                scope.spawn(move || {
                    for _ in 0..2 {
                        for (text, expected) in texts.iter().zip(expected) {
                            let svg = renderer
                                .render_with(text, options)
                                .map_err(|error| error.to_string());
                            assert_eq!(&svg, expected, "{text}");
                        }
                    }
                });
            }
        });
        assert!(lock(&renderer.directive_views).len() <= DIRECTIVE_VIEWS_CAPACITY);
    }
}
