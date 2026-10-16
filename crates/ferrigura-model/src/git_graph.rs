//! Canonical model of a gitGraph diagram: the state of `gitGraphAst.ts` after parsing.
//!
//! The rules of the model are in `docs-port/canonical-model.md` and `docs-port/gitgraph/PLAN.md`
//! (section 8). Branches and commits are in the order of the DB's maps, which is the order they
//! were created in.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed gitGraph diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title: the grammar's `title` when it is non-empty, else the front matter's.
    pub title: String,
    /// Accessible title, empty if absent.
    pub acc_title: String,
    /// Accessible description, empty if absent.
    pub acc_descr: String,
    /// Direction of the graph, `LR` unless the header sets another one.
    pub direction: Direction,
    /// The branch that commits go to.
    pub current_branch: String,
    /// The branches in creation order, the main branch first.
    pub branches: Vec<GitGraphBranch>,
    /// The commits, merges and cherry-picks in the order their ids were first stored.
    pub commits: Vec<GitGraphCommit>,
}

/// The direction of a gitGraph, from its header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Left to right; the default.
    #[default]
    LR,
    /// Top to bottom.
    TB,
    /// Bottom to top.
    BT,
}

/// A branch of the diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphBranch {
    /// The branch name, sanitised.
    pub name: String,
    /// The `order:` of the branch; for the main branch, the `mainBranchOrder` setting.
    pub order: f64,
    /// The id of the branch's head commit, or `null` while the branch has no commit.
    pub head: Option<String>,
}

/// The kind of a commit. Merges and cherry-picks have their own kinds; the other three come from
/// the `type:` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommitType {
    /// `NORMAL`, the default of a commit.
    Normal,
    /// `REVERSE`: drawn with a cross.
    Reverse,
    /// `HIGHLIGHT`: drawn as a highlighted square.
    Highlight,
    /// A merge.
    Merge,
    /// A cherry-pick.
    CherryPick,
}

/// A commit, merge or cherry-pick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphCommit {
    /// The id, sanitised. Generated when the input gives none (see `auto_id`).
    pub id: String,
    /// Position in the order of creation, from 0.
    pub seq: u64,
    /// The branch the commit was made on.
    pub branch: String,
    /// The kind of the commit: `NORMAL`, `REVERSE` or `HIGHLIGHT` for a commit, `MERGE` or
    /// `CHERRY_PICK` otherwise.
    #[serde(rename = "type")]
    pub kind: CommitType,
    /// The `type:` option of a merge, `null` on every other commit and on a merge without one.
    pub custom_type: Option<CommitType>,
    /// Whether a merge was given its own id. Only merges have it: `true` or `false` on a merge,
    /// `null` on every other commit.
    pub custom_id: Option<bool>,
    /// Whether the id was generated because the input gave none.
    pub auto_id: bool,
    /// The message, as the input gives it (not trimmed).
    pub message: String,
    /// The tags, in the order given.
    pub tags: Vec<String>,
    /// The parent ids, in order. Duplicates and self-references are kept as the DB keeps them.
    pub parents: Vec<String>,
}

impl GitGraphDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: Direction::LR,
            current_branch: String::new(),
            branches: Vec::new(),
            commits: Vec::new(),
        }
    }
}

impl Default for GitGraphDiagram {
    fn default() -> Self {
        Self::new()
    }
}
