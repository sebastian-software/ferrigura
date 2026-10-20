//! The agentflow diagram (`agentflow-beta`): `agentflow.jison` and the agentflow DB, ported to Rust.
//! The plan, the grammar inventory, the quirks and the decisions are in
//! `docs-port/agentflow/PLAN.md`; the research is in `docs-port/agentflow/NOTES.md`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter,
//! directives and the entities, with the `%%` comments kept (the reference keeps them for its
//! source positions). The lexer is the flowchart's scanner with agentflow's `DIALECT`; the
//! grammar and the DB are agentflow's own.

mod db;
mod grammar;
mod metadata;
pub mod semantic;
mod shapes;

use ferrigura_model::agentflow::AgentflowDiagram;

use crate::error::ParseError;
use crate::flowchart::scanner::{Dialect, Keyword, Kind, Links, Trailing};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::sanitize::sanitize_text;
use crate::text::is_js_space;

/// The default `maxEdges` (500), which the configuration of the reference gives.
pub const DEFAULT_MAX_EDGES: usize = 500;

/// The `maxTextSize` of the configuration, in UTF-16 units: a text longer than this is parsed
/// without its comments (`Diagram.fromText`).
const MAX_TEXT_SIZE: usize = 50_000;

/// The lexer's values for agentflow (`agentflow.jison`): the header word, the keyword literals in
/// the order of the lexer's rules, the solid and dotted link operators without text, the `%%`
/// comments, and a blank as one token.
pub(crate) const DIALECT: Dialect = Dialect {
    graph_words: &["agentflow-beta"],
    keywords: &[
        keyword("style", Kind::Style, Trailing::Nothing),
        keyword("default", Kind::Default, Trailing::Nothing),
        keyword("linkStyle", Kind::LinkStyle, Trailing::Nothing),
        keyword("interpolate", Kind::Interpolate, Trailing::Nothing),
        keyword("classDef", Kind::ClassDef, Trailing::Nothing),
        keyword("class", Kind::Class, Trailing::Nothing),
        keyword("flow", Kind::Flow, Trailing::Nothing),
        keyword("connector", Kind::Connector, Trailing::Nothing),
        keyword("global", Kind::Global, Trailing::Blanks),
        keyword("end", Kind::End, Trailing::Blanks),
        keyword("_self", Kind::LinkTarget, Trailing::Nothing),
        keyword("_blank", Kind::LinkTarget, Trailing::Nothing),
        keyword("_parent", Kind::LinkTarget, Trailing::Nothing),
        keyword("_top", Kind::LinkTarget, Trailing::Nothing),
    ],
    links: Links::Agentflow,
    comments: true,
    space_run: false,
};

/// The wrapper's `src.replace(/}[^\S\n]*\n/g, '}\n')`: the blanks between a closing brace and the
/// line break after it go, for every brace in the text (`agentflowParser.ts`).
fn strip_brace_blanks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('}') {
        out.push_str(&rest[..=index]);
        let after = &rest[index + 1..];
        let blanks = after.len()
            - after
                .trim_start_matches(|c: char| c != '\n' && is_js_space(c))
                .len();
        if after[blanks..].starts_with('\n') {
            out.push('\n');
            rest = &after[blanks + 1..];
        } else {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// A keyword literal of [`DIALECT`].
const fn keyword(word: &'static str, kind: Kind, trailing: Trailing) -> Keyword {
    Keyword {
        word,
        kind,
        trailing,
    }
}

/// Parses an agentflow diagram with the default `maxEdges` and returns its model.
///
/// # Errors
///
/// The text is not an agentflow diagram, or the parse or the DB rejects it: a syntax error, an
/// unknown shape, an invalid YAML metadata block, or too many edges. The message is ferrigura's own.
pub fn parse(text: &str) -> Result<AgentflowDiagram, ParseError> {
    parse_with_max_edges(text, DEFAULT_MAX_EDGES)
}

/// Parses an agentflow diagram with the given `maxEdges`, which the reference reads from its
/// configuration; the default is [`DEFAULT_MAX_EDGES`].
///
/// # Errors
///
/// As [`parse`].
pub fn parse_with_max_edges(raw: &str, max_edges: usize) -> Result<AgentflowDiagram, ParseError> {
    let prepared = prepare(raw)?;
    // `Diagram.fromText` gives the parser the text with its comments, unless that is longer than
    // `maxTextSize`; the text without them is then the one parsed.
    let with_comments = remove_directives(&prepared.text);
    let source = if with_comments.encode_utf16().count() > MAX_TEXT_SIZE {
        cleanup_comments(&with_comments)
    } else {
        with_comments
    };
    let text = strip_brace_blanks(&(encode_entities(&source) + "\n"));
    let title = prepared
        .title
        .as_deref()
        .map(sanitize_text)
        .unwrap_or_default();
    grammar::parse_text(&text, prepared.line_offset, title, max_edges)
}

/// Parses agentflow text with the grammar and the DB alone, and nothing else: no front matter, no
/// directives, no entities and no trailing line break. It exists for the ported parser specs that
/// call the reference's raw Jison parser, as the flowchart's `parse_grammar` does. [`parse`] is the
/// diagram text, as `Diagram.fromText` gives it to the parser.
///
/// # Errors
///
/// As [`parse`].
pub fn parse_grammar(text: &str) -> Result<AgentflowDiagram, ParseError> {
    grammar::parse_text(text, 0, String::new(), DEFAULT_MAX_EDGES)
}
