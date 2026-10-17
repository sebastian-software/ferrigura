//! Shared lexer base and hand-written parsers of the ferrigura diagram types.
//!
//! Each parser reads the diagram text, cleans it as the TypeScript reference does before parsing,
//! and returns the diagram model of `ferrigura-model`. See `docs-port/decisions/0002` for why the
//! parsers are written by hand on one lexer base.
//!
//! The crate covers the pie, info, flowchart, timeline, journey, gitGraph, ER, class, sequence,
//! state and gantt diagrams, and detects the P2 types whose parsers are not ported yet.

#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod common;
mod cursor;
mod entities;
mod error;
pub mod flowchart;
pub mod gantt;
pub mod git_graph;
mod lexer;
mod preprocess;
mod scan;
mod text;
pub mod url;

pub mod architecture;
pub mod block;
pub mod c4;
pub mod class;
pub mod config;
pub mod er;
pub mod event_modeling;
pub mod info;
pub mod ishikawa;
pub mod journey;
pub mod kanban;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant_chart;
pub mod requirement;
pub mod sanitize;
pub mod sankey;
pub mod sequence;
pub mod state;
pub mod timeline;
pub mod treemap;
pub mod xychart;
pub mod yaml;

use ferrigura_model::architecture::ArchitectureDiagram;
use ferrigura_model::block::BlockDiagram;
use ferrigura_model::c4::C4Diagram;
use ferrigura_model::class::ClassDiagram;
use ferrigura_model::er::ErDiagram;
use ferrigura_model::event_modeling::EventModelingDiagram;
use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_model::gantt::GanttDiagram;
use ferrigura_model::git_graph::GitGraphDiagram;
use ferrigura_model::info::InfoDiagram;
use ferrigura_model::ishikawa::IshikawaDiagram;
use ferrigura_model::journey::JourneyDiagram;
use ferrigura_model::kanban::KanbanDiagram;
use ferrigura_model::mindmap::MindmapDiagram;
use ferrigura_model::packet::PacketDiagram;
use ferrigura_model::pie::PieDiagram;
use ferrigura_model::quadrant_chart::QuadrantChartDiagram;
use ferrigura_model::requirement::RequirementDiagram;
use ferrigura_model::sankey::SankeyDiagram;
use ferrigura_model::sequence::SequenceDiagram;
use ferrigura_model::state::StateDiagram;
use ferrigura_model::timeline::TimelineDiagram;
use ferrigura_model::treemap::TreemapDiagram;
use ferrigura_model::xychart::XyChartDiagram;

pub use error::ParseError;
pub use preprocess::decode_entities;

/// A diagram type this crate can parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    /// The pie diagram.
    Pie,
    /// The info diagram.
    Info,
    /// The flowchart, written `flowchart` or `graph`.
    Flowchart,
    /// The timeline.
    Timeline,
    /// The user journey, written `journey`.
    Journey,
    /// The gitGraph diagram.
    GitGraph,
    /// The ER diagram, written `erDiagram`.
    Er,
    /// The class diagram, written `classDiagram` or `classDiagram-v2`.
    Class,
    /// The sequence diagram, written `sequenceDiagram`.
    Sequence,
    /// The state diagram, written `stateDiagram` or `stateDiagram-v2`.
    State,
    /// The gantt diagram.
    Gantt,
    /// The mindmap, written `mindmap`.
    Mindmap,
    /// The block diagram, written `block`.
    Block,
    /// The packet diagram, written `packet`.
    Packet,
    /// The xychart, written `xychart`.
    XyChart,
    /// The sankey diagram, written `sankey`.
    Sankey,
    /// The quadrant chart, written `quadrantChart`.
    QuadrantChart,
    /// The requirement diagram, written `requirementDiagram`.
    Requirement,
    /// The C4 diagram, written with one of its five headers: `C4Context`, `C4Container`,
    /// `C4Component`, `C4Dynamic` or `C4Deployment`.
    C4,
    /// The kanban board, written `kanban`.
    Kanban,
    /// The architecture diagram, written `architecture` or `architecture-beta`.
    Architecture,
    /// The treemap, written `treemap` or `treemap-beta`.
    Treemap,
    /// The ishikawa diagram, written `ishikawa` or `ishikawa-beta`, in any letter case.
    Ishikawa,
    /// The event modeling diagram, written `eventmodeling`.
    EventModeling,
}

impl DiagramType {
    /// The configuration section of the diagram type, as `getDiagramConfigKey` names it.
    #[must_use]
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Pie => "pie",
            Self::Info => "info",
            Self::Flowchart => "flowchart",
            Self::Timeline => "timeline",
            Self::Journey => "journey",
            Self::GitGraph => "gitGraph",
            Self::Er => "er",
            Self::Class => "class",
            Self::Sequence => "sequence",
            Self::State => "state",
            Self::Gantt => "gantt",
            Self::Mindmap => "mindmap",
            Self::Block => "block",
            Self::Packet => "packet",
            Self::XyChart => "xyChart",
            Self::Sankey => "sankey",
            Self::QuadrantChart => "quadrantChart",
            Self::Requirement => "requirement",
            Self::C4 => "c4",
            Self::Kanban => "kanban",
            Self::Architecture => "architecture",
            Self::Treemap => "treemap",
            Self::Ishikawa => "ishikawa",
            Self::EventModeling => "eventmodeling",
        }
    }
}

/// A parsed diagram of any type this crate covers.
#[derive(Debug, Clone, PartialEq)]
pub enum Diagram {
    /// A pie diagram.
    Pie(PieDiagram),
    /// An info diagram.
    Info(InfoDiagram),
    /// A flowchart.
    Flowchart(FlowchartDiagram),
    /// A timeline.
    Timeline(TimelineDiagram),
    /// A user journey.
    Journey(JourneyDiagram),
    /// A gitGraph.
    GitGraph(GitGraphDiagram),
    /// An ER diagram.
    Er(ErDiagram),
    /// A class diagram.
    Class(ClassDiagram),
    /// A sequence diagram.
    Sequence(SequenceDiagram),
    /// A state diagram.
    State(StateDiagram),
    /// A gantt diagram.
    Gantt(GanttDiagram),
    /// A mindmap.
    Mindmap(MindmapDiagram),
    /// A block diagram.
    Block(BlockDiagram),
    /// A packet diagram.
    Packet(PacketDiagram),
    /// An xychart.
    XyChart(XyChartDiagram),
    /// A sankey diagram.
    Sankey(SankeyDiagram),
    /// A quadrant chart.
    QuadrantChart(QuadrantChartDiagram),
    /// A requirement diagram.
    Requirement(RequirementDiagram),
    /// A C4 diagram.
    C4(C4Diagram),
    /// A kanban board.
    Kanban(KanbanDiagram),
    /// An architecture diagram.
    Architecture(ArchitectureDiagram),
    /// A treemap.
    Treemap(TreemapDiagram),
    /// An ishikawa diagram.
    Ishikawa(IshikawaDiagram),
    /// An event modeling diagram.
    EventModeling(EventModelingDiagram),
}

/// Decides the diagram type from the start of the text, as the detectors of the reference do.
///
/// Front matter, directives and comment lines before the keyword are skipped, so `^\s*pie` and
/// `^\s*info` apply to the first line that is not one of them. The detectors are tried in the order
/// the reference registers them (`diagram-orchestration.ts`), and the first one that matches decides.
/// No two types of this crate match the same text, so the order decides nothing between them.
#[must_use]
pub fn detect(text: &str) -> Option<DiagramType> {
    let prepared = preprocess::prepare(text).ok()?;
    let rest = skip_hidden_prefix(&prepared.text);
    if rest.starts_with("mindmap") {
        Some(DiagramType::Mindmap)
    } else if rest.starts_with("architecture") {
        Some(DiagramType::Architecture)
    } else if is_c4_header(rest) {
        Some(DiagramType::C4)
    } else if rest.starts_with("kanban") {
        Some(DiagramType::Kanban)
    } else if rest.starts_with("classDiagram") {
        Some(DiagramType::Class)
    } else if rest.starts_with("erDiagram") {
        Some(DiagramType::Er)
    } else if rest.starts_with("gantt") {
        Some(DiagramType::Gantt)
    } else if rest.starts_with("info") {
        Some(DiagramType::Info)
    } else if rest.starts_with("pie") {
        Some(DiagramType::Pie)
    } else if rest.starts_with("requirementDiagram") {
        Some(DiagramType::Requirement)
    } else if rest.starts_with("sequenceDiagram") {
        Some(DiagramType::Sequence)
    } else if rest.starts_with("graph") || rest.starts_with("flowchart") {
        Some(DiagramType::Flowchart)
    } else if rest.starts_with("timeline") {
        Some(DiagramType::Timeline)
    } else if rest.starts_with("gitGraph") {
        Some(DiagramType::GitGraph)
    } else if rest.starts_with("stateDiagram") {
        Some(DiagramType::State)
    } else if rest.starts_with("journey") {
        Some(DiagramType::Journey)
    } else if rest.starts_with("quadrantChart") {
        Some(DiagramType::QuadrantChart)
    } else if has_keyword_alone(rest, "sankey") {
        Some(DiagramType::Sankey)
    } else if has_keyword_alone(rest, "packet") {
        Some(DiagramType::Packet)
    } else if has_keyword_alone(rest, "xychart") {
        Some(DiagramType::XyChart)
    } else if has_keyword_alone(rest, "block") {
        Some(DiagramType::Block)
    } else if rest.starts_with("eventmodeling") {
        Some(DiagramType::EventModeling)
    } else if is_ishikawa_header(rest) {
        Some(DiagramType::Ishikawa)
    } else if rest.starts_with("treemap") {
        Some(DiagramType::Treemap)
    } else {
        None
    }
}

/// `^\s*<keyword>(?![\w-])`, the reference's header test for the stable types `sankey`, `packet`,
/// `xychart` and `block`: the keyword is not followed by a word character or a hyphen, so the
/// `-beta` keyword is not taken.
fn has_keyword_alone(text: &str, keyword: &str) -> bool {
    text.strip_prefix(keyword).is_some_and(|after| {
        !matches!(after.as_bytes().first(), Some(&byte) if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    })
}

/// `^\s*(?:C4Context|C4Container|C4Component|C4Dynamic|C4Deployment)`, the detector of `c4`.
fn is_c4_header(text: &str) -> bool {
    [
        "C4Context",
        "C4Container",
        "C4Component",
        "C4Dynamic",
        "C4Deployment",
    ]
    .iter()
    .any(|header| text.starts_with(header))
}

/// `^\s*ishikawa(-beta)?\b` with the `i` flag, the detector of `ishikawa`. The optional `-beta` does
/// not change the outcome: the match ends at a word boundary after `ishikawa`, and a hyphen or the end
/// of the text always gives one. The `i` flag folds ASCII letters only, as it does without `u`.
fn is_ishikawa_header(text: &str) -> bool {
    text.get(..8)
        .is_some_and(|keyword| keyword.eq_ignore_ascii_case("ishikawa"))
        && !matches!(text.as_bytes().get(8), Some(&byte) if byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Parses a diagram of any type this crate covers, detecting its type first.
pub fn parse(text: &str) -> Result<Diagram, ParseError> {
    match detect(text) {
        Some(DiagramType::Er) => er::parse(text).map(Diagram::Er),
        Some(DiagramType::Pie) => pie::parse(text).map(Diagram::Pie),
        Some(DiagramType::Info) => info::parse(text).map(Diagram::Info),
        Some(DiagramType::Flowchart) => flowchart::parse(text)
            .map(Diagram::Flowchart)
            .map_err(|flowchart::FlowchartError::Syntax(error)| error),
        Some(DiagramType::Timeline) => timeline::parse(text).map(Diagram::Timeline),
        Some(DiagramType::Journey) => journey::parse(text).map(Diagram::Journey),
        Some(DiagramType::GitGraph) => git_graph::parse(text).map(Diagram::GitGraph),
        Some(DiagramType::Class) => class::parse(text).map(Diagram::Class),
        Some(DiagramType::Sequence) => sequence::parse(text).map(Diagram::Sequence),
        Some(DiagramType::State) => state::parse(text).map(Diagram::State),
        Some(DiagramType::Gantt) => gantt::parse(text).map(Diagram::Gantt),
        Some(DiagramType::Mindmap) => mindmap::parse(text).map(Diagram::Mindmap),
        Some(DiagramType::Block) => block::parse(text).map(Diagram::Block),
        Some(DiagramType::Packet) => packet::parse(text).map(Diagram::Packet),
        Some(DiagramType::XyChart) => xychart::parse(text).map(Diagram::XyChart),
        Some(DiagramType::Sankey) => sankey::parse(text).map(Diagram::Sankey),
        Some(DiagramType::QuadrantChart) => quadrant_chart::parse(text).map(Diagram::QuadrantChart),
        Some(DiagramType::Requirement) => requirement::parse(text).map(Diagram::Requirement),
        Some(DiagramType::C4) => c4::parse(text).map(Diagram::C4),
        Some(DiagramType::Kanban) => kanban::parse(text).map(Diagram::Kanban),
        Some(DiagramType::Architecture) => architecture::parse(text).map(Diagram::Architecture),
        Some(DiagramType::Treemap) => treemap::parse(text).map(Diagram::Treemap),
        Some(DiagramType::Ishikawa) => ishikawa::parse(text).map(Diagram::Ishikawa),
        Some(DiagramType::EventModeling) => event_modeling::parse(text).map(Diagram::EventModeling),
        None => Err(ParseError {
            line: 1,
            column: 1,
            message: "no diagram type detected".to_owned(),
        }),
    }
}

/// Skips whitespace, directive lines and comment lines at the start of the text.
fn skip_hidden_prefix(mut text: &str) -> &str {
    loop {
        text = text.trim_start_matches(text::is_js_space);
        if text.starts_with("%%{") {
            if let Some(len) = preprocess::loose_directive_len(text) {
                text = &text[len..];
                continue;
            }
        } else if let Some(after) = text.strip_prefix("%%") {
            // A comment line is `%%` followed by a character that is neither `{` nor a line break.
            let is_comment = matches!(after.chars().next(), Some(c) if c != '{' && c != '\n');
            if is_comment {
                text = after.find('\n').map_or("", |newline| &after[newline + 1..]);
                continue;
            }
        }
        return text;
    }
}
