//! The event modeling parser (P2): `event-modeling.langium`, its DB `db.ts` and the title and
//! header rules of the common DB, ported by hand. The plan, the rule table and the quirks kept are
//! in `docs-port/eventmodeling/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, the comment lines, which this parser keeps, entities, and a line
//! break. The grammar builds the syntax tree, the DB places the frames and creates the relations,
//! and the model holds the result without the geometry.

pub mod ast;
mod db;
mod grammar;
mod scanner;
pub mod validator;

use std::collections::HashSet;

use ferrigura_model::event_modeling::{DataEntity, EventModelingDiagram, Frame, FrameKind};

use self::ast::EventModel;
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The default `maxTextSize`: a text whose comment-preserving form is longer than this (in UTF-16
/// units, as JavaScript counts) is parsed without its comment lines. The configuration's own
/// `maxTextSize` is not read by the crate's pipeline.
const MAX_TEXT_SIZE: usize = 50_000;

/// Parses an event modeling diagram and returns its model.
///
/// # Errors
///
/// A lexer or grammar error at the first token that fails, or a frame number that an earlier frame
/// already uses, at the later frame's keyword. Positions count the lines of the original input,
/// front matter included.
pub fn parse(raw: &str) -> Result<EventModelingDiagram, ParseError> {
    let parsed = parse_tree(raw)?;
    let source = Source::new(&parsed.text, parsed.line_offset);
    check_frame_ids(&parsed.ast, source)?;
    let state = db::state(&parsed.ast);
    Ok(to_model(&parsed, &state))
}

/// Parses the text into its syntax tree, without the DB's checks and without the model.
///
/// # Errors
///
/// A lexer or grammar error, as [`parse`] reports it.
pub fn parse_ast(raw: &str) -> Result<EventModel, ParseError> {
    parse_tree(raw).map(|parsed| parsed.ast)
}

/// The syntax tree with the text it was read from, and what positions need.
struct Parsed {
    ast: EventModel,
    /// The text the grammar read: comments kept, entities encoded, and a line break at the end.
    text: String,
    /// The number of front matter lines, which the positions count.
    line_offset: usize,
    /// The front matter's title, which the DB sets before the grammar's title.
    front_matter_title: Option<String>,
}

fn parse_tree(raw: &str) -> Result<Parsed, ParseError> {
    let prepared = prepare(raw)?;
    // The DB keeps the comment lines (`preserveCommentsWhenParsing`), unless the text is too long.
    let with_comments = remove_directives(&prepared.text);
    let source_text = if with_comments.encode_utf16().count() > MAX_TEXT_SIZE {
        cleanup_comments(&with_comments)
    } else {
        with_comments
    };
    let text = encode_entities(&source_text) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let tokens = scanner::scan(&text, source)?;
    let ast = grammar::parse_model(&tokens, source)?;
    Ok(Parsed {
        ast,
        text,
        line_offset: prepared.line_offset,
        front_matter_title: prepared.title,
    })
}

/// `setAst`: a frame number that an earlier frame already uses is an error, at the later frame.
fn check_frame_ids(ast: &EventModel, source: Source<'_>) -> Result<(), ParseError> {
    let mut seen = HashSet::new();
    for frame in &ast.frames {
        if !seen.insert(frame.name.as_str()) {
            return Err(source.error(
                frame.offset,
                format!(
                    "frame id \"{}\" is already used by an earlier frame",
                    frame.name
                ),
            ));
        }
    }
    Ok(())
}

/// The model: the header texts as the common DB sets them, then the frames, data blocks, lanes and
/// relations of the DB state.
fn to_model(parsed: &Parsed, state: &db::State) -> EventModelingDiagram {
    let ast = &parsed.ast;
    // `Diagram.fromText` sets the front matter's title, and `populateCommonDb` then sets the
    // grammar's title, which the DB sanitises whatever it holds.
    let title = match &ast.title {
        Some(raw) => sanitize_text(raw),
        None => parsed
            .front_matter_title
            .as_deref()
            .map(sanitize_text)
            .unwrap_or_default(),
    };
    let acc_title = ast
        .acc_title
        .as_deref()
        .map(|raw| strip_leading_space(&sanitize_text(raw)).to_owned())
        .unwrap_or_default();
    let acc_descr = ast
        .acc_descr
        .as_deref()
        .map(|raw| newline_then_space_to_newline(&sanitize_text(raw)))
        .unwrap_or_default();

    let frames = ast
        .frames
        .iter()
        .zip(&state.lane_of_frame)
        .map(|(frame, &lane)| Frame {
            name: frame.name.clone(),
            kind: match frame.kind {
                ast::FrameKind::Time => FrameKind::Time,
                ast::FrameKind::Reset => FrameKind::Reset,
            },
            model_entity_type: frame.model_entity_type.clone(),
            entity_identifier: frame.entity_identifier.clone(),
            entity_name: db::entity_name(&frame.entity_identifier),
            namespace: db::namespace(&frame.entity_identifier),
            lane,
            source_frames: frame.source_frames.clone(),
            data_inline: frame.data_inline.clone(),
            data_reference: frame.data_reference.clone(),
            data: db::data_text(frame, &ast.data_entities),
        })
        .collect();

    let data_entities = ast
        .data_entities
        .iter()
        .map(|data| DataEntity {
            name: data.name.clone(),
            data_block_value: data.data_block_value.clone(),
        })
        .collect();

    EventModelingDiagram {
        version: ferrigura_model::CANONICAL_VERSION,
        title,
        acc_title,
        acc_descr,
        frames,
        data_entities,
        lanes: state.lanes.clone(),
        relations: state.relations.clone(),
    }
}
