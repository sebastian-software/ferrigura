//! The gantt parser (M6): `gantt.jison`, the gantt DB `ganttDb.js` and the date rules they use.
//! The plan, the quirks and the canonical model are in `docs-port/gantt/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it: front matter, CRLF and
//! attribute quotes (the shared preparation), directives, comment lines, entities and the line
//! break at the end. The DB then compiles the dates, so a diagram with a date that cannot be read
//! is rejected here, as the reference rejects it in `getTasks`.

mod date;
mod db;
mod parser;
mod scanner;
mod url;

use std::time::{SystemTime, UNIX_EPOCH};

use ferrigura_model::gantt::GanttDiagram;

use crate::error::{ParseError, Source};
use crate::flowchart::{cleanup_comments, encode_entities, remove_directives};
use crate::preprocess::prepare;

/// Options of the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The clock, in milliseconds since the epoch. The reference reads `new Date()` where a date
    /// format leaves out a day, and for `after` and `until` without a known task; the port reads
    /// this value instead, so the same text and clock always give the same model.
    pub now: i64,
}

impl Default for Options {
    /// The system clock.
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
            });
        Self { now }
    }
}

/// Parses a gantt diagram with the system clock.
pub fn parse(raw: &str) -> Result<GanttDiagram, ParseError> {
    parse_with(raw, Options::default())
}

/// Parses a gantt diagram and compiles its dates, as the reference's `parse` and `getTasks` do.
pub fn parse_with(raw: &str, options: Options) -> Result<GanttDiagram, ParseError> {
    let prepared = prepare(raw)?;
    // `Diagram.fromText`: the directives and comment lines go, then the entities are encoded and a
    // line break is added for the grammar.
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);

    let mut db = db::Db::new(options.now as f64);
    if let Some(title) = &prepared.title {
        db.set_diagram_title(title);
    }
    let tokens = scanner::tokenize(&text);
    parser::parse_statements(&tokens, source, &mut db)?;
    db.get_tasks()
        .map_err(|error| source.error(error.offset, error.message))?;
    Ok(db.into_model())
}
