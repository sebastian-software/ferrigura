//! The flowchart parser (M3): `flow.jison` and the flowchart DB `flowDb.ts`, ported to Rust. The
//! plan, the grammar inventory and the model are in `docs-port/flowchart/PLAN.md`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter
//! (the shared preparation), directives, comment lines, and the line break after a closing brace.

mod db;
mod grammar;
mod scanner;
mod shapes;
mod unicode;

use std::error::Error;
use std::fmt;

use ferrigura_model::flowchart::FlowchartDiagram;

use crate::error::ParseError;
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::is_js_space;

/// Options of the parser. The defaults are the reference's defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The most edges a diagram may have; more is an error (`maxEdges`, 500 by default).
    pub max_edges: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_edges: db::DEFAULT_MAX_EDGES,
        }
    }
}

/// Why a flowchart could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowchartError {
    /// The text is not a flowchart, or the DB rejects it (an unknown shape, too many edges, …).
    Syntax(ParseError),
}

impl fmt::Display for FlowchartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => write!(f, "{error}"),
        }
    }
}

impl Error for FlowchartError {}

/// Parses a flowchart with the default options.
pub fn parse(raw: &str) -> Result<FlowchartDiagram, FlowchartError> {
    parse_with(raw, Options::default())
}

/// Parses a flowchart with the given options.
pub fn parse_with(raw: &str, options: Options) -> Result<FlowchartDiagram, FlowchartError> {
    parse_with_warnings(raw, options).map(|(diagram, _)| diagram)
}

/// Parses flowchart text with the grammar and the DB alone, and nothing else. It exists for the
/// ported parser specs, which call the reference's parser directly. The text gets only the parser
/// wrapper's closing-brace rule: no front matter, no directive removal, no comment removal, no
/// entity encoding and no trailing line break. [`parse`] is the diagram text, as `Diagram.fromText`
/// gives it to the parser, and it is the entry point for everything else.
pub fn parse_grammar(text: &str) -> Result<FlowchartDiagram, FlowchartError> {
    grammar::parse_text(&close_braces(text), 0, String::new(), Options::default())
        .map(|(diagram, _)| diagram)
}

/// The comment removal of the reference (`cleanupComments`): the lines that start with `%%`
/// (not `%%{`), and the leading blanks of the text.
#[must_use]
pub fn clean_comments(text: &str) -> String {
    cleanup_comments(text)
}

/// Parses a flowchart and returns the warnings the reference would log while parsing, such as a
/// style given to a node that does not exist yet. The warnings are not part of the model.
/// style given to a node that does not exist yet. The warnings are not part of the model.
pub fn parse_with_warnings(
    raw: &str,
    options: Options,
) -> Result<(FlowchartDiagram, Vec<String>), FlowchartError> {
    let prepared = prepare(raw).map_err(FlowchartError::Syntax)?;
    // `Diagram.fromText` encodes the entities and adds a line break before the parser sees the
    // text; the parser's own wrapper then closes the braces.
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = close_braces(&(encode_entities(&cleaned) + "\n"));
    let title = prepared.title.unwrap_or_default();
    grammar::parse_text(&text, prepared.line_offset, title, options)
}

/// `}\s*\n` becomes `}\n`: the blanks after a closing brace are dropped up to the last line
/// break before the next text, as the parser wrapper of the reference does.
fn close_braces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('}') {
        out.push_str(&rest[..=index]);
        let after = &rest[index + 1..];
        let spaces = after.len() - after.trim_start_matches(is_js_space).len();
        match after[..spaces].rfind('\n') {
            Some(newline) => {
                out.push('\n');
                rest = &after[newline + 1..];
            }
            None => rest = after,
        }
    }
    out.push_str(rest);
    out
}
