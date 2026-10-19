//! ferrigura: renders Mermaid diagram text to SVG, without a browser.
//!
//! [`Renderer`] is the interface. It takes the configuration the user would pass to `initialize`
//! once, when it is made, and resolves what its renders share: the configuration merged onto the
//! defaults and typed, and the variables of both shipped themes. A render merges only the diagram's
//! own front matter and directives on top of that, and only when there are any. [`render`] and
//! [`render_with`] take the configuration with each call. They are a shorthand over a small cache of
//! renderers, so a configuration that is rendered again is resolved once. The output is the same
//! either way.
//!
//! The text goes in and an SVG document comes out. [`RenderOptions`] holds what is not Mermaid
//! configuration: the id of the SVG and the colour scheme.

#![warn(missing_docs)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hasher;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ferrigura_model::config::{MermaidConfig, SiteConfig, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::flowchart::{self, FlowchartInput};
use ferrigura_render::pie::render as render_pie;
use ferrigura_render::theme::Themes;
use ferrigura_render::{
    architecture as architecture_render, block as block_render, c4 as c4_render,
    class as class_render, er as er_render, event_modeling as event_modeling_render,
    gantt as gantt_render, git_graph as git_graph_render, info as info_render,
    ishikawa as ishikawa_render, journey as journey_render, kanban as kanban_render,
    mindmap as mindmap_render, packet as packet_render, quadrant_chart as quadrant_chart_render,
    requirement as requirement_render, sankey as sankey_render, sequence as sequence_render,
    state as state_render, timeline as timeline_render, treemap as treemap_render,
    xychart as xychart_render,
};
use ferrigura_syntax::{
    DiagramType, ParseError, architecture, block, c4, class, config, detect, er, event_modeling,
    gantt, git_graph, info, ishikawa, journey, kanban, mindmap, packet, pie, quadrant_chart,
    requirement, sankey, sequence, state, timeline, treemap, xychart,
};

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
    /// The current time, in milliseconds since the epoch, for the dates that depend on it (the
    /// gantt diagram's today marker and dates without a day). `None` reads the system clock.
    pub now: Option<i64>,
    /// The seed of the ids a gitGraph generates for its commits, merges and cherry-picks that have
    /// no `id:`. `None` derives them from the sequence number. The geometry harness gives the
    /// reference's seed, so that the ids match the reference's; it is not part of the interface.
    #[doc(hidden)]
    pub git_graph_seed: Option<u32>,
}

/// A renderer, with the site configuration resolved once.
///
/// [`Renderer::new`] takes the configuration the user would pass to `initialize`. The renderer holds
/// it merged onto the defaults and typed, and the variables of the light and the dark theme with the
/// user's `themeVariables`. Its renders merge only the diagram's own front matter and directives on
/// top, and only when there are any: a text without them is drawn with the resolved configuration as
/// it is. The typed view of a diagram type is resolved at its first render and kept after that.
///
/// The configuration of a renderer does not change once it is made, and the renderer is `Send` and
/// `Sync`, so one renderer can serve renders on several threads at once. For the same configuration,
/// its output is the output of [`render_with`], byte for byte.
///
/// ```
/// use ferrigura::{RenderOptions, Renderer};
/// use ferrigura_model::value::Value;
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
    /// The configuration as it serialises, which the derived id hashes.
    configuration: String,
    /// The user's theme overrides, which the two schemes below are computed with.
    overrides: Value,
    /// The variables of `redux-color` with the overrides: the light scheme.
    light: Cow<'static, Value>,
    /// The variables of `redux-dark-color` with the overrides: the dark scheme.
    dark: Cow<'static, Value>,
    /// The tokens of `light`, built once with the renderer.
    light_tokens: ThemeTokens,
    /// The tokens of `dark`, built once with the renderer.
    dark_tokens: ThemeTokens,
    /// The views of the diagram types, made at their first render. A view that cannot be resolved
    /// keeps its error, which the render reports as it always has.
    views: Mutex<HashMap<DiagramType, Result<Arc<View>, String>>>,
}

impl Renderer {
    /// Resolves the configuration of a renderer. `config` is what the user would pass to
    /// `initialize`, as a [`Value`]; an empty object gives the defaults.
    ///
    /// Making a renderer does not fail. A value of the wrong type is reported by the render that reads
    /// it, as [`render`] reports it.
    #[must_use]
    pub fn new(config: &Value) -> Self {
        let site = SiteConfig::new(Some(config));
        let overrides = site.user_theme_variables(&Value::empty_object());
        let light = Theme::ReduxColor.variables_shared(Some(&overrides));
        let dark = Theme::ReduxDarkColor.variables_shared(Some(&overrides));
        Self {
            light_tokens: ThemeTokens::from_value(&light),
            dark_tokens: ThemeTokens::from_value(&dark),
            light,
            dark,
            configuration: serde_json::to_string(config).unwrap_or_default(),
            overrides,
            site,
            views: Mutex::new(HashMap::new()),
        }
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
        let prepared = self.prepare(text, options, kind)?;
        let (typed_config, themes, id) = (
            &prepared.view.config,
            prepared.themes(),
            prepared.id.as_str(),
        );
        match kind {
            DiagramType::Pie => render_pie(&Context {
                diagram: &pie::parse(text)?,
                section: &typed_config.pie,
                globals: &prepared.view.globals,
                theme: prepared.theme(),
                id,
            }),
            DiagramType::Info => info_render::render(&Context {
                diagram: &info::parse(text)?,
                section: &(),
                globals: &prepared.view.globals,
                theme: prepared.theme(),
                id,
            }),
            DiagramType::Timeline => timeline_render::render(&Context {
                diagram: &timeline::parse(text)?,
                section: &typed_config.timeline,
                globals: &prepared.view.globals,
                theme: prepared.theme(),
                id,
            }),
            DiagramType::Journey => journey_render::render(&Context {
                diagram: &journey::parse(text)?,
                section: &typed_config.journey,
                globals: &prepared.view.globals,
                theme: prepared.theme(),
                id,
            }),
            DiagramType::GitGraph => git_graph_render::render(&git_graph_render::GitGraphInput {
                diagram: &git_graph::parse_with(
                    text,
                    &git_graph::Options {
                        auto_id_seed: options.git_graph_seed,
                        ..git_graph::options_of(text)?
                    },
                )?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Er => er_render::render(&er_render::ErInput {
                diagram: &er::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Class => class_render::render(&class_render::ClassInput {
                diagram: &class::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Sequence => sequence_render::render(&Context {
                diagram: &sequence::parse_with(
                    text,
                    sequence::Options {
                        wrap: sequence_wrap(&prepared.view.effective),
                    },
                )?,
                section: &typed_config.sequence,
                globals: &prepared.view.globals,
                theme: prepared.theme(),
                id,
            }),
            DiagramType::State => state_render::render(&state_render::StateInput {
                diagram: &state::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Gantt => {
                let now = options.now.unwrap_or_else(|| gantt::Options::default().now);
                gantt_render::render(&Context {
                    diagram: &gantt::parse_with(text, gantt::Options { now })?,
                    section: &gantt_render::GanttSection {
                        config: &typed_config.gantt,
                        now,
                    },
                    globals: &prepared.view.globals,
                    theme: prepared.theme(),
                    id,
                })
            }
            DiagramType::Mindmap => mindmap_render::render(&mindmap_render::MindmapInput {
                diagram: &mindmap::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Block => block_render::render(&block_render::BlockInput {
                diagram: &block::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Packet => packet_render::render(&packet_render::PacketInput {
                diagram: &packet::parse_with(text, &prepared.view.effective)?,
                config: typed_config,
                themes,
                id,
                bits_per_row: packet::bits_per_row(&prepared.view.effective),
            }),
            DiagramType::XyChart => xychart_render::render(&xychart_render::XyChartInput {
                diagram: &xychart::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Sankey => sankey_render::render(&sankey_render::SankeyInput {
                diagram: &sankey::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::QuadrantChart => {
                quadrant_chart_render::render(&quadrant_chart_render::QuadrantChartInput {
                    diagram: &quadrant_chart::parse(text)?,
                    config: typed_config,
                    themes,
                    id,
                })
            }
            DiagramType::Requirement => {
                requirement_render::render(&requirement_render::RequirementInput {
                    diagram: &requirement::parse(text)?,
                    config: typed_config,
                    themes,
                    id,
                })
            }
            DiagramType::C4 => c4_render::render(&c4_render::C4Input {
                diagram: &c4::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Kanban => kanban_render::render(&kanban_render::KanbanInput {
                diagram: &kanban::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::Architecture => {
                architecture_render::render(&architecture_render::ArchitectureInput {
                    diagram: &architecture::parse(text)?,
                    config: typed_config,
                    themes,
                    id,
                })
            }
            DiagramType::Treemap => treemap_render::render(&treemap_render::TreemapInput {
                diagram: &treemap::parse(text)?,
                config: typed_config,
                settings: &treemap_render::TreemapSettings::of(&prepared.view.effective),
                themes,
                id,
            }),
            DiagramType::Ishikawa => ishikawa_render::render(&ishikawa_render::IshikawaInput {
                diagram: &ishikawa::parse(text)?,
                config: typed_config,
                themes,
                id,
            }),
            DiagramType::EventModeling => {
                event_modeling_render::render(&event_modeling_render::EventModelingInput {
                    diagram: &event_modeling::parse(text)?,
                    config: typed_config,
                    themes,
                    id,
                })
            }
            DiagramType::Flowchart => with_flowchart(text, &prepared, flowchart::render),
        }
    }

    /// What a render needs besides its diagram, as `mermaidAPI` resolves it before it calls the
    /// diagram's renderer: the id, the configuration in the diagram's scope, and the theme variables
    /// of the schemes that the render draws.
    ///
    /// A text without front matter or directives uses the view of its diagram type and the themes
    /// of the renderer as they are. Otherwise the directive is merged on top here, and the themes are
    /// computed again only when the directive changes the user's overrides.
    fn prepare(
        &self,
        text: &str,
        options: &RenderOptions,
        kind: DiagramType,
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
                view: self.view(kind)?,
                light: Cow::Borrowed(&*self.light),
                dark: Cow::Borrowed(&*self.dark),
                light_tokens: Cow::Borrowed(&self.light_tokens),
                dark_tokens: Cow::Borrowed(&self.dark_tokens),
                scheme,
            });
        }
        let view = View::resolve(&self.site, kind, &directive).map_err(RenderError::Config)?;
        let overrides = self.site.user_theme_variables(&directive);
        let (light, dark, light_tokens, dark_tokens) = if overrides == self.overrides {
            (
                Cow::Borrowed(&*self.light),
                Cow::Borrowed(&*self.dark),
                Cow::Borrowed(&self.light_tokens),
                Cow::Borrowed(&self.dark_tokens),
            )
        } else {
            let light = scheme_variables(
                Theme::ReduxColor,
                matches!(scheme, Some(ColorScheme::Light | ColorScheme::Auto)),
                &overrides,
            );
            let dark = scheme_variables(
                Theme::ReduxDarkColor,
                matches!(scheme, Some(ColorScheme::Dark | ColorScheme::Auto)),
                &overrides,
            );
            let light_tokens = Cow::Owned(ThemeTokens::from_value(&light));
            let dark_tokens = Cow::Owned(ThemeTokens::from_value(&dark));
            (light, dark, light_tokens, dark_tokens)
        };
        Ok(Prepared {
            id,
            view: Arc::new(view),
            light,
            dark,
            light_tokens,
            dark_tokens,
            scheme,
        })
    }

    /// The view of a diagram type with no directive, resolved at its first render.
    fn view(&self, kind: DiagramType) -> Result<Arc<View>, RenderError> {
        let known = lock(&self.views).get(&kind).cloned();
        let resolved = match known {
            Some(resolved) => resolved,
            None => {
                let resolved =
                    View::resolve(&self.site, kind, &Value::empty_object()).map(Arc::new);
                lock(&self.views).entry(kind).or_insert(resolved).clone()
            }
        };
        resolved.map_err(RenderError::Config)
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
    shorthand(config).render_with(text, options)
}

/// The view of a diagram type, its configuration in the diagram's scope and typed, the variables of
/// its configured theme with their tokens, and the globals the renderers read.
#[derive(Debug)]
struct View {
    /// The configuration in the diagram's scope, as the renderers that read it untyped see it.
    effective: Value,
    /// The same configuration, typed.
    config: MermaidConfig,
    /// The `themeVariables` of the configuration: the variables of the configured theme.
    configured: Value,
    /// The tokens of `configured`.
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
    fn resolve(site: &SiteConfig, kind: DiagramType, directive: &Value) -> Result<Self, String> {
        let effective = site.effective(Some(kind.config_key()), directive);
        let mut config = typed(&effective).map_err(|error| error.to_string())?;
        // `getData` sets the mindmap's layout to cose-bilkent unless the top-level `layout` is set by the
        // user (`mindmapDb.ts` 361-367): the schema's default, `elk`, does not reach a mindmap.
        if matches!(kind, DiagramType::Mindmap)
            && !user_sets_layout(Some(site.settings()), directive)
        {
            config.layout = "cose-bilkent".to_owned();
        }
        let configured = effective
            .get("themeVariables")
            .cloned()
            .unwrap_or_else(Value::empty_object);
        Ok(Self {
            globals: Globals::of(&config),
            configured_tokens: ThemeTokens::from_value(&configured),
            effective,
            config,
            configured,
        })
    }
}

/// What a render needs besides its diagram: see [`Renderer::prepare`].
struct Prepared<'a> {
    id: String,
    view: Arc<View>,
    light: Cow<'a, Value>,
    dark: Cow<'a, Value>,
    light_tokens: Cow<'a, ThemeTokens>,
    dark_tokens: Cow<'a, ThemeTokens>,
    scheme: Option<ColorScheme>,
}

impl Prepared<'_> {
    /// The theme of the render as typed tokens: the scheme that the colour scheme chooses, as
    /// `themes` gives it, with the light scheme as the primary one.
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

    /// The theme variables that the render draws with, as the colour scheme chooses them.
    fn themes(&self) -> Themes<'_> {
        match self.scheme {
            None => Themes::Single(&self.view.configured),
            Some(ColorScheme::Light) => Themes::Single(&self.light),
            Some(ColorScheme::Dark) => Themes::Single(&self.dark),
            Some(ColorScheme::Auto) => Themes::Schemes {
                light: &self.light,
                dark: &self.dark,
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

/// The variables of a colour scheme, when the render draws it. A scheme that is not drawn is not
/// computed: the configured theme decides the colours then, and the scheme gives no variables.
fn scheme_variables(theme: Theme, drawn: bool, overrides: &Value) -> Cow<'static, Value> {
    if drawn {
        theme.variables_shared(Some(overrides))
    } else {
        Cow::Owned(Value::empty_object())
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
    let renderer = shorthand(config);
    let prepared = renderer.prepare(text, options, DiagramType::Flowchart)?;
    let stages = with_flowchart(text, &prepared, flowchart::elk_stages)?;
    Ok(serde_json::json!({
        "elkInput": stages.input,
        "elkOutput": stages.output,
    }))
}

/// Parses a flowchart and hands it, with its configuration and theme, to `step`.
fn with_flowchart<T>(
    text: &str,
    prepared: &Prepared<'_>,
    step: impl FnOnce(&FlowchartInput<'_>) -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    let diagram = ferrigura_syntax::flowchart::parse(text)
        .map_err(|ferrigura_syntax::flowchart::FlowchartError::Syntax(error)| error)?;
    step(&FlowchartInput {
        diagram: &diagram,
        config: &prepared.view.config,
        themes: prepared.themes(),
        id: &prepared.id,
    })
}

/// The renderers of [`render`] and [`render_with`], with their configurations, least recently used
/// first.
static SHORTHAND: Mutex<Vec<(Value, Arc<Renderer>)>> = Mutex::new(Vec::new());

/// How many configurations the shorthand keeps a renderer for. A configuration that is not kept is
/// resolved again at its next render; the output does not depend on it.
const SHORTHAND_CAPACITY: usize = 8;

/// The renderer of a configuration, from the shorthand's cache or made for it, and kept there.
fn shorthand(config: &Value) -> Arc<Renderer> {
    if let Some(renderer) = kept(config) {
        return renderer;
    }
    // Two threads that miss at once keep two equal renderers for a while; that costs memory only.
    let renderer = Arc::new(Renderer::new(config));
    let mut cache = lock(&SHORTHAND);
    if cache.len() == SHORTHAND_CAPACITY {
        cache.remove(0);
    }
    cache.push((config.clone(), Arc::clone(&renderer)));
    renderer
}

/// The renderer that the shorthand keeps for a configuration, which becomes the most recently used.
fn kept(config: &Value) -> Option<Arc<Renderer>> {
    let mut cache = lock(&SHORTHAND);
    let at = cache.iter().position(|(key, _)| identical(key, config))?;
    let entry = cache.remove(at);
    let renderer = Arc::clone(&entry.1);
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
/// configuration as it serialises, so the same input always gives the same id.
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
