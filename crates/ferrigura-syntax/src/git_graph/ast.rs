//! The syntax tree of a gitGraph, as `gitGraph.langium` gives it. The texts are the grammar's
//! values (`STRING` without its quotes and escapes resolved, `TITLE` and the accessibility texts
//! through their converters). A missing header text is `""`, which is what the reference's truthy
//! checks test for.

use ferrigura_model::git_graph::{CommitType, Direction};

/// A whole gitGraph: the header, the header texts (the last value given wins) and the statements.
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraph {
    /// The direction of the header, if it gives one.
    pub direction: Option<Direction>,
    /// The `accDescr` text, `""` if none.
    pub acc_descr: String,
    /// The `accTitle` text, `""` if none.
    pub acc_title: String,
    /// The `title` text, `""` if none.
    pub title: String,
    /// The statements in source order.
    pub statements: Vec<Statement>,
}

/// A statement with the byte offset of its keyword in the text the grammar read.
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// Where the statement starts.
    pub offset: usize,
    /// What the statement is.
    pub kind: StatementKind,
}

/// The five statements of the grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    /// `commit`.
    Commit(Commit),
    /// `branch`.
    Branch(Branch),
    /// `merge`.
    Merge(Merge),
    /// `checkout` or `switch`.
    Checkout(Checkout),
    /// `cherry-pick`.
    CherryPick(CherryPick),
}

/// `commit` with its options. A later option of the same kind replaces an earlier one, and `tag:`
/// accumulates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Commit {
    /// The `id:` option.
    pub id: Option<String>,
    /// The `msg:` option, or a bare string.
    pub message: Option<String>,
    /// The `tag:` options, in order.
    pub tags: Vec<String>,
    /// The `type:` option.
    pub kind: Option<CommitType>,
}

/// `branch name`, with an optional `order:`.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    /// The name, a reference or a string.
    pub name: String,
    /// The `order:` number, if given.
    pub order: Option<f64>,
}

/// `merge name` with its options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Merge {
    /// The branch that is merged into the current one.
    pub branch: String,
    /// The `id:` option.
    pub id: Option<String>,
    /// The `tag:` options, in order.
    pub tags: Vec<String>,
    /// The `type:` option.
    pub kind: Option<CommitType>,
}

/// `checkout name` or `switch name`.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkout {
    /// The branch to switch to.
    pub branch: String,
}

/// `cherry-pick` with its options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CherryPick {
    /// The `id:` option, the commit to pick.
    pub id: Option<String>,
    /// The `tag:` options, in order.
    pub tags: Vec<String>,
    /// The `parent:` option, needed when the picked commit is a merge.
    pub parent: Option<String>,
}
