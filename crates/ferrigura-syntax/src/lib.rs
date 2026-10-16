//! Shared lexer base and hand-written parsers of the ferrigura diagram types.
//!
//! Each parser reads the diagram text, cleans it as the TypeScript reference does before parsing,
//! and returns the diagram model of `ferrigura-model`. See `docs-port/decisions/0002` for why the
//! parsers are written by hand on one lexer base.
//!
//! The crate covers the pie, info and flowchart diagrams for now; the other types follow in later
//! milestones.

#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod common;
mod cursor;
mod error;
pub mod flowchart;
pub mod git_graph;
mod lexer;
mod preprocess;
mod scan;
mod text;

pub mod config;
pub mod info;
pub mod pie;
pub mod sanitize;
pub mod yaml;

use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_model::git_graph::GitGraphDiagram;
use ferrigura_model::info::InfoDiagram;
use ferrigura_model::pie::PieDiagram;

pub use error::ParseError;

/// A diagram type this crate can parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    /// The pie diagram.
    Pie,
    /// The info diagram.
    Info,
    /// The flowchart, written `flowchart` or `graph`.
    Flowchart,
    /// The gitGraph diagram.
    GitGraph,
}

impl DiagramType {
    /// The configuration section of the diagram type, as `getDiagramConfigKey` names it.
    #[must_use]
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Pie => "pie",
            Self::Info => "info",
            Self::Flowchart => "flowchart",
            Self::GitGraph => "gitGraph",
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
    /// A gitGraph.
    GitGraph(GitGraphDiagram),
}

/// Decides the diagram type from the start of the text, as the detectors of the reference do.
///
/// Front matter, directives and comment lines before the keyword are skipped, so `^\s*pie` and
/// `^\s*info` apply to the first line that is not one of them.
#[must_use]
pub fn detect(text: &str) -> Option<DiagramType> {
    let prepared = preprocess::prepare(text).ok()?;
    let rest = skip_hidden_prefix(&prepared.text);
    if rest.starts_with("pie") {
        Some(DiagramType::Pie)
    } else if rest.starts_with("info") {
        Some(DiagramType::Info)
    } else if rest.starts_with("graph") || rest.starts_with("flowchart") {
        Some(DiagramType::Flowchart)
    } else if rest.starts_with("gitGraph") {
        Some(DiagramType::GitGraph)
    } else {
        None
    }
}

/// Parses a diagram of any type this crate covers, detecting its type first.
pub fn parse(text: &str) -> Result<Diagram, ParseError> {
    match detect(text) {
        Some(DiagramType::Pie) => pie::parse(text).map(Diagram::Pie),
        Some(DiagramType::Info) => info::parse(text).map(Diagram::Info),
        Some(DiagramType::Flowchart) => flowchart::parse(text)
            .map(Diagram::Flowchart)
            .map_err(|flowchart::FlowchartError::Syntax(error)| error),
        Some(DiagramType::GitGraph) => git_graph::parse(text).map(Diagram::GitGraph),
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
