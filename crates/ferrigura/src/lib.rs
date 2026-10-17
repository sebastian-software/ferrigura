//! ferrigura: renders Mermaid diagram text to SVG, without a browser.
//!
//! [`render`] is the whole interface: the diagram text, and the configuration the user would pass
//! to `initialize`, go in, and an SVG document comes out. [`render_with`] takes the options that
//! are not Mermaid configuration: the id of the SVG and the colour scheme.

#![warn(missing_docs)]

use std::hash::Hasher;

use ferrigura_model::config::{MermaidConfig, effective, typed, user_theme_variables};
use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::flowchart::{self, FlowchartInput};
use ferrigura_render::pie::{PieInput, render as render_pie};
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
    let Some(kind) = detect(text) else {
        return Err(RenderError::Parse(ParseError {
            line: 1,
            column: 1,
            message: "no diagram type detected".to_owned(),
        }));
    };
    let prepared = prepare(text, config, options, kind)?;
    let (typed_config, themes, id) = (&prepared.config, prepared.themes(), prepared.id.as_str());
    match kind {
        DiagramType::Pie => render_pie(&PieInput {
            diagram: &pie::parse(text)?,
            config: &typed_config.pie,
            themes,
            id,
        }),
        DiagramType::Info => info_render::render(&info_render::InfoInput {
            diagram: &info::parse(text)?,
            config: typed_config,
            themes,
            id,
        }),
        DiagramType::Timeline => timeline_render::render(&timeline_render::TimelineInput {
            diagram: &timeline::parse(text)?,
            config: typed_config,
            themes,
            id,
        }),
        DiagramType::Journey => journey_render::render(&journey_render::JourneyInput {
            diagram: &journey::parse(text)?,
            config: typed_config,
            themes,
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
        DiagramType::Sequence => sequence_render::render(&sequence_render::SequenceInput {
            diagram: &sequence::parse_with(
                text,
                sequence::Options {
                    wrap: sequence_wrap(&prepared.effective),
                },
            )?,
            config: typed_config,
            themes,
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
            gantt_render::render(&gantt_render::GanttInput {
                diagram: &gantt::parse_with(text, gantt::Options { now })?,
                config: typed_config,
                themes,
                id,
                now,
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
            diagram: &packet::parse(text)?,
            config: typed_config,
            themes,
            id,
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

/// What every renderer needs besides its diagram: the id, the effective configuration in the
/// diagram's scope, and the theme variables of each scheme.
struct Prepared {
    id: String,
    effective: Value,
    config: MermaidConfig,
    configured: Value,
    light: Value,
    dark: Value,
    scheme: Option<ColorScheme>,
}

impl Prepared {
    fn themes(&self) -> Themes<'_> {
        match self.scheme {
            None => Themes::Single(&self.configured),
            Some(ColorScheme::Light) => Themes::Single(&self.light),
            Some(ColorScheme::Dark) => Themes::Single(&self.dark),
            Some(ColorScheme::Auto) => Themes::Schemes {
                light: &self.light,
                dark: &self.dark,
            },
        }
    }
}

/// Resolves the id, the configuration and the themes of a render, as `mermaidAPI` does before it
/// calls the diagram's renderer.
fn prepare(
    text: &str,
    site: &Value,
    options: &RenderOptions,
    kind: DiagramType,
) -> Result<Prepared, RenderError> {
    let id = match &options.id {
        Some(id) if valid_id(id) => id.clone(),
        Some(id) => return Err(RenderError::Options(format!("invalid id {id:?}"))),
        None => derived_id(text, site),
    };
    let directive = config::read(text)?;
    let effective_config = effective(Some(site), Some(kind.config_key()), &directive);
    let typed_config =
        typed(&effective_config).map_err(|error| RenderError::Config(error.to_string()))?;
    let configured = effective_config
        .get("themeVariables")
        .cloned()
        .unwrap_or_else(Value::empty_object);
    let overrides = user_theme_variables(Some(site), &directive);
    Ok(Prepared {
        id,
        config: typed_config,
        configured,
        light: Theme::ReduxColor.variables(Some(&overrides)),
        dark: Theme::ReduxDarkColor.variables(Some(&overrides)),
        scheme: options.color_scheme,
        effective: effective_config,
    })
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
    let prepared = prepare(text, config, options, DiagramType::Flowchart)?;
    let stages = with_flowchart(text, &prepared, flowchart::elk_stages)?;
    Ok(serde_json::json!({
        "elkInput": stages.input,
        "elkOutput": stages.output,
    }))
}

/// Parses a flowchart and hands it, with its configuration and theme, to `step`.
fn with_flowchart<T>(
    text: &str,
    prepared: &Prepared,
    step: impl FnOnce(&FlowchartInput<'_>) -> Result<T, RenderError>,
) -> Result<T, RenderError> {
    let diagram = ferrigura_syntax::flowchart::parse(text)
        .map_err(|ferrigura_syntax::flowchart::FlowchartError::Syntax(error)| error)?;
    step(&FlowchartInput {
        diagram: &diagram,
        config: &prepared.config,
        themes: prepared.themes(),
        id: &prepared.id,
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
