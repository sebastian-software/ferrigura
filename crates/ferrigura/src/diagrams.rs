//! The diagram types as plugins of a [`Renderer`](crate::Renderer).
//!
//! Each function of this module returns the [`Plugin`] of one diagram type: the kind its detector
//! reports, with the parser and the renderer of that kind. A renderer made with
//! [`Renderer::new`](crate::Renderer::new) registers [`all`] of them. One made with
//! [`Renderer::builder`](crate::Renderer::builder) registers only the plugins it is given, and the
//! parsers, renderers and layout engines of the others are never referenced, so a program that
//! names only the types it draws links only those (decision 0020). The same holds for the
//! configuration: each plugin reads the sections its renderer takes, and only those are typed.

use std::fmt;

use ferrigura_model::config::{MermaidConfig, typed_section};
use ferrigura_model::value::Value;

use ferrigura_render::RenderError;
use ferrigura_render::context::Context;
use ferrigura_render::flowchart;
use ferrigura_render::mindmap::MindmapSection;
use ferrigura_render::pie::render_with_animation as render_pie_diagram;
use ferrigura_render::unified::Section;
use ferrigura_render::{
    class as class_render, er as er_render, gantt as gantt_render, mindmap as mindmap_render,
    quadrant_chart as quadrant_chart_render, sequence as sequence_render, state as state_render,
    xychart as xychart_render,
};
use ferrigura_syntax::{
    DiagramType, class, er, gantt, mindmap, pie, quadrant_chart, sequence, state, xychart,
};

use crate::{Prepared, RenderOptions, sequence_wrap, with_flowchart};

/// The parse-and-render step of one diagram type.
type RenderFn = fn(&Prepared<'_>, &RenderOptions) -> Result<String, RenderError>;

/// Reads the sections of the configuration that one diagram type's renderer takes into a
/// configuration whose other sections keep their defaults.
type ReadFn = fn(&Value, &mut MermaidConfig) -> Result<(), serde_json::Error>;

/// One diagram type that a [`Renderer`](crate::Renderer) draws: the kind its detector reports, with
/// the parser and the renderer of that kind. The functions of this module make them, and
/// [`RendererBuilder::with`](crate::RendererBuilder::with) registers them.
#[derive(Clone, Copy)]
pub struct Plugin {
    kind: DiagramType,
    read: ReadFn,
    render: RenderFn,
}

impl Plugin {
    /// The diagram type this plugin draws.
    #[must_use]
    pub fn kind(&self) -> DiagramType {
        self.kind
    }

    /// Reads the sections of `value` that this plugin's renderer takes into `config`; the other
    /// sections of `config` are left as they are.
    pub(crate) fn read(
        &self,
        value: &Value,
        config: &mut MermaidConfig,
    ) -> Result<(), serde_json::Error> {
        (self.read)(value, config)
    }

    /// Parses the prepared source as a diagram of this plugin's kind and renders it.
    pub(crate) fn render(
        &self,
        prepared: &Prepared<'_>,
        options: &RenderOptions,
    ) -> Result<String, RenderError> {
        ferrigura_text::measure::render_scope(|| (self.render)(prepared, options))
    }
}

impl fmt::Debug for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Plugin")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Every diagram type, which is what [`Renderer::new`](crate::Renderer::new) registers: one plugin
/// for each kind of [`DiagramType::ALL`].
#[must_use]
pub fn all() -> Vec<Plugin> {
    vec![
        pie(),
        flowchart(),
        er(),
        class(),
        sequence(),
        state(),
        gantt(),
        mindmap(),
        xy_chart(),
        quadrant_chart(),
    ]
}

/// The pie diagram.
#[must_use]
pub fn pie() -> Plugin {
    Plugin {
        kind: DiagramType::Pie,
        read: read_pie,
        render: render_pie,
    }
}

fn read_pie(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    config.pie = typed_section(value, "pie")?;
    Ok(())
}

fn render_pie(prepared: &Prepared<'_>, options: &RenderOptions) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    render_pie_diagram(
        &Context {
            fonts: prepared.fonts,
            diagram: &pie::parse_prepared(&prepared.source)?,
            section: &typed_config.pie,
            globals: &prepared.view.globals,
            theme: prepared.theme(),
            id,
        },
        options.animation,
    )
}

/// The flowchart, written `flowchart` or `graph`.
#[must_use]
pub fn flowchart() -> Plugin {
    Plugin {
        kind: DiagramType::Flowchart,
        read: read_flowchart,
        render: render_flowchart,
    }
}

fn read_flowchart(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    read_unified(value, config)?;
    Ok(())
}

fn render_flowchart(
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    with_flowchart(prepared, options.compat, |input| {
        flowchart::render_with_animation(input, options.animation)
    })
}

/// The ER diagram, written `erDiagram`.
#[must_use]
pub fn er() -> Plugin {
    Plugin {
        kind: DiagramType::Er,
        read: read_er,
        render: render_er,
    }
}

fn read_er(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    read_unified(value, config)?;
    Ok(())
}

fn render_er(prepared: &Prepared<'_>, _options: &RenderOptions) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let mut section = Section::of(&prepared.view.config);
    section.design = prepared.design;
    er_render::render(&Context {
        fonts: prepared.fonts,
        diagram: &er::parse_prepared(&prepared.source)?,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The class diagram, written `classDiagram` or `classDiagram-v2`.
#[must_use]
pub fn class() -> Plugin {
    Plugin {
        kind: DiagramType::Class,
        read: read_class,
        render: render_class,
    }
}

fn read_class(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    read_unified(value, config)?;
    Ok(())
}

/// `getEffectiveHtmlLabels(config)` of Mermaid's `config.ts`: root `htmlLabels` wins, falling
/// back to the legacy `flowchart.htmlLabels`, then `true`, evaluated with Mermaid's `evaluate()`.
fn effective_html_labels(effective: &Value) -> bool {
    let Some(raw) = effective
        .get("htmlLabels")
        .filter(|value| !value.is_null())
        .or_else(|| {
            effective
                .get("flowchart")
                .and_then(|flowchart| flowchart.get("htmlLabels"))
                .filter(|value| !value.is_null())
        })
    else {
        return true;
    };
    if matches!(raw, Value::Bool(false)) {
        return false;
    }
    let text = ferrigura_render::common::template_text(raw);
    !matches!(
        ferrigura_render::common::js_trim(&text)
            .to_ascii_lowercase()
            .as_str(),
        "false" | "null" | "0"
    )
}

fn render_class(prepared: &Prepared<'_>, _options: &RenderOptions) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let mut section = Section::of(&prepared.view.config);
    section.design = prepared.design;
    section.roles = ferrigura_render::design::Roles::Class;
    class_render::render(&Context {
        fonts: prepared.fonts,
        diagram: &class::parse_prepared(
            &prepared.source,
            class::Options {
                html_labels: effective_html_labels(&prepared.view.effective),
                security_level: ferrigura_syntax::url::SecurityLevel::from_config(
                    &prepared.view.globals.security_level,
                ),
            },
        )?,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The sequence diagram, written `sequenceDiagram`.
#[must_use]
pub fn sequence() -> Plugin {
    Plugin {
        kind: DiagramType::Sequence,
        read: read_sequence,
        render: render_sequence,
    }
}

fn read_sequence(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    config.sequence = typed_section(value, "sequence")?;
    Ok(())
}

fn render_sequence(
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    sequence_render::render_with_design(
        &Context {
            fonts: prepared.fonts,
            diagram: &sequence::parse_prepared(
                &prepared.source,
                sequence::Options {
                    wrap: sequence_wrap(&prepared.view.effective),
                },
            )?,
            section: &typed_config.sequence,
            globals: &prepared.view.globals,
            theme: prepared.theme(),
            id,
        },
        options.compat,
        prepared.design,
    )
}

/// The state diagram, written `stateDiagram` or `stateDiagram-v2`.
#[must_use]
pub fn state() -> Plugin {
    Plugin {
        kind: DiagramType::State,
        read: read_state,
        render: render_state,
    }
}

fn read_state(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    read_unified(value, config)?;
    Ok(())
}

fn render_state(prepared: &Prepared<'_>, _options: &RenderOptions) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let mut section = Section::of(&prepared.view.config);
    section.design = prepared.design;
    section.roles = ferrigura_render::design::Roles::State;
    state_render::render(&Context {
        fonts: prepared.fonts,
        diagram: &state::parse_prepared(&prepared.source)?,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The gantt diagram.
#[must_use]
pub fn gantt() -> Plugin {
    Plugin {
        kind: DiagramType::Gantt,
        read: read_gantt,
        render: render_gantt,
    }
}

fn read_gantt(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    config.gantt = typed_section(value, "gantt")?;
    Ok(())
}

fn render_gantt(prepared: &Prepared<'_>, options: &RenderOptions) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let now = options.now.unwrap_or_else(|| gantt::Options::default().now);
    gantt_render::render_with_animation(
        &Context {
            fonts: prepared.fonts,
            diagram: &gantt::parse_prepared_with_security(
                &prepared.source,
                gantt::Options { now },
                ferrigura_syntax::url::SecurityLevel::from_config(
                    &prepared.view.globals.security_level,
                ),
            )?,
            section: &gantt_render::GanttSection {
                config: &typed_config.gantt,
                now,
                design: prepared.design,
            },
            globals: &prepared.view.globals,
            theme: prepared.theme(),
            id,
        },
        options.animation,
    )
}

/// The mindmap, written `mindmap`.
#[must_use]
pub fn mindmap() -> Plugin {
    Plugin {
        kind: DiagramType::Mindmap,
        read: read_mindmap,
        render: render_mindmap,
    }
}

fn read_mindmap(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    read_unified(value, config)?;
    config.mindmap = typed_section(value, "mindmap")?;
    Ok(())
}

fn render_mindmap(prepared: &Prepared<'_>, options: &RenderOptions) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let mut section = Section::of(&prepared.view.config);
    section.design = prepared.design;
    mindmap_render::render_with_animation(
        &Context {
            fonts: prepared.fonts,
            diagram: &mindmap::parse_prepared(&prepared.source, &prepared.directive)?,
            section: &MindmapSection {
                mindmap: &typed_config.mindmap,
                unified: section,
                look: prepared.design,
            },
            globals: &prepared.view.globals,
            theme: prepared.theme(),
            id,
        },
        options.animation,
    )
}

/// The xychart, written `xychart`.
#[must_use]
pub fn xy_chart() -> Plugin {
    Plugin {
        kind: DiagramType::XyChart,
        read: read_xy_chart,
        render: render_xy_chart,
    }
}

fn read_xy_chart(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    config.xy_chart = typed_section(value, "xyChart")?;
    Ok(())
}

fn render_xy_chart(
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    // The design mode draws the plots in its palette, unless the text gives one: the parser reads
    // the palette from the text's configuration only.
    let designed = prepared
        .design
        .and_then(|_| design_palette_directive(&prepared.directive, prepared.theme().light));
    xychart_render::render_with_design(
        &Context {
            fonts: prepared.fonts,
            diagram: &xychart::parse_prepared(
                &prepared.source,
                designed.as_ref().unwrap_or(&prepared.directive),
            )?,
            section: &typed_config.xy_chart,
            globals: &prepared.view.globals,
            theme: prepared.theme(),
            id,
        },
        options.animation,
        prepared.design,
    )
}

/// The text's configuration with the palette of the theme the render draws with as the XY chart's
/// `plotColorPalette`, or `None` when the text gives a palette of its own or the theme none.
fn design_palette_directive(
    directive: &Value,
    tokens: &ferrigura_model::theme::ThemeTokens,
) -> Option<Value> {
    let given = directive
        .get("themeVariables")
        .and_then(|variables| variables.get("xyChart"))
        .and_then(|group| group.get("plotColorPalette"))
        .is_some();
    let palette = tokens.xy_chart.as_ref()?.plot_color_palette.text()?;
    if given {
        return None;
    }
    let mut directive = directive.clone();
    let Value::Object(map) = &mut directive else {
        return None;
    };
    let variables = map
        .entry("themeVariables".to_owned())
        .or_insert_with(Value::empty_object);
    let Value::Object(variables) = variables else {
        return None;
    };
    let group = variables
        .entry("xyChart".to_owned())
        .or_insert_with(Value::empty_object);
    let Value::Object(group) = group else {
        return None;
    };
    group.insert("plotColorPalette".to_owned(), Value::from(palette));
    Some(directive)
}

/// The quadrant chart, written `quadrantChart`.
#[must_use]
pub fn quadrant_chart() -> Plugin {
    Plugin {
        kind: DiagramType::QuadrantChart,
        read: read_quadrant_chart,
        render: render_quadrant_chart,
    }
}

fn read_quadrant_chart(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    config.quadrant_chart = typed_section(value, "quadrantChart")?;
    Ok(())
}

fn render_quadrant_chart(
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    quadrant_chart_render::render(&Context {
        fonts: prepared.fonts,
        diagram: &quadrant_chart::parse_prepared(&prepared.source)?,
        section: &typed_config.quadrant_chart,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// Reads the sections that the unified renderer takes (`unified::Section`): `flowchart`, `elk`,
/// `class`, `er` and `state`.
fn read_unified(value: &Value, config: &mut MermaidConfig) -> Result<(), serde_json::Error> {
    config.flowchart = typed_section(value, "flowchart")?;
    config.elk = typed_section(value, "elk")?;
    config.class = typed_section(value, "class")?;
    config.er = typed_section(value, "er")?;
    config.state = typed_section(value, "state")?;
    Ok(())
}
