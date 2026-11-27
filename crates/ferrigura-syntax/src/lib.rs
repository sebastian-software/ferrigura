//! Shared lexer base and hand-written parsers of the ferrigura diagram types.
//!
//! Each parser reads the diagram text, cleans it as the TypeScript reference does before parsing,
//! and returns the diagram model of `ferrigura-model`. See `docs/decisions/0007-hand-written-parsers.md` for why the
//! parsers are written by hand on one lexer base.
//!
//! The crate parses the ten diagram types that ferrigura draws ([`DiagramType::ALL`],
//! `docs/diagrams/README.md`). It also knows the headers of Mermaid 12.2.0's other diagram types,
//! so that a text of one of them is told apart from a text without a header
//! ([`detect_unsupported`]).

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
mod lexer;
pub mod preprocess;
mod scan;
mod text;
mod unsupported;
pub mod url;

pub mod class;
pub mod config;
pub mod er;
pub mod mindmap;
pub mod pie;
pub mod quadrant_chart;
pub mod sanitize;
pub mod sequence;
pub mod state;
pub mod xychart;
pub mod yaml;

use ferrigura_model::class::ClassDiagram;
use ferrigura_model::er::ErDiagram;
use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_model::gantt::GanttDiagram;
use ferrigura_model::mindmap::MindmapDiagram;
use ferrigura_model::pie::PieDiagram;
use ferrigura_model::quadrant_chart::QuadrantChartDiagram;
use ferrigura_model::sequence::SequenceDiagram;
use ferrigura_model::state::StateDiagram;
use ferrigura_model::xychart::XyChartDiagram;

pub use error::ParseError;
pub use preprocess::decode_entities;

/// A diagram type that ferrigura draws, which this crate parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramType {
    /// The pie diagram.
    Pie,
    /// The flowchart, written `flowchart` or `graph`.
    Flowchart,
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
    /// The xychart, written `xychart`.
    XyChart,
    /// The quadrant chart, written `quadrantChart`.
    QuadrantChart,
}

impl DiagramType {
    /// The diagram types ferrigura draws, in three groups: the graphs (flowchart, state, class, ER),
    /// the single types (mindmap, sequence, gantt) and the charts (XY chart, pie, quadrant chart).
    /// The facade registers one plugin for each, and a text of any other type is refused with an
    /// error that lists these (`docs/decisions/0024-a-fixed-list-of-diagram-types.md`).
    pub const ALL: [Self; 10] = [
        Self::Flowchart,
        Self::State,
        Self::Class,
        Self::Er,
        Self::Mindmap,
        Self::Sequence,
        Self::Gantt,
        Self::XyChart,
        Self::Pie,
        Self::QuadrantChart,
    ];

    /// The name of the diagram type in messages, such as `flowchart` or `XY chart`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Pie => "pie",
            Self::Flowchart => "flowchart",
            Self::Er => "ER",
            Self::Class => "class",
            Self::Sequence => "sequence",
            Self::State => "state",
            Self::Gantt => "gantt",
            Self::Mindmap => "mindmap",
            Self::XyChart => "XY chart",
            Self::QuadrantChart => "quadrant chart",
        }
    }

    /// The configuration section of the diagram type, as `getDiagramConfigKey` names it.
    #[must_use]
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Pie => "pie",
            Self::Flowchart => "flowchart",
            Self::Er => "er",
            Self::Class => "class",
            Self::Sequence => "sequence",
            Self::State => "state",
            Self::Gantt => "gantt",
            Self::Mindmap => "mindmap",
            Self::XyChart => "xyChart",
            Self::QuadrantChart => "quadrantChart",
        }
    }
}

/// A parsed diagram of any type this crate covers.
#[derive(Debug, Clone, PartialEq)]
pub enum Diagram {
    /// A pie diagram.
    Pie(PieDiagram),
    /// A flowchart.
    Flowchart(FlowchartDiagram),
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
    /// An xychart.
    XyChart(XyChartDiagram),
    /// A quadrant chart.
    QuadrantChart(QuadrantChartDiagram),
}

/// Decides the diagram type from the start of the text, as the detectors of the reference do.
///
/// Front matter, directives and comment lines before the keyword are skipped, so `^\s*pie` and
/// `^\s*mindmap` apply to the first line that is not one of them. The detectors are tried in the order
/// the reference registers them (`diagram-orchestration.ts`), and the first one that matches decides.
/// No two types of this crate match the same text, so the order decides nothing between them.
#[must_use]
pub fn detect(text: &str) -> Option<DiagramType> {
    let prepared = preprocess::prepare(text).ok()?;
    detect_prepared(&prepared)
}

/// Detects a diagram from an already prepared source.
#[must_use]
pub fn detect_prepared(prepared: &preprocess::Prepared) -> Option<DiagramType> {
    with_header(prepared, detect_header)
}

/// Calls `read` with the text from the diagram's header on: the prepared text without its
/// directives, comment lines and front matter, from its first line that is not one of them.
fn with_header<T>(prepared: &preprocess::Prepared, read: impl FnOnce(&str) -> T) -> T {
    // Fast path: when the first non-whitespace line has no `%%` (directive/comment) and does not
    // start with `---` (front matter), removing directives/comments/front matter cannot change the
    // leading header line that every detector inspects.
    let trimmed = prepared.text.trim_start_matches(text::is_js_space);
    let first_line_end = trimmed.find(text::is_line_term).unwrap_or(trimmed.len());
    let first_line = &trimmed[..first_line_end];
    if !first_line.contains("%%") && !first_line.starts_with("---") {
        return read(trimmed);
    }
    // `preprocessDiagram` removes the directives and the comment lines and trims the start
    // (`preprocess.ts` 58-65); `detectType` then strips a front matter block once more
    // (`detectType.ts` 15-18), which finds one that blank lines or comments kept from the start.
    let without_directives = preprocess::remove_directives(&prepared.text);
    let cleaned = preprocess::cleanup_comments(&without_directives);
    let body = preprocess::strip_front_matter(&cleaned);
    read(skip_hidden_prefix(&body))
}

/// The diagram type whose detector takes the text from the header on.
fn detect_header(rest: &str) -> Option<DiagramType> {
    if rest.starts_with("mindmap") {
        Some(DiagramType::Mindmap)
    } else if rest.starts_with("classDiagram") {
        Some(DiagramType::Class)
    } else if rest.starts_with("erDiagram") {
        Some(DiagramType::Er)
    } else if rest.starts_with("gantt") {
        Some(DiagramType::Gantt)
    } else if rest.starts_with("pie") {
        Some(DiagramType::Pie)
    } else if rest.starts_with("sequenceDiagram") {
        Some(DiagramType::Sequence)
    } else if rest.starts_with("graph") || rest.starts_with("flowchart") {
        Some(DiagramType::Flowchart)
    } else if rest.starts_with("stateDiagram") {
        Some(DiagramType::State)
    } else if rest.starts_with("quadrantChart") {
        Some(DiagramType::QuadrantChart)
    } else if has_keyword_alone(rest, "xychart") {
        Some(DiagramType::XyChart)
    } else {
        None
    }
}

/// `^\s*<keyword>(-beta)?(?![\w-])`, the header test for `sankey`, `packet`, `xychart` and `block`:
/// the keyword or its `-beta` form, as upstream Mermaid takes both, not followed by a word character
/// or a hyphen. Upstream's detectors end without that boundary (`^\s*sankey(-beta)?`), so they also
/// take `sankeyfoo`; ferrigura keeps the boundary, a fix the reference gets as a patch
/// (`docs/decisions/0014-upstream-reference-at-a-pin.md`).
pub(crate) fn has_keyword_alone(text: &str, keyword: &str) -> bool {
    let alone = |after: &str| !matches!(after.as_bytes().first(), Some(&byte) if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    text.strip_prefix(keyword)
        .is_some_and(|after| alone(after) || after.strip_prefix("-beta").is_some_and(alone))
}

/// The name of the Mermaid diagram type whose header starts `text`, when that is a type ferrigura
/// does not draw, such as `sankey` or `Git graph`: [`prepare_and_detect`] rejects such a text as it
/// rejects one without a header, and the facade names the type in its error.
#[must_use]
pub fn detect_unsupported(text: &str) -> Option<&'static str> {
    let prepared = preprocess::prepare(text).ok()?;
    with_header(&prepared, unsupported::name)
}

/// Preprocesses `text` and detects its diagram type, surfacing YAML
/// alias-expansion limit and cyclic-anchor errors from front matter directly instead of folding them
/// into an unrecognized diagram header.
pub fn prepare_and_detect(text: &str) -> Result<(preprocess::Prepared, DiagramType), ParseError> {
    let source = match preprocess::prepare(text) {
        Ok(source) => source,
        Err(error) if yaml::is_limit_or_cycle_error(&error.message) => return Err(error),
        Err(_) => {
            return Err(ParseError {
                line: 1,
                column: 1,
                message: "no diagram type detected".to_owned(),
            });
        }
    };
    let Some(kind) = detect_prepared(&source) else {
        return Err(ParseError {
            line: 1,
            column: 1,
            message: "no diagram type detected".to_owned(),
        });
    };
    Ok((source, kind))
}

/// Parses a diagram of any type this crate covers, detecting its type first.
pub fn parse(text: &str) -> Result<Diagram, ParseError> {
    let (_, kind) = prepare_and_detect(text)?;
    match kind {
        DiagramType::Er => er::parse(text).map(Diagram::Er),
        DiagramType::Pie => pie::parse(text).map(Diagram::Pie),
        DiagramType::Flowchart => flowchart::parse(text)
            .map(Diagram::Flowchart)
            .map_err(|flowchart::FlowchartError::Syntax(error)| error),
        DiagramType::Class => class::parse(text).map(Diagram::Class),
        DiagramType::Sequence => sequence::parse(text).map(Diagram::Sequence),
        DiagramType::State => state::parse(text).map(Diagram::State),
        DiagramType::Gantt => gantt::parse(text).map(Diagram::Gantt),
        DiagramType::Mindmap => mindmap::parse(text).map(Diagram::Mindmap),
        DiagramType::XyChart => xychart::parse(text).map(Diagram::XyChart),
        DiagramType::QuadrantChart => quadrant_chart::parse(text).map(Diagram::QuadrantChart),
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
