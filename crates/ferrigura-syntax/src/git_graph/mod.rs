//! The gitGraph parser (M6): `gitGraph.langium` and the gitGraph DB, ported to Rust. The plan, the
//! lexer's rules and the quirks kept are in `docs-port/gitgraph/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The DB then reads the
//! `mainBranchName` and `mainBranchOrder` settings when it is cleared, which [`parse`] takes from
//! the text's configuration.

pub mod ast;
mod db;
mod grammar;
mod scanner;

use ferrigura_model::git_graph::GitGraphDiagram;

use self::ast::{GitGraph, StatementKind};
use self::db::Db;
use crate::config;
use crate::error::{ParseError, Source};
use crate::preprocess::prepare;
use crate::preprocess::{cleanup_comments, encode_entities, remove_directives};

/// The settings the DB reads when it is cleared.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// `mainBranchName`: the name of the first branch.
    pub main_branch_name: String,
    /// `mainBranchOrder`: the order of the first branch.
    pub main_branch_order: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            main_branch_name: "main".to_owned(),
            main_branch_order: 0.0,
        }
    }
}

/// Parses a gitGraph with the settings the text carries.
pub fn parse(raw: &str) -> Result<GitGraphDiagram, ParseError> {
    let options = options_of(raw)?;
    parse_with(raw, &options)
}

/// Parses a gitGraph with the given settings.
pub fn parse_with(raw: &str, options: &Options) -> Result<GitGraphDiagram, ParseError> {
    parse_with_warnings(raw, options).map(|(diagram, _)| diagram)
}

/// Parses a gitGraph and returns the warnings the reference would log while parsing, such as a
/// repeated commit id. The warnings are not part of the model.
pub fn parse_with_warnings(
    raw: &str,
    options: &Options,
) -> Result<(GitGraphDiagram, Vec<String>), ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let ast = grammar::parse_text(Source::new(&text, prepared.line_offset))?;

    let source = Source::new(&text, prepared.line_offset);
    let mut db = Db::new(options);
    // `Diagram.fromText` sets the front matter title after the DB is cleared; the grammar's
    // title replaces it when the grammar gives one.
    if let Some(title) = &prepared.title {
        db.set_title(title);
    }
    populate(&ast, &mut db, source)?;
    let warnings = db.take_warnings();
    Ok((db.into_model(), warnings))
}

/// Parses the text of a gitGraph as the grammar reads it, with no preparation: the syntax tree,
/// for the parser tests.
pub fn parse_ast(text: &str) -> Result<GitGraph, ParseError> {
    grammar::parse_text(Source::new(text, 0))
}

/// The DB settings of a text: `gitGraph.mainBranchName` and `gitGraph.mainBranchOrder` from its
/// configuration, with the defaults where it gives none.
pub fn options_of(raw: &str) -> Result<Options, ParseError> {
    let defaults = Options::default();
    let settings = config::read(raw)?;
    let section = settings.get("gitGraph");
    let main_branch_name = section
        .and_then(|s| s.get("mainBranchName"))
        .and_then(|v| v.as_str())
        .map_or(defaults.main_branch_name, str::to_owned);
    let main_branch_order = section
        .and_then(|s| s.get("mainBranchOrder"))
        .and_then(|v| v.as_f64())
        .unwrap_or(defaults.main_branch_order);
    Ok(Options {
        main_branch_name,
        main_branch_order,
    })
}

/// The DB calls of `gitGraphParser.ts`'s `populate`: the header texts, the direction, then each
/// statement in order. A DB error is reported at the statement that caused it.
fn populate(ast: &GitGraph, db: &mut Db, source: Source<'_>) -> Result<(), ParseError> {
    if !ast.acc_descr.is_empty() {
        db.set_acc_descr(&ast.acc_descr);
    }
    if !ast.acc_title.is_empty() {
        db.set_acc_title(&ast.acc_title);
    }
    if !ast.title.is_empty() {
        db.set_title(&ast.title);
    }
    if let Some(direction) = ast.direction {
        db.set_direction(direction);
    }
    for statement in &ast.statements {
        let result = match &statement.kind {
            StatementKind::Commit(commit) => db.commit(
                commit.id.as_deref(),
                commit.message.as_deref().unwrap_or(""),
                commit.kind,
                &commit.tags,
            ),
            StatementKind::Branch(branch) => db.branch(&branch.name, branch.order.unwrap_or(0.0)),
            StatementKind::Merge(merge) => {
                db.merge(&merge.branch, merge.id.as_deref(), merge.kind, &merge.tags)
            }
            StatementKind::Checkout(checkout) => db.checkout(&checkout.branch),
            StatementKind::CherryPick(pick) => db.cherry_pick(
                pick.id.as_deref(),
                (!pick.tags.is_empty()).then_some(pick.tags.as_slice()),
                pick.parent.as_deref(),
            ),
        };
        result.map_err(|message| source.error(statement.offset, message))?;
    }
    Ok(())
}
