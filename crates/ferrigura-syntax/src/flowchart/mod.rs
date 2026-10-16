//! The flowchart parser (M3): `flow.jison` and the flowchart DB `flowDb.ts`, ported to Rust. The
//! plan, the grammar inventory and the model are in `docs-port/flowchart/PLAN.md`.
//!
//! The text goes through the same steps as the reference before the grammar sees it: front matter
//! (the shared preparation), directives, comment lines, and the line break after a closing brace.

mod db;
mod grammar;
mod scanner;
mod shapes;
pub(crate) mod unicode;

use std::error::Error;
use std::fmt;

use ferrigura_model::flowchart::FlowchartDiagram;

use crate::error::ParseError;
use crate::preprocess::{loose_directive_len, prepare};
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

/// Removes the `%%{ … }%%` directives, as `removeDirectives` does.
pub(crate) fn remove_directives(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("%%{") {
        out.push_str(&rest[..start]);
        match loose_directive_len(&rest[start..]) {
            Some(len) => rest = &rest[start + len..],
            None => {
                out.push_str("%%{");
                rest = &rest[start + 3..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Removes the comment lines, as `cleanupComments` does: `^\s*%%(?!{)[^\n]+\n?` with the
/// multi-line flag, then the leading whitespace of the text.
fn cleanup_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut line_start = 0;
    while line_start <= text.len() {
        if let Some(end) = comment_match_end(text, line_start) {
            out.push_str(&text[copied..line_start]);
            copied = end;
            line_start = end;
            continue;
        }
        match text[line_start..].find('\n') {
            Some(newline) => line_start += newline + 1,
            None => break,
        }
    }
    out.push_str(&text[copied..]);
    out.trim_start_matches(is_js_space).to_owned()
}

/// The end of a comment match that starts at the line start `start`, if there is one.
fn comment_match_end(text: &str, start: usize) -> Option<usize> {
    let rest = &text[start..];
    let spaces = rest.len() - rest.trim_start_matches(is_js_space).len();
    let after = &rest[spaces..];
    let body = after.strip_prefix("%%")?;
    if body.starts_with('{') || body.is_empty() || body.starts_with('\n') {
        return None;
    }
    let line = body.find('\n').unwrap_or(body.len());
    let mut end = start + spaces + 2 + line;
    if text[end..].starts_with('\n') {
        end += 1;
    }
    Some(end)
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

/// `encodeEntities`: the semicolon that ends a `style` or `classDef` line with a colour is dropped,
/// and `#name;` or `#123;` become placeholders that the renderer decodes.
fn encode_entities(text: &str) -> String {
    let text = drop_colour_semicolons(text, "style");
    let text = drop_colour_semicolons(&text, "classDef");
    encode_hash_entities(&text)
}

/// `<word>.*:\S*#.*;` with the semicolon removed: the last semicolon of the line, when a colon
/// followed by non-blank text and a `#` come after the word on that line.
fn drop_colour_semicolons(text: &str, word: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(found) = text[pos..].find(word) {
        let start = pos + found;
        let after_word = start + word.len();
        let line_end = text[after_word..]
            .find('\n')
            .map_or(text.len(), |newline| after_word + newline);
        let last_semicolon = text[after_word..line_end]
            .rfind(';')
            .map(|i| after_word + i);
        match last_semicolon {
            Some(semicolon) if has_colon_hash(&text[after_word..semicolon]) => {
                out.push_str(&text[pos..semicolon]);
                pos = semicolon + 1;
            }
            _ => {
                out.push_str(&text[pos..after_word]);
                pos = after_word;
            }
        }
    }
    out.push_str(&text[pos..]);
    out
}

/// Whether `segment` has a colon followed by blank-free text and a `#`: `.*:\S*#.*`.
fn has_colon_hash(segment: &str) -> bool {
    segment.char_indices().any(|(colon, c)| {
        c == ':'
            && segment[colon + 1..]
                .char_indices()
                .take_while(|&(_, c)| !is_js_space(c))
                .any(|(_, c)| c == '#')
    })
}

/// `#\w+;` becomes `ﬂ°°<digits>¶ß` for a number and `ﬂ°<name>¶ß` for a name.
fn encode_hash_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(hash) = rest.find('#') {
        out.push_str(&rest[..hash]);
        let after = &rest[hash + 1..];
        let name_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        if name_len > 0 && after[name_len..].starts_with(';') {
            let name = &after[..name_len];
            if name.bytes().all(|b| b.is_ascii_digit()) {
                out.push_str("ﬂ°°");
            } else {
                out.push_str("ﬂ°");
            }
            out.push_str(name);
            out.push_str("¶ß");
            rest = &after[name_len + 1..];
        } else {
            out.push('#');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
