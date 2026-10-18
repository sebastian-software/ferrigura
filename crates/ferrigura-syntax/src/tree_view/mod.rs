//! The parser and DB of the tree view (`treeView-beta`), ported from
//! `packages/parser/src/language/treeView/` and `packages/mermaid/src/diagrams/treeView/`. The
//! research is in `docs-port/tree-view/NOTES.md`, the decisions and the quirks each one keeps in
//! `docs-port/tree-view/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities and a line break. The box-drawing
//! preprocessor then rewrites a tree drawn with `├──` or `└──` as indentation. The grammar reads
//! the result, and the DB is filled in the order `parser.ts` gives it: the header texts, then each
//! node. The renderer is not here: the layout and the icons are the renderer stage.

mod box_drawing;
mod db;
mod grammar;
mod scanner;

use ferrigura_model::tree_view::{NodeType, TreeViewDiagram};

use self::db::Db;
use self::grammar::Ast;
use crate::error::ParseError;
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};

#[doc(hidden)]
pub use self::box_drawing::{BoxDrawing, is_box_drawing_format, preprocess_box_drawing};

/// Parses a tree view diagram, with its front matter, and returns its model.
///
/// # Errors
///
/// A lexer or grammar error at the first token the grammar does not take, with its position in the
/// original text, or one of the two box-drawing errors. Both are rejections.
pub fn parse(raw: &str) -> Result<TreeViewDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let line_offset = prepared.line_offset;
    let boxed = preprocess_box_drawing(&text).map_err(|mut error| {
        error.line += line_offset;
        error
    })?;
    let ast = grammar::parse_text(&boxed.text).map_err(|error| {
        let mut error = boxed.remap_error(error);
        error.line += line_offset;
        error
    })?;

    let mut db = Db::new();
    // `Diagram.fromText` sets the front matter title after the DB is cleared; the grammar's title
    // replaces it when the grammar gives one (`populateCommonDb`).
    if let Some(title) = &prepared.title {
        db.set_title(title);
    }
    populate(&ast, &mut db);
    Ok(db.into_model())
}

/// The DB calls of `parser.ts`'s `populate`: the header texts, which are set only when they are not
/// empty, then one `addNode` for each node in text order.
fn populate(ast: &Ast, db: &mut Db) {
    if !ast.acc_descr.is_empty() {
        db.set_acc_descr(&ast.acc_descr);
    }
    if !ast.acc_title.is_empty() {
        db.set_acc_title(&ast.acc_title);
    }
    if !ast.title.is_empty() {
        db.set_title(&ast.title);
    }
    for node in &ast.nodes {
        // A name with a trailing `/` is a directory, and the `/` goes (`parser.ts`, lines 20-24).
        let (name, node_type) = match node.name.strip_suffix('/') {
            Some(name) => (name.to_owned(), NodeType::Directory),
            None => (node.name.clone(), NodeType::File),
        };
        // An icon that is given as `icon()` is suppressed, and an absent one stays absent.
        let icon = match node.icon.as_deref() {
            Some("") => "none".to_owned(),
            Some(icon) => icon.to_owned(),
            None => String::new(),
        };
        // A description is sanitised only when it is not empty; an empty one is absent.
        let description = match node.description.as_deref() {
            Some(text) if !text.is_empty() => crate::sanitize::sanitize_text(text),
            _ => String::new(),
        };
        db.add_node(
            i64::try_from(node.indent).unwrap_or(i64::MAX),
            name,
            node_type,
            node.class.clone().unwrap_or_default(),
            icon,
            description,
        );
    }
}
