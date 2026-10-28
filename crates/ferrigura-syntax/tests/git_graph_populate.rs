//! Port of the six in-source tests of `packages/mermaid/src/diagrams/git/gitGraphParser.ts`
//! (`GitGraph Parser`): `parseStatement` and `populate`, checked against a mock DB.
//!
//! The reference asserts which DB call each statement makes, with which arguments. The port has
//! no mock DB: each test parses a text whose statements make the same DB calls, and asserts the
//! DB result (the diagram). A statement that the DB would refuse (a merge into the same branch, a
//! cherry-pick from the current branch) is kept out of the texts, so each text is a valid graph.

use ferrigura_model::git_graph::{CommitType, GitGraphDiagram};
use ferrigura_syntax::git_graph::parse;

fn parsed(text: &str) -> GitGraphDiagram {
    parse(text).expect("the gitGraph parses")
}

#[test]
fn should_parse_a_commit_statement() {
    // The reference: `commit` with `{id: '1', msg: 'test', tags: ['tag1', 'tag2'], type: 0}`.
    let diagram =
        parsed("gitGraph\n commit id:\"1\" msg:\"test\" tag:\"tag1\" tag:\"tag2\" type:NORMAL\n");

    assert_eq!(diagram.commits.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.id, "1");
    assert!(!commit.auto_id);
    assert_eq!(commit.message, "test");
    assert_eq!(commit.tags, ["tag1", "tag2"]);
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_parse_a_branch_statement() {
    // The reference: `branch` with `{name: 'newBranch', order: 1}`.
    let diagram = parsed("gitGraph\n commit\n branch newBranch order:1\n");

    let branch = diagram
        .branches
        .iter()
        .find(|branch| branch.name == "newBranch")
        .expect("the branch exists");
    assert_eq!(branch.order, 1.0);
    assert_eq!(diagram.current_branch, "newBranch");
}

#[test]
fn should_parse_a_checkout_statement() {
    // The reference: `checkout` with `'newBranch'`.
    let diagram = parsed("gitGraph\n branch newBranch\n checkout main\n checkout newBranch\n");

    assert_eq!(diagram.current_branch, "newBranch");
}

#[test]
fn should_parse_a_merge_statement() {
    // The reference: `merge` with `{branch: 'newBranch', id: '1', tags: ['tag1', 'tag2'], type: 0}`.
    let diagram = parsed(
        "gitGraph\n commit\n branch newBranch\n commit\n checkout main\n merge newBranch id:\"1\" tag:\"tag1\" tag:\"tag2\" type:NORMAL\n",
    );

    let merge = &diagram.commits[2];
    assert_eq!(merge.id, "1");
    assert!(!merge.auto_id);
    assert_eq!(merge.kind, CommitType::Merge);
    assert_eq!(merge.custom_type, Some(CommitType::Normal));
    assert_eq!(merge.custom_id, Some(true));
    assert_eq!(merge.message, "merged branch newBranch into main");
    assert_eq!(merge.tags, ["tag1", "tag2"]);
    assert_eq!(merge.branch, "main");
}

#[test]
fn should_parse_a_cherry_picking_statement() {
    // The reference: `cherryPick` with `{id: '1', targetId: '', parent: '2', tags: ['tag1', 'tag2']}`.
    // Commit `1` is on `develop` with parent `2`, so the pick from `main` is valid.
    let diagram = parsed(
        "gitGraph\n commit id:\"2\"\n branch develop\n commit id:\"1\"\n checkout main\n cherry-pick id:\"1\" tag:\"tag1\" tag:\"tag2\" parent:\"2\"\n",
    );

    let cherry_pick = &diagram.commits[2];
    assert_eq!(cherry_pick.kind, CommitType::CherryPick);
    assert_eq!(cherry_pick.tags, ["tag1", "tag2"]);
    assert_eq!(cherry_pick.branch, "main");
    assert_eq!(cherry_pick.parents, ["2", "1"]);
    assert!(cherry_pick.auto_id);
}

#[test]
fn should_parse_a_langium_generated_gitgraph_ast() {
    // The reference populates a DB from one statement of each kind: commit, branch, merge,
    // checkout, cherry-pick. This text makes the same kinds of call in an order the DB accepts:
    // the checkout comes before the commit that moves the head of `newBranch`.
    let diagram = parsed(
        "gitGraph\n\
         commit id:\"1\" msg:\"test\" tag:\"tag1\" tag:\"tag2\" type:NORMAL\n\
         branch newBranch order:1\n\
         checkout newBranch\n\
         commit id:\"2\"\n\
         checkout main\n\
         merge newBranch id:\"3\" tag:\"tag1\" tag:\"tag2\" type:NORMAL\n\
         cherry-pick id:\"2\" tag:\"tag1\" tag:\"tag2\" parent:\"1\"\n",
    );

    assert_eq!(diagram.commits.len(), 4);

    let first = &diagram.commits[0];
    assert_eq!(first.id, "1");
    assert_eq!(first.message, "test");
    assert_eq!(first.tags, ["tag1", "tag2"]);
    assert_eq!(first.kind, CommitType::Normal);

    let new_branch = diagram
        .branches
        .iter()
        .find(|branch| branch.name == "newBranch")
        .expect("the branch exists");
    assert_eq!(new_branch.order, 1.0);

    let merge = &diagram.commits[2];
    assert_eq!(merge.id, "3");
    assert_eq!(merge.kind, CommitType::Merge);
    assert_eq!(merge.custom_type, Some(CommitType::Normal));
    assert_eq!(merge.tags, ["tag1", "tag2"]);

    let cherry_pick = &diagram.commits[3];
    assert_eq!(cherry_pick.kind, CommitType::CherryPick);
    assert_eq!(cherry_pick.tags, ["tag1", "tag2"]);
    assert_eq!(cherry_pick.parents, ["3", "2"]);

    assert_eq!(diagram.current_branch, "main");
}
