//! The railroad diagram (P3): four syntaxes over one model and one DB. `railroad-beta` (IR),
//! `railroad-ebnf-beta`, `railroad-abnf-beta` and `railroad-peg-beta` each have a hand-written
//! grammar and the shared scanner base, and the header picks the grammar, as the reference's
//! detectors do. The plan, the token tables and the quirks are in `docs-port/railroad/PLAN.md`.
//!
//! The text goes through the steps of `Diagram.fromText` before the scanner sees it: front matter,
//! directives, comment lines, entity encoding, and a line break. The front matter title does not
//! reach the model, since the reference's parser clears the DB after the front matter sets it.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::railroad::RailroadDiagram;

use crate::error::{ParseError, Source};
use crate::preprocess::{
    cleanup_comments, encode_entities, prepare, remove_directives, strip_front_matter,
};

use scanner::Syntax;

/// Parses a railroad diagram of any of the four syntaxes and returns its model.
///
/// The error position is the first token the grammar rejects, counted from the first line of the
/// original input.
pub fn parse(raw: &str) -> Result<RailroadDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let body = strip_front_matter(&cleaned);
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let Some(syntax) = syntax_of(crate::skip_hidden_prefix(&body)) else {
        return Err(source.error(0, "expected a railroad header: railroad-beta, railroad-ebnf-beta, railroad-abnf-beta or railroad-peg-beta"));
    };
    let tokens = scanner::scan(syntax, &text, source)?;
    let document = grammar::document(syntax, &tokens, source)?;
    Ok(db::build(document))
}

/// The id of the detector the text's header selects (`railroad`, `railroadEbnf`, `railroadAbnf` or
/// `railroadPeg`). The reference names the SVG of a railroad diagram with it (`aria-roledescription`),
/// so the renderer needs it next to the model. The text is prepared as [`parse`] prepares it; a text
/// that does not prepare, or has no header, gives the IR id, which is the one the detector of
/// `railroad-beta` names for any text it accepts.
pub fn detector_id(raw: &str) -> &'static str {
    let body = prepare(raw).map_or_else(
        |_| String::new(),
        |prepared| strip_front_matter(&cleanup_comments(&remove_directives(&prepared.text))),
    );
    syntax_of(crate::skip_hidden_prefix(&body)).map_or("railroad", Syntax::detector_id)
}

/// The syntax a text starts with, by the reference's detectors: the header keyword in any letter
/// case. The header itself is checked case-sensitively by the grammar.
fn syntax_of(text: &str) -> Option<Syntax> {
    [Syntax::Ir, Syntax::Ebnf, Syntax::Abnf, Syntax::Peg]
        .into_iter()
        .find(|syntax| crate::starts_with_ignore_ascii_case(text, syntax.header()))
}
