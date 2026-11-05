//! The diagram types as plugins of a [`Renderer`](crate::Renderer).
//!
//! Each function of this module returns the [`Plugin`] of one diagram type: the kind its detector
//! reports, with the parser and the renderer of that kind. A renderer made with
//! [`Renderer::new`](crate::Renderer::new) registers [`all`] of them. One made with
//! [`Renderer::builder`](crate::Renderer::builder) registers only the plugins it is given, and the
//! parsers, renderers and layout engines of the others are never referenced, so a program that
//! names only the types it draws links only those (decision 0020).

use std::fmt;

use ferrigura_render::RenderError;
use ferrigura_render::agentflow::AgentflowSection;
use ferrigura_render::block::BlockSection;
use ferrigura_render::context::Context;
use ferrigura_render::flowchart;
use ferrigura_render::kanban::KanbanSections;
use ferrigura_render::mindmap::MindmapSection;
use ferrigura_render::pie::render as render_pie_diagram;
use ferrigura_render::swimlanes::SwimlaneSection;
use ferrigura_render::unified::Section;
use ferrigura_render::use_case::UseCaseSection;
use ferrigura_render::{
    agentflow as agentflow_render, architecture as architecture_render, block as block_render,
    c4 as c4_render, class as class_render, cynefin as cynefin_render, er as er_render,
    event_modeling as event_modeling_render, gantt as gantt_render, git_graph as git_graph_render,
    info as info_render, ishikawa as ishikawa_render, journey as journey_render,
    kanban as kanban_render, mindmap as mindmap_render, packet as packet_render,
    quadrant_chart as quadrant_chart_render, radar as radar_render, railroad as railroad_render,
    requirement as requirement_render, sankey as sankey_render, sequence as sequence_render,
    state as state_render, swimlanes as swimlanes_render, timeline as timeline_render,
    tree_view as tree_view_render, treemap as treemap_render, use_case as use_case_render,
    venn as venn_render, wardley as wardley_render, xychart as xychart_render,
};
use ferrigura_syntax::{
    DiagramType, agentflow, architecture, block, c4, class, cynefin, er, event_modeling, gantt,
    git_graph, info, ishikawa, journey, kanban, mindmap, packet, pie, quadrant_chart, radar,
    railroad, requirement, sankey, sequence, state, swimlanes, timeline, tree_view, treemap,
    use_case, venn, wardley, xychart,
};

use crate::{Prepared, RenderOptions, sequence_wrap, with_flowchart};

/// The parse-and-render step of one diagram type.
type RenderFn = fn(&str, &Prepared<'_>, &RenderOptions) -> Result<String, RenderError>;

/// One diagram type that a [`Renderer`](crate::Renderer) draws: the kind its detector reports, with
/// the parser and the renderer of that kind. The functions of this module make them, and
/// [`RendererBuilder::with`](crate::RendererBuilder::with) registers them.
#[derive(Clone, Copy)]
pub struct Plugin {
    kind: DiagramType,
    render: RenderFn,
}

impl Plugin {
    /// The diagram type this plugin draws.
    #[must_use]
    pub fn kind(&self) -> DiagramType {
        self.kind
    }

    /// Parses `text` as a diagram of this plugin's kind and renders it.
    pub(crate) fn render(
        &self,
        text: &str,
        prepared: &Prepared<'_>,
        options: &RenderOptions,
    ) -> Result<String, RenderError> {
        (self.render)(text, prepared, options)
    }
}

impl fmt::Debug for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Plugin")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Every diagram type, which is what [`Renderer::new`](crate::Renderer::new) registers.
#[must_use]
pub fn all() -> Vec<Plugin> {
    vec![
        pie(),
        info(),
        flowchart(),
        timeline(),
        journey(),
        git_graph(),
        er(),
        class(),
        sequence(),
        state(),
        gantt(),
        mindmap(),
        block(),
        packet(),
        xy_chart(),
        sankey(),
        quadrant_chart(),
        requirement(),
        c4(),
        kanban(),
        architecture(),
        treemap(),
        ishikawa(),
        event_modeling(),
        agentflow(),
        cynefin(),
        radar(),
        railroad(),
        swimlanes(),
        tree_view(),
        use_case(),
        venn(),
        wardley(),
    ]
}

/// The pie diagram.
#[must_use]
pub fn pie() -> Plugin {
    Plugin {
        kind: DiagramType::Pie,
        render: render_pie,
    }
}

fn render_pie(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    render_pie_diagram(&Context {
        diagram: &pie::parse(text)?,
        section: &typed_config.pie,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The info diagram.
#[must_use]
pub fn info() -> Plugin {
    Plugin {
        kind: DiagramType::Info,
        render: render_info,
    }
}

fn render_info(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    info_render::render(&Context {
        diagram: &info::parse(text)?,
        section: &(),
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The flowchart, written `flowchart` or `graph`.
#[must_use]
pub fn flowchart() -> Plugin {
    Plugin {
        kind: DiagramType::Flowchart,
        render: render_flowchart,
    }
}

fn render_flowchart(
    text: &str,
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    with_flowchart(text, prepared, options.flowchart_mode, flowchart::render)
}

/// The timeline.
#[must_use]
pub fn timeline() -> Plugin {
    Plugin {
        kind: DiagramType::Timeline,
        render: render_timeline,
    }
}

fn render_timeline(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    timeline_render::render(&Context {
        diagram: &timeline::parse(text)?,
        section: &typed_config.timeline,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The user journey, written `journey`.
#[must_use]
pub fn journey() -> Plugin {
    Plugin {
        kind: DiagramType::Journey,
        render: render_journey,
    }
}

fn render_journey(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    journey_render::render(&Context {
        diagram: &journey::parse(text)?,
        section: &typed_config.journey,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The gitGraph diagram.
#[must_use]
pub fn git_graph() -> Plugin {
    Plugin {
        kind: DiagramType::GitGraph,
        render: render_git_graph,
    }
}

fn render_git_graph(
    text: &str,
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    git_graph_render::render(&Context {
        diagram: &git_graph::parse_with(
            text,
            &git_graph::Options {
                auto_id_seed: options.git_graph_seed,
                ..git_graph::options_of(text)?
            },
        )?,
        section: &typed_config.git_graph,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The ER diagram, written `erDiagram`.
#[must_use]
pub fn er() -> Plugin {
    Plugin {
        kind: DiagramType::Er,
        render: render_er,
    }
}

fn render_er(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    er_render::render(&Context {
        diagram: &er::parse(text)?,
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
        render: render_class,
    }
}

fn render_class(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    class_render::render(&Context {
        diagram: &class::parse(text)?,
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
        render: render_sequence,
    }
}

fn render_sequence(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    sequence_render::render(&Context {
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
    })
}

/// The state diagram, written `stateDiagram` or `stateDiagram-v2`.
#[must_use]
pub fn state() -> Plugin {
    Plugin {
        kind: DiagramType::State,
        render: render_state,
    }
}

fn render_state(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    state_render::render(&Context {
        diagram: &state::parse(text)?,
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
        render: render_gantt,
    }
}

fn render_gantt(
    text: &str,
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
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

/// The mindmap, written `mindmap`.
#[must_use]
pub fn mindmap() -> Plugin {
    Plugin {
        kind: DiagramType::Mindmap,
        render: render_mindmap,
    }
}

fn render_mindmap(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    mindmap_render::render(&Context {
        diagram: &mindmap::parse(text)?,
        section: &MindmapSection {
            mindmap: &typed_config.mindmap,
            unified: section,
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The block diagram, written `block`.
#[must_use]
pub fn block() -> Plugin {
    Plugin {
        kind: DiagramType::Block,
        render: render_block,
    }
}

fn render_block(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    block_render::render(&Context {
        diagram: &block::parse(text)?,
        section: &BlockSection {
            block: &typed_config.block,
            unified: section,
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The packet diagram, written `packet`.
#[must_use]
pub fn packet() -> Plugin {
    Plugin {
        kind: DiagramType::Packet,
        render: render_packet,
    }
}

fn render_packet(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let section = packet_render::PacketSection {
        config: typed_config.packet.clone(),
        bits_per_row: packet::bits_per_row(&prepared.view.effective),
    };
    packet_render::render(&Context {
        diagram: &packet::parse_with(text, &prepared.view.effective)?,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The xychart, written `xychart`.
#[must_use]
pub fn xy_chart() -> Plugin {
    Plugin {
        kind: DiagramType::XyChart,
        render: render_xy_chart,
    }
}

fn render_xy_chart(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    xychart_render::render(&Context {
        diagram: &xychart::parse(text)?,
        section: &typed_config.xy_chart,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The sankey diagram, written `sankey`.
#[must_use]
pub fn sankey() -> Plugin {
    Plugin {
        kind: DiagramType::Sankey,
        render: render_sankey,
    }
}

fn render_sankey(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    sankey_render::render(&Context {
        diagram: &sankey::parse(text)?,
        section: &typed_config.sankey,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The quadrant chart, written `quadrantChart`.
#[must_use]
pub fn quadrant_chart() -> Plugin {
    Plugin {
        kind: DiagramType::QuadrantChart,
        render: render_quadrant_chart,
    }
}

fn render_quadrant_chart(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    quadrant_chart_render::render(&Context {
        diagram: &quadrant_chart::parse(text)?,
        section: &typed_config.quadrant_chart,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The requirement diagram, written `requirementDiagram`.
#[must_use]
pub fn requirement() -> Plugin {
    Plugin {
        kind: DiagramType::Requirement,
        render: render_requirement,
    }
}

fn render_requirement(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    requirement_render::render(&Context {
        diagram: &requirement::parse(text)?,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The C4 diagram, written with one of its five headers: `C4Context`, `C4Container`, `C4Component`, `C4Dynamic` or `C4Deployment`.
#[must_use]
pub fn c4() -> Plugin {
    Plugin {
        kind: DiagramType::C4,
        render: render_c4,
    }
}

fn render_c4(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    c4_render::render(&Context {
        diagram: &c4::parse(text)?,
        section: &typed_config.c4,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The kanban board, written `kanban`.
#[must_use]
pub fn kanban() -> Plugin {
    Plugin {
        kind: DiagramType::Kanban,
        render: render_kanban,
    }
}

fn render_kanban(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    kanban_render::render(&Context {
        diagram: &kanban::parse(text)?,
        section: &KanbanSections {
            kanban: &typed_config.kanban,
            mindmap: &typed_config.mindmap,
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The architecture diagram, written `architecture` or `architecture-beta`.
#[must_use]
pub fn architecture() -> Plugin {
    Plugin {
        kind: DiagramType::Architecture,
        render: render_architecture,
    }
}

fn render_architecture(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    architecture_render::render(&Context {
        diagram: &architecture::parse(text)?,
        section: &typed_config.architecture,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The treemap, written `treemap` or `treemap-beta`.
#[must_use]
pub fn treemap() -> Plugin {
    Plugin {
        kind: DiagramType::Treemap,
        render: render_treemap,
    }
}

fn render_treemap(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let id = prepared.id.as_str();
    let section = treemap_render::TreemapSettings::of(&prepared.view.effective);
    treemap_render::render(&Context {
        diagram: &treemap::parse(text)?,
        section: &section,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The ishikawa diagram, written `ishikawa` or `ishikawa-beta`, in any letter case.
#[must_use]
pub fn ishikawa() -> Plugin {
    Plugin {
        kind: DiagramType::Ishikawa,
        render: render_ishikawa,
    }
}

fn render_ishikawa(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    ishikawa_render::render(&Context {
        diagram: &ishikawa::parse(text)?,
        section: &typed_config.ishikawa,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The event modeling diagram, written `eventmodeling`.
#[must_use]
pub fn event_modeling() -> Plugin {
    Plugin {
        kind: DiagramType::EventModeling,
        render: render_event_modeling,
    }
}

fn render_event_modeling(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    event_modeling_render::render(&Context {
        diagram: &event_modeling::parse(text)?,
        section: &typed_config.eventmodeling,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The agentflow diagram, written `agentflow-beta`.
#[must_use]
pub fn agentflow() -> Plugin {
    Plugin {
        kind: DiagramType::Agentflow,
        render: render_agentflow,
    }
}

fn render_agentflow(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    agentflow_render::render(&Context {
        diagram: &agentflow::parse(text)?,
        section: &AgentflowSection {
            agentflow: &typed_config.agentflow,
            unified: section,
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The cynefin diagram, written `cynefin-beta`.
#[must_use]
pub fn cynefin() -> Plugin {
    Plugin {
        kind: DiagramType::Cynefin,
        render: render_cynefin,
    }
}

fn render_cynefin(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    cynefin_render::render(&Context {
        diagram: &cynefin::parse(text)?,
        section: &typed_config.cynefin,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The radar, written `radar-beta`.
#[must_use]
pub fn radar() -> Plugin {
    Plugin {
        kind: DiagramType::Radar,
        render: render_radar,
    }
}

fn render_radar(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    radar_render::render(&Context {
        diagram: &radar::parse(text)?,
        section: &typed_config.radar,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The railroad diagram, written `railroad-beta`, `railroad-ebnf-beta`, `railroad-abnf-beta` or `railroad-peg-beta`, in any letter case.
#[must_use]
pub fn railroad() -> Plugin {
    Plugin {
        kind: DiagramType::Railroad,
        render: render_railroad,
    }
}

fn render_railroad(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    railroad_render::render(&Context {
        diagram: &railroad::parse(text)?,
        section: &railroad_render::RailroadSection {
            config: &typed_config.railroad,
            detector: railroad::detector_id(text),
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The swimlane diagram, written `swimlane-beta`.
#[must_use]
pub fn swimlanes() -> Plugin {
    Plugin {
        kind: DiagramType::Swimlanes,
        render: render_swimlanes,
    }
}

fn render_swimlanes(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    swimlanes_render::render(&Context {
        diagram: &swimlanes::parse(text)?,
        section: &SwimlaneSection {
            swimlane: &typed_config.swimlane,
            unified: section,
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The tree view, written `treeView-beta`.
#[must_use]
pub fn tree_view() -> Plugin {
    Plugin {
        kind: DiagramType::TreeView,
        render: render_tree_view,
    }
}

fn render_tree_view(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    tree_view_render::render(&Context {
        diagram: &tree_view::parse(text)?,
        section: &typed_config.tree_view,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The use case diagram, written `usecase-beta`.
#[must_use]
pub fn use_case() -> Plugin {
    Plugin {
        kind: DiagramType::UseCase,
        render: render_use_case,
    }
}

fn render_use_case(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    let section = Section::of(&prepared.view.config);
    use_case_render::render(&Context {
        diagram: &use_case::parse(text)?,
        section: &UseCaseSection {
            usecase: &typed_config.usecase,
            unified: section,
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The venn diagram, written `venn-beta`.
#[must_use]
pub fn venn() -> Plugin {
    Plugin {
        kind: DiagramType::Venn,
        render: render_venn,
    }
}

fn render_venn(
    text: &str,
    prepared: &Prepared<'_>,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    venn_render::render(&Context {
        diagram: &venn::parse(text)?,
        section: &venn_render::VennSection {
            config: &typed_config.venn,
            seed: options.random_seed.unwrap_or(venn_render::DEFAULT_SEED),
        },
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}

/// The wardley map, written `wardley-beta`, in any letter case.
#[must_use]
pub fn wardley() -> Plugin {
    Plugin {
        kind: DiagramType::Wardley,
        render: render_wardley,
    }
}

fn render_wardley(
    text: &str,
    prepared: &Prepared<'_>,
    _options: &RenderOptions,
) -> Result<String, RenderError> {
    let typed_config = &prepared.view.config;
    let id = prepared.id.as_str();
    wardley_render::render(&Context {
        diagram: &wardley::parse(text)?,
        section: &typed_config.wardley_beta,
        globals: &prepared.view.globals,
        theme: prepared.theme(),
        id,
    })
}
